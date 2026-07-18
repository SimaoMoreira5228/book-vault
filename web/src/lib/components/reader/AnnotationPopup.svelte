<script lang="ts">
	import Popup from "$lib/components/Popup.svelte";
	import { COLORS, COLOR_NAMES } from "$lib/ir/renderer";

	let {
		show = $bindable(false),
		x = 0,
		y = 0,
		onCreateColor
	}: {
		show?: boolean;
		x?: number;
		y?: number;
		onCreateColor: (color: string) => void;
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
	</div>
</Popup>
