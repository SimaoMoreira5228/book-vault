<script lang="ts">
	import * as m from "$lib/paraglide/messages";
	import { api, authState } from "$lib/api/client";
	import { goto } from "$app/navigation";
	import { page } from "$app/state";
	import BookCover from "$lib/components/BookCover.svelte";
	import ArrowLeft from "@lucide/svelte/icons/arrow-left";
	import Bookmark from "@lucide/svelte/icons/bookmark";

	type SeriesDetail = { id: string; name: string; description: string | null; book_count: number };
	type Book = {
		id: string;
		title: string;
		author: string | null;
		read_status: string;
		series_id: string | null;
	};

	let series = $state<SeriesDetail | null>(null);
	let books = $state<Book[]>([]);
	let loading = $state(true);
	let error = $state("");

	$effect(() => {
		if (!authState.isAuthenticated) {
			goto("/login");
			return;
		}
		if (page.params.id) loadSeries();
	});

	async function loadSeries() {
		loading = true;
		error = "";
		const id = page.params.id!;
		const [sr, br] = await Promise.all([api.series.get(id), api.books.list()]);
		if (sr.isErr()) {
			error = sr.error.message;
			loading = false;
			return;
		}
		series = sr.value as unknown as SeriesDetail;
		if (br.isOk())
			books = (br.value as unknown as { books: Book[] }).books.filter(
				(b: Book) => b.series_id === id
			);
		loading = false;
	}
</script>

<svelte:head><title>{series?.name ?? "Series"} — Book Vault</title></svelte:head>

<section>
	<div class="mb-6">
		<a
			href="/series"
			class="font-label text-label-md text-on-surface-variant hover:text-primary inline-flex items-center gap-1.5 transition-colors"
			><ArrowLeft size={16} />{m.series_title()}</a
		>
	</div>

	{#if loading}
		<div class="flex items-center justify-center py-32">
			<div
				class="border-secondary h-8 w-8 animate-spin rounded-full border-2 border-t-transparent"
			></div>
		</div>
	{:else if !series}
		<div class="paper-card rounded-xl p-16 text-center">
			<p class="font-body text-body-md text-on-surface-variant">{error || "Series not found"}</p>
		</div>
	{:else}
		<div class="mb-10 flex items-start gap-6">
			<div class="bg-surface-container flex h-20 w-20 items-center justify-center rounded-2xl">
				<Bookmark size={36} class="text-secondary" />
			</div>
			<div class="pt-2">
				<h1 class="font-display text-headline-md text-primary mb-2">{series.name}</h1>
				<p class="font-label text-label-sm text-on-surface-variant">
					{m.series_book_count({ count: series.book_count })}
				</p>
				{#if series.description}<p
						class="font-body text-body-md text-on-surface-variant mt-4 max-w-2xl"
					>
						{series.description}
					</p>{/if}
			</div>
		</div>

		<h3 class="font-display text-headline-sm text-primary mb-6">{m.series_books()}</h3>

		{#if books.length === 0}
			<div class="paper-card rounded-xl p-12 text-center">
				<p class="font-body text-body-md text-on-surface-variant">{m.shelf_detail_empty()}</p>
			</div>
		{:else}
			<div class="space-y-3">
				{#each books as book (book.id)}
					<a
						href="/reader/{book.id}"
						class="paper-card flex items-center gap-5 rounded-xl p-5 transition-all hover:shadow-md"
					>
						<BookCover bookId={book.id} class="h-14 w-10 shrink-0 rounded-lg" />
						<div class="min-w-0 flex-1">
							<p class="font-display text-headline-sm truncate">{book.title}</p>
							<p class="font-label text-label-sm text-on-surface-variant">
								{book.author ?? "—"} · {book.read_status}
							</p>
						</div>
					</a>
				{/each}
			</div>
		{/if}
	{/if}
</section>
