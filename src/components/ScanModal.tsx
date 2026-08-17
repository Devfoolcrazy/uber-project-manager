import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import type { ScanCandidate } from "../types";
import { createProject, scanDirectory, setSetting } from "../api";
import { shortenPath } from "../utils";

interface Props {
  defaultRoots: string[];
  onDone: (addedCount: number) => void;
  onClose: () => void;
}

export default function ScanModal({ defaultRoots, onDone, onClose }: Props) {
  const [roots, setRoots] = useState<string[]>(defaultRoots);
  const [candidates, setCandidates] = useState<ScanCandidate[] | null>(null);
  const [checked, setChecked] = useState<Set<string>>(new Set());
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);

  const addRoot = async () => {
    const dir = await open({ directory: true });
    if (typeof dir === "string" && !roots.includes(dir)) {
      setRoots([...roots, dir]);
    }
  };

  const removeRoot = (root: string) =>
    setRoots(roots.filter((r) => r !== root));

  const scan = async () => {
    if (roots.length === 0) return;
    setError("");
    setBusy(true);
    try {
      const errors: string[] = [];
      const byPath = new Map<string, ScanCandidate>();
      for (const root of roots) {
        try {
          for (const c of await scanDirectory(root)) {
            byPath.set(c.path, c);
          }
        } catch (e) {
          errors.push(`${shortenPath(root)} : ${e}`);
        }
      }
      const merged = [...byPath.values()].sort((a, b) =>
        a.name.toLowerCase().localeCompare(b.name.toLowerCase()),
      );
      setCandidates(merged);
      setChecked(new Set());
      setError(errors.join(" — "));
      await setSetting("scan_roots", JSON.stringify(roots));
    } finally {
      setBusy(false);
    }
  };

  const toggle = (path: string) => {
    const next = new Set(checked);
    next.has(path) ? next.delete(path) : next.add(path);
    setChecked(next);
  };

  const addSelection = async () => {
    if (!candidates) return;
    setBusy(true);
    try {
      let added = 0;
      for (const c of candidates) {
        if (!checked.has(c.path)) continue;
        await createProject({
          name: c.name,
          description: "",
          category: "",
          tags: [],
          path: c.path,
          repo: "",
          status: "active",
        });
        added++;
      }
      onDone(added);
    } catch (e) {
      setError(String(e));
      setBusy(false);
    }
  };

  const fresh = candidates?.filter((c) => !c.already_registered) ?? [];
  const known = candidates?.filter((c) => c.already_registered) ?? [];

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal modal-wide" onClick={(e) => e.stopPropagation()}>
        <h2 className="modal-title">Scanner des dossiers</h2>
        <p className="modal-hint">
          Liste les sous-dossiers de chaque dossier racine et te laisse
          référencer ceux qui manquent. Les racines sont mémorisées.
        </p>

        <div className="field">
          <span className="field-label">Dossiers racines</span>
          {roots.length === 0 ? (
            <span className="field-hint">Aucun dossier racine défini.</span>
          ) : (
            <ul className="root-list">
              {roots.map((root) => (
                <li key={root} className="root-row">
                  <span className="scan-path root-path">
                    {shortenPath(root)}
                  </span>
                  <button
                    className="ghost-btn"
                    title="Retirer cette racine"
                    onClick={() => removeRoot(root)}
                  >
                    ✕
                  </button>
                </li>
              ))}
            </ul>
          )}
          <div className="field-row">
            <button type="button" className="secondary-btn" onClick={addRoot}>
              Ajouter un dossier racine…
            </button>
            <button
              type="button"
              className="primary-btn"
              onClick={scan}
              disabled={busy || roots.length === 0}
            >
              Scanner ({roots.length})
            </button>
          </div>
        </div>

        {error && <p className="form-error">{error}</p>}

        {candidates && (
          <div className="scan-results">
            {fresh.length === 0 ? (
              <p className="modal-hint">
                Rien de nouveau : tous les dossiers trouvés sont déjà
                référencés.
              </p>
            ) : (
              <>
                <div className="scan-results-bar">
                  <span>
                    {fresh.length} dossier{fresh.length > 1 ? "s" : ""} non
                    référencé{fresh.length > 1 ? "s" : ""}
                  </span>
                  <button
                    type="button"
                    className="ghost-btn"
                    onClick={() =>
                      setChecked(
                        checked.size === fresh.length
                          ? new Set()
                          : new Set(fresh.map((c) => c.path)),
                      )
                    }
                  >
                    {checked.size === fresh.length
                      ? "Tout décocher"
                      : "Tout cocher"}
                  </button>
                </div>
                <ul className="scan-list">
                  {fresh.map((c) => (
                    <li key={c.path}>
                      <label className="scan-item">
                        <input
                          type="checkbox"
                          checked={checked.has(c.path)}
                          onChange={() => toggle(c.path)}
                        />
                        <span className="scan-name">{c.name}</span>
                        {c.has_git && <span className="scan-git">git</span>}
                        <span className="scan-path">{shortenPath(c.path)}</span>
                      </label>
                    </li>
                  ))}
                </ul>
              </>
            )}
            {known.length > 0 && (
              <p className="modal-hint">
                {known.length} dossier{known.length > 1 ? "s" : ""} déjà
                référencé{known.length > 1 ? "s" : ""} (ignoré
                {known.length > 1 ? "s" : ""}).
              </p>
            )}
          </div>
        )}

        <div className="modal-footer">
          <button type="button" className="secondary-btn" onClick={onClose}>
            Fermer
          </button>
          <button
            type="button"
            className="primary-btn"
            disabled={busy || checked.size === 0}
            onClick={addSelection}
          >
            Ajouter la sélection ({checked.size})
          </button>
        </div>
      </div>
    </div>
  );
}
