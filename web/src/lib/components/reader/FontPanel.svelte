<script lang="ts">
	import * as m from "$lib/paraglide/messages";
	import { Slider } from "bits-ui";

	let {
		fontSize = $bindable(18),
		lineHeight = $bindable(1.8)
	}: {
		fontSize?: number;
		lineHeight?: number;
	} = $props();
</script>

<div
	class="bg-surface border-outline/10 fixed top-20 right-4 z-50 flex min-w-[200px] flex-col gap-4 rounded-xl border px-5 py-4 shadow-lg"
>
	<div class="flex flex-col gap-2">
		<div class="flex items-center justify-between">
			<span class="font-label text-label-sm text-on-surface-variant">{m.reader_font_title()}</span>
			<span class="font-label text-label-sm min-w-[3ch] text-right">{fontSize}px</span>
		</div>
		<Slider.Root
			type="single"
			bind:value={fontSize}
			min={12}
			max={36}
			step={2}
			class="bg-surface-container-low relative flex h-1.5 w-full items-center rounded-full"
		>
			<Slider.Range class="bg-secondary absolute h-full rounded-full" />
			<Slider.Thumb index={0}>
				{#snippet child({ props })}
					<span
						{...props}
						class="bg-secondary border-surface focus-visible:ring-primary/20 block size-4 rounded-full border-2 shadow-sm focus-visible:ring-2 focus-visible:outline-none"
					></span>
				{/snippet}
			</Slider.Thumb>
		</Slider.Root>
	</div>
	<div class="flex flex-col gap-2">
		<div class="flex items-center justify-between">
			<span class="font-label text-label-sm text-on-surface-variant"
				>{m.reader_font_line_height()}</span
			>
			<span class="font-label text-label-sm min-w-[3ch] text-right">{lineHeight.toFixed(1)}</span>
		</div>
		<Slider.Root
			type="single"
			bind:value={lineHeight}
			min={1.2}
			max={3.0}
			step={0.2}
			class="bg-surface-container-low relative flex h-1.5 w-full items-center rounded-full"
		>
			<Slider.Range class="bg-secondary absolute h-full rounded-full" />
			<Slider.Thumb index={0}>
				{#snippet child({ props })}
					<span
						{...props}
						class="bg-secondary border-surface focus-visible:ring-primary/20 block size-4 rounded-full border-2 shadow-sm focus-visible:ring-2 focus-visible:outline-none"
					></span>
				{/snippet}
			</Slider.Thumb>
		</Slider.Root>
	</div>
</div>
