<script lang="ts">
	import * as m from "$lib/paraglide/messages";
	import { page } from "$app/state";
	import { resolve } from "$app/paths";
	import ArrowLeft from "@lucide/svelte/icons/arrow-left";
	import Save from "@lucide/svelte/icons/save";
	import Eye from "@lucide/svelte/icons/eye";
	import Menu from "@lucide/svelte/icons/menu";
	import Clock from "@lucide/svelte/icons/clock";

	let {
		title = "",
		saving = false,
		lastSaved = "",
		children,
		sidebar
	}: {
		title?: string;
		saving?: boolean;
		lastSaved?: string;
		children?: import("svelte").Snippet;
		sidebar?: import("svelte").Snippet;
	} = $props();

	let mobileSidebarOpen = $state(false);
</script>

<div class="flex h-screen flex-col bg-[#fbf9f5] dark:bg-[#121212]">
	<header
		class="bg-surface/90 fixed top-0 z-50 flex h-14 w-full items-center justify-between border-b border-[rgba(0,31,63,0.05)] px-4 shadow-sm backdrop-blur-md"
	>
		<div class="flex items-center gap-3">
			<button
				onclick={() => (mobileSidebarOpen = true)}
				class="text-on-surface-variant hover:text-primary -ml-1 flex items-center justify-center p-1 transition-colors lg:hidden"
				aria-label="Open menu"
			>
				<Menu size={20} />
			</button>
			<a
				href={resolve("/studio")}
				class="text-on-surface-variant hover:text-primary flex items-center gap-1.5 p-1 transition-colors"
			>
				<ArrowLeft size={18} />
			</a>
			<h1
				class="font-display text-headline-sm text-primary ml-2 max-w-[300px] truncate md:max-w-md"
			>
				{title || "Untitled"}
			</h1>
		</div>
		<div class="flex items-center gap-2">
			{#if lastSaved}
				<span
					class="font-label text-label-sm text-on-surface-variant/50 hidden items-center gap-1 md:flex"
				>
					<Clock size={14} />
					{lastSaved}
				</span>
			{/if}
			<a
				href={resolve(`/reader/${page.params.id}`)}
				class="text-on-surface-variant hover:text-primary p-2 transition-colors"
				title="Preview"
			>
				<Eye size={20} />
			</a>
			<button disabled={saving} class="btn-primary">
				<Save size={16} />
				{saving ? m.studio_saving() : m.studio_save()}
			</button>
		</div>
	</header>

	<div class="flex flex-1 pt-14">
		<aside
			class="bg-surface-container-low hidden w-72 flex-col border-r border-[rgba(0,31,63,0.05)] lg:flex"
		>
			{#if sidebar}
				{@render sidebar()}
			{/if}
		</aside>

		{#if mobileSidebarOpen}
			<div
				class="fixed inset-0 z-40 bg-black/40 backdrop-blur-sm lg:hidden"
				onclick={() => (mobileSidebarOpen = false)}
				onkeydown={(e) => {
					if (e.key === "Escape") mobileSidebarOpen = false;
				}}
				role="dialog"
				aria-modal="true"
				tabindex="-1"
			>
				<div
					class="bg-surface h-full w-72 overflow-y-auto py-6 shadow-xl"
					onclick={(e) => e.stopPropagation()}
					onkeydown={(e) => {
						if (e.key === "Escape") mobileSidebarOpen = false;
					}}
					role="none"
				>
					{#if sidebar}
						{@render sidebar()}
					{/if}
				</div>
			</div>
		{/if}

		<main class="flex flex-1 flex-col items-center overflow-y-auto">
			<div class="w-full max-w-3xl px-4 py-10 md:px-8 lg:px-12">
				{#if children}
					{@render children()}
				{/if}
			</div>
		</main>
	</div>
</div>
