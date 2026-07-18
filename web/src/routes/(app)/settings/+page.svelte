<script lang="ts">
	import * as m from "$lib/paraglide/messages";
	import { api, apiBase, authState } from "$lib/api/client.svelte";
	import { goto } from "$app/navigation";
	import { resolve } from "$app/paths";
	import { locales, localizeUrl } from "$lib/paraglide/runtime";
	import { page } from "$app/state";
	import Languages from "@lucide/svelte/icons/languages";
	import Monitor from "@lucide/svelte/icons/monitor";
	import Smartphone from "@lucide/svelte/icons/smartphone";
	import Laptop from "@lucide/svelte/icons/laptop";
	import Tablet from "@lucide/svelte/icons/tablet";
	import Trash2 from "@lucide/svelte/icons/trash-2";
	import LogOut from "@lucide/svelte/icons/log-out";
	import CircleX from "@lucide/svelte/icons/circle-x";
	import Pencil from "@lucide/svelte/icons/pencil";
	import Sun from "@lucide/svelte/icons/sun";
	import { Slider } from "bits-ui";

	type SessionInfo = {
		id: string;
		user_agent: string | null;
		ip_address: string | null;
		created_at: string;
		last_seen_at: string;
		expires_at: string;
		is_current: boolean;
	};

	let sessions = $state<SessionInfo[]>([]);
	let loading = $state(true);
	let revoking = $state<string | null>(null);
	let error = $state("");
	let success = $state("");

	let editingProfile = $state(false);
	let displayName = $state("");
	let savingProfile = $state(false);

	let changingPassword = $state(false);
	let currentPassword = $state("");
	let newPassword = $state("");
	let confirmPassword = $state("");
	let savingPassword = $state(false);
	let passwordError = $state("");

	let readerTheme = $state("light");
	let readerFontSize = $state(18);
	let readerLineHeight = $state(1.8);

	$effect(() => {
		if (authState.isAuthenticated) loadReaderPrefs();
	});

	async function loadReaderPrefs() {
		const r = await api.auth.getPreferences();
		if (r.isOk() && r.value.reader) {
			const p = r.value.reader as Record<string, unknown>;
			if (typeof p.theme === "string") readerTheme = p.theme;
			if (typeof p.fontSize === "number") readerFontSize = p.fontSize;
			if (typeof p.lineHeight === "number") readerLineHeight = p.lineHeight;
		}
	}

	async function saveReaderPrefs() {
		const r = await api.auth.updatePreferences({
			reader: { theme: readerTheme, fontSize: readerFontSize, lineHeight: readerLineHeight }
		});
		if (r.isOk()) success = m.settings_profile_saved();
		else error = r.error.message;
	}

	$effect(() => {
		if (!authState.isAuthenticated) {
			goto(resolve("/login"));
			return;
		}
		loadSessions();
	});

	async function loadSessions() {
		loading = true;
		error = "";
		const r = await api.sessions.list();
		if (r.isOk()) sessions = r.value as unknown as SessionInfo[];
		else error = r.error.message;
		loading = false;
	}

	async function handleRevoke(id: string) {
		revoking = id;
		const r = await api.sessions.revoke(id);
		if (r.isOk()) sessions = sessions.filter((s) => s.id !== id);
		else error = r.error.message;
		revoking = null;
	}

	async function handleLogoutEverywhere() {
		await authState.logout();
		goto(resolve("/login"));
	}

	async function handleSaveProfile() {
		if (!displayName.trim()) return;
		savingProfile = true;
		error = "";
		success = "";
		const r = await api.auth.updateProfile({ display_name: displayName.trim() });
		if (r.isOk()) {
			authState.user = r.value;
			editingProfile = false;
			success = m.settings_profile_saved();
		} else error = r.error.message;
		savingProfile = false;
	}

	function startEditProfile() {
		displayName = authState.user?.display_name ?? "";
		editingProfile = true;
	}

	async function handleChangePassword() {
		passwordError = "";
		error = "";
		success = "";
		if (newPassword !== confirmPassword) {
			passwordError = m.settings_password_mismatch();
			return;
		}
		if (newPassword.length < 6) {
			passwordError = m.settings_password_short();
			return;
		}
		savingPassword = true;
		const r = await api.auth.changePassword({
			current_password: currentPassword,
			new_password: newPassword
		});
		if (r.isOk()) {
			currentPassword = "";
			newPassword = "";
			confirmPassword = "";
			changingPassword = false;
			success = m.settings_password_changed();
		} else error = r.error.message;
		savingPassword = false;
	}

	function deviceIcon(ua: string | null) {
		const u = (ua ?? "").toLowerCase();
		if (u.includes("iphone") || u.includes("android")) return Smartphone;
		if (u.includes("ipad") || u.includes("tablet")) return Tablet;
		if (
			u.includes("macintosh") ||
			u.includes("mac os") ||
			u.includes("linux") ||
			u.includes("windows")
		)
			return Laptop;
		return Monitor;
	}

	const user = $derived(authState.user);

	const currentLocale = $derived(
		locales.find((l) => page.url.pathname.startsWith(`/${l}`)) ?? "en"
	);

	const localeLabels: Record<string, string> = {
		en: m.locale_en(),
		"pt-PT": m["locale_pt-PT"]()
	};

	function handleLocaleChange(locale: "en" | "pt-PT") {
		// eslint-disable-next-line svelte/no-navigation-without-resolve
		goto(localizeUrl(page.url.pathname, { locale }));
	}
