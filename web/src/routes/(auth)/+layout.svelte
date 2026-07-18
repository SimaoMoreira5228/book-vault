<script lang="ts">
	let { children } = $props();
</script>

<div class="bg-surface px-margin-mobile flex min-h-screen items-center justify-center">
	<div class="w-full max-w-md">
		{@render children()}
	</div>
</div>
