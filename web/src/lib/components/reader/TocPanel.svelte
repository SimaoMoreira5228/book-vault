<script lang="ts">
	import * as m from "$lib/paraglide/messages";

	let {
		sections,
		show = $bindable(false),
		onNavigate
	}: {
		sections: Array<{ id: string; title: string | null }>;
		show?: boolean;
		onNavigate: (sectionId: string) => void;
	} = $props();

	const tocEntries = $derived(
		sections.filter((s): s is { id: string; title: string } => !!s.title)
	);
</script>

{#if show && tocEntries.length > 0}
	<div
		class="bg-surface border-outline/10 fixed top-16 right-0 z-40 h-[calc(100vh-4rem)] w-72 overflow-y-auto border-l shadow-lg"
	>
		<div class="p-6">
			<h3 class="font-display text-headline-sm text-primary mb-6">{m.reader_toc_contents()}</h3>
			<nav class="space-y-3">
				{#each tocEntries as entry (entry.id)}
					<button
						onclick={() => {
							onNavigate(entry.id);
							show = false;
						}}
						class="font-label text-label-md text-on-surface-variant hover:text-secondary block w-full text-left transition-colors"
						>{entry.title}</button
					>
				{/each}
			</nav>
		</div>
	</div>
{/if}
