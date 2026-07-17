<script lang="ts">
	import * as m from "$lib/paraglide/messages";
	import { api } from "$lib/api/client";
	import { page } from "$app/state";
	import { goto } from "$app/navigation";
	import type { BookIr, BookResponse, Span } from "$lib/api/generated";
	import Popup from "$lib/components/Popup.svelte";
	import Modal from "$lib/components/Modal.svelte";
	import ArrowLeft from "@lucide/svelte/icons/arrow-left";
	import Type from "@lucide/svelte/icons/type";
	import Bookmark from "@lucide/svelte/icons/bookmark";
	import ChevronLeft from "@lucide/svelte/icons/chevron-left";
	import ChevronRight from "@lucide/svelte/icons/chevron-right";
	import FileDown from "@lucide/svelte/icons/file-down";
	import Download from "@lucide/svelte/icons/download";
	import BookOpen from "@lucide/svelte/icons/book-open";
	import List from "@lucide/svelte/icons/list";
	import Sun from "@lucide/svelte/icons/sun";
	import Plus from "@lucide/svelte/icons/plus";
	import Minus from "@lucide/svelte/icons/minus";
	import X from "@lucide/svelte/icons/x";
	import MessageSquareText from "@lucide/svelte/icons/message-square-text";

	type ReaderTheme = "light" | "dark" | "sepia";

	type Annotation = {
		id: string;
		book_id: string;
		section_id: string;
		block_index: number;
		start_offset: number;
		end_offset: number;
		color: string | null;
		note: string | null;
	};

	type Segment = {
		text: string;
		annotationId?: string;
		color?: string;
	};

	const COLORS: Record<string, string> = {
		yellow: "#fef08a",
		green: "#bbf7d0",
		blue: "#bfdbfe",
		pink: "#fbcfe8",
		orange: "#fed7aa"
	};

	const COLOR_NAMES = ["yellow", "green", "blue", "pink", "orange"];

	const bookId = $derived(page.params.id ?? "");

	let bookData = $state<{ book: BookIr } | null>(null);
	let meta = $state<BookResponse | null>(null);
	let loading = $state(true);
	let progress = $state(0);
	let pdfMode = $state<"text" | "pdf">("text");

	let comicPages = $state<Array<{ page: number; asset_id: string; mime_type: string }>>([]);
	let comicPage = $state(1);
	let comicLoading = $state(false);
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
	let tooltip = $state<{ x: number; y: number; annotation: Annotation } | null>(null);
	let editNoteId = $state<string | null>(null);
	let noteDraft = $state("");
	let deleting = $state<string | null>(null);
	let bookmarks = $state<Array<{ id: string; section_id: string }>>([]);

	async function loadBookmarks() {
		const r = await api.bookmarks.list(bookId);
		if (r.isOk()) bookmarks = r.value as unknown as Array<{ id: string; section_id: string }>;
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

	let formatsWithDownload = $derived(["pdf", "mobi_raw", "epub"]);
	let saveTimer: ReturnType<typeof setInterval> | undefined;

	const themeClasses: Record<ReaderTheme, string> = {
		light: "bg-surface text-on-surface",
		dark: "bg-neutral-900 text-neutral-100",
		sepia: "bg-amber-50 text-amber-950"
	};

	const themeContentStyles: Record<ReaderTheme, string> = {
		light: "text-on-surface/90",
		dark: "text-neutral-200",
		sepia: "text-amber-900"
	};

	function cycleTheme() {
		const themes: ReaderTheme[] = ["light", "sepia", "dark"];
		const idx = themes.indexOf(theme);
		theme = themes[(idx + 1) % themes.length];
	}

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
		if (bookData && sectionId) {
			saveTimer = setInterval(() => saveProgressNow(), 15000);
		}
	});

	async function loadProgress() {
		const result = await api.progress.get(bookId);
		if (result.isOk() && result.value) {
			progress = Math.round(result.value.percentage);
			sectionId = result.value.section_id;
		}
	}

	async function saveProgressNow() {
		if (!sectionId) return;
		const currentBlock = Math.round(progress / 10);
		api.progress.save(bookId, {
			section_id: sectionId,
			block_index: currentBlock,
			char_offset: 0,
			percentage: progress
		});
	}

	async function loadAnnotations() {
		const result = await api.annotations.list(bookId);
		if (result.isOk()) annotations = result.value as unknown as Annotation[];
	}

	function getBlockAnnotations(
		sectionIdx: number,
		blockIdx: number,
		sectionId: string
	): Annotation[] {
		return annotations.filter((a) => a.section_id === sectionId && a.block_index === blockIdx);
	}

	function splitTextAtAnnotations(text: string, blockAnnotations: Annotation[]): Segment[] {
		if (!blockAnnotations.length) return [{ text }];
		const segments: Segment[] = [];
		let pos = 0;
		const sorted = [...blockAnnotations].sort((a, b) => a.start_offset - b.start_offset);
		for (const ann of sorted) {
			if (ann.start_offset > pos) {
				segments.push({ text: text.slice(pos, ann.start_offset) });
			}
			if (ann.start_offset < text.length && ann.end_offset > 0) {
				segments.push({
					text: text.slice(Math.max(0, ann.start_offset), Math.min(text.length, ann.end_offset)),
					annotationId: ann.id,
					color: ann.color ?? "yellow"
				});
			}
			pos = Math.max(pos, ann.end_offset);
		}
		if (pos < text.length) segments.push({ text: text.slice(pos) });
		return segments;
	}

	function getBlockText(block: Record<string, unknown>): string {
		if ("Paragraph" in block) return (block.Paragraph as Span[]).map((s) => s.text).join("");
		if ("Heading" in block)
			return (block.Heading as { spans: Span[] }).spans.map((s) => s.text).join("");
		return "";
	}

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
		await loadAnnotations();
		await loadBookmarks();
	}

	async function loadComicPages() {
		comicLoading = true;
		const result = await api.comic.pages(bookId);
		if (result.isOk()) comicPages = result.value;
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

	function onTextSelect() {
		tooltip = null;
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
		let foundEnd = false;
		let endOffset = 0;
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
			annotations = [...((result.value as unknown as Annotation[]) ?? []), ...annotations];
			const fetched = await api.annotations.list(bookId);
			if (fetched.isOk()) annotations = fetched.value as unknown as Annotation[];
		}
		popup = null;
		window.getSelection()?.removeAllRanges();
	}

	function handleAnnotationClick(ann: Annotation) {
		popup = null;
		window.getSelection()?.removeAllRanges();
		tooltip = { x: 0, y: 0, annotation: ann };
	}

	function startEditNote(ann: Annotation) {
		editNoteId = ann.id;
		noteDraft = ann.note ?? "";
	}

	async function saveNote(annId: string) {
		const result = await api.annotations.update(annId, { note: noteDraft });
		if (result.isOk()) {
			annotations = annotations.map((a) => (a.id === annId ? { ...a, note: noteDraft } : a));
		}
		editNoteId = null;
	}

	async function deleteAnnotation(annId: string) {
		deleting = annId;
		const result = await api.annotations.delete(annId);
		if (result.isOk()) {
			annotations = annotations.filter((a) => a.id !== annId);
		}
		deleting = null;
		tooltip = null;
	}

	const rawUrl = $derived(bookId ? `/api/v1/books/${bookId}/raw` : "");
	const currentComicUrl = $derived(bookId ? `/api/v1/books/${bookId}/comic/page/${comicPage}` : "");
	function getAssetUrl(assetId: string) {
		return api.asset(bookId, assetId);
	}
	const showDownload = $derived(meta ? formatsWithDownload.includes(meta.format) : false);
	let showExport = $state(false);
	const tocEntries = $derived(bookData?.book.spine.filter((s) => s.title) ?? []);

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
</script>

