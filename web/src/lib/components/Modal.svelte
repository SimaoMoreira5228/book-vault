<script lang="ts">
	import X from "@lucide/svelte/icons/x";

	let {
		show = $bindable(false),
		title = "",
		maxWidth = "lg",
		showClose = true,
		children
	}: {
		show?: boolean;
		title?: string;
		maxWidth?: "sm" | "md" | "lg" | "xl";
		showClose?: boolean;
		children?: import("svelte").Snippet;
	} = $props();

	const widths: Record<string, string> = {
		sm: "max-w-sm",
		md: "max-w-md",
		lg: "max-w-lg",
		xl: "max-w-xl"
	};

	function close() {
		show = false;
	}
</script>

{#if show}
	<div
		class="bg-primary/40 fixed inset-0 z-50 flex items-center justify-center p-4 backdrop-blur-sm"
		role="dialog"
		aria-modal="true"
		tabindex="-1"
		onclick={close}
		onkeydown={(e) => {
			if (e.key === "Escape") close();
		}}
	>
		<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
		<div
			class={["bg-surface mx-auto w-full rounded-2xl p-8 shadow-2xl", widths[maxWidth]]}
			role="document"
			tabindex="-1"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => {
				if (e.key === "Escape") close();
			}}
		>
			{#if title || showClose}
				<div class="mb-6 flex items-center justify-between">
					{#if title}
						<h3 class="font-display text-headline-sm text-primary">{title}</h3>
					{/if}
					{#if showClose}
						<button
							onclick={close}
							class="text-on-surface-variant/50 hover:text-on-surface-variant p-1 transition-colors"
						>
							<X size={20} />
						</button>
					{/if}
				</div>
			{/if}
			{#if children}
				{@render children()}
			{/if}
		</div>
	</div>
{/if}