</script>

<svelte:head><title>{m.settings_title()} — Book Vault</title></svelte:head>

<section>
	<header class="mb-10">
		<span class="font-label text-label-sm text-secondary mb-2 block tracking-widest uppercase"
			>{m.settings_subtitle()}</span
		>
		<h2 class="font-display text-headline-md">{m.settings_title()}</h2>
	</header>

	{#if error}<div
			class="font-label text-label-sm text-error bg-error-container/20 mb-6 rounded-lg px-4 py-3"
		>
			{error}
		</div>{/if}
	{#if success}<div
			class="font-label text-label-sm bg-secondary/10 text-secondary mb-6 rounded-lg px-4 py-3"
		>
			{success}
		</div>{/if}

	<div class="paper-card mb-10 rounded-xl p-8">
		<div class="mb-6 flex items-center justify-between">
			<div class="flex items-center gap-4">
				<div
					class="bg-primary flex h-14 w-14 items-center justify-center rounded-full text-xl font-bold text-white"
				>
					{user?.display_name?.charAt(0).toUpperCase() ?? "?"}
				</div>
				<div>
					<h3 class="font-display text-headline-sm text-primary">{m.settings_profile()}</h3>
				</div>
			</div>
			{#if !editingProfile}
				<button
					onclick={startEditProfile}
					class="font-label text-label-md text-secondary hover:text-secondary/80 inline-flex items-center gap-1.5 transition-colors"
				>
					<Pencil size={16} />{m.settings_edit_profile()}
				</button>
			{/if}
		</div>

		{#if editingProfile}
			<div class="space-y-5">
				<div>
					<p
						class="font-label text-label-sm text-on-surface-variant mb-1.5 tracking-widest uppercase"
					>
						{m.settings_display_name()}
					</p>
					<input type="text" bind:value={displayName} class="input-minimal" />
				</div>
				<div>
					<p
						class="font-label text-label-sm text-on-surface-variant mb-1.5 tracking-widest uppercase"
					>
						{m.settings_email()}
					</p>
					<p class="font-body text-body-md text-on-surface-variant">{user?.email ?? "—"}</p>
				</div>
				<div class="flex justify-end gap-3 pt-2">
					<button
						onclick={() => (editingProfile = false)}
						class="font-label text-label-md text-on-surface-variant px-4 py-2"
						>{m.book_detail_cancel()}</button
					>
					<button
						onclick={handleSaveProfile}
						disabled={savingProfile || !displayName.trim()}
						class="btn-primary"
					>
						{savingProfile ? "..." : m.settings_save_profile()}
					</button>
				</div>
			</div>
		{:else}
			<div class="space-y-6">
				<div>
					<p
						class="font-label text-label-sm text-on-surface-variant mb-1.5 tracking-widest uppercase"
					>
						{m.settings_display_name()}
					</p>
					<p class="font-body text-body-md text-primary">{user?.display_name ?? "—"}</p>
				</div>
				<div>
					<p
						class="font-label text-label-sm text-on-surface-variant mb-1.5 tracking-widest uppercase"
					>
						{m.settings_email()}
					</p>
					<p class="font-body text-body-md text-primary">{user?.email ?? "—"}</p>
				</div>
			</div>
		{/if}
	</div>

	<div class="paper-card mb-10 rounded-xl p-8">
		<div class="mb-6 flex items-center justify-between">
			<h3 class="font-display text-headline-sm text-primary">{m.settings_language()}</h3>
		</div>
		<p class="font-label text-label-sm text-on-surface-variant mb-4 tracking-widest uppercase">
			{m.settings_language_subtitle()}
		</p>
		<div class="flex flex-wrap gap-2">
			{#each locales as locale (locale)}
				<button
					onclick={() => handleLocaleChange(locale)}
					class={[
						"font-label text-label-sm inline-flex items-center gap-2 rounded-xl px-5 py-3 transition-all",
						locale === currentLocale
							? "bg-primary text-white shadow-sm"
							: "bg-surface-container-low text-on-surface-variant hover:text-primary hover:bg-surface-container-high border border-transparent"
					]}
				>
					<Languages size={16} />
					{localeLabels[locale] ?? locale}
				</button>
			{/each}
		</div>
	</div>

	<div class="paper-card mb-10 rounded-xl p-8">
		<div class="mb-6 flex items-center justify-between">
			<h3 class="font-display text-headline-sm text-primary">{m.settings_reader()}</h3>
		</div>
		<p class="font-label text-label-sm text-on-surface-variant mb-4 tracking-widest uppercase">
			{m.settings_reader_subtitle()}
		</p>

		<div class="space-y-6">
			<div>
				<p class="font-label text-label-sm text-on-surface-variant mb-3 tracking-widest uppercase">
					{m.settings_reader_theme()}
				</p>
				<div class="flex gap-2">
					<button
						onclick={() => {
							readerTheme = "light";
							saveReaderPrefs();
						}}
						class={[
							"font-label text-label-sm rounded-xl px-5 py-3 transition-all",
							readerTheme === "light"
								? "bg-primary text-white shadow-sm"
								: "bg-surface-container-low text-on-surface-variant hover:text-primary hover:bg-surface-container-high"
						]}
					>
						<Sun size={16} class="inline" />
						{m.settings_reader_theme_light()}
					</button>
					<button
						onclick={() => {
							readerTheme = "sepia";
							saveReaderPrefs();
						}}
						class={[
							"font-label text-label-sm rounded-xl px-5 py-3 transition-all",
							readerTheme === "sepia"
								? "bg-primary text-white shadow-sm"
								: "bg-surface-container-low text-on-surface-variant hover:text-primary hover:bg-surface-container-high"
						]}
					>
						{m.settings_reader_theme_sepia()}
					</button>
					<button
						onclick={() => {
							readerTheme = "dark";
							saveReaderPrefs();
						}}
						class={[
							"font-label text-label-sm rounded-xl px-5 py-3 transition-all",
							readerTheme === "dark"
								? "bg-primary text-white shadow-sm"
								: "bg-surface-container-low text-on-surface-variant hover:text-primary hover:bg-surface-container-high"
						]}
					>
						{m.settings_reader_theme_dark()}
					</button>
				</div>
			</div>

			<div>
				<div class="mb-2 flex items-center justify-between">
					<p class="font-label text-label-sm text-on-surface-variant tracking-widest uppercase">
						{m.settings_reader_font_size()}
					</p>
					<span class="font-label text-label-sm text-on-surface-variant">{readerFontSize}px</span>
				</div>
				<Slider.Root
					type="single"
					bind:value={readerFontSize}
					min={12}
					max={36}
					step={2}
					onValueCommit={saveReaderPrefs}
					class="bg-surface-container-low relative flex h-2 w-full items-center rounded-full"
				>
					<Slider.Range class="bg-secondary absolute h-full rounded-full" />
					<Slider.Thumb index={0}>
						{#snippet child({ props })}
							<span
								{...props}
								class="bg-secondary border-surface focus-visible:ring-primary/20 block h-5 w-5 rounded-full border-2 shadow-sm focus-visible:ring-2 focus-visible:outline-none"
							></span>
						{/snippet}
					</Slider.Thumb>
				</Slider.Root>
			</div>

			<div>
				<div class="mb-2 flex items-center justify-between">
					<p class="font-label text-label-sm text-on-surface-variant tracking-widest uppercase">
						{m.settings_reader_line_height()}
					</p>
					<span class="font-label text-label-sm text-on-surface-variant"
						>{readerLineHeight.toFixed(1)}</span
					>
				</div>
				<Slider.Root
					type="single"
					bind:value={readerLineHeight}
					min={1.2}
					max={3.0}
					step={0.2}
					onValueCommit={saveReaderPrefs}
					class="bg-surface-container-low relative flex h-2 w-full items-center rounded-full"
				>
					<Slider.Range class="bg-secondary absolute h-full rounded-full" />
					<Slider.Thumb index={0}>
						{#snippet child({ props })}
							<span
								{...props}
								class="bg-secondary border-surface focus-visible:ring-primary/20 block h-5 w-5 rounded-full border-2 shadow-sm focus-visible:ring-2 focus-visible:outline-none"
							></span>
						{/snippet}
					</Slider.Thumb>
				</Slider.Root>
			</div>
		</div>
	</div>

	<div class="paper-card mb-10 rounded-xl p-8">
		<div class="mb-6 flex items-center justify-between">
			<h3 class="font-display text-headline-sm text-primary">{m.settings_change_password()}</h3>
			{#if !changingPassword}
				<button
					onclick={() => (changingPassword = true)}
					class="font-label text-label-md text-secondary hover:text-secondary/80 transition-colors"
				>
					<Pencil size={16} />{m.settings_change_password()}
				</button>
			{/if}
		</div>

		{#if changingPassword}
			{#if passwordError}<div
					class="font-label text-label-sm text-error bg-error-container/20 mb-4 rounded-lg px-4 py-2"
				>
					{passwordError}
				</div>{/if}
			<div class="space-y-5">
				<div>
					<p
						class="font-label text-label-sm text-on-surface-variant mb-1.5 tracking-widest uppercase"
					>
						{m.settings_current_password()}
					</p>
					<input type="password" bind:value={currentPassword} class="input-minimal" />
				</div>
				<div>
					<p
						class="font-label text-label-sm text-on-surface-variant mb-1.5 tracking-widest uppercase"
					>
						{m.settings_new_password()}
					</p>
					<input type="password" bind:value={newPassword} class="input-minimal" />
				</div>
				<div>
					<p
						class="font-label text-label-sm text-on-surface-variant mb-1.5 tracking-widest uppercase"
					>
						{m.settings_confirm_password()}
					</p>
					<input type="password" bind:value={confirmPassword} class="input-minimal" />
				</div>
				<div class="flex justify-end gap-3 pt-2">
					<button
						onclick={() => {
							changingPassword = false;
							passwordError = "";
						}}
						class="font-label text-label-md text-on-surface-variant px-4 py-2"
						>{m.book_detail_cancel()}</button
					>
					<button
						onclick={handleChangePassword}
						disabled={savingPassword || !currentPassword || !newPassword}
						class="btn-primary"
					>
						{savingPassword ? "..." : m.settings_change_password()}
					</button>
				</div>
			</div>
		{/if}
	</div>

	<div class="paper-card rounded-xl p-8">
		<div class="mb-6 flex items-center justify-between">
			<h3 class="font-display text-headline-sm text-primary">{m.settings_sessions()}</h3>
			<button
				onclick={handleLogoutEverywhere}
				class="font-label text-label-sm text-error hover:text-error/80 flex items-center gap-1.5 transition-colors"
			>
				<LogOut size={14} />{m.settings_logout()}
			</button>
		</div>

		{#if loading}
			<div class="flex items-center justify-center py-12">
				<div
					class="border-secondary h-8 w-8 animate-spin rounded-full border-2 border-t-transparent"
				></div>
			</div>
		{:else if sessions.length === 0}
			<div class="py-12 text-center">
				<CircleX size={28} class="text-on-surface-variant/30 mb-3 block" />
				<p class="font-body text-body-md text-on-surface-variant">{m.settings_sessions_empty()}</p>
			</div>
		{:else}
			<div class="space-y-3">
				{#each sessions as session (session.id)}
					{@const Icon = deviceIcon(session.user_agent)}
					<div class="bg-surface-container-low flex items-center gap-4 rounded-xl p-4">
						<div
							class="bg-surface-container flex h-10 w-10 flex-shrink-0 items-center justify-center rounded-full"
						>
							<Icon size={18} class="text-on-surface-variant" />
						</div>
						<div class="min-w-0 flex-1">
							<div class="flex items-center gap-2">
								<p class="font-label text-label-md text-primary truncate">
									{session.user_agent ?? "Unknown device"}
								</p>
								{#if session.is_current}
									<span
										class="font-label bg-secondary/10 text-secondary rounded-full px-2 py-0.5 text-[10px] tracking-wider uppercase"
										>{m.settings_current_session()}</span
									>
								{/if}
							</div>
							<p class="font-label text-label-sm text-on-surface-variant">
								{session.ip_address ?? "Unknown IP"} · {session.created_at}
							</p>
						</div>
						<button
							onclick={() => handleRevoke(session.id)}
							disabled={revoking === session.id}
							class="font-label text-label-sm text-error hover:text-error/80 flex items-center gap-1 rounded-lg px-3 py-2 transition-colors enabled:hover:bg-red-50 disabled:opacity-50"
						>
							<Trash2 size={14} />{m.settings_revoke()}
						</button>
					</div>
				{/each}
			</div>
		{/if}
	</div>

	<div class="paper-card rounded-xl p-8">
		<div class="mb-6 flex items-center justify-between">
			<h3 class="font-display text-headline-sm text-primary">{m.settings_integrations()}</h3>
		</div>

		<div class="space-y-5">
			<div>
				<p
					class="font-label text-label-sm text-on-surface-variant mb-1.5 tracking-widest uppercase"
				>
					{m.settings_opds_title()}
				</p>
				<p class="font-body text-body-md text-on-surface-variant mb-2">
					{m.settings_opds_desc()}
				</p>
				<div class="bg-surface-container-low flex items-center gap-3 rounded-xl px-4 py-3">
					<code class="text-primary flex-1 font-mono text-sm break-all">{apiBase}/opds</code>
					<button
						onclick={() => navigator.clipboard.writeText(`${apiBase}/opds`)}
						class="font-label text-label-sm text-secondary hover:text-secondary/80 shrink-0 transition-colors"
					>
						{m.settings_copy()}
					</button>
				</div>
			</div>

			<div>
				<p
					class="font-label text-label-sm text-on-surface-variant mb-1.5 tracking-widest uppercase"
				>
					{m.settings_koreader_title()}
				</p>
				<p class="font-body text-body-md text-on-surface-variant mb-2">
					{m.settings_koreader_desc()}
				</p>
				<div class="bg-surface-container-low flex items-center gap-3 rounded-xl px-4 py-3">
					<code class="text-primary flex-1 font-mono text-sm break-all"
						>{apiBase}/api/v1/koreader/progress</code
					>
				</div>
			</div>
		</div>
	</div>
</section>
