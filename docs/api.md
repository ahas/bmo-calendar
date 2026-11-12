# The boundary API

Every route bmo-calendar answers, the events it publishes, the tool entries it places, and the calls it
makes. The wire format is the platform's (bmo-chat `docs/conventions.md`): camelCase keys, times in epoch
milliseconds unless a field says it is a local ISO datetime, ids as strings the brain's keys are made of,
a failure a non-2xx status with `code` (kebab-case) and `error` (an English sentence), a list `{items,
more}` read with `after` and `limit`. `docs/openapi.json` is generated from the service's own types by
`bmo-calendar openapi`, and a test fails where the committed document differs.

## The control port

The service listens on the loopback at the address `env/calendar.toml` names (`[server] listen`, by
convention `127.0.0.1:13127`), under `Authorization: Bearer <token>` with the service token
(`[server] token`). The console reaches it with the console credential (`[console] token`) and the
signed-in reader's owner key and role, carried as bmo-ui's console plugin contract states (「Console」). A
call with no credential or a wrong one is 401 `unauthorized`; a console call the contract's guard refuses
for its owner key or its role, or whose `Owner-Key` is longer than 200 characters, is 403 `forbidden`. A
service-token call acts as the administrator.

## Realms

A realm is `mine` or `organization`; any other word in its place is 404 `not-found`. A route that takes a
realm reads it from the path. In the `mine` realm the account is the caller's `Owner-Key`; a service-token
call names it with `account=<key>`, or with `holderName=<name>`, the holder of a tool session, which acts for
the owner: the account the account service links to the owner's sender (the receiver's `GET /owner`). A
service-token call in the `mine` realm that names neither, or whose holder's account cannot be found, is 422
`no-account`. In the `organization` realm the same query names the account a write records as its creator
and whose `hidden` mark a read answers.

## Health

| Method | Path | Does |
|---|---|---|
| GET | `/health` | `ready`, `reason` (null when ready), `version` |

## Calendars

| Method | Path | Does |
|---|---|---|
| GET | `/{realm}/calendars` | The realm's calendars, the caller's `hidden` mark on each, oldest first — `{items, more}` |
| POST | `/{realm}/calendars` | `name`, `color`; organization: `writers`, `tell` (defaults `creator`) — 201 |
| PATCH | `/{realm}/calendars/{calendarId}` | `name`, `color`, `writers`, `tell`, `hidden` — 200 |
| DELETE | `/{realm}/calendars/{calendarId}` | Retires it (its memory `done`); its events are retired with it — 204 |

