<script lang="ts">
	import * as m from "$lib/paraglide/messages";
	import { api } from "$lib/api/client.svelte";
	import Modal from "./Modal.svelte";
	import Upload from "@lucide/svelte/icons/upload";
	import CheckCircle from "@lucide/svelte/icons/check-circle";
	import AlertCircle from "@lucide/svelte/icons/alert-circle";
	import LoaderCircle from "@lucide/svelte/icons/loader-circle";

	let { show = $bindable(false), onComplete = () => {} } = $props();

	let dragOver = $state(false);
	let status = $state<"idle" | "uploading" | "processing" | "completed" | "error">("idle");
	let errorMsg = $state("");

	function preventDefaults(e: Event) {
		e.preventDefault();
		e.stopPropagation();
	}

	function onDragEnter(e: DragEvent) {
		preventDefaults(e);
		dragOver = true;
	}

	function onDragLeave(e: DragEvent) {
		preventDefaults(e);
		dragOver = false;
	}

	function onDrop(e: DragEvent) {
		preventDefaults(e);
		dragOver = false;
		const files = e.dataTransfer?.files;
		if (files?.length) uploadFile(files[0]);
	}

	function onFileSelect(e: Event) {
		const input = e.target as HTMLInputElement;
		if (input.files?.length) uploadFile(input.files[0]);
	}

	async function uploadFile(file: File) {
		status = "uploading";
		errorMsg = "";

		const result = await api.books.upload(file);
		if (result.isErr()) {
			status = "error";
			errorMsg = result.error.message;
			return;
		}

		status = "processing";
		const { job_id } = result.value;

		const maxPolls = 30;
		for (let i = 0; i < maxPolls; i++) {
			await new Promise((r) => setTimeout(r, 2000));
			const jobResult = await api.jobs.get(job_id);
			if (jobResult.isOk()) {
				const job = jobResult.value;
				if (job.status === "completed") {
					status = "completed";
					setTimeout(() => {
						show = false;
						status = "idle";
						onComplete();
					}, 1500);
					return;
				}
				if (job.status === "failed" || job.status === "dead_letter") {
					status = "error";
					errorMsg = job.error ?? m.upload_job_failed();
					return;
				}
			}
		}
		status = "error";
		errorMsg = "Upload timed out";
	}
</script>

<svelte:window
	ondragover={preventDefaults}
	ondragenter={preventDefaults}
	ondragleave={preventDefaults}
	ondrop={preventDefaults}
/>

<Modal bind:show>
	<div class="min-h-[300px]">
		{#if status === "completed"}
			<div class="py-8 text-center">
				<CheckCircle size={48} class="text-secondary mb-4 block" />
				<p class="font-body text-body-md text-primary">{m.upload_success()}</p>
			</div>
		{:else if status === "uploading" || status === "processing"}
			<div class="py-8 text-center">
				<LoaderCircle size={40} class="text-secondary mb-4 block animate-spin" />
				<p class="font-body text-body-md text-primary">{m.upload_processing()}</p>
				<div class="mt-6 flex items-center justify-center gap-2">
					<div class="border-secondary h-2 w-2 animate-pulse rounded-full"></div>
					<span class="font-label text-label-sm text-on-surface-variant">
						{status === "uploading" ? "Uploading..." : m.upload_job_processing()}
					</span>
				</div>
			</div>
		{:else if status === "error"}
			<div class="py-8 text-center">
				<AlertCircle size={40} class="text-error mb-4 block" />
				<p class="font-body text-body-md text-error">{errorMsg || m.upload_error()}</p>
				<button
					onclick={() => {
						status = "idle";
						errorMsg = "";
					}}
					class="font-label text-label-md text-secondary mt-6 transition-colors hover:opacity-80"
				>
					Try again
				</button>
			</div>
		{:else}
			<div
				class={[
					"flex cursor-pointer flex-col items-center justify-center rounded-xl border-2 border-dashed px-8 py-12 transition-all",
					dragOver
						? "border-secondary bg-secondary/5"
						: "border-outline-variant/30 hover:border-secondary/50 hover:bg-secondary/[0.02]"
				]}
				ondragenter={onDragEnter}
				ondragover={preventDefaults}
				ondragleave={onDragLeave}
				ondrop={onDrop}
				onclick={() => document.getElementById("file-input")?.click()}
				role="button"
				tabindex="0"
				onkeydown={(e) => {
					if (e.key === "Enter" || e.key === " ") document.getElementById("file-input")?.click();
				}}
			>
				<Upload
					size={32}
					class={[
						"mb-4 transition-colors",
						dragOver ? "text-secondary" : "text-on-surface-variant/30"
					]}
				/>
				<p class="font-body text-body-md text-primary mb-2">
					{dragOver ? m.upload_dropzone_active() : m.upload_dropzone()}
				</p>
				<p class="font-label text-label-sm text-on-surface-variant">{m.upload_supported()}</p>
			</div>
			<input
				id="file-input"
				type="file"
				accept=".epub,.pdf,.cbz,.mobi,.azw,.bvir"
				class="hidden"
				onchange={onFileSelect}
			/>
		{/if}
	</div>
</Modal>
