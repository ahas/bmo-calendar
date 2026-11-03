import { nextTick, onMounted, onUnmounted, ref, watch } from "vue";

import { createAppComposable } from "./shared";

// Which event has its form open: one key for the whole app rather than a flag in every chip, since a chip
// is made again whenever its event moves to another day, which is what editing its dates does.
const _useEventEditor = () => {
  const editingKey = ref<string | null>(null);

  // The forms on screen, counted by the form itself while it stands.
  const anchors = ref(0);

  function openEvent(key: string): void {
    editingKey.value = key;
  }

  function closeEvent(key: string): void {
    if (editingKey.value === key) editingKey.value = null;
  }

  function registerAnchor(): void {
    onMounted(() => anchors.value++);
    onUnmounted(() => anchors.value--);
  }

  // A form with nowhere left to stand — the view changed, the month row it was in scrolled away — lets
  // its key go, once its replacement has had a tick to turn up.
  watch(anchors, (count) => {
    if (count || !editingKey.value) return;

    void nextTick(() => {
      if (!anchors.value) editingKey.value = null;
    });
  });

  return { editingKey, openEvent, closeEvent, registerAnchor };
};

export const useEventEditor = createAppComposable(_useEventEditor);
