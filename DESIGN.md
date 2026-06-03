---
name: Anvics
description: Agent-native source control for humans supervising AI coding agents.
colors:
  ink: "#20231f"
  body-text: "#51584f"
  muted-text: "#687166"
  label-text: "#66715f"
  app-bg: "#f7f7f4"
  header-bg: "#fbfbf8"
  panel-bg: "#ffffff"
  panel-soft: "#fbfbf8"
  border: "#d8d8cf"
  border-soft: "#ecece4"
  hover: "#f4f6f1"
  selected: "#eef2e8"
  status-neutral-bg: "#eef0e8"
  status-neutral-text: "#566052"
  status-success-bg: "#e3f1df"
  status-success-text: "#38633c"
  status-risk-bg: "#fde8df"
  status-risk-text: "#8a341f"
typography:
  title:
    fontFamily: "Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, Segoe UI, sans-serif"
    fontSize: "1.25rem"
    fontWeight: 600
    lineHeight: 1.2
    letterSpacing: "0"
  body:
    fontFamily: "Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, Segoe UI, sans-serif"
    fontSize: "0.875rem"
    fontWeight: 400
    lineHeight: 1.5
    letterSpacing: "0"
  label:
    fontFamily: "Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, Segoe UI, sans-serif"
    fontSize: "0.75rem"
    fontWeight: 600
    lineHeight: 1.3
    letterSpacing: "0"
rounded:
  sm: "4px"
  md: "6px"
spacing:
  xs: "4px"
  sm: "8px"
  md: "12px"
  lg: "16px"
  xl: "20px"
  page-x: "24px"
components:
  button-secondary:
    backgroundColor: "{colors.panel-bg}"
    textColor: "{colors.ink}"
    rounded: "{rounded.md}"
    padding: "8px 12px"
  status-published:
    backgroundColor: "{colors.status-success-bg}"
    textColor: "{colors.status-success-text}"
    rounded: "{rounded.sm}"
    padding: "4px 8px"
  status-risk:
    backgroundColor: "{colors.status-risk-bg}"
    textColor: "{colors.status-risk-text}"
    rounded: "{rounded.sm}"
    padding: "4px 8px"
---

# Design System: Anvics

## 1. Overview

**Creative North Star: "The Review Bench"**

Anvics should look like a focused workbench for evaluating agent output. The visual system is restrained and utilitarian: compact panels, clear borders, low-shadow surfaces, and labels that make provenance easy to scan. The interface should feel quiet enough for repeated use, but not anonymous. It is a product UI for human supervision, not a marketing surface and not an agent control room.

The current local Review Inbox uses a soft neutral canvas, white content panels, green-gray selection states, and explicit risk/publication badges. This is the right direction for private beta: calm, readable, and operational.

**Key Characteristics:**

- Dense but not cramped.
- High-trust copy and labels.
- Borders and tonal layers instead of decorative shadows.
- Semantic status color used sparingly.
- Git concepts absent from primary navigation.

## 2. Colors

The palette is a restrained product palette: cool off-white surfaces, dark neutral text, muted green-gray selection states, and warm risk color only when needed.

### Primary

- **Workbench Ink** (#20231f): primary text, headings, and important labels.
- **Operator Green** (#38633c): successful publication and accepted state text.
- **Risk Rust** (#8a341f): secret-risk or blocked-state text.

### Neutral

- **Canvas Neutral** (#f7f7f4): page background.
- **Header Neutral** (#fbfbf8): header and low-emphasis surface fill.
- **Panel White** (#ffffff): main content surfaces.
- **Divider Neutral** (#d8d8cf): structural borders.
- **Soft Divider** (#ecece4): internal row and panel dividers.
- **Muted Text** (#687166): secondary text and metadata.

### Named Rules

**The Sparse Status Rule.** Status colors appear on badges, risk notes, and selected rows. They do not tint the entire app.

**The Evidence First Rule.** Risk and publication colors must accompany text labels. Color alone never carries review state.

## 3. Typography

**Display Font:** Inter/system sans stack.
**Body Font:** Inter/system sans stack.
**Label/Mono Font:** system sans for labels; mono can be introduced later for object IDs and paths.

**Character:** Type is compact, plain, and work-focused. It should support scanning thread titles, path changes, evidence summaries, and IDs without feeling like a document editor or marketing page.

### Hierarchy

- **Headline** (600, 1.5rem, 1.2): page titles such as "Review Inbox".
- **Title** (600, 1.25rem, 1.2): selected thread or review title.
- **Section Title** (600, 0.875rem, 1.3): panel headings like Changed Paths and Evidence.
- **Body** (400, 0.875rem, 1.5): task text, summaries, path labels.
- **Label** (600, 0.75rem, 1.3): metadata labels and short status text. Uppercase is allowed only for short product labels.

### Named Rules

**The Plain Label Rule.** Labels should say what the object is: thread, review, evidence, risk, publication. Do not invent metaphorical labels.

## 4. Elevation

The system is flat by default. Depth is conveyed with borders, background layers, and selection tint rather than drop shadows. This keeps the UI calm and makes dense review content easier to trust.

### Shadow Vocabulary

No persistent shadow vocabulary exists yet. If shadows are introduced later, they should be stateful and subtle: hover, active drag, or transient popover only.

### Named Rules

**The Border Before Shadow Rule.** Use a clear 1px border and tonal background before introducing any shadow.

## 5. Components

### Buttons

- **Shape:** compact rectangle with a 6px radius.
- **Primary:** not yet established. Future primary actions should use a conservative accent and clear verb-object labels.
- **Secondary:** white background, neutral border, dark text, 8px by 12px padding.
- **Hover / Focus:** use subtle surface tint for hover and a visible focus outline for keyboard users.

### Chips

- **Style:** small filled badges with 4px radius and explicit text.
- **State:** Review is neutral, Published is green, Risk is rust. Badges must stay short.

### Cards / Containers

- **Corner Style:** 6px radius.
- **Background:** white for main containers, near-white for nested information panels.
- **Shadow Strategy:** no persistent shadows.
- **Border:** 1px neutral border for structure.
- **Internal Padding:** 16px to 20px for major panels, 12px to 16px for rows.

### Inputs / Fields

Inputs are not established yet. They should inherit the button shape, use white or panel-soft backgrounds, and expose clear focus, error, disabled, and loading states.

### Navigation

The current surface uses a single page header. Future hosted/project UI should prefer direct product navigation: Inbox, Threads, Publications, Source, Events. Do not organize around Git nouns.

### Review Inbox

The Review Inbox is the signature component. It has a thread list on the left and a selected review detail on the right when space allows. It should make review state visible in under a few seconds: task, status, evidence count, changed paths, risk notes, overlap notes, and publication count.

## 6. Do's and Don'ts

### Do:

- **Do** keep the first screen task-oriented.
- **Do** show evidence, risk, and publication state with labels.
- **Do** use compact rows and stable panel dimensions for scan-heavy workflows.
- **Do** reserve color for state and selection.
- **Do** make recovery and audit paths visible when they exist.

### Don't:

- **Don't** clone GitHub navigation or commit history UI.
- **Don't** make agents the audience of the browser UI.
- **Don't** use flashy AI SaaS gradients, glass cards, or neon terminal decoration.
- **Don't** hide risk, recovery, or evidence behind generic summaries.
- **Don't** rely on beige-neutral styling alone as a brand.
- **Don't** introduce large rounded cards or nested card stacks.
