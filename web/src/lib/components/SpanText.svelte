<script lang="ts">
	let {
		text = "",
		marks = 0,
		href = ""
	}: {
		text: string;
		marks?: number;
		href?: string;
	} = $props();
</script>

{#if href}
	<a
		{href}
		target="_blank"
		rel="noreferrer"
		class="underline decoration-dotted underline-offset-2 hover:decoration-solid"
	>
		{text}
	</a>
{:else if marks & 1 && marks & 2}
	<em><strong>{text}</strong></em>
{:else if marks & 8}
	<s class="decoration-on-surface-variant/40">{text}</s>
{:else if marks & 64}
	<code class="bg-surface-container-high rounded px-1 py-0.5 text-sm">{text}</code>
{:else if marks & 1}
	<strong>{text}</strong>
{:else if marks & 2}
	<em>{text}</em>
{:else if marks & 4}
	<u>{text}</u>
{:else}
	{text}
{/if}
