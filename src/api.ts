import { invoke } from "@tauri-apps/api/core";
import type {
  Category,
  ChangelogEntry,
  Project,
  ProjectInput,
  ProjectInsights,
  ScanCandidate,
  Settings,
  Tag,
} from "./types";

export const listProjects = () => invoke<Project[]>("list_projects");

export const createProject = (input: ProjectInput) =>
  invoke<number>("create_project", { input });

export const updateProject = (id: number, input: ProjectInput) =>
  invoke<void>("update_project", { id, input });

export const deleteProject = (id: number) =>
  invoke<void>("delete_project", { id });

export const listTags = () => invoke<Tag[]>("list_tags");

export const createTag = (name: string) => invoke<number>("create_tag", { name });

export const renameTag = (id: number, name: string) =>
  invoke<void>("rename_tag", { id, name });

export const deleteTag = (id: number) => invoke<void>("delete_tag", { id });

export const openRepo = (repo: string) => invoke<void>("open_repo", { repo });

export const listCategories = () => invoke<Category[]>("list_categories");

export const createCategory = (name: string) =>
  invoke<number>("create_category", { name });

export const renameCategory = (id: number, name: string) =>
  invoke<void>("rename_category", { id, name });

export const deleteCategory = (id: number) =>
  invoke<void>("delete_category", { id });

export const listChangelog = (projectId: number) =>
  invoke<ChangelogEntry[]>("list_changelog", { projectId });

export const addChangelogEntry = (projectId: number, entry: string) =>
  invoke<number>("add_changelog_entry", { projectId, entry });

export const deleteChangelogEntry = (id: number) =>
  invoke<void>("delete_changelog_entry", { id });

export const getSettings = () => invoke<Settings>("get_settings");

export const setSetting = (key: string, value: string) =>
  invoke<void>("set_setting", { key, value });

export const scanDirectory = (root: string) =>
  invoke<ScanCandidate[]>("scan_directory", { root });

export const openInFinder = (path: string) =>
  invoke<void>("open_in_finder", { path });

export const openInIde = (path: string) => invoke<void>("open_in_ide", { path });

export const openInTerminal = (path: string) =>
  invoke<void>("open_in_terminal", { path });

export const projectInsights = (path: string) =>
  invoke<ProjectInsights>("project_insights", { path });

export const exportData = (dest: string) =>
  invoke<void>("export_data", { dest });

export const revealDatabase = () => invoke<void>("reveal_database");
