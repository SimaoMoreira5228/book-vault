<script lang="ts">
	import { authState } from '$lib/api/client';
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import BookOpen from '@lucide/svelte/icons/book-open';
	import Search from '@lucide/svelte/icons/search';
	import PenSquare from '@lucide/svelte/icons/pen-square';
	import ScrollText from '@lucide/svelte/icons/scroll-text';
	import Bell from '@lucide/svelte/icons/bell';

	let { children } = $props();

	let path = $derived(page.url.pathname);

	$effect(() => {
		if (!authState.isAuthenticated) {
			goto('/login');
		}
	});

	const isCurrent = (p: string) => {
		if (p === '/library') return path === '/' || path.startsWith('/reader');
		return path.startsWith(p);
	};
</script>

<div class="min-h-screen bg-surface">
	<header class="fixed top-0 w-full z-50 bg-surface shadow-[0_1px_4px_rgba(0,31,63,0.05)] flex justify-between items-center px-margin-mobile h-16">
		<div class="flex items-center gap-3 active:scale-95 transition-transform duration-200 cursor-pointer">
			<div class="w-8 h-8 rounded-full overflow-hidden border border-outline/10 bg-surface-container" />
			<h1 class="font-display text-display-mobile text-primary">Book Vault</h1>
		</div>
		<button class="text-primary hover:opacity-80 transition-opacity active:scale-95">
			<Bell size={20} />
		</button>
	</header>

	<main class="pt-24 pb-32 max-w-container-max mx-auto px-margin-mobile md:px-margin-desktop">
		{@render children()}
	</main>

	<nav class="fixed bottom-0 w-full z-50 bg-surface shadow-[0_-4px_16px_rgba(0,31,63,0.02)] flex justify-around items-center h-20 px-margin-mobile">
		<a
			href="/"
			class={['flex flex-col items-center justify-center transition-colors active:scale-90 px-4 py-1', isCurrent('/library') ? 'text-secondary bg-secondary-container/10 rounded-full' : 'text-on-surface-variant opacity-70']}
		>
			<BookOpen size={20} />
			<span class="font-label text-label-sm mt-1">Library</span>
		</a>
		<a
			href="/search"
			class={['flex flex-col items-center justify-center transition-colors active:scale-90 px-4 py-1', isCurrent('/search') ? 'text-secondary bg-secondary-container/10 rounded-full' : 'text-on-surface-variant opacity-70']}
		>
			<Search size={20} />
			<span class="font-label text-label-sm mt-1">Search</span>
		</a>
		<a
			href="/studio"
			class={['flex flex-col items-center justify-center transition-colors active:scale-90 px-4 py-1', isCurrent('/studio') ? 'text-secondary bg-secondary-container/10 rounded-full' : 'text-on-surface-variant opacity-70']}
		>
			<PenSquare size={20} />
			<span class="font-label text-label-sm mt-1">Studio</span>
		</a>
		<a
			href="/notes"
			class="flex flex-col items-center justify-center transition-colors active:scale-90 px-4 py-1 text-on-surface-variant opacity-70"
		>
			<ScrollText size={20} />
			<span class="font-label text-label-sm mt-1">Notes</span>
		</a>
	</nav>
</div>
