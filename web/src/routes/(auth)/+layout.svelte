<script lang="ts">
	import { authState } from '$lib/api/client';
	import { goto } from '$app/navigation';

	let { children } = $props();

	$effect(() => {
		if (authState.isAuthenticated) {
			goto('/');
		}
	});
</script>

<div class="min-h-screen bg-surface flex items-center justify-center px-margin-mobile">
	<div class="w-full max-w-md">
		{@render children()}
	</div>
</div>
