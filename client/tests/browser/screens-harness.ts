import ui from "@nuxt/ui/vue-plugin";
import { $fetch } from "ofetch";
import { createApp, defineComponent, h, provide, ref } from "vue";
import { createI18n } from "vue-i18n";
import { RouterView, createRouter, createWebHistory } from "vue-router";

import UApp from "@nuxt/ui/components/App.vue";
import UDashboardGroup from "@nuxt/ui/components/DashboardGroup.vue";
import UDashboardNavbar from "@nuxt/ui/components/DashboardNavbar.vue";
import { markPainted } from "bmo-ui/app/domain/loading";
import { watchInputModality } from "bmo-ui/app/domain/modality";
import { watchOverlays } from "bmo-ui/app/domain/overlays";
import { TOOLTIP_PROVIDER } from "bmo-ui/app/domain/tooltip";

import CalendarBookings from "../../app/pages/calendar/bookings.vue";
import CalendarIndex from "../../app/pages/calendar/index.vue";
import CalendarPage from "../../app/pages/calendar/[view]/[date].vue";
import en from "../../i18n/en.json";
import { holdRouter } from "./nuxt-app";
import sheet from "./screens-harness.css?inline";

import type { Ref } from "vue";

// The calendar screen (`docs/design.md`, 「The console」) on a page standing in for the console: the console's frame
// for a screen a unit brings, its title bar over the screen holding the screen's name and mark, the bundle's routes
// under the router as its module names them (`console/routes.ts`), and the console's relay of the calendar service
// at the relay base, which the test answers. The page wires itself as the console does: the library's app root
// holding the layer's tooltip setting, the overlays the back gesture closes, the page taken over by the browser so a
// skeleton says it is busy, and the console's words for the layer's own keys. No people directory is offered, so the
// screens read none.

/** Where the page relays a call to a service, as the console's relay base. */
export const RELAY_BASE = "/relay";

// The console's own words the layer's components and the screens read: a skeleton's status, a close button, the
// question asked before changes are thrown away, a form's save, a date picker's paging, a select's search field
// and the lines its list shows when nothing stands in it, and a side list's search and refresh. The page is read in English, the language a test is
// written in.
const COMMON = {
  status: { loading: "Loading", no_match: "No match", no_choices: "No items" },
  title: { discard_changes: "Discard unsaved changes?" },
  action: {
    cancel: "Cancel",
    close: "Close",
    discard: "Discard",
    save: "Save",
    previous_month: "Previous month",
    next_month: "Next month",
    previous_year: "Previous year",
    next_year: "Next year",
    refresh: "Refresh"
  },
  aria: { search_named: "Search {name}" },
  placeholder: { search: "Search" }
};

const routes = [
  { path: "/calendar", name: "calendar", component: CalendarIndex, meta: { consoleTitleBar: true } },
  { path: "/calendar/bookings", name: "calendar-bookings", component: CalendarBookings, meta: { consoleTitleBar: true } },
  { path: "/calendar/:view/:date", name: "calendar-view-date", component: CalendarPage, meta: { consoleTitleBar: true } }
];

const router = createRouter({ history: createWebHistory(), routes });

holdRouter(router);

// What the layer's modules take from Nuxt by name: the app's fetcher and its settings, its shared state and its
// router.
const states = new Map<string, Ref<unknown>>();

Object.assign(globalThis, {
  $fetch,
  useRuntimeConfig: () => ({ app: { baseURL: "/" }, public: {} }),
  useRouter: () => router,
  useState: (key: string, start: () => unknown) => {
    if (!states.has(key)) states.set(key, ref(start()));

    return states.get(key);
  }
});

const i18n = createI18n({
  legacy: false,
  locale: "en",
  messages: { en: { ...en, common: COMMON } },
  missingWarn: false,
  fallbackWarn: false
});

const Page = defineComponent(() => {
  // The console offers its title bar to the screens under its layout, which then draw none of their own.
  provide("bmo-console-title-bar", {});

  return () =>
    h(UApp, { tooltip: TOOLTIP_PROVIDER }, () =>
      h(UDashboardGroup, { storage: "local", storageKey: "harness" }, () =>
        h("div", { class: "relative flex h-dvh min-h-0 min-w-0 flex-1 flex-col", "data-console-frame": "" }, [
          h(UDashboardNavbar, { title: i18n.global.t("calendar.title"), icon: "i-material-symbols-calendar-month-outline", toggle: false }),
          h(RouterView)
        ])
      )
    );
});

const style = document.createElement("style");

style.textContent = sheet;
document.head.append(style);

watchInputModality(document);
watchOverlays();
markPainted();

Object.assign(useAppConfig(), { bmoUi: { zone: "UTC", relayBase: RELAY_BASE } });

createApp(Page).use(i18n).use(router).use(ui).mount("#page");
