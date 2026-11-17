import { ref } from "vue";
import { relayCall } from "bmo-ui/app/domain/relay";

import { createCalendarApi } from "../domain/api";
import { createAppComposable } from "./shared";

import type { RelayOptions } from "bmo-ui/app/domain/relay";
import type { Call } from "../domain/api";

// The calendar service as both of the bundle's screens call it, through the console's relay, and the
// company the administrator chose, which every call in the organization realm names (`docs/api.md`,
// 「Companies」). For every other reader the choice stays empty and no call names a company.

export const SERVICE = "calendar";

const relay: Call = <T>(path: string, options?: Parameters<Call>[1]) => relayCall<T>(SERVICE, path, options as RelayOptions);

const _useCalendarApi = () => {
  const company = ref<string | null>(null);
  const api = createCalendarApi(relay, () => company.value);

  return { api, company };
};

export const useCalendarApi = createAppComposable(_useCalendarApi);
