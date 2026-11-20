import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { existsSync } from "node:fs";

import { chromium } from "playwright-core";

import en from "../../i18n/en.json";
import { WIDTHS, buildHarness, openScreen } from "./harness";
import { COMPANIES, DAY, OTHER_COMPANY, OWN_COMPANY } from "./stand-in";

import type { Browser, Locator, Page } from "playwright-core";
import type { Built, Width } from "./harness";

// The bundle's screens drawn by a real browser, read by the platform administrator (`docs/design.md`, 「The console」).
// Both screens stand beside the list of companies: at a desktop's width beside the body, searched as it is typed and
// chosen into the address, and at the narrowest phone's in a drawer opened from the button carrying the company chosen;
// the company the address names is kept as the screens move. At a desktop's width and a phone's, the calendar screen
// opened on another company: its organization heading marked read-only, no calendar offered to add or set, nor a
// calendar or a source to hide, and an event's form holding every field and offering no save and no delete, standing
// inside the screen and closed with nothing asked; the mark's words read at 4.5:1 in both modes. The screens stand on
// the harness page, the stand-in answering the service. Skipped where the browser build is not on the machine.

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
const OWN = COMPANIES.find((company) => company.id === OWN_COMPANY)!;
const OTHER = COMPANIES.find((company) => company.id === OTHER_COMPANY)!;
// The own company's row, named as the reader's own.
const OWN_LABEL = words.label.own_company.replace("{name}", OWN.name);

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
// The read-only mark's words, and the mark itself around them.
const readOnly = (page: Page): Locator => page.getByText(words.label.read_only, { exact: true });
const company = (page: Page): string | null => new URL(page.url()).searchParams.get("company");

/** Opens the week the stand-in's events stand in on the other company, as an address naming it opens it, with the
 *  calendars in view. */
async function openOther(width: Width) {
  const opened = await openScreen(browser, built, width, `/calendar/week/${DAY}?company=${OTHER.id}`);
  const { page } = opened;

  await page.getByText("Dentist").first().waitFor();

  if (width !== "desktop") {
    await button(page, words.action.calendars).click();
    await page.getByRole("dialog").waitFor();
  }

  await readOnly(page).waitFor();
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
    "another company's calendars stand read-only, marked so at the heading's trailing edge",
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
        const placed = await readOnly(page).evaluate((ink) => {
          const mark = ink.parentElement!;
          const row = mark.parentElement!.getBoundingClientRect();
          const heading = mark.parentElement!.querySelector("h2")!.getBoundingClientRect();
          const own = mark.getBoundingClientRect();

          return { trailing: row.right - own.right, centre: heading.top + heading.height / 2 - (own.top + own.height / 2) };
        });

        expect(Math.abs(placed.trailing)).toBeLessThan(0.5);
        expect(Math.abs(placed.centre)).toBeLessThan(0.5);
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
        if (width !== "desktop") {
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
        expect(await contrast(readOnly(page))).toBeGreaterThanOrEqual(4.5);
      } finally {
        await context.close();
      }
    },
    TEST_MS
  );
});

// Each screen the list of companies stands on, by the address it is opened on and what its body reads of the
// reader's own company and of the other: the calendar screen an organization event, the booking screen a resource.
const SCREENS = [
  { screen: "calendar", address: `/calendar/week/${DAY}`, own: "Hanbit review", other: "Nuri weekly meeting" },
  { screen: "booking", address: `/calendar/bookings?date=${DAY}`, own: "Hanbit board room", other: "Nuri hall" }
];

const row = (scope: Page | Locator, value: string): Locator => scope.locator(`[data-choice-sidebar-row="${value}"]`);

/** The values of the rows the list shows, in its order. */
async function rowValues(page: Page): Promise<string[]> {
  return page.locator("[data-choice-sidebar-row]").evaluateAll((rows) => rows.map((one) => one.getAttribute("data-choice-sidebar-row")!));
}

