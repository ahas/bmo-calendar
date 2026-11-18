import { fileURLToPath } from "node:url";

import ui from "@nuxt/ui/vite";
import vue from "@vitejs/plugin-vue";
import { defineConfig } from "vite";

// The calendar screen's harness page built into one script (`tests/browser/screens.test.ts`): Vue, the component
// library, the layer and the bundle's own pages under a router, so a page with no console around it draws them. The
// test hands it an output folder.

const here = (path: string): string => fileURLToPath(new URL(path, import.meta.url));
const layer = (path: string): string => here(`../../node_modules/bmo-ui/${path}`);

// The layer's app configuration is written for Nuxt, which supplies the helper it is wrapped in.
(globalThis as { defineAppConfig?: (config: unknown) => unknown }).defineAppConfig = (config) => config;

const { default: appConfig } = (await import("bmo-ui/app/app.config.ts")) as { default: { ui: Record<string, unknown> & { icons: Record<string, string> } } };

const { icons } = appConfig.ui;

export default defineConfig({
  root: here("../.."),
  // The script runs in the browser alone: what Nuxt states of the side a file runs on is stated here.
  define: { "import.meta.client": "true", "import.meta.server": "false", "import.meta.dev": "false", "process.env.NODE_ENV": JSON.stringify("production") },
  // The pages take the router and the page meta from Nuxt; here they are the harness's own.
  resolve: { alias: { "nuxt/app": here("./nuxt-app.ts") } },
  plugins: [
    vue(),
    ui({
      // Links move through the page's own router, as the console's do.
      router: true,
      colorMode: false,
      dts: false,
      ui: appConfig.ui,
      // The layer's files lean on Vue's names as Nuxt hands them over, so the package is read for them too; the
      // pages' meta is the bundle's own no-op, as its build hands it over.
      autoImport: {
        imports: ["vue", { [here("../../console/page-meta.ts")]: ["definePageMeta"] }],
        exclude: [/[\\/]node_modules[\\/](?!bmo-ui[\\/])/, /[\\/]\.git[\\/]/],
        dts: false
      },
      scanPackages: ["bmo-ui"],
      components: { dirs: [layer("app/components"), here("../../app/components")], dts: false },
      // The icons travel in the script, since the page fetches nothing but it.
      icon: {
        clientBundle: {
          icons: [icons.loading!, icons.close!, icons.check!],
          scan: { globInclude: ["app/**/*.{vue,ts}", "tests/browser/screens-harness.ts", "node_modules/bmo-ui/app/components/*.vue"] }
        }
      }
    })
  ],
  build: {
    emptyOutDir: true,
    lib: { entry: here("./screens-harness.ts"), formats: ["iife"], name: "ScreensHarness", fileName: () => "screens-harness.js" }
  }
});
