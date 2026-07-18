<script lang="ts">
	import * as m from "$lib/paraglide/messages";
	import { api, type SpineItem } from "$lib/api/client.svelte";
	import { authState } from "$lib/api/client.svelte";
	import { goto } from "$app/navigation";
	import { resolve } from "$app/paths";
	import { page } from "$app/state";
	import type { Block, BookResponse } from "$lib/api/generated";
	import EditableBlock from "$lib/components/studio/EditableBlock.svelte";
	import SectionPanel from "$lib/components/studio/SectionPanel.svelte";
	import StudioLayout from "$lib/components/studio/StudioLayout.svelte";
	import { createEmptyBlock, getBlockType } from "../../../../utils/blockEditor.js";

	const bookId = $derived(page.params.id ?? "");

	let book = $state<BookResponse | null>(null);
	let spine = $state<SpineItem[]>([]);
	let activeSectionId = $state<string | null>(null);
	let blocks = $state<Block[]>([]);
	let loading = $state(true);
	let saving = $state(false);
	let lastSaved = $state("");
	let error = $state("");
	let saveTimer: ReturnType<typeof setTimeout> | undefined;

	let dirty = $state(false);

	$effect(() => {
		if (!authState.restoring && !authState.isAuthenticated) {
			goto(resolve("/login"));
			return;
		}
		if (bookId && authState.isAuthenticated) {
			loadBook();
		}
	});

	$effect(() => {
		return () => {
			if (saveTimer) clearTimeout(saveTimer);
		};
	});

	async function loadBook() {
		loading = true;
		error = "";

		const [metaResult, spineResult] = await Promise.all([
			api.books.get(bookId),
			api.readSpine(bookId)
		]);

		if (metaResult.isErr()) {
			error = metaResult.error.message;
			loading = false;
			return;
		}
		if (spineResult.isErr()) {
			error = spineResult.error.message;
			loading = false;
			return;
		}

		book = metaResult.value;
		spine = spineResult.value;

		if (spine.length > 0) {
			selectSection(spine[0].id);
		}
		loading = false;
	}

	async function selectSection(sectionId: string) {
		activeSectionId = sectionId;
		const r = await api.readSection(bookId, sectionId);
		if (r.isOk()) {
			blocks = r.value as unknown as Block[];
			dirty = false;
		}
	}

	async function handleSave() {
		if (!activeSectionId || !dirty) return;
		saving = true;
		const r = await api.studio.saveSection(bookId, activeSectionId, blocks);
		if (r.isOk()) {
			dirty = false;
			lastSaved = new Date().toLocaleTimeString();
		} else {
			error = r.error.message;
		}
		saving = false;
	}

	function debouncedSave() {
		if (saveTimer) clearTimeout(saveTimer);
		saveTimer = setTimeout(() => {
			handleSave();
		}, 2000);
	}

	function updateBlock(index: number, newBlock: Block) {
		blocks = blocks.map((b, i) => (i === index ? newBlock : b));
		dirty = true;
		debouncedSave();
	}

	function deleteBlock(index: number) {
		if (blocks.length <= 1) return;
		blocks = blocks.filter((_, i) => i !== index);
		dirty = true;
		debouncedSave();
	}

	function splitBlock(index: number, beforeText: string, afterText: string) {
		const current = blocks[index];
		const type = getBlockType(current);

		const beforeBlock = createBlockWithText(type, beforeText);
		const afterBlock = createBlockWithText(type, afterText);

		const newBlocks = [...blocks];
		newBlocks.splice(index, 1, beforeBlock, afterBlock);
		blocks = newBlocks;
		dirty = true;
		debouncedSave();
	}

	function createBlockWithText(type: string, text: string): Block {
		const block = createEmptyBlock(type === "HorizontalRule" ? "Paragraph" : type);
		if (typeof block === "string") return block;
		const b = block as Record<string, unknown>;

		if (type === "Paragraph" || type === "HorizontalRule") {
			const spans = text ? [{ text, marks: 0, href: null }] : [];
			b.Paragraph = spans;
		} else if (type === "Heading") {
			b.Heading = { level: 1, spans: text ? [{ text, marks: 0, href: null }] : [] };
		} else if (type === "CodeBlock") {
			b.CodeBlock = { language: null, content: text };
		} else if (type === "BlockQuote") {
			b.BlockQuote = [
				{
					Paragraph: text ? [{ text, marks: 0, href: null }] : []
				}
			];
		} else if (type === "UnorderedList") {
			b.UnorderedList = [
				[
					{
						Paragraph: text ? [{ text, marks: 0, href: null }] : []
					}
				]
			];
		} else if (type === "OrderedList") {
			b.OrderedList = [
				[
					{
						Paragraph: text ? [{ text, marks: 0, href: null }] : []
					}
				]
			];
		}

		return b as Block;
	}

	function mergeUp(index: number) {
		if (index <= 0) return;
		const currentText = blockToSimpleText(blocks[index]);
		const prevBlock = blocks[index - 1];
		const prevText = blockToSimpleText(prevBlock);
		const merged = createBlockWithText(getBlockType(prevBlock), prevText + currentText);
		const newBlocks = blocks.map((b, i) => (i === index - 1 ? merged : b));
		newBlocks.splice(index, 1);
		blocks = newBlocks;
		dirty = true;
		debouncedSave();
	}

	function blockToSimpleText(block: Block): string {
		if (typeof block === "string") return "";
		const b = block as Record<string, unknown>;
		const entry = Object.entries(b)[0];
		if (!entry) return "";
		const [, value] = entry;

		if (
			Array.isArray(value) &&
			value.length > 0 &&
			typeof value[0] === "object" &&
			"text" in value[0]
		) {
			return (value as Array<{ text: string }>).map((s) => s.text).join("");
		}
		if (typeof value === "object" && value !== null && "spans" in value) {
			return (value as { spans: Array<{ text: string }> }).spans.map((s) => s.text).join("");
		}
		if (typeof value === "object" && value !== null && "content" in value) {
			return (value as { content: string }).content;
		}
		return "";
	}

	function addBlockAfter(index: number, newBlock: Block) {
		const newBlocks = [...blocks];
		newBlocks.splice(index + 1, 0, newBlock);
		blocks = newBlocks;
		dirty = true;
		debouncedSave();
	}

	async function addSection() {
		const sectionId = crypto.randomUUID();
		const sections = [
			...spine,
			{ id: sectionId, title: m.studio_untitled_section(), sequence_index: spine.length }
		];
		spine = sections;

		const emptyBlock: Block = { Paragraph: [{ text: "", marks: 0, href: null }] };
		await api.studio.saveSection(bookId, sectionId, [emptyBlock]);

		activeSectionId = sectionId;
		blocks = [emptyBlock];
		dirty = false;
	}

	async function deleteSection(sectionId: string) {
		if (spine.length <= 1) return;
		spine = spine.filter((s) => s.id !== sectionId);
		if (activeSectionId === sectionId) {
			const next = spine[0];
			if (next) selectSection(next.id);
		}
	}

	async function renameSection(sectionId: string, title: string) {
		spine = spine.map((s) => (s.id === sectionId ? { ...s, title } : s));
	}

	function handleFormat(format: string) {
		const sel = window.getSelection();
		if (!sel || sel.isCollapsed) return;
		switch (format) {
			case "bold":
				document.execCommand("bold", false);
				break;
			case "italic":
				document.execCommand("italic", false);
				break;
			case "underline":
				document.execCommand("underline", false);
				break;
			case "strikethrough":
				document.execCommand("strikeThrough", false);
				break;
		}
		sel
			.getRangeAt(0)
			.startContainer.parentElement?.dispatchEvent(new Event("input", { bubbles: true }));
	}

	let toolbarX = $state(0);
	let toolbarY = $state(0);
	let showToolbar = $state(false);

	$effect(() => {
		function onSelectionChange() {
			const sel = window.getSelection();
			if (sel && !sel.isCollapsed && sel.rangeCount > 0) {
				const range = sel.getRangeAt(0);
				const container = range.startContainer;
				const block =
					container instanceof Element
						? container.closest("[data-editable-block]")
						: container.parentElement?.closest("[data-editable-block]");
				if (block) {
					const rect = range.getBoundingClientRect();
					toolbarX = rect.left + rect.width / 2;
					toolbarY = rect.top;
					showToolbar = true;
					return;
				}
			}
			showToolbar = false;
		}

		document.addEventListener("selectionchange", onSelectionChange);
		return () => document.removeEventListener("selectionchange", onSelectionChange);
	});
