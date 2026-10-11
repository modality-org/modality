#!/usr/bin/env node
// Adds Shiki tokens to every `formula` in a JSON data file, for pages that show
// Modality outside Markdown.
//
//   node scripts/highlight-modality.mjs static/data/multi-agent-chess/rules.json
//
// Markdown gets its tokens at build time (src/plugins/rehypeShikiTokens.ts); a
// React page passes `data-shiki` to CodeBlock itself. Same grammar, same themes,
// same token shape. A long formula is broken after each top-level `&`, and the
// broken text goes in `display`.
import fs from 'fs';
import path from 'path';
import {fileURLToPath} from 'url';
import {createHighlighter} from 'shiki';

const here = path.dirname(fileURLToPath(import.meta.url));
const GRAMMAR = path.resolve(here, '../../../common/modality-vscode/syntaxes/modality.tmLanguage.json');
const THEMES = {light: 'github-light', dark: 'dracula'}; // src/components/shikiTokens.ts
const WIDE = 72;

function breakAtAnd(formula) {
  if (formula.length <= WIDE) return formula;
  const parts = [];
  let depth = 0;
  let cur = '';
  for (const ch of formula) {
    if ('(['.includes(ch)) depth++;
    if (')]'.includes(ch)) depth--;
    if (ch === '&' && depth <= 1) {
      parts.push(cur.trimEnd() + ' &');
      cur = '';
      continue;
    }
    cur += ch;
  }
  parts.push(cur);
  return parts.map((p, i) => (i === 0 ? p : '  ' + p.trimStart())).join('\n');
}

function tokens(hl, code) {
  return hl.codeToTokensWithThemes(code, {lang: 'modality', themes: THEMES}).map((line) => {
    const out = [];
    for (const t of line) {
      const l = t.variants.light;
      const d = t.variants.dark;
      const token = [t.content, l.color ?? '', d.color ?? ''];
      if (l.fontStyle || d.fontStyle) token.push(l.fontStyle ?? 0, d.fontStyle ?? 0);
      const prev = out[out.length - 1];
      if (prev && prev.slice(1).join() === token.slice(1).join()) prev[0] += token[0];
      else out.push(token);
    }
    return out;
  });
}

function walk(node, visit) {
  if (Array.isArray(node)) node.forEach((n) => walk(n, visit));
  else if (node && typeof node === 'object') {
    visit(node);
    Object.values(node).forEach((n) => walk(n, visit));
  }
}

const file = process.argv[2];
if (!file) {
  console.error('usage: node scripts/highlight-modality.mjs <file.json>');
  process.exit(2);
}
const grammar = JSON.parse(fs.readFileSync(GRAMMAR, 'utf8'));
const hl = await createHighlighter({themes: Object.values(THEMES), langs: [{...grammar, name: 'modality'}]});
const data = JSON.parse(fs.readFileSync(file, 'utf8'));
let n = 0;
walk(data, (obj) => {
  if (typeof obj.formula === 'string') {
    obj.display = breakAtAnd(obj.formula);
    obj.tokens = tokens(hl, obj.display);
    n++;
  }
});
fs.writeFileSync(file, JSON.stringify(data, null, 1));
console.log(`Highlighted ${n} formulas in ${file}`);
