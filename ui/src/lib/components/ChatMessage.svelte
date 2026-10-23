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
	import AiMarkdown from '$lib/components/AiMarkdown.svelte';
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
			{:else if !message.content.trim() && message.streaming}
				<span
					class="inline-flex items-center gap-2 text-xs text-muted-foreground"
					role="status"
					aria-live="polite"
				>
					<span class="thinking-dots" aria-hidden="true">
						<span></span>
						<span></span>
						<span></span>
					</span>
					{message.thinkingMode === 'think'
						? t('chat.thinking.active_deep')
						: t('chat.thinking.active')}
				</span>
			{:else}
				<AiMarkdown content={message.content} streaming={message.streaming} />
			{/if}
		</div>

		{#if !isUser && message.content.trim() && !message.streaming}
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
	.thinking-dots {
		display: inline-flex;
		align-items: center;
		gap: 0.2rem;
	}

	.thinking-dots span {
		width: 0.28rem;
		height: 0.28rem;
		border-radius: 9999px;
		background: var(--primary);
		animation: thinking-dot 1.2s infinite ease-in-out;
	}

	.thinking-dots span:nth-child(2) {
		animation-delay: 0.15s;
	}

	.thinking-dots span:nth-child(3) {
		animation-delay: 0.3s;
	}

	@keyframes thinking-dot {
		0%,
		70%,
		100% {
			opacity: 0.25;
			transform: translateY(0);
		}
		35% {
			opacity: 1;
			transform: translateY(-0.16rem);
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.thinking-dots span {
			animation: none;
			opacity: 0.7;
		}
	}
</style>
