import { HotkeyChips } from "../HotkeyChips";
import { Button, Select, SettingRow, Toggle, showToast } from "../ui";
import { decodeHotkey, encodeHotkey } from "../../lib/hotkey";
import { convertClipboard, patchSettings, setConvertHotkey } from "../../lib/ipc";
import { strings } from "../../lib/strings";
import type { ConvertOptions, Settings } from "../../lib/types";

type CapsField = "convert_to_all_caps" | "convert_to_all_non_caps" | "convert_to_caps_first_letter" | "convert_to_caps_each_word";

/**
 * The convert tool's body, shared by the Control Panel's "Chuyển mã" tab (the
 * only way in since the status-bar menu was trimmed) and by the standalone
 * `convert` window, which passes `onClose` to get its extra "Đóng" button.
 * Settings come in as a prop so the tab mirrors the Control Panel's single
 * `settings-changed` subscription instead of opening a second one.
 *
 * The 4 caps toggles are mutually exclusive but not mandatory: toggling one
 * zeroes all 4 first, then sets the clicked toggle's new value -- so
 * unchecking the active one just leaves all 4 off ("at most one", not
 * "always exactly one").
 */
export function ConvertTab({ settings, onClose }: { settings: Settings; onClose?: () => void }) {
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
    <div className="tab-panel">
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
            <Button variant="secondary" aria-label={strings.convertTool.reverseAria} onClick={handleReverse}>
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
        {onClose && (
          <Button variant="ghost" onClick={onClose}>
            {strings.convertTool.close}
          </Button>
        )}
      </div>
    </div>
  );
}
