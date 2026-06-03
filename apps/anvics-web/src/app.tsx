import { FileRoutes } from "@solidjs/start/router";
import { Router } from "@solidjs/router";
import "./styles.css";

export default function App() {
  return (
    <Router root={(props) => <main>{props.children}</main>}>
      <FileRoutes />
    </Router>
  );
}
