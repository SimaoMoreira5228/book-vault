<script lang="ts">
	import * as m from "$lib/paraglide/messages";
	import { api } from "$lib/api/client";
	import type { Result } from "neverthrow";

	let tab = $state<"jobs" | "users" | "cleanup">("jobs");
	let jobs = $state<Array<Record<string, unknown>>>([]);
	let users = $state<Array<Record<string, unknown>>>([]);
	let cleanupResult = $state<string | null>(null);
	let loading = $state(false);

	async function loadJobs() {
		loading = true;
		const result = await api.admin.jobs();
		if (result.isOk()) jobs = result.value as unknown as Array<Record<string, unknown>>;
		loading = false;
	}

	async function loadUsers() {
		loading = true;
		const result = await api.admin.users();
		if (result.isOk()) users = result.value as unknown as Array<Record<string, unknown>>;
		loading = false;
	}

	async function doCleanup() {
		loading = true;
		const result = await api.admin.cleanupSessions();
		if (result.isOk()) {
			cleanupResult = `Deleted ${result.value.deleted} sessions`;
		} else {
			cleanupResult = "Cleanup failed";
		}
		loading = false;
	}

	$effect(() => {
		if (tab === "jobs" && jobs.length === 0) loadJobs();
		if (tab === "users" && users.length === 0) loadUsers();
	});
</script>

<svelte:head>
	<title>{m.admin_title()} — Book Vault</title>
</svelte:head>

<section>
	<header class="mb-8">
		<span class="font-label text-label-sm text-secondary mb-2 block tracking-widest uppercase"
			>System</span
		>
		<h2 class="font-display text-headline-md">{m.admin_title()}</h2>
	</header>

	<div class="border-outline/10 mb-8 flex gap-1 rounded-xl border p-1">
		<button
			onclick={() => (tab = "jobs")}
			class={[
				"font-label rounded-lg px-4 py-2 text-sm transition-all",
				tab === "jobs"
					? "bg-primary text-white shadow-sm"
					: "text-on-surface-variant hover:text-primary"
			]}>{m.admin_jobs()}</button
		>
		<button
			onclick={() => (tab = "users")}
			class={[
				"font-label rounded-lg px-4 py-2 text-sm transition-all",
				tab === "users"
					? "bg-primary text-white shadow-sm"
					: "text-on-surface-variant hover:text-primary"
			]}>{m.admin_users()}</button
		>
		<button
			onclick={() => (tab = "cleanup")}
			class={[
				"font-label rounded-lg px-4 py-2 text-sm transition-all",
				tab === "cleanup"
					? "bg-primary text-white shadow-sm"
					: "text-on-surface-variant hover:text-primary"
			]}>{m.admin_cleanup()}</button
		>
	</div>

	{#if tab === "jobs"}
		{#if loading}
			<div class="flex items-center justify-center py-16">
				<div
					class="border-secondary h-8 w-8 animate-spin rounded-full border-2 border-t-transparent"
				/>
			</div>
		{:else if jobs.length === 0}
			<div class="bg-surface-container-low rounded-xl p-12 text-center">
				<p class="font-body text-body-md text-on-surface-variant">No jobs found</p>
			</div>
		{:else}
			<div class="space-y-3">
				{#each jobs as job (job.id as string)}
					<div class="paper-card rounded-xl p-5">
						<div class="flex items-start justify-between gap-4">
							<div class="min-w-0 flex-1">
								<p class="font-label text-label-md text-primary mb-1">{job.kind as string}</p>
								<p class="font-label text-label-sm text-on-surface-variant truncate">
									{job.id as string}
								</p>
							</div>
							<span
								class={[
									"font-label shrink-0 rounded-full px-3 py-1 text-xs",
									job.status === "completed"
										? "bg-green-100 text-green-700"
										: job.status === "dead_letter"
											? "bg-red-100 text-red-700"
											: job.status === "processing"
												? "bg-blue-100 text-blue-700"
												: "bg-yellow-100 text-yellow-700"
								]}
							>
								{job.status as string}
							</span>
						</div>
						{#if job.error}
							<p class="font-label text-label-sm text-error mt-2">{job.error as string}</p>
						{/if}
						<div class="text-on-surface-variant mt-3 flex gap-4">
							<span class="font-label text-label-sm"
								>Retry {String(job.retry_count)}/{String(job.max_retries)}</span
							>
							<span class="font-label text-label-sm">{job.created_at as string}</span>
						</div>
					</div>
				{/each}
			</div>
		{/if}
	{:else if tab === "users"}
		{#if loading}
			<div class="flex items-center justify-center py-16">
				<div
					class="border-secondary h-8 w-8 animate-spin rounded-full border-2 border-t-transparent"
				/>
			</div>
		{:else}
			<div class="space-y-3">
				{#each users as user (user.id as string)}
					<div class="paper-card flex items-center gap-4 rounded-xl p-5">
						<div
							class="bg-primary flex h-10 w-10 items-center justify-center rounded-full text-sm font-bold text-white"
						>
							{(user.display_name as string).charAt(0).toUpperCase()}
						</div>
						<div class="min-w-0 flex-1">
							<p class="font-label text-label-md text-primary">{user.display_name as string}</p>
							<p class="font-label text-label-sm text-on-surface-variant">{user.email as string}</p>
						</div>
						{#if user.is_admin}
							<span class="font-label rounded-full bg-amber-100 px-3 py-1 text-xs text-amber-700"
								>Admin</span
							>
						{/if}
						<span class="font-label text-label-sm text-on-surface-variant"
							>{user.created_at as string}</span
						>
					</div>
				{/each}
			</div>
		{/if}
	{:else if tab === "cleanup"}
		<div class="paper-card rounded-xl p-8">
			<h3 class="font-display text-headline-sm text-primary mb-4">{m.admin_cleanup()}</h3>
			<p class="font-body text-body-md text-on-surface-variant mb-6">
				Remove expired and revoked sessions older than 30 days.
			</p>
			<button onclick={doCleanup} disabled={loading} class="btn-primary">
				{loading ? "Working..." : "Run Cleanup"}
			</button>
			{#if cleanupResult}
				<p class="font-label text-label-md text-secondary mt-4">{cleanupResult}</p>
			{/if}
		</div>
	{/if}
</section>
