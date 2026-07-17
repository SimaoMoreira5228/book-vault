<script lang="ts">
	import { authState } from "$lib/api/client";
	import { goto } from "$app/navigation";

	let { children } = $props();

	$effect(() => {
		if (authState.isAuthenticated) {
			goto("/");
		}
	});
</script>

<div class="bg-surface px-margin-mobile flex min-h-screen items-center justify-center">
	<div class="w-full max-w-md">
		{@render children()}
	</div>
</div>
