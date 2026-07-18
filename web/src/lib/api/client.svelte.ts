import { err, ok, Result } from "neverthrow";
import { env } from "$env/dynamic/public";
import type {
	BookResponse,
	CreateBookRequest,
	LoginRequest,
	RegisterRequest,
	SearchResult,
	PaginatedBooks,
	ProspectiveMetadata,
	SeriesResponse,
	ShelfResponse,
	UserResponse
} from "./generated";
import { SvelteURLSearchParams } from "svelte/reactivity";

const apiBase = env.PUBLIC_API_URL.replace(/\/+$/, "");
export { apiBase };

export type ListBooksParams = {
	limit?: number;
	offset?: number;
	sortBy?: string;
	sortOrder?: "asc" | "desc";
	readStatus?: string;
	format?: string;
	search?: string;
};

export type SpineItem = { id: string; title: string | null; sequence_index: number };

export class ApiError {
	constructor(
		public status: number,
		public message: string
	) {}
}

type HttpMethod = "GET" | "POST" | "PUT" | "DELETE";

type RequestOptions = {
	signal?: AbortSignal;
	dedupe?: boolean;
};

// eslint-disable-next-line svelte/prefer-svelte-reactivity
const inFlight = new Map<string, Promise<Result<unknown, ApiError>>>();

async function request<T>(
	method: HttpMethod,
	path: string,
	body?: unknown,
	options?: RequestOptions
): Promise<Result<T, ApiError>> {
	const dedupe = options?.dedupe ?? method === "GET";
	const key = `${method}:${path}:${body ? JSON.stringify(body) : ""}`;

	if (dedupe) {
		const existing = inFlight.get(key);
		if (existing) return existing as Promise<Result<T, ApiError>>;
	}

	const promise = doRequest<T>(method, path, body, options).finally(() => {
		if (dedupe) inFlight.delete(key);
	});

	if (dedupe) inFlight.set(key, promise as Promise<Result<unknown, ApiError>>);

	return promise;
}

