<script lang="ts">
	let {
		label = "",
		value = $bindable(null),
		id = ""
	}: { label: string; value: number | null; id: string } = $props();
</script>

<div>
	<label
		for={id}
		class="font-label text-label-sm text-on-surface-variant mb-1.5 block tracking-widest uppercase"
		>{label}</label
	>
	<input {id} type="number" bind:value class="input-minimal" min="0" />
</div>
