<script lang="ts">
	import type { Block, Span } from "$lib/api/generated";
	import type { Annotation, ReaderTheme } from "$lib/ir/renderer";
	import {
		getBlockSpans,
		getAnnotatedSegments,
		COLORS,
		THEME_CONTENT_STYLES
	} from "$lib/ir/renderer";
	import SpanText from "$lib/components/SpanText.svelte";
	import { env } from "$env/dynamic/public";

	let {
		block,
		blockIdx,
		sectionId,
		bookId,
		annotations,
		theme,
		onAnnotationClick,
		onTextSelect
	}: {
		block: Block;
		blockIdx: number;
		sectionId: string;
		bookId: string;
		annotations: Annotation[];
		theme: ReaderTheme;
		onAnnotationClick: (ann: Annotation) => void;
		onTextSelect: (e: MouseEvent) => void;
	} = $props();

	const blockAnnotations = $derived(
		annotations.filter((a) => a.section_id === sectionId && a.block_index === blockIdx)
	);

	function getAssetUrl(assetId: string): string {
		const base = (env.PUBLIC_API_URL || "").replace(/\/+$/, "");
		return `${base}/api/v1/books/${bookId}/assets/${assetId}`;
	}
</script>

{#if block === "HorizontalRule"}
	<hr class="border-outline-variant my-12" />
{:else}
	{@const b = block as Record<string, unknown>}
	{@const spans = getBlockSpans(b)}
	{@const segs = getAnnotatedSegments(spans, blockAnnotations)}

	{#if "Paragraph" in b}
		<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
		<p
			data-block-index={blockIdx}
			class={["font-body mb-8 leading-relaxed transition-colors", THEME_CONTENT_STYLES[theme]]}
			onmouseup={onTextSelect}
		>
			{#each segs as seg, segIdx (seg.annotationId ?? `${seg.text}_${segIdx}`)}
				{#if seg.annotationId}
					<!-- svelte-ignore a11y_no_noninteractive_element_to_interactive_role -->
					<mark
						data-annotation-id={seg.annotationId}
						onclick={(e) => {
							e.stopPropagation();
							const ann = annotations.find((a) => a.id === seg.annotationId);
							if (ann) onAnnotationClick(ann);
						}}
						onkeydown={(e) => {
							if (e.key === "Enter" || e.key === " ") {
								e.preventDefault();
								e.stopPropagation();
								const ann = annotations.find((a) => a.id === seg.annotationId);
								if (ann) onAnnotationClick(ann);
							}
						}}
						class="cursor-pointer rounded-sm"
						tabindex="0"
						role="button"
						style="background: {COLORS[seg.color ?? 'yellow']};"
						><SpanText text={seg.text} marks={seg.marks} href={seg.href ?? ""} /></mark
					>
				{:else}
					<SpanText text={seg.text} marks={seg.marks} href={seg.href ?? ""} />
				{/if}
			{/each}
		</p>
	{:else if "Heading" in b}
		<h3
			data-block-index={blockIdx}
			class={[
				"font-display text-headline-sm mt-12 mb-6 transition-colors",
				theme === "light" ? "text-primary" : "text-inherit"
			]}
		>
			{#each segs as seg, segIdx (seg.annotationId ?? `${seg.text}_${segIdx}`)}
				{#if seg.annotationId}
					<!-- svelte-ignore a11y_no_noninteractive_element_to_interactive_role -->
					<mark
						data-annotation-id={seg.annotationId}
						style="background: {COLORS[seg.color ?? 'yellow']};"
						onclick={(e) => {
							e.stopPropagation();
							const ann = annotations.find((a) => a.id === seg.annotationId);
							if (ann) onAnnotationClick(ann);
						}}
						onkeydown={(e) => {
							if (e.key === "Enter" || e.key === " ") {
								e.preventDefault();
								e.stopPropagation();
								const ann = annotations.find((a) => a.id === seg.annotationId);
								if (ann) onAnnotationClick(ann);
							}
						}}
						class="cursor-pointer rounded-sm"
						tabindex="0"
						role="button"><SpanText text={seg.text} marks={seg.marks} href={seg.href ?? ""} /></mark
					>
				{:else}
					<SpanText text={seg.text} marks={seg.marks} href={seg.href ?? ""} />
				{/if}
			{/each}
		</h3>
	{:else if "Image" in b}
		{@const img = b.Image as { asset_ref: string; alt: string | null; src: string | null }}
		<div class="border-on-surface/5 bg-surface-container my-16 overflow-hidden rounded-xl border">
			<img
				src={img.asset_ref ? getAssetUrl(img.asset_ref) : (img.src ?? "")}
				alt={img.alt ?? ""}
				class="h-auto w-full"
				loading="lazy"
			/>
		</div>
	{:else if "CodeBlock" in b}
		{@const cb = b.CodeBlock as { language: string | null; content: string }}
		<pre
			class="bg-surface-container-high mb-8 overflow-x-auto rounded-xl p-6 font-mono text-sm"><code
				>{cb.content}</code
			></pre>
	{:else if "BlockQuote" in b}
		<blockquote class="border-secondary/30 mb-8 border-l-4 pl-6">
			{#each (b.BlockQuote as Array<Record<string, unknown>>) as subBlock, subIdx (subIdx)}
				{@const subSpans = getBlockSpans(subBlock)}
				<p class="font-body text-body-md text-on-surface-variant mb-2 italic">
					{#each subSpans as span (span.text + subIdx)}
						<SpanText text={span.text} marks={span.marks} href={span.href ?? ""} />
					{/each}
				</p>
			{/each}
		</blockquote>
	{:else if "UnorderedList" in b || "OrderedList" in b}
		{@const listType = "UnorderedList" in b ? "ul" : "ol"}
		{@const items = (b.UnorderedList ?? b.OrderedList) as Array<Array<Block>>}
		<svelte:element
			this={listType}
			class="mb-8 pl-6 [&>li]:mb-2"
			style="list-style-type: {'UnorderedList' in b ? 'disc' : 'decimal'};"
		>
			{#each items as item, itemIdx (itemIdx)}
				<li class="font-body text-body-md text-on-surface-variant">
					{#each item as subBlock, subIdx (subIdx)}
						{@const subSpans = getBlockSpans(subBlock)}
						{#each subSpans as span (span.text + subIdx)}
							<SpanText text={span.text} marks={span.marks} href={span.href ?? ""} />
						{/each}
					{/each}
				</li>
			{/each}
		</svelte:element>
	{:else if "RawHtml" in b}
		{@const rh = b.RawHtml as { content: string }}
		<!-- eslint-disable-next-line svelte/no-at-html-tags -->
		<div class="mb-8">{@html rh.content}</div>
	{/if}
{/if}
