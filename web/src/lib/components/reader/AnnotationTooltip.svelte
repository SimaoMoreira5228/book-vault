<script lang="ts">
	import * as m from "$lib/paraglide/messages";
	import Popup from "$lib/components/Popup.svelte";
	import { api } from "$lib/api/client.svelte";
	import { COLORS } from "$lib/ir/renderer";
	import type { Annotation } from "$lib/ir/renderer";
	import X from "@lucide/svelte/icons/x";
	import MessageSquareText from "@lucide/svelte/icons/message-square-text";

	let {
		annotation = $bindable<Annotation | null>(null),
		onUpdate
	}: {
		annotation?: Annotation | null;
		onUpdate: () => Promise<void>;
	} = $props();

	let editNoteId = $state<string | null>(null);
	let noteDraft = $state("");
	let deleting = $state<string | null>(null);

	function startEditNote(ann: Annotation) {
		editNoteId = ann.id;
		noteDraft = ann.note ?? "";
	}

	async function saveNote(annId: string) {
		const result = await api.annotations.update(annId, { note: noteDraft });
		if (result.isOk()) await onUpdate();
		editNoteId = null;
	}

	async function deleteAnnotation(annId: string) {
		deleting = annId;
		const result = await api.annotations.delete(annId);
		if (result.isOk()) await onUpdate();
		deleting = null;
		annotation = null;
	}

	function close() {
		annotation = null;
		editNoteId = null;
	}
</script>

<Popup show={!!annotation} position="center">
	{@const ann = annotation}
	{#if ann}
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
					onclick={close}
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
	{/if}
</Popup>
