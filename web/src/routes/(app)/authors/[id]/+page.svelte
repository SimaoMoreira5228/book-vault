<script lang="ts">
	import * as m from "$lib/paraglide/messages";
	import { api, authState } from "$lib/api/client.svelte";
	import { goto } from "$app/navigation";
	import { page } from "$app/state";
	import BookCover from "$lib/components/BookCover.svelte";
	import ArrowLeft from "@lucide/svelte/icons/arrow-left";
	import UserPen from "@lucide/svelte/icons/user-pen";
	import BookOpen from "@lucide/svelte/icons/book-open";

	type AuthorDetail = {
		id: string;
		name: string;
		sort_name: string | null;
		bio: string | null;
		birth_date: string | null;
		death_date: string | null;
		book_count: number;
	};

	type Book = {
		id: string;
		title: string;
		author: string | null;
		read_status: string;
	};

	let author = $state<AuthorDetail | null>(null);
	let books = $state<Book[]>([]);
	let loading = $state(true);
	let error = $state("");

	$effect(() => {
		if (!authState.isAuthenticated) {
			goto("/login");
			return;
		}
		if (page.params.id) loadAuthor();
	});

	async function loadAuthor() {
		loading = true;
		error = "";
		const id = page.params.id!;
		const [authorR, booksR] = await Promise.all([api.authors.get(id), api.books.list()]);

		if (authorR.isErr()) {
			error = authorR.error.message;
			loading = false;
			return;
		}
		author = authorR.value as unknown as AuthorDetail;

		if (booksR.isOk()) {
			const all = booksR.value.books;
			books = all.filter(
				(b: { author_id: string | null }) => b.author_id === id
			) as unknown as Book[];
		}
		loading = false;
	}
</script>

<svelte:head>
	<title>{author?.name ?? "Author"} — Book Vault</title>
</svelte:head>

<section>
	<div class="mb-6">
		<a
			href="/authors"
			class="font-label text-label-md text-on-surface-variant hover:text-primary inline-flex items-center gap-1.5 transition-colors"
		>
			<ArrowLeft size={16} />{m.authors_title()}
		</a>
	</div>

	{#if loading}
		<div class="flex items-center justify-center py-32">
			<div
				class="border-secondary h-8 w-8 animate-spin rounded-full border-2 border-t-transparent"
			></div>
		</div>
	{:else if !author}
		<div class="paper-card rounded-xl p-16 text-center">
			<p class="font-body text-body-md text-on-surface-variant">{error || "Author not found"}</p>
		</div>
	{:else}
		<div class="mb-10 flex items-start gap-6">
			<div class="bg-surface-container flex h-20 w-20 items-center justify-center rounded-2xl">
				<UserPen size={36} class="text-secondary" />
			</div>
			<div class="pt-2">
				<h1 class="font-display text-headline-md text-primary mb-2">{author.name}</h1>
				<p class="font-label text-label-sm text-on-surface-variant">
					{m.authors_book_count({ count: author.book_count })}
					{#if author.birth_date}
						· {author.birth_date}{/if}
					{#if author.death_date}
						— {author.death_date}{/if}
				</p>
				{#if author.bio}
					<p class="font-body text-body-md text-on-surface-variant mt-4 max-w-2xl">{author.bio}</p>
				{/if}
			</div>
		</div>

		<h3 class="font-display text-headline-sm text-primary mb-6">
			{m.authors_books({ name: author.name })}
		</h3>

		{#if books.length === 0}
			<div class="paper-card rounded-xl p-12 text-center">
				<BookOpen size={32} class="text-on-surface-variant/20 mb-4 block" />
				<p class="font-body text-body-md text-on-surface-variant">{m.shelf_detail_empty()}</p>
			</div>
		{:else}
			<div class="space-y-3">
				{#each books as book (book.id)}
					<a
						href="/reader/{book.id}"
						class="paper-card flex items-center gap-5 rounded-xl p-5 transition-all hover:shadow-md"
					>
						<BookCover bookId={book.id} class="h-14 w-10 shrink-0 rounded-lg" />
						<div class="min-w-0 flex-1">
							<p class="font-display text-headline-sm truncate">{book.title}</p>
							<p class="font-label text-label-sm text-on-surface-variant">{book.read_status}</p>
						</div>
					</a>
				{/each}
			</div>
		{/if}
	{/if}
</section>
