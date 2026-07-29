import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";

import { getPermissionStatus, getSettings } from "../../lib/ipc";
import { strings } from "../../lib/strings";
import { themePrefFromSetting, useTheme } from "../../lib/theme";
import type { Settings } from "../../lib/types";
import { Segmented, ToastHost } from "../ui";
import { PermissionGate } from "../Permission/PermissionGate";
import { AboutTab } from "./AboutTab";
import { ConvertTab } from "./ConvertTab";
import { Footer } from "./Footer";
import { MacroTab } from "./MacroTab";
import { SystemTab } from "./SystemTab";
import { TopBar } from "./TopBar";
import { TypingTab } from "./TypingTab";

type TabKey = "typing" | "macro" | "convert" | "system" | "about";
const TAB_KEYS: TabKey[] = ["typing", "macro", "convert", "system", "about"];

export function ControlPanel() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [tab, setTab] = useState<TabKey>("typing");
  const [permissionGranted, setPermissionGranted] = useState<boolean | null>(null);

  useEffect(() => {
    getSettings().then(setSettings);
    getPermissionStatus().then(setPermissionGranted);

    // Every mutating IPC command persists + broadcasts `settings-changed`
    // server-side (`apply_and_broadcast`), and so does the tray/hotkey/
    // smart-switch live-change path (`tray::sync_after_live_change`); the UI
    // stays a passive mirror of that broadcast instead of hand-maintaining
    // local state, so it picks up tray/hotkey-driven changes for free too.
    const unlisten = listen<Settings>("settings-changed", (event) => {
      setSettings(event.payload);
    });
    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  useTheme(themePrefFromSetting(settings?.theme ?? 0));

  if (permissionGranted === false) {
    return <PermissionGate />;
  }

  if (!settings || permissionGranted === null) return null;

  return (
    <div className="control-panel">
      <TopBar settings={settings} />

      <div className="cp-tabs">
        <Segmented
          full
          ariaLabel="Tab"
          value={tab}
          onChange={setTab}
          items={TAB_KEYS.map((key) => ({ value: key, label: strings.tabs[key] }))}
        />
      </div>

      <div className="tab-content">
        {tab === "typing" && <TypingTab settings={settings} />}
        {tab === "macro" && <MacroTab settings={settings} />}
        {tab === "convert" && <ConvertTab settings={settings} />}
        {tab === "system" && <SystemTab settings={settings} />}
        {tab === "about" && <AboutTab />}
      </div>

      <Footer />

      <ToastHost />
    </div>
  );
}
