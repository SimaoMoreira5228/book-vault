<script lang="ts">
	import X from "@lucide/svelte/icons/x";

	let {
		error = "",
		success = "",
		onDismiss = () => {}
	}: {
		error?: string;
		success?: string;
		onDismiss?: () => void;
	} = $props();

	function dismiss() {
		error = "";
		success = "";
		onDismiss();
	}
</script>

{#if error}
	<div
		class="font-label text-label-sm text-error bg-error-container/20 mb-6 flex items-center justify-between gap-2 rounded-lg px-4 py-3"
	>
		<span>{error}</span>
		<button onclick={dismiss} class="shrink-0 p-0.5 opacity-70 hover:opacity-100"
			><X size={14} /></button
		>
	</div>
{/if}

{#if success}
	<div
		class="font-label text-label-sm bg-secondary/10 text-secondary mb-6 flex items-center justify-between gap-2 rounded-lg px-4 py-3"
	>
		<span>{success}</span>
		<button onclick={dismiss} class="shrink-0 p-0.5 opacity-70 hover:opacity-100"
			><X size={14} /></button
		>
	</div>
{/if}
