import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";

import "@fontsource-variable/geist/wght.css";
import "@fontsource-variable/geist-mono/wght.css";

import { About } from "./components/About/About";
import { ControlPanel } from "./components/ControlPanel/ControlPanel";
import { ConvertTool } from "./components/ConvertTool/ConvertTool";
import { MacroEditor } from "./components/MacroEditor/MacroEditor";
import "./styles.css";

const label = getCurrentWindow().label;

function Root() {
  switch (label) {
    case "settings":
      return <ControlPanel />;
    case "macro":
      return <MacroEditor />;
    case "convert":
      return <ConvertTool />;
    case "about":
      return <About />;
    default:
      return <p>{label}</p>;
  }
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <Root />
  </React.StrictMode>,
);
