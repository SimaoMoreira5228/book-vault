<script lang="ts">
	import * as m from "$lib/paraglide/messages";
	import { api, authState } from "$lib/api/client.svelte";
	import { goto } from "$app/navigation";
	import { resolve } from "$app/paths";
	import { page } from "$app/state";
	import type { BookResponse, ProspectiveMetadata } from "$lib/api/generated";
	import FieldDisplay from "$lib/components/FieldDisplay.svelte";
	import FieldEditor from "$lib/components/FieldEditor.svelte";
	import FieldNumber from "$lib/components/FieldNumber.svelte";
	import BookCover from "$lib/components/BookCover.svelte";
	import FileDown from "@lucide/svelte/icons/file-down";
	import ArrowLeft from "@lucide/svelte/icons/arrow-left";
	import BookOpen from "@lucide/svelte/icons/book-open";
	import Pencil from "@lucide/svelte/icons/pencil";
	import Trash2 from "@lucide/svelte/icons/trash-2";
	import Search from "@lucide/svelte/icons/search";
	import RefreshCw from "@lucide/svelte/icons/refresh-cw";
	import Lock from "@lucide/svelte/icons/lock";
	import Unlock from "@lucide/svelte/icons/unlock";
	import X from "@lucide/svelte/icons/x";

	let book = $state<BookResponse | null>(null);
	let meta = $state<Record<string, unknown> | null>(null);
	let loading = $state(true);
	let error = $state("");
	let success = $state("");

	let editing = $state(false);
	let editTitle = $state("");
	let editAuthor = $state("");
	let editIsbn = $state("");
	let editPublisher = $state("");
	let editLanguage = $state("");
	let editSeries = $state("");
	let editSeriesIndex = $state<number | null>(null);
	let editPageCount = $state<number | null>(null);
	let editRating = $state<number | null>(null);
	let editReadStatus = $state("");
	let saving = $state(false);

	let candidates = $state<ProspectiveMetadata[]>([]);
	let searching = $state(false);
	let confirming = $state(false);
	let selectedCandidate = $state<number | null>(null);
	let searchingMeta = $state(false);

	let deleting = $state(false);
	let showDeleteConfirm = $state(false);

	$effect(() => {
		if (!authState.isAuthenticated) {
			goto(resolve("/login"));
			return;
		}
		loadBook();
	});

	async function loadBook() {
		loading = true;
		error = "";
		success = "";
		const bookId = page.params.id;
		if (!bookId) {
			error = "Invalid book ID";
			loading = false;
			return;
		}

		const [bookResult, metaResult] = await Promise.all([
			api.books.get(bookId),
			api.metadata.get(bookId)
		]);

		if (bookResult.isErr()) {
			error = bookResult.error.message;
			loading = false;
			return;
		}
		book = bookResult.value;
		if (metaResult.isOk()) meta = metaResult.value;
		resetEdit();
		loading = false;
	}

	function resetEdit() {
		if (!book) return;
		editTitle = book.title;
		editAuthor = book.author ?? "";
		editIsbn = book.isbn ?? "";
		editPublisher = book.publisher ?? "";
		editLanguage = book.language ?? "";
		editSeries = book.series ?? "";
		editSeriesIndex = book.series_index ?? null;
		editPageCount = book.page_count ?? null;
		editRating = book.rating ?? null;
		editReadStatus = book.read_status;
	}

	async function handleSave() {
		if (!book) return;
		saving = true;
		error = "";
		success = "";
		const result = await api.books.update(book.id, {
			title: editTitle,
			author: editAuthor || null,
			isbn: editIsbn || null,
			publisher: editPublisher || null,
			language: editLanguage || null,
			series: editSeries || null,
			series_index: editSeriesIndex ?? undefined,
			page_count: editPageCount ?? undefined,
			rating: editRating ?? undefined,
			read_status: editReadStatus
		} as Parameters<typeof api.books.update>[1]);
		if (result.isOk()) {
			book = result.value;
			editing = false;
			success = "Saved";
		} else {
			error = result.error.message;
		}
		saving = false;
	}

	async function handleDelete() {
		if (!book) return;
		deleting = true;
		const result = await api.books.delete(book.id);
		if (result.isOk()) {
			goto(resolve("/"));
		} else {
			error = result.error.message;
			deleting = false;
			showDeleteConfirm = false;
		}
	}

	async function handleSearchCandidates() {
		if (!book) return;
		searching = true;
		error = "";
		const result = await api.metadata.candidates(book.id, {
			title: book.title,
			author: book.author ?? undefined
		});
		if (result.isOk()) {
			candidates = result.value;
		} else {
			error = result.error.message;
		}
		searching = false;
	}

	async function handleConfirmCandidate(index: number) {
		if (!book) return;
		confirming = true;
		selectedCandidate = index;
		error = "";
		success = "";
		const result = await api.metadata.confirm(book.id, candidates[index]);
		if (result.isOk()) {
			meta = result.value;
			success = m.metadata_confirm_success();
			candidates = [];
			selectedCandidate = null;
			loadBook();
		} else {
			error = result.error.message;
			selectedCandidate = null;
		}
		confirming = false;
	}

	async function handleRefresh() {
		if (!book) return;
		searchingMeta = true;
		error = "";
		success = "";
		const result = await api.metadata.refresh(book.id);
		if (result.isOk()) {
			meta = result.value;
			success = "Metadata refreshed";
		} else {
			error = result.error.message;
		}
		searchingMeta = false;
	}

	async function handleLockField(field: string) {
		if (!book) return;
		const result = await api.metadata.lockField(book.id, field);
		if (result.isOk()) meta = result.value;
	}

	async function handleUnlockField(field: string) {
		if (!book) return;
		const result = await api.metadata.unlockField(book.id, field);
		if (result.isOk()) meta = result.value;
	}

	function formatBadge(fmt: string): string {
		const map: Record<string, string> = {
			epub: m.book_format_epub(),
			pdf: m.book_format_pdf(),
			cbz: m.book_format_cbz(),
			mobi_raw: m.book_format_mobi(),
			native: m.book_format_native(),
			bvir: m.book_format_bvir()
		};
		return map[fmt] ?? fmt.toUpperCase();
	}

	function formatDate(d: string | undefined): string {
		return d ? d.split("T")[0] : "—";
	}

	const lockedFields: string[] = $derived((meta?.locked_fields as string[]) ?? []);
	const providerIds = $derived((meta?.provider_ids as Record<string, string>) ?? {});

	const fieldLabels: Record<string, string> = {
		Title: m.book_detail_field_title(),
		Author: m.book_detail_field_author(),
		Description: "Description",
		Cover: "Cover",
		Genres: "Genres",
		PageCount: m.book_detail_field_page_count(),
		Isbn: m.book_detail_field_isbn(),
		Publisher: m.book_detail_field_publisher(),
		PublishedDate: "Published Date",
		Subtitle: "Subtitle"
	};

	const metadataFields = Object.keys(fieldLabels);
