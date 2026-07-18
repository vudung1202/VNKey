import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { message } from "@tauri-apps/plugin-dialog";

import { HotkeyRecorder } from "../HotkeyRecorder";
import { decodeHotkey, encodeHotkey } from "../../lib/hotkey";
import { convertClipboard, getSettings, patchSettings, setConvertHotkey } from "../../lib/ipc";
import { strings } from "../../lib/strings";
import type { ConvertOptions, Settings } from "../../lib/types";

type CapsField = "convert_to_all_caps" | "convert_to_all_non_caps" | "convert_to_caps_first_letter" | "convert_to_caps_each_word";

/**
 * Port of OpenKey's `ConvertToolViewController.mm`. The 4 caps checkboxes
 * are mutually exclusive but not mandatory: `turnOffAllOption` zeroes all
 * 4, then the clicked checkbox's new value is set -- so unchecking the
 * active one just leaves all 4 off ("at most one", not "always exactly
 * one").
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

  if (!settings) return null;

  // Arrow function expressions, not `function` declarations: TS narrows
  // `settings` to non-null here (declared `const`), but that narrowing
  // wouldn't reach hoisted function declarations referencing it.
  const patch = (patchValue: Partial<Settings>) => {
    void patchSettings(settings, patchValue);
  };

  const toggleCaps = (field: CapsField, checked: boolean) => {
    patch({
      convert_to_all_caps: false,
      convert_to_all_non_caps: false,
      convert_to_caps_first_letter: false,
      convert_to_caps_each_word: false,
      [field]: checked,
    });
  };

  const handleReverse = () => {
    patch({ convert_from_code: settings.convert_to_code, convert_to_code: settings.convert_from_code });
  };

  const handleConvert = async () => {
    const options: ConvertOptions = {
      from_code: settings.convert_from_code,
      to_code: settings.convert_to_code,
      remove_mark: settings.convert_remove_mark,
      to_all_caps: settings.convert_to_all_caps,
      to_all_non_caps: settings.convert_to_all_non_caps,
      to_caps_first_letter: settings.convert_to_caps_first_letter,
      to_caps_each_word: settings.convert_to_caps_each_word,
    };
    const converted = await convertClipboard(options);
    if (converted) {
      if (!settings.convert_dont_alert) {
        await message(strings.convertTool.successSub, { title: strings.convertTool.successTitle });
      }
    } else {
      await message(strings.convertTool.failSub, { title: strings.convertTool.failTitle });
    }
  };

  const hotkey = decodeHotkey(settings.convert_hotkey);
  const updateHotkey = (patchValue: Partial<typeof hotkey>) => {
    void setConvertHotkey(encodeHotkey({ ...hotkey, ...patchValue }));
  };

  return (
    <div className="window convert-tool">
      <fieldset className="group-box">
        <legend>{strings.convertTool.generalOptions}</legend>
        <div className="checkbox-grid">
          <label className="checkbox-row">
            <input type="checkbox" checked={settings.convert_to_all_caps} onChange={(e) => toggleCaps("convert_to_all_caps", e.target.checked)} />
            <span>{strings.convertTool.toAllCaps}</span>
          </label>
          <label className="checkbox-row">
            <input
              type="checkbox"
              checked={settings.convert_to_all_non_caps}
              onChange={(e) => toggleCaps("convert_to_all_non_caps", e.target.checked)}
            />
            <span>{strings.convertTool.toAllNonCaps}</span>
          </label>
          <label className="checkbox-row">
            <input
              type="checkbox"
              checked={settings.convert_to_caps_first_letter}
              onChange={(e) => toggleCaps("convert_to_caps_first_letter", e.target.checked)}
            />
            <span>{strings.convertTool.toCapsFirstLetter}</span>
          </label>
          <label className="checkbox-row">
            <input
              type="checkbox"
              checked={settings.convert_to_caps_each_word}
              onChange={(e) => toggleCaps("convert_to_caps_each_word", e.target.checked)}
            />
            <span>{strings.convertTool.toCapsEachWord}</span>
          </label>
          <label className="checkbox-row">
            <input type="checkbox" checked={settings.convert_remove_mark} onChange={(e) => patch({ convert_remove_mark: e.target.checked })} />
            <span>{strings.convertTool.removeMark}</span>
          </label>
          <label className="checkbox-row">
            <input type="checkbox" checked={!settings.convert_dont_alert} onChange={(e) => patch({ convert_dont_alert: !e.target.checked })} />
            <span>{strings.convertTool.notifyWhenDone}</span>
          </label>
        </div>
      </fieldset>

      <fieldset className="group-box">
        <legend>{strings.convertTool.options}</legend>
        <div className="code-row">
          <label>
            {strings.convertTool.fromCode}
            <select value={settings.convert_from_code} onChange={(e) => patch({ convert_from_code: Number(e.target.value) })}>
              {strings.codeTables.map((label, i) => (
                <option key={i} value={i}>
                  {label}
                </option>
              ))}
            </select>
          </label>
          <button onClick={handleReverse}>{strings.convertTool.reverse}</button>
          <label>
            {strings.convertTool.toCode}
            <select value={settings.convert_to_code} onChange={(e) => patch({ convert_to_code: Number(e.target.value) })}>
              {strings.codeTables.map((label, i) => (
                <option key={i} value={i}>
                  {label}
                </option>
              ))}
            </select>
          </label>
        </div>

        <span className="hotkey-group">
          {strings.convertTool.quickHotkey}
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
        </span>

        <label className="checkbox-row disabled-checkbox">
          <input type="checkbox" checked readOnly disabled />
          <span>{strings.convertTool.clipboardCheckbox}</span>
        </label>
      </fieldset>

      <div className="window-actions">
        <button onClick={() => void handleConvert()}>{strings.convertTool.convert}</button>
        <button onClick={() => void getCurrentWindow().hide()}>{strings.convertTool.close}</button>
      </div>
    </div>
  );
}
