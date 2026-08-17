// Stable hue per category name: the same category always gets the same color.
export function hueFor(text: string): number {
  let hash = 0;
  for (let i = 0; i < text.length; i++) {
    hash = (hash * 31 + text.charCodeAt(i)) | 0;
  }
  return ((hash % 360) + 360) % 360;
}

export function categoryColor(category: string): string {
  if (!category) return "var(--muted)";
  return `hsl(${hueFor(category)} 42% 62%)`;
}

export function categoryColorDim(category: string): string {
  if (!category) return "var(--line)";
  return `hsl(${hueFor(category)} 30% 30%)`;
}

import type { ProjectStatus } from "./types";

export const STATUSES: { value: ProjectStatus; label: string }[] = [
  { value: "active", label: "Actif" },
  { value: "paused", label: "En pause" },
  { value: "done", label: "Terminé" },
  { value: "dropped", label: "Abandonné" },
];

export function statusLabel(status: ProjectStatus): string {
  return STATUSES.find((s) => s.value === status)?.label ?? status;
}

export function isArchived(status: ProjectStatus): boolean {
  return status === "done" || status === "dropped";
}

export function formatDate(sqlite: string): string {
  // SQLite gives "YYYY-MM-DD HH:MM:SS" in local time.
  const d = new Date(sqlite.replace(" ", "T"));
  if (isNaN(d.getTime())) return sqlite;
  return d.toLocaleDateString("fr-FR", {
    day: "numeric",
    month: "short",
    year: "numeric",
  });
}

export function formatDateTime(sqlite: string): string {
  const d = new Date(sqlite.replace(" ", "T"));
  if (isNaN(d.getTime())) return sqlite;
  return d.toLocaleDateString("fr-FR", {
    day: "numeric",
    month: "short",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} o`;
  const units = ["Ko", "Mo", "Go", "To"];
  let value = bytes;
  let i = -1;
  do {
    value /= 1024;
    i++;
  } while (value >= 1024 && i < units.length - 1);
  return `${value < 10 ? value.toFixed(1) : Math.round(value)} ${units[i]}`;
}

export function shortenPath(path: string): string {
  const home = path.match(/^\/Users\/[^/]+/);
  return home ? path.replace(home[0], "~") : path;
}
