<script lang="ts">
	import Minus from "@lucide/svelte/icons/minus";
	import Plus from "@lucide/svelte/icons/plus";

	let {
		fontSize = $bindable(18),
		lineHeight = $bindable(1.8)
	}: {
		fontSize?: number;
		lineHeight?: number;
	} = $props();

	function changeFontSize(delta: number) {
		const next = fontSize + delta;
		if (next >= 12 && next <= 36) fontSize = next;
	}

	function changeLineHeight(delta: number) {
		const next = parseFloat((lineHeight + delta).toFixed(1));
		if (next >= 1.2 && next <= 3.0) lineHeight = next;
	}
</script>

<div
	class="bg-surface border-outline/10 fixed top-20 right-4 z-50 flex items-center gap-4 rounded-xl border px-4 py-3 shadow-lg"
>
	<button onclick={() => changeFontSize(-2)} class="p-1 hover:opacity-70"
		><Minus size={16} /></button
	>
	<span class="font-label text-label-sm min-w-[3ch] text-center">{fontSize}</span>
	<button onclick={() => changeFontSize(2)} class="p-1 hover:opacity-70"><Plus size={16} /></button>
	<div class="bg-outline-variant/30 mx-2 h-6 w-px"></div>
	<button onclick={() => changeLineHeight(-0.2)} class="p-1 hover:opacity-70"
		><Minus size={16} /></button
	>
	<span class="font-label text-label-sm min-w-[3ch] text-center">{lineHeight.toFixed(1)}</span>
	<button onclick={() => changeLineHeight(0.2)} class="p-1 hover:opacity-70"
		><Plus size={16} /></button
	>
</div>
