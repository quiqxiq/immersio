/**
 * Small, safe Markdown renderer for AI responses. It deliberately escapes HTML before applying
 * a handful of formatting rules; AI output is never trusted as raw HTML.
 *
 * Shared by the word popup and the global chat panel so both keep the same typography.
 */
export function renderAiMarkdown(md: string): string {
	if (!md) return '';
	return md
		// Escape html tags to avoid XSS
		.replace(/&/g, '&amp;')
		.replace(/</g, '&lt;')
		.replace(/>/g, '&gt;')
		// Fenced code blocks (the content is already escaped)
		.replace(
			/```(?:\w+)?\n?([\s\S]*?)```/g,
			'<pre class="my-2 overflow-x-auto rounded-lg bg-muted p-2 text-[11px] leading-relaxed"><code>$1</code></pre>'
		)
		// Headers
		.replace(/^### (.*$)/gim, '<h4 class="text-xs font-bold text-primary tracking-wide uppercase mt-3 mb-1.5">$1</h4>')
		.replace(/^## (.*$)/gim, '<h3 class="text-sm font-bold text-foreground mt-3 mb-1.5">$1</h3>')
		.replace(/^# (.*$)/gim, '<h2 class="text-base font-bold text-primary mt-2 mb-1">$1</h2>')
		// Bold and italics
		.replace(/\*\*(.*?)\*\*/g, '<strong class="font-bold text-foreground">$1</strong>')
		.replace(/\*(.*?)\*/g, '<em class="italic text-foreground/90">$1</em>')
		// Inline code / keywords
		.replace(/`([^`]+)`/g, '<code class="rounded bg-muted px-1 py-0.5 font-mono text-[11px] text-primary">$1</code>')
		// Bullet points with primary styled bullets
		.replace(/^[•\-\*] (.*$)/gim, '<li class="flex items-start gap-1.5 my-1 text-xs text-foreground/90"><span class="text-primary font-bold select-none">•</span><span>$1</span></li>')
		// Paragraph breaks
		.replace(/\n\n/g, '<div class="h-2"></div>')
		.replace(/\n/g, '<br/>');
}
