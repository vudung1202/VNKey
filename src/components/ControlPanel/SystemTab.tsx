import { useState } from "react";

import { Button, SettingRow } from "../ui";
import { checkUpdate, patchSettings } from "../../lib/ipc";
import { strings } from "../../lib/strings";
import type { Settings } from "../../lib/types";

export function SystemTab({ settings }: { settings: Settings }) {
  const [checking, setChecking] = useState(false);

  function patch(field: keyof Settings, value: boolean) {
    void patchSettings(settings, { [field]: value } as Partial<Settings>);
  }

  async function handleCheckUpdate() {
    setChecking(true);
    const result = await checkUpdate();
    setChecking(false);
    if (result) {
      window.alert(`Phiên bản mới: ${result.version}\n${result.url}`);
    } else {
      window.alert("Bạn đang dùng bản mới nhất.");
    }
  }

  return (
    <div className="tab-panel">
      <div className="ui-card">
        <SettingRow label={strings.systemTab.showIconOnDock} checked={settings.show_icon_on_dock} onChange={(v) => patch("show_icon_on_dock", v)} />
        <SettingRow label={strings.systemTab.showUiOnStartup} checked={settings.show_ui_on_startup} onChange={(v) => patch("show_ui_on_startup", v)} />
        <SettingRow label={strings.systemTab.grayIcon} checked={settings.gray_icon} onChange={(v) => patch("gray_icon", v)} />
        <SettingRow label={strings.systemTab.runOnStartup} checked={settings.run_on_startup} onChange={(v) => patch("run_on_startup", v)} />
        <SettingRow
          label={strings.systemTab.sendKeyStepByStep}
          checked={settings.send_key_step_by_step}
          onChange={(v) => patch("send_key_step_by_step", v)}
        />
        <SettingRow
          label={strings.systemTab.checkUpdateOnStartup}
          checked={!settings.dont_check_update}
          onChange={(v) => patch("dont_check_update", !v)}
        />
        <SettingRow
          label={strings.systemTab.fixChromiumBrowser}
          checked={settings.fix_chromium_browser}
          onChange={(v) => patch("fix_chromium_browser", v)}
        />
        <SettingRow
          label={strings.systemTab.performLayoutCompat}
          checked={settings.perform_layout_compat}
          onChange={(v) => patch("perform_layout_compat", v)}
        />
      </div>
      <Button variant="secondary" disabled={checking} onClick={() => void handleCheckUpdate()}>
        {checking ? strings.systemTab.checking : strings.systemTab.checkUpdate}
      </Button>
    </div>
  );
}
