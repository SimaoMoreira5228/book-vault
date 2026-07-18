<script lang="ts">
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

	function close() {
		show = false;
	}
</script>

{#if show}
	<div
		class="fixed inset-0 z-50"
		onclick={close}
		onkeydown={(e) => {
			if (e.key === "Escape") close();
		}}
		role="presentation"
	>
		<div
			class={[
				"fixed z-50",
				position === "center"
					? "top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2"
					: position === "bottom"
						? "-translate-x-1/2"
						: "-translate-x-1/2 -translate-y-full"
			]}
			style={position !== "center" ? `left: ${x}px; top: ${y}px;` : ""}
			role="presentation"
			onclick={(e) => e.stopPropagation()}
		>
			{#if children}{@render children()}{/if}
		</div>
	</div>
{/if}
