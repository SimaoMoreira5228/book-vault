<script lang="ts">
	let { label = "", value = "", id = "" }: { label: string; value: string; id: string } = $props();
</script>

<div>
	<label
		for={id}
		class="font-label text-label-sm text-on-surface-variant mb-1.5 block tracking-widest uppercase"
		>{label}</label
	>
	<p class="font-body text-body-md text-primary" {id}>{value}</p>
</div>
