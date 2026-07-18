<script lang="ts">
	import * as m from "$lib/paraglide/messages";
	import { api, authState } from "$lib/api/client.svelte";
	import { goto } from "$app/navigation";
	import { resolve } from "$app/paths";
	import { page } from "$app/state";
	import type { ShelfResponse, BookResponse } from "$lib/api/generated";
	import BookCover from "$lib/components/BookCover.svelte";
	import ArrowLeft from "@lucide/svelte/icons/arrow-left";
	import Bookmark from "@lucide/svelte/icons/bookmark";
	import Trash2 from "@lucide/svelte/icons/trash-2";
	import Plus from "@lucide/svelte/icons/plus";
	import Search from "@lucide/svelte/icons/search";
	import X from "@lucide/svelte/icons/x";
	import Check from "@lucide/svelte/icons/check";
	import Info from "@lucide/svelte/icons/info";

	type ShelfBook = {
		book_id: string;
		title: string;
		author: string | null;
		read_status: string;
	};

	let shelf = $state<ShelfResponse | null>(null);
	let books = $state<ShelfBook[]>([]);
	let loading = $state(true);
	let error = $state("");
	let success = $state("");
	let removing = $state<string | null>(null);

	let showAddModal = $state(false);
	let searchQuery = $state("");
	let searchResults = $state<BookResponse[]>([]);
	let searchingBooks = $state(false);
	let adding = $state<string | null>(null);

	let deleting = $state(false);

	$effect(() => {
		if (!authState.isAuthenticated) {
			goto(resolve("/login"));
			return;
		}
		loadShelf();
	});

	async function loadShelf() {
		loading = true;
		error = "";
		const shelfId = page.params.id;
		if (!shelfId) {
			error = "Invalid shelf ID";
			loading = false;
			return;
		}

		const [shelfResult, booksResult] = await Promise.all([
			api.shelves.get(shelfId),
			api.shelves.getBooks(shelfId)
		]);

		if (shelfResult.isErr()) {
			error = shelfResult.error.message;
			loading = false;
			return;
		}
		if (booksResult.isErr()) {
			error = booksResult.error.message;
			loading = false;
			return;
		}

		shelf = shelfResult.value;
		books = booksResult.value as unknown as ShelfBook[];
		loading = false;
	}

	async function handleRemoveBook(bookId: string) {
		if (!shelf) return;
		removing = bookId;
		const result = await api.shelves.removeBook(shelf.id, bookId);
		if (result.isOk()) {
			books = books.filter((b) => b.book_id !== bookId);
			success = m.shelf_detail_book_removed();
		} else {
			error = result.error.message;
		}
		removing = null;
	}

	async function handleDeleteShelf() {
		if (!shelf) return;
		deleting = true;
		const result = await api.shelves.delete(shelf.id);
		if (result.isOk()) {
			goto(resolve("/"));
		} else {
			error = result.error.message;
			deleting = false;
		}
	}

	async function handleSearchBooks() {
		if (!searchQuery.trim()) return;
		searchingBooks = true;
		const result = await api.search(searchQuery);
		if (result.isOk()) {
			const hits = result.value.books;
			const metaResults = await Promise.all(hits.map((h) => api.books.get(h.id)));
			const allBooks = metaResults
				.filter((r) => r.isOk())
				.map((r) => r.value)
				.filter((b) => !books.some((sb) => sb.book_id === b.id));
			searchResults = allBooks;
		}
		searchingBooks = false;
	}

	async function handleAddBook(bookId: string) {
		if (!shelf) return;
		adding = bookId;
		const result = await api.shelves.addBook(shelf.id, bookId);
		if (result.isOk()) {
			const bookResult = await api.books.get(bookId);
			if (bookResult.isOk()) {
				const b = bookResult.value;
				books = [
					...books,
					{ book_id: b.id, title: b.title, author: b.author, read_status: b.read_status }
				];
			}
			success = m.shelf_detail_book_added();
			searchResults = searchResults.filter((r) => r.id !== bookId);
		} else {
			error = result.error.message;
		}
		adding = null;
	}

	function formatDate(d: string | undefined): string {
		return d ? d.split("T")[0] : "—";
	}

	function readStatusLabel(status: string): string {
		const map: Record<string, string> = {
			unread: m.book_detail_read_status_unread(),
			reading: m.book_detail_read_status_reading(),
			finished: m.book_detail_read_status_finished(),
			pending: m.book_detail_read_status_pending()
		};
		return map[status] ?? status;
	}
