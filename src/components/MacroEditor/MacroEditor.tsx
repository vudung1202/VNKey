import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { open, save } from "@tauri-apps/plugin-dialog";

import { Button, ConfirmDialog, SettingRow, ToastHost, showToast } from "../ui";
import { getSettings, macroAdd, macroDelete, macroExport, macroImport, macroList, patchSettings } from "../../lib/ipc";
import { strings } from "../../lib/strings";
import { themePrefFromSetting, useTheme } from "../../lib/theme";
import type { MacroEntry, Settings } from "../../lib/types";

/**
 * Port of OpenKey's `MacroViewController.mm`. One text field pair
 * (`macroName`/`macroContent`) doubles as both the add-form and the
 * edit-form: clicking a table row loads it into the fields, and the primary
 * button's label flips between "Thêm"/"Sửa" depending on whether the typed
 * key already exists (`controlTextDidChange` -> `hasMacro:`) -- there's no
 * separate "edit mode" flag, and no close/OK button in this scene.
 */
export function MacroEditor() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [macros, setMacros] = useState<MacroEntry[]>([]);
  const [text, setText] = useState("");
  const [content, setContent] = useState("");
  // Path chosen in the native file picker, awaiting the in-app keep/replace
  // choice before the actual import fires.
  const [pendingImport, setPendingImport] = useState<string | null>(null);

  useEffect(() => {
    getSettings().then(setSettings);
    macroList().then(setMacros);

    const unlistenSettings = listen<Settings>("settings-changed", (e) => setSettings(e.payload));
    const unlistenMacros = listen("macros-changed", () => {
      macroList().then(setMacros);
    });
    return () => {
      unlistenSettings.then((f) => f());
      unlistenMacros.then((f) => f());
    };
  }, []);

  const isEdit = macros.some((m) => m.text === text);

  function selectRow(m: MacroEntry) {
    setText(m.text);
    setContent(m.content);
  }

  async function handleAddOrEdit() {
    if (text === "" || content === "") {
      showToast({ kind: "error", title: strings.macroEditor.alertTitle, sub: strings.macroEditor.addNeedsBothFields });
      return;
    }
    await macroAdd(text, content);
    setText("");
    setContent("");
  }

  async function handleDelete() {
    if (text === "") {
      showToast({ kind: "error", title: strings.macroEditor.alertTitle, sub: strings.macroEditor.deleteNeedsSelection });
      return;
    }
    await macroDelete(text);
    setText("");
    setContent("");
  }

  async function handleImport() {
    const path = await open({ filters: [{ name: "Text", extensions: ["txt"] }] });
    if (!path || Array.isArray(path)) return;
    setPendingImport(path);
  }

  // Confirm ("Có") = keep existing data (append); the explicit "Không" =
  // replace. Dismissing (Esc / backdrop click) aborts the import entirely
  // rather than silently replacing, since replace wipes all current macros.
  async function resolveImport(keep: boolean) {
    const path = pendingImport;
    setPendingImport(null);
    if (path) await macroImport(path, keep);
  }

  async function handleExport() {
    const path = await save({ defaultPath: "OpenKeyMacro.txt", filters: [{ name: "Text", extensions: ["txt"] }] });
    if (!path) return;
    await macroExport(path);
  }

  useTheme(themePrefFromSetting(settings?.theme ?? 0));

  if (!settings) return null;

  return (
    <div className="window macro-editor">
      <div className="macro-table-wrap">
        <table>
          <thead>
            <tr>
              <th>{strings.macroEditor.keyColumn}</th>
              <th>{strings.macroEditor.contentColumn}</th>
            </tr>
          </thead>
          <tbody>
            {macros.map((m) => (
              <tr key={m.text} className={m.text === text ? "selected" : ""} onClick={() => selectRow(m)}>
                <td>{m.text}</td>
                <td>{m.content}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <div className="macro-form">
        <label className="field-label">
          {strings.macroEditor.keyColumn}
          <input className="ui-input" type="text" value={text} onChange={(e) => setText(e.target.value)} />
        </label>
        <label className="field-label">
          {strings.macroEditor.contentColumn}
          <textarea className="ui-textarea" rows={3} value={content} onChange={(e) => setContent(e.target.value)} />
        </label>
      </div>

      <div className="macro-actions">
        <Button variant="primary" onClick={() => void handleAddOrEdit()}>
          {isEdit ? strings.macroEditor.edit : strings.macroEditor.add}
        </Button>
        <Button variant="danger" onClick={() => void handleDelete()}>
          {strings.macroEditor.delete}
        </Button>
        <Button variant="secondary" onClick={() => void handleImport()}>
          {strings.macroEditor.importFile}
        </Button>
        <Button variant="secondary" onClick={() => void handleExport()}>
          {strings.macroEditor.exportFile}
        </Button>
      </div>

      <div className="ui-card">
        <SettingRow
          label={strings.macroTab.autoCapsMacro}
          checked={settings.auto_caps_macro}
          onChange={(v) => void patchSettings(settings, { auto_caps_macro: v })}
        />
      </div>

      <ConfirmDialog
        open={pendingImport !== null}
        title={strings.macroEditor.importDialogTitle}
        message={strings.macroEditor.importKeepDataQuestion}
        confirmLabel={strings.footer.yes}
        cancelLabel={strings.footer.no}
        onConfirm={() => void resolveImport(true)}
        onCancel={() => void resolveImport(false)}
        onDismiss={() => setPendingImport(null)}
      />
      <ToastHost />
    </div>
  );
}
