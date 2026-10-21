// Global AI chat: one transcript, optional sticky source attachment.
//
// The backend command is stateless; this module owns the visible thread, persistence calls,
// and the attachment lifecycle. Context text is kept only in memory (and only long enough to
// send it) — chat history stores labels, never the article/lyrics body.
import * as api from './api';
import type {
	ArticleContent,
	ChatContextKind,
	ChatContextPayload,
	ChatMessageRecord,
	ChatThreadSummary,
	ChatTurn,
	LyricLine,
	Lyrics
} from './api';
import { playback, videoTranscript } from './player.svelte';

export interface ChatAttachment extends ChatContextPayload {
	/** Cached so a follow-up does not fetch the same song's lyrics again. */
	lyrics?: Lyrics | null;
}

export interface ChatMessage extends ChatMessageRecord {
	/** True only for the assistant bubble currently receiving deltas. */
	streaming?: boolean;
}

export interface ChatNotice {
	oldLabel: string;
	newLabel: string | null;
}

interface ArticleChatContext {
	routeUrl: string;
	id: string;
	title: string;
	source: string;
	text: string;
	paywall: boolean;
}

let articleContext = $state<ArticleChatContext | null>(null);

export const aiChat = $state({
	open: false,
	view: 'chat' as 'chat' | 'history',
	threadId: null as string | null,
	messages: [] as ChatMessage[],
	threads: [] as ChatThreadSummary[],
	draft: '',
	available: null as ChatContextPayload | null,
	attached: null as ChatAttachment | null,
	attaching: false,
	streaming: false,
	loadingThread: false,
	error: null as string | null,
	notice: null as ChatNotice | null
});

function randomId(): string {
	return (
		globalThis.crypto?.randomUUID?.() ??
		`chat-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 10)}`
	);
}

function nowSecs(): number {
	return Math.floor(Date.now() / 1000);
}

function formatTime(positionSecs: number): string {
	const total = Math.max(0, Math.floor(positionSecs));
	const hours = Math.floor(total / 3600);
	const minutes = Math.floor((total % 3600) / 60);
	const seconds = total % 60;
	return hours > 0
		? `${hours}:${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`
		: `${minutes}:${String(seconds).padStart(2, '0')}`;
}

function formatLineTime(ms: number | undefined): string {
	return formatTime((ms ?? 0) / 1000);
}

function lineIndexAt(lines: LyricLine[], positionSecs: number): number {
	if (!lines.length) return -1;
	const ms = positionSecs * 1000;
	let found = -1;
	for (let i = 0; i < lines.length; i++) {
		if ((lines[i].time_ms ?? Number.POSITIVE_INFINITY) <= ms) found = i;
		else break;
	}
	return found;
}

function windowedLines(lines: LyricLine[], active: number, radius = 28): LyricLine[] {
	if (lines.length <= radius * 2 + 1) return lines;
	const center = active >= 0 ? active : 0;
	const start = Math.max(0, center - radius);
	return lines.slice(start, Math.min(lines.length, center + radius + 1));
}

function buildVideoText(): string {
	const now = playback.now;
	if (!now?.isVideo) return '';
	const active =
		videoTranscript.activeIndex >= 0
			? videoTranscript.activeIndex
			: lineIndexAt(videoTranscript.lines, playback.position);
	const picked = windowedLines(videoTranscript.lines, active);
	const lines = picked.map((line) => `[${formatLineTime(line.time_ms)}] ${line.text}`);
	if (!lines.length) {
		return `Video: ${now.title}\nChannel: ${now.artists}\nCurrent position: ${formatTime(playback.position)}\nNo transcript was available.`;
	}
	return [
		`Video: ${now.title}`,
		`Channel: ${now.artists}`,
		`Current position: ${formatTime(playback.position)}`,
		'',
		...lines
	].join('\n');
}

function buildMusicText(lyrics: Lyrics | null | undefined): string {
	const now = playback.now;
	if (!now || now.isVideo) return '';
	const active = lyrics?.lines?.length
		? lineIndexAt(lyrics.lines, playback.position)
		: -1;
	const picked = lyrics?.lines?.length
		? windowedLines(lyrics.lines, active)
		: [];
	const lines = picked.map((line) => {
		const prefix = line.time_ms !== undefined ? `[${formatLineTime(line.time_ms)}] ` : '';
		return `${prefix}${line.text}`;
	});
	const header = [
		`Song: ${now.title}`,
		`Artist: ${now.artists}`,
		`Current position: ${formatTime(playback.position)}`
	];
	if (!lines.length) {
		return [...header, 'No lyrics were available for this track.'].join('\n');
	}
	return [...header, '', ...lines].join('\n');
}

