<script lang="ts">
	import { authState } from "$lib/api/client";
	import { goto } from "$app/navigation";
	import type { RegisterRequest } from "$lib/api/generated";

	let email = $state("");
	let password = $state("");
	let displayName = $state("");
	let error = $state("");
	let loading = $state(false);

	async function handleSubmit() {
		error = "";
		loading = true;

		const result = await authState.register({
			email,
			password,
			display_name: displayName
		} as RegisterRequest);

		if (result.isOk()) {
			goto("/");
		} else {
			error = result.error.message || "Registration failed";
		}
		loading = false;
	}
</script>

<div class="mb-12 text-center">
	<h1 class="font-display text-display-mobile text-primary mb-2">Create Account</h1>
	<p class="font-label text-label-sm text-on-surface-variant tracking-widest uppercase">
		Join the Sanctuary
	</p>
</div>

<form onsubmit={handleSubmit} class="space-y-8">
	{#if error}
		<div class="font-label text-label-sm text-error bg-error-container/20 rounded-lg px-4 py-3">
			{error}
		</div>
	{/if}

	<div>
		<label
			for="name"
			class="font-label text-label-sm text-on-surface-variant mb-2 block tracking-widest uppercase"
			>Display Name</label
		>
		<input
			id="name"
			type="text"
			bind:value={displayName}
			class="input-minimal"
			placeholder="Your name"
			required
		/>
	</div>

	<div>
		<label
			for="email"
			class="font-label text-label-sm text-on-surface-variant mb-2 block tracking-widest uppercase"
			>Email</label
		>
		<input
			id="email"
			type="email"
			bind:value={email}
			class="input-minimal"
			placeholder="your@email.com"
			required
		/>
	</div>

	<div>
		<label
			for="password"
			class="font-label text-label-sm text-on-surface-variant mb-2 block tracking-widest uppercase"
			>Password</label
		>
		<input
			id="password"
			type="password"
			bind:value={password}
			class="input-minimal"
			placeholder="••••••••"
			required
		/>
	</div>

	<button type="submit" disabled={loading} class="btn-primary w-full">
		{loading ? "Creating..." : "Create Account"}
	</button>

	<p class="font-label text-label-sm text-on-surface-variant text-center">
		Already have an account?
		<a
			href="/login"
			class="text-secondary border-secondary/30 hover:border-secondary border-b transition-all"
			>Sign In</a
		>
	</p>
</form>
