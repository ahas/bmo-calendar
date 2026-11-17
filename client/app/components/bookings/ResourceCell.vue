<template>
  <!-- A resource at the start of its row: its name, its seats and, where a booking of it waits for a keeper,
       the approval mark. It stays at the row's start while what follows scrolls under it. -->
  <div class="sticky start-0 z-5 flex w-32 shrink-0 flex-col justify-center border-e border-default bg-default px-3 py-1 sm:w-48">
    <span class="truncate text-sm font-medium text-highlighted">{{ props.resource.name }}</span>
    <span class="flex items-center gap-1 text-sm text-muted">
      <span class="tabular-nums">{{ $t("calendar.label.seats_count", { n: countText(props.resource.seats) }) }}</span>
      <HintTip v-if="props.resource.approval" :text="$t('calendar.label.approval_needed')" class="flex">
        <UIcon name="i-material-symbols-approval-outline" class="size-4" role="img" :aria-label="$t('calendar.label.approval_needed')" />
      </HintTip>
    </span>
  </div>
</template>

<script setup lang="ts">
import { countText } from "bmo-ui/app/domain/numbers";

import type { Resource } from "../../../shared/types";

const props = defineProps<{ resource: Resource }>();
</script>
