// Highlights fenced code with Shiki at build time.
//
// Docusaurus only gives a code block its copy button, title and line numbers
// when the block's children are a plain string, so this plugin leaves the
// text alone. It adds the tokens as a `data-shiki` attribute on the <code>
// element; src/theme/CodeBlock reads it and paints the lines. Fences in a
// language Shiki does not know keep Docusaurus's Prism highlighting.
//
// `modality` uses the grammar the editor extension ships, so the site and the
// editor highlight the same way.

import fs from 'fs';
import path from 'path';
import {
  bundledLanguages,
  type BundledLanguage,
  createHighlighter,
  type Highlighter,
  type LanguageRegistration,
} from 'shiki';
import {THEMES, type ShikiLine, type ShikiToken} from '../components/shikiTokens';

const MODALITY_GRAMMAR = path.resolve(
  __dirname,
  '../../../../common/modality-vscode/syntaxes/modality.tmLanguage.json',
);

// Rendered by other components, not as code blocks.
const SKIP = new Set(['mermaid']);

let highlighter: Promise<Highlighter> | undefined;

function getHighlighter(): Promise<Highlighter> {
  highlighter ??= (async () => {
    const modality = JSON.parse(fs.readFileSync(MODALITY_GRAMMAR, 'utf8'));
    return createHighlighter({
      themes: Object.values(THEMES),
      langs: [{...modality, name: 'modality'} as LanguageRegistration],
    });
  })();
  return highlighter;
}

type HastNode = {
  type: string;
  tagName?: string;
  value?: string;
  properties?: Record<string, unknown>;
  children?: HastNode[];
};

function codeElements(node: HastNode, out: HastNode[] = []): HastNode[] {
  if (node.type === 'element' && node.tagName === 'pre') {
    for (const child of node.children ?? []) {
      if (child.type === 'element' && child.tagName === 'code') out.push(child);
    }
  }
  for (const child of node.children ?? []) codeElements(child, out);
  return out;
}

function language(code: HastNode): string | undefined {
  const classes = code.properties?.className;
  const list = Array.isArray(classes) ? classes : [];
  const cls = list.find((c) => typeof c === 'string' && c.startsWith('language-'));
  return typeof cls === 'string' ? cls.slice('language-'.length) : undefined;
}

function text(node: HastNode): string {
  if (node.type === 'text') return node.value ?? '';
  return (node.children ?? []).map(text).join('');
}

async function tokenize(hl: Highlighter, code: string, lang: string): Promise<ShikiLine[] | undefined> {
  if (lang !== 'modality') {
    if (!(lang in bundledLanguages)) return undefined;
    if (!hl.getLoadedLanguages().includes(lang)) {
      await hl.loadLanguage(lang as BundledLanguage);
    }
  }
  const lines = hl.codeToTokensWithThemes(code, {lang: lang as BundledLanguage, themes: THEMES});
  return lines.map((line) => {
    const out: ShikiLine = [];
    for (const t of line) {
      const l = t.variants.light;
      const d = t.variants.dark;
      const token: ShikiToken = [t.content, l.color ?? '', d.color ?? ''];
      if (l.fontStyle || d.fontStyle) token.push(l.fontStyle ?? 0, d.fontStyle ?? 0);
      // Merge neighbours that look the same: smaller pages, same picture.
      const prev = out[out.length - 1];
      if (prev && prev.slice(1).join() === token.slice(1).join()) {
        prev[0] += token[0];
      } else {
        out.push(token);
      }
    }
    return out;
  });
}

export default function rehypeShikiTokens() {
  return async (tree: HastNode) => {
    const codes = codeElements(tree);
    if (codes.length === 0) return;
    const hl = await getHighlighter();
    for (const code of codes) {
      const lang = language(code);
      if (!lang || SKIP.has(lang)) continue;
      const lines = await tokenize(hl, text(code).replace(/\n$/, ''), lang);
      if (lines) code.properties = {...code.properties, dataShiki: JSON.stringify(lines)};
    }
  };
}
