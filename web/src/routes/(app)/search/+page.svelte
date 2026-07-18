<script lang="ts">
	import * as m from "$lib/paraglide/messages";
	import { api } from "$lib/api/client.svelte";
	import type { BookHit, ContentHit } from "$lib/api/generated";
	import Search from "@lucide/svelte/icons/search";
	import SearchX from "@lucide/svelte/icons/search-x";
	import BookCover from "$lib/components/BookCover.svelte";
	import FileText from "@lucide/svelte/icons/file-text";
	import { resolve } from "$app/paths";
	import { SvelteMap, SvelteSet } from "svelte/reactivity";

	let query = $state("");
	let bookResults = $state<(BookHit & { format?: string })[]>([]);
	let contentResults = $state<ContentHit[]>([]);
	let bookMeta = $state<Map<string, { title: string; author: string | null }>>(new Map());
	let searched = $state(false);
	let searching = $state(false);

	function formatBadge(format: string | undefined): string {
		if (!format) return "";
		if (format === "mobi_raw") return "MOBI";
		if (format === "cbz") return "CBZ";
		return format.toUpperCase();
	}

	async function handleSearch(e?: Event) {
		e?.preventDefault();
		if (!query.trim()) return;
		searched = true;
		searching = true;

		const result = await api.search(query);
		if (result.isOk()) {
			const { books, content_hits } = result.value;
			contentResults = content_hits as unknown as ContentHit[];

			const metaResults = await Promise.all(books.map((h) => api.books.get(h.id)));
			bookResults = books.map((h, i) => {
				const meta = metaResults[i];
				return { ...h, format: meta.isOk() ? meta.value.format : undefined };
			});

			const meta = new SvelteMap<string, { title: string; author: string | null }>();
			const allIds = new SvelteSet(books.map((b) => b.id));
			content_hits.forEach((ch) => allIds.add(ch.book_id));
			const entries = await Promise.all(
				[...allIds].map(async (id) => {
					const r = await api.books.get(id);
					return r.isOk()
						? ([r.value.id, { title: r.value.title, author: r.value.author }] as const)
						: null;
				})
			);
			entries.filter(Boolean).forEach((e) => {
				if (e) meta.set(e[0], e[1]);
			});
			bookMeta = meta;
		}
		searching = false;
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

	{#if searching}
		<div class="flex items-center justify-center py-16">
			<div
				class="border-secondary h-8 w-8 animate-spin rounded-full border-2 border-t-transparent"
			></div>
		</div>
	{:else if searched}
		{#if bookResults.length === 0 && contentResults.length === 0}
			<div class="py-16 text-center">
				<SearchX size={32} class="text-on-surface-variant/30 mb-4 block" />
				<p class="font-body text-body-md text-on-surface-variant">
					{m.search_no_results({ query })}
				</p>
			</div>
		{:else}
			{#if bookResults.length > 0}
				<div class="mb-10">
					<h3 class="font-display text-headline-sm text-primary mb-4">Books</h3>
					<div class="space-y-3">
						{#each bookResults as hit (hit.id)}
							<a
								href={resolve(`/reader/${hit.id}`)}
								class="paper-card flex items-center gap-5 rounded-xl p-5 transition-all hover:shadow-md"
							>
								<div
									class="bg-surface-container flex h-14 w-10 shrink-0 items-center justify-center rounded-lg"
								>
									<BookCover bookId={hit.id} class="h-14 w-10 shrink-0 rounded-lg" />
								</div>
								<div class="min-w-0 flex-1">
									<div class="flex items-center gap-2">
										<h3 class="font-display text-headline-sm truncate">{hit.title}</h3>
										{#if hit.format}
											<span
												class="font-label bg-surface-container-high shrink-0 rounded px-2 py-0.5 text-[10px] tracking-wider uppercase"
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
				</div>
			{/if}

			{#if contentResults.length > 0}
				<div>
					<h3 class="font-display text-headline-sm text-primary mb-4">Content Matches</h3>
					<div class="space-y-3">
						{#each contentResults as hit (hit.book_id + hit.section_id + hit.block_index)}
							{@const meta = bookMeta.get(hit.book_id)}
							<a
								href={resolve(`/reader/${hit.book_id}#section-${hit.section_id}`)}
								class="paper-card block rounded-xl p-5 transition-all hover:shadow-md"
							>
								<div class="mb-2 flex items-center gap-2">
									<FileText size={14} class="text-secondary" />
									<p class="font-label text-label-md text-secondary truncate">
										{meta?.title ?? "Unknown Book"}
									</p>
									<span
										class="font-label text-label-sm text-on-surface-variant/50 ml-auto whitespace-nowrap"
										>score: {hit.score.toFixed(1)}</span
									>
								</div>
								<p class="font-body text-body-md text-on-surface-variant leading-relaxed">
									{hit.snippet}
								</p>
							</a>
						{/each}
					</div>
				</div>
			{/if}
		{/if}
	{/if}
</section>
