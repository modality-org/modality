import type {SidebarsConfig} from '@docusaurus/plugin-content-docs';

const sidebars: SidebarsConfig = {
  docsSidebar: [
    'for-agents',
    {
      type: 'category',
      label: 'Getting Started',
      items: [
        'getting-started/index',
        'getting-started/installation',
        'getting-started/first-contract',
      ],
    },
    {
      type: 'category',
      label: 'Core Concepts',
      items: [
        'concepts/index',
        'concepts/append-only-logs',
        'concepts/state-machines',
        'concepts/modal-logic',
        'concepts/predicates',
        'concepts/potentialism',
      ],
    },
    {
      type: 'category',
      label: 'Language Reference',
      items: [
        'language/index',
        'language/model-syntax',
        'language/rule-syntax',
        'language/predicates',
        'language/path-types',
      ],
    },
    {
      type: 'category',
      label: 'CLI Reference',
      items: [
        'cli/index',
        'cli/contract-commands',
        'cli/identity-commands',
        'cli/node-commands',
        'cli/hub-commands',
        'cli/predicate-commands',
        'cli/network-commands',
      ],
    },
    {
      type: 'category',
      label: 'Tutorials',
      items: [
        'tutorials/multi-party-contract',
        'tutorials/multisig-treasury',
        'tutorials/oracle-escrow',
        'tutorials/contract-hub',
        'tutorials/hub-and-assets',
        'tutorials/js-sdk-hub',
      ],
    },
    {
      type: 'category',
      label: 'Reference',
      items: [
        'reference/standard-predicates',
      ],
    },
    {
      type: 'category',
      label: 'Resources',
      items: [
        'resources/rfc-0001',
        'resources/potentialist-lts',
      ],
    },
    'faq',
  ],
};

export default sidebars;
