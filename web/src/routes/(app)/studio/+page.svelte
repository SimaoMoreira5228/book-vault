<script lang="ts">
	import { goto } from '$app/navigation';
	import { api } from '$lib/api/client';
	import type { BookResponse } from '$lib/api/generated';
	import Plus from '@lucide/svelte/icons/plus';
	import ChevronDown from '@lucide/svelte/icons/chevron-down';
	import BookOpen from '@lucide/svelte/icons/book-open';
	import Clock from '@lucide/svelte/icons/clock';
	import PenSquare from '@lucide/svelte/icons/pen-square';
	import PlusCircle from '@lucide/svelte/icons/plus-circle';

	let drafts = $state<BookResponse[]>([]);
	let loading = $state(true);

	$effect(() => {
		loadDrafts();
	});

	async function loadDrafts() {
		loading = true;
		const result = await api.books.list();
		if (result.isOk()) {
			drafts = result.value.filter((b) => b.format === 'native');
		}
		loading = false;
	}

	async function createNew() {
		const result = await api.books.create({
			title: 'Untitled Manuscript',
			format: 'native',
			author: null,
			isbn: null,
			language: null,
			publisher: null,
			series: null,
			series_index: null,
			page_count: null
		});
		if (result.isOk()) {
			drafts = [result.value, ...drafts];
		}
	}
</script>

<!-- Header & Action Section -->
<section class="flex flex-col md:flex-row md:items-end justify-between gap-6 mb-12">
	<div>
		<p class="font-label text-label-sm text-secondary uppercase tracking-widest mb-2">Author Dashboard</p>
		<h2 class="font-display text-headline-md text-primary">Creator Studio</h2>
	</div>
	<button onclick={createNew} class="btn-primary">
		<Plus size={20} />
		Start New Project
	</button>
</section>

<!-- Insights Bento Grid -->
<section class="grid grid-cols-1 md:grid-cols-3 gap-6 mb-section-gap">
	<div class="md:col-span-2 bg-white rounded-xl p-8 border border-primary/5 shadow-sm">
		<div class="flex justify-between items-start mb-8">
			<h3 class="font-display text-headline-sm text-primary">Writing Insights</h3>
			<div class="flex items-center gap-2 text-on-surface-variant font-label text-label-sm">
				<span>This Week</span>
				<ChevronDown size={18} />
			</div>
		</div>
		<div class="h-48 flex items-end justify-between gap-2 md:gap-4 px-2">
			{#each [
				{ day: 'M', height: 40 },
				{ day: 'T', height: 65 },
				{ day: 'W', height: 85 },
				{ day: 'T', height: 95 },
				{ day: 'F', height: 30 },
				{ day: 'S', height: 55 },
				{ day: 'S', height: 20 }
			] as bar}
				<div class="flex flex-col items-center flex-1">
					<div
						class="w-full bg-surface-container rounded-t-lg transition-all duration-1000"
						style="height: {bar.height}%"
					/>
					<span class="mt-2 font-label text-label-sm opacity-50">{bar.day}</span>
				</div>
			{/each}
		</div>
	</div>
	<div class="flex flex-col gap-6">
		<div class="flex-1 bg-surface-container-low rounded-xl p-6 border border-primary/5 flex flex-col justify-between">
			<BookOpen size={20} class="text-secondary" />
			<div>
				<p class="text-[32px] font-display leading-none mb-1">{drafts.length > 0 ? `${drafts.length}` : '0'}</p>
				<p class="font-label text-label-sm opacity-60">Manuscripts</p>
			</div>
		</div>
		<div class="flex-1 bg-primary-container text-white rounded-xl p-6 border border-primary/5 flex flex-col justify-between">
			<Clock size={20} class="text-on-primary-container" />
			<div>
				<p class="text-[32px] font-display leading-none mb-1">--</p>
				<p class="font-label text-label-sm opacity-60">Deep focus time</p>
			</div>
		</div>
	</div>
</section>

<!-- Current Drafts -->
<section class="mb-section-gap">
	<div class="flex items-center justify-between mb-8">
		<h3 class="font-display text-headline-sm text-primary">Current Drafts</h3>
	</div>

	{#if loading}
		<div class="flex items-center justify-center py-16">
			<div class="w-8 h-8 border-2 border-secondary border-t-transparent rounded-full animate-spin" />
		</div>
	{:else if drafts.length === 0}
		<div class="border-2 border-dashed border-outline-variant/30 rounded-xl flex flex-col items-center justify-center p-16 hover:bg-surface-container/30 transition-colors cursor-pointer group" onclick={createNew}>
			<div class="w-16 h-16 rounded-full bg-surface-container flex items-center justify-center mb-4 group-hover:scale-110 transition-transform">
				<PlusCircle size={20} class="text-on-surface-variant" />
			</div>
			<p class="font-display text-[16px] text-on-surface-variant">New Manuscript</p>
			<p class="font-label text-label-sm text-on-surface-variant opacity-60 text-center mt-2">Start your next masterpiece</p>
		</div>
	{:else}
		<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-8">
			{#each drafts as draft}
				<a href="/reader/{draft.id}" class="group cursor-pointer">
					<div class="aspect-[3/4] relative rounded-xl overflow-hidden mb-4 shadow-lg transition-transform group-hover:-translate-y-2 duration-500 bg-surface-container">
						<div class="absolute inset-0 book-spine-gradient" />
						<div class="w-full h-full flex items-center justify-center">
							<PenSquare size={48} class="text-on-surface-variant/20" />
						</div>
						<div class="absolute bottom-4 right-4">
							<span class="bg-white/90 backdrop-blur px-3 py-1 rounded-full text-[10px] font-label uppercase tracking-widest text-primary">Draft</span>
						</div>
					</div>
					<h4 class="font-display text-[18px] mb-1">{draft.title}</h4>
					<div class="flex items-center gap-4 text-on-surface-variant font-label text-label-sm">
						<span class="flex items-center gap-1">
							<Clock size={16} />
							{draft.created_at}
						</span>
					</div>
				</a>
			{/each}
			<div class="border-2 border-dashed border-outline-variant/30 rounded-xl flex flex-col items-center justify-center aspect-[3/4] hover:bg-surface-container/30 transition-colors cursor-pointer group" onclick={createNew}>
				<div class="w-16 h-16 rounded-full bg-surface-container flex items-center justify-center mb-4 group-hover:scale-110 transition-transform">
					<PlusCircle size={20} class="text-on-surface-variant" />
				</div>
				<p class="font-display text-[16px] text-on-surface-variant">New Manuscript</p>
			</div>
		</div>
	{/if}
</section>