</script>

<svelte:head>
	<title>{book?.title ?? "Studio"} — Book Vault</title>
</svelte:head>

{#if loading}
	<div class="flex min-h-screen items-center justify-center">
		<div
			class="border-secondary h-8 w-8 animate-spin rounded-full border-2 border-t-transparent"
		></div>
	</div>
{:else if error}
	<div class="flex min-h-screen flex-col items-center justify-center gap-4">
		<p class="font-body text-body-md text-on-surface-variant">{error}</p>
		<a href={resolve("/studio")} class="font-label text-label-md text-secondary underline"
			>{m.nav_studio()}</a
		>
	</div>
{:else}
	<StudioLayout title={book?.title ?? ""} {saving} {lastSaved}>
		{#snippet sidebar()}
			<SectionPanel
				sections={spine}
				{activeSectionId}
				onSelectSection={selectSection}
				onAddSection={addSection}
				onDeleteSection={deleteSection}
				onRenameSection={renameSection}
			/>
		{/snippet}

		{#if activeSectionId && blocks}
			<div class="space-y-1" data-studio-editor>
				{#each blocks as block, blockIndex (blockIndex)}
					<EditableBlock
						{block}
						{blockIndex}
						totalBlocks={blocks.length}
						onUpdate={updateBlock}
						onDelete={deleteBlock}
						onSplit={splitBlock}
						onMergeUp={mergeUp}
						onAddAfter={addBlockAfter}
					/>
				{/each}

				<div class="pt-2">
					<button
						onclick={() => addBlockAfter(blocks.length - 1, createEmptyBlock("Paragraph"))}
						class="text-on-surface-variant/30 hover:text-secondary inline-flex items-center gap-1.5 rounded-lg px-3 py-2 text-sm transition-colors"
					>
						+ Add block
					</button>
				</div>
			</div>
		{/if}

		{#if showToolbar}
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<div
				class="bg-surface border-outline/10 fixed z-[100] flex -translate-x-1/2 -translate-y-full gap-0.5 rounded-xl border px-2 py-1.5 shadow-lg"
				style="left: {toolbarX}px; top: {toolbarY - 12}px;"
				onmouseup={(e) => e.stopPropagation()}
			>
				<button
					onclick={() => handleFormat("bold")}
					class="text-on-surface-variant hover:text-primary hover:bg-surface-container-low rounded-lg p-1.5 font-bold transition-all"
					title="Bold">B</button
				>
				<button
					onclick={() => handleFormat("italic")}
					class="text-on-surface-variant hover:text-primary hover:bg-surface-container-low rounded-lg p-1.5 italic transition-all"
					title="Italic">I</button
				>
				<button
					onclick={() => handleFormat("underline")}
					class="text-on-surface-variant hover:text-primary hover:bg-surface-container-low rounded-lg p-1.5 underline transition-all"
					title="Underline">U</button
				>
				<button
					onclick={() => handleFormat("strikethrough")}
					class="text-on-surface-variant hover:text-primary hover:bg-surface-container-low rounded-lg p-1.5 line-through transition-all"
					title="Strikethrough">S</button
				>
			</div>
		{/if}
	</StudioLayout>
{/if}
