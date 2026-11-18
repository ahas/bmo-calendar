import { useRoute, useRouter } from "vue-router";

import type { RouteLocationRaw, Router } from "vue-router";

// What the screens take from Nuxt's app module, as the harness page hands it over: the route and the router of the
// harness's own router, a move to another address through it, and the one app the screens' shared composables are
// kept on.

let standing: Router | null = null;

/** The one app the harness page runs: what a composable kept for the whole app is kept on. */
const app = {};

/** Hands the harness's router to the moves the screens make outside a component's setup. */
export function holdRouter(router: Router): void {
  standing = router;
}

export function navigateTo(to: RouteLocationRaw, options?: { replace?: boolean }): Promise<unknown> {
  if (standing === null) throw new Error("the harness holds no router");

  return options?.replace ? standing.replace(to) : standing.push(to);
}

export function useNuxtApp(): object {
  return app;
}

export { useRoute, useRouter };
