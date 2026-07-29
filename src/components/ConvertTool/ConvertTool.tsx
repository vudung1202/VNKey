import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

import { ConvertTab } from "../ControlPanel/ConvertTab";
import { ToastHost } from "../ui";
import { getSettings } from "../../lib/ipc";
import { themePrefFromSetting, useTheme } from "../../lib/theme";
import type { Settings } from "../../lib/types";

/**
 * The standalone `convert` window: window chrome around the shared
 * [`ConvertTab`] body.
 *
 * Nothing opens this window any more -- the status-bar menu's "Công cụ chuyển
 * mã..." entry was removed when the menu was trimmed, and the Control Panel's
 * "Chuyển mã" tab took its place. Kept (rather than deleted along with the
 * menu entry) because the window is still declared in `tauri.conf.json` and
 * routed in `main.tsx`; deleting the feature outright is a product call, not a
 * side effect of shortening a menu.
 */
export function ConvertTool() {
  const [settings, setSettings] = useState<Settings | null>(null);

  useEffect(() => {
    getSettings().then(setSettings);
    const unlisten = listen<Settings>("settings-changed", (e) => setSettings(e.payload));
    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  useTheme(themePrefFromSetting(settings?.theme ?? 0));

  if (!settings) return null;

  return (
    <div className="window convert-tool">
      <ConvertTab settings={settings} onClose={() => void getCurrentWindow().hide()} />
      <ToastHost />
    </div>
  );
}
