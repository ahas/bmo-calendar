import { describe, expect, test } from "bun:test";
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";

import { attribute, component, elements, templateElements } from "./template";
import { PACKS, keysOf, word } from "./words";

import type { Element } from "./template";

// The bundle as a console finds it: the words every key the screens ask for, in every locale, under the
// bundle's namespace, and no Korean anywhere in the code. The manifest is the service's, read through bmo-ui's
// own reading by the service's tests.

const root = new URL("..", import.meta.url).pathname;

function files(folder: string): string[] {
  return readdirSync(folder).flatMap((name) => {
    const path = join(folder, name);

    return statSync(path).isDirectory() ? files(path) : [path];
  });
}

const sources = [...files(join(root, "app")), ...files(join(root, "shared")), ...files(join(root, "console"))].filter((path) => /\.(ts|vue)$/.test(path));

describe("the words", () => {
  test("every locale holds the same keys, all under the bundle's namespace", () => {
    const shipped = readdirSync(join(root, "i18n")).filter((name) => name.endsWith(".json")).map((name) => name.replace(/\.json$/, "")).sort();

    expect(shipped).toEqual(Object.keys(PACKS).sort());

    for (const [locale, pack] of Object.entries(PACKS)) expect([locale, keysOf(pack).sort()]).toEqual([locale, keysOf(PACKS.ko).sort()]);

    expect(keysOf(PACKS.ko).every((key) => key.startsWith("calendar."))).toBe(true);
  });

  test("every locale keeps the marks its words are filled at", () => {
    const marks = (text: string | undefined) => [...(text ?? "").matchAll(/\{(\w+)\}/g)].map((match) => match[1]).sort();

    for (const [locale, pack] of Object.entries(PACKS)) {
      for (const key of keysOf(PACKS.en)) expect([locale, key, marks(word(pack, key))]).toEqual([locale, key, marks(word(PACKS.en, key))]);
    }
  });

  test("every key the screens ask for is held", () => {
    const asked = new Set(sources.flatMap((path) => [...readFileSync(path, "utf8").matchAll(/["'`](calendar\.(?:label|action|aria|placeholder|status|message|error|notice)\.[a-z_.]+)["'`]/g)].map((match) => match[1]!)));

    expect(asked.size).toBeGreaterThan(40);

    for (const key of asked) expect([key, word(PACKS.ko, key)]).toEqual([key, expect.any(String)]);
  });

  test("the notice the console rings for an alarm has its words", () => {
    expect(word(PACKS.ko, "calendar.notice.alarm.told.title")).toBeString();
  });
});

describe("the code", () => {
  test("holds no Korean: the words stand in the pack and the reading words in its data file", () => {
    const korean = new RegExp(`[${String.fromCharCode(0x3131)}-${String.fromCharCode(0x318e)}${String.fromCharCode(0xac00)}-${String.fromCharCode(0xd7a3)}]`);

    for (const path of [...sources, ...files(join(root, "tests")), join(root, "nuxt.config.ts")]) expect([path, korean.test(readFileSync(path, "utf8"))]).toEqual([path, false]);
  });
});

describe("the screen's bars", () => {
  const screen = component(join(root, "app/pages/calendar/[view]/[date].vue")).template!.ast!;

  // The library's button, and bmo-ui's icon button drawing it as its icon alone.
  const BUTTONS = ["UButton", "IconButton"];
  const toolbars = elements(screen).filter((element) => element.tag === "ToolBar");
  const head = toolbars.find((bar) => attribute(bar, "label")?.includes("calendar.aria.toolbar"))!;

  test("lays nothing into the console's title bar, and its own bar holds the title alone", () => {
    for (const path of sources) expect([path, /screenActions|screenLead|<Teleport/.test(readFileSync(path, "utf8"))]).toEqual([path, false]);

    const ownBars = elements(screen).filter((element) => element.tag === "UDashboardNavbar");

    expect(ownBars.map((bar) => bar.children.length)).toEqual([0]);
  });

  test("its toolbar holds the offline mark, the refresh and, at its right end, the new event", () => {
    const cells = head.children.filter((child): child is Element => "tag" in child);

    expect(cells.at(-1)!.tag).toBe("CalendarNewEvent");
    expect(elements(cells.at(-2)!).some((element) => BUTTONS.includes(element.tag) && attribute(element, "icon") === "i-material-symbols-refresh")).toBe(true);
    expect(elements(cells.at(-3)!).some((element) => element.tag === "UBadge" && attribute(element, "icon") === "i-material-symbols-cloud-off-outline")).toBe(true);
  });

  test("a new event is made from the one create button alone, set once on the screen", () => {
    const newEvent = templateElements(join(root, "app/components/calendar/CalendarNewEvent.vue"));
    const all = elements(screen);

    expect(all.filter((element) => element.tag === "CalendarNewEvent").length).toBe(1);
    expect(newEvent.filter((element) => element.tag === "CreateButton").length).toBe(1);
    expect([...all, ...newEvent].some((element) => element.tag === "UButton" && attribute(element, "icon") === "i-material-symbols-add")).toBe(false);
  });

  test("every button in either bar is the small button at its own size", () => {
    const buttons = toolbars.flatMap((bar) => elements(bar)).filter((element) => BUTTONS.includes(element.tag));

    expect(buttons.length).toBeGreaterThan(5);

    for (const button of buttons) expect([button.loc.start.line, attribute(button, "size"), /\b(h-full|min-w-|p-0)\b/.test(attribute(button, "class") ?? "")]).toEqual([button.loc.start.line, "sm", false]);
  });

  test("names nothing with the library's own tooltip, which no finger opens, but with bmo-ui's shared one", () => {
    for (const path of sources) expect([path, /<UTooltip\b/.test(readFileSync(path, "utf8"))]).toEqual([path, false]);
  });
});
