import type { TeamSummary, TeamOverview, ProcessData, MemberListEntry, MemberDetail, ApiError } from './types.js';

class ApiClient {
	private baseUrl: string;

	constructor(baseUrl = '') {
		this.baseUrl = baseUrl;
	}

	private async request<T>(path: string, options?: RequestInit): Promise<T> {
		const response = await fetch(`${this.baseUrl}${path}`, options);
		if (!response.ok) {
			const body: ApiError = await response.json().catch(() => ({
				error: `HTTP ${response.status}: ${response.statusText}`
			}));
			throw new Error(body.error);
		}
		return response.json();
	}

	async fetchTeams(): Promise<TeamSummary[]> {
		return this.request<TeamSummary[]>('/api/teams');
	}

	async fetchOverview(team: string): Promise<TeamOverview> {
		return this.request<TeamOverview>(`/api/teams/${encodeURIComponent(team)}/overview`);
	}

	async fetchProcess(team: string): Promise<ProcessData> {
		return this.request<ProcessData>(`/api/teams/${encodeURIComponent(team)}/process`);
	}

	async fetchMembers(team: string): Promise<MemberListEntry[]> {
		return this.request<MemberListEntry[]>(`/api/teams/${encodeURIComponent(team)}/members`);
	}

	async fetchMember(team: string, name: string): Promise<MemberDetail> {
		return this.request<MemberDetail>(
			`/api/teams/${encodeURIComponent(team)}/members/${encodeURIComponent(name)}`
		);
	}
}

export const api = new ApiClient();
