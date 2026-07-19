import { useEffect, useState } from "react";

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
 * tabs when `get_permission_status()` is false. Mirrors OpenKey's
 * `AppDelegate.m: askPermission` (94-112) but as an in-window explainer with
 * an explicit restart button instead of an `NSAlert` + silent `terminate:`.
 */
export function PermissionGate() {
  const [granted, setGranted] = useState(false);

  useEffect(() => {
    const id = window.setInterval(() => {
      void getPermissionStatus().then(setGranted);
    }, POLL_INTERVAL_MS);
    return () => window.clearInterval(id);
  }, []);

  return (
    <div className="window permission-gate">
      <h2>{strings.permissionGate.title}</h2>
      <p>{strings.permissionGate.explain}</p>
      {granted && <p className="permission-gate-granted">{strings.permissionGate.granted}</p>}
      <div className="window-actions">
        {!granted && <button onClick={() => void openAccessibilitySettings()}>{strings.permissionGate.openSettings}</button>}
        <button onClick={() => void restartApp()}>{strings.permissionGate.restart}</button>
      </div>
    </div>
  );
}
