export type ProjectStatus = "active" | "paused" | "done" | "dropped";

export interface Project {
  id: number;
  name: string;
  description: string;
  category: string;
  tags: string[];
  path: string;
  repo: string;
  status: ProjectStatus;
  created_at: string;
  updated_at: string;
  changelog_count: number;
  path_exists: boolean;
}

export interface ProjectInput {
  name: string;
  description: string;
  category: string;
  tags: string[];
  path: string;
  repo: string;
  status: ProjectStatus;
}

export interface Tag {
  id: number;
  name: string;
  usage_count: number;
}

export interface Category {
  id: number;
  name: string;
  usage_count: number;
}

export interface ChangelogEntry {
  id: number;
  project_id: number;
  entry: string;
  created_at: string;
}

export interface ScanCandidate {
  name: string;
  path: string;
  has_git: boolean;
  already_registered: boolean;
}

export type Settings = Record<string, string>;

export interface GitInfo {
  branch: string;
  last_commit_at: string | null;
  dirty: boolean;
}

export interface ArtifactDir {
  rel_path: string;
  bytes: number;
}

export interface ProjectInsights {
  techs: string[];
  git: GitInfo | null;
  size_bytes: number;
  reclaim_bytes: number;
  artifacts: ArtifactDir[];
}
