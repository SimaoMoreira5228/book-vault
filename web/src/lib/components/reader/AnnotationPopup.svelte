<script lang="ts">
	import * as m from "$lib/paraglide/messages";
	import Popup from "$lib/components/Popup.svelte";
	import { COLORS, COLOR_NAMES } from "$lib/ir/renderer";
	import BookOpen from "@lucide/svelte/icons/book-open";

	let {
		show = $bindable(false),
		x = 0,
		y = 0,
		onCreateColor,
		onLookup
	}: {
		show?: boolean;
		x?: number;
		y?: number;
		onCreateColor: (color: string) => void;
		onLookup?: () => void;
	} = $props();
</script>

<Popup bind:show {x} {y} position="top">
	<div
		class="bg-surface border-outline/10 flex items-center gap-1 rounded-lg border px-2 py-1.5 shadow-lg"
	>
		{#each COLOR_NAMES as color (color)}
			<button
				onclick={() => onCreateColor(color)}
				class="h-6 w-6 rounded-full border-2 border-white/50 shadow-sm transition-transform hover:scale-110"
				style="background: {COLORS[color]};"
				title={color}
			></button>
		{/each}
		<span class="bg-outline-variant/30 mx-1 block h-6 w-px"></span>
		<button
			onclick={onLookup}
			class="text-on-surface-variant hover:text-primary flex items-center gap-1 rounded-md px-2 py-1 text-xs transition-colors"
			title={m.reader_lookup()}
		>
			<BookOpen size={14} />
			{m.reader_lookup()}
		</button>
	</div>
</Popup>
