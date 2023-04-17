// @ts-check
// Note: type annotations allow type checking and IDEs autocompletion

const lightCodeTheme = require('prism-react-renderer/themes/github');
const darkCodeTheme = require('prism-react-renderer/themes/dracula');

/** @type {import('@docusaurus/types').Config} */
const config = {
  title: 'Starknet Devnet',
  tagline: 'A Flask wrapper of Starknet state. Similar in purpose to Ganache. \n Aims to mimic Starknet\'s Alpha testnet, but with simplified functionality.',
  url: 'https://github.com',
  baseUrl: '/starknet-devnet/',
  // baseUrl: '/',
  onBrokenLinks: 'warn',
  onBrokenMarkdownLinks: 'warn',
  favicon: 'img/favicon.ico',
  organizationName: 'SpaceShard',
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
          editUrl: 'https://github.com/0xSpaceShard/starknet-devnet/blob/master/page',
        },
        blog: {
          showReadingTime: true,
          editUrl: 'https://github.com/0xSpaceShard/starknet-devnet',
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
      content: `⭐️  &nbsp; If you like Starknet Devnet, give it a star on <a target="_blank" rel="noopener noreferrer" href="https://github.com/0xSpaceShard/starknet-devnet">GitHub</a>! &nbsp; ⭐️`,
    },
      navbar: {
        title: 'Starknet Devnet',
        logo: {
          alt: 'starknet-devnet Logo',
          src: 'https://user-images.githubusercontent.com/2848732/193076972-da6fa36e-11f7-4cb3-aa29-673224f8576d.png',
        },
        items: [
          {
            type: 'doc',
            docId: 'intro',
            position: 'left',
            label: 'Docs',
          },
          {
            href: 'https://github.com/0xSpaceShard/starknet-devnet',
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
          {
            title: 'Community',
            items: [
              // {
              //   label: 'Stack Overflow',
              //   href: 'https://stackoverflow.com/questions/tagged/docusaurus',
              // },
              {
                label: 'Discord',
                href: 'https://discordapp.com/channels/793094838509764618/985824027950055434',
              },
              // {
              //   label: 'Twitter',
              //   href: 'https://twitter.com/docusaurus',
              // },
            ],
          },
          {},{},{},{},
          {
            title: 'More',
            items: [
              {
                label: 'GitHub',
                href: 'https://github.com/0xSpaceShard/starknet-devnet',
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
