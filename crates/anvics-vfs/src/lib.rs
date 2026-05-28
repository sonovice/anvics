use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path, PathBuf},
    sync::{Arc, Mutex},
};
#[cfg(feature = "fuse")]
use std::{
    ffi::{OsStr, OsString},
    time::SystemTime,
};
use thiserror::Error;
use walkdir::WalkDir;

#[derive(Debug, Error)]
pub enum VfsError {
    #[error("FUSE support is not compiled; rebuild with --features vfs-fuse")]
    FuseNotCompiled,
    #[error("path is outside the VFS root: {0}")]
    OutsideRoot(PathBuf),
    #[error("unsupported VFS entry type: {0}")]
    UnsupportedEntry(PathBuf),
    #[error("FUSE mount failed at {mount_path}: {source}")]
    MountFailed {
        mount_path: PathBuf,
        source: std::io::Error,
    },
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Walk(#[from] walkdir::Error),
}

pub type Result<T> = std::result::Result<T, VfsError>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VfsFileEffect {
    pub path: String,
    pub status: VfsFileEffectStatus,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VfsFileEffectStatus {
    Added,
    Modified,
    Deleted,
}

#[derive(Clone, Debug)]
pub struct MountedWorkspace {
    mount_path: PathBuf,
    mirror: WorkspaceMirror,
    #[cfg(feature = "fuse")]
    _session: Arc<fuser::BackgroundSession>,
}

impl MountedWorkspace {
    pub fn mount_path(&self) -> &Path {
        &self.mount_path
    }

    pub fn persist_to_path(&self, target: &Path) -> Result<()> {
        self.mirror.persist_to_path(target)
    }

    pub fn changed_paths(&self) -> Vec<VfsFileEffect> {
        self.mirror.changed_paths()
    }
}

#[cfg(feature = "fuse")]
pub fn mount_workspace(source: &Path, mount_path: &Path) -> Result<MountedWorkspace> {
    fs::create_dir_all(mount_path)?;
    let mirror = WorkspaceMirror::from_path(source)?;
    let filesystem = AnvicsFuseFilesystem::new(mirror.clone());
    let mut config = fuser::Config::default();
    config.mount_options = vec![
        fuser::MountOption::FSName("anvics-workspace".to_owned()),
        fuser::MountOption::Subtype("anvics".to_owned()),
        fuser::MountOption::RW,
        fuser::MountOption::NoDev,
        fuser::MountOption::NoSuid,
    ];
    let session = fuser::spawn_mount2(filesystem, mount_path, &config).map_err(|source| {
        VfsError::MountFailed {
            mount_path: mount_path.to_path_buf(),
            source,
        }
    })?;

    Ok(MountedWorkspace {
        mount_path: mount_path.to_path_buf(),
        mirror,
        _session: Arc::new(session),
    })
}

#[cfg(not(feature = "fuse"))]
pub fn mount_workspace(_source: &Path, _mount_path: &Path) -> Result<MountedWorkspace> {
    Err(VfsError::FuseNotCompiled)
}

#[derive(Clone, Debug)]
pub struct WorkspaceMirror {
    state: Arc<Mutex<MirrorState>>,
}

impl WorkspaceMirror {
    pub fn from_path(root: &Path) -> Result<Self> {
        let mut state = MirrorState::new();
        for entry in WalkDir::new(root).sort_by_file_name() {
            let entry = entry?;
            let path = entry.path();
            if path == root {
                continue;
            }
            let relative = path
                .strip_prefix(root)
                .map_err(|_| VfsError::OutsideRoot(path.to_path_buf()))?;
            if entry.file_type().is_dir() {
                state.insert_dir(relative.to_path_buf());
            } else if entry.file_type().is_file() {
                state.insert_file(relative.to_path_buf(), fs::read(path)?);
            } else {
                return Err(VfsError::UnsupportedEntry(path.to_path_buf()));
            }
        }
        state.initial_files = state.file_digest_map();
        Ok(Self {
            state: Arc::new(Mutex::new(state)),
        })
    }

    pub fn persist_to_path(&self, target: &Path) -> Result<()> {
        let state = self.state.lock().expect("workspace mirror mutex poisoned");
        if target.exists() {
            fs::remove_dir_all(target)?;
        }
        fs::create_dir_all(target)?;
        for (path, entry) in &state.entries {
            if path.as_os_str().is_empty() {
                continue;
            }
            let target_path = target.join(path);
            match entry {
                MirrorEntry::Directory => fs::create_dir_all(&target_path)?,
                MirrorEntry::File { bytes } => {
                    if let Some(parent) = target_path.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::write(target_path, bytes)?;
                }
            }
        }
        Ok(())
    }

    pub fn changed_paths(&self) -> Vec<VfsFileEffect> {
        let state = self.state.lock().expect("workspace mirror mutex poisoned");
        let after = state.file_digest_map();
        let mut paths = BTreeSet::new();
        paths.extend(state.initial_files.keys().cloned());
        paths.extend(after.keys().cloned());

        paths
            .into_iter()
            .filter_map(|path| {
                let before = state.initial_files.get(&path);
                let current = after.get(&path);
                match (before, current) {
                    (None, Some(_)) => Some(VfsFileEffect {
                        path,
                        status: VfsFileEffectStatus::Added,
                    }),
                    (Some(_), None) => Some(VfsFileEffect {
                        path,
                        status: VfsFileEffectStatus::Deleted,
                    }),
                    (Some(before), Some(current)) if before != current => Some(VfsFileEffect {
                        path,
                        status: VfsFileEffectStatus::Modified,
                    }),
                    _ => None,
                }
            })
            .collect()
    }
}

#[derive(Debug)]
struct MirrorState {
    entries: BTreeMap<PathBuf, MirrorEntry>,
    initial_files: BTreeMap<String, [u8; 32]>,
    inodes: BTreeMap<u64, PathBuf>,
    paths: BTreeMap<PathBuf, u64>,
    next_inode: u64,
}

impl MirrorState {
    fn new() -> Self {
        let mut entries = BTreeMap::new();
        let mut inodes = BTreeMap::new();
        let mut paths = BTreeMap::new();
        entries.insert(PathBuf::new(), MirrorEntry::Directory);
        inodes.insert(1, PathBuf::new());
        paths.insert(PathBuf::new(), 1);
        Self {
            entries,
            initial_files: BTreeMap::new(),
            inodes,
            paths,
            next_inode: 2,
        }
    }

    fn insert_dir(&mut self, path: PathBuf) -> u64 {
        self.ensure_parents(&path);
        self.insert_entry(path, MirrorEntry::Directory)
    }

    fn insert_file(&mut self, path: PathBuf, bytes: Vec<u8>) -> u64 {
        self.ensure_parents(&path);
        self.insert_entry(path, MirrorEntry::File { bytes })
    }

    fn insert_entry(&mut self, path: PathBuf, entry: MirrorEntry) -> u64 {
        if let Some(ino) = self.paths.get(&path).copied() {
            self.entries.insert(path, entry);
            return ino;
        }
        let ino = self.next_inode;
        self.next_inode += 1;
        self.entries.insert(path.clone(), entry);
        self.paths.insert(path.clone(), ino);
        self.inodes.insert(ino, path);
        ino
    }

    fn ensure_parents(&mut self, path: &Path) {
        let mut current = PathBuf::new();
        for component in path.parent().into_iter().flat_map(Path::components) {
            if let Component::Normal(name) = component {
                current.push(name);
                if !self.entries.contains_key(&current) {
                    self.insert_entry(current.clone(), MirrorEntry::Directory);
                }
            }
        }
    }

    fn file_digest_map(&self) -> BTreeMap<String, [u8; 32]> {
        self.entries
            .iter()
            .filter_map(|(path, entry)| match entry {
                MirrorEntry::File { bytes } => Some((
                    path.to_string_lossy().replace('\\', "/"),
                    blake3_hash(bytes),
                )),
                MirrorEntry::Directory => None,
            })
            .collect()
    }

    #[cfg(feature = "fuse")]
    fn path_for_inode(&self, ino: u64) -> Option<PathBuf> {
        self.inodes.get(&ino).cloned()
    }

    #[cfg(feature = "fuse")]
    fn inode_for_path(&self, path: &Path) -> Option<u64> {
        self.paths.get(path).copied()
    }

    #[cfg(feature = "fuse")]
    fn lookup_child(&self, parent: u64, name: &OsStr) -> Option<(u64, PathBuf, MirrorEntry)> {
        let parent_path = self.path_for_inode(parent)?;
        let child = if parent_path.as_os_str().is_empty() {
            PathBuf::from(name)
        } else {
            parent_path.join(name)
        };
        let ino = self.inode_for_path(&child)?;
        let entry = self.entries.get(&child)?.clone();
        Some((ino, child, entry))
    }

    #[cfg(feature = "fuse")]
    fn children_of(&self, ino: u64) -> Vec<(u64, OsString, MirrorEntry)> {
        let Some(parent_path) = self.path_for_inode(ino) else {
            return Vec::new();
        };
        let parent_depth = path_depth(&parent_path);
        self.entries
            .iter()
            .filter_map(|(path, entry)| {
                if path.as_os_str().is_empty()
                    || path.parent().unwrap_or(Path::new("")) != parent_path
                {
                    return None;
                }
                if path_depth(path) != parent_depth + 1 {
                    return None;
                }
                Some((
                    self.inode_for_path(path)?,
                    path.file_name()?.to_os_string(),
                    entry.clone(),
                ))
            })
            .collect()
    }

    #[cfg(any(feature = "fuse", test))]
    fn remove_path_recursive(&mut self, path: &Path) {
        let targets: Vec<PathBuf> = self
            .entries
            .keys()
            .filter(|candidate| *candidate == path || candidate.starts_with(path))
            .cloned()
            .collect();
        for target in targets {
            self.entries.remove(&target);
            if let Some(ino) = self.paths.remove(&target) {
                self.inodes.remove(&ino);
            }
        }
    }
}

#[derive(Clone, Debug)]
enum MirrorEntry {
    Directory,
    File { bytes: Vec<u8> },
}

fn blake3_hash(bytes: &[u8]) -> [u8; 32] {
    *blake3::hash(bytes).as_bytes()
}

#[cfg(feature = "fuse")]
fn path_depth(path: &Path) -> usize {
    path.components()
        .filter(|component| matches!(component, Component::Normal(_)))
        .count()
}

#[cfg(feature = "fuse")]
#[derive(Debug)]
struct AnvicsFuseFilesystem {
    mirror: WorkspaceMirror,
}

#[cfg(feature = "fuse")]
impl AnvicsFuseFilesystem {
    fn new(mirror: WorkspaceMirror) -> Self {
        Self { mirror }
    }
}

#[cfg(feature = "fuse")]
impl fuser::Filesystem for AnvicsFuseFilesystem {
    fn lookup(
        &self,
        _req: &fuser::Request,
        parent: fuser::INodeNo,
        name: &OsStr,
        reply: fuser::ReplyEntry,
    ) {
        let state = self
            .mirror
            .state
            .lock()
            .expect("workspace mirror mutex poisoned");
        let Some((ino, _path, entry)) = state.lookup_child(parent.into(), name) else {
            reply.error(fuser::Errno::ENOENT);
            return;
        };
        reply.entry(&ttl(), &file_attr(ino, &entry), fuser::Generation(0));
    }

    fn getattr(
        &self,
        _req: &fuser::Request,
        ino: fuser::INodeNo,
        _fh: Option<fuser::FileHandle>,
        reply: fuser::ReplyAttr,
    ) {
        let state = self
            .mirror
            .state
            .lock()
            .expect("workspace mirror mutex poisoned");
        let Some(path) = state.path_for_inode(ino.into()) else {
            reply.error(fuser::Errno::ENOENT);
            return;
        };
        let Some(entry) = state.entries.get(&path) else {
            reply.error(fuser::Errno::ENOENT);
            return;
        };
        reply.attr(&ttl(), &file_attr(ino.into(), entry));
    }

    fn readdir(
        &self,
        _req: &fuser::Request,
        ino: fuser::INodeNo,
        _fh: fuser::FileHandle,
        offset: u64,
        mut reply: fuser::ReplyDirectory,
    ) {
        let state = self
            .mirror
            .state
            .lock()
            .expect("workspace mirror mutex poisoned");
        if !state.inodes.contains_key(&ino.into()) {
            reply.error(fuser::Errno::ENOENT);
            return;
        }

        let mut entries = vec![
            (ino, OsString::from("."), fuser::FileType::Directory),
            (
                fuser::INodeNo(1),
                OsString::from(".."),
                fuser::FileType::Directory,
            ),
        ];
        entries.extend(
            state
                .children_of(ino.into())
                .into_iter()
                .map(|(child_ino, name, entry)| {
                    (fuser::INodeNo(child_ino), name, file_type(&entry))
                }),
        );

        for (index, (entry_ino, name, kind)) in
            entries.into_iter().enumerate().skip(offset as usize)
        {
            if reply.add(entry_ino, (index + 1) as u64, kind, name) {
                break;
            }
        }
        reply.ok();
    }

    fn open(
        &self,
        _req: &fuser::Request,
        ino: fuser::INodeNo,
        _flags: fuser::OpenFlags,
        reply: fuser::ReplyOpen,
    ) {
        let state = self
            .mirror
            .state
            .lock()
            .expect("workspace mirror mutex poisoned");
        match state
            .path_for_inode(ino.into())
            .and_then(|path| state.entries.get(&path))
        {
            Some(MirrorEntry::File { .. }) => {
                reply.opened(fuser::FileHandle(0), fuser::FopenFlags::empty())
            }
            Some(MirrorEntry::Directory) => reply.error(fuser::Errno::EISDIR),
            None => reply.error(fuser::Errno::ENOENT),
        }
    }

    fn read(
        &self,
        _req: &fuser::Request,
        ino: fuser::INodeNo,
        _fh: fuser::FileHandle,
        offset: u64,
        size: u32,
        _flags: fuser::OpenFlags,
        _lock_owner: Option<fuser::LockOwner>,
        reply: fuser::ReplyData,
    ) {
        let state = self
            .mirror
            .state
            .lock()
            .expect("workspace mirror mutex poisoned");
        let Some(path) = state.path_for_inode(ino.into()) else {
            reply.error(fuser::Errno::ENOENT);
            return;
        };
        match state.entries.get(&path) {
            Some(MirrorEntry::File { bytes }) => {
                let start = offset as usize;
                let end = bytes.len().min(start.saturating_add(size as usize));
                reply.data(if start >= bytes.len() {
                    &[]
                } else {
                    &bytes[start..end]
                });
            }
            Some(MirrorEntry::Directory) => reply.error(fuser::Errno::EISDIR),
            None => reply.error(fuser::Errno::ENOENT),
        }
    }

    fn write(
        &self,
        _req: &fuser::Request,
        ino: fuser::INodeNo,
        _fh: fuser::FileHandle,
        offset: u64,
        data: &[u8],
        _write_flags: fuser::WriteFlags,
        _flags: fuser::OpenFlags,
        _lock_owner: Option<fuser::LockOwner>,
        reply: fuser::ReplyWrite,
    ) {
        let mut state = self
            .mirror
            .state
            .lock()
            .expect("workspace mirror mutex poisoned");
        let Some(path) = state.path_for_inode(ino.into()) else {
            reply.error(fuser::Errno::ENOENT);
            return;
        };
        match state.entries.get_mut(&path) {
            Some(MirrorEntry::File { bytes }) => {
                let start = offset as usize;
                if bytes.len() < start {
                    bytes.resize(start, 0);
                }
                let end = start + data.len();
                if bytes.len() < end {
                    bytes.resize(end, 0);
                }
                bytes[start..end].copy_from_slice(data);
                reply.written(data.len() as u32);
            }
            Some(MirrorEntry::Directory) => reply.error(fuser::Errno::EISDIR),
            None => reply.error(fuser::Errno::ENOENT),
        }
    }

    fn setattr(
        &self,
        _req: &fuser::Request,
        ino: fuser::INodeNo,
        _mode: Option<u32>,
        _uid: Option<u32>,
        _gid: Option<u32>,
        size: Option<u64>,
        _atime: Option<fuser::TimeOrNow>,
        _mtime: Option<fuser::TimeOrNow>,
        _ctime: Option<SystemTime>,
        _fh: Option<fuser::FileHandle>,
        _crtime: Option<SystemTime>,
        _chgtime: Option<SystemTime>,
        _bkuptime: Option<SystemTime>,
        _flags: Option<fuser::BsdFileFlags>,
        reply: fuser::ReplyAttr,
    ) {
        let mut state = self
            .mirror
            .state
            .lock()
            .expect("workspace mirror mutex poisoned");
        let Some(path) = state.path_for_inode(ino.into()) else {
            reply.error(fuser::Errno::ENOENT);
            return;
        };
        if let Some(size) = size {
            match state.entries.get_mut(&path) {
                Some(MirrorEntry::File { bytes }) => bytes.resize(size as usize, 0),
                Some(MirrorEntry::Directory) => {
                    reply.error(fuser::Errno::EISDIR);
                    return;
                }
                None => {
                    reply.error(fuser::Errno::ENOENT);
                    return;
                }
            }
        }
        let Some(entry) = state.entries.get(&path) else {
            reply.error(fuser::Errno::ENOENT);
            return;
        };
        reply.attr(&ttl(), &file_attr(ino.into(), entry));
    }

    fn create(
        &self,
        _req: &fuser::Request,
        parent: fuser::INodeNo,
        name: &OsStr,
        _mode: u32,
        _umask: u32,
        _flags: i32,
        reply: fuser::ReplyCreate,
    ) {
        let mut state = self
            .mirror
            .state
            .lock()
            .expect("workspace mirror mutex poisoned");
        let Some(parent_path) = state.path_for_inode(parent.into()) else {
            reply.error(fuser::Errno::ENOENT);
            return;
        };
        let path = if parent_path.as_os_str().is_empty() {
            PathBuf::from(name)
        } else {
            parent_path.join(name)
        };
        if state.entries.contains_key(&path) {
            reply.error(fuser::Errno::EEXIST);
            return;
        }
        let ino = state.insert_file(path, Vec::new());
        let attr = file_attr(ino, &MirrorEntry::File { bytes: Vec::new() });
        reply.created(
            &ttl(),
            &attr,
            fuser::Generation(0),
            fuser::FileHandle(0),
            fuser::FopenFlags::empty(),
        );
    }

    fn mknod(
        &self,
        _req: &fuser::Request,
        parent: fuser::INodeNo,
        name: &OsStr,
        _mode: u32,
        _umask: u32,
        _rdev: u32,
        reply: fuser::ReplyEntry,
    ) {
        let mut state = self
            .mirror
            .state
            .lock()
            .expect("workspace mirror mutex poisoned");
        let Some(parent_path) = state.path_for_inode(parent.into()) else {
            reply.error(fuser::Errno::ENOENT);
            return;
        };
        let path = if parent_path.as_os_str().is_empty() {
            PathBuf::from(name)
        } else {
            parent_path.join(name)
        };
        if state.entries.contains_key(&path) {
            reply.error(fuser::Errno::EEXIST);
            return;
        }
        let ino = state.insert_file(path, Vec::new());
        reply.entry(
            &ttl(),
            &file_attr(ino, &MirrorEntry::File { bytes: Vec::new() }),
            fuser::Generation(0),
        );
    }

    fn mkdir(
        &self,
        _req: &fuser::Request,
        parent: fuser::INodeNo,
        name: &OsStr,
        _mode: u32,
        _umask: u32,
        reply: fuser::ReplyEntry,
    ) {
        let mut state = self
            .mirror
            .state
            .lock()
            .expect("workspace mirror mutex poisoned");
        let Some(parent_path) = state.path_for_inode(parent.into()) else {
            reply.error(fuser::Errno::ENOENT);
            return;
        };
        let path = if parent_path.as_os_str().is_empty() {
            PathBuf::from(name)
        } else {
            parent_path.join(name)
        };
        if state.entries.contains_key(&path) {
            reply.error(fuser::Errno::EEXIST);
            return;
        }
        let ino = state.insert_dir(path);
        reply.entry(
            &ttl(),
            &file_attr(ino, &MirrorEntry::Directory),
            fuser::Generation(0),
        );
    }

    fn unlink(
        &self,
        _req: &fuser::Request,
        parent: fuser::INodeNo,
        name: &OsStr,
        reply: fuser::ReplyEmpty,
    ) {
        let mut state = self
            .mirror
            .state
            .lock()
            .expect("workspace mirror mutex poisoned");
        let Some((_, path, entry)) = state.lookup_child(parent.into(), name) else {
            reply.error(fuser::Errno::ENOENT);
            return;
        };
        match entry {
            MirrorEntry::File { .. } => {
                state.remove_path_recursive(&path);
                reply.ok();
            }
            MirrorEntry::Directory => reply.error(fuser::Errno::EISDIR),
        }
    }

    fn rmdir(
        &self,
        _req: &fuser::Request,
        parent: fuser::INodeNo,
        name: &OsStr,
        reply: fuser::ReplyEmpty,
    ) {
        let mut state = self
            .mirror
            .state
            .lock()
            .expect("workspace mirror mutex poisoned");
        let Some((_, path, entry)) = state.lookup_child(parent.into(), name) else {
            reply.error(fuser::Errno::ENOENT);
            return;
        };
        match entry {
            MirrorEntry::Directory => {
                if state
                    .entries
                    .keys()
                    .any(|candidate| candidate.parent().unwrap_or(Path::new("")) == path)
                {
                    reply.error(fuser::Errno::ENOTEMPTY);
                } else {
                    state.remove_path_recursive(&path);
                    reply.ok();
                }
            }
            MirrorEntry::File { .. } => reply.error(fuser::Errno::ENOTDIR),
        }
    }
}

#[cfg(feature = "fuse")]
fn ttl() -> std::time::Duration {
    std::time::Duration::from_secs(1)
}

#[cfg(feature = "fuse")]
fn file_type(entry: &MirrorEntry) -> fuser::FileType {
    match entry {
        MirrorEntry::Directory => fuser::FileType::Directory,
        MirrorEntry::File { .. } => fuser::FileType::RegularFile,
    }
}

#[cfg(feature = "fuse")]
fn file_attr(ino: u64, entry: &MirrorEntry) -> fuser::FileAttr {
    let now = SystemTime::now();
    let (kind, size, perm, nlink) = match entry {
        MirrorEntry::Directory => (fuser::FileType::Directory, 0, 0o777, 2),
        MirrorEntry::File { bytes } => (fuser::FileType::RegularFile, bytes.len() as u64, 0o666, 1),
    };
    fuser::FileAttr {
        ino: fuser::INodeNo(ino),
        size,
        blocks: size.div_ceil(512),
        atime: now,
        mtime: now,
        ctime: now,
        crtime: now,
        kind,
        perm,
        nlink,
        uid: 0,
        gid: 0,
        rdev: 0,
        blksize: 4096,
        flags: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "fuse")]
    use std::process::Command;

    #[test]
    fn mirror_reports_added_modified_deleted_paths() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("modified.txt"), "before\n").unwrap();
        fs::write(dir.path().join("deleted.txt"), "delete\n").unwrap();
        let mirror = WorkspaceMirror::from_path(dir.path()).unwrap();
        {
            let mut state = mirror.state.lock().unwrap();
            state.insert_file(PathBuf::from("modified.txt"), b"after\n".to_vec());
            state.remove_path_recursive(Path::new("deleted.txt"));
            state.insert_file(PathBuf::from("added.txt"), b"new\n".to_vec());
        }

