<script lang="ts">
	import * as m from "$lib/paraglide/messages";
	import { goto } from "$app/navigation";
	import { resolve } from "$app/paths";
	import { THEME_CLASSES, cycleTheme, type ReaderTheme } from "$lib/ir/renderer";
	import ArrowLeft from "@lucide/svelte/icons/arrow-left";
	import List from "@lucide/svelte/icons/list";
	import Sun from "@lucide/svelte/icons/sun";
	import Type from "@lucide/svelte/icons/type";

	let {
		progress = 0,
		title = "",
		theme = $bindable("light" as ReaderTheme),
		showToc = $bindable(false),
		showFontPanel = $bindable(false),
		children,
		headerExtra
	}: {
		progress?: number;
		title?: string;
		theme?: ReaderTheme;
		showToc?: boolean;
		showFontPanel?: boolean;
		children?: import("svelte").Snippet;
		headerExtra?: import("svelte").Snippet;
	} = $props();

	function back() {
		goto(resolve("/"));
	}
	function toggleToc() {
		showToc = !showToc;
		showFontPanel = false;
	}
	function toggleFont() {
		showFontPanel = !showFontPanel;
		showToc = false;
	}
	function handleCycleTheme() {
		theme = cycleTheme(theme);
	}
</script>

<div class={["min-h-screen transition-colors duration-300", THEME_CLASSES[theme]]}>
	<div class="bg-surface-container-low/20 fixed top-0 left-0 z-[60] w-full">
		<div class="bg-secondary h-[2px] transition-all duration-300" style="width: {progress}%;"></div>
	</div>

	<header
		class="bg-surface/90 px-margin-mobile md:px-margin-desktop fixed top-0 z-50 flex h-16 w-full items-center justify-between shadow-[0_1px_4px_rgba(0,31,63,0.05)] backdrop-blur-md"
	>
		<div class="flex items-center gap-4">
			<button
				onclick={back}
				class="flex items-center justify-center p-2 transition-transform duration-200 hover:opacity-80 active:scale-95"
			>
				<ArrowLeft size={20} class="text-primary" />
			</button>
			<h1 class="font-display text-headline-sm text-primary max-w-[240px] truncate md:max-w-md">
				{title || m.reader_loading()}
			</h1>
		</div>
		<div class="flex items-center gap-2">
			{#if headerExtra}{@render headerExtra()}{/if}
			<button
				onclick={toggleToc}
				class="p-2 transition-transform duration-200 hover:opacity-80 active:scale-95"
				aria-label={m.reader_toc_contents()}
			>
				<List size={20} class="text-on-surface-variant" />
			</button>
			<button
				onclick={handleCycleTheme}
				class="p-2 transition-transform duration-200 hover:opacity-80 active:scale-95"
				aria-label={m.reader_toggle_theme()}
			>
				<Sun size={20} class="text-on-surface-variant" />
			</button>
			<button
				onclick={toggleFont}
				class="p-2 transition-transform duration-200 hover:opacity-80 active:scale-95"
				aria-label={m.reader_font_settings()}
			>
				<Type size={20} class="text-on-surface-variant" />
			</button>
		</div>
	</header>

	<main
		data-reader-main
		class="px-margin-mobile min-h-screen pt-32 pb-40 transition-colors duration-300 md:px-0"
		style="max-width: 800px; margin: 0 auto;"
	>
		{#if children}{@render children()}{/if}
	</main>
</div>
