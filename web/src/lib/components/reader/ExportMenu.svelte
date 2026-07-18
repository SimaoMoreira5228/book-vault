<script lang="ts">
	import * as m from "$lib/paraglide/messages";
	import { DropdownMenu } from "bits-ui";
	import { api } from "$lib/api/client.svelte";
	import FileDown from "@lucide/svelte/icons/file-down";

	let {
		bookId,
		onExport = () => {}
	}: {
		bookId: string;
		onExport?: () => void;
	} = $props();

	function doExport(format: string) {
		api.export(bookId, format);
		onExport();
	}
</script>

<DropdownMenu.Root>
	<DropdownMenu.Trigger
		class="focus-visible:ring-primary/20 rounded-lg p-2 transition-transform duration-200 hover:opacity-80 focus-visible:ring-2 focus-visible:outline-none active:scale-95"
		title={m.reader_export()}
	>
		<FileDown size={20} class="text-on-surface-variant" />
	</DropdownMenu.Trigger>
	<DropdownMenu.Portal>
		<DropdownMenu.Content
			class="bg-surface border-outline/10 data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 z-50 min-w-[140px] rounded-xl border p-1.5 shadow-lg"
		>
			<DropdownMenu.Item
				onclick={() => doExport("epub")}
				class="font-label text-label-md text-on-surface-variant hover:text-primary hover:bg-surface-container-low w-full cursor-pointer rounded-lg px-4 py-2 text-left transition-colors"
				>{m.reader_export_epub()}</DropdownMenu.Item
			>
			<DropdownMenu.Item
				onclick={() => doExport("pdf")}
				class="font-label text-label-md text-on-surface-variant hover:text-primary hover:bg-surface-container-low w-full cursor-pointer rounded-lg px-4 py-2 text-left transition-colors"
				>{m.reader_export_pdf()}</DropdownMenu.Item
			>
			<DropdownMenu.Item
				onclick={() => doExport("markdown")}
				class="font-label text-label-md text-on-surface-variant hover:text-primary hover:bg-surface-container-low w-full cursor-pointer rounded-lg px-4 py-2 text-left transition-colors"
				>{m.reader_export_markdown()}</DropdownMenu.Item
			>
		</DropdownMenu.Content>
	</DropdownMenu.Portal>
</DropdownMenu.Root>
