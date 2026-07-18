<script lang="ts">
	import { Popover } from "bits-ui";

	let {
		show = $bindable(false),
		x = 0,
		y = 0,
		position = "top" as "top" | "bottom" | "center",
		children
	}: {
		show?: boolean;
		x?: number;
		y?: number;
		position?: "top" | "bottom" | "center";
		children?: import("svelte").Snippet;
	} = $props();

	let anchorEl = $state<HTMLSpanElement>();

	function handleOpenChange(o: boolean) {
		if (!o) show = false;
	}

	const side = $derived(position === "center" ? "top" : position);

	const contentClass = $derived(
		position === "center"
			? ""
			: "z-50 data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95"
	);
</script>

{#if show}
	<span
		bind:this={anchorEl}
		style="position: fixed; left: {x}px; top: {y}px; width: 1px; height: 1px; pointer-events: none;"
	></span>
{/if}
<Popover.Root open={show} onOpenChange={handleOpenChange}>
	<Popover.Trigger>
		<span></span>
	</Popover.Trigger>
	<Popover.Content
		customAnchor={anchorEl}
		{side}
		align="center"
		class={contentClass}
		onInteractOutside={(e) => e.preventDefault()}
	>
		{#if children}
			{@render children()}
		{/if}
	</Popover.Content>
</Popover.Root>
