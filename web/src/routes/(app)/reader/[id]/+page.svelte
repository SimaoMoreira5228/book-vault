<script lang="ts">
	import * as m from "$lib/paraglide/messages";
	import { api, apiBase } from "$lib/api/client.svelte";
	import { page } from "$app/state";
	import type { BookIr, BookResponse } from "$lib/api/generated";
	import type { Annotation, ReaderTheme } from "$lib/ir/renderer";
	import { getBlockText } from "$lib/ir/renderer";
	import ReaderLayout from "$lib/components/reader/ReaderLayout.svelte";
	import BlockRenderer from "$lib/components/reader/BlockRenderer.svelte";
	import AnnotationPopup from "$lib/components/reader/AnnotationPopup.svelte";
	import AnnotationTooltip from "$lib/components/reader/AnnotationTooltip.svelte";
	import TocPanel from "$lib/components/reader/TocPanel.svelte";
	import FontPanel from "$lib/components/reader/FontPanel.svelte";
	import ComicViewer from "$lib/components/reader/ComicViewer.svelte";
	import ExportMenu from "$lib/components/reader/ExportMenu.svelte";
	import SpanText from "$lib/components/SpanText.svelte";
	import BookOpen from "@lucide/svelte/icons/book-open";
	import FileDown from "@lucide/svelte/icons/file-down";
	import Bookmark from "@lucide/svelte/icons/bookmark";
	import Download from "@lucide/svelte/icons/download";
	import ChevronLeft from "@lucide/svelte/icons/chevron-left";
	import ChevronRight from "@lucide/svelte/icons/chevron-right";

	const bookId = $derived(page.params.id ?? "");

	let bookData = $state<{ book: BookIr } | null>(null);
	let meta = $state<BookResponse | null>(null);
	let loading = $state(true);
	let progress = $state(0);
	let pdfMode = $state<"text" | "pdf">("text");

	let comicPages = $state<Array<{ page: number; asset_id: string; mime_type: string }>>([]);
	let comicCurrentPage = $state(1);
	let sectionId = $state("");

	let theme = $state<ReaderTheme>("light");
	let fontSize = $state(18);
	let lineHeight = $state(1.8);
	let showToc = $state(false);
	let showFontPanel = $state(false);

	let annotations = $state<Annotation[]>([]);
	let popup = $state<{
		x: number;
		y: number;
		text: string;
		blockIndex: number;
		startOffset: number;
		endOffset: number;
	} | null>(null);
	let tooltipAnn = $state<Annotation | null>(null);
	let bookmarks = $state<Array<{ id: string; section_id: string }>>([]);
	let showExport = $state(false);

	let saveTimer: ReturnType<typeof setInterval> | undefined;

	const formatsWithDownload = $derived(["pdf", "mobi_raw", "epub"]);
	const showDownload = $derived(meta ? formatsWithDownload.includes(meta.format) : false);

	$effect(() => {
		if (bookId) {
			loadBook();
			loadProgress();
		}
		return () => {
			if (saveTimer) clearInterval(saveTimer);
		};
	});

	$effect(() => {
		if (bookData && sectionId) saveTimer = setInterval(() => saveProgressNow(), 15000);
	});

	async function loadBook() {
		loading = true;
		const [metaResult, readResult] = await Promise.all([api.books.get(bookId), api.read(bookId)]);
		if (metaResult.isOk()) meta = metaResult.value;
		if (readResult.isOk()) {
			bookData = readResult.value as { book: BookIr };
			const firstSection = (readResult.value as { book: BookIr }).book.spine[0];
			if (firstSection) sectionId = firstSection.id;
		}
		if (metaResult.isOk() && metaResult.value.format === "cbz") loadComicPages();
		loading = false;
		await Promise.all([loadAnnotations(), loadBookmarks()]);
	}

	async function loadAnnotations() {
		const result = await api.annotations.list(bookId);
		if (result.isOk()) annotations = result.value as unknown as Annotation[];
	}

	async function loadBookmarks() {
		const r = await api.bookmarks.list(bookId);
		if (r.isOk()) bookmarks = r.value as unknown as Array<{ id: string; section_id: string }>;
	}

	async function loadComicPages() {
		const result = await api.comic.pages(bookId);
		if (result.isOk()) comicPages = result.value;
	}

	async function loadProgress() {
		const result = await api.progress.get(bookId);
		if (result.isOk() && result.value) {
			progress = Math.round(result.value.percentage);
			sectionId = result.value.section_id;
		}
	}

	async function saveProgressNow() {
		if (!sectionId) return;
		api.progress.save(bookId, {
			section_id: sectionId,
			block_index: Math.round(progress / 10),
			char_offset: 0,
			percentage: progress
		});
	}

	async function toggleBookmark() {
		const existing = bookmarks.find((b) => b.section_id === sectionId);
		if (existing) {
			await api.bookmarks.delete(existing.id);
			bookmarks = bookmarks.filter((b) => b.section_id !== sectionId);
		} else {
			const r = await api.bookmarks.create({
				book_id: bookId,
				section_id: sectionId,
				block_index: Math.round(progress / 10)
			});
			if (r.isOk())
				bookmarks = [
					...bookmarks,
					{ id: (r.value as unknown as { id: string }).id, section_id: sectionId }
				];
		}
	}

	function onScroll(e: Event) {
		const el = e.target as HTMLElement;
		const scrollTop = el.scrollTop;
		const scrollHeight = el.scrollHeight - el.clientHeight;
		progress = scrollHeight > 0 ? Math.min(100, Math.round((scrollTop / scrollHeight) * 100)) : 0;
	}

	function onTextSelect() {
		tooltipAnn = null;
		const sel = window.getSelection();
		if (!sel || sel.isCollapsed || !sel.rangeCount) {
			popup = null;
			return;
		}
		const range = sel.getRangeAt(0);
		const text = sel.toString().trim();
		if (!text) {
			popup = null;
			return;
		}

		let node: Node | null = range.startContainer;
		while (node && (!(node as HTMLElement).dataset || !(node as HTMLElement).dataset.blockIndex)) {
			node = node.parentElement;
		}
		if (!node) {
			popup = null;
			return;
		}
		const blockIndex = parseInt((node as HTMLElement).dataset.blockIndex ?? "");

		const blockEl = node as HTMLElement;
		const walker = document.createTreeWalker(blockEl, NodeFilter.SHOW_TEXT);
		let startOffset = 0;
		let foundStart = false;
		let endOffset = 0;
		let foundEnd = false;
		while (walker.nextNode()) {
			const tn = walker.currentNode as Text;
			if (!foundStart && tn === range.startContainer) {
				startOffset += range.startOffset;
				foundStart = true;
			} else if (!foundStart) {
				startOffset += tn.length;
			}
			if (!foundEnd && tn === range.endContainer) {
				endOffset += range.endOffset;
				foundEnd = true;
			} else if (!foundEnd) {
				endOffset += tn.length;
			}
		}

		const rect = range.getBoundingClientRect();
		popup = {
			x: rect.left + rect.width / 2,
			y: rect.top - 10,
			text,
			blockIndex,
			startOffset,
			endOffset
		};
	}

	async function createAnnotation(color: string) {
		if (!popup) return;
		const section = bookData?.book.spine.find((s) => s.id === sectionId);
		if (!section) return;
		const block = section.blocks[popup.blockIndex] as Record<string, unknown>;
		const blockText = getBlockText(block);
		const startOffset = Math.max(0, Math.min(popup.startOffset, blockText.length));
		const endOffset = Math.max(startOffset, Math.min(popup.endOffset, blockText.length));
		if (startOffset >= endOffset) {
			popup = null;
			return;
		}

		const result = await api.annotations.create(bookId, {
			section_id: sectionId,
			block_index: popup.blockIndex,
			start_offset: startOffset,
			end_offset: endOffset,
			color
		});
		if (result.isOk()) {
			const fetched = await api.annotations.list(bookId);
			if (fetched.isOk()) annotations = fetched.value as unknown as Annotation[];
		}
		popup = null;
		window.getSelection()?.removeAllRanges();
	}

	function scrollToSection(sid: string) {
		const el = document.querySelector(`[data-section-id="${sid}"]`);
		if (el) {
			el.scrollIntoView({ behavior: "smooth" });
			sectionId = sid;
		}
		showToc = false;
	}

	function goToPrevSection() {
		const idx = bookData?.book.spine.findIndex((s) => s.id === sectionId) ?? -1;
		if (idx > 0) scrollToSection(bookData!.book.spine[idx - 1].id);
	}

	function goToNextSection() {
		const idx = bookData?.book.spine.findIndex((s) => s.id === sectionId) ?? -1;
		if (idx < (bookData?.book.spine.length ?? 1) - 1)
			scrollToSection(bookData!.book.spine[idx + 1].id);
	}

	$effect(() => {
		const el = document.querySelector("main");
		if (!el) return;
		const handler = (e: Event) => onScroll(e);
		el.addEventListener("scroll", handler);
		return () => el.removeEventListener("scroll", handler);
	});

	function handleAnnotationClick(ann: Annotation) {
		popup = null;
		window.getSelection()?.removeAllRanges();
		tooltipAnn = ann;
	}

	const tocSections = $derived(bookData?.book.spine ?? []);
	const rawUrl = $derived(bookId ? `${apiBase}/api/v1/books/${bookId}/raw` : "");
