<script lang="ts">
	import * as m from "$lib/paraglide/messages";
	import { authState } from "$lib/api/client.svelte";
	import { page } from "$app/state";
	import { goto } from "$app/navigation";
	import BookOpen from "@lucide/svelte/icons/book-open";
	import Search from "@lucide/svelte/icons/search";
	import PenSquare from "@lucide/svelte/icons/pen-square";
	import ScrollText from "@lucide/svelte/icons/scroll-text";
	import Bell from "@lucide/svelte/icons/bell";
	import Settings from "@lucide/svelte/icons/settings";

	let { children } = $props();

	let path = $derived(page.url.pathname);

	$effect(() => {
		if (!authState.restoring && !authState.isAuthenticated) {
			goto("/login");
		}
	});

	const isCurrent = (p: string) => {
		if (p === "/library") return path === "/" || path.startsWith("/reader");
		return path.startsWith(p);
	};
</script>

<div class="bg-surface min-h-screen">
	<header
		class="bg-surface px-margin-mobile fixed top-0 z-50 flex h-16 w-full items-center justify-between shadow-[0_1px_4px_rgba(0,31,63,0.05)]"
	>
		<div
			class="flex cursor-pointer items-center gap-3 transition-transform duration-200 active:scale-95"
		>
			<div
				class="border-outline/10 bg-surface-container h-8 w-8 overflow-hidden rounded-full border"
			></div>
			<h1 class="font-display text-display-mobile text-primary">{m.app_name()}</h1>
		</div>
		<div class="flex items-center gap-2">
			<button class="text-primary transition-opacity hover:opacity-80 active:scale-95">
				<Bell size={20} />
			</button>
			<a href="/settings" class="text-primary transition-opacity hover:opacity-80 active:scale-95">
				<Settings size={20} />
			</a>
		</div>
	</header>

	<main class="max-w-container-max px-margin-mobile md:px-margin-desktop mx-auto pt-24 pb-32">
		{@render children()}
	</main>

	<nav
		class="bg-surface px-margin-mobile fixed bottom-0 z-50 flex h-20 w-full items-center justify-around shadow-[0_-4px_16px_rgba(0,31,63,0.02)]"
	>
		<a
			href="/"
			class={[
				"flex flex-col items-center justify-center px-4 py-1 transition-colors active:scale-90",
				isCurrent("/library")
					? "text-secondary bg-secondary-container/10 rounded-full"
					: "text-on-surface-variant opacity-70"
			]}
		>
			<BookOpen size={20} />
			<span class="font-label text-label-sm mt-1">{m.nav_library()}</span>
		</a>
		<a
			href="/search"
			class={[
				"flex flex-col items-center justify-center px-4 py-1 transition-colors active:scale-90",
				isCurrent("/search")
					? "text-secondary bg-secondary-container/10 rounded-full"
					: "text-on-surface-variant opacity-70"
			]}
		>
			<Search size={20} />
			<span class="font-label text-label-sm mt-1">{m.nav_search()}</span>
		</a>
		<a
			href="/studio"
			class={[
				"flex flex-col items-center justify-center px-4 py-1 transition-colors active:scale-90",
				isCurrent("/studio")
					? "text-secondary bg-secondary-container/10 rounded-full"
					: "text-on-surface-variant opacity-70"
			]}
		>
			<PenSquare size={20} />
			<span class="font-label text-label-sm mt-1">{m.nav_studio()}</span>
		</a>
		<a
			href="/notes"
			class={[
				"flex flex-col items-center justify-center px-4 py-1 transition-colors active:scale-90",
				isCurrent("/notes")
					? "text-secondary bg-secondary-container/10 rounded-full"
					: "text-on-surface-variant opacity-70"
			]}
		>
			<ScrollText size={20} />
			<span class="font-label text-label-sm mt-1">{m.nav_notes()}</span>
		</a>
	</nav>
</div>
