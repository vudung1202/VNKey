import { Window } from "@tauri-apps/api/window";

import { Button, SettingRow } from "../ui";
import { patchSettings } from "../../lib/ipc";
import { strings } from "../../lib/strings";
import type { Settings } from "../../lib/types";

const FIELDS: Array<[string, keyof Settings]> = [
  [strings.macroTab.useMacro, "use_macro"],
  [strings.macroTab.quickTelex, "quick_telex"],
  [strings.macroTab.useMacroInEnglishMode, "use_macro_in_english_mode"],
  [strings.macroTab.autoCapsMacro, "auto_caps_macro"],
  [strings.macroTab.quickStartConsonant, "quick_start_consonant"],
  [strings.macroTab.quickEndConsonant, "quick_end_consonant"],
];

async function openMacroWindow() {
  const win = await Window.getByLabel("macro");
  await win?.show();
  await win?.setFocus();
}

export function MacroTab({ settings }: { settings: Settings }) {
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
      <Button variant="secondary" onClick={() => void openMacroWindow()}>
        {strings.macroTab.openMacroTable}
      </Button>
    </div>
  );
}
