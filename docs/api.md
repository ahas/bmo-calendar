# The boundary API

Every route bmo-calendar answers, the events it publishes, the tool entries it places, and the calls it
makes. The wire format is the platform's (bmo-chat `docs/conventions.md`): camelCase keys, times in epoch
milliseconds unless a field says it is a local ISO datetime, ids as strings the brain's keys are made of,
a failure a non-2xx status with `code` (kebab-case) and `error` (an English sentence), a list `{items,
more}` read with `after` and `limit`. `docs/openapi.json` is generated from the service's own types by
`bmo-calendar openapi`, and a test fails where the committed document differs.

## The control port

The service listens on the loopback at the address `env/calendar.toml` names (`[server] listen`, by
convention `127.0.0.1:13109`), under `Authorization: Bearer <token>` with the service token
(`[server] token`, written to `env/calendar.token`). The console reaches it with the console credential
(`[console] token`) and the headers the console sets on every relayed call: `Owner-Key`, the signed-in
account's id, and `Owner-Role`, `administrator` or `member`. A call with no credential is 401; a console
call with no `Owner-Key` is 403.

## Realms

A realm is `mine` or `organization`. A route that takes a realm reads it from the path. In the `mine`
realm the account is the caller's `Owner-Key`; a service-token call names it with `account=<key>`.

## Health

| Method | Path | Does |
|---|---|---|
| GET | `/health` | `ready`, `reason` (null when ready), `version` |

## Calendars

| Method | Path | Does |
|---|---|---|
| GET | `/{realm}/calendars` | The realm's calendars, the caller's `hidden` mark on each |
| POST | `/{realm}/calendars` | `name`, `color`; organization: `writers`, `tell` (defaults `creator`) — 201 |
| PATCH | `/{realm}/calendars/{calendarId}` | `name`, `color`, `writers`, `tell`, `hidden` — 200 |
| DELETE | `/{realm}/calendars/{calendarId}` | Retires it (its memory `done`); its events are retired with it — 204 |

A calendar is `id`, `realm`, `name`, `color`, `writers`, `tell`, `hidden`, `creator`, `createdAt`. A realm
whose brain holds no calendar yet answers one made on first read: `mine` → 「내 일정」 `primary`;
`organization` → 「조직 일정」 `info`. A member may not write an organization calendar whose `writers` is
`administrator`; 403 `not-a-writer`.

## Events

| Method | Path | Does |
|---|---|---|
| GET | `/{realm}/events?start=&end=` | The events overlapping the span (local ISO datetimes, at most 92 days); a repeating event answers one row per occurrence in the span, `occurrence` naming its start |
| GET | `/{realm}/events/{eventId}` | One event |
| POST | `/{realm}/events` | 201 |
| PATCH | `/{realm}/events/{eventId}` | Changes the fields named; `occurrence` limits a repeating event's change to that one (which becomes an event of its own) — 200 |
| DELETE | `/{realm}/events/{eventId}` | Cancels it (`done`); `occurrence` cancels one occurrence — 204 |

An event is `id`, `realm`, `calendarId`, `title`, `description` (may be empty), `start`, `end` (local
ISO datetimes, `end` after `start`), `allDay`, `alarm` (`none` or minutes before start: 0, 5, 15, 30, 60,
1440), `repeat` (`none`, `daily`, `weekly`, `monthly`, `yearly`), `creator`, `createdAt`, `changedAt`,
`told` (the occurrence last told, or null). A write names `calendarId`, `title`, `start`, `end` and may
name the rest; a write under a calendar the caller may not write is 403 `not-a-writer`; an end not after
the start is 422 `end-before-start`; an unknown calendar is 422 `unknown-calendar`.

Every write is idempotent on `id`: a POST that names an `id` the realm already holds answers 200 with the
event as it stands after the write (the console's offline replay sends the same write twice).

## Alarms

| Method | Path | Does |
|---|---|---|
| GET | `/{realm}/alarms?from=&to=` | The alarm moments in the span, each `eventId`, `occurrence`, `at`, `told` |
| POST | `/beats` | Service token only: runs one beat now (the minute's work: tell what is due) — 200 with what it told |

The beat runs every minute on the service's own clock and once at start. It tells each due alarm once:
the mark `told:<eventId>:<occurrence>` is kept in the unit's own store, and the event's `told` is read
from it. A due alarm found at start after the service was down is told with how late it is.

How an alarm is told (「Calls the calendar makes」): one `POST /instructions` on the receiver, sort
`calendar`, kind `calendar:<eventId>:<occurrence>`, `cause` `reply`, `said.order` naming the event's
title, its start, its calendar and realm, and that the owner asked to be reminded, in English; `room` the
owner's private room (the receiver's `GET /owner`) for the owner's own events and for organization events
whose `tell` reaches the owner, and the room the receiver links to a member's account where it answers
one (the receiver's accounts route), else the owner's private room naming the member.

## Console

| Method | Path | Does |
|---|---|---|
| GET | `/console-manifest` | The bundle manifest (bmo-ui `docs/console-bundles.md`): `name` `calendar`, `title` `calendar.title`, `icon` `i-material-symbols-calendar-month-outline`, `screenPath` `/calendar`, `role` `member`, `ownerKey` true, `events` the stream below, `pushKinds` `[alarm.told]` |
| GET | `/console/words` | The bundle's words, every locale |
| GET | `/console/{file}` | The bundle's module and stylesheet |

## Events the calendar publishes

`GET /events` is a server-sent stream in the platform's shape (`after`, `kinds`): `calendar.changed`
(`realm`, `calendarId`), `event.changed` (`realm`, `eventId`, `occurrence?`), `alarm.told` (`realm`,
`eventId`, `occurrence`, `at`). The console refreshes the visible span on `event.changed` and rings
`alarm.told`.

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

The `mine` realm of a tool call is the account the turn's holder is linked to: the owner's for the
supervisor's turn (the session's `holderName` and the account the receiver links it to); a call that cannot
name an account is refused `no-account`. The tool reaches the service with the service token from
`env/calendar.env` (`CALENDAR_ADDRESS`, `CALENDAR_TOKEN`), which the deployment writes before the flip.

## Calls the calendar makes

| What | Route |
|---|---|
| the realm's brain, made on first write | brain `GET /brains?accountKey=&agentKey=`, `POST /brains` |
| calendars and events read, written, versioned, retired | brain `GET /memories`, `POST /memories`, `POST /memories/{id}/versions`, `PATCH /memories/{id}` |
| the brain's changes followed | brain `GET /events` (`memory.written`, `memory.moved`, `scope=brain:<id>`) |
| an alarm told | receiver `POST /instructions`; the owner's room from `GET /owner`; a member's room from the receiver's account link |

The brain's address and token are `BRAIN_ADDRESS` and `BRAIN_TOKEN` from `env/brain.env`; the receiver's
from `env/receiver.toml` and `env/receiver.token`. Every call waits for its answer; no clock cuts one.
