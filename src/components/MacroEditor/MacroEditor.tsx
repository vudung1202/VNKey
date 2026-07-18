import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { confirm, message, open, save } from "@tauri-apps/plugin-dialog";

import { CheckboxRow } from "../CheckboxRow";
import { getSettings, macroAdd, macroDelete, macroExport, macroImport, macroList, patchSettings } from "../../lib/ipc";
import { strings } from "../../lib/strings";
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
      await message(strings.macroEditor.addNeedsBothFields, { title: strings.macroEditor.alertTitle, kind: "error" });
      return;
    }
    await macroAdd(text, content);
    setText("");
    setContent("");
  }

  async function handleDelete() {
    if (text === "") {
      await message(strings.macroEditor.deleteNeedsSelection, { title: strings.macroEditor.alertTitle, kind: "error" });
      return;
    }
    await macroDelete(text);
    setText("");
    setContent("");
  }

  async function handleImport() {
    const path = await open({ filters: [{ name: "Text", extensions: ["txt"] }] });
    if (!path || Array.isArray(path)) return;

    const append = await confirm(strings.macroEditor.importKeepDataQuestion, {
      title: strings.macroEditor.importDialogTitle,
      okLabel: strings.footer.yes,
      cancelLabel: strings.footer.no,
    });
    await macroImport(path, append);
  }

  async function handleExport() {
    const path = await save({ defaultPath: "OpenKeyMacro.txt", filters: [{ name: "Text", extensions: ["txt"] }] });
    if (!path) return;
    await macroExport(path);
  }

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
        <label>
          {strings.macroEditor.keyColumn}
          <input type="text" value={text} onChange={(e) => setText(e.target.value)} />
        </label>
        <label>
          {strings.macroEditor.contentColumn}
          <textarea rows={3} value={content} onChange={(e) => setContent(e.target.value)} />
        </label>
      </div>

      <div className="macro-actions">
        <button onClick={() => void handleAddOrEdit()}>{isEdit ? strings.macroEditor.edit : strings.macroEditor.add}</button>
        <button onClick={() => void handleDelete()}>{strings.macroEditor.delete}</button>
        <button onClick={() => void handleImport()}>{strings.macroEditor.importFile}</button>
        <button onClick={() => void handleExport()}>{strings.macroEditor.exportFile}</button>
      </div>

      <CheckboxRow
        label={strings.macroTab.autoCapsMacro}
        checked={settings.auto_caps_macro}
        onChange={(v) => void patchSettings(settings, { auto_caps_macro: v })}
      />
    </div>
  );
}
