<script lang="ts">
	import * as m from "$lib/paraglide/messages";
	import { authState } from "$lib/api/client.svelte";
	import { page } from "$app/state";
	import { goto } from "$app/navigation";
	import { resolve } from "$app/paths";
	import BookOpen from "@lucide/svelte/icons/book-open";
	import Search from "@lucide/svelte/icons/search";
	import PenSquare from "@lucide/svelte/icons/pen-square";
	import ScrollText from "@lucide/svelte/icons/scroll-text";
	import Bell from "@lucide/svelte/icons/bell";
	import Settings from "@lucide/svelte/icons/settings";
	import Menu from "@lucide/svelte/icons/menu";
	import X from "@lucide/svelte/icons/x";
	import LibraryBig from "@lucide/svelte/icons/library-big";
	import Users from "@lucide/svelte/icons/users";
	import BookMarked from "@lucide/svelte/icons/book-marked";
	import Plus from "@lucide/svelte/icons/plus";

	let { children } = $props();

	let path = $derived(page.url.pathname);

	$effect(() => {
		if (!authState.restoring && !authState.isAuthenticated) {
			goto(resolve("/login"));
		}
	});

	const isReader = $derived(path.startsWith("/reader"));

	type NavItem = {
		href: "/" | "/search" | "/studio" | "/notes" | "/authors" | "/series" | "/settings";
		icon:
			| typeof LibraryBig
			| typeof Search
			| typeof PenSquare
			| typeof ScrollText
			| typeof Users
			| typeof BookMarked
			| typeof Settings;
		label: string;
		match: string;
	};

	const navItems: NavItem[] = [
		{ href: "/", icon: LibraryBig, label: m.nav_library(), match: "/library" },
		{ href: "/search", icon: Search, label: m.nav_search(), match: "/search" },
		{ href: "/studio", icon: PenSquare, label: m.nav_studio(), match: "/studio" },
		{ href: "/notes", icon: ScrollText, label: m.nav_notes(), match: "/notes" }
	];

	const sidebarItems: NavItem[] = [
		{ href: "/", icon: LibraryBig, label: m.nav_library(), match: "/library" },
		{ href: "/search", icon: Search, label: m.nav_search(), match: "/search" },
		{ href: "/studio", icon: PenSquare, label: m.nav_studio(), match: "/studio" },
		{ href: "/notes", icon: ScrollText, label: m.nav_notes(), match: "/notes" },
		{ href: "/authors", icon: Users, label: m.authors_title(), match: "/authors" },
		{ href: "/series", icon: BookMarked, label: m.series_title(), match: "/series" }
	];

	function isActive(item: { match: string }): boolean {
		if (item.match === "/library") return path === "/" || path.startsWith("/reader");
		return path.startsWith(item.match);
	}

	let sidebarOpen = $state(false);
</script>

