<script lang="ts">
	import { HugeiconsIcon } from '@hugeicons/svelte';
	import {
		Delete02Icon,
		HistoryIcon,
		Loading03Icon
	} from '@hugeicons/core-free-icons';
	import { Button } from '$lib/components/ui/button';
	import { aiChat, clearHistory, deleteThread, loadThread } from '$lib/aiChat.svelte';
	import { currentLocale, t } from '$lib/i18n.svelte';

	let pendingDelete = $state<string | null>(null);
	let clearArmed = $state(false);

	function relativeTime(seconds: number): string {
		const diffMs = seconds * 1000 - Date.now();
		const abs = Math.abs(diffMs);
		if (abs < 60_000) return t('chat.history.just_now');
		const formatter = new Intl.RelativeTimeFormat(currentLocale.id, { numeric: 'auto' });
		if (abs < 3_600_000) return formatter.format(Math.round(diffMs / 60_000), 'minute');
		if (abs < 86_400_000) return formatter.format(Math.round(diffMs / 3_600_000), 'hour');
		if (abs < 7 * 86_400_000) return formatter.format(Math.round(diffMs / 86_400_000), 'day');
		return new Date(seconds * 1000).toLocaleDateString(currentLocale.id, {
			month: 'short',
			day: 'numeric'
		});
	}

	async function remove(id: string) {
		pendingDelete = null;
		await deleteThread(id);
	}

	async function clearAll() {
		clearArmed = false;
		await clearHistory();
	}
</script>

<div class="flex min-h-0 flex-1 flex-col">
	{#if aiChat.error}
		<div class="shrink-0 border-b border-destructive/30 bg-destructive/10 px-3 py-2 text-[11px] leading-relaxed text-destructive">
			{aiChat.error}
		</div>
	{/if}
	{#if aiChat.loadingThread}
		<div class="flex flex-1 items-center justify-center gap-2 text-xs text-muted-foreground">
			<HugeiconsIcon icon={Loading03Icon} class="h-4 w-4 animate-spin" />
			{t('common.loading')}
		</div>
	{:else if aiChat.threads.length === 0}
		<div class="flex flex-1 flex-col items-center justify-center px-8 text-center">
			<div class="mb-3 flex h-11 w-11 items-center justify-center rounded-full bg-muted text-muted-foreground">
				<HugeiconsIcon icon={HistoryIcon} class="h-5 w-5" strokeWidth={1.7} />
			</div>
			<p class="text-sm font-medium">{t('chat.history.empty')}</p>
			<p class="mt-1 max-w-56 text-xs leading-relaxed text-muted-foreground">
				{t('chat.history.empty_hint')}
			</p>
		</div>
	{:else}
		<div class="min-h-0 flex-1 overflow-y-auto p-2">
			{#each aiChat.threads as thread (thread.id)}
				<div
					class="group flex items-start gap-1 rounded-xl p-1 transition-colors {thread.id ===
					aiChat.threadId
						? 'bg-accent/15'
						: 'hover:bg-accent/10'}"
				>
					<button
						type="button"
						class="min-w-0 flex-1 rounded-lg px-2 py-1.5 text-left disabled:cursor-not-allowed disabled:opacity-60"
						onclick={() => void loadThread(thread.id)}
						disabled={aiChat.streaming || aiChat.loadingThread}
						title={t('chat.history.open')}
					>
						<div class="flex items-start gap-2">
							<div class="min-w-0 flex-1">
								<p class="truncate text-xs font-semibold leading-snug">{thread.title}</p>
								{#if thread.preview}
									<p class="mt-0.5 line-clamp-2 text-[11px] leading-snug text-muted-foreground">
										{thread.preview}
									</p>
								{/if}
								<div class="mt-1 flex flex-wrap items-center gap-1.5 text-[9px] text-muted-foreground">
									<span>{relativeTime(thread.updated_at)}</span>
									{#if thread.last_context_label}
										<span
											class="inline-flex min-w-0 items-center gap-0.5 rounded-full bg-muted px-1.5 py-0.5"
										>
											<span class="truncate">{thread.last_context_label}</span>
										</span>
									{/if}
								</div>
							</div>
						</div>
					</button>

					<button
						type="button"
						class="mt-0.5 flex h-7 min-w-7 items-center justify-center rounded-md px-1 text-[9px] font-semibold transition-colors {pendingDelete ===
						thread.id
							? 'bg-destructive/10 text-destructive'
							: 'text-muted-foreground opacity-0 hover:bg-destructive/10 hover:text-destructive group-hover:opacity-100 group-focus-within:opacity-100 focus-visible:opacity-100'}"
						onclick={() =>
							pendingDelete === thread.id ? void remove(thread.id) : (pendingDelete = thread.id)}
						disabled={aiChat.streaming}
						title={
							pendingDelete === thread.id
								? t('chat.history.confirm_delete')
								: t('chat.history.delete')
						}
						aria-label={
							pendingDelete === thread.id
								? t('chat.history.confirm_delete')
								: t('chat.history.delete')
						}
					>
						{#if pendingDelete === thread.id}
							{t('chat.history.confirm_delete_short')}
						{:else}
							<HugeiconsIcon icon={Delete02Icon} class="h-3.5 w-3.5" strokeWidth={2} />
						{/if}
					</button>
				</div>
			{/each}
		</div>

		<div class="border-t p-3">
			<Button
				variant="ghost"
				size="sm"
				class="w-full text-xs text-muted-foreground hover:text-destructive"
				onclick={() => (clearArmed ? void clearAll() : (clearArmed = true))}
				disabled={aiChat.streaming}
			>
				<HugeiconsIcon icon={Delete02Icon} class="h-3.5 w-3.5" />
				{clearArmed ? t('chat.history.confirm_clear') : t('chat.history.clear_all')}
			</Button>
		</div>
	{/if}
</div>
