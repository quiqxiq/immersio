// Self-check for the shared AI Markdown renderer. It handles provider output, so the one
// non-negotiable invariant is that raw HTML never survives into the `{@html}` sink.
//
//     node --experimental-strip-types ui/src/lib/aiMarkdown.check.ts
import { renderAiMarkdown } from './aiMarkdown.ts';

function ok(cond: boolean, what: string): void {
	if (!cond) throw new Error(`FAIL: ${what}`);
}

const hostile = renderAiMarkdown('# Title <img src=x onerror="alert(1)">\n\n**bold** and `code`');
ok(!hostile.includes('<img'), 'raw tags are escaped before formatting');
ok(hostile.includes('&lt;img'), 'the tag text is still visible');
ok(hostile.includes('<h2'), 'headings still render');
ok(hostile.includes('<strong'), 'bold still renders');
ok(hostile.includes('<code'), 'inline code still renders');

const fenced = renderAiMarkdown('Before\n\n```js\n<script>alert(1)</script>\n```\n\nAfter');
ok(!fenced.includes('<script>'), 'fenced code is escaped too');
ok(fenced.includes('&lt;script&gt;'), 'fenced code keeps the source text');
ok(fenced.includes('<pre'), 'fenced code gets a pre block');

const plain = renderAiMarkdown('line one\nline two\n\n- item one\n- item two');
ok(plain.includes('<br/>'), 'single newlines become breaks');
ok((plain.match(/<li/g) ?? []).length === 2, 'bullet rows render');

ok(renderAiMarkdown('') === '', 'empty input stays empty');
console.log('ok');
