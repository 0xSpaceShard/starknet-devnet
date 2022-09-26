// @ts-check
// Note: type annotations allow type checking and IDEs autocompletion

const lightCodeTheme = require('prism-react-renderer/themes/github');
const darkCodeTheme = require('prism-react-renderer/themes/dracula');

/** @type {import('@docusaurus/types').Config} */
const config = {
  title: 'StarkNet Devnet',
  tagline: 'A Flask wrapper of Starknet state. Similar in purpose to Ganache.',
  url: 'https://github.com',
  baseUrl: '/starknet-devnet/',
  // baseUrl: '/',
  onBrokenLinks: 'warn',
  onBrokenMarkdownLinks: 'warn',
  favicon: 'img/favicon.ico',
  organizationName: 'Shard-Labs',
  projectName: 'starknet-devnet',
  deploymentBranch: "gh-pages",

  i18n: {
    defaultLocale: 'en',
    locales: ['en'],
  },

  presets: [
    [
      'classic',
      /** @type {import('@docusaurus/preset-classic').Options} */
      ({
        docs: {
          sidebarPath: require.resolve('./sidebars.js'),
          editUrl: 'https://github.com/Shard-Labs/starknet-devnet',
        },
        blog: {
          showReadingTime: true,
          editUrl: 'https://github.com/Shard-Labs/starknet-devnet',
        },
        theme: {
          customCss: require.resolve('./src/css/custom.css'),
        },
      }),
    ],
  ],

  themeConfig:
    /** @type {import('@docusaurus/preset-classic').ThemeConfig} */
    ({
      announcementBar: {
      id: "support_us",
      content: `⭐️  &nbsp; If you like Starknet Devnet, give it a star on <a target="_blank" rel="noopener noreferrer" href="https://github.com/Shard-Labs/starknet-devnet">GitHub</a>! &nbsp; ⭐️`,
    },
      navbar: {
        title: 'Starknet Devnet',
        logo: {
          alt: 'starknet-devnet Logo',
          src: 'img/logo.svg',
        },
        items: [
          {
            type: 'doc',
            docId: 'intro',
            position: 'left',
            label: 'Docs',
          },
          {
            href: 'https://github.com/Shard-Labs/starknet-devnet',
            label: 'GitHub',
            position: 'right',
          },
        ],
      },
      footer: {
        style: 'dark',
        links: [
          {
            title: 'Docs',
            items: [
              {
                label: 'Get Started',
                to: '/docs/intro',
              },
            ],
          },
          // {
          //   title: 'Community',
          //   items: [
          //     {
          //       label: 'Stack Overflow',
          //       href: 'https://stackoverflow.com/questions/tagged/docusaurus',
          //     },
          //     {
          //       label: 'Discord',
          //       href: 'https://discordapp.com/invite/docusaurus',
          //     },
          //     {
          //       label: 'Twitter',
          //       href: 'https://twitter.com/docusaurus',
          //     },
          //   ],
          // },
          {},{},{},{},
          {
            title: 'More',
            items: [
              {
                label: 'GitHub',
                href: 'https://github.com/Shard-Labs/starknet-devnet',
              },
            ],
          },
        ],
        copyright: `Copyright © ${new Date().getFullYear()} starknet-devnet`,
      },
      prism: {
        theme: lightCodeTheme,
        darkTheme: darkCodeTheme,
      },
    }),
};

module.exports = config;
