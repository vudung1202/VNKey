import { useEffect, useState } from "react";

import { HotkeyRecorder } from "../HotkeyRecorder";
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
    <div className="top-bar">
      <div className="top-bar-row">
        <label>
          {strings.topBar.inputType}
          <select value={settings.input_type} onChange={(e) => void setInputType(Number(e.target.value))}>
            {strings.inputTypes.map((label, i) => (
              <option key={i} value={i}>
                {label}
              </option>
            ))}
          </select>
        </label>

        <label>
          {strings.topBar.codeTable}
          <select value={settings.code_table} onChange={(e) => void setCodeTable(Number(e.target.value))}>
            {strings.codeTables.map((label, i) => (
              <option key={i} value={i}>
                {label}
              </option>
            ))}
          </select>
        </label>

        <span className="radio-group">
          {strings.topBar.mode}
          <label>
            <input type="radio" name="language" checked={settings.language === 1} onChange={() => void setLanguage(1)} />
            {strings.topBar.vietnamese}
          </label>
          <label>
            <input type="radio" name="language" checked={settings.language === 0} onChange={() => void setLanguage(0)} />
            {strings.topBar.english}
          </label>
        </span>
      </div>

      <div className="top-bar-row">
        <span className="hotkey-group">
          {strings.topBar.switchHotkey}
          <label>
            <input type="checkbox" checked={hotkey.ctrl} onChange={(e) => updateHotkey({ ctrl: e.target.checked })} />⌃
          </label>
          <label>
            <input type="checkbox" checked={hotkey.opt} onChange={(e) => updateHotkey({ opt: e.target.checked })} />⌥
          </label>
          <label>
            <input type="checkbox" checked={hotkey.cmd} onChange={(e) => updateHotkey({ cmd: e.target.checked })} />⌘
          </label>
          <label>
            <input type="checkbox" checked={hotkey.shift} onChange={(e) => updateHotkey({ shift: e.target.checked })} />⇧
          </label>
          <HotkeyRecorder char={hotkey.char} onChange={(keycode, char) => updateHotkey({ keycode, char })} />
          <label>
            <input type="checkbox" checked={hotkey.beep} onChange={(e) => updateHotkey({ beep: e.target.checked })} />
            {strings.topBar.beep}
          </label>
        </span>

        {granted === false && (
          <span className="permission-warning">
            {strings.topBar.permissionMissing}
            <button onClick={() => void openAccessibilitySettings()}>{strings.topBar.grantPermission}</button>
          </span>
        )}
      </div>
    </div>
  );
}
