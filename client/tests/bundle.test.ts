import { describe, expect, test } from "bun:test";
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";

import { parse } from "vue/compiler-sfc";

import type { SFCDescriptor } from "vue/compiler-sfc";

import { PACKS, keysOf, word } from "./words";

// The bundle as a console finds it: the words every key the screens ask for, in both locales, under the
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
  test("both locales hold the same keys, all under the bundle's namespace", () => {
    expect(keysOf(PACKS.en).sort()).toEqual(keysOf(PACKS.ko).sort());
    expect(keysOf(PACKS.ko).every((key) => key.startsWith("calendar."))).toBe(true);
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

// The screen's template as the Vue compiler reads it: its elements by tag, an attribute by its name whether
// written plain or bound.
type TemplateNode = NonNullable<NonNullable<SFCDescriptor["template"]>["ast"]>["children"][number];
type Element = Extract<TemplateNode, { tag: string }>;

describe("the screen's bars", () => {
  const screen = parse(readFileSync(join(root, "app/pages/calendar/[view]/[date].vue"), "utf8")).descriptor.template!.ast!;

  function elements(node: { children?: unknown }): Element[] {
    const children = Array.isArray(node.children) ? (node.children as TemplateNode[]) : [];

    return children.flatMap((child) => ("tag" in child ? [child, ...elements(child)] : elements(child)));
  }

  function attribute(element: Element, name: string): string | undefined {
    for (const prop of element.props) {
      if (!("exp" in prop) && prop.name === name) return prop.value?.content;
      if ("exp" in prop && prop.name === "bind" && prop.arg && "content" in prop.arg && prop.arg.content === name) return prop.exp?.loc.source;
    }

    return undefined;
  }

  const toolbars = elements(screen).filter((element) => element.tag === "ToolBar");
  const head = toolbars.find((bar) => attribute(bar, "label")?.includes("calendar.aria.toolbar"))!;

  test("lays nothing into the console's title bar, and its own bar holds the title alone", () => {
    for (const path of sources) expect([path, /screenActions|screenLead|<Teleport/.test(readFileSync(path, "utf8"))]).toEqual([path, false]);

    const ownBars = elements(screen).filter((element) => element.tag === "UDashboardNavbar");

    expect(ownBars.map((bar) => bar.children.length)).toEqual([0]);
  });

  test("its toolbar holds the refresh and the offline mark at the right end", () => {
    const cells = head.children.filter((child): child is Element => "tag" in child);
    const last = elements(cells.at(-1)!);

    expect(last.some((element) => element.tag === "UButton" && attribute(element, "icon") === "i-material-symbols-refresh")).toBe(true);
    expect(elements(cells.at(-2)!).some((element) => element.tag === "UBadge" && attribute(element, "icon") === "i-material-symbols-cloud-off-outline")).toBe(true);
  });

  test("every button in either bar is the small button at its own size", () => {
    const buttons = toolbars.flatMap((bar) => elements(bar)).filter((element) => element.tag === "UButton");

    expect(buttons.length).toBeGreaterThan(5);

    for (const button of buttons) expect([button.loc.start.line, attribute(button, "size"), /\b(h-full|min-w-|p-0)\b/.test(attribute(button, "class") ?? "")]).toEqual([button.loc.start.line, "sm", false]);
  });
});
