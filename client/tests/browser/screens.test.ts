import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { existsSync } from "node:fs";

import { chromium } from "playwright-core";

import en from "../../i18n/en.json";
import { WIDTHS, buildHarness, openScreen } from "./harness";
import { COMPANIES, DAY, OTHER_COMPANY, OWN_COMPANY } from "./stand-in";

import type { Browser, Locator, Page } from "playwright-core";
import type { Built, Width } from "./harness";

// The bundle's screens drawn by a real browser (`docs/design.md`, 「The console」). For the platform administrator both
// screens stand beside the list of companies, the body standing as the list arrives: at a desktop's width beside the
// body, searched as it is typed, and at the narrowest phone's in a drawer opened from the button carrying the company
// chosen, which Esc and Back close alone; a choice is one step of the history and names the company in the address,
// which the screens keep as they move. A member is shown the body alone at either width. At a desktop's width and a
// phone's, the calendar screen opened on another company: its organization heading marked read-only by a lock named in
// its tooltip, no calendar offered to add or set, nor a calendar or a source to hide, and an event's form holding every
// field and offering no save and no delete, standing inside the screen and closed with nothing asked; the lock reads at
// 3:1 in both modes. The screens stand on the harness page, the stand-in answering the service. Skipped where the
// browser build is not on the machine.

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
// The read-only lock, named by its words: the box its tooltip stands on, the icon inside it.
const readOnly = (page: Page): Locator => page.getByRole("img", { name: words.label.read_only, exact: true });
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

