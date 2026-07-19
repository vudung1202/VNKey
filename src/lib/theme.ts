import { useEffect } from "react";

/**
 * App theme control. The app is dark-first; `styles.css` exposes the light
 * palette under `:root[data-theme="light"]`. This module resolves the user's
 * preference (persisted as `Settings.theme`) to a concrete "light"/"dark"
 * value, writes it to `documentElement.dataset.theme`, and -- when the
 * preference is "system" -- keeps it in sync with the OS live.
 *
 * The concrete value is also mirrored to localStorage so the pre-paint script
 * in index.html can restore it without a flash on the next launch.
 */
export type ThemePref = "system" | "light" | "dark";

// Index = the raw `Settings.theme` u8 (0 system, 1 light, 2 dark).
const PREFS: ThemePref[] = ["system", "light", "dark"];

export function themePrefFromSetting(value: number): ThemePref {
  return PREFS[value] ?? "system";
}

export function themeSettingFromPref(pref: ThemePref): number {
  return PREFS.indexOf(pref);
}

function prefersDark(): boolean {
  return window.matchMedia("(prefers-color-scheme: dark)").matches;
}

function apply(pref: ThemePref) {
  const resolved = pref === "system" ? (prefersDark() ? "dark" : "light") : pref;
  document.documentElement.dataset.theme = resolved;
  try {
    localStorage.setItem("vnkey-theme", resolved);
  } catch {
    // Ignore private-mode / storage-disabled failures; the theme still applies.
  }
}

/** Apply `pref` and, while it is "system", follow OS appearance changes live. */
export function useTheme(pref: ThemePref) {
  useEffect(() => {
    apply(pref);
    if (pref !== "system") return;
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const onChange = () => apply("system");
    mq.addEventListener("change", onChange);
    return () => mq.removeEventListener("change", onChange);
  }, [pref]);
}
