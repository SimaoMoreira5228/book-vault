<script lang="ts">
	import * as m from "$lib/paraglide/messages";
	import { apiBase } from "$lib/api/client.svelte";
	import ChevronLeft from "@lucide/svelte/icons/chevron-left";
	import ChevronRight from "@lucide/svelte/icons/chevron-right";

	let {
		bookId,
		pages = $bindable([]),
		currentPage = $bindable(1)
	}: {
		bookId: string;
		pages?: Array<{ page: number; asset_id: string; mime_type: string }>;
		currentPage?: number;
	} = $props();

	let loading = $state(true);

	$effect(() => {
		if (pages.length > 0) loading = false;
	});

	function prev() {
		if (currentPage > 1) currentPage--;
	}
	function next() {
		if (currentPage < pages.length) currentPage++;
	}

	const currentUrl = $derived(`${apiBase}/api/v1/books/${bookId}/comic/page/${currentPage}`);
</script>

{#if loading}
	<div class="flex items-center justify-center py-16">
		<div
			class="border-secondary h-8 w-8 animate-spin rounded-full border-2 border-t-transparent"
		></div>
	</div>
{:else if pages.length > 0}
	<div class="flex flex-col items-center gap-6">
		<div
			class="border-outline/10 bg-surface-container w-full max-w-3xl overflow-hidden rounded-xl border"
		>
			<img src={currentUrl} alt="Page {currentPage}" class="h-auto w-full" />
		</div>
		<div class="flex items-center gap-4">
			<button
				onclick={prev}
				disabled={currentPage <= 1}
				class="font-label text-label-sm text-on-surface-variant hover:text-primary flex items-center gap-1 p-2 transition-colors disabled:cursor-not-allowed disabled:opacity-30"
				><ChevronLeft size={14} />{m.reader_prev()}</button
			>
			<span class="font-label text-label-md text-on-surface-variant/60 tracking-widest"
				>{currentPage} / {pages.length}</span
			>
			<button
				onclick={next}
				disabled={currentPage >= pages.length}
				class="font-label text-label-sm text-on-surface-variant hover:text-primary flex items-center gap-1 p-2 transition-colors disabled:cursor-not-allowed disabled:opacity-30"
				>{m.reader_next()}<ChevronRight size={14} /></button
			>
		</div>
	</div>
{/if}
