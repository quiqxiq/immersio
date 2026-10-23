<script lang="ts">
	import { page } from '$app/state';
	import { fade, fly } from 'svelte/transition';
	import { cubicOut } from 'svelte/easing';
	import { HugeiconsIcon } from '@hugeicons/svelte';
	import {
		AiChat01Icon,
		AlertCircleIcon,
		ArrowLeft01Icon,
		CameraVideoIcon,
		Cancel01Icon,
		ChatAddIcon,
		File01Icon,
		HistoryIcon,
		Loading03Icon,
		MusicNote01Icon,
		SentIcon,
		Settings02Icon
	} from '@hugeicons/core-free-icons';
	import { Button } from '$lib/components/ui/button';
	import ChatHistory from './ChatHistory.svelte';
	import ChatMessage from './ChatMessage.svelte';
	import {
		aiChat,
		attachAvailable,
		closePanel,
		detach,
		newChat,
		refreshThreads,
		send,
		setThinkingMode,
		syncAvailable
	} from '$lib/aiChat.svelte';
	import type { ChatContextKind, ChatContextPayload } from '$lib/api';
	import { ui } from '$lib/player.svelte';
	import { t } from '$lib/i18n.svelte';

	let panelEl = $state<HTMLElement | null>(null);
	let transcriptEl = $state<HTMLElement | null>(null);
	let composerEl = $state<HTMLTextAreaElement | null>(null);
	// Plain variables: these only detect a view/thread switch so the scroll effect can jump to
	// the latest message once, then follow the tail only while the user is already near it.
	let lastThreadId: string | null = null;
	let lastView: 'chat' | 'history' = 'chat';
	let lastOpen = false;
	let lastShown = false;

	const genericSuggestions = [
		{
			label: 'chat.suggestion.grammar',
			prompt: 'Explain a useful English grammar point with two short examples.'
		},
		{
			label: 'chat.suggestion.conversation',
			prompt: 'Start a short, friendly English conversation with me. Ask one question at a time.'
		},
		{
			label: 'chat.suggestion.vocabulary',
			prompt: 'Teach me three useful English words for everyday conversation, with example sentences.'
		}
	] as const;

	// Quick-action prompts are product instructions, not UI copy: keep them in English even when
	// the surrounding interface is Indonesian.
	const quickPrompts: Record<ChatContextKind, string> = {
		article:
			'Summarize this article in English. Focus on its main argument, the key supporting points, and anything a language learner should notice.',
		video:
			'Summarize this video up to the current playback position in English. Use the transcript context and do not invent anything not present there.',
		music:
			'Explain the meaning of the current lyrics in English. Quote only short phrases from the lyrics, then explain them clearly for a language learner.'
	};

	const thinkingModes = [
		{ id: 'auto', label: 'chat.thinking.auto', hint: 'chat.thinking.auto_hint' },
		{ id: 'think', label: 'chat.thinking.think', hint: 'chat.thinking.think_hint' },
		{ id: 'none', label: 'chat.thinking.none', hint: 'chat.thinking.none_hint' }
	] as const;

	function modeButtonClass(id: (typeof thinkingModes)[number]['id']): string {
		return aiChat.thinkingMode === id
			? 'bg-primary text-primary-foreground shadow-sm'
			: 'text-muted-foreground hover:bg-muted hover:text-foreground';
	}

	// The drawer must be mounted even while closed so an in-flight stream and auto-detach keep
	// working. This effect tracks the route, the playing item, and the registered article.
	$effect(() => {
		syncAvailable(page.url);
	});

	$effect(() => {
		if (!aiChat.open || aiChat.view !== 'chat') return;
		const el = composerEl;
		if (!el) return;
		requestAnimationFrame(() => el.focus());
	});

	$effect(() => {
		// Read the tail so the effect reruns on each delta as well as on a new message.
		const threadId = aiChat.threadId;
		const view = aiChat.view;
		const open = aiChat.open;
		const shown = !ui.theaterOpen;
		const switched =
			threadId !== lastThreadId || view !== lastView || open !== lastOpen || shown !== lastShown;
		lastThreadId = threadId;
		lastView = view;
		lastOpen = open;
		lastShown = shown;

		const last = aiChat.messages.at(-1);
		const content = last?.content;
		const count = aiChat.messages.length;
		const el = transcriptEl;
		if (!el || count === 0 || content === undefined) return;
		const nearBottom = switched || el.scrollHeight - el.scrollTop - el.clientHeight < 180;
		if (nearBottom) {
			requestAnimationFrame(() => {
				const node = transcriptEl;
				if (node) node.scrollTop = node.scrollHeight;
			});
		}
	});

	function handleWindowKeydown(event: KeyboardEvent) {
		if (event.key !== 'Escape' || !aiChat.open) return;
		const target = event.target;
		if (target instanceof Node && panelEl?.contains(target)) {
			event.preventDefault();
			closePanel();
		}
	}

	function kindLabel(kind: ChatContextKind): string {
		if (kind === 'article') return t('chat.kind.article');
		if (kind === 'video') return t('chat.kind.video');
		return t('chat.kind.music');
	}

	function kindIcon(kind: ChatContextKind) {
		if (kind === 'video') return CameraVideoIcon;
		if (kind === 'music') return MusicNote01Icon;
		return File01Icon;
	}

	function contextDetail(context: ChatContextPayload): string {
		return [context.source, context.position_label].filter(Boolean).join(' · ');
	}

	function quickLabel(kind: ChatContextKind): string {
		if (kind === 'article') return t('chat.quick.article');
		if (kind === 'video') return t('chat.quick.video');
		return t('chat.quick.music');
	}

	function quickPrompt(kind: ChatContextKind): string {
		return quickPrompts[kind];
	}

	function submit() {
		if (aiChat.streaming || aiChat.attaching) return;
		void send(aiChat.draft);
	}

	function onComposerKeydown(event: KeyboardEvent) {
		if (event.key === 'Enter' && !event.shiftKey && !event.isComposing) {
			event.preventDefault();
			submit();
		}
	}

	function openAiSettings() {
		ui.settingsFocus = 'immersion';
		ui.settingsOpen = true;
	}

	function toggleHistory() {
		if (aiChat.view === 'history') {
			aiChat.view = 'chat';
		} else {
			aiChat.view = 'history';
			void refreshThreads();
		}
	}
