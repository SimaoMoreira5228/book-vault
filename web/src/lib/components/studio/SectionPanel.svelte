<script lang="ts">
	import * as m from "$lib/paraglide/messages";
	import Plus from "@lucide/svelte/icons/plus";
	import Trash2 from "@lucide/svelte/icons/trash-2";
	import FileText from "@lucide/svelte/icons/file-text";
	import GripVertical from "@lucide/svelte/icons/grip-vertical";
	import type { SpineItem } from "$lib/api/client.svelte";

	let {
		sections = [],
		activeSectionId,
		onSelectSection,
		onAddSection,
		onDeleteSection,
		onRenameSection
	}: {
		sections: SpineItem[];
		activeSectionId: string | null;
		onSelectSection: (id: string) => void;
		onAddSection: () => void;
		onDeleteSection: (id: string) => void;
		onRenameSection: (id: string, title: string) => void;
	} = $props();

	function handleRename(id: string, e: Event) {
		const target = e.currentTarget as HTMLElement;
		const title = target.textContent?.trim() ?? "";
		if (title) onRenameSection(id, title);
	}
</script>

<div class="flex h-full flex-col">
	<div class="mb-6 flex items-center justify-between px-6">
		<h3 class="font-display text-headline-sm text-primary">Chapters</h3>
		<button
			onclick={onAddSection}
			class="text-secondary hover:text-secondary/80 inline-flex items-center gap-1 rounded-lg px-3 py-1.5 text-sm transition-colors"
		>
			<Plus size={16} />
			{m.studio_new_section()}
		</button>
	</div>

	<nav class="flex-1 space-y-0.5 overflow-y-auto px-6">
		{#each sections as section (section.id)}
			<div
				onclick={() => onSelectSection(section.id)}
				onkeydown={(e) => {
					if (e.key === "Enter" || e.key === " ") {
						e.preventDefault();
						onSelectSection(section.id);
					}
				}}
				role="button"
				tabindex="0"
				class={[
					"font-label text-label-md flex w-full items-center gap-3 rounded-xl px-4 py-3 text-left transition-all",
					activeSectionId === section.id
						? "bg-secondary/5 text-secondary font-medium"
						: "text-on-surface-variant hover:bg-surface-container-low hover:text-primary"
				]}
			>
				<GripVertical size={14} class="text-on-surface-variant/20 shrink-0" />
				<div class="min-w-0 flex-1">
					<span
						contenteditable={activeSectionId === section.id}
						onblur={(e) => handleRename(section.id, e)}
						class="block truncate outline-none"
						role="textbox"
						aria-label="Section name"
					>
						{section.title ?? m.studio_untitled_section()}
					</span>
				</div>
				<button
					onclick={(e) => {
						e.stopPropagation();
						onDeleteSection(section.id);
					}}
					class="text-on-surface-variant/20 hover:text-error shrink-0 opacity-0 transition-all group-hover:opacity-100"
					aria-label="Delete section"
				>
					<Trash2 size={14} />
				</button>
			</div>
		{/each}
	</nav>
</div>
