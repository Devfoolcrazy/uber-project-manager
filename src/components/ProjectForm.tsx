import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import type {
  Category,
  Project,
  ProjectInput,
  ProjectStatus,
  Tag,
} from "../types";
import { STATUSES } from "../utils";

interface Props {
  initial: Project | null;
  availableCategories: Category[];
  availableTags: Tag[];
  onSubmit: (input: ProjectInput) => Promise<void>;
  onOpenAdmin: () => void;
  onClose: () => void;
}

export default function ProjectForm({
  initial,
  availableCategories,
  availableTags,
  onSubmit,
  onOpenAdmin,
  onClose,
}: Props) {
  const [name, setName] = useState(initial?.name ?? "");
  const [description, setDescription] = useState(initial?.description ?? "");
  const [category, setCategory] = useState(initial?.category ?? "");
  const [tags, setTags] = useState<string[]>(initial?.tags ?? []);
  const [path, setPath] = useState(initial?.path ?? "");
  const [repo, setRepo] = useState(initial?.repo ?? "");
  const [status, setStatus] = useState<ProjectStatus>(
    initial?.status ?? "active",
  );
  const [error, setError] = useState("");

  const pickFolder = async () => {
    const dir = await open({ directory: true, defaultPath: path || undefined });
    if (typeof dir === "string") {
      setPath(dir);
      if (!name) setName(dir.split("/").filter(Boolean).pop() ?? "");
    }
  };

  const toggleTag = (tag: string) =>
    setTags((prev) =>
      prev.includes(tag) ? prev.filter((t) => t !== tag) : [...prev, tag],
    );

  const submit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim() || !path.trim()) {
      setError("Le nom et le chemin sont obligatoires.");
      return;
    }
    try {
      await onSubmit({
        name: name.trim(),
        description: description.trim(),
        category: category.trim(),
        tags,
        path: path.trim(),
        repo: repo.trim(),
        status,
      });
    } catch (err) {
      setError(String(err));
    }
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <form
        className="modal"
        onClick={(e) => e.stopPropagation()}
        onSubmit={submit}
      >
        <h2 className="modal-title">
          {initial ? "Modifier le projet" : "Nouveau projet"}
        </h2>

        <label className="field">
          <span className="field-label">Nom</span>
          <input
            autoFocus
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="mon-super-poc"
          />
        </label>

        <label className="field">
          <span className="field-label">Chemin</span>
          <div className="field-row">
            <input
              value={path}
              onChange={(e) => setPath(e.target.value)}
              placeholder="/Users/…/Projet/mon-super-poc"
              className="mono"
            />
            <button type="button" className="secondary-btn" onClick={pickFolder}>
              Parcourir…
            </button>
          </div>
          <span className="field-hint">
            Le dossier doit exister : il est vérifié à l'enregistrement.
          </span>
        </label>

        <label className="field">
          <span className="field-label">Repo / sauvegarde</span>
          <input
            className="mono"
            value={repo}
            onChange={(e) => setRepo(e.target.value)}
            placeholder="https://github.com/… ou /Volumes/Backup/…"
          />
          <span className="field-hint">
            URL d'un repo distant ou chemin d'un dossier de sauvegarde
            (facultatif).
          </span>
        </label>

        <label className="field">
          <span className="field-label">Description</span>
          <textarea
            value={description}
            rows={3}
            onChange={(e) => setDescription(e.target.value)}
            placeholder="À quoi sert ce projet, où il en est…"
          />
        </label>

        <label className="field">
          <span className="field-label">Statut</span>
          <select
            value={status}
            onChange={(e) => setStatus(e.target.value as ProjectStatus)}
          >
            {STATUSES.map((s) => (
              <option key={s.value} value={s.value}>
                {s.label}
              </option>
            ))}
          </select>
          <span className="field-hint">
            « Terminé » et « Abandonné » rangent le projet dans la vue
            Archivés.
          </span>
        </label>

        <label className="field">
          <span className="field-label">Catégorie</span>
          <select
            value={category}
            onChange={(e) => setCategory(e.target.value)}
          >
            <option value="">Sans catégorie</option>
            {availableCategories.map((c) => (
              <option key={c.id} value={c.name}>
                {c.name}
              </option>
            ))}
          </select>
          <span className="field-hint">
            <button type="button" className="link-btn" onClick={onOpenAdmin}>
              Gérer les catégories
            </button>
          </span>
        </label>

        <div className="field">
          <span className="field-label">Tags</span>
          {availableTags.length === 0 ? (
            <span className="field-hint">
              Aucun tag défini pour l'instant.{" "}
              <button type="button" className="link-btn" onClick={onOpenAdmin}>
                Créer des tags
              </button>
            </span>
          ) : (
            <>
              <div className="tag-select">
                {availableTags.map((tag) => (
                  <button
                    key={tag.id}
                    type="button"
                    className={`tag-chip ${tags.includes(tag.name) ? "active" : ""}`}
                    onClick={() => toggleTag(tag.name)}
                  >
                    {tag.name}
                  </button>
                ))}
              </div>
              <span className="field-hint">
                <button type="button" className="link-btn" onClick={onOpenAdmin}>
                  Gérer les tags
                </button>
              </span>
            </>
          )}
        </div>

        {error && <p className="form-error">{error}</p>}

        <div className="modal-footer">
          <button type="button" className="secondary-btn" onClick={onClose}>
            Annuler
          </button>
          <button type="submit" className="primary-btn">
            {initial ? "Enregistrer" : "Ajouter le projet"}
          </button>
        </div>
      </form>
    </div>
  );
}
