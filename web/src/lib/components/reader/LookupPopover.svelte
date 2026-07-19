<script lang="ts">
	import * as m from "$lib/paraglide/messages";
	import { api } from "$lib/api/client.svelte";
	import Popup from "$lib/components/Popup.svelte";
	import BookOpen from "@lucide/svelte/icons/book-open";
	import LoaderCircle from "@lucide/svelte/icons/loader-circle";
	import Plus from "@lucide/svelte/icons/plus";
	import Check from "@lucide/svelte/icons/check";

	type DictEntry = {
		word: string;
		lemma: string;
		sense_label: string | null;
		part_of_speech: string | null;
		definition: string;
		example_sentences: string[];
		pronunciation: string | null;
		frequency_rank: number | null;
	};

	type LookupResponse = {
		entries: DictEntry[];
		cached: boolean;
		translation: string | null;
	};

	let {
		show = $bindable(false),
		x = 0,
		y = 0,
		text = "",
		context = "",
		language = "en",
		definitionLanguage = ""
	}: {
		show?: boolean;
		x?: number;
		y?: number;
		text?: string;
		context?: string;
		language?: string;
		definitionLanguage?: string;
	} = $props();

	let entries = $state<DictEntry[]>([]);
	let translation = $state<string | null>(null);
	let loading = $state(false);
	let added = $state<string | null>(null);
	let err = $state("");

	$effect(() => {
		if (!show || !text.trim()) return;
		lookup();
	});

	async function lookup() {
		loading = true;
		err = "";
		entries = [];
		translation = null;
		const r = await api.vocabulary.lookup({
			word: text,
			context,
			language,
			definition_language: definitionLanguage || undefined
		});
		if (r.isOk()) {
			const data = r.value as unknown as LookupResponse;
			entries = data.entries;
			translation = data.translation ?? null;
			if (entries.length === 0) err = m.vocab_no_definitions();
		} else {
			err = r.error.message;
		}
		loading = false;
	}

	async function addToVocab(entry: DictEntry) {
		added = entry.lemma + (entry.sense_label ?? "");
		await api.vocabulary.add({
			language,
			lemma: entry.lemma,
			sense_label: entry.sense_label ?? undefined,
			definition: entry.definition,
			context_sentence: context || undefined,
			source: "reader"
		});
	}

	function posColor(pos: string | null): string {
		const map: Record<string, string> = {
			noun: "bg-blue-100 text-blue-700 dark:bg-blue-900/40 dark:text-blue-300",
			verb: "bg-green-100 text-green-700 dark:bg-green-900/40 dark:text-green-300",
			adjective: "bg-purple-100 text-purple-700 dark:bg-purple-900/40 dark:text-purple-300",
			adverb: "bg-orange-100 text-orange-700 dark:bg-orange-900/40 dark:text-orange-300"
		};
		return map[pos ?? ""] ?? "bg-surface-container-high text-on-surface-variant";
	}
</script>

<Popup bind:show {x} {y} position="top">
	<div class="bg-surface border-outline/10 max-w-sm min-w-[260px] rounded-xl border p-3 shadow-lg">
		{#if loading}
			<div class="flex items-center justify-center gap-2 py-4">
				<LoaderCircle size={16} class="animate-spin" />
				<span class="font-label text-label-sm text-on-surface-variant">{m.vocab_looking_up()}</span>
			</div>
		{:else if err}
			<p class="font-body text-body-sm text-error">{err}</p>
		{:else}
			{#each entries as entry, i (entry.lemma + (entry.sense_label ?? "") + i)}
				<div class="border-outline-variant/30 {i > 0 ? 'mt-3 border-t pt-3' : ''}">
					<div class="mb-1 flex items-center gap-2">
						<span class="font-display text-headline-sm">{entry.word}</span>
						{#if entry.pronunciation}
							<span class="font-label text-label-sm text-on-surface-variant/60"
								>/{entry.pronunciation}/</span
							>
						{/if}
						<span
							class={[
								"font-label rounded-full px-2 py-0.5 text-[10px] tracking-wider uppercase",
								posColor(entry.part_of_speech)
							]}
						>
							{entry.part_of_speech ?? "?"}
						</span>
					</div>
					<p class="font-body text-body-md text-on-surface-variant mb-2">{entry.definition}</p>
					{#if entry.example_sentences.length > 0}
						<p class="font-body text-body-sm text-on-surface-variant/60 mb-2 italic">
							"{entry.example_sentences[0]}"
						</p>
					{/if}
					<div class="flex items-center gap-2">
						{#if added === entry.lemma + (entry.sense_label ?? "")}
							<span class="font-label text-label-sm flex items-center gap-1 text-green-600"
								><Check size={14} />{m.vocab_added()}</span
							>
						{:else}
							<button
								onclick={() => addToVocab(entry)}
								class="font-label text-label-sm text-secondary hover:text-secondary/80 flex items-center gap-1 transition-colors"
							>
								<Plus size={14} />{m.vocab_add()}
							</button>
						{/if}
						{#if entry.frequency_rank}
							<span class="font-label text-label-sm text-on-surface-variant/40 ml-auto"
								>#{entry.frequency_rank}</span
							>
						{/if}
					</div>
				</div>
			{/each}

			{#if translation}
				<div class="border-outline-variant/30 mt-3 border-t pt-3">
					<span class="font-label text-label-sm text-on-surface-variant/60 mb-1 block"
						>Translation</span
					>
					<p class="font-body text-body-md text-on-surface-variant">{translation}</p>
				</div>
			{/if}
		{/if}
	</div>
</Popup>
