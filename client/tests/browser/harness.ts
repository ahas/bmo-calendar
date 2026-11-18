import { mkdtempSync, readFileSync, readdirSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { extname, join } from "node:path";

import { answerOf } from "./stand-in";

import type { Browser, BrowserContext, Page, Route } from "playwright-core";

// The harness page (`screens-harness.ts`) built and opened in a real browser: the build written into a scratch
// folder, and a page at a desktop's width or a phone's whose calls to the calendar service the stand-in answers as it
// answers the administrator. Nothing outside the page is reached; every address is answered here, the service's
// event stream by a stream that tells nothing.

const ROOT = new URL("../../", import.meta.url).pathname;
const ORIGIN = "http://page.test";
const RELAY = "/relay/calendar";
const PAGE = `<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<script src="/screens-harness.js" defer></script></head><body><div id="page"></div></body></html>`;

const TYPES: Record<string, string> = { ".js": "text/javascript", ".css": "text/css", ".woff2": "font/woff2" };

export type Width = "desktop" | "phone";

export const WIDTHS: Width[] = ["desktop", "phone"];

// A desktop's window, and a Galaxy S25 held upright, the phone the screens are designed against.
const CONTEXTS = {
  desktop: { viewport: { width: 1280, height: 800 } },
  phone: { viewport: { width: 412, height: 915 }, deviceScaleFactor: 2.625, isMobile: true, hasTouch: true }
};

/** The built page: the folder its files stand in, and the way to take it away. */
export interface Built {
  folder: string;
  remove(): void;
}

/** Builds the harness page into a scratch folder; a failed build throws with what the build said. */
export function buildHarness(): Built {
  const folder = mkdtempSync(join(tmpdir(), "calendar-screens-"));
  const built = Bun.spawnSync(
    ["bun", "x", "vite", "build", "--configLoader", "runner", "--config", "tests/browser/screens-harness.config.ts", "--outDir", folder, "--logLevel", "error"],
    { cwd: ROOT, env: { ...process.env, NODE_ENV: "production", VITE_CONFIG_NATIVE_IGNORE_WARNING: "true" } }
  );

  if (built.exitCode !== 0) {
    rmSync(folder, { recursive: true, force: true });
    throw new Error(`the harness build failed: ${built.stderr.toString()}`);
  }

  return { folder, remove: () => rmSync(folder, { recursive: true, force: true }) };
}

/** A page opened on the harness, and its context to close. */
export interface Opened {
  context: BrowserContext;
  page: Page;
}

/** Opens the harness at one width on the address given, the stand-in answering the service. */
export async function openScreen(browser: Browser, built: Built, width: Width, address: string): Promise<Opened> {
  const context = await browser.newContext(CONTEXTS[width]);
  const page = await context.newPage();
  const files = new Set(readdirSync(built.folder));

  await page.route(
    (url) => url.origin === ORIGIN,
    (route: Route) => {
      const url = new URL(route.request().url());
      const path = url.pathname;

      if (path === `${RELAY}/events`) return route.fulfill({ contentType: "text/event-stream", body: "retry: 86400000\n\n" });

      if (path.startsWith(`${RELAY}/`)) {
        const answer = answerOf(route.request().method(), path.slice(RELAY.length), url.searchParams);

        return route.fulfill({ status: answer.status, contentType: "application/json", body: JSON.stringify(answer.body) });
      }

      const file = path.slice(1);

      if (files.has(file)) return route.fulfill({ contentType: TYPES[extname(file)] ?? "application/octet-stream", body: readFileSync(join(built.folder, file)) });
      if (path.startsWith("/calendar")) return route.fulfill({ contentType: "text/html; charset=utf-8", body: PAGE });

      return route.fulfill({ status: 404, body: "" });
    }
  );

  await page.goto(`${ORIGIN}${address}`);
  await page.locator("[data-console-frame]").waitFor();

  return { context, page };
}
