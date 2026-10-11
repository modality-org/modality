import React, {type ReactNode} from 'react';
import CodeBlock from '@theme-original/CodeBlock';
import type CodeBlockType from '@theme/CodeBlock';
import type {WrapperProps} from '@docusaurus/types';
import {ShikiTokensContext, parseShikiTokens} from '@site/src/components/shikiTokens';

type Props = WrapperProps<typeof CodeBlockType>;

// Takes the build-time Shiki tokens off the block (see
// src/plugins/rehypeShikiTokens.ts) and hands them to CodeBlock/Content.
export default function CodeBlockWrapper(props: Props): ReactNode {
  const {'data-shiki': raw, ...rest} = props as Props & {'data-shiki'?: string};
  return (
    <ShikiTokensContext.Provider value={parseShikiTokens(raw)}>
      <CodeBlock {...rest} />
    </ShikiTokensContext.Provider>
  );
}