describe("the list of companies", () => {
  test.each(SCREENS)(
    "at 1280 stands beside the $screen screen's body, narrows as it is searched, and a choice reads the company and names it in the address",
    async ({ address, own, other }) => {
      if (!ready) return;

      const { context, page } = await openScreen(browser, built, "desktop", address);

      try {
        const list = page.locator("[data-choice-sidebar-list]");
        const body = page.locator("[data-choice-sidebar-body]");

        await body.getByText(own).first().waitFor();
        await list.waitFor();

        const [listBox, bodyBox] = [(await list.boundingBox())!, (await body.boundingBox())!];

        expect(listBox.x + listBox.width).toBeLessThanOrEqual(bodyBox.x + 1);
        expect(await page.locator("[data-choice-sidebar-open]").count()).toBe(0);
        expect(await rowValues(page)).toEqual(COMPANIES.map((one) => one.id));
        expect((await row(page, OWN.id).innerText()).trim()).toBe(OWN_LABEL);
        expect(await row(page, OWN.id).getAttribute("aria-current")).toBe("page");

        await page.locator("[data-choice-sidebar-search]").fill(OTHER.name.slice(0, 2));
        await row(page, OWN.id).waitFor({ state: "detached" });
        expect(await rowValues(page)).toEqual([OTHER.id]);

        await row(page, OTHER.id).click();
        await body.getByText(other).first().waitFor();
        expect(await body.getByText(own).count()).toBe(0);
        expect(company(page)).toBe(OTHER.id);
        expect(await row(page, OTHER.id).getAttribute("aria-current")).toBe("page");
      } finally {
        await context.close();
      }
    },
    TEST_MS
  );

  test.each(SCREENS)(
    "at 360 stands in a drawer the $screen screen's button opens, carrying the company chosen, and a choice closes it and reads the company",
    async ({ address, own, other }) => {
      if (!ready) return;

      const { context, page } = await openScreen(browser, built, "narrow", address);

      try {
        const body = page.locator("[data-choice-sidebar-body]");
        const opener = page.locator("[data-choice-sidebar-open]");

        await body.getByText(own).first().waitFor();
        await opener.waitFor();
        expect(await page.locator("[data-choice-sidebar-list]").count()).toBe(0);
        expect((await opener.innerText()).trim()).toBe(OWN_LABEL);

        await opener.click();

        const drawer = page.getByRole("dialog", { name: words.label.organizations });

        await drawer.locator("[data-choice-sidebar-list]").waitFor();
        await row(drawer, OTHER.id).click();
        await drawer.waitFor({ state: "hidden" });
        await body.getByText(other).first().waitFor();
        expect(await body.getByText(own).count()).toBe(0);
        expect((await opener.innerText()).trim()).toBe(OTHER.name);
        expect(company(page)).toBe(OTHER.id);
      } finally {
        await context.close();
      }
    },
    TEST_MS
  );

  test.skipIf(!ready)(
    "keeps the company the address names through the calendar's steps and views, onto the booking screen and back",
    async () => {
      const { context, page } = await openScreen(browser, built, "desktop", `/calendar/week/${DAY}?company=${OTHER.id}`);

      try {
        const body = page.locator("[data-choice-sidebar-body]");

        await body.getByText("Nuri weekly meeting").first().waitFor();
        expect(await row(page, OTHER.id).getAttribute("aria-current")).toBe("page");

        await page.getByRole("link", { name: words.action.next, exact: true }).click();
        await page.waitForURL((url) => url.pathname !== `/calendar/week/${DAY}`);
        expect(company(page)).toBe(OTHER.id);

        await page.keyboard.press("m");
        await page.waitForURL((url) => url.pathname.startsWith("/calendar/month/"));
        expect(company(page)).toBe(OTHER.id);

        await page.getByRole("link", { name: words.label.bookings, exact: true }).click();
        await body.getByText("Nuri hall").first().waitFor();
        expect([new URL(page.url()).pathname, company(page)]).toEqual(["/calendar/bookings", OTHER.id]);
        expect(await row(page, OTHER.id).getAttribute("aria-current")).toBe("page");

        await button(page, words.action.back).click();
        await page.waitForURL((url) => url.pathname.startsWith("/calendar/month/"));
        expect(company(page)).toBe(OTHER.id);
      } finally {
        await context.close();
      }
    },
    TEST_MS
  );
});
