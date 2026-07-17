<script lang="ts">
	import { api } from '$lib/api/client';
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import type { BookIr, Block, Span } from '$lib/api/generated';
	import ArrowLeft from '@lucide/svelte/icons/arrow-left';
	import Type from '@lucide/svelte/icons/type';
	import Bookmark from '@lucide/svelte/icons/bookmark';
	import Image from '@lucide/svelte/icons/image';
	import ChevronLeft from '@lucide/svelte/icons/chevron-left';
	import ChevronRight from '@lucide/svelte/icons/chevron-right';

	let bookData = $state<{ book: BookIr } | null>(null);
	let loading = $state(true);
	let progress = $state(0);

	const bookId = $derived(page.params.id ?? '');

	$effect(() => {
		if (bookId) {
			loadBook();
		}
	});

	async function loadBook() {
		loading = true;
		const result = await api.read(bookId);
		if (result.isOk()) {
			bookData = result.value as { book: BookIr };
		}
		loading = false;
	}

	function onScroll(e: Event) {
		const el = e.target as HTMLElement;
		const scrollTop = el.scrollTop;
		const scrollHeight = el.scrollHeight - el.clientHeight;
		progress = scrollHeight > 0 ? Math.min(100, Math.round((scrollTop / scrollHeight) * 100)) : 0;
	}

	function extractText(spans: Span[]): string {
		return spans.map((s) => s.text).join('');
	}

	function blockValue<T>(b: Block, key: string): T | undefined {
		return (b as Record<string, unknown>)[key] as T | undefined;
	}
</script>

<svelte:window onscroll={onScroll} />

<!-- Reading Progress Indicator -->
<div class="fixed top-0 left-0 w-full z-[60] bg-surface-container-low/20">
	<div class="h-[2px] bg-secondary transition-all duration-300" style="width: {progress}%;" />
</div>

<!-- TopAppBar -->
<header class="fixed top-0 w-full z-50 bg-surface/90 backdrop-blur-md shadow-[0_1px_4px_rgba(0,31,63,0.05)] flex justify-between items-center px-margin-mobile md:px-margin-desktop h-16">
	<div class="flex items-center gap-4">
		<button onclick={() => goto('/')} class="active:scale-95 transition-transform duration-200 hover:opacity-80 flex items-center justify-center p-2">
			<ArrowLeft size={20} class="text-primary" />
		</button>
		<h1 class="font-display text-headline-sm text-primary truncate max-w-[240px] md:max-w-md">
			{bookData?.book.spine[0]?.title ?? 'Loading...'}
		</h1>
	</div>
	<div class="flex items-center gap-2">
		<button class="active:scale-95 transition-transform duration-200 hover:opacity-80 p-2">
			<Type size={20} class="text-on-surface-variant" />
		</button>
		<button class="active:scale-95 transition-transform duration-200 hover:opacity-80 p-2">
			<Bookmark size={20} class="text-on-surface-variant" />
		</button>
	</div>
</header>

<main class="min-h-screen pt-32 pb-40 px-margin-mobile md:px-0" style="max-width: 800px; margin: 0 auto;">
	{#if loading}
		<div class="flex items-center justify-center py-32">
			<div class="w-8 h-8 border-2 border-secondary border-t-transparent rounded-full animate-spin" />
		</div>
	{:else if bookData}
		<article class="space-y-8">
			{#each bookData.book.spine as section}
				{#if section.title}
					<h2 class="font-display text-headline-md text-primary text-center mb-12">{section.title}</h2>
				{/if}
				{#each section.blocks as block}
					{@const b = block as Record<string, unknown>}
					{#if 'Paragraph' in b}
						<p class="font-body text-body-lg text-on-surface/90 leading-relaxed mb-8">
							{extractText(b.Paragraph as Span[])}
						</p>
					{:else if 'Heading' in b}
						{@const h = b.Heading as { level: number; spans: Span[] }}
						<h3 class="font-display text-headline-sm text-primary mt-12 mb-6">{extractText(h.spans)}</h3>
					{:else if 'Image' in b}
						<div class="my-16 overflow-hidden rounded-xl border border-on-surface/5 bg-surface-container">
							<div class="aspect-[16/9] flex items-center justify-center">
								<Image size={32} class="text-on-surface-variant/30" />
							</div>
						</div>
					{:else if 'CodeBlock' in b}
						{@const cb = b.CodeBlock as { language: string | null; content: string }}
						<pre class="bg-surface-container-high p-6 rounded-xl overflow-x-auto font-mono text-sm mb-8"><code>{cb.content}</code></pre>
					{:else if 'HorizontalRule' in b}
						<hr class="border-outline-variant my-12" />
					{/if}
				{/each}
			{/each}
		</article>
	{/if}
</main>

<!-- Footer Pagination -->
<footer class="fixed bottom-0 left-0 w-full py-6 px-margin-mobile flex justify-between items-center bg-surface/80 backdrop-blur-sm">
	<div class="flex items-center gap-2">
		<button class="flex items-center gap-1 font-label text-label-sm text-on-surface-variant hover:text-primary transition-colors p-2">
			<ChevronLeft size={14} />
			Previous
		</button>
	</div>
	<div class="font-label text-label-md text-on-surface-variant/60 tracking-widest">
		{progress}% Complete
	</div>
	<div class="flex items-center gap-2">
		<button class="flex items-center gap-1 font-label text-label-sm text-on-surface-variant hover:text-primary transition-colors p-2">
			Next
			<ChevronRight size={14} />
		</button>
	</div>
</footer>
