<script lang="ts">
	import { onMount } from 'svelte';
	import { HugeiconsIcon } from '@hugeicons/svelte';
	import {
		ExternalLinkIcon,
		Key01Icon,
		RefreshIcon,
		SparklesIcon,
		Tick02Icon,
		ViewIcon,
		ViewOffSlashIcon
	} from '@hugeicons/core-free-icons';
	import { Button } from '$lib/components/ui/button';
	import { Switch } from '$lib/components/ui/switch';
	import { toast } from '$lib/player.svelte';
	import * as api from '$lib/api';

	type Draft = { token: string; accountId: string; show: boolean };
	type TestStatus = { ok: boolean; message: string; model?: string };

	let providers = $state<api.AiProviderInfo[]>([]);
	let drafts = $state<Record<string, Draft>>({});
	let loading = $state(true);
	let saving = $state<Record<string, boolean>>({});
	let testing = $state<Record<string, boolean>>({});
	let testStatus = $state<Record<string, TestStatus | null>>({});

	async function load() {
		loading = true;
		try {
			const next = await api.getAiProviders();
			const nextDrafts: Record<string, Draft> = {};
			for (const provider of next) {
				const previous = drafts[provider.id];
				nextDrafts[provider.id] = {
					token: previous?.token ?? '',
					accountId: provider.account_id || '',
					show: previous?.show ?? false
				};
			}
			providers = next;
			drafts = nextDrafts;
		} catch (e: any) {
			toast.error(`Gagal memuat penyedia AI: ${e?.message || e}`);
		} finally {
			loading = false;
		}
	}

	async function save(provider: api.AiProviderInfo) {
		const draft = drafts[provider.id];
		if (!draft) return;
		saving[provider.id] = true;
		try {
			// Completing the first missing credential should make the provider usable
			// immediately. The switch is disabled while the provider is unconfigured, so
			// this is the expected path (including a Cloudflare token that already exists
			// but is waiting for its Account ID).
			const firstSetup =
				!provider.configured && !!(draft.token.trim() || draft.accountId.trim());
			const enabled = provider.enabled || firstSetup;
			await api.setAiProvider(
				provider.id,
				enabled,
				draft.token.trim() || undefined,
				provider.id === 'cloudflare' ? draft.accountId.trim() : undefined
			);
			draft.token = '';
			testStatus[provider.id] = null;
			await load();
			toast(`${provider.name} berhasil disimpan.`);
		} catch (e: any) {
			toast.error(`Gagal menyimpan ${provider.name}: ${e?.message || e}`);
		} finally {
			saving[provider.id] = false;
		}
	}

	async function toggle(provider: api.AiProviderInfo, enabled: boolean) {
		const previous = provider.enabled;
		provider.enabled = enabled;
		try {
			await api.setAiProvider(provider.id, enabled);
		} catch (e: any) {
			provider.enabled = previous;
			toast.error(`Gagal mengubah status ${provider.name}: ${e?.message || e}`);
		}
	}

	async function test(provider: api.AiProviderInfo) {
		const draft = drafts[provider.id];
		if (!draft) return;
		if (!draft.token.trim() && !provider.has_token) {
			toast.error(`Masukkan API key ${provider.name} terlebih dahulu.`);
			return;
		}
		testing[provider.id] = true;
		testStatus[provider.id] = null;
		try {
			const model = await api.testAiProvider(
				provider.id,
				draft.token.trim() || undefined,
				provider.id === 'cloudflare' ? draft.accountId.trim() || undefined : undefined
			);
			testStatus[provider.id] = {
				ok: true,
				message: `Koneksi ${provider.name} valid!`,
				model
			};
			toast(`Koneksi ${provider.name} aktif! (${model})`);
		} catch (e: any) {
			const message = typeof e === 'string' ? e : e?.message || String(e);
			testStatus[provider.id] = { ok: false, message };
			toast.error(`Koneksi ${provider.name} gagal: ${message}`);
		} finally {
			testing[provider.id] = false;
		}
	}

	onMount(load);
