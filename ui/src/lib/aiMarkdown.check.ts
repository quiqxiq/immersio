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
ok(hostile.includes('<h1'), 'headings render');
ok(hostile.includes('<strong'), 'bold renders');
ok(hostile.includes('<code'), 'inline code renders');

const fenced = renderAiMarkdown('Before\n\n```js\n<script>alert(1)</script>\n```\n\nAfter');
ok(!fenced.includes('<script>'), 'fenced code is escaped');
ok(fenced.includes('&lt;script&gt;'), 'fenced code keeps the source text');
ok(fenced.includes('<pre'), 'fenced code gets a pre block');

const plain = renderAiMarkdown('line one\nline two\n\n- item one\n- item two');
ok(plain.includes('<br>'), 'single newlines become breaks');
ok(plain.includes('<ul>'), 'bullet lists get a list wrapper');
ok((plain.match(/<li>/g) ?? []).length === 2, 'bullet rows render');

const ordered = renderAiMarkdown('1. first\n2. second');
ok(ordered.includes('<ol>'), 'ordered lists render');

const table = renderAiMarkdown('| Name | Value |\n| --- | ---: |\n| one | 1 |');
ok(table.includes('<table>'), 'GFM tables render');
ok(table.includes('<th>Name</th>'), 'table headers render');

const image = renderAiMarkdown('![diagram](https://example.com/tracker.png)');
ok(!image.includes('<img'), 'images never load remotely');
ok(image.includes('diagram'), 'image alt text is preserved');

const unsafeLink = renderAiMarkdown('[click](javascript:alert(1))');
ok(!/href=["']javascript:/i.test(unsafeLink), 'javascript links are rejected');

const safeLink = renderAiMarkdown('[docs](https://example.com/docs)');
ok(safeLink.includes('href="https://example.com/docs"'), 'https links survive');
ok(safeLink.includes('rel="noopener noreferrer"'), 'links get a safe rel');

const openFence = renderAiMarkdown('```js\nconst x = 1;');
ok(openFence.includes('<pre'), 'an unfinished code fence still renders as code');

ok(renderAiMarkdown('') === '', 'empty input stays empty');
console.log('ok');
