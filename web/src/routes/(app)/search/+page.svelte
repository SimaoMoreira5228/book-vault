<script lang="ts">
	import { api } from '$lib/api/client';
	import type { BookHit } from '$lib/api/generated';
	import Search from '@lucide/svelte/icons/search';
	import SearchX from '@lucide/svelte/icons/search-x';
	import BookOpen from '@lucide/svelte/icons/book-open';

	let query = $state('');
	let results = $state<BookHit[]>([]);
	let searched = $state(false);

	async function handleSearch() {
		if (!query.trim()) return;
		searched = true;
		const result = await api.search(query);
		if (result.isOk()) {
			results = result.value.books;
		}
	}
</script>

<section class="mb-section-gap">
	<header class="mb-8">
		<span class="font-label text-label-sm uppercase tracking-widest text-secondary mb-2 block">Discover</span>
		<h2 class="font-display text-headline-md">Search Library</h2>
	</header>

	<form onsubmit={handleSearch} class="mb-12">
		<div class="relative">
			<Search size={20} class="absolute left-4 top-1/2 -translate-y-1/2 text-on-surface-variant" />
			<input
				type="text"
				bind:value={query}
				placeholder="Search by title or author..."
				class="w-full pl-12 pr-4 py-4 bg-surface-container-low rounded-xl border border-outline/10 font-body text-body-md text-on-surface placeholder:text-on-surface-variant/50 focus:outline-none focus:ring-2 focus:ring-primary/10"
			/>
		</div>
	</form>

	{#if searched}
		{#if results.length === 0}
			<div class="text-center py-16">
				<SearchX size={32} class="text-on-surface-variant/30 block mb-4" />
				<p class="font-body text-body-md text-on-surface-variant">No results found for "{query}"</p>
			</div>
		{:else}
			<div class="space-y-4">
				{#each results as hit}
					<a href="/reader/{hit.id}" class="paper-card rounded-xl p-6 flex items-center gap-6 hover:shadow-lg transition-all">
						<div class="w-12 h-16 rounded-lg overflow-hidden bg-surface-container flex-shrink-0">
							<div class="w-full h-full flex items-center justify-center">
								<BookOpen size={20} class="text-on-surface-variant/30" />
							</div>
						</div>
						<div>
							<h3 class="font-display text-headline-sm mb-1">{hit.title}</h3>
							<p class="font-label text-label-sm text-on-surface-variant">{hit.author ?? 'Unknown Author'}</p>
						</div>
					</a>
				{/each}
			</div>
		{/if}
	{/if}
</section>
