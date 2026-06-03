import { mount, StartClient } from "@solidjs/start/client";

function start() {
  mount(() => <StartClient />, document.getElementById("app")!);
}

start();

export default start;
