<script lang="ts">
	import { HugeiconsIcon } from '@hugeicons/svelte';
	import {
		AiChat01Icon,
		CameraVideoIcon,
		Copy01Icon,
		File01Icon,
		MusicNote01Icon,
		Tick02Icon
	} from '@hugeicons/core-free-icons';
	import { renderAiMarkdown } from '$lib/aiMarkdown';
	import { copyText } from '$lib/clipboard';
	import { t } from '$lib/i18n.svelte';
	import type { ChatMessage } from '$lib/aiChat.svelte';

	let { message }: { message: ChatMessage } = $props();

	let copied = $state(false);
	let copyTimer: ReturnType<typeof setTimeout> | undefined;

	const isUser = $derived(message.role === 'user');
	const kind = $derived(message.context_kind ?? null);
	const contextTitle = $derived(message.context_label ?? '');
	const contextDetail = $derived(message.context_detail ?? '');

	function kindLabel(value: string): string {
		if (value === 'article') return t('chat.kind.article');
		if (value === 'video') return t('chat.kind.video');
		if (value === 'music') return t('chat.kind.music');
		return value;
	}

	function kindIcon(value: string) {
		if (value === 'video') return CameraVideoIcon;
		if (value === 'music') return MusicNote01Icon;
		return File01Icon;
	}

	async function copyMessage() {
		try {
			await copyText(message.content);
			copied = true;
			clearTimeout(copyTimer);
			copyTimer = setTimeout(() => (copied = false), 1600);
		} catch {
			// Clipboard failures are not worth interrupting the conversation for.
		}
	}
</script>

<div class="flex gap-2.5 {isUser ? 'justify-end' : 'justify-start'}">
	{#if !isUser}
		<div
			class="mt-0.5 flex h-7 w-7 shrink-0 items-center justify-center rounded-full bg-primary/15 text-primary"
			aria-hidden="true"
		>
			<HugeiconsIcon icon={AiChat01Icon} class="h-3.5 w-3.5" strokeWidth={2} />
		</div>
	{/if}

	<div class="min-w-0 max-w-[86%] {isUser ? 'items-end' : ''}">
		{#if kind && contextTitle}
			<div
				class="mb-1 flex min-w-0 items-center gap-1 text-[10px] text-muted-foreground {isUser
					? 'justify-end'
					: ''}"
				title={contextDetail || contextTitle}
			>
				<HugeiconsIcon icon={kindIcon(kind)} class="h-3 w-3 shrink-0" strokeWidth={2} />
				<span class="truncate">
					{isUser
						? `${kindLabel(kind)} · ${contextTitle}`
						: t('chat.based_on', { kind: kindLabel(kind), title: contextTitle })}
				</span>
			</div>
		{/if}

		<div
			class="rounded-2xl px-3 py-2 text-sm {isUser
				? 'rounded-br-md bg-primary text-primary-foreground'
				: 'rounded-bl-md border border-border/60 bg-muted/55 text-foreground'}"
			aria-live={message.streaming ? 'polite' : undefined}
		>
			{#if isUser}
				<p class="whitespace-pre-wrap break-words leading-relaxed">{message.content}</p>
			{:else if !message.content && message.streaming}
				<span class="inline-flex items-center gap-1.5 text-xs text-muted-foreground">
					<span class="h-1.5 w-1.5 animate-pulse rounded-full bg-primary"></span>
					{t('chat.streaming')}
				</span>
			{:else}
				<div class="ai-chat-markdown break-words leading-relaxed">
					{@html renderAiMarkdown(message.content)}
					{#if message.streaming}
						<span
							class="ml-0.5 inline-block h-3.5 w-1.5 animate-pulse rounded-sm bg-primary align-[-2px]"
						></span>
					{/if}
				</div>
			{/if}
		</div>

		{#if !isUser && message.content && !message.streaming}
			<div class="mt-1 flex items-center gap-2 px-1 text-[10px] text-muted-foreground">
				<button
					type="button"
					class="inline-flex items-center gap-1 rounded transition-colors hover:text-foreground"
					onclick={copyMessage}
					title={t('chat.copy')}
					aria-label={t('chat.copy')}
				>
					<HugeiconsIcon
						icon={copied ? Tick02Icon : Copy01Icon}
						class="h-3 w-3"
						strokeWidth={2}
					/>
					{copied ? t('chat.copied') : t('chat.copy')}
				</button>
				{#if message.model}
					<span class="truncate opacity-70">{message.model}</span>
				{/if}
			</div>
		{/if}
	</div>
</div>

<style>
	.ai-chat-markdown :global(p) {
		margin: 0.35rem 0;
	}
	.ai-chat-markdown :global(ul) {
		margin: 0.35rem 0;
	}
	.ai-chat-markdown :global(li) {
		list-style: none;
	}
</style>
