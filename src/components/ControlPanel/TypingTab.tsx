import { SettingRow } from "../ui";
import { patchSettings } from "../../lib/ipc";
import { strings } from "../../lib/strings";
import type { Settings } from "../../lib/types";

const FIELDS: Array<[string, keyof Settings]> = [
  [strings.typingTab.modernOrthography, "modern_orthography"],
  [strings.typingTab.fixRecommendBrowser, "fix_recommend_browser"],
  [strings.typingTab.checkSpelling, "check_spelling"],
  [strings.typingTab.restoreIfWrongSpelling, "restore_if_wrong_spelling"],
  [strings.typingTab.allowConsonantZfwj, "allow_consonant_zfwj"],
  [strings.typingTab.tempOffSpelling, "temp_off_spelling"],
  [strings.typingTab.useSmartSwitchKey, "use_smart_switch_key"],
  [strings.typingTab.upperCaseFirstChar, "upper_case_first_char"],
  [strings.typingTab.rememberCode, "remember_code"],
  [strings.typingTab.tempOffOpenKey, "temp_off_openkey"],
  [strings.typingTab.otherLanguage, "other_language"],
];

export function TypingTab({ settings }: { settings: Settings }) {
  return (
    <div className="tab-panel">
      <div className="ui-card">
        {FIELDS.map(([label, field]) => (
          <SettingRow
            key={field}
            label={label}
            checked={settings[field] as boolean}
            onChange={(checked) => void patchSettings(settings, { [field]: checked } as Partial<Settings>)}
          />
        ))}
      </div>
    </div>
  );
}
