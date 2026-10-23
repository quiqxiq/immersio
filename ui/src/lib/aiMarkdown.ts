import MarkdownIt from 'markdown-it';

/**
 * Shared Markdown renderer for AI output. AI responses are untrusted text, so raw HTML is
 * disabled outright; the parser only emits the tags it owns. Links are restricted to http(s),
 * and images are rendered as alt text so a response cannot trigger remote loads.
 */
const md = new MarkdownIt({
	html: false,
	breaks: true,
	linkify: true,
	typographer: false
});

md.validateLink = (url) => /^https?:\/\//i.test(url.trim());

md.renderer.rules.link_open = (tokens, idx, options, env, self) => {
	tokens[idx].attrSet('target', '_blank');
	tokens[idx].attrSet('rel', 'noopener noreferrer');
	return self.renderToken(tokens, idx, options);
};

md.renderer.rules.image = (tokens, idx) => {
	const alt = tokens[idx].content || 'image';
	return `<span class="ai-markdown-image-alt">${md.utils.escapeHtml(alt)}</span>`;
};

/** Close a fenced code block that is still arriving, so a half-streamed answer stays readable. */
function closeOpenFence(source: string): string {
	const fences = source.match(/^[ \t]*(`{3,}|~{3,})/gm);
	if (!fences || fences.length % 2 === 0) return source;
	const opener = fences[fences.length - 1].trim();
	return `${source}\n${opener}`;
}

/** Render untrusted Markdown into safe HTML for the AI chat and word popup. */
export function renderAiMarkdown(source: string): string {
	if (!source) return '';
	return md.render(closeOpenFence(source));
}