async function doRequest<T>(
	method: HttpMethod,
	path: string,
	body: unknown,
	options?: RequestOptions
): Promise<Result<T, ApiError>> {
	try {
		const url = `${apiBase}${path}`;
		const isForm = body instanceof FormData;
		const res = await fetch(url, {
			method,
			headers: body && !isForm ? { "Content-Type": "application/json" } : undefined,
			body: body ? (isForm ? body : JSON.stringify(body)) : undefined,
			credentials: "include",
			signal: options?.signal
		});

		if (res.status === 401) {
			const text = await res.text().catch(() => "");
			if (path.startsWith("/api/v1/auth/") || text.toLowerCase().includes("unauthorized")) {
				authState.handleUnauthorized();
			}
			return err(new ApiError(401, text || "Session expired"));
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
		if (e instanceof DOMException && e.name === "AbortError") {
			return err(new ApiError(0, "Aborted"));
		}
		return err(new ApiError(0, e instanceof Error ? e.message : "Network error"));
	}
}

class AuthState {
	user = $state<UserResponse | null>(null);
	restoring = $state(true);
	isAuthenticated = $derived(this.user !== null);
	private restorePromise: Promise<void> | null = null;
	private lastConfirmedAuthAt = 0;

	clear() {
		this.user = null;
	}

	handleUnauthorized() {
		this.clear();
	}

	async restore(): Promise<void> {
		if (this.restorePromise) return this.restorePromise;

		this.restoring = true;
		this.restorePromise = (async () => {
			const result = await request<UserResponse>("GET", "/api/v1/auth/profile");
			if (result.isOk()) {
				this.user = result.value;
				this.lastConfirmedAuthAt = Date.now();
			} else if (result.error.status === 401) {
				this.user = null;
			}
			this.restoring = false;
		})();

		try {
			await this.restorePromise;
		} finally {
			this.restorePromise = null;
		}
	}

	async login(credentials: LoginRequest): Promise<Result<UserResponse, ApiError>> {
		const result = await request<{ user: UserResponse; cookie: string }>(
			"POST",
			"/api/v1/auth/login",
			credentials,
			{ dedupe: false }
		);
		if (result.isOk()) {
			this.user = result.value.user;
			this.lastConfirmedAuthAt = Date.now();
		}
		return result.map((r) => r.user);
	}

	async logout(): Promise<Result<void, ApiError>> {
		const result = await request<void>("POST", "/api/v1/auth/logout", undefined, {
			dedupe: false
		});
		if (result.isOk()) {
			this.clear();
		}
		return result;
	}

	async register(data: RegisterRequest): Promise<Result<UserResponse, ApiError>> {
		const result = await request<UserResponse>("POST", "/api/v1/auth/register", data, {
			dedupe: false
		});
		if (result.isOk()) {
			this.user = result.value;
			this.lastConfirmedAuthAt = Date.now();
		}
		return result;
	}
}

export const authState = new AuthState();

function toQueryString(params: Record<string, string | undefined>): string {
	const search = new SvelteURLSearchParams();
	for (const [key, value] of Object.entries(params)) {
		if (value !== undefined) search.set(key, value);
	}
	return search.toString();
}

export const api = {
	auth: {
		login: (data: LoginRequest) => authState.login(data),
		logout: () => authState.logout(),
		register: (data: RegisterRequest) => authState.register(data),
		updateProfile: (data: { display_name?: string }) =>
			request<UserResponse>("PUT", "/api/v1/auth/profile", data, { dedupe: false }),
		changePassword: (data: { current_password: string; new_password: string }) =>
			request<Record<string, unknown>>("PUT", "/api/v1/auth/password", data, { dedupe: false }),
		getPreferences: () => request<Record<string, unknown>>("GET", "/api/v1/auth/preferences"),
		updatePreferences: (preferences: Record<string, unknown>) =>
			request<Record<string, unknown>>(
				"PUT",
				"/api/v1/auth/preferences",
				{ preferences },
				{ dedupe: false }
			)
	},

	books: {
		list: (params?: ListBooksParams, options?: RequestOptions) => {
			const query = toQueryString({
				limit: params?.limit !== undefined ? String(params.limit) : undefined,
				offset: params?.offset !== undefined ? String(params.offset) : undefined,
				sortBy: params?.sortBy,
				sortOrder: params?.sortOrder,
				read_status: params?.readStatus,
				format: params?.format,
				search: params?.search
			});
			return request<PaginatedBooks>("GET", `/api/v1/books?${query}`, undefined, options);
		},
		get: (id: string) => request<BookResponse>("GET", `/api/v1/books/${id}`),
		create: (data: CreateBookRequest) =>
			request<BookResponse>("POST", "/api/v1/books", data, { dedupe: false }),
		update: (id: string, data: Partial<CreateBookRequest>) =>
			request<BookResponse>("PUT", `/api/v1/books/${id}`, data, { dedupe: false }),
		delete: (id: string) =>
			request<void>("DELETE", `/api/v1/books/${id}`, undefined, { dedupe: false }),
		upload: async (file: File) => {
			const form = new FormData();
			form.append("file", file);
			try {
				const res = await fetch(`${apiBase}/api/v1/books/upload`, {
					method: "POST",
					body: form,
					credentials: "include"
				});
				if (res.status === 401) {
					authState.handleUnauthorized();
					return err(new ApiError(401, "Session expired"));
				}
				if (!res.ok) {
					const text = await res.text().catch(() => "Unknown error");
					return err(new ApiError(res.status, text));
				}
				const data = await res.json();
				return ok(data as { job_id: string });
			} catch (e) {
				return err(new ApiError(0, e instanceof Error ? e.message : "Network error"));
			}
		}
	},

	shelves: {
		list: (options?: RequestOptions) =>
			request<ShelfResponse[]>("GET", "/api/v1/shelves", undefined, options),
		get: (id: string) => request<ShelfResponse>("GET", `/api/v1/shelves/${id}`),
		create: (data: { name: string; description?: string; kind?: string }) =>
			request<ShelfResponse>("POST", "/api/v1/shelves", data, { dedupe: false }),
		delete: (id: string) =>
			request<void>("DELETE", `/api/v1/shelves/${id}`, undefined, { dedupe: false }),
		getBooks: (id: string) =>
			request<
				Array<{
					book_id: string;
					title: string;
					author: string | null;
					read_status: string;
				}>
			>("GET", `/api/v1/shelves/${id}/books`),
		addBook: (shelfId: string, bookId: string) =>
			request<Record<string, unknown>>(
				"POST",
				`/api/v1/shelves/${shelfId}/books`,
				{ book_id: bookId },
				{ dedupe: false }
			),
		removeBook: (shelfId: string, bookId: string) =>
			request<void>("DELETE", `/api/v1/shelves/${shelfId}/books/${bookId}`, undefined, {
				dedupe: false
			})
	},

	search: (q: string) => request<SearchResult>("GET", `/api/v1/search?q=${encodeURIComponent(q)}`),

	read: (id: string) => request<{ book: unknown }>("GET", `/api/v1/books/${id}/read`),
	readSpine: (id: string) =>
		request<Array<{ id: string; title: string | null; sequence_index: number }>>(
			"GET",
			`/api/v1/books/${id}/read/spine`
		),
	readSection: (id: string, sectionId: string) =>
		request<Array<Record<string, unknown>>>("GET", `/api/v1/books/${id}/read/section/${sectionId}`),

	export: (id: string, format: string) => {
		window.open(`${apiBase}/api/v1/books/${id}/export?format=${format}`, "_blank");
	},

	raw: (id: string) => {
		window.open(`${apiBase}/api/v1/books/${id}/raw`, "_blank");
	},

	comic: {
		pages: (id: string) =>
			request<Array<{ page: number; asset_id: string; mime_type: string }>>(
				"GET",
				`/api/v1/books/${id}/comic/pages`
			),
		page: (id: string, n: number) => {
			window.open(`${apiBase}/api/v1/books/${id}/comic/page/${n}`, "_blank");
		}
	},

	asset: (bookId: string, assetId: string) => `${apiBase}/api/v1/books/${bookId}/assets/${assetId}`,

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
			}>("PUT", `/api/v1/books/${bookId}/progress`, data, { dedupe: false })
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
			}>("POST", `/api/v1/books/${bookId}/annotations`, data, { dedupe: false }),
		update: (annotationId: string, data: { color?: string; note?: string }) =>
			request("PUT", `/api/v1/annotations/${annotationId}`, data, { dedupe: false }),
		delete: (annotationId: string) =>
			request("DELETE", `/api/v1/annotations/${annotationId}`, undefined, { dedupe: false })
	},

	studio: {
		saveSection: (bookId: string, sectionId: string, blocks: unknown) =>
			request<{ message: string; version: number }>(
				"PUT",
				`/api/v1/books/${bookId}/sections/${sectionId}`,
				{ blocks },
				{ dedupe: false }
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
					`/api/v1/revisions/${revisionId}/restore`,
					undefined,
					{ dedupe: false }
				)
		}
	},

	metadata: {
		get: (bookId: string) =>
			request<Record<string, unknown>>("GET", `/api/v1/books/${bookId}/metadata`),
		candidates: (bookId: string, query: { title?: string; author?: string; isbn?: string }) => {
			const qs = toQueryString(query);
			return request<ProspectiveMetadata[]>(
				"GET",
				`/api/v1/books/${bookId}/metadata/candidates?${qs}`
			);
		},
		confirm: (bookId: string, candidate: ProspectiveMetadata) =>
			request<Record<string, unknown>>(
				"POST",
				`/api/v1/books/${bookId}/metadata/confirm`,
				{ candidate },
				{ dedupe: false }
			),
		refresh: (bookId: string) =>
			request<Record<string, unknown>>(
				"POST",
				`/api/v1/books/${bookId}/metadata/refresh`,
				undefined,
				{ dedupe: false }
			),
		lockField: (bookId: string, field: string) =>
			request<Record<string, unknown>>(
				"POST",
				`/api/v1/books/${bookId}/metadata/lock/${field}`,
				undefined,
				{ dedupe: false }
			),
		unlockField: (bookId: string, field: string) =>
			request<Record<string, unknown>>(
				"DELETE",
				`/api/v1/books/${bookId}/metadata/lock/${field}`,
				undefined,
				{ dedupe: false }
			)
	},

	authors: {
		list: () =>
			request<
				Array<{
					id: string;
					name: string;
					sort_name: string | null;
					bio: string | null;
					birth_date: string | null;
					death_date: string | null;
					book_count: number;
				}>
			>("GET", "/api/v1/authors"),
		get: (id: string) =>
			request<{
				id: string;
				name: string;
				sort_name: string | null;
				bio: string | null;
				birth_date: string | null;
				death_date: string | null;
				book_count: number;
			}>("GET", `/api/v1/authors/${id}`),
		create: (data: {
			name: string;
			sort_name?: string;
			bio?: string;
			birth_date?: string;
			death_date?: string;
		}) => request<Record<string, unknown>>("POST", "/api/v1/authors", data, { dedupe: false }),
		linkBook: (bookId: string, authorId: string) =>
			request<void>(
				"PUT",
				`/api/v1/books/${bookId}/link-author`,
				{ author_id: authorId },
				{ dedupe: false }
			)
	},

	series: {
		list: () => request<SeriesResponse[]>("GET", "/api/v1/series"),
		get: (id: string) => request<SeriesResponse>("GET", `/api/v1/series/${id}`),
		create: (data: { name: string; description?: string }) =>
			request<SeriesResponse>("POST", "/api/v1/series", data, { dedupe: false })
	},

	bookmarks: {
		list: (bookId: string) =>
			request<
				Array<{
					id: string;
					book_id: string;
					section_id: string;
					block_index: number;
					title: string | null;
					note: string | null;
					created_at: string;
				}>
			>("GET", `/api/v1/bookmarks/${bookId}`),
		create: (data: {
			book_id: string;
			section_id: string;
			block_index: number;
			title?: string;
			note?: string;
		}) =>
			request<Record<string, unknown>>("POST", `/api/v1/bookmarks/${data.book_id}`, data, {
				dedupe: false
			}),
		delete: (bookmarkId: string) =>
			request<void>("DELETE", `/api/v1/bookmarks/single/${bookmarkId}`, undefined, {
				dedupe: false
			})
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
		revoke: (id: string) =>
			request<void>("DELETE", `/api/v1/auth/sessions/${id}`, undefined, { dedupe: false })
	},

	jobs: {
		get: (id: string) =>
			request<{
				id: string;
				kind: string;
				status: string;
				error: string | null;
				retry_count: number;
				max_retries: number;
				created_at: string;
			}>("GET", `/api/v1/jobs/${id}`, undefined, { dedupe: false })
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
		cleanupSessions: () =>
			request<{ deleted: number }>("GET", "/api/v1/admin/sessions/cleanup", undefined, {
				dedupe: false
			})
	}
};
