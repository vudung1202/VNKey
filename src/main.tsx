import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";

import { ControlPanel } from "./components/ControlPanel/ControlPanel";
import { MacroEditor } from "./components/MacroEditor/MacroEditor";
import "./styles.css";

const label = getCurrentWindow().label;

function Root() {
  switch (label) {
    case "settings":
      return <ControlPanel />;
    case "macro":
      return <MacroEditor />;
    // `convert`/`about` are separate windows built later in Task 14.
    default:
      return <p>{label}</p>;
  }
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <Root />
  </React.StrictMode>,
);
