import { browser } from "$app/environment";
import { api } from "$lib/api/client.svelte";

export type AppTheme = "light" | "dark";

let current = $state<AppTheme>("light");

function apply(t: AppTheme) {
	if (!browser) return;
	document.documentElement.classList.toggle("dark", t === "dark");
}

if (browser) {
	const saved = localStorage.getItem("bookvault-theme") as AppTheme | null;
	const initial: AppTheme = saved === "light" || saved === "dark" ? saved : "light";
	current = initial;
	apply(initial);
}

export function getTheme(): AppTheme {
	return current;
}

export function toggleTheme() {
	const next: AppTheme = current === "light" ? "dark" : "light";
	current = next;
	if (!browser) return;
	localStorage.setItem("bookvault-theme", next);
	apply(next);
	api.auth.updatePreferences({ theme: next });
}

export function setTheme(t: AppTheme) {
	current = t;
	if (!browser) return;
	localStorage.setItem("bookvault-theme", t);
	apply(t);
	api.auth.updatePreferences({ theme: t });
}
