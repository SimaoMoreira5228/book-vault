<script lang="ts">
	import * as m from "$lib/paraglide/messages";
	import { api, authState } from "$lib/api/client";
	import type { BookResponse } from "$lib/api/generated";
	import BookOpen from "@lucide/svelte/icons/book-open";
	import LibraryBig from "@lucide/svelte/icons/library-big";
	import PlusCircle from "@lucide/svelte/icons/plus-circle";

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

	const currentlyReading = $derived(books.filter((b) => b.read_status === "reading"));
	const forYou = $derived(books.filter((b) => b.read_status !== "reading"));

	function formatBadge(format: string): string {
		if (format === "mobi_raw") return m.book_format_mobi();
		if (format === "cbz") return m.book_format_cbz();
		if (format === "pdf") return m.book_format_pdf();
		if (format === "epub") return m.book_format_epub();
		if (format === "native") return m.book_format_native();
		if (format === "bvir") return m.book_format_bvir();
		return format.toUpperCase();
	}
</script>

{#if loading}
	<div class="flex items-center justify-center py-32">
		<div class="border-secondary h-8 w-8 animate-spin rounded-full border-2 border-t-transparent" />
	</div>
{:else}
	<!-- Currently Reading Section -->
	<section class="mb-section-gap">
		<header class="mb-8 flex items-end justify-between">
			<div>
				<span class="font-label text-label-sm text-secondary mb-2 block tracking-widest uppercase"
					>{m.library_reading_subtitle()}</span
				>
				<h2 class="font-display text-headline-md">{m.library_reading_title()}</h2>
			</div>
			<a
				href="/search"
				class="font-label text-label-md text-secondary hover:border-secondary border-b border-transparent transition-all"
				>{m.library_reading_view_all()}</a
			>
		</header>

		{#if currentlyReading.length === 0}
			<div class="paper-card rounded-xl p-12 text-center">
				<BookOpen size={32} class="text-on-surface-variant/30 mb-4 block" />
				<p class="font-body text-body-md text-on-surface-variant">
					{m.library_reading_empty()}
				</p>
			</div>
		{:else}
			<div class="gap-gutter grid grid-cols-1 md:grid-cols-2">
				{#each currentlyReading as book (book.id)}
					<a
						href="/reader/{book.id}"
						class="paper-card flex h-full flex-col overflow-hidden rounded-xl transition-all hover:shadow-lg sm:flex-row"
					>
						<div class="bg-surface-container relative aspect-[2/3] w-full sm:aspect-auto sm:w-1/3">
							<div class="book-spine-effect absolute inset-0" />
							<div class="flex h-full w-full items-center justify-center">
								<BookOpen size={32} class="text-on-surface-variant/30" />
							</div>
						</div>
						<div class="flex flex-1 flex-col justify-between p-6">
							<div>
								<div class="text-on-surface-variant mb-2 flex items-center gap-2">
									<span
										class="font-label text-label-sm bg-surface-container-high rounded px-2 py-0.5 tracking-wider uppercase"
										>{formatBadge(book.format)}</span
									>
								</div>
								<h3 class="font-display text-headline-sm mb-1">{book.title}</h3>
								<p class="font-body text-body-md text-on-surface-variant italic">
									{book.author ?? m.library_unknown_author()}
								</p>
							</div>
							<div class="mt-6">
								<div class="mb-2 flex items-end justify-between">
									<span class="font-label text-label-sm text-on-surface-variant uppercase"
										>{m.library_reading_progress()}</span
									>
								</div>
								<div class="reading-progress-bar overflow-hidden rounded-full">
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
		<header class="mb-8 flex items-end justify-between">
			<div>
				<span class="font-label text-label-sm text-secondary mb-2 block tracking-widest uppercase"
					>{m.library_for_you_subtitle()}</span
				>
				<h2 class="font-display text-headline-md">{m.library_for_you_title()}</h2>
			</div>
		</header>

		{#if forYou.length === 0 && !loading}
			<div class="border-outline-variant/30 rounded-xl border-2 border-dashed p-12 text-center">
				<LibraryBig size={32} class="text-on-surface-variant/30 mb-4 block" />
				<p class="font-body text-body-md text-on-surface-variant">
					{m.library_for_you_empty()}
				</p>
			</div>
		{:else}
			<div class="gap-gutter grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4">
				{#each forYou as book (book.id)}
					<a href="/reader/{book.id}" class="group cursor-pointer">
						<div
							class="paper-card bg-surface-container relative mb-4 aspect-[2/3] overflow-hidden rounded-xl border-none"
						>
							<div class="book-spine-effect absolute inset-0" />
							<div class="flex h-full w-full items-center justify-center">
								<BookOpen size={40} class="text-on-surface-variant/20" />
							</div>
							{#if book.format === "mobi_raw" || book.format === "cbz"}
								<div class="absolute top-2 right-2">
									<span
										class="font-label text-primary rounded bg-white/90 px-2 py-0.5 text-[10px] tracking-wider uppercase backdrop-blur"
										>{formatBadge(book.format)}</span
									>
								</div>
							{/if}
						</div>
						<h4
							class="font-label text-label-md text-primary group-hover:text-secondary mb-1 transition-colors"
						>
							{book.title}
						</h4>
						<p class="font-label text-label-sm text-on-surface-variant">
							{book.author ?? m.library_unknown_author()}
						</p>
					</a>
				{/each}
				<!-- Empty slot for new book -->
				<a
					href="/studio"
					class="border-outline-variant/30 hover:bg-surface-container/30 group flex aspect-[2/3] cursor-pointer flex-col items-center justify-center rounded-xl border-2 border-dashed transition-colors"
				>
					<PlusCircle
						size={28}
						class="text-on-surface-variant/40 transition-transform group-hover:scale-110"
					/>
					<span class="font-label text-label-sm text-on-surface-variant mt-2"
						>{m.library_add_book()}</span
					>
				</a>
			</div>
		{/if}
	</section>
{/if}
