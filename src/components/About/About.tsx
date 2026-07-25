import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { openUrl } from "@tauri-apps/plugin-opener";

import { Button, SettingRow, ToastHost, showToast } from "../ui";
import { checkUpdate, getSettings, patchSettings } from "../../lib/ipc";
import { strings } from "../../lib/strings";
import { themePrefFromSetting, useTheme } from "../../lib/theme";
import type { Settings } from "../../lib/types";

/**
 * Content (name/description/credit/source URL) is shared with the embedded
 * `AboutTab` since it's the same information; this window adds the "Trang
 * chủ"/"Mã nguồn" links, the update-check toggle + button (same check_update
 * flow as `SystemTab`), and a Đóng button.
 */
export function About() {
  const [version, setVersion] = useState("");
  const [settings, setSettings] = useState<Settings | null>(null);
  const [checking, setChecking] = useState(false);

  useEffect(() => {
    getVersion().then(setVersion);
    getSettings().then(setSettings);
    const unlisten = listen<Settings>("settings-changed", (e) => setSettings(e.payload));
    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  useTheme(themePrefFromSetting(settings?.theme ?? 0));

  if (!settings) return null;

  const handleCheckUpdate = async () => {
    setChecking(true);
    const result = await checkUpdate();
    setChecking(false);
    if (result) {
      showToast({ title: `Phiên bản mới: ${result.version}`, sub: result.url });
    } else {
      showToast({ title: "Bạn đang dùng bản mới nhất." });
    }
  };

  return (
    <div className="window about-window">
      <img className="app-logo" src="/icon.png" alt="" width={96} height={96} />
      <h1>{strings.aboutTab.appName}</h1>
      <p className="version">{version ? `Phiên bản ${version}` : ""}</p>
      <p>{strings.aboutTab.description}</p>
      <p>{strings.aboutTab.credit}</p>

      <div className="about-links">
        <Button variant="secondary" onClick={() => void openUrl(strings.aboutTab.sourceCodeUrl)}>
          {strings.about.homePage}
        </Button>
        <Button variant="secondary" onClick={() => void openUrl(strings.aboutTab.sourceCodeUrl)}>
          {strings.aboutTab.sourceCode}
        </Button>
      </div>

      <SettingRow
        label={strings.about.checkUpdateOnStartup}
        checked={!settings.dont_check_update}
        onChange={(v) => void patchSettings(settings, { dont_check_update: !v })}
      />

      <div className="window-actions">
        <Button variant="secondary" disabled={checking} onClick={() => void handleCheckUpdate()}>
          {checking ? strings.systemTab.checking : strings.systemTab.checkUpdate}
        </Button>
        <Button variant="primary" onClick={() => void getCurrentWindow().hide()}>
          {strings.about.close}
        </Button>
      </div>

      <ToastHost />
    </div>
  );
}