        assert_eq!(
            mirror.changed_paths(),
            vec![
                VfsFileEffect {
                    path: "added.txt".to_owned(),
                    status: VfsFileEffectStatus::Added,
                },
                VfsFileEffect {
                    path: "deleted.txt".to_owned(),
                    status: VfsFileEffectStatus::Deleted,
                },
                VfsFileEffect {
                    path: "modified.txt".to_owned(),
                    status: VfsFileEffectStatus::Modified,
                },
            ]
        );
    }

    #[cfg(feature = "fuse")]
    #[test]
    fn mounted_workspace_supports_basic_shell_tools_when_enabled() {
        if std::env::var("ANVICS_RUN_FUSE_TESTS").ok().as_deref() != Some("1") {
            eprintln!("skipping real FUSE test; set ANVICS_RUN_FUSE_TESTS=1 to run it");
            return;
        }

        let source = tempfile::tempdir().unwrap();
        let mount = tempfile::tempdir().unwrap();
        fs::write(source.path().join("app.txt"), "base\n").unwrap();
        fs::write(source.path().join("delete.txt"), "delete\n").unwrap();

        let mounted = mount_workspace(source.path(), mount.path()).unwrap();
        let status = Command::new("sh")
            .arg("-c")
            .arg("ls && cat app.txt && printf 'changed\\n' > app.txt && printf 'new\\n' > added.txt && rm delete.txt")
            .current_dir(mounted.mount_path())
            .status()
            .unwrap();
        assert!(status.success());

        mounted.persist_to_path(source.path()).unwrap();
        assert_eq!(
            mounted.changed_paths(),
            vec![
                VfsFileEffect {
                    path: "added.txt".to_owned(),
                    status: VfsFileEffectStatus::Added,
                },
                VfsFileEffect {
                    path: "app.txt".to_owned(),
                    status: VfsFileEffectStatus::Modified,
                },
                VfsFileEffect {
                    path: "delete.txt".to_owned(),
                    status: VfsFileEffectStatus::Deleted,
                },
            ]
        );
        assert_eq!(
            fs::read_to_string(source.path().join("app.txt")).unwrap(),
            "changed\n"
        );
        assert!(source.path().join("added.txt").exists());
        assert!(!source.path().join("delete.txt").exists());
    }
}