</script>

<svelte:head>
	<title>{shelf?.name ?? "Shelf"} — Book Vault</title>
</svelte:head>

<section>
	<div class="mb-6">
		<a
			href={resolve("/")}
			class="font-label text-label-md text-on-surface-variant hover:text-primary inline-flex items-center gap-1.5 transition-colors"
		>
			<ArrowLeft size={16} />
			{m.shelf_detail_back()}
		</a>
	</div>

	{#if loading}
		<div class="flex items-center justify-center py-32">
			<div
				class="border-secondary h-8 w-8 animate-spin rounded-full border-2 border-t-transparent"
			></div>
		</div>
	{:else if !shelf}
		<div class="paper-card rounded-xl p-16 text-center">
			<p class="font-body text-body-md text-on-surface-variant">Shelf not found</p>
		</div>
	{:else}
		{#if error}
			<div
				class="font-label text-label-sm text-error bg-error-container/20 mb-6 rounded-lg px-4 py-3"
			>
				{error}
			</div>
		{/if}
		{#if success}
			<div
				class="font-label text-label-sm bg-secondary/10 text-secondary mb-6 rounded-lg px-4 py-3"
			>
				{success}
			</div>
		{/if}

		<div class="mb-10 flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
			<div class="flex items-start gap-5">
				<div class="bg-surface-container flex h-14 w-14 items-center justify-center rounded-xl">
					<Bookmark size={24} class="text-secondary" />
				</div>
				<div>
					<h1 class="font-display text-headline-md text-primary mb-2">{shelf.name}</h1>
					{#if shelf.description}
						<p class="font-body text-body-md text-on-surface-variant mb-2">{shelf.description}</p>
					{/if}
					<div class="flex items-center gap-3">
						<span class="font-label text-label-sm text-on-surface-variant"
							>{shelf.book_count} books</span
						>
						<span
							class="bg-surface-container-high font-label text-label-sm rounded px-2 py-0.5 uppercase"
							>{shelf.kind}</span
						>
						<span class="font-label text-label-sm text-on-surface-variant"
							>Created {formatDate(shelf.created_at)}</span
						>
					</div>
				</div>
			</div>
			<div class="flex gap-2">
				{#if shelf.kind === "static"}
					<button
						onclick={() => (showAddModal = true)}
						class="font-label text-label-md text-secondary hover:text-secondary/80 inline-flex items-center gap-1.5 rounded-lg border border-[rgba(175,43,62,0.2)] px-4 py-2 transition-colors"
					>
						<Plus size={16} />{m.shelf_detail_add_book()}
					</button>
				{/if}
				<button
					onclick={handleDeleteShelf}
					disabled={deleting}
					class="font-label text-label-md text-error hover:text-error/80 inline-flex items-center gap-1.5 rounded-lg px-4 py-2 transition-colors disabled:opacity-50"
				>
					<Trash2 size={16} />{deleting ? "..." : m.shelf_title()}
				</button>
			</div>
		</div>

		{#if books.length === 0}
			<div class="paper-card rounded-xl p-16 text-center">
				<Bookmark size={36} class="text-on-surface-variant/20 mb-4 block" />
				<p class="font-body text-body-md text-on-surface-variant mb-6">{m.shelf_detail_empty()}</p>
				{#if shelf.kind === "static"}
					<button
						onclick={() => (showAddModal = true)}
						class="font-label text-label-md text-secondary hover:text-secondary/80 inline-flex items-center gap-1.5 transition-colors"
					>
						<Plus size={16} />{m.shelf_detail_add_book()}
					</button>
				{/if}
			</div>
		{:else}
			<div class="space-y-3">
				{#each books as book (book.book_id)}
					<div
						class="paper-card flex items-center gap-5 rounded-xl p-5 transition-all hover:shadow-md"
					>
						<BookCover bookId={book.book_id} class="h-14 w-10 shrink-0 rounded-lg" />
						<div class="min-w-0 flex-1">
							<a
								href={resolve(`/reader/${book.book_id}`)}
								class="font-display text-headline-sm text-primary hover:text-secondary mb-0.5 block transition-colors"
							>
								{book.title}
							</a>
							<p class="font-label text-label-sm text-on-surface-variant">
								{book.author ?? "—"} · {readStatusLabel(book.read_status)}
							</p>
						</div>
						<div class="flex items-center gap-2">
							<a
								href={resolve(`/book/${book.book_id}`)}
								class="font-label text-label-sm text-on-surface-variant/50 hover:text-primary px-2 py-1 transition-colors"
								title="Details"
							>
								<Info size={14} />
							</a>
							{#if shelf.kind === "static"}
								<button
									onclick={() => handleRemoveBook(book.book_id)}
									disabled={removing === book.book_id}
									class="text-on-surface-variant/30 hover:text-error p-1 transition-colors disabled:opacity-30"
									title={m.shelf_detail_remove_book()}
								>
									<X size={14} />
								</button>
							{/if}
						</div>
					</div>
				{/each}
			</div>
		{/if}

		{#if showAddModal}
			<div
				class="bg-primary/40 fixed inset-0 z-50 flex items-center justify-center p-4 backdrop-blur-sm"
				role="dialog"
				aria-modal="true"
				tabindex="-1"
				onclick={() => (showAddModal = false)}
				onkeydown={(e) => {
					if (e.key === "Escape") showAddModal = false;
				}}
			>
				<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
				<div
					class="bg-surface mx-auto w-full max-w-lg rounded-2xl p-8 shadow-2xl"
					role="document"
					tabindex="-1"
					onclick={(e) => e.stopPropagation()}
					onkeydown={(e) => {
						if (e.key === "Escape") showAddModal = false;
					}}
				>
					<div class="mb-6 flex items-center justify-between">
						<h3 class="font-display text-headline-sm text-primary">{m.shelf_detail_add_book()}</h3>
						<button
							onclick={() => (showAddModal = false)}
							class="text-on-surface-variant/50 hover:text-on-surface-variant p-1"
						>
							<X size={20} />
						</button>
					</div>
					<form onsubmit={handleSearchBooks} class="mb-6">
						<div class="relative">
							<Search
								size={18}
								class="text-on-surface-variant absolute top-1/2 left-3 -translate-y-1/2"
							/>
							<input
								type="text"
								bind:value={searchQuery}
								placeholder={m.shelf_detail_search_placeholder()}
								class="bg-surface-container-low border-outline/10 font-body text-body-md text-on-surface placeholder:text-on-surface-variant/50 focus:ring-primary/10 w-full rounded-xl border py-3 pr-4 pl-10 focus:ring-2 focus:outline-none"
							/>
						</div>
					</form>
					{#if searchingBooks}
						<div class="flex items-center justify-center py-8">
							<div
								class="border-secondary h-6 w-6 animate-spin rounded-full border-2 border-t-transparent"
							></div>
						</div>
					{:else if searchResults.length > 0}
						<div class="max-h-80 space-y-2 overflow-y-auto">
							{#each searchResults as result (result.id)}
								<div
									class="bg-surface-container-low flex items-center justify-between rounded-xl p-3"
								>
									<div class="min-w-0 flex-1">
										<p class="font-display text-headline-sm truncate">{result.title}</p>
										<p class="font-label text-label-sm text-on-surface-variant">
											{result.author ?? "—"}
										</p>
									</div>
									<button
										onclick={() => handleAddBook(result.id)}
										disabled={adding === result.id}
										class="font-label text-label-sm text-secondary hover:text-secondary/80 ml-3 shrink-0 transition-colors disabled:opacity-50"
									>
										{#if adding === result.id}...{:else}<Check size={16} />{/if}
									</button>
								</div>
							{/each}
						</div>
					{:else if searchQuery}
						<p class="font-body text-body-md text-on-surface-variant py-4 text-center">
							{m.shelf_detail_no_results()}
						</p>
					{/if}
				</div>
			</div>
		{/if}
	{/if}
</section>
