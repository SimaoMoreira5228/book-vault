<script lang="ts">
	import * as m from "$lib/paraglide/messages";
	import { api, authState } from "$lib/api/client.svelte";
	import { goto } from "$app/navigation";
	import { resolve } from "$app/paths";
	import Bookmark from "@lucide/svelte/icons/bookmark";

	type Series = { id: string; name: string; description: string | null; book_count: number };

	let series = $state<Series[]>([]);
	let loading = $state(true);

	$effect(() => {
		if (!authState.isAuthenticated) {
			goto(resolve("/login"));
			return;
		}
		loadSeries();
	});

	async function loadSeries() {
		loading = true;
		const r = await api.series.list();
		if (r.isOk()) series = r.value as unknown as Series[];
		loading = false;
	}
</script>

<svelte:head><title>{m.series_title()} — Book Vault</title></svelte:head>

<section>
	<header class="mb-8">
		<span class="font-label text-label-sm text-secondary mb-2 block tracking-widest uppercase"
			>{m.series_subtitle()}</span
		>
		<h2 class="font-display text-headline-md">{m.series_title()}</h2>
	</header>

	{#if loading}
		<div class="flex items-center justify-center py-32">
			<div
				class="border-secondary h-8 w-8 animate-spin rounded-full border-2 border-t-transparent"
			></div>
		</div>
	{:else if series.length === 0}
		<div class="paper-card rounded-xl p-16 text-center">
			<Bookmark size={40} class="text-on-surface-variant/20 mb-4 block" />
			<p class="font-body text-body-md text-on-surface-variant">{m.series_empty()}</p>
		</div>
	{:else}
		<div class="gap-gutter grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3">
			{#each series as s (s.id)}
				<a
					href={resolve(`/series/${s.id}`)}
					class="paper-card group rounded-xl p-6 transition-all hover:shadow-lg"
				>
					<div class="mb-4 flex items-start gap-4">
						<div class="bg-surface-container flex h-10 w-10 items-center justify-center rounded-lg">
							<Bookmark size={18} class="text-secondary" />
						</div>
						<div class="min-w-0 flex-1">
							<h3 class="font-display text-headline-sm truncate">{s.name}</h3>
							<p class="font-label text-label-sm text-on-surface-variant">
								{m.series_book_count({ count: s.book_count })}
							</p>
						</div>
					</div>
					{#if s.description}<p class="font-body text-body-md text-on-surface-variant line-clamp-2">
							{s.description}
						</p>{/if}
				</a>
			{/each}
		</div>
	{/if}
</section>