function musicCandidate(): ChatContextPayload | null {
	const now = playback.now;
	if (!now || now.isVideo) return null;
	return {
		kind: 'music',
		id: now.videoId,
		title: now.title,
		source: now.artists,
		text: ''
	};
}

function videoCandidate(): ChatContextPayload | null {
	const now = playback.now;
	if (!now?.isVideo) return null;
	return {
		kind: 'video',
		id: now.videoId,
		title: now.title,
		source: now.artists,
		text: ''
	};
}

function articleCandidate(currentUrl: URL | null): ChatContextPayload | null {
	if (!articleContext || !currentUrl || currentUrl.pathname !== '/articles/read') return null;
	if (currentUrl.searchParams.get('url') !== articleContext.routeUrl) return null;
	return {
		kind: 'article',
		id: articleContext.id,
		title: articleContext.title,
		source: articleContext.source,
		url: articleContext.id,
		text: articleContext.text
	};
}

/**
 * Recompute what the open page/player can offer. Called from an effect in ChatPanel, so it
 * also runs while the drawer is closed and can auto-detach a stale attachment immediately.
 */
export function syncAvailable(currentUrl: URL | null): void {
	const next =
		articleCandidate(currentUrl) ??
		videoCandidate() ??
		musicCandidate();

	const previous = aiChat.attached;
	aiChat.available = next;

	if (!previous) {
		// A detach notice can outlive the content it offered if the user keeps browsing before
		// choosing a replacement. Keep its "attach new" button and its text in sync.
		if (aiChat.notice && aiChat.notice.newLabel !== (next?.title ?? null)) {
			aiChat.notice = { oldLabel: aiChat.notice.oldLabel, newLabel: next?.title ?? null };
		}
		return;
	}
	if (next && next.kind === previous.kind && next.id === previous.id) return;

	aiChat.attached = null;
	aiChat.notice = { oldLabel: previous.title, newLabel: next?.title ?? null };
}

export function setArticleContext(article: ArticleContent, routeUrl?: string): void {
	articleContext = {
		routeUrl: routeUrl || article.url,
		id: article.url,
		title: article.title || 'Article',
		source: article.source,
		text: article.markdown || article.html.replace(/<[^>]*>/g, ' ').replace(/\s+/g, ' ').trim(),
		paywall: article.is_paywall_blocked
	};
}

export function clearArticleContext(routeUrl: string): void {
	if (articleContext?.routeUrl === routeUrl) articleContext = null;
}

export function openPanel(): void {
	aiChat.open = true;
	aiChat.error = null;
	void refreshThreads();
}

export function closePanel(): void {
	aiChat.open = false;
}

export function togglePanel(): void {
	if (aiChat.open) closePanel();
	else openPanel();
}

export function newChat(): void {
	if (aiChat.streaming) return;
	aiChat.threadId = null;
	aiChat.messages = [];
	aiChat.draft = '';
	aiChat.view = 'chat';
	aiChat.error = null;
	aiChat.notice = null;
	aiChat.attached = null;
}

export async function refreshThreads(): Promise<void> {
	try {
		aiChat.threads = await api.chatListThreads();
	} catch (e) {
		// History is useful but never worth blocking the main chat path for.
		console.error('Failed to load chat threads:', e);
	}
}

export async function loadThread(id: string): Promise<void> {
	if (aiChat.streaming || aiChat.loadingThread) return;
	if (id === aiChat.threadId) {
		// Clicking the already-open thread in history should return to its transcript, not
		// leave the panel stuck on the list.
		aiChat.view = 'chat';
		aiChat.error = null;
		return;
	}
	aiChat.loadingThread = true;
	aiChat.error = null;
	try {
		const thread = await api.chatGetThread(id);
		aiChat.threadId = thread.id;
		aiChat.messages = thread.messages;
		aiChat.view = 'chat';
		aiChat.attached = null;
		aiChat.notice = null;
		aiChat.draft = '';
	} catch (e) {
		aiChat.error = String(e);
	} finally {
		aiChat.loadingThread = false;
	}
}

