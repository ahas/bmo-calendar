import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { existsSync } from "node:fs";

import { chromium } from "playwright-core";

import en from "../../i18n/en.json";
import { WIDTHS, buildHarness, openScreen } from "./harness";
import { COMPANIES, DAY } from "./stand-in";

import type { Browser, Locator, Page } from "playwright-core";
import type { Built, Width } from "./harness";

// The calendar screen drawn by a real browser at a desktop's width and a phone's, read by the platform administrator
// with another company chosen (`docs/design.md`, 「The console」): its organization heading marked read-only by a mark that
// opens the choice of company, no calendar offered to add or set, nor a calendar or a source to hide, and an event's
// form holding every field and offering no save and no delete, standing inside the screen and closed with nothing
// asked. The mark's words read at 4.5:1 in both modes. The screen stands on the harness page, the stand-in answering
// the service. Skipped where the browser build is not on the machine.

function browserReady(): boolean {
  try {
    return existsSync(chromium.executablePath());
  } catch {
    return false;
  }
}

const ready = browserReady();

// How long the harness is given to build and the browser to start, and a test to run.
const HOOK_MS = 300_000;
const TEST_MS = 120_000;

const words = en.calendar;
// The console's words the layer's question and the form's save are drawn in (`screens-harness.ts`).
const QUESTION = "Discard unsaved changes?";
const SAVE = "Save";
const OTHER = COMPANIES.find((company) => !company.writes)!;

let built: Built;
let browser: Browser;

beforeAll(async () => {
  if (!ready) return;

  built = buildHarness();
  browser = await chromium.launch({ ignoreDefaultArgs: ["--hide-scrollbars"] });
}, HOOK_MS);

afterAll(async () => {
  await browser?.close();
  built?.remove();
}, HOOK_MS);

const button = (page: Page, name: string): Locator => page.getByRole("button", { name, exact: true });
const checkbox = (page: Page, name: string): Locator => page.getByRole("checkbox", { name, exact: true });

/** Opens the week the stand-in's events stand in, with the calendars in view and the other company chosen. */
async function openOther(width: Width) {
  const opened = await openScreen(browser, built, width, `/calendar/week/${DAY}`);
  const { page } = opened;

  await page.getByText("Dentist").first().waitFor();

  if (width === "phone") {
    await button(page, words.action.calendars).click();
    await page.getByRole("dialog").waitFor();
  }

  await page.getByLabel(words.label.company, { exact: true }).click();
  await page.getByRole("option", { name: OTHER.name, exact: true }).click();
  await button(page, words.label.read_only).waitFor();
  await page.getByText("Project schedules").first().waitFor();

  return opened;
}

// The words the mark shows read against what stands under it: its own ground laid over every ground beneath it, each
// colour read as the browser paints it, the page's own ground the mode's.
async function contrast(node: Locator): Promise<number> {
  return node.evaluate((element) => {
    const paint = document.createElement("canvas").getContext("2d", { willReadFrequently: true })!;
    const rgba = (color: string): [number, number, number, number] => {
      paint.clearRect(0, 0, 1, 1);
      paint.fillStyle = color;
      paint.fillRect(0, 0, 1, 1);
      const [r, g, b, a] = paint.getImageData(0, 0, 1, 1).data;

      return [r!, g!, b!, a! / 255];
    };
    const over = (top: [number, number, number, number], under: number[]) => top.slice(0, 3).map((channel, i) => channel * top[3] + under[i]! * (1 - top[3]));
    const grounds: string[] = [];

    for (let at: Element | null = element; at !== null; at = at.parentElement) grounds.push(getComputedStyle(at).backgroundColor);

    let under = document.documentElement.classList.contains("dark") ? [0, 0, 0] : [255, 255, 255];

    for (const ground of grounds.reverse()) under = over(rgba(ground), under);

    const ink = over(rgba(getComputedStyle(element).color), under);
    const light = (rgb: number[]) =>
      rgb.map((channel) => channel / 255).map((c) => (c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4)).reduce((sum, c, i) => sum + c * [0.2126, 0.7152, 0.0722][i]!, 0);
    const [high, low] = [light(ink), light(under)].sort((one, other) => other - one);

    return (high! + 0.05) / (low! + 0.05);
  });
}

