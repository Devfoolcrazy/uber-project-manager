import { useState } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import type { Settings } from "../types";
import { exportData, revealDatabase, setSetting } from "../api";

interface Props {
  settings: Settings;
  onSaved: () => void;
  onClose: () => void;
}

export default function SettingsModal({ settings, onSaved, onClose }: Props) {
  const [ideCommand, setIdeCommand] = useState(settings.ide_command ?? "code");
  const [terminalApp, setTerminalApp] = useState(
    settings.terminal_app ?? "Terminal",
  );
  const [error, setError] = useState("");
  const [exported, setExported] = useState("");

  const saveSettings = async (e: React.FormEvent) => {
    e.preventDefault();
    try {
      await setSetting("ide_command", ideCommand.trim() || "code");
      await setSetting("terminal_app", terminalApp.trim() || "Terminal");
      onSaved();
    } catch (err) {
      setError(String(err));
    }
  };

  const doExport = async () => {
    setError("");
    setExported("");
    const today = new Date().toISOString().slice(0, 10);
    const dest = await save({
      defaultPath: `uber-pm-export-${today}.json`,
      filters: [{ name: "JSON", extensions: ["json"] }],
    });
    if (typeof dest !== "string") return;
    try {
      await exportData(dest);
      setExported(`Base exportée vers ${dest}`);
    } catch (err) {
      setError(String(err));
    }
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <form
        className="modal"
        onClick={(e) => e.stopPropagation()}
        onSubmit={saveSettings}
      >
        <h2 className="modal-title">Réglages</h2>

        <label className="field">
          <span className="field-label">Commande IDE</span>
          <input
            className="mono"
            value={ideCommand}
            onChange={(e) => setIdeCommand(e.target.value)}
            placeholder="code"
          />
          <span className="field-hint">
            La commande reçoit le chemin du projet en argument. Exemples :{" "}
            <code>code</code>, <code>cursor</code>, <code>zed</code>,{" "}
            <code>open -a "Visual Studio Code"</code>
          </span>
        </label>

        <label className="field">
          <span className="field-label">Application terminal</span>
          <input
            value={terminalApp}
            onChange={(e) => setTerminalApp(e.target.value)}
            placeholder="Terminal"
          />
          <span className="field-hint">
            Nom de l'app macOS : Terminal, iTerm, Ghostty, Warp…
          </span>
        </label>

        <div className="field">
          <span className="field-label">Sauvegarde</span>
          <div className="field-row">
            <button type="button" className="secondary-btn" onClick={doExport}>
              Exporter la base (JSON)…
            </button>
            <button
              type="button"
              className="secondary-btn"
              onClick={() => revealDatabase().catch((e) => setError(String(e)))}
            >
              Révéler la base dans le Finder
            </button>
          </div>
          <span className="field-hint">
            L'export contient projets, tags, catégories, changelogs et
            réglages.
          </span>
        </div>

        {exported && <p className="form-success">{exported}</p>}
        {error && <p className="form-error">{error}</p>}

        <div className="modal-footer">
          <button type="button" className="secondary-btn" onClick={onClose}>
            Annuler
          </button>
          <button type="submit" className="primary-btn">
            Enregistrer
          </button>
        </div>
      </form>
    </div>
  );
}
