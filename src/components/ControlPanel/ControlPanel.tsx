import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { confirm } from "@tauri-apps/plugin-dialog";

import { getPermissionStatus, getSettings, hasOpenKeyData, importOpenKeySettings, isFirstRun } from "../../lib/ipc";
import { strings } from "../../lib/strings";
import type { Settings } from "../../lib/types";
import { PermissionGate } from "../Permission/PermissionGate";
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
  const [permissionGranted, setPermissionGranted] = useState<boolean | null>(null);
  const importOfferAsked = useRef(false);

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

  // Task 15 first-run OpenKey import offer: `isFirstRun()` reflects whether
  // `settings.json` existed before THIS launch (`state::FirstRun`, captured
  // in `lib.rs`'s `setup` before `Settings::load()` would auto-create it),
  // so this only ever fires once. Held off until permission is confirmed
  // granted -- the gate below takes priority, same as `should_auto_show_
  // settings` on the Rust side.
  useEffect(() => {
    if (permissionGranted !== true || importOfferAsked.current) return;
    importOfferAsked.current = true;
    void (async () => {
      const [firstRun, openKeyData] = await Promise.all([isFirstRun(), hasOpenKeyData()]);
      if (!firstRun || !openKeyData) return;
      const yes = await confirm(strings.firstRun.offer, {
        title: strings.firstRun.title,
        okLabel: strings.footer.yes,
        cancelLabel: strings.footer.no,
      });
      if (yes) {
        // Applies + persists + reloads the engine server-side; mirror the
        // result into local state the same way `settings-changed` does.
        setSettings(await importOpenKeySettings());
      }
    })();
  }, [permissionGranted]);

  if (permissionGranted === false) {
    return <PermissionGate />;
  }

  if (!settings || permissionGranted === null) return null;

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
