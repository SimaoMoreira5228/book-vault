function resolveBaseUrl(): string {
	if (typeof window !== "undefined" && (window as Record<string, unknown>).__API_URL__) {
		return (window as Record<string, unknown>).__API_URL__ as string;
	}
	try {
		return import.meta.env.PUBLIC_API_URL ?? "";
	} catch {
		return "";
	}
}

const base = resolveBaseUrl();

export const apiConfig = {
	baseUrl: base,
	credentials: base && !base.startsWith("/") ? "include" : "same-origin"
} as const;
