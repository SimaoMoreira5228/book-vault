<script lang="ts">
	import * as m from "$lib/paraglide/messages";
	import { api } from "$lib/api/client";
	import type { BookHit } from "$lib/api/generated";
	import Search from "@lucide/svelte/icons/search";
	import SearchX from "@lucide/svelte/icons/search-x";
	import BookOpen from "@lucide/svelte/icons/book-open";

	let query = $state("");
	let results = $state<(BookHit & { format?: string })[]>([]);
	let searched = $state(false);

	function formatBadge(format: string | undefined): string {
		if (!format) return "";
		if (format === "mobi_raw") return "MOBI";
		if (format === "cbz") return "CBZ";
		return format.toUpperCase();
	}

	async function handleSearch() {
		if (!query.trim()) return;
		searched = true;
		const result = await api.search(query);
		if (result.isOk()) {
			const hits = result.value.books;
			const metaResults = await Promise.all(hits.map((h) => api.books.get(h.id)));
			results = hits.map((h, i) => {
				const meta = metaResults[i];
				return {
					...h,
					format: meta.isOk() ? meta.value.format : undefined
				};
			});
		}
	}
</script>

<section class="mb-section-gap">
	<header class="mb-8">
		<span class="font-label text-label-sm text-secondary mb-2 block tracking-widest uppercase"
			>{m.search_subtitle()}</span
		>
		<h2 class="font-display text-headline-md">{m.search_title()}</h2>
	</header>

	<form onsubmit={handleSearch} class="mb-12">
		<div class="relative">
			<Search size={20} class="text-on-surface-variant absolute top-1/2 left-4 -translate-y-1/2" />
			<input
				type="text"
				bind:value={query}
				placeholder={m.search_placeholder()}
				class="bg-surface-container-low border-outline/10 font-body text-body-md text-on-surface placeholder:text-on-surface-variant/50 focus:ring-primary/10 w-full rounded-xl border py-4 pr-4 pl-12 focus:ring-2 focus:outline-none"
			/>
		</div>
	</form>

	{#if searched}
		{#if results.length === 0}
			<div class="py-16 text-center">
				<SearchX size={32} class="text-on-surface-variant/30 mb-4 block" />
				<p class="font-body text-body-md text-on-surface-variant">
					{m.search_no_results({ query })}
				</p>
			</div>
		{:else}
			<div class="space-y-4">
				{#each results as hit (hit.id)}
					<a
						href="/reader/{hit.id}"
						class="paper-card flex items-center gap-6 rounded-xl p-6 transition-all hover:shadow-lg"
					>
						<div class="bg-surface-container h-16 w-12 flex-shrink-0 overflow-hidden rounded-lg">
							<div class="flex h-full w-full items-center justify-center">
								<BookOpen size={20} class="text-on-surface-variant/30" />
							</div>
						</div>
						<div class="min-w-0 flex-1">
							<div class="mb-1 flex items-center gap-2">
								<h3 class="font-display text-headline-sm truncate">{hit.title}</h3>
								{#if hit.format}
									<span
										class="font-label bg-surface-container-high flex-shrink-0 rounded px-2 py-0.5 text-[10px] tracking-wider uppercase"
										>{formatBadge(hit.format)}</span
									>
								{/if}
							</div>
							<p class="font-label text-label-sm text-on-surface-variant">
								{hit.author ?? m.library_unknown_author()}
							</p>
						</div>
					</a>
				{/each}
			</div>
		{/if}
	{/if}
</section>
