<script lang="ts">
	import * as m from "$lib/paraglide/messages";
	import * as v from "valibot";
	import { authState } from "$lib/api/client.svelte";
	import { goto } from "$app/navigation";
	import { resolve } from "$app/paths";
	import { LoginSchema } from "$lib/validation";
	import type { LoginFormData } from "$lib/validation";

	let email = $state("");
	let password = $state("");
	let apiError = $state("");
	let fieldErrors = $state<Partial<Record<keyof LoginFormData, string>>>({});
	let loading = $state(false);

	function validate(): LoginFormData | null {
		const result = v.safeParse(LoginSchema, { email, password });
		if (!result.success) {
			const errors: Partial<Record<keyof LoginFormData, string>> = {};
			for (const issue of result.issues) {
				const path = issue.path?.[0]?.key as keyof LoginFormData;
				if (path && !errors[path]) errors[path] = issue.message;
			}
			fieldErrors = errors;
			return null;
		}
		fieldErrors = {};
		return result.output;
	}

	async function handleSubmit() {
		apiError = "";
		const data = validate();
		if (!data) return;

		loading = true;
		const result = await authState.login({ email: data.email, password: data.password });

		if (result.isOk()) {
			goto(resolve("/"));
		} else {
			apiError = result.error.message || "Login failed";
		}
		loading = false;
	}
</script>

<div class="mb-12 text-center">
	<h1 class="font-display text-display-mobile text-primary mb-2">{m.app_name()}</h1>
	<p class="font-label text-label-sm text-on-surface-variant tracking-widest uppercase">
		{m.app_tagline()}
	</p>
</div>

<form onsubmit={handleSubmit} novalidate class="space-y-8">
	{#if apiError}
		<div class="font-label text-label-sm text-error bg-error-container/20 rounded-lg px-4 py-3">
			{apiError}
		</div>
	{/if}

	<div>
		<label
			for="email"
			class="font-label text-label-sm text-on-surface-variant mb-2 block tracking-widest uppercase"
			>{m.auth_login_email_label()}</label
		>
		<input
			id="email"
			type="email"
			bind:value={email}
			class={["input-minimal", fieldErrors.email ? "border-error" : "border-on-surface-variant/20"]}
			placeholder="your@email.com"
			oninput={() => (fieldErrors.email = undefined)}
		/>
		{#if fieldErrors.email}
			<p class="font-label text-label-sm text-error mt-1">{fieldErrors.email}</p>
		{/if}
	</div>

	<div>
		<label
			for="password"
			class="font-label text-label-sm text-on-surface-variant mb-2 block tracking-widest uppercase"
			>{m.auth_login_password_label()}</label
		>
		<input
			id="password"
			type="password"
			bind:value={password}
			class={[
				"input-minimal",
				fieldErrors.password ? "border-error" : "border-on-surface-variant/20"
			]}
			placeholder="••••••••"
			oninput={() => (fieldErrors.password = undefined)}
		/>
		{#if fieldErrors.password}
			<p class="font-label text-label-sm text-error mt-1">{fieldErrors.password}</p>
		{/if}
	</div>

	<button type="submit" disabled={loading} class="btn-primary w-full">
		{loading ? m.auth_login_submitting() : m.auth_login_submit()}
	</button>

	<p class="font-label text-label-sm text-on-surface-variant text-center">
		{m.auth_login_no_account()}
		<a
			href={resolve("/register")}
			class="text-secondary border-secondary/30 hover:border-secondary border-b transition-all"
			>{m.auth_login_link_register()}</a
		>
	</p>
</form>