</script>

<svelte:window onselectionchange={onTextSelect} />

<ReaderLayout
	bind:theme
	{progress}
	bind:showToc
	bind:showFontPanel
	title={meta?.title ?? bookData?.book.spine[0]?.title ?? m.reader_loading()}
>
	{#snippet headerExtra()}
		{#if meta?.format === "pdf"}
			<div class="bg-surface-container-high mr-2 flex items-center gap-1 rounded-lg p-1">
				<button
					onclick={() => (pdfMode = "text")}
					class={[
						"font-label rounded-md px-3 py-1.5 text-xs transition-all",
						pdfMode === "text"
							? "bg-secondary text-white shadow-sm"
							: "text-on-surface-variant hover:text-primary"
					]}>{m.reader_tab_text()}</button
				>
				<button
					onclick={() => (pdfMode = "pdf")}
					class={[
						"font-label rounded-md px-3 py-1.5 text-xs transition-all",
						pdfMode === "pdf"
							? "bg-secondary text-white shadow-sm"
							: "text-on-surface-variant hover:text-primary"
					]}>{m.reader_tab_pdf()}</button
				>
			</div>
		{/if}
		{#if showDownload}
			<div class="relative">
				<button
					onclick={() => (showExport = !showExport)}
					class="p-2 transition-transform duration-200 hover:opacity-80 active:scale-95"
					title={m.reader_export()}
				>
					<FileDown size={20} class="text-on-surface-variant" />
				</button>
				<ExportMenu bind:show={showExport} {bookId} />
			</div>
		{/if}
		<button
			onclick={toggleBookmark}
			class="p-2 transition-transform duration-200 hover:opacity-80 active:scale-95"
			title={bookmarks.find((b) => b.section_id === sectionId)
				? m.reader_bookmark_remove()
				: m.reader_bookmark_add()}
		>
			<Bookmark
				size={20}
				class={bookmarks.find((b) => b.section_id === sectionId)
					? "text-secondary"
					: "text-on-surface-variant"}
			/>
		</button>
	{/snippet}

	<TocPanel sections={tocSections} bind:show={showToc} onNavigate={scrollToSection} />
	<FontPanel bind:fontSize bind:lineHeight />

	{#if loading}
		<div class="flex items-center justify-center py-32">
			<div
				class="border-secondary h-8 w-8 animate-spin rounded-full border-2 border-t-transparent"
			></div>
		</div>
	{:else if meta?.format === "pdf" && pdfMode === "pdf"}
		<object
			data={rawUrl}
			type="application/pdf"
			class="border-outline/10 h-screen w-full rounded-xl border"
			title="PDF Viewer"
		>
			<p class="font-body text-body-md text-on-surface-variant py-16 text-center">
				Your browser does not support PDF embedding.
				<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
				<a href={rawUrl} class="text-secondary underline" target="_blank">Download instead</a>.
			</p>
		</object>
	{:else if meta?.format === "cbz"}
		<ComicViewer {bookId} bind:pages={comicPages} bind:currentPage={comicCurrentPage} />
	{:else if meta?.format === "mobi_raw"}
		<div class="flex flex-col items-center justify-center gap-6 py-32">
			<BookOpen size={64} class="text-on-surface-variant/20" />
			<p class="font-body text-body-lg text-on-surface-variant text-center">
				This book is in MOBI format and cannot be previewed inline.
			</p>
			<button onclick={() => api.raw(bookId)} class="btn-primary"
				><Download size={20} /> {m.reader_download()}</button
			>
		</div>
	{:else if bookData}
		<article
			data-book-content
			class="space-y-8"
			style="font-size: {fontSize}px; line-height: {lineHeight};"
		>
			{#each bookData.book.spine as section (section.id)}
				<div data-section-id={section.id}>
					{#if section.title}
						<h2 class="font-display text-headline-md text-primary mb-12 text-center">
							{section.title}
						</h2>
					{/if}
					{#each section.blocks as block, blockIdx (blockIdx)}
						<BlockRenderer
							{block}
							{blockIdx}
							sectionId={section.id}
							{annotations}
							{theme}
							onAnnotationClick={handleAnnotationClick}
							{onTextSelect}
						/>
					{/each}
				</div>
			{/each}
		</article>
	{/if}
</ReaderLayout>

<AnnotationPopup
	show={popup !== null}
	x={popup?.x ?? 0}
	y={popup?.y ?? 0}
	onCreateColor={createAnnotation}
/>
<AnnotationTooltip bind:annotation={tooltipAnn} onUpdate={loadAnnotations} />

<footer
	class="px-margin-mobile bg-surface/80 fixed bottom-0 left-0 flex w-full items-center justify-between py-6 backdrop-blur-sm"
	style="z-index: 40;"
>
	<div class="flex items-center gap-2">
		<button
			onclick={goToPrevSection}
			class="font-label text-label-sm text-on-surface-variant hover:text-primary flex items-center gap-1 p-2 transition-colors"
		>
			<ChevronLeft size={14} />{m.reader_prev()}
		</button>
	</div>
	<div class="font-label text-label-md text-on-surface-variant/60 tracking-widest">
		{m.reader_complete({ progress })}
	</div>
	<div class="flex items-center gap-2">
		<button
			onclick={goToNextSection}
			class="font-label text-label-sm text-on-surface-variant hover:text-primary flex items-center gap-1 p-2 transition-colors"
		>
			{m.reader_next()}<ChevronRight size={14} />
		</button>
	</div>
</footer>
