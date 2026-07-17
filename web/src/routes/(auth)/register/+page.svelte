<script lang="ts">
	import * as m from "$lib/paraglide/messages";
	import * as v from "valibot";
	import { authState } from "$lib/api/client";
	import { goto } from "$app/navigation";
	import { RegisterSchema } from "$lib/validation";
	import type { RegisterFormData } from "$lib/validation";

	let email = $state("");
	let password = $state("");
	let displayName = $state("");
	let apiError = $state("");
	let fieldErrors = $state<Partial<Record<keyof RegisterFormData, string>>>({});
	let loading = $state(false);

	function validate(): RegisterFormData | null {
		const result = v.safeParse(RegisterSchema, { displayName, email, password });
		if (!result.success) {
			const errors: Partial<Record<keyof RegisterFormData, string>> = {};
			for (const issue of result.issues) {
				const path = issue.path?.[0]?.key as keyof RegisterFormData;
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
		const result = await authState.register({
			email: data.email,
			password: data.password,
			display_name: data.displayName
		});

		if (result.isOk()) {
			goto("/");
		} else {
			apiError = result.error.message || "Registration failed";
		}
		loading = false;
	}
</script>

<div class="mb-12 text-center">
	<h1 class="font-display text-display-mobile text-primary mb-2">{m.auth_register_title()}</h1>
	<p class="font-label text-label-sm text-on-surface-variant tracking-widest uppercase">
		{m.auth_register_tagline()}
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
			for="name"
			class="font-label text-label-sm text-on-surface-variant mb-2 block tracking-widest uppercase"
			>{m.auth_register_display_name()}</label
		>
		<input
			id="name"
			type="text"
			bind:value={displayName}
			class={[
				"input-minimal",
				fieldErrors.displayName ? "border-error" : "border-on-surface-variant/20"
			]}
			placeholder="Your name"
			oninput={() => (fieldErrors.displayName = undefined)}
		/>
		{#if fieldErrors.displayName}
			<p class="font-label text-label-sm text-error mt-1">{fieldErrors.displayName}</p>
		{/if}
	</div>

	<div>
		<label
			for="email"
			class="font-label text-label-sm text-on-surface-variant mb-2 block tracking-widest uppercase"
			>{m.auth_register_email_label()}</label
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
			>{m.auth_register_password_label()}</label
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
		{loading ? m.auth_register_submitting() : m.auth_register_submit()}
	</button>

	<p class="font-label text-label-sm text-on-surface-variant text-center">
		{m.auth_register_has_account()}
		<a
			href="/login"
			class="text-secondary border-secondary/30 hover:border-secondary border-b transition-all"
			>{m.auth_register_link_login()}</a
		>
	</p>
</form>
