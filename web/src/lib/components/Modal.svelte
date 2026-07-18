<script lang="ts">
	import { Dialog } from "bits-ui";
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
</script>

<Dialog.Root bind:open={show}>
	<Dialog.Portal>
		<Dialog.Overlay
			class="data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 fixed inset-0 z-50 bg-black/40 backdrop-blur-sm"
		/>
		<Dialog.Content
			class="bg-surface data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 fixed top-1/2 left-1/2 z-50 mx-auto w-full -translate-x-1/2 -translate-y-1/2 rounded-2xl p-8 shadow-2xl {widths[
				maxWidth
			]} outline-hidden"
		>
			{#if title || showClose}
				<div class="mb-6 flex items-center justify-between">
					{#if title}
						<Dialog.Title class="font-display text-headline-sm text-primary m-0"
							>{title}</Dialog.Title
						>
					{/if}
					{#if showClose}
						<Dialog.Close
							class="text-on-surface-variant/50 hover:text-on-surface-variant focus-visible:ring-primary/20 rounded-md p-1 transition-colors focus-visible:ring-2 focus-visible:outline-none"
						>
							<X size={20} />
						</Dialog.Close>
					{/if}
				</div>
			{/if}
			{#if children}
				{@render children()}
			{/if}
		</Dialog.Content>
	</Dialog.Portal>
</Dialog.Root>
