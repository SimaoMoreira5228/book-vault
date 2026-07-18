import type { ClientInit } from "@sveltejs/kit";
import { Locale } from "$lib/paraglide.svelte";
import { authState } from "$lib/api/client.svelte";

export const init: ClientInit = () => {
	new Locale();
	authState.restore();
};