{#if isReader}
	{@render children()}
{:else}
	<div class="bg-surface min-h-screen">
		<header
			class="bg-surface/90 fixed top-0 right-0 left-0 z-50 flex h-16 items-center justify-between px-4 shadow-[0_1px_4px_rgba(0,31,63,0.05)] backdrop-blur-md lg:pr-8 lg:pl-6"
		>
			<div class="flex items-center gap-3">
				<button
					onclick={() => (sidebarOpen = true)}
					class="text-primary -ml-1 flex items-center justify-center p-1 lg:hidden"
					aria-label="Open menu"
				>
					<Menu size={22} />
				</button>
				<a href={resolve("/")} class="flex items-center gap-2.5">
					<div
						class="border-outline/10 bg-surface-container h-8 w-8 overflow-hidden rounded-full border"
					>
						<BookOpen size={18} class="text-primary mx-auto mt-1" />
					</div>
					<span class="font-display text-display-mobile text-primary hidden sm:block"
						>{m.app_name()}</span
					>
				</a>
			</div>
			<div class="flex items-center gap-1">
				<a
					href={resolve("/search")}
					class="btn-ghost text-on-surface-variant hover:text-primary hidden items-center gap-2 rounded-xl px-3 py-2 text-sm transition-colors lg:flex"
				>
					<Search size={16} />
					<span class="font-label text-label-sm opacity-60">Ctrl+K</span>
				</a>
				<button class="btn-ghost text-on-surface-variant hover:text-primary p-2 transition-colors">
					<Bell size={20} />
				</button>
				<a
					href={resolve("/settings")}
					class="btn-ghost text-on-surface-variant hover:text-primary p-2 transition-colors"
				>
					<Settings size={20} />
				</a>
				<div class="border-primary/10 ml-2 h-8 w-8 overflow-hidden rounded-full border-2">
					<div
						class="bg-primary-container flex h-full w-full items-center justify-center text-sm font-medium text-white"
					>
						{authState.user?.display_name?.charAt(0)?.toUpperCase() ?? "?"}
					</div>
				</div>
			</div>
		</header>

		{#if sidebarOpen}
			<div
				class="fixed inset-0 z-40 bg-black/40 backdrop-blur-sm lg:hidden"
				onclick={() => (sidebarOpen = false)}
				onkeydown={(e) => {
					if (e.key === "Escape") sidebarOpen = false;
				}}
				role="dialog"
				aria-modal="true"
				tabindex="-1"
			>
				<div
					class="bg-surface h-full w-72 overflow-y-auto px-4 py-6 shadow-xl"
					onclick={(e) => e.stopPropagation()}
					onkeydown={(e) => {
						if (e.key === "Escape") sidebarOpen = false;
					}}
					role="none"
				>
					<div class="mb-8 flex items-center justify-between">
						<span class="font-display text-headline-sm text-primary">{m.app_name()}</span>
						<button
							onclick={() => (sidebarOpen = false)}
							class="text-on-surface-variant hover:text-primary p-1"
						>
							<X size={20} />
						</button>
					</div>
					<nav class="flex flex-col gap-1">
						{#each sidebarItems as item (item.href)}
							<a
								href={resolve(item.href)}
								onclick={() => (sidebarOpen = false)}
								class={[
									"font-label text-label-md flex items-center gap-3 rounded-xl px-4 py-3 transition-all",
									isActive(item)
										? "bg-secondary/5 text-secondary font-medium"
										: "text-on-surface-variant hover:bg-surface-container-low hover:text-primary"
								]}
							>
								<item.icon size={18} />
								{item.label}
							</a>
						{/each}
					</nav>
					<div class="mt-8">
						<a
							href={resolve("/settings")}
							onclick={() => (sidebarOpen = false)}
							class={[
								"font-label text-label-md flex items-center gap-3 rounded-xl px-4 py-3 transition-all",
								isActive({ match: "/settings" })
									? "bg-secondary/5 text-secondary font-medium"
									: "text-on-surface-variant hover:bg-surface-container-low hover:text-primary"
							]}
						>
							<Settings size={18} />
							{m.settings_title()}
						</a>
					</div>
				</div>
			</div>
		{/if}

		<aside
			class="fixed top-16 bottom-0 left-0 z-30 hidden w-64 flex-col overflow-y-auto border-r border-[rgba(0,31,63,0.05)] px-3 py-6 lg:flex"
		>
			<nav class="flex flex-col gap-0.5">
				{#each sidebarItems as item (item.href)}
					<a
						href={resolve(item.href)}
						class={[
							"font-label text-label-md flex items-center gap-3 rounded-xl px-4 py-2.5 transition-all",
							isActive(item)
								? "bg-secondary/5 text-secondary font-medium"
								: "text-on-surface-variant hover:bg-surface-container-low hover:text-primary"
						]}
					>
						<item.icon size={18} />
						{item.label}
					</a>
				{/each}
			</nav>
			<div class="mt-auto pt-6">
				<a
					href={resolve("/settings")}
					class={[
						"font-label text-label-md flex items-center gap-3 rounded-xl px-4 py-2.5 transition-all",
						isActive({ match: "/settings" })
							? "bg-secondary/5 text-secondary font-medium"
							: "text-on-surface-variant hover:bg-surface-container-low hover:text-primary"
					]}
				>
					<Settings size={18} />
					{m.settings_title()}
				</a>
			</div>
		</aside>

		<main class="pt-24 pb-32 lg:ml-64 lg:pt-28">
			<div class="max-w-container-max mx-auto px-4 md:px-8">
				{@render children()}
			</div>
		</main>

		<nav
			class="bg-surface fixed right-0 bottom-0 left-0 z-30 flex h-20 items-center justify-around border-t border-[rgba(0,31,63,0.05)] px-2 lg:hidden"
		>
			{#each navItems as item (item.href)}
				<a
					href={resolve(item.href)}
					class={[
						"flex flex-col items-center justify-center px-4 py-1 transition-colors active:scale-90",
						isActive(item)
							? "text-secondary"
							: "text-on-surface-variant opacity-60 hover:opacity-100"
					]}
				>
					<item.icon size={20} />
					<span class="font-label text-label-sm mt-1">{item.label}</span>
				</a>
			{/each}
		</nav>
	</div>
{/if}
