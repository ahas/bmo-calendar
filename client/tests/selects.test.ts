import { describe, expect, test } from "bun:test";
import { readdirSync, statSync } from "node:fs";
import { join, relative } from "node:path";

import { attribute, component, listener, templateElements } from "./template";

// Every value the screens choose from a list stands on bmo-ui's one select, which alone decides when a list
// searches, and every form floating over the screen closes through its open state alone, its close guard the
// one refusal (`docs/design.md`, 「The console」).

const root = new URL("..", import.meta.url).pathname;

function files(folder: string): string[] {
  return readdirSync(folder).flatMap((name) => {
    const path = join(folder, name);

    return statSync(path).isDirectory() ? files(path) : [path];
  });
}

const screens = files(join(root, "app")).filter((path) => path.endsWith(".vue"));
const at = (path: string) => join(root, "app", path);

// The component library's own ways of picking a value, each a select of its own that searches by its own rule.
const LIBRARY_PICKERS = ["USelect", "USelectMenu", "UInputMenu", "UDropdownMenu"];

describe("a value chosen from a list", () => {
  test("never stands on the library's select, select menu, input menu or a dropdown menu", () => {
    for (const path of screens) {
      const bare = templateElements(path).filter((element) => LIBRARY_PICKERS.includes(element.tag));

      expect([relative(root, path), bare.map((element) => element.tag)]).toEqual([relative(root, path), []]);
    }
  });

  // Each list the screens offer, by the file it stands in and what its choices are bound to.
  const LISTS = [
    { path: "components/bookings/BookingForm.vue", items: "resourceItems" },
    { path: "components/bookings/BookingForm.vue", items: "repeatItems" },
    { path: "components/bookings/CompanySelect.vue", items: "items" },
    { path: "components/calendar/CalendarEventForm.vue", items: "calendarItems" },
    { path: "components/calendar/CalendarEventForm.vue", items: "alarmItems" },
    { path: "components/calendar/CalendarEventForm.vue", items: "repeatItems" },
    { path: "components/calendar/CalendarSettings.vue", items: "colorItems" },
    { path: "components/calendar/CalendarSettings.vue", items: "writersItems" },
    { path: "components/calendar/CalendarSettings.vue", items: "tellItems" },
    { path: "pages/calendar/bookings.vue", items: "viewItems" },
    { path: "pages/calendar/bookings.vue", items: "kindItems" },
    { path: "pages/calendar/[view]/[date].vue", items: "viewItems" }
  ];

  test.each(LISTS)("stands on the one select, choosing the value itself: $items in $path", ({ path, items }) => {
    const select = templateElements(at(path)).find((element) => attribute(element, "items") === items)!;

    expect([select.tag, attribute(select, "value-key")]).toEqual(["ChoiceSelect", "value"]);
  });

  test("the keepers are found through the people directory as their names are typed", () => {
    const picker = templateElements(at("components/bookings/KeeperPicker.vue")).find((element) => element.tag === "ChoiceSelect")!;

    expect(attribute(picker, "find")).toBe("find");
  });
});

describe("a form floating over the screen", () => {
  test("is never told to refuse a close on its own, nor answers the library's refused close", () => {
    for (const path of screens) {
      const refusing = templateElements(path).filter((element) => attribute(element, "dismissible") !== undefined || listener(element, "close:prevent") !== undefined);

      expect([relative(root, path), refusing.map((element) => element.loc.start.line)]).toEqual([relative(root, path), []]);
    }
  });

  // Each guarded float by the file it stands in and its tag, and the guard's leave its close request reaches.
  const FLOATS = [
    { path: "components/bookings/BookingDialog.vue", tag: "UModal" },
    { path: "components/bookings/ResourceDialog.vue", tag: "UModal" },
    { path: "components/calendar/CalendarSettingsPopover.vue", tag: "CalendarFormFloat" },
    { path: "components/calendar/CalendarEventDraft.vue", tag: "CalendarFormFloat" },
    { path: "components/calendar/CalendarEventPopover.vue", tag: "CalendarFormFloat" }
  ];

  test.each(FLOATS)("hands a request to close to its guard through its open state: $path", ({ path, tag }) => {
    const float = templateElements(at(path)).find((element) => element.tag === tag)!;
    const onOpen = listener(float, "update:open")!;
    // A listener written in place, or the function it names, read from the component's script.
    const runs = /^\w+$/.test(onOpen) ? component(at(path)).scriptSetup!.content : onOpen;

    expect(attribute(float, "open")).toBeString();
    expect(runs).toContain("void leave()");
  });

  test("its close in the corner reaches the guard through the modal's own close alone, so it asks once", () => {
    for (const path of ["components/bookings/BookingDialog.vue", "components/bookings/ResourceDialog.vue", "components/calendar/CalendarFormFloat.vue"]) {
      const close = templateElements(at(path)).find((element) => element.tag === "ShellClose")!;

      expect([path, listener(close, "close")]).toEqual([path, undefined]);
    }
  });
});
