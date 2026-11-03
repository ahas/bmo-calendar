import { describe, expect, test } from "bun:test";
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";

import { consoleManifest } from "bmo-ui/shared/console-manifest";

import { PACKS, keysOf, word } from "./words";

// The bundle as a console finds it: its declaration, the words every key the screens ask for, in both
// locales, under the bundle's namespace, and no Korean anywhere in the code.

const root = new URL("..", import.meta.url).pathname;

function files(folder: string): string[] {
  return readdirSync(folder).flatMap((name) => {
    const path = join(folder, name);

    return statSync(path).isDirectory() ? files(path) : [path];
  });
}

const sources = [...files(join(root, "app")), ...files(join(root, "shared")), ...files(join(root, "console"))].filter((path) => /\.(ts|vue)$/.test(path));

describe("the declaration", () => {
  const declared = JSON.parse(readFileSync(join(root, "console-bundle.json"), "utf8")) as Record<string, unknown>;

  test("names the bundle, its screen, its stream and its notice", () => {
    expect(declared).toMatchObject({ name: "calendar", title: "calendar.title", screenPath: "/calendar", serviceName: "calendar", events: true, pushKinds: ["alarm.told"] });
  });

  test("reads as a manifest by the contract, its title a word of the bundle", () => {
    const { languagePack: _pack, ...declaration } = declared;
    const manifest = consoleManifest({ ...declaration, bundle: "/console/bundle.js", words: "/console/words", role: "member", ownerKey: true });

    expect(manifest.name).toBe("calendar");
    expect(word(PACKS.ko, manifest.title)).toBeString();
  });
});

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

    for (const path of [...sources, ...files(join(root, "tests"))]) expect([path, korean.test(readFileSync(path, "utf8"))]).toEqual([path, false]);
  });
});