</script>

<svelte:head>
	<title>{book?.title ?? "Book"} — Book Vault</title>
</svelte:head>

<section>
	<div class="mb-6">
		<a
			href={resolve("/")}
			class="font-label text-label-md text-on-surface-variant hover:text-primary inline-flex items-center gap-1.5 transition-colors"
		>
			<ArrowLeft size={16} />
			{m.nav_library()}
		</a>
	</div>

	{#if loading}
		<div class="flex items-center justify-center py-32">
			<div
				class="border-secondary h-8 w-8 animate-spin rounded-full border-2 border-t-transparent"
			></div>
		</div>
	{:else if !book}
		<div class="paper-card rounded-xl p-16 text-center">
			<p class="font-body text-body-md text-on-surface-variant">Book not found</p>
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
			<div class="flex items-start gap-6">
				<BookCover bookId={book.id} class="h-32 w-24 shrink-0 rounded-xl" />
				<div class="min-w-0 flex-1 pt-2">
					<h1 class="font-display text-headline-md text-primary mb-2">{book.title}</h1>
					<p class="font-body text-body-md text-on-surface-variant italic">{book.author ?? "—"}</p>
					<div class="mt-3 flex flex-wrap items-center gap-2">
						<span
							class="font-label text-label-sm bg-surface-container-high rounded px-2.5 py-1 tracking-wider uppercase"
							>{formatBadge(book.format)}</span
						>
						<span class="font-label text-label-sm text-on-surface-variant"
							>{m.book_detail_field_read_status()}: {book.read_status}</span
						>
						{#if book.rating}
							<span class="text-secondary font-label text-label-sm">★ {book.rating}/5</span>
						{/if}
					</div>
				</div>
			</div>
			<div class="flex flex-wrap gap-2">
				<a
					href={resolve(`/reader/${book.id}`)}
					class="btn-primary text-label-md inline-flex items-center gap-1.5"
				>
					<BookOpen size={16} />
					{m.book_detail_reader_link()}
				</a>
				<button
					onclick={() => book && api.export(book.id, "epub")}
					class="font-label text-label-md text-on-surface-variant hover:text-primary inline-flex items-center gap-1.5 rounded-lg border border-[rgba(0,31,63,0.1)] px-4 py-2 transition-colors"
				>
					<FileDown size={16} />
					{m.reader_export()}
				</button>
				<button
					onclick={() => {
						editing = !editing;
						if (!editing) resetEdit();
					}}
					class="font-label text-label-md text-on-surface-variant hover:text-primary inline-flex items-center gap-1.5 rounded-lg border border-[rgba(0,31,63,0.1)] px-4 py-2 transition-colors"
				>
					<Pencil size={16} />
					{editing ? m.book_detail_cancel() : m.book_detail_edit()}
				</button>
				<button
					onclick={() => (showDeleteConfirm = true)}
					class="font-label text-label-md text-error hover:text-error/80 inline-flex items-center gap-1.5 rounded-lg px-4 py-2 transition-colors"
				>
					<Trash2 size={16} />
				</button>
			</div>
		</div>

		<div class="paper-card mb-section-gap rounded-xl p-8">
			<h3 class="font-display text-headline-sm text-primary mb-6">
				{editing ? m.book_detail_edit() : m.book_detail_title()}
			</h3>

			{#if editing}
				<div class="space-y-5">
					<div class="grid grid-cols-1 gap-5 md:grid-cols-2">
						<FieldEditor
							label={m.book_detail_field_title()}
							bind:value={editTitle}
							id="edit-title"
						/>
						<FieldEditor
							label={m.book_detail_field_author()}
							bind:value={editAuthor}
							id="edit-author"
						/>
						<FieldEditor label={m.book_detail_field_isbn()} bind:value={editIsbn} id="edit-isbn" />
						<FieldEditor
							label={m.book_detail_field_publisher()}
							bind:value={editPublisher}
							id="edit-publisher"
						/>
						<FieldEditor
							label={m.book_detail_field_language()}
							bind:value={editLanguage}
							id="edit-language"
						/>
						<FieldEditor
							label={m.book_detail_field_series()}
							bind:value={editSeries}
							id="edit-series"
						/>
						<FieldNumber
							label={m.book_detail_field_series_index()}
							bind:value={editSeriesIndex}
							id="edit-series-idx"
						/>
						<FieldNumber
							label={m.book_detail_field_page_count()}
							bind:value={editPageCount}
							id="edit-pages"
						/>
					</div>
					<div>
						<label
							for="edit-rating"
							class="font-label text-label-sm text-on-surface-variant mb-1.5 block tracking-widest uppercase"
							>{m.book_detail_field_rating()}</label
						>
						<div class="flex items-center gap-1">
							{#each [1, 2, 3, 4, 5] as star (star)}
								<button
									type="button"
									onclick={() => (editRating = star)}
									class="text-2xl transition-colors {(editRating ?? 0) >= star
										? 'text-secondary'
										: 'text-on-surface-variant/20'}">★</button
								>
							{/each}
							{#if editRating}
								<button
									onclick={() => (editRating = null)}
									class="text-on-surface-variant/30 ml-2 p-1"><X size={14} /></button
								>
							{/if}
						</div>
					</div>
					<div>
						<label
							for="edit-status"
							class="font-label text-label-sm text-on-surface-variant mb-1.5 block tracking-widest uppercase"
							>{m.book_detail_field_read_status()}</label
						>
						<select
							id="edit-status"
							bind:value={editReadStatus}
							class="bg-surface-container-low border-outline/10 font-label text-label-md text-primary w-full rounded-xl border px-4 py-3"
						>
							<option value="unread">{m.book_detail_read_status_unread()}</option>
							<option value="reading">{m.book_detail_read_status_reading()}</option>
							<option value="finished">{m.book_detail_read_status_finished()}</option>
							<option value="pending">{m.book_detail_read_status_pending()}</option>
						</select>
					</div>
					<div class="flex justify-end gap-3 pt-2">
						<button
							onclick={() => {
								editing = false;
								resetEdit();
							}}
							class="font-label text-label-md text-on-surface-variant px-4 py-2"
							>{m.book_detail_cancel()}</button
						>
						<button onclick={handleSave} disabled={saving} class="btn-primary"
							>{saving ? "..." : m.book_detail_save()}</button
						>
					</div>
				</div>
			{:else}
				<div class="grid grid-cols-1 gap-x-8 gap-y-5 md:grid-cols-2">
					<FieldDisplay label={m.book_detail_field_title()} value={book.title} id="disp-title" />
					<FieldDisplay
						label={m.book_detail_field_author()}
						value={book.author ?? "—"}
						id="disp-author"
					/>
					<FieldDisplay
						label={m.book_detail_field_isbn()}
						value={book.isbn ?? "—"}
						id="disp-isbn"
					/>
					<FieldDisplay
						label={m.book_detail_field_publisher()}
						value={book.publisher ?? "—"}
						id="disp-publisher"
					/>
					<FieldDisplay
						label={m.book_detail_field_language()}
						value={book.language ?? "—"}
						id="disp-lang"
					/>
					<FieldDisplay
						label={m.book_detail_field_series()}
						value={book.series ?? "—"}
						id="disp-series"
					/>
					<FieldDisplay
						label={m.book_detail_field_series_index()}
						value={book.series_index?.toString() ?? "—"}
						id="disp-series-idx"
					/>
					<FieldDisplay
						label={m.book_detail_field_page_count()}
						value={book.page_count?.toString() ?? "—"}
						id="disp-pages"
					/>
					<FieldDisplay
						label={m.book_detail_field_rating()}
						value={book.rating ? `★ ${book.rating}/5` : "—"}
						id="disp-rating"
					/>
					<FieldDisplay
						label={m.book_detail_field_read_status()}
						value={book.read_status}
						id="disp-status"
					/>
					<FieldDisplay
						label={m.book_detail_format()}
						value={formatBadge(book.format)}
						id="disp-format"
					/>
					<FieldDisplay
						label={m.book_detail_created()}
						value={formatDate(book.created_at)}
						id="disp-created"
					/>
				</div>
			{/if}
		</div>

		<div class="paper-card rounded-xl p-8">
			<div class="mb-6 flex items-center justify-between">
				<h3 class="font-display text-headline-sm text-primary">{m.metadata_title()}</h3>
				<div class="flex gap-2">
					<button
						onclick={handleSearchCandidates}
						disabled={searching}
						class="font-label text-label-md text-secondary hover:text-secondary/80 inline-flex items-center gap-1.5 transition-colors"
					>
						<Search size={16} />{searching ? m.metadata_searching() : m.metadata_search()}
					</button>
					<button
						onclick={handleRefresh}
						disabled={searchingMeta}
						class="font-label text-label-md text-on-surface-variant hover:text-primary inline-flex items-center gap-1.5 transition-colors"
					>
						<RefreshCw size={14} class={searchingMeta ? "animate-spin" : ""} />
						{searchingMeta ? m.metadata_refreshing() : m.metadata_refresh()}
					</button>
				</div>
			</div>

			{#if candidates.length > 0}
				<div class="mb-6 space-y-3">
					<p
						class="font-label text-label-sm text-on-surface-variant mb-2 tracking-widest uppercase"
					>
						Candidates
					</p>
					{#each candidates as candidate, i (candidate.provider + candidate.provider_id)}
						<div class="bg-surface-container-low rounded-xl p-4">
							<div class="mb-2 flex items-start justify-between gap-4">
								<div>
									<p class="font-display text-headline-sm mb-1">{candidate.title ?? "Unknown"}</p>
									<p class="font-body text-body-md text-on-surface-variant">
										{candidate.authors.join(", ") || "—"}
									</p>
									<p class="font-label text-label-sm text-secondary mt-1">
										{m.metadata_candidate_from({ provider: candidate.provider })}
									</p>
								</div>
								<button
									onclick={() => handleConfirmCandidate(i)}
									disabled={confirming && selectedCandidate === i}
									class="btn-primary text-label-sm shrink-0"
								>
									{confirming && selectedCandidate === i ? "..." : m.metadata_confirm()}
								</button>
							</div>
							{#if candidate.description}
								<p class="font-body text-body-md text-on-surface-variant mt-2 line-clamp-3">
									{candidate.description}
								</p>
							{/if}
							<div class="mt-2 flex flex-wrap gap-x-4 gap-y-1 text-xs">
								{#if candidate.publisher}<span
										class="font-label text-label-sm text-on-surface-variant/60"
										>{candidate.publisher}</span
									>{/if}
								{#if candidate.published_date}<span
										class="font-label text-label-sm text-on-surface-variant/60"
										>{candidate.published_date}</span
									>{/if}
								{#if candidate.page_count}<span
										class="font-label text-label-sm text-on-surface-variant/60"
										>{candidate.page_count}p</span
									>{/if}
								{#if candidate.isbn13}<span
										class="font-label text-label-sm text-on-surface-variant/60"
										>ISBN: {candidate.isbn13}</span
									>{/if}
								{#if candidate.isbn10}<span
										class="font-label text-label-sm text-on-surface-variant/60"
										>ISBN10: {candidate.isbn10}</span
									>{/if}
								{#if candidate.rating}<span class="text-secondary font-label text-label-sm"
										>★ {candidate.rating.toFixed(1)}</span
									>{/if}
							</div>
							{#if candidate.genres.length > 0}
								<div class="mt-2 flex flex-wrap gap-1">
									{#each candidate.genres as genre (genre)}
										<span
											class="font-label text-label-sm bg-secondary/5 text-secondary rounded-full px-2 py-0.5 text-[10px]"
											>{genre}</span
										>
									{/each}
								</div>
							{/if}
						</div>
					{/each}
				</div>
			{:else if searching}
				<div class="flex items-center justify-center py-8">
					<div
						class="border-secondary h-6 w-6 animate-spin rounded-full border-2 border-t-transparent"
					></div>
				</div>
			{/if}

			{#if Object.keys(providerIds).length > 0}
				<div class="mb-6">
					<p
						class="font-label text-label-sm text-on-surface-variant mb-2 tracking-widest uppercase"
					>
						{m.metadata_provider_ids()}
					</p>
					<div class="flex flex-wrap gap-2">
						{#each Object.entries(providerIds) as [provider, id] (provider)}
							<span
								class="font-label text-label-sm bg-surface-container-high rounded-lg px-3 py-1.5"
							>
								{provider}: <span class="text-secondary">{id as string}</span>
							</span>
						{/each}
					</div>
				</div>
			{/if}

			<div class="mb-6">
				<p class="font-label text-label-sm text-on-surface-variant tracking-widest uppercase">
					{m.metadata_last_refreshed()}: {meta?.last_refreshed_at
						? formatDate(meta.last_refreshed_at as string)
						: m.metadata_never()}
				</p>
			</div>

			<div>
				<p class="font-label text-label-sm text-on-surface-variant mb-3 tracking-widest uppercase">
					Field Locks
				</p>
				<div class="flex flex-wrap gap-2">
					{#each metadataFields as field (field)}
						{@const locked = lockedFields.includes(field)}
						<button
							onclick={() => (locked ? handleUnlockField(field) : handleLockField(field))}
							class={[
								"font-label text-label-sm inline-flex items-center gap-1.5 rounded-lg border px-3 py-1.5 transition-all",
								locked
									? "bg-secondary/5 border-secondary/20 text-secondary"
									: "text-on-surface-variant border-[rgba(0,31,63,0.08)] hover:border-[rgba(0,31,63,0.2)]"
							]}
							title={locked ? m.metadata_unlock_field() : m.metadata_lock_field()}
						>
							{#if locked}<Lock size={12} />{:else}<Unlock size={12} />{/if}
							{fieldLabels[field]}
						</button>
					{/each}
				</div>
			</div>
		</div>

		{#if showDeleteConfirm}
			<div
				class="bg-primary/40 fixed inset-0 z-50 flex items-center justify-center p-4 backdrop-blur-sm"
				role="dialog"
				aria-modal="true"
				tabindex="-1"
				onclick={() => (showDeleteConfirm = false)}
				onkeydown={(e) => {
					if (e.key === "Escape") showDeleteConfirm = false;
				}}
			>
				<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
				<div
					class="bg-surface mx-auto w-full max-w-md rounded-2xl p-8 shadow-2xl"
					role="document"
					tabindex="-1"
					onclick={(e) => e.stopPropagation()}
					onkeydown={(e) => {
						if (e.key === "Escape") showDeleteConfirm = false;
					}}
				>
					<h4 class="font-display text-headline-sm text-primary mb-4">{m.book_detail_delete()}</h4>
					<p class="font-body text-body-md text-on-surface-variant mb-6">
						{m.book_detail_delete_confirm({ title: book.title })}
					</p>
					<div class="flex justify-end gap-3">
						<button
							onclick={() => (showDeleteConfirm = false)}
							class="font-label text-label-md text-on-surface-variant px-4 py-2"
							>{m.book_detail_cancel()}</button
						>
						<button
							onclick={handleDelete}
							disabled={deleting}
							class="font-label text-label-md bg-error inline-flex items-center gap-1.5 rounded-lg px-4 py-2 text-white transition-colors hover:opacity-90 disabled:opacity-50"
						>
							<Trash2 size={16} />{deleting ? "..." : m.book_detail_delete()}
						</button>
					</div>
				</div>
			</div>
		{/if}
	{/if}
</section>