// The ink a mark is drawn in read against what stands under it: its own ground laid over every ground beneath it, each
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

        // The lock stands at the heading's trailing edge, on its centre line, and leaves the heading's words whole.
        const placed = await readOnly(page).evaluate((mark) => {
          const row = mark.parentElement!.getBoundingClientRect();
          const heading = mark.parentElement!.querySelector("h2")!;
          const words = heading.querySelector("span")!;
          const own = mark.getBoundingClientRect();
          const head = heading.getBoundingClientRect();

          return { trailing: row.right - own.right, centre: head.top + head.height / 2 - (own.top + own.height / 2), cut: words.scrollWidth - words.clientWidth };
        });

        expect(Math.abs(placed.trailing)).toBeLessThan(0.5);
        expect(Math.abs(placed.centre)).toBeLessThan(0.5);
        expect(placed.cut).toBeLessThanOrEqual(0);

        // Its words stand in its tooltip, a pointer bringing it up and a tap where a finger is all there is.
        const mark = readOnly(page);

        if (width === "desktop") await mark.hover();
        else await mark.tap();

        await page.locator('[role="tooltip"]', { hasText: words.label.read_only }).first().waitFor({ state: "attached" });
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
    "the read-only lock reads at 3:1 or better in the %s mode",
    async (scheme) => {
      if (!ready) return;

      const { context, page } = await openOther(width);

      try {
        // The screen's colours ease into the mode, so the mark is read once every colour transition has ended.
        await page.evaluate(async (dark) => {
          document.documentElement.classList.toggle("dark", dark);
          await Promise.allSettled(document.getAnimations().filter((motion) => motion instanceof CSSTransition).map((motion) => motion.finished));
        }, scheme === "dark");
        expect(await contrast(readOnly(page).locator("svg"))).toBeGreaterThanOrEqual(3);
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

const historyLength = (page: Page): Promise<number> => page.evaluate(() => history.length);

/** What stands of the list where it is laid aside: nothing beside the body, no button and no splitter's line. */
async function laidAside(page: Page): Promise<{ list: number; opener: number; handle: number }> {
  return {
    list: await page.locator("[data-choice-sidebar-list]").count(),
    opener: await page.locator("[data-choice-sidebar-open]").count(),
    handle: await page.locator("[data-choice-sidebar] > [data-slot=handle]").count()
  };
}

/** The values of the rows the list shows, in its order. */
async function rowValues(page: Page): Promise<string[]> {
  return page.locator("[data-choice-sidebar-row]").evaluateAll((rows) => rows.map((one) => one.getAttribute("data-choice-sidebar-row")!));
}

describe("the list of companies", () => {
  test.each(SCREENS)(
    "at 1280 comes beside the $screen screen's standing body, narrows as it is searched, and a choice is one step of the history naming the company",
    async ({ address, own, other }) => {
      if (!ready) return;

      const { promise: companiesHeld, resolve: answerCompanies } = Promise.withResolvers<void>();
      const { context, page } = await openScreen(browser, built, "desktop", address, { companiesHeld });

      try {
        const list = page.locator("[data-choice-sidebar-list]");
        const body = page.locator("[data-choice-sidebar-body]");

        // Until the companies are read the body stands alone, and it is the same body once the list comes.
        await body.getByText(own).first().waitFor();
        expect(await laidAside(page)).toEqual({ list: 0, opener: 0, handle: 0 });
        await body.getByText(own).first().evaluate((node) => Object.assign(window, { standing: node }));
        answerCompanies();
        await list.waitFor();
        expect(await page.evaluate(() => (window as unknown as { standing: Node }).standing.isConnected)).toBe(true);

        const [listBox, bodyBox] = [(await list.boundingBox())!, (await body.boundingBox())!];

        expect(listBox.x + listBox.width).toBeLessThanOrEqual(bodyBox.x + 1);
        expect(await page.locator("[data-choice-sidebar-open]").count()).toBe(0);
        expect(await rowValues(page)).toEqual(COMPANIES.map((one) => one.id));
        expect((await row(page, OWN.id).innerText()).trim()).toBe(OWN_LABEL);
        expect(await row(page, OWN.id).getAttribute("aria-current")).toBe("page");

        await page.locator("[data-choice-sidebar-search]").fill(OTHER.name.slice(0, 2));
        await row(page, OWN.id).waitFor({ state: "detached" });
        expect(await rowValues(page)).toEqual([OTHER.id]);

        const steps = await historyLength(page);

        await row(page, OTHER.id).click();
        await body.getByText(other).first().waitFor();
        expect(await body.getByText(own).count()).toBe(0);
        expect(company(page)).toBe(OTHER.id);
        expect(await historyLength(page)).toBe(steps + 1);
        expect(await row(page, OTHER.id).getAttribute("aria-current")).toBe("page");

        // Back is the step before the choice: the company named before it, read again.
        await page.goBack();
        await body.getByText(own).first().waitFor();
        expect(company(page)).toBeNull();
        await page.locator("[data-choice-sidebar-search]").fill("");
        expect(await row(page, OWN.id).getAttribute("aria-current")).toBe("page");
      } finally {
        await context.close();
      }
    },
    TEST_MS
  );

  test.each(SCREENS)(
    "at 360 opens on the $screen screen from the button carrying the company chosen, Esc and Back close it alone, and a choice closes it in one step of the history",
    async ({ address, own, other }) => {
      if (!ready) return;

      const { promise: companiesHeld, resolve: answerCompanies } = Promise.withResolvers<void>();
      const { context, page } = await openScreen(browser, built, "narrow", address, { companiesHeld });

      try {
        const body = page.locator("[data-choice-sidebar-body]");
        const opener = page.locator("[data-choice-sidebar-open]");
        const drawer = page.getByRole("dialog", { name: words.label.organizations });

        await body.getByText(own).first().waitFor();
        expect(await laidAside(page)).toEqual({ list: 0, opener: 0, handle: 0 });
        await body.getByText(own).first().evaluate((node) => Object.assign(window, { standing: node }));
        answerCompanies();
        await opener.waitFor();
        expect(await page.evaluate(() => (window as unknown as { standing: Node }).standing.isConnected)).toBe(true);
        expect(await page.locator("[data-choice-sidebar-list]").count()).toBe(0);
        expect((await opener.innerText()).trim()).toBe(OWN_LABEL);

        const steps = await historyLength(page);
        const at = page.url();

        // Esc, and then Back, put the drawer away and leave the page and its company as they stood.
        for (const close of [() => page.keyboard.press("Escape"), () => page.goBack()]) {
          await opener.click();
          await drawer.locator("[data-choice-sidebar-list]").waitFor();
          await close();
          await drawer.waitFor({ state: "hidden" });
          expect([page.url(), (await opener.innerText()).trim()]).toEqual([at, OWN_LABEL]);
        }

        await opener.click();
        await row(drawer, OTHER.id).click();
        await drawer.waitFor({ state: "hidden" });
        await body.getByText(other).first().waitFor();
        expect(await body.getByText(own).count()).toBe(0);
        expect((await opener.innerText()).trim()).toBe(OTHER.name);
        expect(company(page)).toBe(OTHER.id);
        expect(await historyLength(page)).toBe(steps + 1);

        // Back is the step before the choice.
        await page.goBack();
        await body.getByText(own).first().waitFor();
        expect([page.url(), (await opener.innerText()).trim(), await drawer.count()]).toEqual([at, OWN_LABEL, 0]);
      } finally {
        await context.close();
      }
    },
    TEST_MS
  );

  test.each(["desktop", "narrow"] as const)(
    "is laid aside for a member at the %s width, both screens showing the body alone across the screen",
    async (width) => {
      if (!ready) return;

      for (const { address, own } of SCREENS) {
        const { promise: companiesHeld, resolve: answerCompanies } = Promise.withResolvers<void>();
        const { context, page } = await openScreen(browser, built, width, address, { reader: "member", companiesHeld });

        try {
          const body = page.locator("[data-choice-sidebar-body]");

          await body.getByText(own).first().waitFor();

          // The member's one company answered, and the frames after it drawn.
          const answered = page.waitForResponse((response) => new URL(response.url()).pathname.endsWith("/organization/companies"));

          answerCompanies();
          await answered;
          await page.evaluate(() => new Promise((drawn) => requestAnimationFrame(() => requestAnimationFrame(drawn))));
          expect(await laidAside(page)).toEqual({ list: 0, opener: 0, handle: 0 });
          expect((await body.boundingBox())!.width).toBe(page.viewportSize()!.width);
        } finally {
          await context.close();
        }
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
