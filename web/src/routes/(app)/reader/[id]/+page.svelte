<script lang="ts">
	import * as m from "$lib/paraglide/messages";
	import { api } from "$lib/api/client";
	import { page } from "$app/state";
	import { goto } from "$app/navigation";
	import type { BookIr, BookResponse, Span } from "$lib/api/generated";
	import ArrowLeft from "@lucide/svelte/icons/arrow-left";
	import Type from "@lucide/svelte/icons/type";
	import Bookmark from "@lucide/svelte/icons/bookmark";
	import ChevronLeft from "@lucide/svelte/icons/chevron-left";
	import ChevronRight from "@lucide/svelte/icons/chevron-right";
	import Download from "@lucide/svelte/icons/download";
	import BookOpen from "@lucide/svelte/icons/book-open";

	const bookId = $derived(page.params.id ?? "");

	let bookData = $state<{ book: BookIr } | null>(null);
	let meta = $state<BookResponse | null>(null);
	let loading = $state(true);
	let progress = $state(0);
	let pdfMode = $state<"text" | "pdf">("text");

	let comicPages = $state<Array<{ page: number; asset_id: string; mime_type: string }>>([]);
	let comicPage = $state(1);
	let comicLoading = $state(false);

	let formatsWithDownload = $derived(["pdf", "mobi_raw", "epub"]);

	$effect(() => {
		if (bookId) {
			loadBook();
		}
	});

	async function loadBook() {
		loading = true;
		const [metaResult, readResult] = await Promise.all([api.books.get(bookId), api.read(bookId)]);
		if (metaResult.isOk()) {
			meta = metaResult.value;
		}
		if (readResult.isOk()) {
			bookData = readResult.value as { book: BookIr };
		}
		if (metaResult.isOk() && metaResult.value.format === "cbz") {
			loadComicPages();
		}
		loading = false;
	}

	async function loadComicPages() {
		comicLoading = true;
		const result = await api.comic.pages(bookId);
		if (result.isOk()) {
			comicPages = result.value;
		}
		comicLoading = false;
	}

	function prevPage() {
		if (comicPage > 1) comicPage--;
	}

	function nextPage() {
		if (comicPage < comicPages.length) comicPage++;
	}

	function onScroll(e: Event) {
		const el = e.target as HTMLElement;
		const scrollTop = el.scrollTop;
		const scrollHeight = el.scrollHeight - el.clientHeight;
		progress = scrollHeight > 0 ? Math.min(100, Math.round((scrollTop / scrollHeight) * 100)) : 0;
	}

	function extractText(spans: Span[]): string {
		return spans.map((s) => s.text).join("");
	}

	const rawUrl = $derived(bookId ? `/api/v1/books/${bookId}/raw` : "");
	const currentComicUrl = $derived(bookId ? `/api/v1/books/${bookId}/comic/page/${comicPage}` : "");
	function getAssetUrl(assetId: string) {
		return api.asset(bookId, assetId);
	}
	const showDownload = $derived(meta ? formatsWithDownload.includes(meta.format) : false);
</script>

<svelte:window onscroll={onScroll} />

<div class="bg-surface-container-low/20 fixed top-0 left-0 z-[60] w-full">
	<div class="bg-secondary h-[2px] transition-all duration-300" style="width: {progress}%;" />
</div>

<header
	class="bg-surface/90 px-margin-mobile md:px-margin-desktop fixed top-0 z-50 flex h-16 w-full items-center justify-between shadow-[0_1px_4px_rgba(0,31,63,0.05)] backdrop-blur-md"
