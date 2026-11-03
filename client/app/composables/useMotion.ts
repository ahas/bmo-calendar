import { usePreferredReducedMotion } from "@vueuse/core";
import { computed } from "vue";

import { scrollMotion } from "../domain/a11y";
import { createAppComposable } from "./shared";

// Whether the reader asked for less movement, read once for the app: a scroll the screen makes on its own
// glides only for a reader who did not.
const _useMotion = () => {
  const preference = usePreferredReducedMotion();
  const reduced = computed(() => preference.value === "reduce");
  const scrollBehavior = computed(() => scrollMotion(reduced.value));

  return { reduced, scrollBehavior };
};

export const useMotion = createAppComposable(_useMotion);
