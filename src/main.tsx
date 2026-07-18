import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";

import { ControlPanel } from "./components/ControlPanel/ControlPanel";
import "./styles.css";

const label = getCurrentWindow().label;

function Root() {
  switch (label) {
    case "settings":
      return <ControlPanel />;
    // `macro`/`convert`/`about` are separate windows built in Task 14-15;
    // this task only wires up the `settings` (Control Panel) window.
    default:
      return <p>{label}</p>;
  }
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <Root />
  </React.StrictMode>,
);
