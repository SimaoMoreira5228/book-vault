<script lang="ts">
	import * as m from "$lib/paraglide/messages";
	import { api, apiBase } from "$lib/api/client.svelte";
	import { page } from "$app/state";
	import type { Block, BookResponse } from "$lib/api/generated";
	import type { Annotation, ReaderTheme } from "$lib/ir/renderer";
	import { ContextMenu } from "bits-ui";
	import { getBlockText, COLOR_NAMES, COLORS } from "$lib/ir/renderer";
	import ReaderLayout from "$lib/components/reader/ReaderLayout.svelte";
	import BlockRenderer from "$lib/components/reader/BlockRenderer.svelte";
	import AnnotationPopup from "$lib/components/reader/AnnotationPopup.svelte";
	import AnnotationTooltip from "$lib/components/reader/AnnotationTooltip.svelte";
	import TocPanel from "$lib/components/reader/TocPanel.svelte";
	import FontPanel from "$lib/components/reader/FontPanel.svelte";
	import ComicViewer from "$lib/components/reader/ComicViewer.svelte";
	import ExportMenu from "$lib/components/reader/ExportMenu.svelte";
	import LookupPopover from "$lib/components/reader/LookupPopover.svelte";
	import BookOpen from "@lucide/svelte/icons/book-open";
	import Bookmark from "@lucide/svelte/icons/bookmark";
	import Download from "@lucide/svelte/icons/download";

	const bookId = $derived(page.params.id ?? "");

	let meta = $state<BookResponse | null>(null);
	let loadError = $state("");
	let spine = $state<Array<{ id: string; title: string | null }>>([]);
	let loadedBlocks = $state<Record<string, Block[]>>({});
	let loading = $state(true);
	let progress = $state(0);
	let pdfMode = $state<"text" | "pdf">("text");

	let comicPages = $state<Array<{ page: number; asset_id: string; mime_type: string }>>([]);
	let comicCurrentPage = $state(1);

	let theme = $state<ReaderTheme>("light");
	let fontSize = $state(18);
	let lineHeight = $state(1.8);
	let showToc = $state(false);
	let showFontPanel = $state(false);
	let prefsLoaded = $state(false);

	let lookupText = $state("");
	let lookupContext = $state("");
	let lookupLanguage = $state("en");
	let lookupDefinitionLanguage = $state("");
	let lookupPos = $state({ x: 0, y: 0 });
	let showLookup = $state(false);

	let annotations = $state<Annotation[]>([]);
	let popup = $state<{
		x: number;
		y: number;
		text: string;
		blockIndex: number;
		startOffset: number;
		endOffset: number;
	} | null>(null);
	let popupSectionId = $state<string | null>(null);
	let tooltipAnn = $state<Annotation | null>(null);
	let bookmarks = $state<Array<{ id: string; section_id: string }>>([]);

	let contextMenuOpen = $state(false);
	let contextType = $state<"text" | "annotation" | "code" | "image" | "none">("none");
	let contextAnnId = $state<string | null>(null);
	let contextCodeText = $state("");
	let contextImageUrl = $state("");
	let anchorEl = $state<HTMLSpanElement>();

	let saveTimer: ReturnType<typeof setInterval> | undefined;

	const formatsWithDownload = $derived(["pdf", "mobi_raw", "epub"]);
	const showDownload = $derived(meta ? formatsWithDownload.includes(meta.format) : false);

	$effect(() => {
		if (bookId) {
			loadBook();
			loadPreferences();
		}
		return () => {
			if (saveTimer) clearInterval(saveTimer);
			if (prefSaveTimer) clearTimeout(prefSaveTimer);
		};
	});

	let prefSaveTimer: ReturnType<typeof setTimeout> | undefined;
	$effect(() => {
		if (!prefsLoaded) return;
		if (prefSaveTimer) clearTimeout(prefSaveTimer);
		prefSaveTimer = setTimeout(async () => {
			const r = await api.auth.updatePreferences({ reader: { theme, fontSize, lineHeight } });
			if (r.isErr()) console.warn("Failed to save prefs:", r.error.message);
		}, 2000);
		return () => {
			if (prefSaveTimer) clearTimeout(prefSaveTimer);
		};
	});

	$effect(() => {
		saveTimer = setInterval(() => saveProgressNow(), 15000);
		return () => clearInterval(saveTimer);
	});

	let restoreAfterLoad = $state<{ sectionId: string; percentage: number } | null>(null);

	async function loadBook() {
		loading = true;
		loadError = "";
		const [metaResult, spineResult, progressResult] = await Promise.all([
			api.books.get(bookId),
			api.readSpine(bookId),
			api.progress.get(bookId)
		]);
		if (metaResult.isErr()) {
			loadError = metaResult.error.message;
			loading = false;
			return;
		}
		meta = metaResult.value;
		lookupLanguage = meta.language ?? "en";
		lookupDefinitionLanguage = meta.language ?? "";
		if (spineResult.isErr()) {
			loadError = spineResult.error.message;
			loading = false;
			return;
		}
		if (spineResult.isOk()) {
			spine = spineResult.value.map((s) => ({ id: s.id, title: s.title }));
			if (progressResult.isOk() && progressResult.value) {
				restoreAfterLoad = {
					sectionId: progressResult.value.section_id,
					percentage: progressResult.value.percentage
				};
			}
			loadSectionBlocks(0);
		}
		if (metaResult.isOk() && metaResult.value.format === "cbz") loadComicPages();
		loading = false;
		await Promise.all([loadAnnotations(), loadBookmarks()]);
	}

	async function loadSectionBlocks(index: number) {
		const section = spine[index];
		if (!section || loadedBlocks[section.id]) return;
		const r = await api.readSection(bookId, section.id);
		if (r.isOk()) {
			loadedBlocks[section.id] = r.value as unknown as Block[];
		} else {
			console.warn("Failed to load section", section.id, r.error);
		}
	}

	async function loadPreferences() {
		const r = await api.auth.getPreferences();
		if (r.isOk() && r.value.reader) {
			const p = r.value.reader as Record<string, unknown>;
			if (typeof p.theme === "string") theme = p.theme as ReaderTheme;
			if (typeof p.fontSize === "number") fontSize = p.fontSize;
			if (typeof p.lineHeight === "number") lineHeight = p.lineHeight;
		}
		prefsLoaded = true;
	}

	async function loadAnnotations() {
		const r = await api.annotations.list(bookId);
		if (r.isOk()) annotations = r.value as unknown as Annotation[];
	}

	async function loadBookmarks() {
		const r = await api.bookmarks.list(bookId);
		if (r.isOk()) bookmarks = r.value as unknown as Array<{ id: string; section_id: string }>;
	}

	async function loadComicPages() {
		const r = await api.comic.pages(bookId);
		if (r.isOk()) comicPages = r.value;
	}

	let scrolledToRestore = false;
	let scrollRestoreTimer: ReturnType<typeof setTimeout> | undefined;

	$effect(() => {
		if (spine.length === 0) return;
		const hash = page.url.hash;
		if (hash && hash.startsWith("#section-")) {
			const sectionId = hash.replace("#section-", "");
			setTimeout(() => {
				const el = document.getElementById(`section-${sectionId}`);
				if (el) {
					el.scrollIntoView({ behavior: "instant" });
					scrolledToRestore = true;
				}
			}, 300);
			return;
		}
		if (!restoreAfterLoad || scrolledToRestore) return;
		scrollRestoreTimer = setTimeout(() => {
			const idx = spine.findIndex((s) => s.id === restoreAfterLoad!.sectionId);
			if (idx >= 0) {
				const el = document.getElementById(`section-${restoreAfterLoad!.sectionId}`);
				if (el) {
					el.scrollIntoView({ behavior: "instant" });
					scrolledToRestore = true;
				}
			}
		}, 300);
		return () => {
			if (scrollRestoreTimer) clearTimeout(scrollRestoreTimer);
		};
	});

	async function saveProgressNow() {
		const visible = findVisibleSection();
		if (!visible) return;
		const maxScroll = document.documentElement.scrollHeight - window.innerHeight;
		const pct = maxScroll > 0 ? Math.round((window.scrollY / maxScroll) * 100) : 0;
		const r = await api.progress.save(bookId, {
			section_id: visible,
			block_index: 0,
			char_offset: 0,
			percentage: pct
		});
		if (r.isErr()) console.warn("Failed to save progress:", r.error.message);
	}

	function findVisibleSection(): string | null {
		const viewportMid = window.scrollY + window.innerHeight / 2;
		let bestId: string | null = null;
		let bestDist = Infinity;
		for (const s of spine) {
			const el = document.getElementById(`section-${s.id}`);
			if (!el) continue;
			const top = el.offsetTop;
			const bottom = top + el.offsetHeight;
			const mid = (top + bottom) / 2;
			const dist = Math.abs(mid - viewportMid);
			if (dist < bestDist) {
				bestDist = dist;
				bestId = s.id;
			}
		}
		return bestId;
	}

	async function toggleBookmark() {
		const sid = findVisibleSection();
		if (!sid) return;
		const existing = bookmarks.find((b) => b.section_id === sid);
		if (existing) {
			await api.bookmarks.delete(existing.id);
			bookmarks = bookmarks.filter((b) => b.section_id !== sid);
		} else {
			const r = await api.bookmarks.create({
				book_id: bookId,
				section_id: sid,
				block_index: 0
			});
			if (r.isOk())
				bookmarks = [
					...bookmarks,
					{ id: (r.value as unknown as { id: string }).id, section_id: sid }
				];
		}
	}

	function onScroll() {
		const maxScroll = document.documentElement.scrollHeight - window.innerHeight;
		progress = maxScroll > 0 ? Math.min(100, Math.round((window.scrollY / maxScroll) * 100)) : 0;
	}

	function onTextSelect() {
		const sel = window.getSelection();
		if (sel && sel.rangeCount > 0) {
			const container = sel.getRangeAt(0).startContainer;
			if (container instanceof Element && !container.closest("[data-book-content]")) {
				showLookup = false;
				popup = null;
				return;
			}
			if (container?.parentElement && !container.parentElement.closest("[data-book-content]")) {
				showLookup = false;
				popup = null;
				return;
			}
		}
		showLookup = false;
		tooltipAnn = null;
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

		let sectionNode: Node | null = range.startContainer;
		while (
			sectionNode &&
			(!(sectionNode as HTMLElement).dataset || !(sectionNode as HTMLElement).dataset.sectionId)
		) {
			sectionNode = sectionNode.parentElement;
		}
		popupSectionId = (sectionNode as HTMLElement)?.dataset?.sectionId ?? findVisibleSection();

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

	function handleLookup() {
		if (!popup) return;
		tooltipAnn = null;
		const fullText = popup.text;
		const words = fullText.split(/\s+/).filter(Boolean);
		lookupText = words.length === 1 ? words[0] : fullText;
		lookupContext = findContextSentence();
		lookupPos = { x: popup.x, y: popup.y };
		showLookup = true;
		popup = null;
		popupSectionId = null;
		window.getSelection()?.removeAllRanges();
	}

	function findContextSentence(): string {
		const sel = window.getSelection();
		if (!sel || !sel.rangeCount) return "";
		let node: Node | null = sel.getRangeAt(0).startContainer;
		while (
			node &&
			!(
				node instanceof HTMLParagraphElement ||
				node instanceof HTMLHeadingElement ||
				node instanceof HTMLQuoteElement
			)
		) {
			node = node.parentElement;
		}
		if (!node) {
			const blockEl = popup
				? document.querySelector(`[data-block-index="${popup.blockIndex}"]`)
				: null;
			return blockEl?.textContent ?? "";
		}
		return node.textContent ?? "";
	}

	async function createAnnotation(color: string) {
		if (!popup) return;
		const sid = popupSectionId ?? findVisibleSection();
		if (!sid) {
			console.warn("annotate: no visible section");
			return;
		}
		const blocks = loadedBlocks[sid] ?? [];
		const block = blocks[popup.blockIndex] as Record<string, unknown> | undefined;
		if (!block) {
			console.warn("annotate: block not found", {
				sid,
				blockIndex: popup.blockIndex,
				blocksLen: blocks.length
			});
			return;
		}
		const blockText = getBlockText(block);
		const startOffset = Math.max(0, Math.min(popup.startOffset, blockText.length));
		const endOffset = Math.max(startOffset, Math.min(popup.endOffset, blockText.length));
		if (startOffset >= endOffset) {
			console.warn("annotate: invalid offsets", {
				startOffset,
				endOffset,
				blockTextLen: blockText.length
			});
			popup = null;
			return;
		}

		const r = await api.annotations.create(bookId, {
			section_id: sid,
			block_index: popup.blockIndex,
			start_offset: startOffset,
			end_offset: endOffset,
			color
		});
		if (r.isOk()) {
			const fetched = await api.annotations.list(bookId);
			if (fetched.isOk()) annotations = fetched.value as unknown as Annotation[];
		} else {
			console.warn("annotate: api failed", r.error);
		}
		popupSectionId = null;
		popup = null;
		window.getSelection()?.removeAllRanges();
	}

	function handleAnnotationClick(ann: Annotation) {
		popup = null;
		window.getSelection()?.removeAllRanges();
		tooltipAnn = ann;
	}

	function observeSection(node: HTMLElement, sectionIdx: number) {
		const observer = new IntersectionObserver(
			(entries) => {
				for (const entry of entries) {
					if (entry.isIntersecting) {
						loadSectionBlocks(sectionIdx);
						observer.unobserve(node);
					}
				}
			},
			{ rootMargin: "400px" }
		);
		observer.observe(node);
		return {
			destroy() {
				observer.disconnect();
			}
		};
	}

	function handleContextMenu(e: MouseEvent) {
		const target = e.target as HTMLElement;

		contextAnnId = null;
		contextCodeText = "";
		contextImageUrl = "";

		const annEl = target.closest("[data-annotation-id]");
		if (annEl instanceof HTMLElement && annEl.dataset.annotationId) {
			contextType = "annotation";
			contextAnnId = annEl.dataset.annotationId;
		} else if (target.closest("pre")) {
			const pre = target.closest("pre")!;
			contextType = "code";
			contextCodeText = pre.textContent ?? "";
		} else if (target.closest("img")) {
			const img = target.closest("img")! as HTMLImageElement;
			contextType = "image";
			contextImageUrl = img.src;
		} else if (window.getSelection() && !window.getSelection()?.isCollapsed) {
			contextType = "text";
		} else {
			contextType = "none";
			return;
		}

		e.preventDefault();
		anchorEl?.remove();
		const span = document.createElement("span");
		span.style.cssText = `position:fixed;left:${e.clientX}px;top:${e.clientY}px;width:1px;height:1px;pointer-events:none;`;
		document.body.appendChild(span);
		anchorEl = span;
		contextMenuOpen = true;
	}

	function closeContextMenu() {
		contextMenuOpen = false;
		anchorEl?.remove();
		anchorEl = undefined;
	}

	function handleContextCopy() {
		const sel = window.getSelection();
		if (sel && !sel.isCollapsed) navigator.clipboard.writeText(sel.toString());
		closeContextMenu();
	}

	function handleContextCopyCode() {
		if (contextCodeText) navigator.clipboard.writeText(contextCodeText);
		closeContextMenu();
	}

	function handleContextViewImage() {
		if (contextImageUrl) window.open(contextImageUrl, "_blank");
		closeContextMenu();
	}

	async function handleContextChangeColor(color: string) {
		if (!contextAnnId) return;
		const r = await api.annotations.update(contextAnnId, { color });
		if (r.isOk()) {
			const fetched = await api.annotations.list(bookId);
			if (fetched.isOk()) annotations = fetched.value as unknown as Annotation[];
		}
		closeContextMenu();
	}

	async function handleContextRemoveHighlight() {
		if (!contextAnnId) return;
		const r = await api.annotations.delete(contextAnnId);
		if (r.isOk()) annotations = annotations.filter((a) => a.id !== contextAnnId);
		closeContextMenu();
	}

	function handleContextEditNote() {
		if (!contextAnnId) return;
		const ann = annotations.find((a) => a.id === contextAnnId);
		if (ann) tooltipAnn = ann;
		closeContextMenu();
	}

	const rawUrl = $derived(bookId ? `${apiBase}/api/v1/books/${bookId}/raw` : "");
</script>

<svelte:head
	><title>{meta?.title ?? spine[0]?.title ?? m.reader_loading()} — {m.app_name()}</title
	></svelte:head
>

<svelte:window onscroll={onScroll} onselectionchange={onTextSelect} />

<ReaderLayout
	bind:theme
	{progress}
	bind:showToc
	bind:showFontPanel
	title={meta?.title ?? spine[0]?.title ?? m.reader_loading()}
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
			<ExportMenu {bookId} />
		{/if}
		<button
			onclick={toggleBookmark}
			class="p-2 transition-transform duration-200 hover:opacity-80 active:scale-95"
			title={(() => {
				const sid = findVisibleSection();
				return sid && bookmarks.find((b) => b.section_id === sid)
					? m.reader_bookmark_remove()
					: m.reader_bookmark_add();
			})()}
		>
			<Bookmark
				size={20}
				class={(() => {
					const sid = findVisibleSection();
					return sid && bookmarks.find((b) => b.section_id === sid)
						? "text-secondary"
						: "text-on-surface-variant";
				})()}
			/>
		</button>
		<div class="mr-2 flex items-center gap-1">
			<select
				bind:value={lookupLanguage}
				class="bg-surface-container-high font-label text-label-sm text-on-surface-variant rounded-lg border-0 px-2 py-1.5 outline-none"
				aria-label="Lookup language"
			>
				<option value="en">EN</option>
				<option value="pt">PT</option>
				<option value="es">ES</option>
				<option value="fr">FR</option>
				<option value="de">DE</option>
				<option value="it">IT</option>
			</select>
			<select
				bind:value={lookupDefinitionLanguage}
				class="bg-surface-container-high font-label text-label-sm text-on-surface-variant rounded-lg border-0 px-2 py-1.5 outline-none"
				aria-label="Definition language"
			>
				<option value="">{m.vocab_definition_language_auto()}</option>
				<option value="en">EN</option>
				<option value="pt">PT</option>
				<option value="es">ES</option>
				<option value="fr">FR</option>
				<option value="de">DE</option>
				<option value="it">IT</option>
			</select>
		</div>
	{/snippet}

	<TocPanel
		sections={spine}
		bind:show={showToc}
		onNavigate={(sectionId) => {
			const el = document.getElementById(`section-${sectionId}`);
			if (el) {
				el.scrollIntoView({ behavior: "smooth" });
				showToc = false;
			}
		}}
	/>
	{#if showFontPanel}
		<FontPanel bind:fontSize bind:lineHeight />
	{/if}

	{#if loading}
		<div class="flex items-center justify-center py-32">
			<div
				class="border-secondary h-8 w-8 animate-spin rounded-full border-2 border-t-transparent"
			></div>
		</div>
	{:else if loadError}
		<div class="flex flex-col items-center justify-center gap-4 py-32">
			<p class="font-body text-body-md text-error">{loadError}</p>
			<button
				onclick={loadBook}
				class="font-label text-label-md text-secondary underline transition-colors hover:opacity-80"
			>
				Try again
			</button>
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
				<a href={rawUrl} class="text-secondary underline" target="_blank" rel="external"
					>Download instead</a
				>.
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
	{:else if spine.length > 0}
		<article
			data-book-content
			class="space-y-16"
			style="font-size: {fontSize}px; line-height: {lineHeight};"
			oncontextmenu={handleContextMenu}
		>
			{#each spine as section, sectionIdx (section.id)}
				<section
					id="section-{section.id}"
					data-section-id={section.id}
					use:observeSection={sectionIdx}
				>
					{#if section.title}
						<h2 class="font-display text-headline-md text-primary mb-12 text-center">
							{section.title}
						</h2>
					{/if}
					{#if loadedBlocks[section.id]}
						{@const blocks = loadedBlocks[section.id]!}
						{#each blocks as block, blockIdx (blockIdx)}
							<BlockRenderer
								{block}
								{blockIdx}
								sectionId={section.id}
								{bookId}
								{annotations}
								{theme}
								onAnnotationClick={handleAnnotationClick}
								{onTextSelect}
							/>
						{/each}
					{:else}
						<div class="bg-surface-container/50 flex items-center justify-center rounded-xl py-16">
							<div
								class="border-secondary h-6 w-6 animate-spin rounded-full border-2 border-t-transparent"
							></div>
						</div>
					{/if}
				</section>
			{/each}
		</article>
	{/if}
</ReaderLayout>

<ContextMenu.Root
	open={contextMenuOpen}
	onOpenChange={(o) => {
		if (!o) closeContextMenu();
	}}
>
	<ContextMenu.Portal>
		<ContextMenu.Content
			customAnchor={anchorEl}
			class="bg-surface border-outline/10 data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 z-50 min-w-[180px] rounded-xl border p-1.5 shadow-lg"
			sideOffset={4}
		>
			{#if contextType === "text"}
				<ContextMenu.Sub>
					<ContextMenu.SubTrigger
						class="font-label text-label-md data-highlighted:bg-surface-container-low flex w-full cursor-pointer items-center gap-2 rounded-lg px-3 py-2 text-left transition-colors focus-visible:outline-none"
					>
						{m.reader_annotation_color()}
					</ContextMenu.SubTrigger>
					<ContextMenu.SubContent
						class="bg-surface border-outline/10 z-50 ml-1 rounded-xl border p-1.5 shadow-lg"
					>
						{#each COLOR_NAMES as color (color)}
							<ContextMenu.Item
								onclick={() => {
									createAnnotation(color);
									closeContextMenu();
								}}
								class="font-label text-label-md data-highlighted:bg-surface-container-low flex w-full cursor-pointer items-center gap-2 rounded-lg px-3 py-2 text-left transition-colors focus-visible:outline-none"
							>
								<span class="inline-block h-3 w-3 rounded-full" style="background: {COLORS[color]};"
								></span>
								<span class="capitalize">{color}</span>
							</ContextMenu.Item>
						{/each}
					</ContextMenu.SubContent>
				</ContextMenu.Sub>
				<ContextMenu.Item
					onclick={handleContextCopy}
					class="font-label text-label-md data-highlighted:bg-surface-container-low flex w-full cursor-pointer items-center gap-2 rounded-lg px-3 py-2 text-left transition-colors focus-visible:outline-none"
					>Copy</ContextMenu.Item
				>
			{:else if contextType === "annotation"}
				<ContextMenu.Sub>
					<ContextMenu.SubTrigger
						class="font-label text-label-md data-highlighted:bg-surface-container-low flex w-full cursor-pointer items-center gap-2 rounded-lg px-3 py-2 text-left transition-colors focus-visible:outline-none"
					>
						{m.reader_annotation_color()}
					</ContextMenu.SubTrigger>
					<ContextMenu.SubContent
						class="bg-surface border-outline/10 z-50 ml-1 rounded-xl border p-1.5 shadow-lg"
					>
						{#each COLOR_NAMES as color (color)}
							<ContextMenu.Item
								onclick={() => handleContextChangeColor(color)}
								class="font-label text-label-md data-highlighted:bg-surface-container-low flex w-full cursor-pointer items-center gap-2 rounded-lg px-3 py-2 text-left transition-colors focus-visible:outline-none"
							>
								<span class="inline-block h-3 w-3 rounded-full" style="background: {COLORS[color]};"
								></span>
								<span class="capitalize">{color}</span>
							</ContextMenu.Item>
						{/each}
					</ContextMenu.SubContent>
				</ContextMenu.Sub>
				<ContextMenu.Item
					onclick={handleContextEditNote}
					class="font-label text-label-md data-highlighted:bg-surface-container-low flex w-full cursor-pointer items-center gap-2 rounded-lg px-3 py-2 text-left transition-colors focus-visible:outline-none"
					>{m.reader_add_note()}</ContextMenu.Item
				>
				<ContextMenu.Separator class="bg-outline-variant/30 -mx-1 my-1 block h-px" />
				<ContextMenu.Item
					onclick={handleContextRemoveHighlight}
					class="font-label text-label-md data-highlighted:bg-surface-container-low text-error flex w-full cursor-pointer items-center gap-2 rounded-lg px-3 py-2 text-left transition-colors focus-visible:outline-none"
					>{m.reader_annotation_delete()}</ContextMenu.Item
				>
			{:else if contextType === "code"}
				<ContextMenu.Item
					onclick={handleContextCopyCode}
					class="font-label text-label-md data-highlighted:bg-surface-container-low flex w-full cursor-pointer items-center gap-2 rounded-lg px-3 py-2 text-left transition-colors focus-visible:outline-none"
					>Copy Code</ContextMenu.Item
				>
			{:else if contextType === "image"}
				<ContextMenu.Item
					onclick={handleContextViewImage}
					class="font-label text-label-md data-highlighted:bg-surface-container-low flex w-full cursor-pointer items-center gap-2 rounded-lg px-3 py-2 text-left transition-colors focus-visible:outline-none"
					>View Full Size</ContextMenu.Item
				>
			{/if}
		</ContextMenu.Content>
	</ContextMenu.Portal>
</ContextMenu.Root>

<AnnotationPopup
	show={popup !== null}
	x={popup?.x ?? 0}
	y={popup?.y ?? 0}
	onCreateColor={createAnnotation}
	onLookup={handleLookup}
/>
<LookupPopover
	bind:show={showLookup}
	x={lookupPos.x}
	y={lookupPos.y}
	text={lookupText}
	context={lookupContext}
	language={lookupLanguage}
	definitionLanguage={lookupDefinitionLanguage}
/>
<AnnotationTooltip bind:annotation={tooltipAnn} onUpdate={loadAnnotations} />
