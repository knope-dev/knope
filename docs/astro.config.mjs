import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";

import expressiveCode from "astro-expressive-code";
import starlightLinksValidator from "starlight-links-validator";

/** @type {import('astro-expressive-code').AstroExpressiveCodeOptions} */
const expressiveCodeOptions = {
  theme: ["github-light", "github-dark"],
  frames: {
    removeCommentsWhenCopyingTerminalFrames: true,
  },
};

// https://astro.build/config
export default defineConfig({
  site: "https://knope.tech",
  integrations: [
    starlightLinksValidator(),
    starlight({
      title: "Knope",
      favicon: "/favicon.png",
      social: {
        github: "https://github.com/knope-dev/knope",
      },
      editLink: {
        baseUrl: "https://github.com/knope-dev/knope/edit/main/docs/",
      },
      customCss: ["./src/custom.css"],
      sidebar: [
        { label: "Installation", link: "/installation" },
        {
          label: "Tutorials",
          autogenerate: {
            directory: "tutorials",
          },
        },
        {
          label: "Recipes",
          autogenerate: {
            directory: "recipes",
          },
        },
        {
          label: "Reference",
          autogenerate: {
            directory: "reference",
          },
          collapsed: true,
        },
        {
          label: "FAQ",
          autogenerate: {
            directory: "faq",
          },
        },
      ],
    }),
    expressiveCode(expressiveCodeOptions),
  ],
});
