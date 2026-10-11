// Shiki tokens passed from src/plugins/rehypeShikiTokens.ts (build time) to
// src/theme/CodeBlock (render time).

import {createContext, useContext} from 'react';

// The site's two color modes. Keep in line with `prism` in docusaurus.config.ts,
// which still colors the block background and Prism-highlighted languages.
export const THEMES = {light: 'github-light', dark: 'dracula'} as const;

// [text, light color, dark color, light font style?, dark font style?]
// Font style is Shiki's bit set: 1 italic, 2 bold, 4 underline.
export type ShikiToken = [string, string, string, number?, number?];
export type ShikiLine = ShikiToken[];

export const ShikiTokensContext = createContext<ShikiLine[] | undefined>(undefined);

export function useShikiTokens(): ShikiLine[] | undefined {
  return useContext(ShikiTokensContext);
}

export function parseShikiTokens(value: unknown): ShikiLine[] | undefined {
  if (typeof value !== 'string') return undefined;
  try {
    const lines = JSON.parse(value);
    return Array.isArray(lines) ? lines : undefined;
  } catch {
    return undefined;
  }
}
