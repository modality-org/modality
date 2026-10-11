import React, {type CSSProperties, type ComponentProps, type ReactNode} from 'react';
import clsx from 'clsx';
import {useCodeBlockContext} from '@docusaurus/theme-common/internal';
import {usePrismTheme} from '@docusaurus/theme-common';
import {Highlight} from 'prism-react-renderer';
import type {Props} from '@theme/CodeBlock/Content';
import Line from '@theme/CodeBlock/Line';
import {useShikiTokens, type ShikiLine, type ShikiToken} from '@site/src/components/shikiTokens';

import styles from './styles.module.css';

// TODO Docusaurus v4: remove useless forwardRef
const Pre = React.forwardRef<HTMLPreElement, ComponentProps<'pre'>>(
  (props, ref) => {
    return (
      <pre
        ref={ref}
        /* eslint-disable-next-line jsx-a11y/no-noninteractive-tabindex */
        tabIndex={0}
        {...props}
        className={clsx(props.className, styles.codeBlock, 'thin-scrollbar')}
      />
    );
  },
);

function Code(props: ComponentProps<'code'>) {
  const {metadata} = useCodeBlockContext();
  return (
    <code
      {...props}
      className={clsx(
        props.className,
        styles.codeBlockLines,
        metadata.lineNumbersStart !== undefined &&
          styles.codeBlockLinesWithNumbering,
      )}
      style={{
        ...props.style,
        counterReset:
          metadata.lineNumbersStart === undefined
            ? undefined
            : `line-count ${metadata.lineNumbersStart - 1}`,
      }}
    />
  );
}

// Shiki's font-style bits as CSS variables, per color mode.
function fontVars(mode: 'light' | 'dark', bits: number | undefined): Record<string, string> {
  if (!bits) return {};
  const vars: Record<string, string> = {};
  if (bits & 1) vars[`--shiki-${mode}-font-style`] = 'italic';
  if (bits & 2) vars[`--shiki-${mode}-font-weight`] = 'bold';
  if (bits & 4) vars[`--shiki-${mode}-text-decoration`] = 'underline';
  return vars;
}

function tokenStyle([, light, dark, lightFont, darkFont]: ShikiToken): CSSProperties {
  return {
    '--shiki-light': light,
    '--shiki-dark': dark,
    ...fontVars('light', lightFont),
    ...fontVars('dark', darkFont),
  } as CSSProperties;
}

// Build-time Shiki tokens for this block, when they spell exactly the code
// Docusaurus shows (magic comments, for one, change the code).
function useMatchingShikiTokens(code: string): ShikiLine[] | undefined {
  const lines = useShikiTokens();
  if (!lines) return undefined;
  const text = lines.map((line) => line.map((t) => t[0]).join('')).join('\n');
  return text === code ? lines : undefined;
}

export default function CodeBlockContent({
  className: classNameProp,
}: Props): ReactNode {
  const {metadata, wordWrap} = useCodeBlockContext();
  const prismTheme = usePrismTheme();
  const {code, language, lineNumbersStart, lineClassNames} = metadata;
  const shikiLines = useMatchingShikiTokens(code);
  if (shikiLines) {
    return (
      <Pre
        ref={wordWrap.codeBlockRef}
        className={clsx(classNameProp, `language-${language}`)}
        style={{
          color: prismTheme.plain.color,
          backgroundColor: prismTheme.plain.backgroundColor,
        }}>
        <Code>
          {shikiLines.map((line, i) => (
            <Line
              key={i}
              line={line.map((t) => ({types: ['plain'], content: t[0], style: tokenStyle(t)}))}
              getLineProps={({className}) => ({
                className: clsx('token-line', className),
              })}
              getTokenProps={({token}) => ({
                className: 'shiki-token',
                style: (token as {style?: CSSProperties}).style,
                children: token.content,
              })}
              classNames={lineClassNames[i]}
              showLineNumbers={lineNumbersStart !== undefined}
            />
          ))}
        </Code>
      </Pre>
    );
  }
  return (
    <Highlight theme={prismTheme} code={code} language={language}>
      {({className, style, tokens: lines, getLineProps, getTokenProps}) => (
        <Pre
          ref={wordWrap.codeBlockRef}
          className={clsx(classNameProp, className)}
          style={style}>
          <Code>
            {lines.map((line, i) => (
              <Line
                key={i}
                line={line}
                getLineProps={getLineProps}
                getTokenProps={getTokenProps}
                classNames={lineClassNames[i]}
                showLineNumbers={lineNumbersStart !== undefined}
              />
            ))}
          </Code>
        </Pre>
      )}
    </Highlight>
  );
}
