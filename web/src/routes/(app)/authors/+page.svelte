<script lang="ts">
	import * as m from "$lib/paraglide/messages";
	import { api, authState } from "$lib/api/client";
	import { goto } from "$app/navigation";
	import UserPen from "@lucide/svelte/icons/user-pen";
	import BookOpen from "@lucide/svelte/icons/book-open";
	import Plus from "@lucide/svelte/icons/plus";

	type Author = {
		id: string;
		name: string;
		sort_name: string | null;
		book_count: number;
	};

	let authors = $state<Author[]>([]);
	let loading = $state(true);

	$effect(() => {
		if (!authState.isAuthenticated) {
			goto("/login");
			return;
		}
		loadAuthors();
	});

	async function loadAuthors() {
		loading = true;
		const r = await api.authors.list();
		if (r.isOk()) authors = r.value as unknown as Author[];
		loading = false;
	}
</script>

<svelte:head>
	<title>{m.authors_title()} — Book Vault</title>
</svelte:head>

<section>
	<header class="mb-8 flex items-end justify-between">
		<div>
			<span class="font-label text-label-sm text-secondary mb-2 block tracking-widest uppercase"
				>{m.authors_subtitle()}</span
			>
			<h2 class="font-display text-headline-md">{m.authors_title()}</h2>
		</div>
	</header>

	{#if loading}
		<div class="flex items-center justify-center py-32">
			<div
				class="border-secondary h-8 w-8 animate-spin rounded-full border-2 border-t-transparent"
			></div>
		</div>
	{:else if authors.length === 0}
		<div class="paper-card rounded-xl p-16 text-center">
			<UserPen size={40} class="text-on-surface-variant/20 mb-4 block" />
			<p class="font-body text-body-md text-on-surface-variant">{m.authors_empty()}</p>
		</div>
	{:else}
		<div class="gap-gutter grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3">
			{#each authors as author (author.id)}
				<a
					href="/authors/{author.id}"
					class="paper-card group rounded-xl p-6 transition-all hover:shadow-lg"
				>
					<div class="mb-4 flex items-start gap-4">
						<div class="bg-surface-container flex h-10 w-10 items-center justify-center rounded-lg">
							<UserPen size={18} class="text-secondary" />
						</div>
						<div class="min-w-0 flex-1">
							<h3 class="font-display text-headline-sm truncate">{author.name}</h3>
							<p class="font-label text-label-sm text-on-surface-variant">
								{m.authors_book_count({ count: author.book_count })}
							</p>
						</div>
					</div>
				</a>
			{/each}
		</div>
	{/if}
</section>