export async function deleteThread(id: string): Promise<void> {
	if (aiChat.streaming) return;
	try {
		await api.chatDeleteThread(id);
		if (aiChat.threadId === id) newChat();
		await refreshThreads();
	} catch (e) {
		aiChat.error = String(e);
	}
}

export async function clearHistory(): Promise<void> {
	if (aiChat.streaming) return;
	try {
		await api.chatClearHistory();
		newChat();
		await refreshThreads();
	} catch (e) {
		aiChat.error = String(e);
	}
}

async function attach(context: ChatContextPayload): Promise<void> {
	aiChat.attaching = true;
	aiChat.error = null;
	aiChat.notice = null;
	try {
		if (context.kind === 'music') {
			const now = playback.now;
			if (!now || now.isVideo || now.videoId !== context.id) {
				aiChat.error = 'Lagu sudah berubah. Coba lampirkan lagi.';
				return;
			}
			let lyrics: Lyrics | null = null;
			try {
				lyrics = await api.getLyrics({
					videoId: now.videoId,
					title: now.title,
					artists: now.artists,
					album: now.album ?? undefined,
					duration: now.duration ? parseDuration(now.duration) : undefined
				});
			} catch {
				// Metadata is still useful; the chip says no lyrics were available.
			}
			if (playback.now?.videoId !== context.id || playback.now?.isVideo) {
				aiChat.error = 'Lagu sudah berubah. Coba lampirkan lagi.';
				return;
			}
			aiChat.attached = {
				...context,
				text: buildMusicText(lyrics),
				position_ms: Math.round(playback.position * 1000),
				position_label: formatTime(playback.position),
				lyrics
			};
		} else if (context.kind === 'video') {
			const now = playback.now;
			if (!now?.isVideo || now.videoId !== context.id) {
				aiChat.error = 'Video sudah berubah. Coba lampirkan lagi.';
				return;
			}
			aiChat.attached = {
				...context,
				text: buildVideoText(),
				position_ms: Math.round(playback.position * 1000),
				position_label: formatTime(playback.position)
			};
		} else {
			aiChat.attached = { ...context };
		}
	} finally {
		aiChat.attaching = false;
	}
}

export async function attachAvailable(): Promise<void> {
	if (!aiChat.available || aiChat.attaching) return;
	await attach(aiChat.available);
}

export function detach(): void {
	aiChat.attached = null;
	aiChat.notice = null;
}

function parseDuration(value: string): number | undefined {
	const parts = value.split(':').map(Number);
	if (parts.some((part) => Number.isNaN(part))) return undefined;
	return parts.reduce((total, part) => total * 60 + part, 0);
}

function contextForSend(): ChatContextPayload | null {
	const attached = aiChat.attached;
	if (!attached) return null;

	if (attached.kind === 'article') {
		return {
			kind: attached.kind,
			id: attached.id,
			title: attached.title,
			source: attached.source,
			url: attached.url,
			text: attached.text
		};
	}

	const now = playback.now;
	if (!now || now.videoId !== attached.id || (attached.kind === 'video') !== !!now.isVideo) {
		aiChat.attached = null;
		aiChat.notice = { oldLabel: attached.title, newLabel: null };
		return null;
	}

	if (attached.kind === 'video') {
		return {
			kind: attached.kind,
			id: attached.id,
			title: attached.title,
			source: attached.source,
			text: buildVideoText(),
			position_ms: Math.round(playback.position * 1000),
			position_label: formatTime(playback.position)
		};
	}

	return {
		kind: attached.kind,
		id: attached.id,
		title: attached.title,
		source: attached.source,
		text: buildMusicText(attached.lyrics),
		position_ms: Math.round(playback.position * 1000),
		position_label: formatTime(playback.position)
	};
}

function contextFields(context: ChatContextPayload | null) {
	return context
		? {
				context_kind: context.kind as ChatContextKind,
				context_id: context.id,
				context_label: context.title,
				context_detail:
					[context.source, context.position_label].filter(Boolean).join(' · ') || null
			}
		: {
				context_kind: null,
				context_id: null,
				context_label: null,
				context_detail: null
			};
}

