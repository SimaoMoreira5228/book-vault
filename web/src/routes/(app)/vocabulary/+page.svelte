<script lang="ts">
	import * as m from "$lib/paraglide/messages";
	import { api, authState } from "$lib/api/client.svelte";
	import { goto } from "$app/navigation";
	import { resolve } from "$app/paths";
	import Trash2 from "@lucide/svelte/icons/trash-2";
	import GraduationCap from "@lucide/svelte/icons/graduation-cap";
	import Search from "@lucide/svelte/icons/search";

	type VocabEntry = {
		id: string;
		user_id: string;
		language: string;
		lemma: string;
		state: string;
		first_seen_at: string;
		sentence_snippet: string | null;
	};

	let entries = $state<VocabEntry[]>([]);
	let loading = $state(true);
	let error = $state("");
	let filterLang = $state("");
	let filterState = $state("");

	$effect(() => {
		if (!authState.isAuthenticated) {
			goto(resolve("/login"));
			return;
		}
		loadVocab();
	});

	async function loadVocab() {
		loading = true;
		error = "";
		const params: { language?: string; state?: string } = {};
		if (filterLang) params.language = filterLang;
		if (filterState) params.state = filterState;
		const r = await api.vocabulary.list(Object.keys(params).length ? params : undefined);
		if (r.isOk()) entries = r.value as unknown as VocabEntry[];
		else error = r.error.message;
		loading = false;
	}

	async function handleStateChange(id: string, state: string) {
		await api.vocabulary.update(id, { state });
		entries = entries.map((e) => (e.id === id ? { ...e, state } : e));
	}

	async function handleDelete(id: string) {
		await api.vocabulary.delete(id);
		entries = entries.filter((e) => e.id !== id);
	}

	function stateBadge(state: string): string {
		const map: Record<string, string> = {
			unknown: m.vocab_state_unknown(),
			learning: m.vocab_state_learning(),
			known: m.vocab_state_known()
		};
		return map[state] ?? state;
	}

	function stateColor(state: string): string {
		const map: Record<string, string> = {
			unknown: "bg-secondary/10 text-secondary",
			learning: "bg-amber-100 text-amber-700 dark:bg-amber-900/40 dark:text-amber-300",
			known: "bg-green-100 text-green-700 dark:bg-green-900/40 dark:text-green-300"
		};
		return map[state] ?? "";
	}

	const languages = $derived([...new Set(entries.map((e) => e.language))].sort());
</script>

<svelte:head><title>{m.vocab_title()} — {m.app_name()}</title></svelte:head>

<section>
	<header class="mb-8">
		<span class="font-label text-label-sm text-secondary mb-2 block tracking-widest uppercase"
			>{m.vocab_subtitle()}</span
		>
		<h2 class="font-display text-headline-md">{m.vocab_title()}</h2>
	</header>

	<div class="border-outline/10 mb-8 flex flex-wrap items-center gap-3 rounded-xl border p-3">
		<div class="flex items-center gap-2">
			<Search size={16} class="text-on-surface-variant" />
			<select
				bind:value={filterLang}
				onchange={loadVocab}
				class="font-label text-label-sm text-on-surface-variant border-0 bg-transparent outline-none"
			>
				<option value="">{m.vocab_all_languages()}</option>
				{#each languages as lang (lang)}
					<option value={lang}>{lang}</option>
				{/each}
			</select>
		</div>
		<div class="flex gap-1 rounded-lg border border-[rgba(0,31,63,0.08)] p-0.5">
			<button
				onclick={() => {
					filterState = "";
					loadVocab();
				}}
				class={[
					"font-label text-label-sm rounded-md px-3 py-1.5 transition-all",
					!filterState ? "bg-primary text-white" : "text-on-surface-variant hover:text-primary"
				]}>{m.vocab_all()}</button
			>
			<button
				onclick={() => {
					filterState = "unknown";
					loadVocab();
				}}
				class={[
					"font-label text-label-sm rounded-md px-3 py-1.5 transition-all",
					filterState === "unknown"
						? "bg-primary text-white"
						: "text-on-surface-variant hover:text-primary"
				]}>{m.vocab_state_unknown()}</button
			>
			<button
				onclick={() => {
					filterState = "learning";
					loadVocab();
				}}
				class={[
					"font-label text-label-sm rounded-md px-3 py-1.5 transition-all",
					filterState === "learning"
						? "bg-primary text-white"
						: "text-on-surface-variant hover:text-primary"
				]}>{m.vocab_state_learning()}</button
			>
			<button
				onclick={() => {
					filterState = "known";
					loadVocab();
				}}
				class={[
					"font-label text-label-sm rounded-md px-3 py-1.5 transition-all",
					filterState === "known"
						? "bg-primary text-white"
						: "text-on-surface-variant hover:text-primary"
				]}>{m.vocab_state_known()}</button
			>
		</div>
	</div>

	{#if loading}
		<div class="flex items-center justify-center py-32">
			<div
				class="border-secondary h-8 w-8 animate-spin rounded-full border-2 border-t-transparent"
			></div>
		</div>
	{:else if error}
		<div class="font-label text-label-sm text-error bg-error-container/20 rounded-lg px-4 py-3">
			{error}
		</div>
	{:else if entries.length === 0}
		<div class="paper-card rounded-xl p-16 text-center">
			<GraduationCap size={40} class="text-on-surface-variant/30 mb-4 block" />
			<p class="font-body text-body-md text-on-surface-variant">{m.vocab_empty()}</p>
		</div>
	{:else}
		<div class="space-y-2">
			{#each entries as entry (entry.id)}
				<div
					class="paper-card flex items-center gap-4 rounded-xl p-4 transition-all hover:shadow-md"
				>
					<div class="min-w-0 flex-1">
						<div class="flex items-center gap-2">
							<span class="font-display text-headline-sm">{entry.lemma}</span>
							<span
								class={[
									"font-label rounded-full px-2 py-0.5 text-[10px] tracking-wider uppercase",
									stateColor(entry.state)
								]}>{stateBadge(entry.state)}</span
							>
							<span class="font-label text-label-sm text-on-surface-variant/50"
								>[{entry.language}]</span
							>
						</div>
						{#if entry.sentence_snippet}
							<p class="font-body text-body-md text-on-surface-variant mt-1 italic">
								"{entry.sentence_snippet}"
							</p>
						{/if}
					</div>
					<div class="flex items-center gap-2">
						{#if entry.state !== "learning"}
							<button
								onclick={() => handleStateChange(entry.id, "learning")}
								class="font-label text-label-sm text-secondary hover:text-secondary/80 transition-colors"
								>{m.vocab_mark_learning()}</button
							>
						{/if}
						{#if entry.state !== "known"}
							<button
								onclick={() => handleStateChange(entry.id, "known")}
								class="font-label text-label-sm text-green-600 transition-colors hover:text-green-700"
								>{m.vocab_mark_known()}</button
							>
						{/if}
						<button
							onclick={() => handleDelete(entry.id)}
							class="text-on-surface-variant/30 hover:text-error p-1 transition-colors"
						>
							<Trash2 size={14} />
						</button>
					</div>
				</div>
			{/each}
		</div>
	{/if}
</section>
