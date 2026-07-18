<script lang="ts">
	import { apiBase } from "$lib/api/client.svelte";
	import BookOpen from "@lucide/svelte/icons/book-open";

	let {
		bookId = "",
		alt = "Book cover",
		class: className = "",
		coverClass = ""
	}: {
		bookId: string;
		alt?: string;
		class?: string;
		coverClass?: string;
	} = $props();

	let loaded = $state(false);
	let error = $state(false);

	function onLoad() {
		loaded = true;
	}
	function onError() {
		error = true;
		loaded = true;
	}
</script>

<div class={["relative overflow-hidden", className]}>
	{#if !error}
		<img
			src="{apiBase}/api/v1/books/{bookId}/cover"
			{alt}
			loading="lazy"
			class={[
				"h-full w-full object-cover transition-opacity duration-300",
				!loaded ? "opacity-0" : "opacity-100",
				coverClass
			]}
			onload={onLoad}
			onerror={onError}
		/>
		<div class="book-spine-effect absolute inset-0"></div>
	{/if}
	{#if !loaded}
		<div class="flex h-full w-full items-center justify-center bg-[rgba(0,31,63,0.03)]">
			<BookOpen size={24} class="text-on-surface-variant/20" />
		</div>
	{/if}
</div>