</script>

<div class="space-y-3">
	<div class="rounded-xl border border-border/60 bg-muted/20 px-3.5 py-2.5">
		<div class="flex flex-wrap items-center gap-2 text-[11px] text-muted-foreground">
			<span class="font-semibold text-foreground">Urutan fallback otomatis:</span>
			<span class="rounded-full bg-background px-2 py-0.5 font-medium">1. OpenRouter</span>
			<span class="text-muted-foreground/60">→</span>
			<span class="rounded-full bg-background px-2 py-0.5 font-medium">2. NVIDIA NIM</span>
			<span class="text-muted-foreground/60">→</span>
			<span class="rounded-full bg-background px-2 py-0.5 font-medium">3. Cloudflare</span>
		</div>
		<p class="mt-1 text-[11px] leading-relaxed text-muted-foreground">
			Aktifkan penyedia yang ingin dipakai. Limusic mencoba yang aktif secara berurutan;
			penyedia yang nonaktif atau belum punya token akan dilewati. Jawaban yang sudah
			tersimpan di cache tetap bisa dibuka tanpa koneksi.
		</p>
	</div>

	{#if loading}
		<div class="flex items-center justify-center gap-2 rounded-xl border bg-card px-4 py-8 text-xs text-muted-foreground">
			<HugeiconsIcon icon={RefreshIcon} size={14} class="animate-spin" />
			Memuat konfigurasi penyedia AI...
		</div>
	{:else}
		{#each providers as provider (provider.id)}
			{#if drafts[provider.id]}
				{@const draft = drafts[provider.id]}
				{@const status = testStatus[provider.id]}
				<div class="rounded-xl border border-border/70 bg-card">
					<div class="flex items-start justify-between gap-3 border-b border-border/50 px-4 py-3">
						<div class="min-w-0 space-y-1">
							<div class="flex flex-wrap items-center gap-2">
								<span class="flex h-7 w-7 items-center justify-center rounded-lg bg-primary/10 text-primary">
									<HugeiconsIcon icon={SparklesIcon} size={14} />
								</span>
								<span class="text-sm font-semibold text-foreground">{provider.name}</span>
								<span class="rounded bg-muted px-1.5 py-0.5 font-mono text-[10px] text-muted-foreground">
									{provider.model}
								</span>
							</div>
							<p class="text-xs leading-relaxed text-muted-foreground">{provider.description}</p>
						</div>

						<div class="flex shrink-0 items-center gap-2 pt-0.5">
							<span
								class="rounded-full px-2 py-0.5 text-[10px] font-semibold {provider.enabled && provider.configured
									? 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400'
									: 'bg-muted text-muted-foreground'}"
							>
								{provider.enabled && provider.configured
									? 'Aktif'
									: provider.configured
										? 'Nonaktif'
										: 'Belum siap'}
							</span>
							<Switch
								checked={provider.enabled}
								disabled={!provider.configured}
								aria-label={`Aktifkan ${provider.name}`}
								onCheckedChange={(checked) => toggle(provider, checked)}
							/>
						</div>
					</div>

					<div class="space-y-3 px-4 py-3.5">
						{#if provider.id === 'cloudflare'}
							<label class="block space-y-1">
								<span class="text-[11px] font-medium text-muted-foreground">Account ID</span>
								<input
									type="text"
									bind:value={draft.accountId}
									placeholder="3150... (Cloudflare Account ID)"
									class="w-full rounded-lg border border-input bg-background px-3 py-1.5 font-mono text-xs text-foreground placeholder:text-muted-foreground/60 focus:outline-none focus:ring-1 focus:ring-primary"
								/>
							</label>
						{/if}

						<div class="flex flex-wrap items-center gap-2">
							{#if provider.has_token}
								<span class="inline-flex shrink-0 items-center gap-1 rounded-full bg-emerald-500/10 px-2 py-0.5 text-[10px] font-semibold text-emerald-600 dark:text-emerald-400">
									<HugeiconsIcon icon={Tick02Icon} size={10} />
									Token tersimpan
								</span>
							{/if}
							<div class="relative min-w-[220px] flex-1">
								<div class="pointer-events-none absolute inset-y-0 left-0 flex items-center pl-3 text-muted-foreground">
									<HugeiconsIcon icon={Key01Icon} size={14} />
								</div>
								<input
									type={draft.show ? 'text' : 'password'}
									bind:value={draft.token}
									placeholder={provider.has_token
										? 'Tersimpan — tempel token baru untuk mengganti'
										: 'Tempel API key di sini'}
									class="w-full rounded-lg border border-input bg-background py-1.5 pl-9 pr-10 font-mono text-xs text-foreground placeholder:text-muted-foreground/60 focus:outline-none focus:ring-1 focus:ring-primary"
								/>
								<button
									type="button"
									onclick={() => (draft.show = !draft.show)}
									class="absolute inset-y-0 right-0 flex items-center pr-3 text-muted-foreground transition hover:text-foreground cursor-pointer"
									title={draft.show ? 'Sembunyikan token' : 'Tampilkan token'}
								>
									<HugeiconsIcon icon={draft.show ? ViewOffSlashIcon : ViewIcon} size={14} />
								</button>
							</div>

							<Button
								size="sm"
								variant="secondary"
								onclick={() => save(provider)}
								disabled={saving[provider.id]}
								class="h-8 shrink-0 gap-1 text-xs cursor-pointer"
							>
								{#if saving[provider.id]}
									<HugeiconsIcon icon={RefreshIcon} size={13} class="animate-spin" />
								{:else}
									<HugeiconsIcon icon={Tick02Icon} size={13} />
								{/if}
								Simpan
							</Button>

							<Button
								size="sm"
								variant="outline"
								onclick={() => test(provider)}
								disabled={testing[provider.id] || (!draft.token.trim() && !provider.has_token)}
								class="h-8 shrink-0 gap-1 text-xs cursor-pointer"
							>
								{#if testing[provider.id]}
									<HugeiconsIcon icon={RefreshIcon} size={13} class="animate-spin" />
									Menguji...
								{:else}
									<HugeiconsIcon icon={SparklesIcon} size={13} />
									Tes Koneksi
								{/if}
							</Button>
						</div>

						{#if status}
							<div
								class="flex items-start justify-between gap-2 rounded-lg border px-3 py-2 text-xs {status.ok
									? 'border-emerald-500/30 bg-emerald-500/10 text-emerald-600 dark:text-emerald-400'
									: 'border-destructive/30 bg-destructive/10 text-destructive'}"
							>
								<span class="font-medium leading-relaxed">{status.message}</span>
								{#if status.model}
									<span class="shrink-0 rounded bg-background/50 px-1.5 py-0.5 font-mono text-[10px] opacity-80">
										{status.model}
									</span>
								{/if}
							</div>
						{/if}

						<div class="flex flex-wrap items-center justify-between gap-2 text-[11px] text-muted-foreground">
							<span>
								{#if provider.has_token}
									Token tersimpan. Mengetik token baru akan menggantinya.
								{:else}
									Belum ada token.
								{/if}
								{#if !provider.configured && provider.id === 'cloudflare'}
									Lengkapi Account ID dan token agar bisa diaktifkan.
								{/if}
							</span>
							<button
								type="button"
								onclick={() => api.openExternal(provider.docs_url)}
								class="inline-flex items-center gap-1 rounded-md border border-input bg-background/50 px-2 py-1 font-medium transition hover:bg-accent hover:text-foreground cursor-pointer"
							>
								Dapatkan API key
								<HugeiconsIcon icon={ExternalLinkIcon} size={11} />
							</button>
						</div>
					</div>
				</div>
			{/if}
		{/each}
	{/if}
</div>
