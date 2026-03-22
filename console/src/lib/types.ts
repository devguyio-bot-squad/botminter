export interface TeamSummary {
	name: string;
	profile: string;
	github_repo: string;
	path: string;
}

export interface TeamOverview {
	name: string;
	profile: string;
	github_repo: string;
	members: MemberSummary[];
	knowledge_files: string[];
	invariant_files: string[];
}

export interface MemberSummary {
	name: string;
	role: string;
	hat_count: number;
}

export interface MemberDetail {
	name: string;
	role: string;
	ralph_yml: string;
}

export interface FileEntry {
	name: string;
	path: string;
	is_dir: boolean;
}

export interface FileContent {
	path: string;
	content: string;
}

export interface FileSaveResult {
	path: string;
	commit_sha: string;
}

export interface ProcessData {
	dot_files: DotFile[];
	statuses: StatusEntry[];
	process_md: string | null;
}

export interface DotFile {
	name: string;
	content: string;
}

export interface StatusEntry {
	name: string;
	label: string;
}

export interface ApiError {
	error: string;
}
