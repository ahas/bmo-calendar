import { inject, reactive, ref } from "vue";

import { createAppComposable } from "./shared";

// The console's people directory (bmo-ui's `docs/console-bundles.md`, 「How a bundle mounts」, the people
// directory): the people of the reader's company, searched by their words, each drawn by the name the
// directory answers for the account key. A console that offers none leaves every account standing as its
// key, and keepers are then written as keys.

/** The key the console offers its people directory under. */
export const PEOPLE_KEY = "bmo-console-people";

export interface ConsolePerson {
  user: string;
  name: string;
  organization: string;
  jobTitle: string;
}

type Answer<T> = T | Promise<T>;

export interface ConsolePeople {
  search: (words: string) => Answer<ConsolePerson[]>;
  person: (user: string) => Answer<ConsolePerson | null>;
  me: () => Answer<ConsolePerson | null>;
}

const _usePeople = () => {
  const directory = inject<ConsolePeople | null>(PEOPLE_KEY, null);

  // The people already asked for, by account key; null where the directory names nobody for the key.
  const known = reactive(new Map<string, ConsolePerson | null>());
  const asking = new Set<string>();
  const me = ref<ConsolePerson | null>(null);
  const meRead = ref(false);

  async function ask(user: string): Promise<void> {
    if (!directory || known.has(user) || asking.has(user)) return;

    asking.add(user);

    try {
      known.set(user, await directory.person(user));
    } catch {
      known.set(user, null);
    } finally {
      asking.delete(user);
    }
  }

  /** The name the directory gives an account, asked for the first time it is wanted; null while it is not
   *  known — no directory, another company's account, or not answered yet. */
  function knownName(user: string): string | null {
    if (!known.has(user)) void ask(user);

    return known.get(user)?.name || null;
  }

  /** The name an account is drawn by: the directory's, else its key. An accessible name says nothing of an
   *  account the directory does not know (`knownName`), rather than a bare key. */
  function nameOf(user: string): string {
    return knownName(user) ?? user;
  }

  /** The people whose words hold the given ones; none without a directory. */
  async function search(words: string): Promise<ConsolePerson[]> {
    if (!directory || !words.trim()) return [];

    const found = await directory.search(words.trim());

    for (const person of found) known.set(person.user, person);

    return found;
  }

  /** The signed-in reader, read once; null where the directory names none or there is no directory. */
  async function loadMe(): Promise<void> {
    if (meRead.value || !directory) {
      meRead.value = true;
      return;
    }

    try {
      me.value = await directory.me();
    } catch {
      me.value = null;
    } finally {
      meRead.value = true;
    }
  }

  return { found: directory !== null, nameOf, knownName, search, me, meRead, loadMe };
};

export const usePeople = createAppComposable(_usePeople);
