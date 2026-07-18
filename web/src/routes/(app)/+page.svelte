<script lang="ts">
	import * as m from "$lib/paraglide/messages";
	import { api, authState } from "$lib/api/client.svelte";
	import { goto } from "$app/navigation";
	import { resolve } from "$app/paths";
	import type { ListBooksParams } from "$lib/api/client.svelte";

	type LibBook = {
		id: string;
		title: string;
		author: string | null;
		format: string;
		read_status: string;
		rating: number | null;
		isbn: string | null;
		language: string | null;
		publisher: string | null;
		series: string | null;
		series_index: number | null;
		page_count: number | null;
		author_id: string | null;
		created_at: string;
		updated_at: string;
	};
	import Modal from "$lib/components/Modal.svelte";
	import UploadModal from "$lib/components/UploadModal.svelte";
	import BookCover from "$lib/components/BookCover.svelte";
	import BookOpen from "@lucide/svelte/icons/book-open";
	import LibraryBig from "@lucide/svelte/icons/library-big";
	import PlusCircle from "@lucide/svelte/icons/plus-circle";
	import Upload from "@lucide/svelte/icons/upload";
	import Bookmark from "@lucide/svelte/icons/bookmark";
	import Plus from "@lucide/svelte/icons/plus";
	import Trash2 from "@lucide/svelte/icons/trash-2";
	import ChevronRight from "@lucide/svelte/icons/chevron-right";
	import Info from "@lucide/svelte/icons/info";
	import ArrowUpDown from "@lucide/svelte/icons/arrow-up-down";
	import ChevronDown from "@lucide/svelte/icons/chevron-down";

	type ShelfInfo = {
		id: string;
		library_id: string;
		name: string;
		description: string | null;
		kind: string;
		book_count: number;
		created_at: string;
	};

	let books: LibBook[] = $state([]);
	let shelves = $state<ShelfInfo[]>([]);
	let progressMap = $state<Record<string, number>>({});
	let loading = $state(true);
	let showUpload = $state(false);
	let showCreateShelf = $state(false);
	let shelfName = $state("");
	let shelfDesc = $state("");
	let shelfKind = $state<"static" | "dynamic">("static");
	let deletingShelf = $state<string | null>(null);

	let sortBy = $state("updated_at");
	let sortOrder = $state<"desc" | "asc">("desc");
	let filterStatus = $state("");
	let offset = $state(0);
	let total = $state(0);
	let loadingMore = $state(false);
	let showSortMenu = $state(false);

	const PAGE = 24;

	let initialLoadStarted = false;
	let booksAbort: AbortController | null = null;
	let shelvesAbort: AbortController | null = null;

	$effect(() => {
		if (authState.restoring || initialLoadStarted) return;
		if (!authState.isAuthenticated) return;
		initialLoadStarted = true;
		loadAll();
	});

	$effect(() => {
		return () => {
			booksAbort?.abort();
			shelvesAbort?.abort();
		};
	});

	async function loadAll() {
		loading = true;
		offset = 0;
		await Promise.all([loadBooks(true), loadShelves()]);
		loading = false;
	}

	async function loadShelves() {
		shelvesAbort?.abort();
		const controller = new AbortController();
		shelvesAbort = controller;
		const r = await api.shelves.list({ signal: controller.signal });
		if (controller.signal.aborted) return;
		if (r.isOk()) shelves = r.value as unknown as ShelfInfo[];
	}

	async function loadBooks(reset = false) {
		booksAbort?.abort();
		const controller = new AbortController();
		booksAbort = controller;

		const params: ListBooksParams = { limit: PAGE, offset, sortBy, sortOrder };
		if (filterStatus) params.readStatus = filterStatus;
		const r = await api.books.list(params, { signal: controller.signal });
		if (controller.signal.aborted) return;
		if (r.isOk()) {
			const data = r.value as { books: LibBook[]; total: number };
			total = data.total;
			if (reset) books = data.books;
			else books = [...books, ...data.books];
		}
	}

	async function handleSort(field: string) {
		if (sortBy === field) {
			sortOrder = sortOrder === "desc" ? "asc" : "desc";
		} else {
			sortBy = field;
			sortOrder = "desc";
		}
		showSortMenu = false;
		offset = 0;
		await loadBooks(true);
	}

	async function handleFilter(status: string) {
		filterStatus = filterStatus === status ? "" : status;
		offset = 0;
		await loadBooks(true);
	}

	async function loadMore() {
		loadingMore = true;
		offset = books.length;
		await loadBooks(false);
		loadingMore = false;
	}

	const currentlyReading = $derived(books.filter((b: LibBook) => b.read_status === "reading"));
	const forYou = $derived(
		books.filter((b: LibBook) => {
			if (filterStatus) return b.read_status === filterStatus;
			return b.read_status !== "reading";
		})
	);

	function formatBadge(fmt: string): string {
		const m2: Record<string, string> = {
			mobi_raw: m.book_format_mobi(),
			cbz: m.book_format_cbz(),
			pdf: m.book_format_pdf(),
			epub: m.book_format_epub(),
			native: m.book_format_native(),
			bvir: m.book_format_bvir()
		};
		return m2[fmt] ?? fmt.toUpperCase();
	}

	async function handleCreateShelf() {
		if (!shelfName.trim()) return;
		const r = await api.shelves.create({
			name: shelfName.trim(),
			description: shelfDesc.trim() || undefined,
			kind: shelfKind
		});
		if (r.isOk()) {
			shelves = [r.value as unknown as ShelfInfo, ...shelves];
			shelfName = "";
			shelfDesc = "";
			showCreateShelf = false;
		}
	}

	async function handleDeleteShelf(id: string) {
		deletingShelf = id;
		if ((await api.shelves.delete(id)).isOk()) shelves = shelves.filter((s) => s.id !== id);
		deletingShelf = null;
	}

	const sortLabels: Record<string, string> = {
		title: m.library_sort_title(),
		author: m.library_sort_author(),
		created_at: m.library_sort_date(),
		updated_at: m.library_sort_updated()
	};

	const filterOptions = [
		{ key: "reading", label: m.library_filter_reading() },
		{ key: "finished", label: m.library_filter_finished() },
		{ key: "unread", label: m.library_filter_unread() },
		{ key: "pending", label: m.library_filter_pending() }
	];
