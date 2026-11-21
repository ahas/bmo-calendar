import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { existsSync } from "node:fs";

import { chromium } from "playwright-core";

import en from "../../i18n/en.json";
import { WIDTHS, buildHarness, openScreen } from "./harness";
import { BUSY_DAY, COMPANIES, DAY, OTHER_COMPANY, OWN_COMPANY } from "./stand-in";

import type { Browser, Locator, Page } from "playwright-core";
import type { Built, Width } from "./harness";

// The bundle's screens drawn by a real browser (`docs/design.md`, 「The console」). The month grid at a desktop's width
// with the console's navigation and right panel open, and at the narrowest phone's, standing inside the screen with no
// word of its own cut and every event's title showing at least its first characters. For the platform administrator the
// calendar screen holds the companies in its calendars' list, under the organization's heading: their skeleton until
// they are read, the grid standing where it stays as they arrive, searched as it is typed, and at the narrowest phone's
// in the calendars' sheet, which a choice closes. The booking screen stands beside the list of companies, its skeleton
// until the companies are read and the body standing where it stays as the list arrives: at a desktop's width beside
// the body, searched as it is typed, and at the narrowest phone's in a drawer opened from the button carrying the
// company chosen, which Esc and Back close alone. On either screen a choice is one step of the history and names the
// company in the address, which the screens keep as they move; at a desktop's width the line under the row heading
// each of its views, the weekdays, the days or the hours, runs on from the line under the list's head. A member is shown the companies' skeleton until his
// company is read, and then no company at either width. At a desktop's width and a phone's, the calendar screen opened
// on another company: its row marked read-only by a lock named in its tooltip, no calendar offered to add or set, nor a
// calendar or a source to hide, and an event's form holding every field and offering no save and no delete, standing
// inside the screen and closed with nothing asked; the lock reads at 3:1 in both modes. The screens stand on the harness
// page, the stand-in answering the service. Skipped where the browser build is not on the machine.

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
    "another company's calendars stand read-only, marked so at the trailing edge of its row",
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

        // The lock stands at the trailing edge of the company's row, on its centre line, and leaves its words whole.
        const placed = await readOnly(page).evaluate((mark) => {
          const row = mark.parentElement!.getBoundingClientRect();
          const heading = mark.parentElement!.firstElementChild!;
          const words = heading.querySelector("[data-layer-words]")!;
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

// A company's row in the calendar screen's list of calendars, by its value.
const companyRow = (scope: Page | Locator, value: string): Locator => scope.locator(`[data-company-row="${value}"]`);

// The month grid, and the narrowest an event's title is ever shown: about its first two characters.
const MONTH_GRID = "[role=grid]:has([data-date])";
const TITLE_SHOWN_PX = 24;

/** Every piece of words the grid shows, by how much of it stands and how wide it is whole, and whether it is an
 *  event's title, which gives way where the cell has no room for it. */
async function gridWords(page: Page): Promise<{ text: string; shown: number; whole: number; title: boolean }[]> {
  return page.locator(MONTH_GRID).evaluate((grid) =>
    [...grid.querySelectorAll<HTMLElement>("*")]
      .filter((node) => node.getClientRects().length > 0 && [...node.childNodes].some((child) => child.nodeType === Node.TEXT_NODE && child.textContent!.trim() !== ""))
      .map((node) => ({
        text: node.textContent!.trim(),
        shown: node.clientWidth,
        whole: node.scrollWidth,
        title: node.closest("[data-event]") !== null && !node.hasAttribute("data-time")
      }))
  );
}

describe("the month grid", () => {
  test.each(["console", "narrow"] as const)(
    "at the %s width stands inside the screen, cuts no word of its own and shows every event's title at least by its first characters",
    async (width) => {
      if (!ready) return;

      const { context, page } = await openScreen(browser, built, width, `/calendar/month/${BUSY_DAY}`);

      try {
        const grid = page.locator(MONTH_GRID);

        await grid.locator("[data-event]", { hasText: "Lunch" }).waitFor();
        // The calendars' list stands at its final width once the companies are read.
        if (width === "console") await companyRow(page, OWN.id).waitFor();

        const pieces = await gridWords(page);

        expect(pieces.filter((piece) => !piece.title && piece.shown < piece.whole)).toEqual([]);
        expect(pieces.filter((piece) => piece.title && piece.shown < Math.min(piece.whole, TITLE_SHOWN_PX))).toEqual([]);
        // The busy day counts the events it has no room for.
        expect(pieces.some((piece) => piece.text === words.label.more_short.replace("{n}", "4"))).toBe(true);

        const placed = await grid.evaluate((node) => {
          const box = node.getBoundingClientRect();

          return { left: box.left, right: box.right, screen: window.innerWidth, scroll: document.documentElement.scrollWidth };
        });

        expect(placed.left).toBeGreaterThanOrEqual(0);
        expect(placed.right).toBeLessThanOrEqual(placed.screen);
        expect(placed.scroll).toBeLessThanOrEqual(placed.screen);
      } finally {
        await context.close();
      }
    },
    TEST_MS
  );
});

// What the calendar screen reads of the reader's own company and of the other.
const OWN_EVENT = "Hanbit review";
const OTHER_EVENT = "Nuri weekly meeting";

/** The values of the company rows the calendars' list shows, in its order. */
async function companyValues(scope: Page | Locator): Promise<string[]> {
  return scope.locator("[data-company-row]").evaluateAll((rows) => rows.map((one) => one.getAttribute("data-company-row")!));
}

/** Where the words of an event the grid reads stand across the screen. */
async function eventPlace(page: Page, title: string): Promise<{ x: number; width: number }> {
  const held = (await page.locator("[data-calendar-body]").getByText(title).first().boundingBox())!;

  return { x: Math.round(held.x), width: Math.round(held.width) };
}

const historyLength = (page: Page): Promise<number> => page.evaluate(() => history.length);

describe("the companies in the calendars' list", () => {
  test.skipIf(!ready)(
    "at 1280 stand as their skeleton under the organization's heading and then as rows, the grid standing where it stays, narrow as they are searched, and a choice is one step of the history naming the company",
    async () => {
      const { promise: companiesHeld, resolve: answerCompanies } = Promise.withResolvers<void>();
      const { context, page } = await openScreen(browser, built, "desktop", `/calendar/week/${DAY}`, { companiesHeld });

      try {
        // Until the companies are read their rows stand as a skeleton, and the grid stands where it stays once they come.
        await page.locator("[data-calendar-body]").getByText(OWN_EVENT).first().waitFor();
        await page.locator("[data-company-skeleton]").waitFor();
        expect(await page.locator("[data-company-row]").count()).toBe(0);

        const unread = await eventPlace(page, OWN_EVENT);

        answerCompanies();
        await companyRow(page, OWN.id).waitFor();
        expect(await eventPlace(page, OWN_EVENT)).toEqual(unread);
        expect(await companyValues(page)).toEqual(COMPANIES.map((one) => one.id));
        expect((await companyRow(page, OWN.id).innerText()).trim()).toBe(OWN_LABEL);
        expect(await companyRow(page, OWN.id).getAttribute("aria-current")).toBe("page");
        expect(await checkbox(page, "Hanbit company").isVisible()).toBe(true);

        await page.locator("[data-company-search]").fill(OTHER.name.slice(0, 2));
        await companyRow(page, OWN.id).waitFor({ state: "detached" });
        expect(await companyValues(page)).toEqual([OTHER.id]);

        const steps = await historyLength(page);

        // The company chosen holds its calendars under its row.
        await companyRow(page, OTHER.id).click();
        await page.locator("[data-calendar-body]").getByText(OTHER_EVENT).first().waitFor();
        expect(await page.getByText(OWN_EVENT).count()).toBe(0);
        expect(company(page)).toBe(OTHER.id);
        expect(await historyLength(page)).toBe(steps + 1);
        expect(await companyRow(page, OTHER.id).getAttribute("aria-current")).toBe("page");
        await checkbox(page, "Nuri company").waitFor();

        // Back is the step before the choice: the company named before it, read again.
        await page.goBack();
        await page.locator("[data-calendar-body]").getByText(OWN_EVENT).first().waitFor();
        expect(company(page)).toBeNull();
        await page.locator("[data-company-search]").fill("");
        expect(await companyRow(page, OWN.id).getAttribute("aria-current")).toBe("page");
      } finally {
        await context.close();
      }
    },
    TEST_MS
  );

  test.skipIf(!ready)(
    "at 360 stand in the calendars' sheet, and a choice closes it in one step of the history naming the company",
    async () => {
      const { context, page } = await openScreen(browser, built, "narrow", `/calendar/week/${DAY}`);

      try {
        const sheet = page.getByRole("dialog", { name: words.action.calendars });

        await page.getByText(OWN_EVENT).first().waitFor();

        const steps = await historyLength(page);
        const at = page.url();

        await button(page, words.action.calendars).click();
        await companyRow(sheet, OTHER.id).click();
        await sheet.waitFor({ state: "hidden" });
        await page.getByText(OTHER_EVENT).first().waitFor();
        expect(company(page)).toBe(OTHER.id);
        expect(await historyLength(page)).toBe(steps + 1);

        // Back is the step before the choice.
        await page.goBack();
        await page.getByText(OWN_EVENT).first().waitFor();
        expect([page.url(), await sheet.count()]).toEqual([at, 0]);
      } finally {
        await context.close();
      }
    },
    TEST_MS
  );

  test.each(["desktop", "narrow"] as const)(
    "stand as their skeleton until a member's company is read, and then none stands at the %s width, the organization's heading offering its calendar to add",
    async (width) => {
      if (!ready) return;

      const { promise: companiesHeld, resolve: answerCompanies } = Promise.withResolvers<void>();
      const { context, page } = await openScreen(browser, built, width, `/calendar/week/${DAY}`, { reader: "member", companiesHeld });

      try {
        await page.getByText(OWN_EVENT).first().waitFor();

        if (width !== "desktop") {
          await button(page, words.action.calendars).click();
          await page.getByRole("dialog").waitFor();
        }

        await page.locator("[data-company-skeleton]").waitFor();

        // The member's one company answered, and the frames after it drawn.
        const answered = page.waitForResponse((response) => new URL(response.url()).pathname.endsWith("/organization/companies"));

        answerCompanies();
        await answered;
        await page.evaluate(() => new Promise((drawn) => requestAnimationFrame(() => requestAnimationFrame(drawn))));
        expect(await page.locator("[data-company-row], [data-company-search], [data-company-skeleton]").count()).toBe(0);
        expect(await button(page, words.action.add_calendar).count()).toBe(2);
        expect(await checkbox(page, "Hanbit company").isVisible()).toBe(true);
      } finally {
        await context.close();
      }
    },
    TEST_MS
  );
});

// The booking screen stands beside the list of companies: the address it is opened on and what its body reads of the
// reader's own company and of the other, a resource.
const SCREENS = [{ screen: "booking", address: `/calendar/bookings?date=${DAY}`, own: "Hanbit board room", other: "Nuri hall" }];

const row = (scope: Page | Locator, value: string): Locator => scope.locator(`[data-choice-sidebar-row="${value}"]`);
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

// The skeleton the list stands as until the companies are read: its rows' beside the body at a desktop's width, and the
// button's bar at the body's head at a phone's.
const LIST_SKELETON = "[data-choice-sidebar-list] [role=status][aria-busy=true]";
const OPENER_SKELETON = '[data-choice-sidebar-body] > div:first-child > [aria-hidden="true"][aria-busy="true"]';

/** Where the body stands across the screen, and how far down it the words it reads stand. */
async function bodyPlace(page: Page, words: string): Promise<{ x: number; width: number; top: number }> {
  const body = (await page.locator("[data-choice-sidebar-body]").boundingBox())!;
  const held = (await page.locator("[data-choice-sidebar-body]").getByText(words).first().boundingBox())!;

  return { x: Math.round(body.x), width: Math.round(body.width), top: Math.round(held.y) };
}

// Each view of the booking screen by its address, the row heading its body marked `data-body-head`.
const HEADED_VIEWS = [
  { view: "the bookings' month", address: `/calendar/bookings?view=month&date=${DAY}` },
  { view: "the bookings' week", address: `/calendar/bookings?view=week&date=${DAY}` },
  { view: "the bookings' day", address: `/calendar/bookings?view=day&date=${DAY}` }
];

/**
 * The pixel rows the line under the list's head and the line under the row heading the body take, each as its top
 * and bottom. The head's line is drawn over its last pixel; the heading row's is its own bottom border where it draws
 * one, and otherwise the top border of what follows it.
 */
async function headLines(page: Page): Promise<{ list: number[]; heading: number[] }> {
  return page.evaluate(() => {
    const head = document.querySelector("[data-choice-sidebar-list]")!.firstElementChild!.getBoundingClientRect();
    const heading = document.querySelector("[data-choice-sidebar-body] [data-body-head]")!;
    const box = heading.getBoundingClientRect();
    const own = getComputedStyle(heading).borderBottomWidth === "1px";

    return { list: [head.bottom - 1, head.bottom], heading: own ? [box.bottom - 1, box.bottom] : [box.bottom, box.bottom + 1] };
  });
}

describe("the booking screen's list of companies", () => {
  test.each(HEADED_VIEWS)(
    "at 1280 meets $view in one line: the line under the row heading it runs on from the line under the list's head",
    async ({ address }) => {
      if (!ready) return;

      const { context, page } = await openScreen(browser, built, "desktop", address);

      try {
        await row(page.locator("[data-choice-sidebar-list]"), OWN.id).waitFor();
        await page.locator("[data-choice-sidebar-body] [data-body-head]").waitFor();

        const lines = await headLines(page);

        expect(lines.heading).toEqual(lines.list);
      } finally {
        await context.close();
      }
    },
    TEST_MS
  );

  test.each(SCREENS)(
    "at 1280 stands as its skeleton and then its rows beside the $screen screen's standing body, narrows as it is searched, and a choice is one step of the history naming the company",
    async ({ address, own, other }) => {
      if (!ready) return;

      const { promise: companiesHeld, resolve: answerCompanies } = Promise.withResolvers<void>();
      const { context, page } = await openScreen(browser, built, "desktop", address, { companiesHeld });

      try {
        const list = page.locator("[data-choice-sidebar-list]");
        const body = page.locator("[data-choice-sidebar-body]");

        // Until the companies are read the list stands as its skeleton beside the body, and the body is the same body,
        // standing where it stays, once the list comes.
        await body.getByText(own).first().waitFor();
        await page.locator(LIST_SKELETON).waitFor();
        expect(await page.locator("[data-choice-sidebar-row]").count()).toBe(0);
        await body.getByText(own).first().evaluate((node) => Object.assign(window, { standing: node }));

        const unread = await bodyPlace(page, own);

        answerCompanies();
        await row(list, OWN.id).waitFor();
        expect(await page.evaluate(() => (window as unknown as { standing: Node }).standing.isConnected)).toBe(true);
        expect(await bodyPlace(page, own)).toEqual(unread);

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
    "at 360 stands as the skeleton of the $screen screen's button, then opens from the button carrying the company chosen, Esc and Back close it alone, and a choice closes it in one step of the history",
    async ({ address, own, other }) => {
      if (!ready) return;

      const { promise: companiesHeld, resolve: answerCompanies } = Promise.withResolvers<void>();
      const { context, page } = await openScreen(browser, built, "narrow", address, { companiesHeld });

      try {
        const body = page.locator("[data-choice-sidebar-body]");
        const opener = page.locator("[data-choice-sidebar-open]");
        const drawer = page.getByRole("dialog", { name: words.label.organizations });

        // Until the companies are read the button stands as its skeleton at the body's head, and the body is the same
        // body, standing where it stays, once the button comes.
        await body.getByText(own).first().waitFor();
        await page.locator(OPENER_SKELETON).waitFor();
        expect(await laidAside(page)).toEqual({ list: 0, opener: 0, handle: 0 });
        await body.getByText(own).first().evaluate((node) => Object.assign(window, { standing: node }));

        const unread = await bodyPlace(page, own);

        answerCompanies();
        await opener.waitFor();
        expect(await page.evaluate(() => (window as unknown as { standing: Node }).standing.isConnected)).toBe(true);
        expect(await bodyPlace(page, own)).toEqual(unread);
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
    "stands as its skeleton until a member's company is read, and is then laid aside at the %s width, the screen showing the body alone across it",
    async (width) => {
      if (!ready) return;

      for (const { address, own } of SCREENS) {
        const { promise: companiesHeld, resolve: answerCompanies } = Promise.withResolvers<void>();
        const { context, page } = await openScreen(browser, built, width, address, { reader: "member", companiesHeld });

        try {
          const body = page.locator("[data-choice-sidebar-body]");

          await body.getByText(own).first().waitFor();
          await page.locator(width === "desktop" ? LIST_SKELETON : OPENER_SKELETON).waitFor();

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

        await page.getByText(OTHER_EVENT).first().waitFor();
        expect(await companyRow(page, OTHER.id).getAttribute("aria-current")).toBe("page");

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
