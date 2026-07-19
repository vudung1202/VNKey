import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

import { HotkeyChips } from "../HotkeyChips";
import { Button, Select, SettingRow, ToastHost, Toggle, showToast } from "../ui";
import { decodeHotkey, encodeHotkey } from "../../lib/hotkey";
import { convertClipboard, getSettings, patchSettings, setConvertHotkey } from "../../lib/ipc";
import { strings } from "../../lib/strings";
import type { ConvertOptions, Settings } from "../../lib/types";

type CapsField = "convert_to_all_caps" | "convert_to_all_non_caps" | "convert_to_caps_first_letter" | "convert_to_caps_each_word";

/**
 * Port of OpenKey's `ConvertToolViewController.mm`. The 4 caps toggles are
 * mutually exclusive but not mandatory: `turnOffAllOption` zeroes all 4, then
 * the clicked toggle's new value is set -- so unchecking the active one just
 * leaves all 4 off ("at most one", not "always exactly one").
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
        showToast({ title: strings.convertTool.successTitle, sub: strings.convertTool.successSub });
      }
    } else {
      showToast({ kind: "error", title: strings.convertTool.failTitle, sub: strings.convertTool.failSub });
    }
  };

  const hotkey = decodeHotkey(settings.convert_hotkey);
  const updateHotkey = (patchValue: Partial<typeof hotkey>) => {
    void setConvertHotkey(encodeHotkey({ ...hotkey, ...patchValue }));
  };

  return (
    <div className="window convert-tool">
      <div className="ui-card">
        <div className="ui-card__title">{strings.convertTool.generalOptions}</div>
        <SettingRow
          label={strings.convertTool.toAllCaps}
          checked={settings.convert_to_all_caps}
          onChange={(checked) => toggleCaps("convert_to_all_caps", checked)}
        />
        <SettingRow
          label={strings.convertTool.toAllNonCaps}
          checked={settings.convert_to_all_non_caps}
          onChange={(checked) => toggleCaps("convert_to_all_non_caps", checked)}
        />
        <SettingRow
          label={strings.convertTool.toCapsFirstLetter}
          checked={settings.convert_to_caps_first_letter}
          onChange={(checked) => toggleCaps("convert_to_caps_first_letter", checked)}
        />
        <SettingRow
          label={strings.convertTool.toCapsEachWord}
          checked={settings.convert_to_caps_each_word}
          onChange={(checked) => toggleCaps("convert_to_caps_each_word", checked)}
        />
        <SettingRow
          label={strings.convertTool.removeMark}
          checked={settings.convert_remove_mark}
          onChange={(checked) => patch({ convert_remove_mark: checked })}
        />
        <SettingRow
          label={strings.convertTool.notifyWhenDone}
          checked={!settings.convert_dont_alert}
          onChange={(checked) => patch({ convert_dont_alert: !checked })}
        />
      </div>

      <div className="ui-card">
        <div className="ui-card__title">{strings.convertTool.options}</div>
        <div className="convert-section">
          <div className="code-row">
            <label className="field-label">
              {strings.convertTool.fromCode}
              <Select value={settings.convert_from_code} onChange={(e) => patch({ convert_from_code: Number(e.target.value) })}>
                {strings.codeTables.map((label, i) => (
                  <option key={i} value={i}>
                    {label}
                  </option>
                ))}
              </Select>
            </label>
            <Button variant="secondary" aria-label="Đảo bảng mã nguồn và đích" onClick={handleReverse}>
              {strings.convertTool.reverse}
            </Button>
            <label className="field-label">
              {strings.convertTool.toCode}
              <Select value={settings.convert_to_code} onChange={(e) => patch({ convert_to_code: Number(e.target.value) })}>
                {strings.codeTables.map((label, i) => (
                  <option key={i} value={i}>
                    {label}
                  </option>
                ))}
              </Select>
            </label>
          </div>

          <div className="cp-hotkey__row">
            <span className="cp-field-row__label">{strings.convertTool.quickHotkey}</span>
            <HotkeyChips
              mods={hotkey}
              char={hotkey.char}
              onToggle={(key) => updateHotkey({ [key]: !hotkey[key] } as Partial<typeof hotkey>)}
              onRecord={(keycode, char) => updateHotkey({ keycode, char })}
            />
          </div>

          <label className="cp-inline-toggle">
            <span className="muted">{strings.convertTool.clipboardCheckbox}</span>
            <Toggle checked disabled onChange={() => {}} ariaLabel={strings.convertTool.clipboardCheckbox} />
          </label>
        </div>
      </div>

      <div className="window-actions">
        <Button variant="primary" onClick={() => void handleConvert()}>
          {strings.convertTool.convert}
        </Button>
        <Button variant="ghost" onClick={() => void getCurrentWindow().hide()}>
          {strings.convertTool.close}
        </Button>
      </div>

      <ToastHost />
    </div>
  );
}
