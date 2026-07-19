<script lang="ts">
	import * as m from "$lib/paraglide/messages";
	import type { BookResponse, ProspectiveMetadata } from "$lib/api/generated";
	import { api } from "$lib/api/client.svelte";
	import Search from "@lucide/svelte/icons/search";
	import RefreshCw from "@lucide/svelte/icons/refresh-cw";
	import Lock from "@lucide/svelte/icons/lock";
	import Unlock from "@lucide/svelte/icons/unlock";

	let {
		book,
		onRefresh
	}: {
		book: BookResponse;
		onRefresh: () => void;
	} = $props();

	let candidates = $state<ProspectiveMetadata[]>([]);
	let searching = $state(false);
	let confirming = $state(false);
	let selectedCandidate = $state<number | null>(null);
	let searchingMeta = $state(false);
	let meta = $state<Record<string, unknown> | null>(null);

	let loaded = $state(false);
	let fieldLabels: Record<string, string> = {
		Title: m.book_detail_field_title(),
		Author: m.book_detail_field_author(),
		Description: m.metadata_field_description(),
		Cover: m.metadata_field_cover(),
		Genres: m.metadata_field_genres(),
		PageCount: m.book_detail_field_page_count(),
		Isbn: m.book_detail_field_isbn(),
		Publisher: m.book_detail_field_publisher(),
		PublishedDate: m.metadata_field_published_date(),
		Subtitle: m.metadata_field_subtitle()
	};

	const metadataFields = Object.keys(fieldLabels);

	$effect(() => {
		if (!loaded) {
			loaded = true;
			loadMeta();
		}
	});

	async function loadMeta() {
		const r = await api.metadata.get(book.id);
		if (r.isOk()) meta = r.value;
	}

	async function handleSearchCandidates() {
		searching = true;
		const r = await api.metadata.candidates(book.id, {
			title: book.title,
			author: book.author ?? undefined
		});
		if (r.isOk()) candidates = r.value;
		searching = false;
	}

	async function handleConfirmCandidate(index: number) {
		confirming = true;
		selectedCandidate = index;
		const r = await api.metadata.confirm(book.id, candidates[index]);
		if (r.isOk()) {
			meta = r.value;
			candidates = [];
			selectedCandidate = null;
			onRefresh();
		} else {
			selectedCandidate = null;
		}
		confirming = false;
	}

	async function handleRefresh() {
		searchingMeta = true;
		const r = await api.metadata.refresh(book.id);
		if (r.isOk()) meta = r.value;
		searchingMeta = false;
	}

	async function handleLockField(field: string) {
		const r = await api.metadata.lockField(book.id, field);
		if (r.isOk()) meta = r.value;
	}

	async function handleUnlockField(field: string) {
		const r = await api.metadata.unlockField(book.id, field);
		if (r.isOk()) meta = r.value;
	}

	function formatDate(d: string | undefined): string {
		return d ? d.split("T")[0] : "—";
	}

	const lockedFields: string[] = $derived((meta?.locked_fields as string[]) ?? []);
	const providerIds = $derived((meta?.provider_ids as Record<string, string>) ?? {});
</script>

<div class="paper-card rounded-xl p-8">
	<div class="mb-6 flex items-center justify-between">
		<h3 class="font-display text-headline-sm text-primary">{m.metadata_title()}</h3>
		<div class="flex gap-2">
			<button
				onclick={handleSearchCandidates}
				disabled={searching}
				class="font-label text-label-md text-secondary hover:text-secondary/80 inline-flex items-center gap-1.5 transition-colors"
			>
				<Search size={16} />{searching ? m.metadata_searching() : m.metadata_search()}
			</button>
			<button
				onclick={handleRefresh}
				disabled={searchingMeta}
				class="font-label text-label-md text-on-surface-variant hover:text-primary inline-flex items-center gap-1.5 transition-colors"
			>
				<RefreshCw size={14} class={searchingMeta ? "animate-spin" : ""} />
				{searchingMeta ? m.metadata_refreshing() : m.metadata_refresh()}
			</button>
		</div>
	</div>

	{#if candidates.length > 0}
		<div class="mb-6 space-y-3">
			<p class="font-label text-label-sm text-on-surface-variant mb-2 tracking-widest uppercase">
				{m.metadata_field_candidates()}
			</p>
			{#each candidates as candidate, i (candidate.provider + candidate.provider_id)}
				<div class="bg-surface-container-low rounded-xl p-4">
					<div class="mb-2 flex items-start justify-between gap-4">
						<div>
							<p class="font-display text-headline-sm mb-1">{candidate.title ?? "Unknown"}</p>
							<p class="font-body text-body-md text-on-surface-variant">
								{candidate.authors.join(", ") || "—"}
							</p>
							<p class="font-label text-label-sm text-secondary mt-1">
								{m.metadata_candidate_from({ provider: candidate.provider })}
							</p>
						</div>
						<button
							onclick={() => handleConfirmCandidate(i)}
							disabled={confirming && selectedCandidate === i}
							class="btn-primary text-label-sm shrink-0"
						>
							{confirming && selectedCandidate === i ? "..." : m.metadata_confirm()}
						</button>
					</div>
					{#if candidate.description}<p
							class="font-body text-body-md text-on-surface-variant mt-2 line-clamp-3"
						>
							{candidate.description}
						</p>{/if}
				</div>
			{/each}
		</div>
	{/if}

	{#if Object.keys(providerIds).length > 0}
		<div class="mb-6">
			<p class="font-label text-label-sm text-on-surface-variant mb-2 tracking-widest uppercase">
				{m.metadata_provider_ids()}
			</p>
			<div class="flex flex-wrap gap-2">
				{#each Object.entries(providerIds) as [provider, id] (provider)}
					<span class="font-label text-label-sm bg-surface-container-high rounded-lg px-3 py-1.5"
						>{provider}: <span class="text-secondary">{id as string}</span></span
					>
				{/each}
			</div>
		</div>
	{/if}

	<div class="mb-6">
		<p class="font-label text-label-sm text-on-surface-variant tracking-widest uppercase">
			{m.metadata_last_refreshed()}: {meta?.last_refreshed_at
				? formatDate(meta.last_refreshed_at as string)
				: m.metadata_never()}
		</p>
	</div>

	<div>
		<p class="font-label text-label-sm text-on-surface-variant mb-3 tracking-widest uppercase">
			{m.metadata_field_locks()}
		</p>
		<div class="flex flex-wrap gap-2">
			{#each metadataFields as field (field)}
				{@const locked = lockedFields.includes(field)}
				<button
					onclick={() => (locked ? handleUnlockField(field) : handleLockField(field))}
					class={[
						"font-label text-label-sm inline-flex items-center gap-1.5 rounded-lg border px-3 py-1.5 transition-all",
						locked
							? "bg-secondary/5 border-secondary/20 text-secondary"
							: "text-on-surface-variant border-[rgba(0,31,63,0.08)] hover:border-[rgba(0,31,63,0.2)]"
					]}
				>
					{#if locked}<Lock size={12} />{:else}<Unlock size={12} />{/if}
					{fieldLabels[field]}
				</button>
			{/each}
		</div>
	</div>
</div>