>
	<div class="flex items-center gap-4">
		<button
			onclick={() => goto("/")}
			class="flex items-center justify-center p-2 transition-transform duration-200 hover:opacity-80 active:scale-95"
		>
			<ArrowLeft size={20} class="text-primary" />
		</button>
		<h1 class="font-display text-headline-sm text-primary max-w-[240px] truncate md:max-w-md">
			{meta?.title ?? bookData?.book.spine[0]?.title ?? m.reader_loading()}
		</h1>
	</div>
	<div class="flex items-center gap-2">
		{#if meta?.format === "pdf"}
			<div class="bg-surface-container-high mr-2 flex items-center gap-1 rounded-lg p-1">
				<button
					onclick={() => (pdfMode = "text")}
					class={[
						"font-label rounded-md px-3 py-1.5 text-xs transition-all",
						pdfMode === "text"
							? "bg-secondary text-white shadow-sm"
							: "text-on-surface-variant hover:text-primary"
					]}
				>
					{m.reader_tab_text()}
				</button>
				<button
					onclick={() => (pdfMode = "pdf")}
					class={[
						"font-label rounded-md px-3 py-1.5 text-xs transition-all",
						pdfMode === "pdf"
							? "bg-secondary text-white shadow-sm"
							: "text-on-surface-variant hover:text-primary"
					]}
				>
					{m.reader_tab_pdf()}
				</button>
			</div>
		{/if}
		{#if showDownload}
			<button
				onclick={() => api.raw(bookId)}
				class="p-2 transition-transform duration-200 hover:opacity-80 active:scale-95"
				title={m.reader_download()}
			>
				<Download size={20} class="text-on-surface-variant" />
			</button>
		{/if}
		<button class="p-2 transition-transform duration-200 hover:opacity-80 active:scale-95">
			<Type size={20} class="text-on-surface-variant" />
		</button>
		<button class="p-2 transition-transform duration-200 hover:opacity-80 active:scale-95">
			<Bookmark size={20} class="text-on-surface-variant" />
		</button>
	</div>
</header>

<main
	class="px-margin-mobile min-h-screen pt-32 pb-40 md:px-0"
	style="max-width: 800px; margin: 0 auto;"
>
	{#if loading}
		<div class="flex items-center justify-center py-32">
			<div
				class="border-secondary h-8 w-8 animate-spin rounded-full border-2 border-t-transparent"
			/>
		</div>
	{:else if meta?.format === "pdf" && pdfMode === "pdf"}
		<object
			data={rawUrl}
			type="application/pdf"
			class="border-outline/10 h-screen w-full rounded-xl border"
			title="PDF Viewer"
		>
			<p class="font-body text-body-md text-on-surface-variant py-16 text-center">
				Your browser does not support PDF embedding. <a
					href={rawUrl}
					class="text-secondary underline"
					target="_blank">Download instead</a
				>.
			</p>
		</object>
	{:else if meta?.format === "cbz" && !comicLoading}
		{#if comicPages.length > 0}
			<div class="flex flex-col items-center gap-6">
				<div
					class="border-outline/10 bg-surface-container w-full max-w-3xl overflow-hidden rounded-xl border"
				>
					<img src={currentComicUrl} alt="Page {comicPage}" class="h-auto w-full" />
				</div>
				<div class="flex items-center gap-4">
					<button
						onclick={prevPage}
						disabled={comicPage <= 1}
						class="font-label text-label-sm text-on-surface-variant hover:text-primary flex items-center gap-1 p-2 transition-colors disabled:cursor-not-allowed disabled:opacity-30"
					>
						<ChevronLeft size={14} />
						{m.reader_prev()}
					</button>
					<span class="font-label text-label-md text-on-surface-variant/60 tracking-widest">
						{comicPage} / {comicPages.length}
					</span>
					<button
						onclick={nextPage}
						disabled={comicPage >= comicPages.length}
						class="font-label text-label-sm text-on-surface-variant hover:text-primary flex items-center gap-1 p-2 transition-colors disabled:cursor-not-allowed disabled:opacity-30"
					>
						{m.reader_next()}
						<ChevronRight size={14} />
					</button>
				</div>
			</div>
		{/if}
	{:else if meta?.format === "mobi_raw"}
		<div class="flex flex-col items-center justify-center gap-6 py-32">
			<BookOpen size={64} class="text-on-surface-variant/20" />
			<p class="font-body text-body-lg text-on-surface-variant text-center">
				This book is in MOBI format and cannot be previewed inline.
			</p>
			<button onclick={() => api.raw(bookId)} class="btn-primary">
				<Download size={20} />
				{m.reader_download()}
			</button>
		</div>
	{:else if bookData}
		<article class="space-y-8">
			{#each bookData.book.spine as section (section.id)}
				{#if section.title}
					<h2 class="font-display text-headline-md text-primary mb-12 text-center">
						{section.title}
					</h2>
				{/if}
				{#each section.blocks as block, i (i)}
					{@const b = block as Record<string, unknown>}
					{#if "Paragraph" in b}
						<p class="font-body text-body-lg text-on-surface/90 mb-8 leading-relaxed">
							{extractText(b.Paragraph as Span[])}
						</p>
					{:else if "Heading" in b}
						{@const h = b.Heading as { level: number; spans: Span[] }}
						<h3 class="font-display text-headline-sm text-primary mt-12 mb-6">
							{extractText(h.spans)}
						</h3>
					{:else if "Image" in b}
						{@const img = b.Image as { asset_ref: string; alt: string | null }}
						<div
							class="border-on-surface/5 bg-surface-container my-16 overflow-hidden rounded-xl border"
						>
							<img
								src={getAssetUrl(img.asset_ref)}
								alt={img.alt ?? ""}
								class="h-auto w-full"
								loading="lazy"
							/>
						</div>
					{:else if "CodeBlock" in b}
						{@const cb = b.CodeBlock as { language: string | null; content: string }}
						<pre
							class="bg-surface-container-high mb-8 overflow-x-auto rounded-xl p-6 font-mono text-sm"><code
								>{cb.content}</code
							></pre>
					{:else if "HorizontalRule" in b}
						<hr class="border-outline-variant my-12" />
					{/if}
				{/each}
			{/each}
		</article>
	{/if}
</main>

<footer
	class="px-margin-mobile bg-surface/80 fixed bottom-0 left-0 flex w-full items-center justify-between py-6 backdrop-blur-sm"
>
	<div class="flex items-center gap-2">
		<button
			class="font-label text-label-sm text-on-surface-variant hover:text-primary flex items-center gap-1 p-2 transition-colors"
		>
			<ChevronLeft size={14} />
			{m.reader_prev()}
		</button>
	</div>
	<div class="font-label text-label-md text-on-surface-variant/60 tracking-widest">
		{m.reader_complete({ progress })}
	</div>
	<div class="flex items-center gap-2">
		<button
			class="font-label text-label-sm text-on-surface-variant hover:text-primary flex items-center gap-1 p-2 transition-colors"
		>
			{m.reader_next()}
			<ChevronRight size={14} />
		</button>
	</div>
</footer>