A calendar is `id`, `realm`, `name`, `color`, `writers`, `tell`, `hidden`, `creator`, `createdAt`;
`writers` and `tell` are null in the `mine` realm. A realm whose brain holds no live calendar answers one
made on the read: `mine` → 「내 일정」 `primary`, id `mine`; `organization` → 「조직 일정」 `info`, id
`organization` (the names are settings, 「Settings」, which the deployment sets to these words; the
repository's defaults are English). `hidden` is the caller's own: a PATCH of it hides or
shows the calendar for the caller's account alone and is open to every account that reads the realm. In
the `organization` realm, making a calendar is the administrator's, and changing its `name`, `color`,
`writers` or `tell` or retiring it the administrator's and its creator's; anyone else is 403
`not-a-writer`. A `color`, `writers` or `tell` outside its words, or a blank name, is 400 `bad-request`; a
calendar the realm does not hold live is 404 `not-found`.

## Events

| Method | Path | Does |
|---|---|---|
| GET | `/{realm}/events?start=&end=` | The events overlapping the span (local ISO datetimes, at most 92 days), by start — `{items, more}`; a repeating event answers one row per occurrence in the span, `occurrence` naming its start |
| GET | `/{realm}/events/{eventId}` | One event |
| POST | `/{realm}/events` | 201 |
| PATCH | `/{realm}/events/{eventId}` | Changes the fields named; `occurrence` limits a repeating event's change to that one (which becomes an event of its own) — 200 |
| DELETE | `/{realm}/events/{eventId}` | Cancels it (`done`); `occurrence` cancels one occurrence — 204 |

An event is `id`, `realm`, `calendarId`, `title`, `description` (may be empty), `start`, `end` (local
ISO datetimes, `YYYY-MM-DDTHH:mm:ss`, `end` after `start`), `allDay`, `alarm` (`none` or minutes before
start: `0`, `5`, `15`, `30`, `60`, `1440`, each a string), `repeat` (`none`, `daily`, `weekly`, `monthly`,
`yearly`), `occurrence` (in a span's rows, the start of the occurrence a repeating event's row stands for,
whose `start` and `end` the row carries; null otherwise), `creator`, `createdAt`, `changedAt`, `told` (the
occurrence last told, or null). A write names `calendarId`, `title`, `start`, `end` and may name the rest
and an `id` (1 to 64 letters, digits, `-` and `_`; one is made where none is given); a title is one line.
A write under a calendar the caller may not write is 403 `not-a-writer`; an end not after the start is 422
`end-before-start`; an unknown calendar is 422 `unknown-calendar`; a span over 92 days or ending before it
starts is 422 `span-too-long`; an `occurrence` that is not one of the event's is 422 `not-an-occurrence`;
an event the realm does not hold live is 404 `not-found`; a field outside its shape or its words is 400
`bad-request`. In the `organization` realm a calendar whose `writers` is `administrator` is written by the
administrator alone, one whose `writers` is `creator` takes new events from every account and a change or
a cancellation from the administrator and the event's creator, and one whose `writers` is `everyone` from
every account.

Every write is idempotent on `id`: a POST that names an `id` the realm already holds answers 200 with the
event as it stands after the write (the console's offline replay sends the same write twice).

## Alarms

| Method | Path | Does |
|---|---|---|
| GET | `/{realm}/alarms?from=&to=` | The alarm moments in the span (epoch ms, at most 92 days), by moment — `{items, more}`, each `realm`, `eventId`, `occurrence`, `at`, `told` |
| POST | `/beats` | Service token only (403 `forbidden` to the console): runs one beat now (the minute's work: tell what is due) — 200, `told`, each alarm told: `realm`, `eventId`, `occurrence`, `at`, `lateMs`, `rooms` |

The beat runs every minute on the service's own clock and once at start. It tells each due alarm once:
the mark `told:<eventId>:<occurrence>` is kept per realm in the unit's own store, and the event's `told` is
read from it. An alarm is due when its moment has come and it came after the last beat the store
remembers, or when its event was written after its moment and has not started yet. A due alarm found at
start after the service was down is told with how late it is (`lateMs`, 0 where it is on time).

How an alarm is told (「Calls the calendar makes」): one `POST /instructions` on the receiver for each room
it reaches, sort `calendar`, kind `calendar:<eventId>:<occurrence>`, `cause` `reply`, `summary` the event's
title, `said.order` opening on the marker `CALENDAR ALARM.` and then the data alone, in English: the event's
title, its start, its calendar and realm, how late it is where it is late, that a reminder of it was asked
for, and whom it is for. The turn carries no instruction; how a quiet instruction is answered stands in the
receiving session's own rules. Who is told: in the `mine` realm the account; in the `organization` realm the
calendar's `tell`: `creator` the event's creator, `everyone` the owner and every account holding a `mine`
realm, `nobody` none. Where each is told: the owner's private room
(the receiver's `GET /owner`) for the owner's account (the account the account service links to the owner's
sender); for a member, the room numbered as one of the senders the account service links to the member's
account (`GET /users/{user}`) that the receiver holds registered (`GET /rooms/{roomId}`), else the owner's
private room naming the member. One room is told once, naming every account it is told for. A telling the
receiver refuses or does not answer keeps no mark, and the next beat tells it again.

## Console

The calendar's screen is a console bundle under bmo-ui's console plugin contract (bmo-ui
`docs/console-bundles.md`), which states the manifest, the bundle's files, the layer a bundle is built on
and the guard of every route a console reaches. This section names only what the calendar chooses within
it.

- **The bundle.** Named `calendar`, its title the word `calendar.title` and a month icon, its screens under
  `/calendar`, relayed as the service `calendar` with the whole API under the prefix `/`.
- **Its readers.** Every member and administrator, each with an owner key: the `mine` realm is the reader's
  own account (「Realms」).
- **What it reads.** The event stream (「Events the calendar publishes」), whose `alarm.told` a console raises
  as a notice. No WebSocket, no share page, no sidebar count, no other service called and no route relayed
  outside the prefix.
- **Its files.** The words are answered at `/console/words`, the module and the stylesheet under `/console/`
  at names made from their bytes. They are read once at start from the folder `bun run bundle` builds in
  `client/` with bmo-ui's bundle build (`[console] folder`), and the manifest names the layer that build
  wrote there. A folder missing any of the module, the stylesheet, the words or the layer leaves the bundle
  unserved, its routes answered 404 `not-found`.
- **Its guard.** The four routes take the console credential alone; the service token is refused there with
  a 401, as any credential that is not the console's is.

## Events the calendar publishes

`GET /events` is a server-sent stream in the platform's shape (`after` or `Last-Event-ID`, `kinds`; the
same events as a page to `Accept: application/json`, with `after` and `limit`): `calendar.changed`
(`realm`, `calendarId`), `event.changed` (`realm`, `eventId`, `occurrence`, null where the change is the
whole event's), `alarm.told` (`realm`, `eventId`, `occurrence`, `at`). A console reader is given the
`organization` realm's events and its own account's `mine` events; the service token reads every one. The
service keeps the events since it started, at most the last 1000, numbered upward from its start's moment
in milliseconds times 1000; a resume from before them is 410 `gone`. The console refreshes the visible span
on `event.changed` and rings `alarm.told`.

## The tool

Placed on the tool host by the deployment (`bmo-pocket put --service calendar --placed --allow
room.tools tools/calendar/.placed`), namespace `bmo`, capability `room.tools`, each entry's input and
output schema in `docs/openapi.json` under `x-tools`:

| Entry | Does |
|---|---|
| `calendar_list` | The calendars of a realm |
| `calendar_events` | The events of a realm in a span (`realm`, `start`, `end`), or the next `n` from now |
| `calendar_show` | One event |
| `calendar_add` | Adds an event: `realm`, `calendarId?` (the realm's first where none), `title`, `start`, `end?` (an hour where none, the day where `allDay`), `description?`, `allDay?`, `alarm?`, `repeat?` |
| `calendar_change` | Changes the fields named |
| `calendar_cancel` | Cancels an event or one occurrence |

The `mine` realm of a tool call is the account the turn's holder acts for: the tool hands the session's
`holderName` to the service, which answers it with the owner's account (「Realms」); a call that cannot
name an account is refused `no-account`. `calendar_events` with `n` and no span reads from now to 92 days
ahead and answers the first `n`. The tool reaches the service with the service token from
`env/calendar.env` under the data root (`CALENDAR_ADDRESS`, `CALENDAR_TOKEN`; the same names in the call's
environment come first), which the deployment writes before the flip with `bmo-calendar tool-env`.

## Calls the calendar makes

| What | Route |
|---|---|
| the realm's brain, made on first write | brain `GET /brains?accountKey=&agentKey=`, `POST /brains` |
| calendars and events read, written, versioned, retired | brain `GET /memories`, `POST /memories`, `POST /memories/{id}/versions`, `PATCH /memories/{id}` |
| the brain's changes followed | brain `GET /events` (`memory.written`, `memory.moved`, `scope=brain:<id>`) |
| an alarm told | receiver `POST /instructions`; the owner's sender and room from `GET /owner`; a member's room by `GET /rooms/{roomId}` |
| whose account is whose | the account service `GET /senders/{sender}/user` (the owner's account), `GET /users/{user}` (a member's name and senders) |

The brain's address and token are `BRAIN_ADDRESS` and `BRAIN_TOKEN` from `env/brain.env`; the receiver's
`listen` from `env/receiver.toml` and its token from `env/receiver.token`; the account service's `[server]
listen` and `token` from `env/web.toml`; each under the data root, read afresh for every call, and each
overridden by the settings where they name one (「Settings」). Every call waits for its answer; no clock cuts
one. A brain or receiver that refuses the connection is asked again two seconds later; the brain's event
stream, once broken, is opened again from its last event two seconds later, and a resume the brain answers
410 `gone` reads the realm's memories whole again.

## Codes

Beside the common codes of bmo-chat's `docs/conventions.md` (`bad-request`, `unauthorized`, `forbidden`,
`no-route`, `not-found`, `gone`, `failed`, `upstream-failed`, `not-ready`), each answered 422:

| Code | When |
|---|---|
| `no-account` | a `mine` call names no account, or its holder's account cannot be found |
| `not-a-writer` | a write the caller may not make (answered 403) |
| `end-before-start` | an end not after its start |
| `unknown-calendar` | an event written under a calendar the realm does not hold live |
| `span-too-long` | a span over 92 days, or one ending before it starts |
| `not-an-occurrence` | an `occurrence` that is not one of the event's |

`upstream-failed` (502) is the brain, the receiver or the account service answering a failure;
`not-ready` (503) is a realm asked for before the service has read the brain once.

## Settings

The settings are read at start in layers, each overriding the one before: the defaults built into the binary
(the repository's `settings.toml`); then `settings.toml` and `settings.local.toml` in the settings folder,
the one `BMO_CALENDAR_SETTINGS` names, else the folder above the one holding the binary (a deployment's
root, where `settings.local.toml` links to the data root's `env/calendar.toml`); then
`BMO_CALENDAR_<TABLE>__<KEY>` in the environment.

| Table | Key | What it is |
|---|---|---|
| `server` | `listen` | where the service answers, `host:port` |
| `server` | `token` | the service token; an empty one lets no caller in |
| `server` | `data_root` | the data root whose `env/` the peers' addresses and tokens are read from, and under which `tool-env` writes `env/calendar.env` |
| `console` | `token` | the console credential; empty, the console reaches nothing |
| `console` | `folder` | the console bundle's built folder (`client/dist` where none is set; a relative one read from where the process starts) |
| `store` | `path` | the SQLite file of the told marks |
| `calendars` | `mine_name`, `organization_name` | the names of the calendar a realm is given when it holds none |
| `brain`, `receiver`, `accounts` | `address`, `token` | a peer's base address and token where the data root's files are not to be read; empty, the files are read |
