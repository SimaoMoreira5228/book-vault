import { err, ok, Result } from "neverthrow";
import { apiConfig } from "./config";
import type {
	BookResponse,
	CreateBookRequest,
	LoginRequest,
	RegisterRequest,
	SearchResult,
	ShelfResponse,
	UserResponse
} from "./generated";

export class ApiError {
	constructor(
		public status: number,
		public message: string
	) {}
}

type HttpMethod = "GET" | "POST" | "PUT" | "DELETE";

async function request<T>(
	method: HttpMethod,
	path: string,
	body?: unknown
): Promise<Result<T, ApiError>> {
	try {
		const url = apiConfig.baseUrl ? `${apiConfig.baseUrl}${path}` : path;
		const res = await fetch(url, {
			method,
			headers: body ? { "Content-Type": "application/json" } : undefined,
			body: body ? JSON.stringify(body) : undefined,
			credentials: apiConfig.credentials
		});

		if (res.status === 401) {
			authState.clear();
			return err(new ApiError(401, "Session expired"));
		}

		if (!res.ok) {
			const text = await res.text().catch(() => "Unknown error");
			return err(new ApiError(res.status, text));
		}

		if (res.headers.get("content-type")?.includes("application/json")) {
			const data = await res.json();
			return ok(data as T);
		}

		return ok(undefined as T);
	} catch (e) {
		return err(new ApiError(0, e instanceof Error ? e.message : "Network error"));
	}
}

class AuthState {
	user = $state<UserResponse | null>(null);
	isAuthenticated = $derived(this.user !== null);

	clear() {
		this.user = null;
	}

	async login(credentials: LoginRequest): Promise<Result<UserResponse, ApiError>> {
		const result = await request<{ user: UserResponse; cookie: string }>(
			"POST",
			"/api/v1/auth/login",
			credentials
		);
		if (result.isOk()) {
			this.user = result.value.user;
		}
		return result.map((r) => r.user);
	}

	async logout(): Promise<Result<void, ApiError>> {
		const result = await request<void>("POST", "/api/v1/auth/logout");
		if (result.isOk()) {
			this.clear();
		}
		return result;
	}

	async register(data: RegisterRequest): Promise<Result<UserResponse, ApiError>> {
		const result = await request<UserResponse>("POST", "/api/v1/auth/register", data);
		if (result.isOk()) {
			this.user = result.value;
		}
		return result;
	}
}

export const authState = new AuthState();

