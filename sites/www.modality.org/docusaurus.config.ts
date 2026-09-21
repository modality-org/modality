import {themes as prismThemes} from 'prism-react-renderer';
import type {Config} from '@docusaurus/types';
import type * as Preset from '@docusaurus/preset-classic';
import remarkMath from 'remark-math';
import rehypeKatex from 'rehype-katex';

const config: Config = {
  title: 'Modality',
  tagline: 'Formally verified. Built to scale.',
  favicon: 'img/favicon.ico',

  url: 'https://www.modality.org',
  baseUrl: '/',

  organizationName: 'modality-org',
  projectName: 'modality',
  deploymentBranch: 'gh-pages',
  trailingSlash: false,

  onBrokenLinks: 'warn',
  onBrokenMarkdownLinks: 'warn',

  markdown: {
    format: 'md',
    mermaid: true,
  },

  themes: ['@docusaurus/theme-mermaid'],

  clientModules: ['./src/clientModules/cmdOutputStack.ts'],

  i18n: {
    defaultLocale: 'en',
    locales: ['en'],
  },

  stylesheets: [
    {
      href: 'https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700;800&family=JetBrains+Mono:wght@400;500&display=swap',
      type: 'text/css',
    },
    {
      href: 'https://cdn.jsdelivr.net/npm/katex@0.16.9/dist/katex.min.css',
      type: 'text/css',
      integrity: 'sha384-n8MVd4RsNIU0tAv4ct0nTaAbDJwPJzDEaqSD1odI+WdtXRGWt2kTvGFasHpSy3SV',
      crossorigin: 'anonymous',
    },
  ],

  presets: [
    [
      'classic',
      {
        docs: {
          sidebarPath: './sidebars.ts',
          editUrl: 'https://github.com/modality-org/modality/tree/main/docs/',
          remarkPlugins: [remarkMath],
          rehypePlugins: [rehypeKatex],
        },
        blog: {
          showReadingTime: true,
          blogTitle: 'Modality Blog',
          blogDescription: 'Notes on verifiable contracts, formal verification, and agential cooperation',
          postsPerPage: 10,
          blogSidebarTitle: 'Recent posts',
          blogSidebarCount: 5,
        },
        theme: {
          customCss: './src/css/custom.css',
        },
      } satisfies Preset.Options,
    ],
  ],

  themeConfig: {
    image: 'img/modality-social-card.png',
    navbar: {
      title: 'Modality',
      logo: {
        alt: 'Modality Logo',
        src: 'img/logo.svg',
        srcDark: 'img/logo-dark.svg',
      },
      items: [
        {
          type: 'docSidebar',
          sidebarId: 'docsSidebar',
          position: 'left',
          label: 'Docs',
        },
        {
          to: '/docs/',
          label: 'For agents',
          position: 'left',
        },
        {
          to: '/blog',
          label: 'Blog',
          position: 'left',
        },
        {
          href: 'https://github.com/modality-org/modality',
          label: 'GitHub',
          position: 'right',
        },
        {
          to: '/docs/getting-started/installation',
          label: 'Install',
          position: 'right',
          className: 'navbar-cta',
        },
      ],
    },
    footer: {
      style: 'dark',
      links: [
        {
          title: 'Language',
          items: [
            {
              label: 'Getting Started',
              to: '/docs/getting-started',
            },
            {
              label: 'First contract',
              to: '/docs/getting-started/first-contract',
            },
            {
              label: 'For agents',
              to: '/docs/',
            },
            {
              label: 'Language Reference',
              to: '/docs/language',
            },
          ],
        },
        {
          title: 'Community',
          items: [
            {
              label: 'X (Twitter)',
              href: 'https://x.com/modalitylang',
            },
            {
              label: 'Discord',
              href: 'https://discord.gg/KpYFdrfnkS',
            },
            {
              label: 'Discuss',
              href: 'https://discuss.modality.org/',
            },
            {
              label: 'GitHub',
              href: 'https://github.com/modality-org/modality',
            },
          ],
        },
      ],
      copyright: `Copyright © ${new Date().getFullYear()} Modality. Inspired by <a href="https://scholar.google.com/citations?user=kXVBr20AAAAJ&hl=en&oi=ao" target="_blank">Bud Mishra</a>'s work on formal verification.`,
    },
    mermaid: {
      theme: {light: 'neutral', dark: 'dark'},
    },
    prism: {
      theme: prismThemes.github,
      darkTheme: prismThemes.dracula,
      additionalLanguages: ['bash', 'json'],
    },
    colorMode: {
      defaultMode: 'dark',
      disableSwitch: false,
      respectPrefersColorScheme: true,
    },
  } satisfies Preset.ThemeConfig,
};

export default config;