describe.each(WIDTHS)("at a %s's width", (width) => {
  test.skipIf(!ready)(
    "another company's calendars stand read-only, the mark opening the choice of company",
    async () => {
      const { context, page } = await openOther(width);

      try {
        // Only the reader's own realm offers a calendar to add and its calendar to set.
        expect(await button(page, words.action.add_calendar).count()).toBe(1);
        expect(await button(page, words.aria.settings_named.replace("{name}", "Personal")).count()).toBe(1);
        expect(await button(page, words.aria.settings_named.replace("{name}", "Nuri company")).count()).toBe(0);

        // The other company's calendar and source are not hidden from here; the reader's own calendar is.
        expect(await checkbox(page, "Nuri company").isDisabled()).toBe(true);
        expect(await checkbox(page, "Project schedules").isDisabled()).toBe(true);
        expect(await checkbox(page, "Personal").isDisabled()).toBe(false);

        // The mark stands at the heading's trailing edge, on its centre line.
        const placed = await button(page, words.label.read_only).evaluate((mark) => {
          const row = mark.parentElement!.getBoundingClientRect();
          const heading = mark.parentElement!.querySelector("h2")!.getBoundingClientRect();
          const own = mark.getBoundingClientRect();

          return { trailing: row.right - own.right, centre: heading.top + heading.height / 2 - (own.top + own.height / 2) };
        });

        expect(Math.abs(placed.trailing)).toBeLessThan(0.5);
        expect(Math.abs(placed.centre)).toBeLessThan(0.5);

        // Pressed, it opens the choice of company, where the reader goes back to a company he writes in.
        await button(page, words.label.read_only).click();
        await page.getByRole("listbox").waitFor();
        expect(await page.getByRole("option").count()).toBe(COMPANIES.length);
      } finally {
        await context.close();
      }
    },
    TEST_MS
  );

  test.skipIf(!ready)(
    "the other company's event opens its form with every field held, no save and no delete, and closes asking nothing",
    async () => {
      const { context, page } = await openOther(width);

      try {
        if (width === "phone") {
          await page.goBack();
          await page.getByRole("dialog").waitFor({ state: "hidden" });
        }

        await page.locator("[data-event]", { hasText: "Nuri weekly meeting" }).first().click();

        const form = page.locator("form");

        await form.waitFor();
        expect(await form.locator("textarea").inputValue()).toBe("Third floor room");

        const held = await form.evaluate((node) =>
          [...node.querySelectorAll("input, textarea, button, [role=spinbutton]")]
            .filter((control) => !((control as HTMLInputElement).disabled || control.getAttribute("aria-disabled") === "true" || control.hasAttribute("data-disabled")))
            .map((control) => control.outerHTML.slice(0, 80))
        );

        expect(held).toEqual([]);

        // On a phone it stands as a dialog, whole inside the screen.
        if (width === "phone") {
          const placed = await page.getByRole("dialog").evaluate((dialog) => {
            const box = dialog.getBoundingClientRect();

            return { top: box.top, bottom: box.bottom, height: window.innerHeight };
          });

          expect(placed.top).toBeGreaterThanOrEqual(0);
          expect(placed.bottom).toBeLessThanOrEqual(placed.height);
        }
        expect(await form.getByRole("button", { name: SAVE, exact: true }).count()).toBe(0);
        expect(await form.getByRole("button", { name: words.action.delete, exact: true }).count()).toBe(0);

        await page.keyboard.press("Escape");
        await form.waitFor({ state: "hidden" });
        expect(await page.getByText(QUESTION).count()).toBe(0);
      } finally {
        await context.close();
      }
    },
    TEST_MS
  );

  test.each(["light", "dark"] as const)(
    "the read-only mark reads at 4.5:1 or better in the %s mode",
    async (scheme) => {
      if (!ready) return;

      const { context, page } = await openOther(width);

      try {
        // The screen's colours ease into the mode, so the mark is read once every colour transition has ended.
        await page.evaluate(async (dark) => {
          document.documentElement.classList.toggle("dark", dark);
          await Promise.allSettled(document.getAnimations().filter((motion) => motion instanceof CSSTransition).map((motion) => motion.finished));
        }, scheme === "dark");
        expect(await contrast(button(page, words.label.read_only).locator("span").first())).toBeGreaterThanOrEqual(4.5);
      } finally {
        await context.close();
      }
    },
    TEST_MS
  );
});
