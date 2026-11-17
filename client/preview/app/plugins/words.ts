// The bundle's own word packs, installed as the console installs them when it mounts the bundle.
import { createI18n } from "vue-i18n";

import en from "../../../i18n/en.json";
import ko from "../../../i18n/ko.json";

export default defineNuxtPlugin((nuxtApp) => {
  nuxtApp.vueApp.use(createI18n({ legacy: false, globalInjection: true, locale: "ko", fallbackLocale: "en", messages: { ko, en } }));
});
