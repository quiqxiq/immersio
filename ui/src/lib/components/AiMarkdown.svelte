<script lang="ts">
	import * as api from '$lib/api';
	import { renderAiMarkdown } from '$lib/aiMarkdown';

	let {
		content,
		streaming = false,
		class: className = ''
	}: { content: string; streaming?: boolean; class?: string } = $props();

	function handleClick(event: MouseEvent) {
		const target = event.target;
		if (!(target instanceof Element)) return;
		const anchor = target.closest('a');
		if (!anchor) return;
		event.preventDefault();
		const href = (anchor.getAttribute('href') ?? '').trim();
		if (/^https?:\/\//i.test(href)) api.openExternal(href).catch((e) => console.error(e));
	}

	/** Delegate link clicks without making the Markdown wrapper itself look interactive. */
	function externalLinks(node: HTMLElement) {
		const listener = (event: MouseEvent) => handleClick(event);
		node.addEventListener('click', listener);
		return {
			destroy() {
				node.removeEventListener('click', listener);
			}
		};
	}
</script>

<div class="ai-markdown {className}" class:is-streaming={streaming} use:externalLinks>
	{@html renderAiMarkdown(content)}
</div>

<style>
	.ai-markdown {
		overflow-wrap: anywhere;
	}

	.ai-markdown :global(p) {
		margin: 0.35em 0;
	}

	.ai-markdown :global(h1),
	.ai-markdown :global(h2),
	.ai-markdown :global(h3),
	.ai-markdown :global(h4) {
		margin: 0.7em 0 0.3em;
		font-weight: 700;
		line-height: 1.25;
	}

	.ai-markdown :global(h1) {
		font-size: 1.18em;
	}

	.ai-markdown :global(h2) {
		font-size: 1.1em;
	}

	.ai-markdown :global(h3) {
		font-size: 1.03em;
	}

	.ai-markdown :global(h4) {
		font-size: 0.97em;
	}

	.ai-markdown :global(ul),
	.ai-markdown :global(ol) {
		margin: 0.35em 0;
		padding-left: 1.2em;
	}

	.ai-markdown :global(ul) {
		list-style: disc;
	}

	.ai-markdown :global(ol) {
		list-style: decimal;
	}

	.ai-markdown :global(li) {
		margin: 0.15em 0;
	}

	.ai-markdown :global(li::marker) {
		color: var(--primary);
	}

	.ai-markdown :global(li > p) {
		margin: 0;
	}

	.ai-markdown :global(blockquote) {
		margin: 0.5em 0;
		border-left: 2px solid var(--primary);
		padding-left: 0.7em;
		color: var(--muted-foreground);
	}

	.ai-markdown :global(pre) {
		margin: 0.5em 0;
		overflow-x: auto;
		border-radius: 0.5rem;
		background: var(--muted);
		padding: 0.55rem 0.65rem;
		font-size: 0.85em;
		line-height: 1.5;
	}

	.ai-markdown :global(pre code) {
		background: transparent;
		padding: 0;
		color: inherit;
		font-size: inherit;
	}

	.ai-markdown :global(code) {
		border-radius: 0.25rem;
		background: var(--muted);
		padding: 0.08rem 0.25rem;
		color: var(--primary);
		font-family: var(--font-mono);
		font-size: 0.88em;
	}

	.ai-markdown :global(a) {
		color: var(--primary);
		text-decoration: underline;
		text-underline-offset: 2px;
	}

	.ai-markdown :global(table) {
		display: block;
		width: max-content;
		max-width: 100%;
		margin: 0.5em 0;
		overflow-x: auto;
		border-collapse: collapse;
		font-size: 0.92em;
	}

	.ai-markdown :global(th),
	.ai-markdown :global(td) {
		border: 1px solid var(--border);
		padding: 0.3rem 0.45rem;
		text-align: left;
		vertical-align: top;
	}

	.ai-markdown :global(th) {
		background: var(--muted);
		font-weight: 700;
	}

	.ai-markdown :global(hr) {
		margin: 0.8em 0;
		border: 0;
		border-top: 1px solid var(--border);
	}

	.ai-markdown :global(strong) {
		font-weight: 700;
	}

	.ai-markdown :global(del) {
		opacity: 0.7;
	}

	.ai-markdown :global(.ai-markdown-image-alt) {
		color: var(--muted-foreground);
		font-style: italic;
	}

	.ai-markdown.is-streaming > :global(:last-child)::after {
		content: '';
		display: inline-block;
		width: 0.4rem;
		height: 0.9em;
		margin-left: 0.1rem;
		border-radius: 0.1rem;
		background: var(--primary);
		vertical-align: -0.1em;
		animation: ai-markdown-cursor 1s steps(2, start) infinite;
	}

	@keyframes ai-markdown-cursor {
		50% {
			opacity: 0;
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.ai-markdown.is-streaming > :global(:last-child)::after {
			animation: none;
		}
	}
</style>
