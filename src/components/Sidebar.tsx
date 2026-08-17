import type { Project } from "../types";
import { categoryColor } from "../utils";

interface Props {
  projects: Project[];
  activeCount: number;
  archivedCount: number;
  showArchived: boolean;
  onShowArchived: (v: boolean) => void;
  category: string | null;
  onCategory: (c: string | null) => void;
  tags: string[];
  onToggleTag: (t: string) => void;
}

export default function Sidebar({
  projects,
  activeCount,
  archivedCount,
  showArchived,
  onShowArchived,
  category,
  onCategory,
  tags,
  onToggleTag,
}: Props) {
  const categories = new Map<string, number>();
  const allTags = new Map<string, number>();
  for (const p of projects) {
    const cat = p.category || "Sans catégorie";
    categories.set(cat, (categories.get(cat) ?? 0) + 1);
    for (const t of p.tags) allTags.set(t, (allTags.get(t) ?? 0) + 1);
  }
  const sortedCats = [...categories.entries()].sort((a, b) =>
    a[0].localeCompare(b[0], "fr"),
  );
  const sortedTags = [...allTags.entries()].sort((a, b) => b[1] - a[1]);

  return (
    <aside className="sidebar">
      <div className="sidebar-section">
        <h2 className="sidebar-title">Vue</h2>
        <button
          className={`cat-row ${!showArchived ? "active" : ""}`}
          onClick={() => onShowArchived(false)}
        >
          <span className="cat-name">En cours</span>
          <span className="cat-count">{activeCount}</span>
        </button>
        <button
          className={`cat-row ${showArchived ? "active" : ""}`}
          onClick={() => onShowArchived(true)}
        >
          <span className="cat-name">Archivés</span>
          <span className="cat-count">{archivedCount}</span>
        </button>
      </div>

      <div className="sidebar-section">
        <h2 className="sidebar-title">Catégories</h2>
        <button
          className={`cat-row ${category === null ? "active" : ""}`}
          onClick={() => onCategory(null)}
        >
          <span className="cat-dot all" />
          <span className="cat-name">Toutes</span>
          <span className="cat-count">{projects.length}</span>
        </button>
        {sortedCats.map(([cat, count]) => (
          <button
            key={cat}
            className={`cat-row ${category === cat ? "active" : ""}`}
            onClick={() => onCategory(category === cat ? null : cat)}
          >
            <span
              className="cat-dot"
              style={{ background: categoryColor(cat) }}
            />
            <span className="cat-name">{cat}</span>
            <span className="cat-count">{count}</span>
          </button>
        ))}
      </div>

      {sortedTags.length > 0 && (
        <div className="sidebar-section">
          <h2 className="sidebar-title">Tags</h2>
          <div className="tag-cloud">
            {sortedTags.map(([tag, count]) => (
              <button
                key={tag}
                className={`tag-chip ${tags.includes(tag) ? "active" : ""}`}
                onClick={() => onToggleTag(tag)}
              >
                {tag}
                <span className="tag-count">{count}</span>
              </button>
            ))}
          </div>
        </div>
      )}
    </aside>
  );
}
