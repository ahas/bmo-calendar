<template>
  <!-- A received schedule's detail (docs/design.md, 「The console」), read alone where an event's form would stand: its
       title beside the row it opened from (a phone's dialog names it in its own head), when it happens, whose it is
       where the people directory names its assignee, and the source it came from. The way to the item it stands for
       is offered for the reader's own schedule alone. -->
  <div class="flex flex-col gap-3">
    <dl class="grid grid-cols-[auto_minmax(0,1fr)] gap-x-3 gap-y-2 text-sm">
      <template v-if="!narrow">
        <dt class="whitespace-nowrap text-muted">{{ $t("calendar.label.event_title") }}</dt>
        <dd class="min-w-0 break-words font-medium text-highlighted">{{ props.event.title || $t("calendar.label.untitled") }}</dd>
      </template>

      <dt class="whitespace-nowrap text-muted">{{ $t("calendar.label.when") }}</dt>
      <dd class="min-w-0 tabular-nums">
        {{ span }}<template v-if="props.event.allDay"> · {{ $t("calendar.label.all_day") }}</template>
      </dd>

      <template v-if="assignee">
        <dt class="whitespace-nowrap text-muted">{{ $t("calendar.label.assignee") }}</dt>
        <dd class="min-w-0 break-words">{{ assignee }}</dd>
      </template>

      <template v-if="source">
        <dt class="whitespace-nowrap text-muted">{{ $t("calendar.label.source") }}</dt>
        <dd class="flex min-w-0 items-center gap-2">
          <span class="size-2 shrink-0 rounded-full" :class="CALENDAR_DOT[source.color]" />
          <span class="min-w-0 break-words">{{ source.name }}</span>
        </dd>
      </template>
    </dl>

    <UButton v-if="link" v-bind="link" class="self-end" color="neutral" variant="subtle" :trailing-icon="link.external ? 'i-material-symbols-open-in-new' : undefined" :label="$t('calendar.action.open_item')" />
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { useNarrow } from "bmo-ui/app/domain/screen";

import { CALENDAR_DOT } from "../../domain/colors";
import { formatSpan } from "../../domain/dates";
import { originLink } from "../../domain/origin";
import { useCalendarEvents } from "../../composables/useCalendarEvents";
import { usePeople } from "../../composables/usePeople";

import type { CalendarEvent } from "../../../shared/types";

const props = defineProps<{ event: CalendarEvent }>();

const { layerOf } = useCalendarEvents();
const { knownName } = usePeople();
const narrow = useNarrow();

const span = computed(() => formatSpan(props.event));
// The assignee by the name the directory gives, else left out rather than shown as a bare key.
const assignee = computed(() => (props.event.origin?.assignee ? knownName(props.event.origin.assignee) : null));
const source = computed(() => layerOf(props.event));
const link = computed(() => originLink(props.event.origin));
</script>
