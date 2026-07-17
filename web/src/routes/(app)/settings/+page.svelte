<script lang="ts">
	import * as m from "$lib/paraglide/messages";
	import { api, authState } from "$lib/api/client";
	import { goto } from "$app/navigation";
	import Monitor from "@lucide/svelte/icons/monitor";
	import Smartphone from "@lucide/svelte/icons/smartphone";
	import Laptop from "@lucide/svelte/icons/laptop";
	import Tablet from "@lucide/svelte/icons/tablet";
	import Trash2 from "@lucide/svelte/icons/trash-2";
	import LogOut from "@lucide/svelte/icons/log-out";
	import CircleX from "@lucide/svelte/icons/circle-x";

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

	$effect(() => {
		if (!authState.isAuthenticated) {
			goto("/login");
			return;
		}
		loadSessions();
	});

	async function loadSessions() {
		loading = true;
		error = "";
		const result = await api.sessions.list();
		if (result.isOk()) {
			sessions = result.value as unknown as SessionInfo[];
		} else {
			error = result.error.message;
		}
		loading = false;
	}

	async function handleRevoke(id: string) {
		revoking = id;
		const result = await api.sessions.revoke(id);
		if (result.isOk()) {
			sessions = sessions.filter((s) => s.id !== id);
		} else {
			error = result.error.message;
		}
		revoking = null;
	}

	async function handleLogoutEverywhere() {
		await authState.logout();
		goto("/login");
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
</script>

<svelte:head>
	<title>{m.settings_title()} — Book Vault</title>
</svelte:head>

<section>
	<header class="mb-10">
		<span class="font-label text-label-sm text-secondary mb-2 block tracking-widest uppercase"
			>{m.settings_subtitle()}</span
		>
		<h2 class="font-display text-headline-md">{m.settings_title()}</h2>
	</header>

	{#if error}
		<div
			class="font-label text-label-sm text-error bg-error-container/20 mb-8 rounded-lg px-4 py-3"
		>
			{error}
		</div>
	{/if}

	<div class="paper-card mb-10 rounded-xl p-8">
		<div class="mb-6 flex items-center gap-4">
			<div
				class="bg-primary flex h-14 w-14 items-center justify-center rounded-full text-xl font-bold text-white"
			>
				{user?.display_name?.charAt(0).toUpperCase() ?? "?"}
			</div>
			<div>
				<h3 class="font-display text-headline-sm text-primary">{m.settings_profile()}</h3>
				<p class="font-label text-label-sm text-on-surface-variant"></p>
			</div>
		</div>

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
	</div>

	<div class="paper-card rounded-xl p-8">
		<div class="mb-6 flex items-center justify-between">
			<h3 class="font-display text-headline-sm text-primary">{m.settings_sessions()}</h3>
			<button
				onclick={handleLogoutEverywhere}
				class="font-label text-label-sm text-error hover:text-error/80 flex items-center gap-1.5 transition-colors"
			>
				<LogOut size={14} />
				{m.settings_logout()}
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
									>
										{m.settings_current_session()}
									</span>
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
							<Trash2 size={14} />
							{m.settings_revoke()}
						</button>
					</div>
				{/each}
			</div>
		{/if}
	</div>
</section>