</script>

<svelte:window onkeydown={handleWindowKeydown} />

{#if aiChat.open && !ui.theaterOpen}
	<button
		class="absolute inset-0 z-30 cursor-default bg-black/40 lg:hidden"
		onclick={closePanel}
		aria-label={t('chat.close')}
		transition:fade={{ duration: 150 }}
	></button>

	<div
		bind:this={panelEl}
		transition:fly={{ x: 36, duration: 220, easing: cubicOut }}
		class="absolute inset-y-0 right-0 z-40 flex h-full flex-col border-l border-border/70 bg-card/95 shadow-2xl backdrop-blur-sm"
		style="width:min(480px, calc(100vw - 48px));"
		role="dialog"
		aria-modal="false"
		aria-label={t('chat.title')}
	>
		<header class="flex shrink-0 items-center gap-2 border-b border-border/60 px-3 py-2.5">
			<div
				class="flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-primary/15 text-primary"
				aria-hidden="true"
			>
				<HugeiconsIcon icon={AiChat01Icon} class="h-4 w-4" strokeWidth={2} />
			</div>
			<div class="min-w-0 flex-1">
				<div class="flex items-center gap-1.5">
					<h2 class="truncate text-sm font-semibold leading-tight">
						{aiChat.view === 'history' ? t('chat.history.title') : t('chat.title')}
					</h2>
					{#if aiChat.view === 'chat'}
						<span
							class="rounded-full border border-primary/30 bg-primary/10 px-1.5 py-0.5 text-[9px] font-bold uppercase tracking-wide text-primary"
							title={t('chat.english_only')}>EN</span
						>
					{/if}
				</div>
				<p class="truncate text-[10px] text-muted-foreground">
					{aiChat.view === 'history' ? t('chat.history.subtitle') : t('chat.subtitle')}
				</p>
			</div>

			{#if aiChat.view === 'history'}
				<Button
					variant="ghost"
					size="icon-sm"
					class="rounded-full text-muted-foreground hover:text-foreground"
					onclick={toggleHistory}
					title={t('chat.back')}
					aria-label={t('chat.back')}
				>
					<HugeiconsIcon icon={ArrowLeft01Icon} class="h-4 w-4" />
				</Button>
			{:else}
				<Button
					variant="ghost"
					size="icon-sm"
					class="rounded-full text-muted-foreground hover:text-foreground"
					onclick={toggleHistory}
					title={t('chat.recent')}
					aria-label={t('chat.recent')}
				>
					<HugeiconsIcon icon={HistoryIcon} class="h-4 w-4" />
				</Button>
				<Button
					variant="ghost"
					size="icon-sm"
					class="rounded-full text-muted-foreground hover:text-foreground"
					onclick={newChat}
					disabled={aiChat.streaming}
					title={t('chat.new')}
					aria-label={t('chat.new')}
				>
					<HugeiconsIcon icon={ChatAddIcon} class="h-4 w-4" />
				</Button>
			{/if}

			<Button
				variant="ghost"
				size="icon-sm"
				class="rounded-full text-muted-foreground hover:text-foreground"
				onclick={closePanel}
				title={t('chat.close')}
				aria-label={t('chat.close')}
			>
				<HugeiconsIcon icon={Cancel01Icon} class="h-4 w-4" />
			</Button>
		</header>

		{#if aiChat.view === 'history'}
			<ChatHistory />
		{:else}
			<div
				bind:this={transcriptEl}
				class="min-h-0 flex-1 space-y-4 overflow-y-auto px-3 py-4"
				aria-live="polite"
			>
				{#if aiChat.messages.length === 0}
					<div class="flex h-full flex-col">
						<div class="flex flex-col items-center px-3 pb-5 pt-6 text-center">
							<div class="mb-3 flex h-12 w-12 items-center justify-center rounded-full bg-primary/10 text-primary">
								<HugeiconsIcon icon={AiChat01Icon} class="h-6 w-6" strokeWidth={1.7} />
							</div>
							<h3 class="text-base font-semibold">{t('chat.empty_title')}</h3>
							<p class="mt-1 max-w-64 text-xs leading-relaxed text-muted-foreground">
								{t('chat.empty_hint')}
							</p>
						</div>

						{#if aiChat.available && !aiChat.attached}
							{@const available = aiChat.available}
							<div class="mb-4 rounded-2xl border border-primary/20 bg-primary/5 p-3">
								<p class="text-[9px] font-bold uppercase tracking-wider text-primary">
									{t('chat.current_context')}
								</p>
								<div class="mt-1.5 flex items-start gap-2">
									<div class="mt-0.5 text-primary">
										<HugeiconsIcon
											icon={kindIcon(available.kind)}
											class="h-4 w-4"
											strokeWidth={2}
										/>
									</div>
									<div class="min-w-0 flex-1">
										<p class="truncate text-xs font-semibold">{available.title}</p>
										<p class="mt-0.5 truncate text-[10px] text-muted-foreground">
											{kindLabel(available.kind)}
											{#if contextDetail(available)}
												· {contextDetail(available)}
											{/if}
										</p>
									</div>
								</div>
								<div class="mt-2.5 flex flex-wrap gap-1.5">
									<Button
										size="xs"
										class="rounded-full text-[10px]"
										onclick={() => void attachAvailable()}
										disabled={aiChat.attaching}
									>
										{#if aiChat.attaching}
											<HugeiconsIcon icon={Loading03Icon} class="h-3 w-3 animate-spin" />
										{/if}
										{t('chat.attach_this', { kind: kindLabel(available.kind) })}
									</Button>
									<Button
										variant="outline"
										size="xs"
										class="rounded-full bg-background/70 text-[10px]"
										onclick={() => void send(quickPrompt(available.kind), { attachFirst: true })}
										disabled={aiChat.attaching}
									>
										{quickLabel(available.kind)}
									</Button>
								</div>
							</div>
						{/if}

						<div class="mt-auto space-y-1.5">
							<p class="px-1 pb-0.5 text-[10px] font-medium text-muted-foreground">
								{t('chat.try')}
							</p>
							{#each genericSuggestions as suggestion (suggestion.label)}
								<button
									type="button"
									class="flex w-full items-center gap-2 rounded-xl border border-border/60 bg-background/60 px-3 py-2 text-left text-xs transition-colors hover:border-primary/30 hover:bg-primary/5"
									onclick={() => void send(suggestion.prompt)}
								>
									<span class="h-1.5 w-1.5 shrink-0 rounded-full bg-primary/70"></span>
									<span>{t(suggestion.label)}</span>
								</button>
							{/each}
							<p class="px-1 pt-2 text-[10px] leading-relaxed text-muted-foreground">
								{t('chat.english_only')}
							</p>
						</div>
					</div>
				{:else}
					{#each aiChat.messages as message (message.id)}
						<ChatMessage {message} />
					{/each}
				{/if}
			</div>

			<div class="shrink-0 border-t border-border/60 p-3">
				{#if aiChat.notice}
					<div class="mb-2 rounded-xl border border-amber-500/30 bg-amber-500/10 px-2.5 py-2 text-[11px] text-amber-700 dark:text-amber-300">
						<div class="flex items-start gap-1.5">
							<HugeiconsIcon
								icon={AlertCircleIcon}
								class="mt-0.5 h-3.5 w-3.5 shrink-0"
								strokeWidth={2}
							/>
							<p class="min-w-0 flex-1 leading-relaxed">
								{aiChat.notice.newLabel
									? t('chat.context_changed', {
											old: aiChat.notice.oldLabel,
											new: aiChat.notice.newLabel
										})
									: t('chat.context_gone', { old: aiChat.notice.oldLabel })}
							</p>
						</div>
						<div class="mt-1.5 flex items-center gap-2 pl-5">
							{#if aiChat.notice.newLabel && aiChat.available}
								<Button
									size="xs"
									variant="outline"
									class="h-6 rounded-full bg-background/60 px-2 text-[10px]"
									onclick={() => void attachAvailable()}
									disabled={aiChat.attaching}
								>
									{t('chat.attach_new')}
								</Button>
							{/if}
							<button
								type="button"
								class="text-[10px] font-medium text-muted-foreground hover:text-foreground"
								onclick={() => (aiChat.notice = null)}
							>
								{t('chat.dismiss')}
							</button>
						</div>
					</div>
				{/if}

				{#if aiChat.error}
					<div class="mb-2 rounded-xl border border-destructive/30 bg-destructive/10 px-2.5 py-2 text-[11px] text-destructive">
						<div class="flex items-start gap-1.5">
							<HugeiconsIcon
								icon={AlertCircleIcon}
								class="mt-0.5 h-3.5 w-3.5 shrink-0"
								strokeWidth={2}
							/>
							<div class="min-w-0 flex-1">
								<p class="font-medium">{t('chat.error.title')}</p>
								<p class="mt-0.5 line-clamp-3 break-words leading-relaxed text-destructive/85">
									{aiChat.error}
								</p>
							</div>
						</div>
						<Button
							variant="outline"
							size="xs"
							class="mt-1.5 h-6 rounded-full border-destructive/30 bg-background/60 px-2 text-[10px] text-destructive"
							onclick={openAiSettings}
						>
							<HugeiconsIcon icon={Settings02Icon} class="h-3 w-3" />
							{t('chat.error.settings')}
						</Button>
					</div>
				{/if}

				{#if aiChat.attached}
					<div class="mb-2 rounded-xl border border-primary/25 bg-primary/5 px-2.5 py-2">
						<div class="flex items-start gap-2">
							<HugeiconsIcon
								icon={kindIcon(aiChat.attached.kind)}
								class="mt-0.5 h-3.5 w-3.5 shrink-0 text-primary"
								strokeWidth={2}
							/>
							<div class="min-w-0 flex-1">
								<p class="text-[9px] font-bold uppercase tracking-wider text-primary">
									{t('chat.attached')}
								</p>
								<p class="truncate text-xs font-semibold">{aiChat.attached.title}</p>
								{#if contextDetail(aiChat.attached)}
									<p class="truncate text-[10px] text-muted-foreground">
										{contextDetail(aiChat.attached)}
									</p>
								{/if}
							</div>
							<button
								type="button"
								class="rounded-md p-0.5 text-muted-foreground transition-colors hover:bg-destructive/10 hover:text-destructive"
								onclick={detach}
								title={t('chat.detach')}
								aria-label={t('chat.detach')}
							>
								<HugeiconsIcon icon={Cancel01Icon} class="h-3.5 w-3.5" strokeWidth={2} />
							</button>
						</div>
						<p class="mt-1 pl-5 text-[9px] leading-relaxed text-muted-foreground">
							{t('chat.context_sent')}
						</p>
					</div>
				{:else if aiChat.available}
					<button
						type="button"
						class="mb-2 flex w-full items-center gap-2 rounded-xl border border-dashed border-border/80 px-2.5 py-2 text-left text-[11px] text-muted-foreground transition-colors hover:border-primary/40 hover:bg-primary/5 hover:text-foreground"
						onclick={() => void attachAvailable()}
						disabled={aiChat.attaching}
					>
						{#if aiChat.attaching}
							<HugeiconsIcon icon={Loading03Icon} class="h-3.5 w-3.5 shrink-0 animate-spin text-primary" />
						{:else}
							<HugeiconsIcon icon={ChatAddIcon} class="h-3.5 w-3.5 shrink-0 text-primary" />
						{/if}
						<span class="min-w-0 truncate">
							{t('chat.attach_this', { kind: kindLabel(aiChat.available.kind) })}
							· {aiChat.available.title}
						</span>
					</button>
				{/if}

				<div class="mb-2 flex flex-wrap items-center justify-between gap-2 px-0.5">
					<span class="text-[10px] font-medium text-muted-foreground">
						{t('chat.thinking.label')}
					</span>
					<div
						class="inline-flex rounded-full border border-border/70 bg-background/70 p-0.5"
						role="group"
						aria-label={t('chat.thinking.label')}
					>
						{#each thinkingModes as mode (mode.id)}
							<button
								type="button"
								class="rounded-full px-2.5 py-1 text-[10px] font-semibold transition-colors disabled:opacity-50 {modeButtonClass(
									mode.id
								)}"
								aria-pressed={aiChat.thinkingMode === mode.id}
								title={t(mode.hint)}
								disabled={aiChat.streaming || aiChat.attaching}
								onclick={() => setThinkingMode(mode.id)}
							>
								{t(mode.label)}
							</button>
						{/each}
					</div>
				</div>

				<form
					onsubmit={(event) => {
						event.preventDefault();
						submit();
					}}
				>
					<div
						class="flex items-end gap-1.5 rounded-2xl border border-input bg-background/80 p-1.5 transition-colors focus-within:border-ring focus-within:ring-[3px] focus-within:ring-ring/30"
					>
						<textarea
							bind:this={composerEl}
							bind:value={aiChat.draft}
							rows="1"
							class="field-sizing-content max-h-36 min-h-9 flex-1 resize-none bg-transparent px-2 py-1.5 text-sm leading-relaxed outline-none placeholder:text-muted-foreground"
							placeholder={t('chat.placeholder')}
							aria-label={t('chat.placeholder')}
							onkeydown={onComposerKeydown}
						></textarea>
						<Button
							type="submit"
							size="icon-sm"
							class="h-8 w-8 shrink-0 rounded-full"
							disabled={!aiChat.draft.trim() || aiChat.streaming || aiChat.attaching}
							title={aiChat.streaming || aiChat.attaching ? t('chat.streaming') : t('chat.send')}
							aria-label={aiChat.streaming ? t('chat.streaming') : t('chat.send')}
						>
							{#if aiChat.streaming}
								<HugeiconsIcon icon={Loading03Icon} class="h-4 w-4 animate-spin" />
							{:else}
								<HugeiconsIcon icon={SentIcon} class="h-4 w-4" />
							{/if}
						</Button>
					</div>
					<p class="mt-1.5 px-1 text-[10px] leading-relaxed text-muted-foreground">
						{t('chat.english_only')}
					</p>
				</form>
			</div>
		{/if}
	</div>
{/if}
