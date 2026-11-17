// The screen check (README.md, 「Screen check」): the bundle's screens as a Nuxt app over the preview service,
// the client as a layer and its relay calls proxied to the service with the console's credential and one
// reader's key and role, as the console relays them.
const reader = { key: process.env.PREVIEW_KEY ?? "2", role: process.env.PREVIEW_ROLE ?? "member" };

export default defineNuxtConfig({
  extends: [".."],
  compatibilityDate: "2025-09-01",
  devtools: { enabled: false },
  ssr: false,
  appConfig: { bmoUi: { relayBase: "/relay" } },
  // The dev server's cache under the client's own packages, never at the repository's root.
  vite: { cacheDir: "../node_modules/.cache/vite-preview" },
  nitro: {
    devProxy: {
      "/relay/calendar": {
        target: process.env.PREVIEW_SERVICE ?? "http://127.0.0.1:1",
        changeOrigin: true,
        headers: { Authorization: "Bearer console-token", "Owner-Key": reader.key, "Owner-Role": reader.role }
      }
    }
  }
});
