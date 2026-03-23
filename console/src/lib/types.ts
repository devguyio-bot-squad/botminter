export interface TeamSummary {
	name: string;
	profile: string;
	github_repo: string;
	path: string;
}

export interface TeamOverview {
	name: string;
	profile: string;
	display_name: string;
	description: string;
	version: string;
	github_repo: string;
	default_coding_agent: string | null;
	roles: RoleSummary[];
	members: MemberSummary[];
	status_count: number;
	label_count: number;
	projects: ProjectSummary[];
	bridge: BridgeOverview;
	knowledge_files: string[];
	invariant_files: string[];
}

export interface RoleSummary {
	name: string;
	description: string;
}

export interface MemberSummary {
	name: string;
	role: string;
	comment_emoji: string;
	hat_count: number;
}

export interface ProjectSummary {
	name: string;
	fork_url: string;
}

export interface BridgeOverview {
	selected: string | null;
	available: string[];
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
	markdown: string | null;
	workflows: WorkflowEntry[];
	statuses: StatusEntry[];
	labels: LabelEntry[];
	views: ViewEntry[];
}

export interface WorkflowEntry {
	name: string;
	dot: string;
}

export interface StatusEntry {
	name: string;
	description: string;
}

export interface LabelEntry {
	name: string;
	color: string;
	description: string;
}

export interface ViewEntry {
	name: string;
	prefixes: string[];
	also_include: string[];
}

export interface ApiError {
	error: string;
}
