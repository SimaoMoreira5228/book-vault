<script lang="ts">
	import {
		renderBlockContent,
		htmlToSpans,
		createEmptyBlock,
		getBlockType
	} from "../../../utils/blockEditor.js";
	import type { Block } from "$lib/api/generated";
	import { ContextMenu } from "bits-ui";
	import Plus from "@lucide/svelte/icons/plus";
	import Heading1 from "@lucide/svelte/icons/heading-1";
	import Heading2 from "@lucide/svelte/icons/heading-2";
	import Heading3 from "@lucide/svelte/icons/heading-3";
	import Quote from "@lucide/svelte/icons/quote";
	import List from "@lucide/svelte/icons/list";
	import ListOrdered from "@lucide/svelte/icons/list-ordered";
	import Code from "@lucide/svelte/icons/code";
	import Minus from "@lucide/svelte/icons/minus";
	import Type from "@lucide/svelte/icons/type";
	import Trash2 from "@lucide/svelte/icons/trash-2";

	let {
		blockIndex,
		totalBlocks,
		block,
		onUpdate,
		onDelete,
		onSplit,
		onMergeUp,
		onAddAfter
	}: {
		blockIndex: number;
		totalBlocks: number;
		block: Block;
		onUpdate: (index: number, newBlock: Block) => void;
		onDelete: (index: number) => void;
		onSplit: (index: number, beforeText: string, afterText: string) => void;
		onMergeUp: (index: number) => void;
		onAddAfter: (index: number, block: Block) => void;
	} = $props();

	let blockEl = $state<HTMLElement>();
	let contextMenuOpen = $state(false);
	let anchorEl = $state<HTMLSpanElement>();

	let blockType = $derived(getBlockType(block));
	let isTextBlock = $derived(
		blockType === "Paragraph" || blockType === "Heading" || blockType === "BlockQuote"
	);

	let showAddMenu = $state(false);

	function handleInput(e: Event) {
		const target = e.currentTarget as HTMLElement;
		const spans = htmlToSpans(target);
		const updated = JSON.parse(JSON.stringify(block)) as Record<string, unknown>;

		if (blockType === "Paragraph") {
			updated.Paragraph = spans;
		} else if (blockType === "Heading") {
			const h = { ...(updated.Heading as Record<string, unknown>) };
			h.spans = spans;
			updated.Heading = h;
		} else if (blockType === "BlockQuote") {
			updated.BlockQuote = [{ Paragraph: spans }];
		} else {
			return;
		}

		onUpdate(blockIndex, updated as Block);
	}

	function handleKeydown(e: KeyboardEvent) {
		const el = e.currentTarget as HTMLElement;

		if (e.key === "Enter" && !e.shiftKey) {
			e.preventDefault();
			const sel = window.getSelection();
			if (!sel || !sel.rangeCount) return;

			const allText = el.textContent ?? "";
			const caretPos = getCaretPosition(el);
			const before = allText.slice(0, caretPos);
			const after = allText.slice(caretPos);

			onSplit(blockIndex, before, after);
		}

		if (e.key === "Backspace") {
			const sel = window.getSelection();
			if (!sel || !sel.rangeCount) return;
			const r = sel.getRangeAt(0);
			if (r.collapsed && r.startOffset === 0) {
				e.preventDefault();
				if (totalBlocks > 1) {
					onMergeUp(blockIndex);
				}
			}
		}

		if (e.key === "ArrowUp" && blockIndex > 0) {
			const sel = window.getSelection();
			if (sel && sel.rangeCount > 0) {
				const range = sel.getRangeAt(0);
				if (range.collapsed && range.startOffset === 0) {
					e.preventDefault();
					moveFocusToBlock(blockIndex - 1, "end");
				}
			}
		}

		if (e.key === "ArrowDown" && blockIndex < totalBlocks - 1) {
			const sel = window.getSelection();
			if (sel && sel.rangeCount > 0) {
				const range = sel.getRangeAt(0);
				if (range.collapsed && isAtEnd(el)) {
					e.preventDefault();
					moveFocusToBlock(blockIndex + 1, "start");
				}
			}
		}
	}

	function getCaretPosition(el: HTMLElement): number {
		const sel = window.getSelection();
		if (!sel || !sel.rangeCount) return 0;
		const range = sel.getRangeAt(0);
		const temp = document.createRange();
		temp.selectNodeContents(el);
		temp.setEnd(range.startContainer, range.startOffset);
		return temp.toString().length;
	}

	function isAtEnd(el: HTMLElement): boolean {
		const sel = window.getSelection();
		if (!sel || !sel.rangeCount) return false;
		const pos = getCaretPosition(el);
		return pos >= (el.textContent ?? "").length;
	}

	function moveFocusToBlock(index: number, position: "start" | "end") {
		const allBlocks = document.querySelectorAll("[data-editable-block]");
		const target = allBlocks[index] as HTMLElement;
		if (!target) return;
		target.focus();
		const sel = window.getSelection();
		if (!sel) return;
		const range = document.createRange();
		if (position === "end") {
			range.selectNodeContents(target);
			range.collapse(false);
		} else {
			range.setStart(target.firstChild ?? target, 0);
			range.collapse(true);
		}
		sel.removeAllRanges();
		sel.addRange(range);
	}

	function handleFormat(format: string) {
		if (!blockEl) return;
		blockEl.focus();
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

		handleInput({ currentTarget: blockEl } as unknown as Event);
	}

	function changeBlockType(type: string) {
		const newBlock = createEmptyBlock(type);
		if (typeof block === "string" || typeof newBlock === "string") {
			onUpdate(blockIndex, newBlock);
			return;
		}

		const currentText = blockToNativeText();
		const newB = newBlock as Record<string, unknown>;
		if (type === "Paragraph") {
			newB.Paragraph = [{ text: currentText, marks: 0, href: null }];
		} else if (type === "Heading") {
			newB.Heading = { level: 1, spans: [{ text: currentText, marks: 0, href: null }] };
		} else if (type === "CodeBlock") {
			newB.CodeBlock = { language: null, content: currentText };
		}

		onUpdate(blockIndex, newB as Block);
		contextMenuOpen = false;
		showAddMenu = false;
	}

	function blockToNativeText(): string {
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

	function handleContextMenu(e: MouseEvent) {
		e.preventDefault();
		anchorEl?.remove();
		const span = document.createElement("span");
		span.style.cssText = `position:fixed;left:${e.clientX}px;top:${e.clientY}px;width:1px;height:1px;pointer-events:none;`;
		document.body.appendChild(span);
		anchorEl = span;
		contextMenuOpen = true;
	}

	function handleAddBlock(type: string) {
		onAddAfter(blockIndex, createEmptyBlock(type));
		showAddMenu = false;
	}
</script>

<div class="group relative mb-0.5">
	<div
		class="absolute top-0 -left-10 flex h-full flex-col items-center justify-center opacity-0 transition-opacity group-hover:opacity-100"
	>
		<button
			onclick={() => (showAddMenu = !showAddMenu)}
			class="text-on-surface-variant/30 hover:text-secondary flex h-6 w-6 items-center justify-center rounded-full transition-colors"
			aria-label="Add block"
		>
			<Plus size={14} />
		</button>
	</div>

	{#if showAddMenu}
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			class="absolute top-0 -left-32 z-50 flex gap-0.5 rounded-lg border border-[rgba(0,31,63,0.08)] bg-white p-1 shadow-lg"
			onclick={(e) => e.stopPropagation()}
		>
			<button
				onclick={() => handleAddBlock("Paragraph")}
				class="text-on-surface-variant hover:text-primary hover:bg-surface-container-low rounded-md p-1.5 transition-all"
				title="Paragraph"><Type size={16} /></button
			>
			<button
				onclick={() => handleAddBlock("Heading")}
				class="text-on-surface-variant hover:text-primary hover:bg-surface-container-low rounded-md p-1.5 transition-all"
				title="Heading 1"><Heading1 size={16} /></button
			>
			<button
				onclick={() => handleAddBlock("Heading")}
				class="text-on-surface-variant hover:text-primary hover:bg-surface-container-low rounded-md p-1.5 transition-all"
				title="Heading 2"><Heading2 size={16} /></button
			>
			<button
				onclick={() => handleAddBlock("Heading")}
				class="text-on-surface-variant hover:text-primary hover:bg-surface-container-low rounded-md p-1.5 transition-all"
				title="Heading 3"><Heading3 size={16} /></button
			>
			<button
				onclick={() => handleAddBlock("BlockQuote")}
				class="text-on-surface-variant hover:text-primary hover:bg-surface-container-low rounded-md p-1.5 transition-all"
				title="Quote"><Quote size={16} /></button
			>
			<button
				onclick={() => handleAddBlock("UnorderedList")}
				class="text-on-surface-variant hover:text-primary hover:bg-surface-container-low rounded-md p-1.5 transition-all"
				title="List"><List size={16} /></button
			>
			<button
				onclick={() => handleAddBlock("OrderedList")}
				class="text-on-surface-variant hover:text-primary hover:bg-surface-container-low rounded-md p-1.5 transition-all"
				title="Ordered List"><ListOrdered size={16} /></button
			>
			<button
				onclick={() => handleAddBlock("CodeBlock")}
				class="text-on-surface-variant hover:text-primary hover:bg-surface-container-low rounded-md p-1.5 transition-all"
				title="Code Block"><Code size={16} /></button
			>
			<button
				onclick={() => handleAddBlock("HorizontalRule")}
				class="text-on-surface-variant hover:text-primary hover:bg-surface-container-low rounded-md p-1.5 transition-all"
				title="Divider"><Minus size={16} /></button
			>
		</div>
	{/if}

	{#if blockType === "HorizontalRule"}
		<div class="flex items-center gap-3 py-4" role="none">
			<div class="bg-outline-variant/30 h-px flex-1"></div>
			<button
				onclick={() => onDelete(blockIndex)}
				class="text-on-surface-variant/30 hover:text-error flex h-6 w-6 items-center justify-center rounded-full transition-colors"
				aria-label="Remove divider"
			>
				<Trash2 size={12} />
			</button>
			<div class="bg-outline-variant/30 h-px flex-1"></div>
		</div>
	{:else if blockType === "CodeBlock"}
		<div class="relative">
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<div
				class="bg-surface-container-high min-h-[60px] w-full rounded-xl border-0 px-6 py-4 font-mono text-sm leading-relaxed outline-none"
				contenteditable="true"
				aria-multiline="true"
				data-editable-block={blockIndex}
				oninput={handleInput}
				onkeydown={handleKeydown}
				bind:this={blockEl}
			></div>
			<div class="absolute top-2 right-2">
				<button
					onclick={() => onDelete(blockIndex)}
					class="text-on-surface-variant/30 hover:text-error flex h-6 w-6 items-center justify-center rounded-full transition-colors"
					aria-label="Delete block"
				>
					<Trash2 size={12} />
				</button>
			</div>
		</div>
	{:else if blockType === "UnorderedList" || blockType === "OrderedList"}
		<div class="pl-6">
			<div class="flex items-center gap-2 py-1">
				<span class="text-on-surface-variant/40 text-sm"
					>{blockType === "OrderedList" ? "1." : "•"}</span
				>
				{#if blockType === "UnorderedList"}
					<!-- eslint-disable-next-line @typescript-eslint/no-unused-vars -->
					{#each (block as { UnorderedList: Array<Array<Block>> }).UnorderedList as _, i (i)}
						<div class="w-full">
							<p class="w-full" contenteditable="true"></p>
						</div>
					{/each}
				{:else}
					<!-- eslint-disable-next-line @typescript-eslint/no-unused-vars -->
					{#each (block as { OrderedList: Array<Array<Block>> }).OrderedList as _, i (i)}
						<span class="w-full" contenteditable="true"></span>
					{/each}
				{/if}
			</div>
		</div>
	{:else}
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div class="relative" oncontextmenu={handleContextMenu}>
			{#if blockType === "Heading"}
				<h3
					class="font-display text-headline-sm text-primary w-full border-0 bg-transparent px-0 py-2 outline-none"
					contenteditable="true"
					data-editable-block={blockIndex}
					oninput={handleInput}
					onkeydown={handleKeydown}
					bind:this={blockEl}
				>
					<!-- eslint-disable-next-line svelte/no-at-html-tags -->
					{@html renderBlockContent(block)}
				</h3>
			{:else if blockType === "BlockQuote"}
				<blockquote
					class="border-secondary/30 text-on-surface-variant mb-0 w-full border-l-4 pl-6 italic"
					contenteditable="true"
					aria-multiline="true"
					data-editable-block={blockIndex}
					oninput={handleInput}
					onkeydown={handleKeydown}
					bind:this={blockEl}
				>
					<!-- eslint-disable-next-line svelte/no-at-html-tags -->
					{@html renderBlockContent(block)}
				</blockquote>
			{:else}
				<p
					class="text-on-surface w-full border-0 bg-transparent px-0 py-1 leading-relaxed outline-none"
					contenteditable="true"
					aria-multiline="true"
					data-editable-block={blockIndex}
					oninput={handleInput}
					onkeydown={handleKeydown}
					bind:this={blockEl}
				>
					<!-- eslint-disable-next-line svelte/no-at-html-tags -->
					{@html renderBlockContent(block)}
				</p>
			{/if}
		</div>
	{/if}
</div>

<ContextMenu.Root
	open={contextMenuOpen}
	onOpenChange={(o) => {
		if (!o) contextMenuOpen = false;
	}}
>
	<ContextMenu.Portal>
		<ContextMenu.Content
			customAnchor={anchorEl}
			class="bg-surface border-outline/10 data-[state=open]:animate-in data-[state=closed]:animate-out z-50 min-w-[160px] rounded-xl border p-1.5 shadow-lg"
			sideOffset={4}
		>
			{#if isTextBlock}
				<ContextMenu.Sub>
					<ContextMenu.SubTrigger
						class="font-label text-label-md data-highlighted:bg-surface-container-low flex w-full cursor-pointer items-center gap-2 rounded-lg px-3 py-2 text-left transition-colors focus-visible:outline-none"
					>
						Format
					</ContextMenu.SubTrigger>
					<ContextMenu.SubContent
						class="bg-surface border-outline/10 z-50 ml-1 rounded-xl border p-1 shadow-lg"
					>
						<ContextMenu.Item
							onclick={() => handleFormat("bold")}
							class="font-label text-label-md data-highlighted:bg-surface-container-low flex w-full cursor-pointer items-center gap-2 rounded-lg px-3 py-2 text-left transition-colors focus-visible:outline-none"
						>
							<b>Bold</b>
						</ContextMenu.Item>
						<ContextMenu.Item
							onclick={() => handleFormat("italic")}
							class="font-label text-label-md data-highlighted:bg-surface-container-low flex w-full cursor-pointer items-center gap-2 rounded-lg px-3 py-2 text-left transition-colors focus-visible:outline-none"
						>
							<i>Italic</i>
						</ContextMenu.Item>
						<ContextMenu.Item
							onclick={() => handleFormat("underline")}
							class="font-label text-label-md data-highlighted:bg-surface-container-low flex w-full cursor-pointer items-center gap-2 rounded-lg px-3 py-2 text-left transition-colors focus-visible:outline-none"
						>
							<u>Underline</u>
						</ContextMenu.Item>
						<ContextMenu.Item
							onclick={() => handleFormat("strikethrough")}
							class="font-label text-label-md data-highlighted:bg-surface-container-low flex w-full cursor-pointer items-center gap-2 rounded-lg px-3 py-2 text-left transition-colors focus-visible:outline-none"
						>
							<s>Strikethrough</s>
						</ContextMenu.Item>
					</ContextMenu.SubContent>
				</ContextMenu.Sub>
			{/if}

			<ContextMenu.Sub>
				<ContextMenu.SubTrigger
					class="font-label text-label-md data-highlighted:bg-surface-container-low flex w-full cursor-pointer items-center gap-2 rounded-lg px-3 py-2 text-left transition-colors focus-visible:outline-none"
				>
					Turn into
				</ContextMenu.SubTrigger>
				<ContextMenu.SubContent
					class="bg-surface border-outline/10 z-50 ml-1 rounded-xl border p-1 shadow-lg"
				>
					{#each ["Paragraph", "Heading", "BlockQuote", "CodeBlock", "UnorderedList", "HorizontalRule"] as t (t)}
						<ContextMenu.Item
							onclick={() => changeBlockType(t)}
							class="font-label text-label-md data-highlighted:bg-surface-container-low flex w-full cursor-pointer items-center gap-2 rounded-lg px-3 py-2 text-left transition-colors focus-visible:outline-none"
						>
							{t === blockType ? "✓ " : ""}{t}
						</ContextMenu.Item>
					{/each}
				</ContextMenu.SubContent>
			</ContextMenu.Sub>

			<ContextMenu.Separator class="bg-outline-variant/30 -mx-1 my-1 block h-px" />

			<ContextMenu.Item
				onclick={() => onDelete(blockIndex)}
				class="font-label text-label-md data-highlighted:bg-surface-container-low text-error flex w-full cursor-pointer items-center gap-2 rounded-lg px-3 py-2 text-left transition-colors focus-visible:outline-none"
			>
				Delete
			</ContextMenu.Item>
		</ContextMenu.Content>
	</ContextMenu.Portal>
</ContextMenu.Root>
