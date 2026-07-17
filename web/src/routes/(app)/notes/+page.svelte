<script lang="ts">
	import * as m from "$lib/paraglide/messages";
	import { api, authState } from "$lib/api/client";
	import { goto } from "$app/navigation";
	import ScrollText from "@lucide/svelte/icons/scroll-text";
	import BookOpen from "@lucide/svelte/icons/book-open";
	import Highlighter from "@lucide/svelte/icons/highlighter";
	import MessageSquareText from "@lucide/svelte/icons/message-square-text";
	import Trash2 from "@lucide/svelte/icons/trash-2";
	import ExternalLink from "@lucide/svelte/icons/external-link";

	type Annotation = {
		id: string;
		book_id: string;
		section_id: string;
		block_index: number;
		start_offset: number;
		end_offset: number;
		color: string | null;
		note: string | null;
		created_at: string;
		updated_at: string;
	};

	type BookMeta = {
		id: string;
		title: string;
		author: string | null;
	};

	let annotations = $state<Annotation[]>([]);
	let books = $state<Map<string, BookMeta>>(new Map());
	let loading = $state(true);
	let error = $state("");
	let selectedBook = $state<string | null>(null);
	let deleting = $state<string | null>(null);

	let filteredAnnotations = $derived(
		selectedBook ? annotations.filter((a) => a.book_id === selectedBook) : annotations
	);

	$effect(() => {
		if (!authState.isAuthenticated) {
			goto("/login");
			return;
		}
		loadData();
	});

	async function loadData() {
		loading = true;
		error = "";

		const [booksResult, annotationsResult] = await Promise.all([
			api.books.list(),
			api.annotations.listAll()
		]);

		if (booksResult.isOk()) {
			books = new Map(
				booksResult.value.books.map((b) => [b.id, { id: b.id, title: b.title, author: b.author }])
			);
		}
		if (annotationsResult.isOk()) {
			annotations = annotationsResult.value as unknown as Annotation[];
		} else {
			error = annotationsResult.error.message;
		}
		loading = false;
	}

	async function handleDelete(annotationId: string) {
		deleting = annotationId;
		const result = await api.annotations.delete(annotationId);
		if (result.isOk()) {
			annotations = annotations.filter((a) => a.id !== annotationId);
		}
		deleting = null;
	}

	function colorClass(color: string | null): string {
		switch (color) {
			case "yellow":
				return "bg-yellow-200/60";
			case "green":
				return "bg-green-200/60";
			case "blue":
				return "bg-blue-200/60";
			case "pink":
				return "bg-pink-200/60";
			case "orange":
				return "bg-orange-200/60";
			default:
				return "bg-yellow-200/60";
		}
	}

	function formatDate(date: string): string {
		return date.split("T")[0];
	}

	const uniqueBooks = $derived(
		[...new Set(annotations.map((a) => a.book_id))].map(
			(id) => books.get(id) ?? { id, title: "Unknown Book", author: null }
		)
	);
</script>

<svelte:head>
	<title>{m.notes_title()} — Book Vault</title>
</svelte:head>

<section>
	<header class="mb-8">
		<span class="font-label text-label-sm text-secondary mb-2 block tracking-widest uppercase"
			>{m.notes_subtitle()}</span
		>
		<h2 class="font-display text-headline-md">{m.notes_title()}</h2>
	</header>

	{#if error}
		<div
			class="font-label text-label-sm text-error bg-error-container/20 mb-8 rounded-lg px-4 py-3"
		>
			{error}
		</div>
	{/if}

	{#if loading}
		<div class="flex items-center justify-center py-32">
			<div
				class="border-secondary h-8 w-8 animate-spin rounded-full border-2 border-t-transparent"
			></div>
		</div>
	{:else if annotations.length === 0}
		<div class="paper-card rounded-xl p-16 text-center">
			<ScrollText size={40} class="text-on-surface-variant/30 mb-4 block" />
			<p class="font-body text-body-md text-on-surface-variant">{m.notes_empty()}</p>
		</div>
	{:else}
		<!-- Book filter tabs -->
		<div class="border-outline/10 mb-8 flex flex-wrap gap-2 rounded-xl border p-2">
			<button
				onclick={() => (selectedBook = null)}
				class={[
					"font-label rounded-lg px-4 py-2 text-sm transition-all",
					selectedBook === null
						? "bg-primary text-white shadow-sm"
						: "text-on-surface-variant hover:text-primary"
				]}
			>
				All ({annotations.length})
			</button>
			{#each uniqueBooks as bm (bm.id)}
				<button
					onclick={() => (selectedBook = bm.id)}
					class={[
						"font-label rounded-lg px-4 py-2 text-sm transition-all",
						selectedBook === bm.id
							? "bg-primary text-white shadow-sm"
							: "text-on-surface-variant hover:text-primary"
					]}
				>
					{bm.title} ({annotations.filter((a) => a.book_id === bm.id).length})
				</button>
			{/each}
		</div>

		<!-- Annotations list -->
		<div class="space-y-4">
			{#each filteredAnnotations as annotation (annotation.id)}
				{@const bookMeta = books.get(annotation.book_id)}
				<div class="paper-card rounded-xl p-6">
					<div class="mb-3 flex items-start justify-between gap-4">
						<div class="min-w-0 flex-1">
							<a
								href="/reader/{annotation.book_id}"
								class="font-label text-label-md text-secondary hover:text-secondary/80 mb-1 flex items-center gap-1.5 transition-colors"
							>
								<BookOpen size={14} />
								{bookMeta?.title ?? "Unknown Book"}
								<ExternalLink size={12} class="opacity-50" />
							</a>
						</div>
						<button
							onclick={() => handleDelete(annotation.id)}
							disabled={deleting === annotation.id}
							class="text-on-surface-variant/50 hover:text-error shrink-0 p-1 transition-colors disabled:opacity-30"
						>
							<Trash2 size={14} />
						</button>
					</div>

					<div class="mb-3 flex items-center gap-3">
						<div class={["h-5 w-1 rounded-full", colorClass(annotation.color)]}></div>
						<span class="font-label text-label-sm text-on-surface-variant">
							{formatDate(annotation.created_at)}
						</span>
					</div>

					{#if annotation.note}
						<div class="bg-surface-container-low mb-3 flex items-start gap-3 rounded-lg p-4">
							<MessageSquareText size={16} class="text-on-surface-variant/50 mt-0.5 shrink-0" />
							<p class="font-body text-body-md text-primary">{annotation.note}</p>
						</div>
					{/if}

					<div class={["inline rounded-lg px-3 py-1 text-sm", colorClass(annotation.color)]}>
						<Highlighter size={12} class="mr-1 inline" />
						Highlight
					</div>
				</div>
			{/each}
		</div>
	{/if}
</section>
