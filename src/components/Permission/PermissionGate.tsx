import { useEffect, useRef, useState } from "react";

import { Button } from "../ui";
import { getPermissionStatus, openAccessibilitySettings, restartApp } from "../../lib/ipc";
import { strings } from "../../lib/strings";

// No OS notification exists for an Accessibility grant change
// (`src-tauri/src/events.rs`'s `permission_changed` doc comment), so this
// polls while the gate is shown -- the only way to notice the user granted
// the permission in System Settings. The CGEventTap only attaches once, at
// startup (`platform::mac::start`), so granting permission alone doesn't
// make typing work; the call-to-action switches to "please restart" instead
// of silently unblocking the Control Panel.
const POLL_INTERVAL_MS = 1500;

/**
 * Full-window blocking view `ControlPanel` renders in place of the normal
 * tabs when `get_permission_status()` is false. An in-window explainer with
 * an explicit restart button, rather than a modal alert that quits the app.
 */
export function PermissionGate() {
  const [granted, setGranted] = useState(false);
  const pollId = useRef<number | undefined>(undefined);

  useEffect(() => {
    pollId.current = window.setInterval(() => {
      void getPermissionStatus().then(setGranted);
    }, POLL_INTERVAL_MS);
    return () => window.clearInterval(pollId.current);
  }, []);

  const handleRestart = () => {
    window.clearInterval(pollId.current);
    void restartApp();
  };

  return (
    <div className="window permission-gate">
      <img className="app-logo" src="/icon.png" alt="" width={72} height={72} />
      <h2>{strings.permissionGate.title}</h2>
      <p>{strings.permissionGate.explain}</p>
      {granted && <p className="permission-gate-granted">{strings.permissionGate.granted}</p>}
      <div className="window-actions">
        {!granted && (
          <Button variant="primary" onClick={() => void openAccessibilitySettings()}>
            {strings.permissionGate.openSettings}
          </Button>
        )}
        <Button variant="secondary" onClick={handleRestart}>
          {strings.permissionGate.restart}
        </Button>
      </div>
    </div>
  );
}
