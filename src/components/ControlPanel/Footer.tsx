import { getCurrentWindow } from "@tauri-apps/api/window";
import { confirm } from "@tauri-apps/plugin-dialog";

import { quitApp, restoreDefaultSettings } from "../../lib/ipc";
import { strings } from "../../lib/strings";

async function handleRestoreDefault() {
  const yes = await confirm(strings.footer.restoreDefaultConfirm, {
    okLabel: strings.footer.yes,
    cancelLabel: strings.footer.no,
  });
  if (yes) {
    // `restore_default_settings` persists + broadcasts `settings-changed`
    // itself; the `ControlPanel` listener picks up the reset values.
    await restoreDefaultSettings();
  }
}

export function Footer() {
  return (
    <div className="footer">
      <button onClick={() => void getCurrentWindow().hide()}>{strings.footer.ok}</button>
      <button onClick={() => void handleRestoreDefault()}>{strings.footer.restoreDefault}</button>
      {/* Mirrors OpenKey's `onTerminateApp`: quits directly, no confirmation. */}
      <button onClick={() => void quitApp()}>{strings.footer.quit}</button>
    </div>
  );
}
