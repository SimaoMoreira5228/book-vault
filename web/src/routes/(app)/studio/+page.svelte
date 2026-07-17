<script lang="ts">
	import * as m from "$lib/paraglide/messages";
	import { api } from "$lib/api/client";
	import type { BookResponse } from "$lib/api/generated";
	import Plus from "@lucide/svelte/icons/plus";
	import ChevronDown from "@lucide/svelte/icons/chevron-down";
	import BookOpen from "@lucide/svelte/icons/book-open";
	import Clock from "@lucide/svelte/icons/clock";
	import PenSquare from "@lucide/svelte/icons/pen-square";
	import PlusCircle from "@lucide/svelte/icons/plus-circle";

	let drafts = $state<BookResponse[]>([]);
	let loading = $state(true);

	$effect(() => {
		loadDrafts();
	});

	async function loadDrafts() {
		loading = true;
		const result = await api.books.list();
		if (result.isOk()) {
			drafts = result.value.books.filter((b) => b.format === "native");
		}
		loading = false;
	}

	async function createNew() {
		const result = await api.books.create({
			title: "Untitled Manuscript",
			format: "native",
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
<section class="mb-12 flex flex-col justify-between gap-6 md:flex-row md:items-end">
	<div>
		<p class="font-label text-label-sm text-secondary mb-2 tracking-widest uppercase">
			{m.studio_dashboard()}
		</p>
		<h2 class="font-display text-headline-md text-primary">{m.studio_title()}</h2>
	</div>
	<button onclick={createNew} class="btn-primary">
		<Plus size={20} />
		{m.studio_new_project()}
	</button>
</section>

<!-- Insights Bento Grid -->
<section class="mb-section-gap grid grid-cols-1 gap-6 md:grid-cols-3">
	<div class="border-primary/5 rounded-xl border bg-white p-8 shadow-sm md:col-span-2">
		<div class="mb-8 flex items-start justify-between">
			<h3 class="font-display text-headline-sm text-primary">{m.studio_insights_title()}</h3>
			<div class="text-on-surface-variant font-label text-label-sm flex items-center gap-2">
				<span>{m.studio_insights_week()}</span>
				<ChevronDown size={18} />
			</div>
		</div>
		<div class="flex h-48 items-end justify-between gap-2 px-2 md:gap-4">
			{#each [{ day: "M", height: 40 }, { day: "T", height: 65 }, { day: "W", height: 85 }, { day: "T", height: 95 }, { day: "F", height: 30 }, { day: "S", height: 55 }, { day: "S", height: 20 }] as bar, i (i)}
				<div class="flex flex-1 flex-col items-center">
					<div
						class="bg-surface-container w-full rounded-t-lg transition-all duration-1000"
						style="height: {bar.height}%"
					></div>
					<span class="font-label text-label-sm mt-2 opacity-50">{bar.day}</span>
				</div>
			{/each}
		</div>
	</div>
	<div class="flex flex-col gap-6">
		<div
			class="bg-surface-container-low border-primary/5 flex flex-1 flex-col justify-between rounded-xl border p-6"
		>
			<BookOpen size={20} class="text-secondary" />
			<div>
				<p class="font-display mb-1 text-[32px] leading-none">
					{drafts.length > 0 ? `${drafts.length}` : "0"}
				</p>
				<p class="font-label text-label-sm opacity-60">{m.studio_manuscripts()}</p>
			</div>
		</div>
		<div
			class="bg-primary-container border-primary/5 flex flex-1 flex-col justify-between rounded-xl border p-6 text-white"
		>
			<Clock size={20} class="text-on-primary-container" />
			<div>
				<p class="font-display mb-1 text-[32px] leading-none">--</p>
				<p class="font-label text-label-sm opacity-60">{m.studio_focus_time()}</p>
			</div>
		</div>
	</div>
</section>

<!-- Current Drafts -->
<section class="mb-section-gap">
	<div class="mb-8 flex items-center justify-between">
		<h3 class="font-display text-headline-sm text-primary">{m.studio_drafts_title()}</h3>
	</div>

	{#if loading}
		<div class="flex items-center justify-center py-16">
			<div
				class="border-secondary h-8 w-8 animate-spin rounded-full border-2 border-t-transparent"
			></div>
		</div>
	{:else if drafts.length === 0}
		<div
			class="border-outline-variant/30 hover:bg-surface-container/30 group flex cursor-pointer flex-col items-center justify-center rounded-xl border-2 border-dashed p-16 transition-colors"
			role="button"
			tabindex="0"
			onclick={createNew}
			onkeydown={(e) => {
				if (e.key === "Enter" || e.key === " ") createNew();
			}}
		>
			<div
				class="bg-surface-container mb-4 flex h-16 w-16 items-center justify-center rounded-full transition-transform group-hover:scale-110"
			>
				<PlusCircle size={20} class="text-on-surface-variant" />
			</div>
			<p class="font-display text-on-surface-variant text-[16px]">{m.studio_new_manuscript()}</p>
			<p class="font-label text-label-sm text-on-surface-variant mt-2 text-center opacity-60">
				{m.studio_start_masterpiece()}
			</p>
		</div>
	{:else}
		<div class="grid grid-cols-1 gap-8 md:grid-cols-2 lg:grid-cols-3">
			{#each drafts as draft (draft.id)}
				<a href="/reader/{draft.id}" class="group cursor-pointer">
					<div
						class="bg-surface-container relative mb-4 aspect-[3/4] overflow-hidden rounded-xl shadow-lg transition-transform duration-500 group-hover:-translate-y-2"
					>
						<div class="book-spine-gradient absolute inset-0"></div>
						<div class="flex h-full w-full items-center justify-center">
							<PenSquare size={48} class="text-on-surface-variant/20" />
						</div>
						<div class="absolute right-4 bottom-4">
							<span
								class="font-label text-primary rounded-full bg-white/90 px-3 py-1 text-[10px] tracking-widest uppercase backdrop-blur"
								>{m.studio_draft_badge()}</span
							>
						</div>
					</div>
					<h4 class="font-display mb-1 text-[18px]">{draft.title}</h4>
					<div class="text-on-surface-variant font-label text-label-sm flex items-center gap-4">
						<span class="flex items-center gap-1">
							<Clock size={16} />
							{draft.created_at}
						</span>
					</div>
				</a>
			{/each}
			<div
				class="border-outline-variant/30 hover:bg-surface-container/30 group flex aspect-[3/4] cursor-pointer flex-col items-center justify-center rounded-xl border-2 border-dashed transition-colors"
				role="button"
				tabindex="0"
				onclick={createNew}
				onkeydown={(e) => {
					if (e.key === "Enter" || e.key === " ") createNew();
				}}
			>
				<div
					class="bg-surface-container mb-4 flex h-16 w-16 items-center justify-center rounded-full transition-transform group-hover:scale-110"
				>
					<PlusCircle size={20} class="text-on-surface-variant" />
				</div>
				<p class="font-display text-on-surface-variant text-[16px]">{m.studio_new_manuscript()}</p>
			</div>
		</div>
	{/if}
</section>
