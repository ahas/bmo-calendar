<template>
  <!-- The ghost of an event being made, dressed as the event it would be, its form open beside it, or on
       a phone as a centred dialog over it. Only the segment holding its start carries the form. -->
  <CalendarFormFloat anchor :open="shown" :keep-focus="redrawn" :title="$t('calendar.action.new_event')" :content="content" @update:open="(standing: boolean) => (standing ? (shown = true) : void leave())">
    <div
      ref="el"
      v-bind="$attrs"
      data-draft
      data-active
      aria-hidden="true"
      class="select-none text-sm text-highlighted transition-colors"
      :class="[
        variant === 'block'
          ? [EVENT_FILL[color], 'absolute z-20 flex flex-col items-start overflow-hidden rounded-sm py-1 ps-3 pe-2 text-start']
          : ['flex h-6 min-w-0 items-center gap-1 rounded-sm px-1.5', draft?.allDay ? EVENT_FILL[color] : ['bg-elevated', CHIP_COMPACT_FILL[color]]],
        continuesBefore && 'rounded-s-none',
        continuesAfter && 'rounded-e-none'
      ]"
      @pointerdown.stop
    >
      <span v-if="variant === 'block'" class="absolute inset-y-1 start-1 w-1 rounded-full" :class="CALENDAR_DOT[color]" />
      <span v-else-if="!draft?.allDay" class="size-2 shrink-0 rounded-full max-lg:hidden" :class="CALENDAR_DOT[color]" />

      <span class="truncate font-medium" :class="variant === 'block' && 'w-full'">{{ title }}</span>
      <span v-if="times" class="truncate tabular-nums" :class="variant === 'block' ? 'w-full text-toned' : 'ms-auto shrink-0 text-muted'">{{ times }}</span>
    </div>

    <template #content>
      <CalendarEventForm v-if="draft" v-model:dirty="formDirty" :draft="draft" :redrawn="redrawn" @update="updateDraft" @commit="commitDraft" @escape="leave" />
    </template>
  </CalendarFormFloat>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, ref, useTemplateRef, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useCloseGuard } from "bmo-ui/app/domain/form-save";

import CalendarEventForm from "./CalendarEventForm.vue";
import CalendarFormFloat from "./CalendarFormFloat.vue";
import { CALENDAR_DOT, CHIP_COMPACT_FILL, EVENT_FILL } from "../../domain/colors";
import { formatTime } from "../../domain/dates";
import { useCalendar } from "../../composables/useCalendar";
import { useCalendarEvents } from "../../composables/useCalendarEvents";
import { useEventDraft } from "../../composables/useEventDraft";
import { useMotion } from "../../composables/useMotion";

defineOptions({ inheritAttrs: false });

const props = defineProps<{ variant: "block" | "chip"; anchored?: boolean; continuesBefore?: boolean; continuesAfter?: boolean }>();

const { t } = useI18n();
const { formSide } = useCalendar();
const { draft, open, changed, pendingScroll, updateDraft, discardDraft, commitDraft, standForm, registerAnchor } = useEventDraft();
const { calendarByKey } = useCalendarEvents();
const { scrollBehavior } = useMotion();

registerAnchor();

// The form standing here reads its changes against the draft it was drawn with here; the draft reads
// them against what it was drawn as, wherever it has moved since, and the form alone holds what the
// draft cannot take, an end before its start among them.
const formDirty = ref(false);
const dirty = computed(() => formDirty.value || changed.value);
const color = computed(() => (draft.value ? calendarByKey(draft.value.calendar)?.color : undefined) ?? "primary");
const title = computed(() => draft.value?.title || t("calendar.label.untitled"));
const times = computed(() => {
  if (!draft.value || draft.value.allDay) return null;

  return props.variant === "block" ? `${formatTime(draft.value.start)}~${formatTime(draft.value.end)}` : formatTime(draft.value.start);
});

const content = computed(() => ({ side: formSide.value, sideOffset: 8, collisionPadding: 16 }));

// A draft is no event until it is saved: closing it throws it away, asking first where it holds a change,
// one question at a time.
const shown = computed({
  get: () => !!props.anchored && open.value,
  set(value: boolean) {
    if (value) return;

    formDirty.value = false;
    discardDraft(true);
  }
});
const { leave } = useCloseGuard(shown, () => dirty.value, t);

// The form standing here is the draft's first, or one drawn again where the draft moved to, which leaves
// the focus to the form rather than to its first field (CalendarEventForm). Read as it opens, before the
// form stands.
const redrawn = ref(false);

watch(
  shown,
  (value) => {
    if (value) redrawn.value = standForm();
  },
  { immediate: true, flush: "sync" }
);

const el = useTemplateRef("el");

// The new-event button draws on the day the screen stands on, which may be scrolled out of sight.
function reveal(): void {
  if (!props.anchored || !pendingScroll.value) return;

  pendingScroll.value = false;
  void nextTick(() => el.value?.scrollIntoView({ block: "nearest", behavior: scrollBehavior.value }));
}

onMounted(reveal);
watch(pendingScroll, reveal);
</script>