function record(
	threadId: string,
	role: 'user' | 'assistant',
	content: string,
	context: ChatContextPayload | null,
	model: string | null = null
): ChatMessageRecord {
	const fields = contextFields(context);
	return {
		id: randomId(),
		thread_id: threadId,
		role,
		content,
		model,
		...fields,
		created_at: nowSecs()
	};
}

let sendLock = false;

export async function send(
	text: string,
	options: { attachFirst?: boolean } = {}
): Promise<void> {
	// The composer disables itself, but a quick action and Enter can still land in the same tick.
	if (sendLock || aiChat.streaming || aiChat.attaching) return;
	sendLock = true;
	try {
		await sendInner(text, options);
	} finally {
		sendLock = false;
	}
}

async function sendInner(
	text: string,
	options: { attachFirst?: boolean } = {}
): Promise<void> {
	const content = text.trim();
	if (!content || aiChat.streaming) return;

	if (options.attachFirst && aiChat.available && !aiChat.attached) {
		await attachAvailable();
		// A quick action promises an answer grounded in the content it names. If the content
		// changed (or failed to attach) while lyrics were loading, abort rather than silently
		// sending a global-chat answer.
		if (aiChat.error || !aiChat.attached) return;
	}

	// If the attached content changed between the last sync and this send, contextForSend
	// detaches it. Abort instead of silently sending the message as a global chat.
	const hadAttachment = !!aiChat.attached;
	const context = contextForSend();
	if (hadAttachment && !context) return;
	let threadId = aiChat.threadId;
	if (!threadId) {
		threadId = randomId();
		try {
			await api.chatCreateThread(threadId, content);
		} catch (e) {
			aiChat.error = String(e);
			return;
		}
		aiChat.threadId = threadId;
	}

	const user = record(threadId, 'user', content, context);
	aiChat.messages.push(user);
	aiChat.draft = '';
	aiChat.error = null;
	aiChat.notice = null;
	try {
		await api.chatAppendMessage(user);
	} catch (e) {
		aiChat.error = String(e);
		return;
	}

	const history: ChatTurn[] = aiChat.messages
		.filter((message) => !message.streaming && message.content.trim())
		.map((message) => ({ role: message.role, content: message.content }));

	aiChat.streaming = true;
	let assistantId: string | null = null;

	try {
		await api.aiChatStream(history, context, (event) => {
			if (event.type === 'start') {
				// A retry or fallback sends another Start. Reuse the bubble instead of
				// leaving the reset one behind as a second, forever-streaming message.
				const existing = assistantId
					? aiChat.messages.find((message) => message.id === assistantId)
					: undefined;
				if (existing) {
					existing.model = event.model;
					existing.streaming = true;
				} else {
					assistantId = randomId();
					aiChat.messages.push({
						...record(threadId!, 'assistant', '', context, event.model),
						id: assistantId,
						streaming: true
					});
				}
				return;
			}
			if (!assistantId) return;
			const assistant = aiChat.messages.find((message) => message.id === assistantId);
			if (!assistant) return;

			if (event.type === 'delta') {
				assistant.content += event.content;
			} else if (event.type === 'reset') {
				assistant.content = '';
				assistant.model = event.model;
			} else if (event.type === 'done') {
				assistant.content = event.content;
				assistant.model = event.model;
				assistant.streaming = false;
				const saved: api.ChatMessageRecord = {
					id: assistant.id,
					thread_id: assistant.thread_id,
					role: assistant.role,
					content: assistant.content,
					model: assistant.model ?? null,
					context_kind: assistant.context_kind ?? null,
					context_id: assistant.context_id ?? null,
					context_label: assistant.context_label ?? null,
					context_detail: assistant.context_detail ?? null,
					created_at: assistant.created_at
				};
				void api.chatAppendMessage(saved).catch((e) => {
					aiChat.error = String(e);
				});
			}
		});
	} catch (e) {
		aiChat.error = String(e);
		if (assistantId) {
			const index = aiChat.messages.findIndex((message) => message.id === assistantId);
			if (index >= 0) aiChat.messages.splice(index, 1);
		}
	} finally {
		aiChat.streaming = false;
		if (assistantId) {
			const assistant = aiChat.messages.find((message) => message.id === assistantId);
			if (assistant) assistant.streaming = false;
		}
		void refreshThreads();
	}
}