<svelte:window onselectionchange={onTextSelect} />

<div class={["min-h-screen transition-colors duration-300", themeClasses[theme]]}>
	<div class="bg-surface-container-low/20 fixed top-0 left-0 z-[60] w-full">
		<div class="bg-secondary h-[2px] transition-all duration-300" style="width: {progress}%;"></div>
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
			{#if tocEntries.length > 0}
				<button
					onclick={() => (showToc = !showToc)}
					class="p-2 transition-transform duration-200 hover:opacity-80 active:scale-95"
					title="Table of Contents"
				>
					<List size={20} class="text-on-surface-variant" />
				</button>
			{/if}
			<button
				onclick={cycleTheme}
				class="p-2 transition-transform duration-200 hover:opacity-80 active:scale-95"
				title="Toggle theme"
			>
				<Sun size={20} class="text-on-surface-variant" />
			</button>
			<button
				onclick={() => (showFontPanel = !showFontPanel)}
				class="p-2 transition-transform duration-200 hover:opacity-80 active:scale-95"
				title="Font settings"
			>
				<Type size={20} class="text-on-surface-variant" />
			</button>
			{#if showDownload}
				<div class="relative">
					<button
						onclick={() => (showExport = !showExport)}
						class="p-2 transition-transform duration-200 hover:opacity-80 active:scale-95"
						title={m.reader_export()}
					>
						<FileDown size={20} class="text-on-surface-variant" />
					</button>
					{#if showExport}
						<div
							class="bg-surface border-outline/10 absolute top-10 right-0 z-50 min-w-[140px] rounded-xl border p-1.5 shadow-lg"
						>
							<button
								onclick={() => {
									api.export(bookId, "epub");
									showExport = false;
								}}
								class="font-label text-label-md text-on-surface-variant hover:text-primary hover:bg-surface-container-low w-full rounded-lg px-4 py-2 text-left transition-colors"
								>{m.reader_export_epub()}</button
							>
							<button
								onclick={() => {
									api.export(bookId, "pdf");
									showExport = false;
								}}
								class="font-label text-label-md text-on-surface-variant hover:text-primary hover:bg-surface-container-low w-full rounded-lg px-4 py-2 text-left transition-colors"
								>{m.reader_export_pdf()}</button
							>
							<button
								onclick={() => {
									api.export(bookId, "markdown");
									showExport = false;
								}}
								class="font-label text-label-md text-on-surface-variant hover:text-primary hover:bg-surface-container-low w-full rounded-lg px-4 py-2 text-left transition-colors"
								>{m.reader_export_markdown()}</button
							>
						</div>
					{/if}
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
		</div>
	</header>

	{#if showFontPanel}
		<div
			class="bg-surface border-outline/10 fixed top-20 right-4 z-50 flex items-center gap-4 rounded-xl border px-4 py-3 shadow-lg"
		>
			<button
				onclick={() => {
					if (fontSize > 12) fontSize -= 2;
				}}
				class="p-1 hover:opacity-70"><Minus size={16} /></button
			>
			<span class="font-label text-label-sm min-w-[3ch] text-center">{fontSize}</span>
			<button
				onclick={() => {
					if (fontSize < 36) fontSize += 2;
				}}
				class="p-1 hover:opacity-70"><Plus size={16} /></button
			>
			<div class="bg-outline-variant/30 mx-2 h-6 w-px"></div>
			<button
				onclick={() => {
					if (lineHeight > 1.2) lineHeight -= 0.2;
				}}
				class="p-1 hover:opacity-70"><Minus size={16} /></button
			>
			<span class="font-label text-label-sm min-w-[3ch] text-center">{lineHeight.toFixed(1)}</span>
			<button
				onclick={() => {
					if (lineHeight < 3.0) lineHeight += 0.2;
				}}
				class="p-1 hover:opacity-70"><Plus size={16} /></button
			>
		</div>
	{/if}

	{#if showToc && tocEntries.length > 0}
		<div
			class="bg-surface border-outline/10 fixed top-16 right-0 z-40 h-[calc(100vh-4rem)] w-72 overflow-y-auto border-l shadow-lg"
		>
			<div class="p-6">
				<h3 class="font-display text-headline-sm text-primary mb-6">Contents</h3>
				<nav class="space-y-3">
					{#each tocEntries as entry (entry.id)}
						<button
							onclick={() => scrollToSection(entry.id)}
							class="font-label text-label-md text-on-surface-variant hover:text-secondary block w-full text-left transition-colors"
							>{entry.title}</button
						>
					{/each}
				</nav>
			</div>
		</div>
	{/if}

	<Popup show={!!popup} x={popup?.x ?? 0} y={popup?.y ?? 0} position="top">
		<div
			class="bg-surface border-outline/10 flex items-center gap-1 rounded-lg border px-2 py-1.5 shadow-lg"
		>
			{#each COLOR_NAMES as color (color)}
				<button
					onclick={() => createAnnotation(color)}
					class="h-6 w-6 rounded-full border-2 border-white/50 shadow-sm transition-transform hover:scale-110"
					style="background: {COLORS[color]};"
					title={color}
				></button>
			{/each}
		</div>
	</Popup>

	<Popup show={!!tooltip} position="center">
		{@const ann = tooltip!.annotation}
		<div class="bg-surface border-outline/10 w-80 rounded-xl border p-5 shadow-2xl">
			<div class="mb-4 flex items-start justify-between">
				<div class="flex items-center gap-2">
					<div
						class="h-4 w-4 rounded-full"
						style="background: {COLORS[ann.color ?? 'yellow']};"
					></div>
					<span class="font-label text-label-sm text-on-surface-variant"
						>{m.reader_annotation_color()}</span
					>
				</div>
				<button
					onclick={() => (tooltip = null)}
					class="text-on-surface-variant/50 hover:text-on-surface-variant p-1"
				>
					<X size={16} />
				</button>
			</div>
			{#if editNoteId === ann.id}
				<textarea
					bind:value={noteDraft}
					class="bg-surface-container-low border-outline/10 font-body text-body-md text-primary mb-3 h-24 w-full resize-none rounded-xl border p-3 focus:outline-none"
					placeholder={m.reader_add_note()}></textarea>
				<div class="flex gap-2">
					<button
						onclick={() => (editNoteId = null)}
						class="font-label text-label-sm text-on-surface-variant px-3 py-1.5 transition-colors"
						>{m.book_detail_cancel()}</button
					>
					<button onclick={() => saveNote(ann.id)} class="btn-primary text-label-sm px-3 py-1.5"
						>{m.reader_save_note()}</button
					>
				</div>
			{:else}
				{#if ann.note}
					<div class="bg-surface-container-low mb-3 flex items-start gap-2 rounded-lg p-3">
						<MessageSquareText size={14} class="text-on-surface-variant/50 mt-0.5 shrink-0" />
						<p class="font-body text-body-md text-primary">{ann.note}</p>
					</div>
				{:else}
					<p class="font-body text-body-md text-on-surface-variant mb-3 italic">
						{m.reader_add_note()}
					</p>
				{/if}
				<div class="flex gap-2">
					<button
						onclick={() => startEditNote(ann)}
						class="font-label text-label-sm text-secondary hover:text-secondary/80 transition-colors"
						>{m.reader_add_note()}</button
					>
					<button
						onclick={() => deleteAnnotation(ann.id)}
						disabled={deleting === ann.id}
						class="font-label text-label-sm text-error hover:text-error/80 ml-auto transition-colors disabled:opacity-50"
					>
						{deleting === ann.id ? "..." : m.reader_annotation_delete()}
					</button>
				</div>
			{/if}
		</div>
	</Popup>

	<main
		class="px-margin-mobile min-h-screen pt-32 pb-40 transition-colors duration-300 md:px-0"
		style="max-width: 800px; margin: 0 auto;"
	>
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
							<ChevronLeft size={14} />{m.reader_prev()}</button
						>
						<span class="font-label text-label-md text-on-surface-variant/60 tracking-widest"
							>{comicPage} / {comicPages.length}</span
						>
						<button
							onclick={nextPage}
							disabled={comicPage >= comicPages.length}
							class="font-label text-label-sm text-on-surface-variant hover:text-primary flex items-center gap-1 p-2 transition-colors disabled:cursor-not-allowed disabled:opacity-30"
						>
							{m.reader_next()}<ChevronRight size={14} /></button
						>
					</div>
				</div>
			{/if}
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
				{#each bookData.book.spine as section, sectionIdx (section.id)}
					<div data-section-id={section.id}>
						{#if section.title}
							<h2 class="font-display text-headline-md text-primary mb-12 text-center">
								{section.title}
							</h2>
						{/if}
						{#each section.blocks as block, blockIdx (blockIdx)}
							{@const b = block as Record<string, unknown>}
							{@const blockText = getBlockText(b)}
							{@const blockAnnotations = getBlockAnnotations(sectionIdx, blockIdx, section.id)}
							{#if "Paragraph" in b}
								<p
									data-block-index={blockIdx}
									class={[
										"font-body mb-8 leading-relaxed transition-colors",
										themeContentStyles[theme]
									]}
									onclick={() => {
										tooltip = null;
									}}
								>
									{#each splitTextAtAnnotations(blockText, blockAnnotations) as seg (seg.annotationId ?? seg.text.slice(0, 20))}
										{#if seg.annotationId}
											<mark
												data-annotation-id={seg.annotationId}
												onclick={(e) => {
													e.stopPropagation();
													handleAnnotationClick(
														annotations.find((a) => a.id === seg.annotationId)!
													);
												}}
												class="cursor-pointer rounded-sm"
												style="background: {COLORS[seg.color ?? 'yellow']};"
											>
												{seg.text}</mark
											>
										{:else}
											{seg.text}
										{/if}
									{/each}
								</p>
							{:else if "Heading" in b}
								<h3
									data-block-index={blockIdx}
									class={[
										"font-display text-headline-sm mt-12 mb-6 transition-colors",
										theme === "light" ? "text-primary" : "text-inherit"
									]}
								>
									{#each splitTextAtAnnotations(blockText, blockAnnotations) as seg (seg.annotationId ?? seg.text.slice(0, 20))}
										{#if seg.annotationId}
											<mark
												data-annotation-id={seg.annotationId}
												style="background: {COLORS[seg.color ?? 'yellow']};"
												onclick={(e) => {
													e.stopPropagation();
													handleAnnotationClick(
														annotations.find((a) => a.id === seg.annotationId)!
													);
												}}
												class="cursor-pointer rounded-sm">{seg.text}</mark
											>
										{:else}{seg.text}{/if}
									{/each}
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
					</div>
				{/each}
			</article>
		{/if}
	</main>

	<footer
		class="px-margin-mobile bg-surface/80 fixed bottom-0 left-0 flex w-full items-center justify-between py-6 backdrop-blur-sm"
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
</div>