export const api = {
	auth: {
		login: (data: LoginRequest) => authState.login(data),
		logout: () => authState.logout(),
		register: (data: RegisterRequest) => authState.register(data)
	},

	books: {
		list: () => request<BookResponse[]>("GET", "/api/v1/books"),
		get: (id: string) => request<BookResponse>("GET", `/api/v1/books/${id}`),
		create: (data: CreateBookRequest) => request<BookResponse>("POST", "/api/v1/books", data),
		update: (id: string, data: Partial<CreateBookRequest>) =>
			request<BookResponse>("PUT", `/api/v1/books/${id}`, data),
		delete: (id: string) => request<void>("DELETE", `/api/v1/books/${id}`),
		upload: (file: File) => {
			const form = new FormData();
			form.append("file", file);
			return request<{ job_id: string }>("POST", "/api/v1/books/upload", form);
		}
	},

	shelves: {
		list: () => request<ShelfResponse[]>("GET", "/api/v1/shelves"),
		create: (data: { name: string; description?: string; kind?: string }) =>
			request<ShelfResponse>("POST", "/api/v1/shelves", data),
		delete: (id: string) => request<void>("DELETE", `/api/v1/shelves/${id}`)
	},

	search: (q: string) => request<SearchResult>("GET", `/api/v1/search?q=${encodeURIComponent(q)}`),

	read: (id: string) => request<{ book: unknown }>("GET", `/api/v1/books/${id}/read`),

	export: (id: string, format: string) => {
		window.open(`/api/v1/books/${id}/export?format=${format}`, "_blank");
	},

	raw: (id: string) => {
		window.open(`/api/v1/books/${id}/raw`, "_blank");
	},

	comic: {
		pages: (id: string) =>
			request<Array<{ page: number; asset_id: string; mime_type: string }>>(
				"GET",
				`/api/v1/books/${id}/comic/pages`
			),
		page: (id: string, n: number) => {
			window.open(`/api/v1/books/${id}/comic/page/${n}`, "_blank");
		}
	},

	asset: (bookId: string, assetId: string) => `/api/v1/books/${bookId}/assets/${assetId}`,

	progress: {
		get: (bookId: string) =>
			request<{
				section_id: string;
				block_index: number;
				char_offset: number;
				percentage: number;
				updated_at: string;
			} | null>("GET", `/api/v1/books/${bookId}/progress`),
		save: (
			bookId: string,
			data: {
				section_id: string;
				block_index: number;
				char_offset: number;
				percentage: number;
			}
		) =>
			request<{
				section_id: string;
				block_index: number;
				char_offset: number;
				percentage: number;
				updated_at: string;
			}>("PUT", `/api/v1/books/${bookId}/progress`, data)
	},

	annotations: {
		listAll: () =>
			request<
				Array<{
					id: string;
					book_id: string;
					section_id: string;
					block_index: number;
					start_offset: number;
					end_offset: number;
					color: string | null;
					note: string | null;
					created_at: string;
					updated_at: string;
				}>
			>("GET", "/api/v1/annotations/all"),
		list: (bookId: string) =>
			request<
				Array<{
					id: string;
					book_id: string;
					section_id: string;
					block_index: number;
					start_offset: number;
					end_offset: number;
					color: string | null;
					note: string | null;
					created_at: string;
					updated_at: string;
				}>
			>("GET", `/api/v1/books/${bookId}/annotations`),
		create: (
			bookId: string,
			data: {
				section_id: string;
				block_index: number;
				start_offset: number;
				end_offset: number;
				color?: string;
				note?: string;
			}
		) =>
			request<{
				id: string;
				book_id: string;
				section_id: string;
				block_index: number;
				start_offset: number;
				end_offset: number;
				color: string | null;
				note: string | null;
				created_at: string;
				updated_at: string;
			}>("POST", `/api/v1/books/${bookId}/annotations`, data),
		update: (annotationId: string, data: { color?: string; note?: string }) =>
			request("PUT", `/api/v1/annotations/${annotationId}`, data),
		delete: (annotationId: string) => request("DELETE", `/api/v1/annotations/${annotationId}`)
	},

	studio: {
		saveSection: (bookId: string, sectionId: string, blocks: unknown) =>
			request<{ message: string; version: number }>(
				"PUT",
				`/api/v1/books/${bookId}/sections/${sectionId}`,
				{ blocks }
			),
		revisions: {
			list: (bookId: string) =>
				request<
					Array<{
						id: string;
						book_id: string;
						section_id: string;
						version: number;
						created_at: string;
					}>
				>("GET", `/api/v1/books/${bookId}/revisions`),
			get: (revisionId: string) =>
				request<{
					id: string;
					book_id: string;
					section_id: string;
					version: number;
					snapshot: unknown;
					created_at: string;
				}>("GET", `/api/v1/revisions/${revisionId}`),
			restore: (revisionId: string) =>
				request<{ message: string; version: number }>(
					"POST",
					`/api/v1/revisions/${revisionId}/restore`
				)
		}
	},

	sessions: {
		list: () =>
			request<
				Array<{
					id: string;
					user_agent: string | null;
					ip_address: string | null;
					created_at: string;
					last_seen_at: string;
					expires_at: string;
					is_current: boolean;
				}>
			>("GET", "/api/v1/auth/sessions"),
		revoke: (id: string) => request<void>("DELETE", `/api/v1/auth/sessions/${id}`)
	},

	admin: {
		jobs: () =>
			request<
				Array<{
					id: string;
					kind: string;
					status: string;
					error: string | null;
					retry_count: number;
					max_retries: number;
					created_at: string;
				}>
			>("GET", "/api/v1/admin/jobs"),
		users: () =>
			request<
				Array<{
					id: string;
					email: string;
					display_name: string;
					is_admin: boolean;
					created_at: string;
				}>
			>("GET", "/api/v1/admin/users"),
		cleanupSessions: () => request<{ deleted: number }>("GET", "/api/v1/admin/sessions/cleanup")
	}
};
