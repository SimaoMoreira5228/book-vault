<script lang="ts">
	import Bold from "@lucide/svelte/icons/bold";
	import Italic from "@lucide/svelte/icons/italic";
	import Underline from "@lucide/svelte/icons/underline";
	import Strikethrough from "@lucide/svelte/icons/strikethrough";

	let {
		show = false,
		x = 0,
		y = 0,
		onFormat
	}: {
		show?: boolean;
		x?: number;
		y?: number;
		onFormat: (format: string) => void;
	} = $props();
</script>

{#if show}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		class="bg-surface border-outline/10 fixed z-[100] flex -translate-x-1/2 -translate-y-full gap-0.5 rounded-xl border px-2 py-1.5 shadow-lg"
		style="left: {x}px; top: {y - 12}px;"
		onmouseup={(e) => e.stopPropagation()}
	>
		<button
			onclick={() => onFormat("bold")}
			class="text-on-surface-variant hover:text-primary hover:bg-surface-container-low rounded-lg p-1.5 transition-all"
			title="Bold"
		>
			<Bold size={16} />
		</button>
		<button
			onclick={() => onFormat("italic")}
			class="text-on-surface-variant hover:text-primary hover:bg-surface-container-low rounded-lg p-1.5 transition-all"
			title="Italic"
		>
			<Italic size={16} />
		</button>
		<button
			onclick={() => onFormat("underline")}
			class="text-on-surface-variant hover:text-primary hover:bg-surface-container-low rounded-lg p-1.5 transition-all"
			title="Underline"
		>
			<Underline size={16} />
		</button>
		<button
			onclick={() => onFormat("strikethrough")}
			class="text-on-surface-variant hover:text-primary hover:bg-surface-container-low rounded-lg p-1.5 transition-all"
			title="Strikethrough"
		>
			<Strikethrough size={16} />
		</button>
	</div>
{/if}
