import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";

import { getSettings } from "../../lib/ipc";
import { strings } from "../../lib/strings";
import type { Settings } from "../../lib/types";
import { AboutTab } from "./AboutTab";
import { Footer } from "./Footer";
import { MacroTab } from "./MacroTab";
import { SystemTab } from "./SystemTab";
import { TopBar } from "./TopBar";
import { TypingTab } from "./TypingTab";

type TabKey = "typing" | "macro" | "system" | "about";
const TAB_KEYS: TabKey[] = ["typing", "macro", "system", "about"];

export function ControlPanel() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [tab, setTab] = useState<TabKey>("typing");

  useEffect(() => {
    getSettings().then(setSettings);

    // Every mutating IPC command persists + broadcasts `settings-changed`
    // server-side (`apply_and_broadcast`); the UI stays a passive mirror of
    // that broadcast instead of hand-maintaining local state, so it also
    // picks up tray/hotkey-driven changes for free.
    const unlisten = listen<Settings>("settings-changed", (event) => {
      setSettings(event.payload);
    });
    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  if (!settings) return null;

  return (
    <div className="control-panel">
      <TopBar settings={settings} />

      <nav className="tab-bar">
        {TAB_KEYS.map((key) => (
          <button key={key} className={key === tab ? "active" : ""} onClick={() => setTab(key)}>
            {strings.tabs[key]}
          </button>
        ))}
      </nav>

      <div className="tab-content">
        {tab === "typing" && <TypingTab settings={settings} />}
        {tab === "macro" && <MacroTab settings={settings} />}
        {tab === "system" && <SystemTab settings={settings} />}
        {tab === "about" && <AboutTab />}
      </div>

      <Footer />
    </div>
  );
}