</script>

<UploadModal show={showUpload} onComplete={loadAll} />

<section class="mb-section-gap">
	<header class="mb-8 flex flex-col justify-between gap-4 sm:flex-row sm:items-end">
		<div>
			<span class="font-label text-label-sm text-secondary mb-2 block tracking-widest uppercase"
				>{m.library_reading_subtitle()}</span
			>
			<h2 class="font-display text-headline-md">{m.library_reading_title()}</h2>
		</div>
		<button onclick={() => (showUpload = true)} class="btn-primary"
			><Upload size={18} />{m.upload_title()}</button
		>
	</header>

	{#if loading}
		<div class="flex items-center justify-center py-32">
			<div
				class="border-secondary h-8 w-8 animate-spin rounded-full border-2 border-t-transparent"
			></div>
		</div>
	{:else}
		{#if currentlyReading.length > 0}
			<div class="mb-section-gap">
				<header class="mb-6 flex items-end justify-between">
					<h3 class="font-display text-headline-sm">{m.library_reading_title()}</h3>
					<a
						href={resolve("/search")}
						class="font-label text-label-md text-secondary hover:border-secondary border-b border-transparent transition-all"
						>{m.library_reading_view_all()}</a
					>
				</header>
				<div class="gap-gutter grid grid-cols-1 md:grid-cols-2">
					{#each currentlyReading as book (book.id)}
						<a
							href={resolve(`/reader/${book.id}`)}
							class="paper-card flex h-full flex-col overflow-hidden rounded-xl transition-all hover:shadow-lg sm:flex-row"
						>
							<BookCover
								bookId={book.id}
								class="aspect-[2/3] w-full sm:aspect-auto sm:w-1/3"
								coverClass="rounded-none"
							/>
							<div class="flex flex-1 flex-col justify-between p-6">
								<div>
									<div class="text-on-surface-variant mb-2 flex items-center gap-2">
										<span
											class="font-label text-label-sm bg-surface-container-high rounded px-2 py-0.5 tracking-wider uppercase"
											>{formatBadge(book.format)}</span
										>
										<button
											onclick={(e) => {
												e.preventDefault();
												goto(resolve(`/book/${book.id}`));
											}}
											class="text-on-surface-variant/40 hover:text-secondary ml-auto cursor-pointer transition-colors"
											title="Details"><Info size={14} /></button
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
										<div
											class="reading-progress-fill"
											style="width: {progressMap[book.id] ?? 0}%;"
										></div>
									</div>
								</div>
							</div>
						</a>
					{/each}
				</div>
			</div>
		{/if}

		<section class="mb-section-gap">
			<header class="mb-6 flex items-center justify-between">
				<h3 class="font-display text-headline-sm">{m.shelf_title()}</h3>
				<button
					onclick={() => (showCreateShelf = true)}
					class="font-label text-label-sm text-secondary hover:text-secondary/80 flex items-center gap-1 transition-colors"
					><Plus size={16} />{m.shelf_create()}</button
				>
			</header>
			<Modal bind:show={showCreateShelf} title={m.shelf_create()} maxWidth="md">
				<div class="space-y-5">
					<div>
						<label
							for="shelf-name"
							class="font-label text-label-sm text-on-surface-variant mb-1.5 block tracking-widest uppercase"
							>{m.shelf_name()}</label
						><input
							id="shelf-name"
							type="text"
							bind:value={shelfName}
							class="input-minimal"
							placeholder="e.g. Sci-Fi Favorites"
						/>
					</div>
					<div>
						<label
							for="shelf-desc"
							class="font-label text-label-sm text-on-surface-variant mb-1.5 block tracking-widest uppercase"
							>{m.shelf_description()}</label
						><input
							id="shelf-desc"
							type="text"
							bind:value={shelfDesc}
							class="input-minimal"
							placeholder="Books I want to read this year"
						/>
					</div>
					<div class="border-outline/10 flex gap-1 rounded-xl border p-1">
						<button
							onclick={() => (shelfKind = "static")}
							class={[
								"font-label flex-1 rounded-lg px-4 py-2 text-sm transition-all",
								shelfKind === "static"
									? "bg-primary text-white shadow-sm"
									: "text-on-surface-variant hover:text-primary"
							]}>{m.shelf_kind_static()}</button
						>
						<button
							onclick={() => (shelfKind = "dynamic")}
							class={[
								"font-label flex-1 rounded-lg px-4 py-2 text-sm transition-all",
								shelfKind === "dynamic"
									? "bg-primary text-white shadow-sm"
									: "text-on-surface-variant hover:text-primary"
							]}>{m.shelf_kind_dynamic()}</button
						>
					</div>
					<div class="flex justify-end gap-3 pt-2">
						<button
							onclick={() => (showCreateShelf = false)}
							class="font-label text-label-md text-on-surface-variant px-4 py-2">Cancel</button
						>
						<button onclick={handleCreateShelf} class="btn-primary" disabled={!shelfName.trim()}
							>{m.shelf_create()}</button
						>
					</div>
				</div>
			</Modal>
			{#if shelves.length === 0}
				<div class="border-outline-variant/30 rounded-xl border-2 border-dashed p-10 text-center">
					<Bookmark size={28} class="text-on-surface-variant/30 mb-3 block" />
					<p class="font-body text-body-md text-on-surface-variant">{m.shelf_create()}</p>
				</div>
			{:else}
				<div class="gap-gutter grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3">
					{#each shelves as shelf (shelf.id)}
						<a
							href={resolve(`/shelves/${shelf.id}`)}
							class="paper-card group block rounded-xl p-6 transition-all hover:shadow-lg"
						>
							<div class="mb-4 flex items-start justify-between">
								<div
									class="bg-surface-container flex h-10 w-10 items-center justify-center rounded-lg"
								>
									<Bookmark size={18} class="text-secondary" />
								</div>
								<button
									onclick={(e) => {
										e.preventDefault();
										handleDeleteShelf(shelf.id);
									}}
									disabled={deletingShelf === shelf.id}
									class="text-on-surface-variant/30 hover:text-error p-1 opacity-0 transition-all group-hover:opacity-100 disabled:opacity-30"
									><Trash2 size={14} /></button
								>
							</div>
							<h4 class="font-display text-headline-sm mb-1">{shelf.name}</h4>
							{#if shelf.description}<p
									class="font-body text-body-md text-on-surface-variant mb-4 line-clamp-2"
								>
									{shelf.description}
								</p>{/if}
							<div class="mt-4 flex items-center justify-between">
								<span class="font-label text-label-sm text-on-surface-variant"
									>{shelf.book_count}
									{shelf.book_count === 1 ? "book" : "books"}<span
										class="bg-surface-container-high ml-2 rounded px-1.5 py-0.5 text-[10px] uppercase"
										>{shelf.kind}</span
									></span
								>
								<ChevronRight size={16} class="text-on-surface-variant/30" />
							</div>
						</a>
					{/each}
				</div>
			{/if}
		</section>

		<section>
			<header class="mb-6 flex flex-wrap items-center justify-between gap-3">
				<div class="flex items-center gap-2">
					<h3 class="font-display text-headline-sm">{m.library_for_you_title()}</h3>
					<span class="font-label text-label-sm text-on-surface-variant/50">({total})</span>
				</div>
				<div class="flex flex-wrap items-center gap-2">
					<button
						onclick={() => handleFilter("")}
						class={[
							"font-label text-label-sm rounded-lg border px-3 py-1.5 transition-all",
							!filterStatus
								? "bg-secondary border-secondary text-white"
								: "border-outline/10 text-on-surface-variant hover:text-primary"
						]}>All</button
					>
					{#each filterOptions as opt (opt.key)}
						<button
							onclick={() => handleFilter(opt.key)}
							class={[
								"font-label text-label-sm rounded-lg border px-3 py-1.5 transition-all",
								filterStatus === opt.key
									? "bg-secondary border-secondary text-white"
									: "border-outline/10 text-on-surface-variant hover:text-primary"
							]}>{opt.label}</button
						>
					{/each}
					<div class="relative">
						<button
							onclick={() => (showSortMenu = !showSortMenu)}
							class="font-label text-label-sm text-on-surface-variant hover:text-primary border-outline/10 flex items-center gap-1 rounded-lg border px-3 py-1.5 transition-all"
							><ArrowUpDown size={14} />{sortLabels[sortBy] ?? sortBy}<ChevronDown
								size={12}
							/></button
						>
						{#if showSortMenu}
							<div
								class="bg-surface border-outline/10 absolute top-9 right-0 z-50 min-w-[160px] rounded-xl border p-1.5 shadow-lg"
							>
								{#each ["title", "author", "created_at", "updated_at"] as field (field)}
									<button
										onclick={() => handleSort(field)}
										class={[
											"font-label text-label-md w-full rounded-lg px-4 py-2 text-left transition-colors",
											sortBy === field
												? "bg-secondary/5 text-secondary"
												: "text-on-surface-variant hover:text-primary hover:bg-surface-container-low"
										]}
										>{sortLabels[field]}
										{sortBy === field ? (sortOrder === "asc" ? "↑" : "↓") : ""}</button
									>
								{/each}
							</div>
						{/if}
					</div>
				</div>
			</header>

			{#if forYou.length === 0 && !loadingMore}
				<div class="border-outline-variant/30 rounded-xl border-2 border-dashed p-12 text-center">
					<LibraryBig size={32} class="text-on-surface-variant/30 mb-4 block" />
					<p class="font-body text-body-md text-on-surface-variant">
						{filterStatus ? "No books match this filter" : m.library_for_you_empty()}
					</p>
				</div>
			{:else}
				<div class="gap-gutter grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4">
					{#each forYou as book (book.id)}
						<a href={resolve(`/reader/${book.id}`)} class="group cursor-pointer">
							<BookCover
								bookId={book.id}
								class="paper-card bg-surface-container mb-4 aspect-[2/3] rounded-xl border-none"
							/>
							{#if book.format === "mobi_raw" || book.format === "cbz"}
								<div class="absolute top-2 right-2">
									<span
										class="font-label text-primary rounded bg-white/90 px-2 py-0.5 text-[10px] tracking-wider uppercase backdrop-blur"
										>{formatBadge(book.format)}</span
									>
								</div>
							{/if}
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
					<button
						onclick={() => (showUpload = true)}
						class="border-outline-variant/30 hover:bg-surface-container/30 group flex aspect-[2/3] cursor-pointer flex-col items-center justify-center rounded-xl border-2 border-dashed transition-colors"
					>
						<PlusCircle
							size={28}
							class="text-on-surface-variant/40 transition-transform group-hover:scale-110"
						/>
						<span class="font-label text-label-sm text-on-surface-variant mt-2"
							>{m.library_add_book()}</span
						>
					</button>
				</div>
				{#if books.length < total}
					<div class="mt-8 text-center">
						<button
							onclick={loadMore}
							disabled={loadingMore}
							class="font-label text-label-md text-secondary hover:text-secondary/80 border-secondary/20 inline-flex items-center gap-2 rounded-xl border px-6 py-3 transition-colors disabled:opacity-50"
						>
							{loadingMore ? m.library_loading_more() : m.library_load_more()}
						</button>
					</div>
				{/if}
			{/if}
		</section>
	{/if}
</section>
