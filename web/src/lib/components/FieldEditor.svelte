<script lang="ts">
	let {
		label = "",
		value = $bindable(""),
		id = ""
	}: { label: string; value: string; id: string } = $props();
</script>

<div>
	<label
		for={id}
		class="font-label text-label-sm text-on-surface-variant mb-1.5 block tracking-widest uppercase"
		>{label}</label
	>
	<input {id} type="text" bind:value class="input-minimal" />
</div>
