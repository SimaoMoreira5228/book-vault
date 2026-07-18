<script lang="ts">
	import * as m from "$lib/paraglide/messages";
	import { api } from "$lib/api/client.svelte";

	let {
		show = $bindable(false),
		bookId,
		onExport = () => {}
	}: {
		show?: boolean;
		bookId: string;
		onExport?: () => void;
	} = $props();
</script>

{#if show}
	<div
		class="bg-surface border-outline/10 absolute top-10 right-0 z-50 min-w-[140px] rounded-xl border p-1.5 shadow-lg"
	>
		<button
			onclick={() => {
				api.export(bookId, "epub");
				show = false;
				onExport();
			}}
			class="font-label text-label-md text-on-surface-variant hover:text-primary hover:bg-surface-container-low w-full rounded-lg px-4 py-2 text-left transition-colors"
			>{m.reader_export_epub()}</button
		>
		<button
			onclick={() => {
				api.export(bookId, "pdf");
				show = false;
				onExport();
			}}
			class="font-label text-label-md text-on-surface-variant hover:text-primary hover:bg-surface-container-low w-full rounded-lg px-4 py-2 text-left transition-colors"
			>{m.reader_export_pdf()}</button
		>
		<button
			onclick={() => {
				api.export(bookId, "markdown");
				show = false;
				onExport();
			}}
			class="font-label text-label-md text-on-surface-variant hover:text-primary hover:bg-surface-container-low w-full rounded-lg px-4 py-2 text-left transition-colors"
			>{m.reader_export_markdown()}</button
		>
	</div>
{/if}
