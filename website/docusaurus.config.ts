import { themes as prismThemes } from "prism-react-renderer";
import type { Config } from "@docusaurus/types";
import type * as Preset from "@docusaurus/preset-classic";

const GITHUB_REPO_URL = "https://github.com/0xSpaceShard/starknet-devnet-rs";

const config: Config = {
  title: "Starknet Devnet",
  tagline: "A local testnet for Starknet... in Rust",
  favicon: "img/favicon.ico",

  // Set the production url of your site here
  url: "https://0xspaceshard.github.io",
  // Set the /<baseUrl>/ pathname under which your site is served
  // For GitHub pages deployment, it is often '/<projectName>/'
  baseUrl: "/starknet-devnet-rs/",

  // GitHub pages deployment config.
  // If you aren't using GitHub pages, you don't need these.
  organizationName: "0xSpaceShard", // Usually your GitHub org/user name.
  projectName: "starknet-devnet-rs", // Usually your repo name.

  onBrokenLinks: "throw",
  onBrokenMarkdownLinks: "warn",

  // Even if you don't use internationalization, you can use this field to set
  // useful metadata like html lang. For example, if your site is Chinese, you
  // may want to replace "en" with "zh-Hans".
  i18n: {
    defaultLocale: "en",
    locales: ["en"],
  },

  presets: [
    [
      "classic",
      {
        docs: {
          sidebarPath: "./sidebars.ts",
          editUrl:
            "https://github.com/0xSpaceShard/starknet-devnet-rs/blob/master/website",
        },
        theme: {
          customCss: "./src/css/custom.css",
        },
      } satisfies Preset.Options,
    ],
  ],

  themeConfig: {
    // Replace with your project's social card
    image: "img/docusaurus-social-card.jpg",
    navbar: {
      title: "Starknet Devnet",
      logo: {
        alt: "Devnet Logo",
        src: "img/devnet-logo.png",
      },
      items: [
        {
          type: "docSidebar",
          sidebarId: "docSidebar",
          position: "left",
          label: "Docs",
        },
        {
          href: GITHUB_REPO_URL,
          label: "GitHub",
          position: "right",
        },
      ],
    },
    footer: {
      style: "dark",
      links: [
        {
          title: "Docs",
          items: [
            {
              label: "Get started",
              to: "/docs/intro",
            },
          ],
        },
        {
          title: "Community",
          items: [
            {
              label: "Discord",
              href: "https://discordapp.com/channels/793094838509764618/985824027950055434",
            },
            {
              label: "Starknet",
              href: "https://community.starknet.io/t/starknet-devnet/69",
            },
          ],
        },
        {
          title: "More",
          items: [
            {
              label: "GitHub",
              href: GITHUB_REPO_URL,
            },
          ],
        },
      ],
      copyright: `Copyright © ${new Date().getFullYear()} SpaceShard. Built with Docusaurus.`,
    },
    prism: {
      theme: prismThemes.github,
      darkTheme: prismThemes.dracula,
    },
  } satisfies Preset.ThemeConfig,
};

export default config;
