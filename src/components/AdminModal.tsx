import { useState } from "react";
import type { Category, Tag } from "../types";
import {
  createCategory,
  createTag,
  deleteCategory,
  deleteTag,
  renameCategory,
  renameTag,
} from "../api";

interface Item {
  id: number;
  name: string;
  usage_count: number;
}

interface SectionProps {
  title: string;
  items: Item[];
  placeholder: string;
  usageLabel: (count: number) => string;
  deleteHint: string;
  onCreate: (name: string) => Promise<unknown>;
  onRename: (id: number, name: string) => Promise<unknown>;
  onDelete: (id: number) => Promise<unknown>;
  onChanged: () => void;
}

function ReferentialSection({
  title,
  items,
  placeholder,
  usageLabel,
  deleteHint,
  onCreate,
  onRename,
  onDelete,
  onChanged,
}: SectionProps) {
  const [newName, setNewName] = useState("");
  const [editingId, setEditingId] = useState<number | null>(null);
  const [editDraft, setEditDraft] = useState("");
  const [confirmId, setConfirmId] = useState<number | null>(null);
  const [error, setError] = useState("");

  const run = async (action: () => Promise<unknown>) => {
    setError("");
    try {
      await action();
      onChanged();
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <section className="admin-section">
      <h3 className="admin-section-title">{title}</h3>
      <form
        className="field-row"
        onSubmit={(e) => {
          e.preventDefault();
          run(async () => {
            await onCreate(newName);
            setNewName("");
          });
        }}
      >
        <input
          value={newName}
          onChange={(e) => setNewName(e.target.value)}
          placeholder={placeholder}
        />
        <button type="submit" className="primary-btn" disabled={!newName.trim()}>
          Ajouter
        </button>
      </form>

      {error && <p className="form-error">{error}</p>}

      {items.length === 0 ? (
        <p className="modal-hint">Rien pour l'instant.</p>
      ) : (
        <ul className="tag-admin-list">
          {items.map((item) => (
            <li key={item.id} className="tag-admin-row">
              {editingId === item.id ? (
                <form
                  className="field-row tag-admin-edit"
                  onSubmit={(e) => {
                    e.preventDefault();
                    run(async () => {
                      await onRename(item.id, editDraft);
                      setEditingId(null);
                    });
                  }}
                >
                  <input
                    autoFocus
                    value={editDraft}
                    onChange={(e) => setEditDraft(e.target.value)}
                  />
                  <button type="submit" className="primary-btn">
                    OK
                  </button>
                  <button
                    type="button"
                    className="secondary-btn"
                    onClick={() => setEditingId(null)}
                  >
                    Annuler
                  </button>
                </form>
              ) : (
                <>
                  <span className="tag-admin-name">{item.name}</span>
                  <span className="tag-admin-count">
                    {usageLabel(item.usage_count)}
                  </span>
                  {confirmId === item.id ? (
                    <>
                      <button
                        className="danger-btn"
                        title={deleteHint}
                        onClick={() =>
                          run(async () => {
                            await onDelete(item.id);
                            setConfirmId(null);
                          })
                        }
                      >
                        Confirmer
                      </button>
                      <button
                        className="secondary-btn"
                        onClick={() => setConfirmId(null)}
                      >
                        Annuler
                      </button>
                    </>
                  ) : (
                    <>
                      <button
                        className="ghost-btn"
                        onClick={() => {
                          setEditingId(item.id);
                          setEditDraft(item.name);
                          setConfirmId(null);
                        }}
                      >
                        Renommer
                      </button>
                      <button
                        className="danger-ghost-btn"
                        onClick={() => setConfirmId(item.id)}
                      >
                        Supprimer
                      </button>
                    </>
                  )}
                </>
              )}
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

interface Props {
  tags: Tag[];
  categories: Category[];
  onChanged: () => void;
  onClose: () => void;
}

export default function AdminModal({
  tags,
  categories,
  onChanged,
  onClose,
}: Props) {
  const plural = (n: number) =>
    `${n} projet${n > 1 ? "s" : ""}`;

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <h2 className="modal-title">Administration</h2>
        <p className="modal-hint">
          Renommer se propage à tous les projets concernés. Supprimer une
          catégorie bascule ses projets en « Sans catégorie » ; supprimer un
          tag le retire de tous les projets.
        </p>

        <ReferentialSection
          title="Catégories"
          items={categories}
          placeholder="Nouvelle catégorie (ex. POC, Perso, Client…)"
          usageLabel={plural}
          deleteHint="Les projets concernés passeront en « Sans catégorie »"
          onCreate={createCategory}
          onRename={renameCategory}
          onDelete={deleteCategory}
          onChanged={onChanged}
        />

        <ReferentialSection
          title="Tags"
          items={tags}
          placeholder="Nouveau tag (ex. rust, cli, ia…)"
          usageLabel={plural}
          deleteHint="Le tag sera retiré de tous les projets"
          onCreate={createTag}
          onRename={renameTag}
          onDelete={deleteTag}
          onChanged={onChanged}
        />

        <div className="modal-footer">
          <button type="button" className="secondary-btn" onClick={onClose}>
            Fermer
          </button>
        </div>
      </div>
    </div>
  );
}
