<script lang="ts">
	import { api, authState } from '$lib/api/client';
	import type { BookResponse } from '$lib/api/generated';
	import BookOpen from '@lucide/svelte/icons/book-open';
	import LibraryBig from '@lucide/svelte/icons/library-big';
	import PlusCircle from '@lucide/svelte/icons/plus-circle';

	let books = $state<BookResponse[]>([]);
	let loading = $state(true);

	$effect(() => {
		if (authState.isAuthenticated) {
			loadBooks();
		}
	});

	async function loadBooks() {
		loading = true;
		const result = await api.books.list();
		if (result.isOk()) {
			books = result.value;
		}
		loading = false;
	}

	const currentlyReading = $derived(books.filter((b) => b.read_status === 'reading'));
	const forYou = $derived(books.filter((b) => b.read_status !== 'reading'));
</script>

{#if loading}
	<div class="flex items-center justify-center py-32">
		<div class="w-8 h-8 border-2 border-secondary border-t-transparent rounded-full animate-spin" />
	</div>
{:else}
	<!-- Currently Reading Section -->
	<section class="mb-section-gap">
		<header class="flex items-end justify-between mb-8">
			<div>
				<span class="font-label text-label-sm uppercase tracking-widest text-secondary mb-2 block">Resume Journey</span>
				<h2 class="font-display text-headline-md">Currently Reading</h2>
			</div>
			<a href="/search" class="font-label text-label-md text-secondary border-b border-transparent hover:border-secondary transition-all">View All</a>
		</header>

		{#if currentlyReading.length === 0}
			<div class="paper-card rounded-xl p-12 text-center">
				<BookOpen size={32} class="text-on-surface-variant/30 block mb-4" />
				<p class="font-body text-body-md text-on-surface-variant">No books being read yet. Upload or create your first book!</p>
			</div>
		{:else}
			<div class="grid grid-cols-1 md:grid-cols-2 gap-gutter">
				{#each currentlyReading as book}
					<a href="/reader/{book.id}" class="paper-card rounded-xl overflow-hidden flex flex-col sm:flex-row h-full hover:shadow-lg transition-all">
						<div class="relative w-full sm:w-1/3 aspect-[2/3] sm:aspect-auto bg-surface-container">
							<div class="absolute inset-0 book-spine-effect" />
							<div class="w-full h-full flex items-center justify-center">
								<BookOpen size={32} class="text-on-surface-variant/30" />
							</div>
						</div>
						<div class="p-6 flex flex-col justify-between flex-1">
							<div>
								<div class="flex items-center gap-2 text-on-surface-variant mb-2">
									<span class="font-label text-label-sm uppercase">{book.format}</span>
								</div>
								<h3 class="font-display text-headline-sm mb-1">{book.title}</h3>
								<p class="font-body text-body-md text-on-surface-variant italic">{book.author ?? 'Unknown Author'}</p>
							</div>
							<div class="mt-6">
								<div class="flex justify-between items-end mb-2">
									<span class="font-label text-label-sm text-on-surface-variant uppercase">In Progress</span>
								</div>
								<div class="reading-progress-bar rounded-full overflow-hidden">
									<div class="reading-progress-fill" style="width: 40%;" />
								</div>
							</div>
						</div>
					</a>
				{/each}
			</div>
		{/if}
	</section>

	<!-- For You Section -->
	<section>
		<header class="flex items-end justify-between mb-8">
			<div>
				<span class="font-label text-label-sm uppercase tracking-widest text-secondary mb-2 block">Curated Selection</span>
				<h2 class="font-display text-headline-md">For You</h2>
			</div>
		</header>

		{#if forYou.length === 0 && !loading}
			<div class="border-2 border-dashed border-outline-variant/30 rounded-xl p-12 text-center">
				<LibraryBig size={32} class="text-on-surface-variant/30 block mb-4" />
				<p class="font-body text-body-md text-on-surface-variant">Your library is empty. Upload an EPUB or PDF to get started.</p>
			</div>
		{:else}
			<div class="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-gutter">
				{#each forYou as book}
					<a href="/reader/{book.id}" class="group cursor-pointer">
						<div class="relative aspect-[2/3] rounded-xl overflow-hidden mb-4 paper-card border-none bg-surface-container">
							<div class="absolute inset-0 book-spine-effect" />
							<div class="w-full h-full flex items-center justify-center">
								<BookOpen size={40} class="text-on-surface-variant/20" />
							</div>
						</div>
						<h4 class="font-label text-label-md text-primary mb-1 group-hover:text-secondary transition-colors">{book.title}</h4>
						<p class="font-label text-label-sm text-on-surface-variant">{book.author ?? 'Unknown Author'}</p>
					</a>
				{/each}
				<!-- Empty slot for new book -->
				<a href="/studio" class="border-2 border-dashed border-outline-variant/30 rounded-xl flex flex-col items-center justify-center aspect-[2/3] hover:bg-surface-container/30 transition-colors cursor-pointer group">
					<PlusCircle size={28} class="text-on-surface-variant/40 group-hover:scale-110 transition-transform" />
					<span class="font-label text-label-sm text-on-surface-variant mt-2">Add Book</span>
				</a>
			</div>
		{/if}
	</section>
{/if}
