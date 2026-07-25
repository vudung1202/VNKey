import { useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";

import { Button, ConfirmDialog } from "../ui";
import { quitApp, restoreDefaultSettings } from "../../lib/ipc";
import { strings } from "../../lib/strings";

export function Footer() {
  const [confirmOpen, setConfirmOpen] = useState(false);

  // `restore_default_settings` persists + broadcasts `settings-changed`
  // itself; the `ControlPanel` listener picks up the reset values.
  async function doRestore() {
    setConfirmOpen(false);
    await restoreDefaultSettings();
  }

  return (
    <div className="footer">
      <Button variant="primary" onClick={() => void getCurrentWindow().hide()}>
        {strings.footer.ok}
      </Button>
      <Button variant="secondary" onClick={() => setConfirmOpen(true)}>
        {strings.footer.restoreDefault}
      </Button>
      {/* Quits directly, no confirmation. */}
      <Button variant="danger" onClick={() => void quitApp()}>
        {strings.footer.quit}
      </Button>

      <ConfirmDialog
        open={confirmOpen}
        message={strings.footer.restoreDefaultConfirm}
        confirmLabel={strings.footer.yes}
        cancelLabel={strings.footer.no}
        onConfirm={() => void doRestore()}
        onCancel={() => setConfirmOpen(false)}
      />
    </div>
  );
}
