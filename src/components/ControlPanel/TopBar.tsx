import { useEffect, useState } from "react";

import { HotkeyChips } from "../HotkeyChips";
import { Button, Segmented, Select, Toggle } from "../ui";
import { decodeHotkey, encodeHotkey } from "../../lib/hotkey";
import { getPermissionStatus, openAccessibilitySettings, setCodeTable, setInputType, setLanguage, setSwitchHotkey } from "../../lib/ipc";
import { strings } from "../../lib/strings";
import type { Settings } from "../../lib/types";

export function TopBar({ settings }: { settings: Settings }) {
  const [granted, setGranted] = useState<boolean | null>(null);

  useEffect(() => {
    function check() {
      getPermissionStatus().then(setGranted);
    }
    check();
    window.addEventListener("focus", check);
    return () => window.removeEventListener("focus", check);
  }, []);

  const hotkey = decodeHotkey(settings.switch_key_status);

  function updateHotkey(patch: Partial<typeof hotkey>) {
    void setSwitchHotkey(encodeHotkey({ ...hotkey, ...patch }));
  }

  return (
    <header className="cp-header">
      <div className="cp-field-row">
        <span className="cp-field-row__label">{strings.topBar.mode}</span>
        <Segmented
          ariaLabel={strings.topBar.mode}
          value={settings.language}
          onChange={(v) => void setLanguage(v)}
          items={[
            { value: 1, label: strings.topBar.vietnamese, accent: true },
            { value: 0, label: strings.topBar.english, accent: true },
          ]}
        />
      </div>

      <div className="cp-fields">
        <div className="cp-field-row">
          <span className="cp-field-row__label">{strings.topBar.inputType}</span>
          <Select value={settings.input_type} onChange={(e) => void setInputType(Number(e.target.value))}>
            {strings.inputTypes.map((label, i) => (
              <option key={i} value={i}>
                {label}
              </option>
            ))}
          </Select>
        </div>

        <div className="cp-field-row">
          <span className="cp-field-row__label">{strings.topBar.codeTable}</span>
          <Select value={settings.code_table} onChange={(e) => void setCodeTable(Number(e.target.value))}>
            {strings.codeTables.map((label, i) => (
              <option key={i} value={i}>
                {label}
              </option>
            ))}
          </Select>
        </div>
      </div>

      <div className="cp-hotkey">
        <div className="cp-hotkey__row">
          <span className="cp-field-row__label">{strings.topBar.switchHotkey}</span>
          <HotkeyChips
            mods={hotkey}
            char={hotkey.char}
            onToggle={(key) => updateHotkey({ [key]: !hotkey[key] } as Partial<typeof hotkey>)}
            onRecord={(keycode, char) => updateHotkey({ keycode, char })}
          />
        </div>

        <label className="cp-inline-toggle">
          <span>{strings.topBar.beep}</span>
          <Toggle checked={hotkey.beep} onChange={(v) => updateHotkey({ beep: v })} ariaLabel={strings.topBar.beep} />
        </label>
      </div>

      {granted === false && (
        <div className="ui-banner ui-banner--warning">
          <span>{strings.topBar.permissionMissing}</span>
          <Button variant="secondary" onClick={() => void openAccessibilitySettings()} style={{ marginLeft: "auto" }}>
            {strings.topBar.grantPermission}
          </Button>
        </div>
      )}
    </header>
  );
}
