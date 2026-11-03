import CalendarIndex from "../app/pages/calendar/index.vue";
import CalendarPage from "../app/pages/calendar/[view]/[date].vue";

import "./screens.css";

import type { ConsoleBundleModule } from "bmo-ui/shared/console-manifest";
import type { RouteLocationNormalizedLoaded } from "vue-router";

// The console bundle's module (bmo-ui's `docs/console-bundles.md`): the calendar screen under the screen path,
// its view and its day in the address, under the console's title bar.
const bundle: ConsoleBundleModule = {
  routes: [
    { path: "/calendar", name: "calendar", component: CalendarIndex, meta: { consoleTitleBar: true } },
    // One page for every day of a view, as its own page meta keys it in the Nuxt layer.
    {
      path: "/calendar/:view/:date",
      name: "calendar-view-date",
      component: CalendarPage,
      meta: { key: (route: RouteLocationNormalizedLoaded) => `calendar-${String(route.params.view)}`, consoleTitleBar: true }
    }
  ]
};

export default bundle;
