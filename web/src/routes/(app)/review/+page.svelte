<script lang="ts">
	import * as m from "$lib/paraglide/messages";
	import { api, authState } from "$lib/api/client.svelte";
	import { goto } from "$app/navigation";
	import { resolve } from "$app/paths";
	import GraduationCap from "@lucide/svelte/icons/graduation-cap";
	import RefreshCw from "@lucide/svelte/icons/refresh-cw";

	type ReviewCard = {
		id: string;
		lemma: string;
		language: string;
		sense_label: string | null;
		definition: string | null;
		context_sentence: string | null;
	};

	let cards = $state<ReviewCard[]>([]);
	let index = $state(0);
	let loading = $state(true);
	let flipped = $state(false);
	let finished = $state(false);
	let empty = $state(false);

	let current = $derived(cards[index] ?? null);

	$effect(() => {
		if (!authState.isAuthenticated) {
			goto(resolve("/login"));
			return;
		}
		loadCards();
	});

	async function loadCards() {
		loading = true;
		const r = await api.vocabulary.reviewCards({ due: true });
		if (r.isOk()) {
			const data = r.value as unknown as ReviewCard[];
			cards = data;
			if (data.length === 0) empty = true;
			index = 0;
			flipped = false;
		}
		loading = false;
	}

	async function submitReview(quality: number) {
		if (!current) return;
		await api.vocabulary.submitReview(current.id, quality);
		if (index < cards.length - 1) {
			index++;
			flipped = false;
		} else {
			finished = true;
		}
	}
</script>

<svelte:head><title>{m.review_title()} — {m.app_name()}</title></svelte:head>

<section class="mx-auto max-w-lg">
	<header class="mb-8 text-center">
		<span class="font-label text-label-sm text-secondary mb-2 block tracking-widest uppercase"
			>{m.review_subtitle()}</span
		>
		<h2 class="font-display text-headline-md">{m.review_title()}</h2>
	</header>

	{#if loading}
		<div class="flex items-center justify-center py-32">
			<div
				class="border-secondary h-8 w-8 animate-spin rounded-full border-2 border-t-transparent"
			></div>
		</div>
	{:else if empty}
		<div class="paper-card rounded-xl p-16 text-center">
			<GraduationCap size={48} class="text-on-surface-variant/20 mx-auto mb-4 block" />
			<p class="font-body text-body-md text-on-surface-variant mb-6">{m.review_empty()}</p>
			<a href={resolve("/vocabulary")} class="font-label text-label-md text-secondary underline"
				>{m.vocab_title()}</a
			>
		</div>
	{:else if finished}
		<div class="paper-card rounded-xl p-16 text-center">
			<GraduationCap size={48} class="text-secondary mx-auto mb-4 block" />
			<p class="font-display text-headline-sm mb-2">{m.review_done()}</p>
			<p class="font-body text-body-md text-on-surface-variant mb-6">
				{m.review_done_desc({ count: cards.length })}
			</p>
			<button onclick={loadCards} class="btn-primary"
				><RefreshCw size={16} />{m.review_again()}</button
			>
		</div>
	{:else if current}
		<div class="paper-card rounded-xl p-8 md:p-12">
			<div class="mb-2 flex items-center justify-between">
				<span class="font-label text-label-sm text-on-surface-variant/50"
					>{index + 1} / {cards.length}</span
				>
				<span class="font-label text-label-sm text-on-surface-variant/50">[{current.language}]</span
				>
			</div>

			<div class="py-8 text-center">
				<h3 class="font-display text-display-mobile mb-2">{current.lemma}</h3>
				{#if current.sense_label}
					<span class="font-label text-label-sm text-secondary">({current.sense_label})</span>
				{/if}
			</div>

			{#if flipped}
				<div class="border-outline-variant/30 mb-8 border-t pt-6">
					{#if current.definition}
						<p class="font-body text-body-lg text-on-surface-variant mb-4">{current.definition}</p>
					{/if}
					{#if current.context_sentence}
						<p class="font-body text-body-md text-on-surface-variant/60 italic">
							"{current.context_sentence}"
						</p>
					{/if}
				</div>

				<div class="flex flex-wrap justify-center gap-2">
					<button
						onclick={() => submitReview(1)}
						class="font-label text-label-sm rounded-xl bg-red-100 px-5 py-3 text-red-700 transition-colors hover:bg-red-200 dark:bg-red-900/40 dark:text-red-300"
						>{m.review_again_btn()}</button
					>
					<button
						onclick={() => submitReview(3)}
						class="font-label text-label-sm rounded-xl bg-amber-100 px-5 py-3 text-amber-700 transition-colors hover:bg-amber-200 dark:bg-amber-900/40 dark:text-amber-300"
						>{m.review_hard()}</button
					>
					<button
						onclick={() => submitReview(4)}
						class="font-label text-label-sm rounded-xl bg-green-100 px-5 py-3 text-green-700 transition-colors hover:bg-green-200 dark:bg-green-900/40 dark:text-green-300"
						>{m.review_good()}</button
					>
					<button
						onclick={() => submitReview(5)}
						class="font-label text-label-sm bg-primary rounded-xl px-5 py-3 text-white transition-colors"
						>{m.review_easy()}</button
					>
				</div>
			{:else}
				<div class="text-center">
					<button onclick={() => (flipped = true)} class="btn-primary">{m.review_show()}</button>
				</div>
			{/if}
		</div>
	{/if}
</section>
