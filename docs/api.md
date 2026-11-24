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
realm reads it from the path. In the `mine` realm the account is the caller's `Owner-Key`. The `account` a
console call names is not read, but for the platform administrator (a console reader whose role is
`administrator`) in the `mine` realm: there it names the account whose realm he reads, as that account reads
it, its `hidden` marks among it, and a write naming another account than his own is 403 `forbidden`
(「What the administrator writes」). A service-token call names the account with `account=<key>`, or with
`holderName=<name>`, the holder of a tool session, the agent the session's record is held by. A holder acts
for the account agency names as its owner: a supervisor's `ownerAccount` on agency's `GET /supervisors/{supervisorName}`, else a role agent's `ownerAccount` among
agency's `GET /role-agents`, so a supervisor of a person's account acts for that person. A holder agency names
no account for, a supervisor of no account or a name agency does not hold, acts for the owner: the account the
account service links to the owner's sender (the receiver's `GET /owner`). A service-token call in the `mine`
realm that names neither, or whose holder acts for the owner and the owner's sender is linked to no account,
is 422 `no-account`; an agency that does not answer, or answers something that does not read, is 502
`upstream-failed`, never read as the owner. In the `organization` realm the same query names the account a
write records as its creator and whose `hidden` mark a read answers.

### Companies

The `organization` realm is one company's (`docs/design.md`, 「Two realms, kept apart」). A route in it reads
the company from the query's `company`: a company's unit id as the account service names it, or the word
`platform` for the platform's own realm.

- **Where none is named,** the company is the caller's own: the company of the account the call acts for,
  asked of the account service (`GET /users/{user}/unit`) at every call. A service-token call that names no
  account and no holder has no company of its own.
- **Who reads which.** An account reads its own company alone; the platform administrator (a console
  reader whose role is `administrator`, and every service-token call) reads every company and `platform`,
  and writes in his own alone (「What the administrator writes」).
  A `company` the caller does not read is answered as if nothing stood there: 404 `not-found`.
- **No company.** A call with no company, named or its own, reads nothing: its lists are empty; its writes
  are 422 `no-company`.
- **Every organization row names its company:** a calendar, an event, a schedule, a resource and a booking
  read carry `company`, the company's id or `platform`.

| Method | Path | Does |
|---|---|---|
| GET | `/organization/companies` | The companies the caller reads, each `id`, `name`, `own` (the caller's own), `writes` (whether the caller writes in its organization realm: every reader in its own company, the administrator also in `platform`, the service token in every one) and `manages` (whether the caller makes, changes and retires its resources: the administrator and a head of the company for their own company, the service token for every company but `platform`), the caller's own first, then by name, `platform` (named by the setting `[calendars] platform_name`) last for the administrator — `{items, more}` |

The administrator's companies are the account service's every company (`GET /units`, asked with the
caller's owner key and role) and every company whose brain the calendar holds; an id the account service
no longer names is listed by its id.

### What the administrator writes

The platform administrator reads everything and changes nothing of another's (`docs/design.md`, 「Two
realms, kept apart」). He reads every company's `organization` realm (「Companies」) and every account's
`mine` realm (`account=<key>`), and writes in his own realms alone: his own `mine` realm, his own company's
`organization` realm and the platform's own. Any write in another realm is 403 `forbidden`: a calendar, an
event, a `hidden` mark on a calendar or a source, a resource, a booking and a decision alike. His read of a
realm not his own makes nothing in it: no brain, no first calendar (「Calendars」) and no mark. The service
token, the units' own credential, writes in whichever realm its call names.

## Removing an account or a company

The platform administrator deleting an account or an organisation unit for good takes the realm the calendar keeps
of it (`docs/design.md`, 「Two realms, kept apart」). These routes stand outside the console face's guard: the service
token reaches them, and the console credential only with `Owner-Role` `administrator`; any other caller is 403
`forbidden`. An account, a unit and a company are their ids as the account service names them, positive decimal
digits; any other is 400 `bad-request`.

| Method | Path | Does |
|---|---|---|
| GET | `/accounts/{account}/removal` | What the removal below would take now, changing nothing: `{"counts": {"calendars", "events"}}` |
| DELETE | `/accounts/{account}` | The account's `mine` realm let go of: its brain deleted on the brain whole, its told marks with it; answers `{"removed": {"calendars", "events"}}` |
| GET | `/org-units/{unit}/removal?company={company}` | For a company (`unit` equal to `company`), what its organization realm holds: `{"counts": {"calendars", "events", "schedules", "resources", "bookings"}}`; zeros for a unit below a company, whose things are its company's |
| DELETE | `/org-units/{unit}?company={company}` | For a company, its organization realm let go of as an account's is; answers `{"removed": {...}}` |

A realm is counted from what the calendar holds of it, live memories alone, and read from the brain first where
it is not held yet. A removal is the same again where nothing stands: a second call answers zeros, so a removal
broken half-way is finished by calling again. A booking a removed account made in its company's realm is the
company's and stays; the platform's own realm is never removed this way. `upstream-failed` (502) is the brain
answering a failure.

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

A calendar is `id`, `realm`, `company` (null in the `mine` realm), `name`, `color`, `writers`, `tell`,
`hidden`, `creator`, `createdAt`; `writers` and `tell` are null in the `mine` realm. A realm whose brain holds
no live calendar answers one made on the read: `mine` → 「내 일정」 `primary`, id `mine`; `organization` →
「조직 일정」 `info`, id `organization`, in each company (the names are settings, 「Settings」, which the
deployment sets to these words; the repository's defaults are English). The administrator's read of a realm
not his own makes none, and answers the calendars that stand there, none where none does (「What the
administrator writes」). `hidden` is the caller's own: a PATCH of it hides or
shows the calendar for the caller's account alone and is open to every account that writes in the realm. In
the `organization` realm, making a calendar is the administrator's, and changing its `name`, `color`, `writers` or
`tell` or retiring it the administrator's and its creator's; anyone else is 403 `not-a-writer`. A `color`,
`writers` or `tell` outside its words, or a blank name, is 400 `bad-request`; a calendar the realm does not
hold live is 404 `not-found`.

## Events

| Method | Path | Does |
|---|---|---|
| GET | `/{realm}/events?start=&end=` | The events overlapping the span (local ISO datetimes, at most 92 days), by start — `{items, more}`; a repeating event answers one row per occurrence in the span, `occurrence` naming its start |
| GET | `/{realm}/events/{eventId}` | One event |
| POST | `/{realm}/events` | 201 |
| PATCH | `/{realm}/events/{eventId}` | Changes the fields named; `occurrence` limits a repeating event's change to that one (which becomes an event of its own) — 200 |
| DELETE | `/{realm}/events/{eventId}` | Cancels it (`done`); `occurrence` cancels one occurrence — 204 |

An event is `id`, `realm`, `company` (null in the `mine` realm), `calendarId`, `title`, `description` (may be
empty), `start`, `end` (local ISO datetimes, `YYYY-MM-DDTHH:mm:ss`, `end` after `start`), `allDay`, `alarm`
(`none` or minutes before start: `0`, `5`, `15`, `30`, `60`, `1440`, each a string), `repeat` (`none`,
`daily`, `weekly`, `monthly`, `yearly`), `occurrence` (in a span's rows, the start of the occurrence a
repeating event's row stands for, whose `start` and `end` the row carries; null otherwise), `creator`,
`createdAt`, `changedAt`, `told` (the occurrence last told, or null). A write names `calendarId`, `title`,
`start`, `end` and may name the rest and an `id` (1 to 64 letters, digits, `-` and `_`; one is made where none
is given); a title is one line. A write under a calendar the caller may not write is 403 `not-a-writer`; an
end not after the start is 422 `end-before-start`; an unknown calendar is 422 `unknown-calendar`; a span over
92 days or ending before it starts is 422 `span-too-long`; an `occurrence` that is not one of the event's is
422 `not-an-occurrence`; an event the realm does not hold live is 404 `not-found`; a field outside its shape
or its words is 400 `bad-request`. In the `organization` realm a calendar whose `writers` is `administrator`
is written by the administrator alone, one whose `writers` is `creator` takes new events from every account
and a change or a cancellation from the administrator and the event's creator, and one whose `writers` is
`everyone` from every account.

Every write is idempotent on `id`: a POST that names an `id` the realm already holds answers 200 with the
event as it stands after the write (the console's offline replay sends the same write twice).

## Sources and schedules

Another unit pushes its own dated items here as **schedules** under a **source** of its own
(`docs/design.md`, 「Received schedules」). A source is only data: the name a unit writes under. The
calendar's code names no unit, and treats every source alike.

| Method | Path | Does |
|---|---|---|
| PUT | `/sources/{sourceId}` | Service token only: the source's `name` and `color` (where none is given, the one it has, else `secondary`) — 201 made, 200 otherwise |
| PUT | `/sources/{sourceId}/schedules/{itemId}` | Service token only: writes or changes one schedule — 201 made, 200 otherwise |
| DELETE | `/sources/{sourceId}/schedules/{itemId}` | Service token only: removes one schedule — 204, also where none stands |
| PUT | `/sources/{sourceId}/schedules` | Service token only: the source's whole set, `{items}` — 200, `written`, `unchanged`, `removed` (counts) |
| GET | `/{realm}/sources` | Every source, the caller's `hidden` mark for the realm on each, oldest first — `{items, more}` |
| PATCH | `/{realm}/sources/{sourceId}` | `hidden` — 200 |
| GET | `/{realm}/schedules?start=&end=` | The schedules overlapping the span (local ISO datetimes, at most 92 days), by start — `{items, more}` |

- **A source** is `id`, `name`, `color` (one of the six), `hidden`, `createdAt`. A schedule written under a
  source the calendar does not hold yet makes the source, named by its id, its colour `secondary`; a
  `PUT /sources/{sourceId}` names it and colours it at any time, and keeps every account's `hidden` mark.
- **A schedule written** is `title` (one line, not blank), `start` and `end` (local ISO datetimes, `end`
  after `start`), `allDay` (default `false`, read as an event's), `link`, `assignee` and `company`. `link`
  is the place the item it stands for is opened at: a console path (one `/` and then the path) or an `http`
  or `https` address. `assignee` is the account key of the person the item is assigned to, at most 200
  characters; null, empty or absent where it has none. `company` is the id of the company the item belongs
  to, as the account service names its unit (1 to 64 letters, digits, `-` and `_`); null, empty or absent
  where it belongs to none. A unit sends the company of every item it has one for: a schedule with none is
  read by the platform administrator alone, in the `platform` company. The calendar does not ask whether a
  company stands. `private` (default `false`) says the schedule is its assignee's alone: it stands in the
  assignee's `mine` realm and in no `organization` realm, so no other reader of the company, nor of the
  `platform` company, is answered it. A private schedule names both its `assignee` and its `company`.
- **A schedule read** is `sourceId`, `itemId`, `realm` (the realm it is read in), `company` (`platform`
  where it was sent with none), `title`, `start`, `end`, `allDay`, `link`, `assignee` (null where none),
  `private`, `createdAt`, `changedAt`.
- **The ids.** A `sourceId` and an `itemId` are each 1 to 64 letters, digits, `-` and `_`; a unit writes
  under its own source and keeps its own item ids, so a schedule is known by the pair alone.
- **Writes are idempotent.** A write that names what already stands keeps it and answers 200; a delete of a
  schedule that does not stand answers 204, so a unit sends a write again without asking first. A write
  naming another company than the one the schedule stands in moves it: it is written in the new company and
  removed from the old.
- **The whole set.** `PUT /sources/{sourceId}/schedules` takes `items`, each a schedule written with its
  `itemId`, of every company. Every item is checked before anything is written; then each one that differs
  from what stands is written, each standing schedule of the source in any company that the set does not
  name is removed, and the rest is left as it is. An empty set removes every schedule of the source. One
  source's writes are made one at a time, so two sets never interleave. A set the brain fails part way
  through answers 502 and keeps what was written; the same set sent again finishes it.
- **Who writes.** The writes are the service token's, the units' own credential (`env/calendar.env`,
  「The tool」); a console call to one is 403 `forbidden`.
- **Who reads.** Every console reader reads every source. In the `organization` realm a read answers every
  schedule of the company but the private ones (「Companies」); in the `mine` realm only the schedules of the
  account's own company whose `assignee` is the account, private or not (「Realms」), so the administrator reads a
  private schedule where he reads its assignee's realm, and nowhere else. `hidden` is the caller's own for the realm in the path, as a
  calendar's is: a PATCH of it hides or shows the source's schedules in that realm for the caller's account
  alone. A read answers the hidden ones as well; the screen leaves them off the grid.
- **Refusals.** An id, a title, a link, an assignee, a company or a colour outside its shape, a private
  schedule naming no assignee or no company, or an `itemId` named twice in one set, is 400 `bad-request`; an end not after the start is 422 `end-before-start`; a span
  over 92 days or ending before it starts is 422 `span-too-long`; a source the calendar does not hold in a
  PATCH is 404 `not-found`.
- **No alarms.** A schedule is never told: its unit tells of its own items.

## Resources and bookings

A company's resources and their bookings (`docs/design.md`, 「Resources and bookings」), in its
`organization` realm and read by its company alone (「Companies」); a booking also stands in its booker's
`mine` realm.

| Method | Path | Does |
|---|---|---|
| GET | `/organization/resources` | The company's live resources, by kind, place and name — `{items, more}`; `kind` narrows them |
| GET | `/organization/resources/free?start=&end=&seats=&repeat=&kind=` | The resources with at least `seats` (1 where none) that no standing booking meets over the span and its repeat (`none` where none), fewest seats first, then by name — `{items, more}` |
| POST | `/organization/resources` | `name`, `kind`, `place`, `seats`, `approval`, `keepers`, `id?` — 201 |
| PATCH | `/organization/resources/{resourceId}` | Changes the fields named — 200 |
| DELETE | `/organization/resources/{resourceId}` | Retires it (`done`) and cancels its standing bookings — 204 |
| GET | `/{realm}/bookings?start=&end=` | The bookings overlapping the span (local ISO datetimes, at most 92 days), by start, one row per occurrence as events answer — `{items, more}`; in the `organization` realm every requested and booked one of the company, `resourceId` narrowing them; in the `mine` realm the account's own, requested, booked and refused |
| GET | `/organization/bookings/requests` | The requested bookings of the resources the caller keeps, oldest request first, a booking a row — `{items, more}` |
| GET | `/organization/bookings/{bookingId}` | One booking |
| POST | `/organization/bookings` | `resourceId`, `title`, `start`, `end`, `allDay?`, `repeat?`, `id?` — 201 |
| PATCH | `/organization/bookings/{bookingId}` | Changes the fields named, `resourceId` among them; `occurrence` limits a repeating booking's change to that one (which becomes a booking of its own) — 200 |
| DELETE | `/organization/bookings/{bookingId}` | Cancels it (`done`); `occurrence` cancels one occurrence — 204 |
| POST | `/organization/bookings/{bookingId}/decision` | A keeper's answer to a requested booking: `approve` (`true`/`false`) and, refusing, `reason` — 200 |

- **A resource** is `id`, `company`, `name`, `kind`, `place`, `seats`, `approval`, `keepers`, `creator`,
  `createdAt`, `changedAt`. `name`, `kind` and `place` are one line each, `name` and `kind` not blank;
  `seats` a whole number from 1 to 10000; `keepers` the account keys of people of the company, at least one
  where `approval` is `true`. Making, changing and retiring a resource is a head of the company's (the
  account service's `POST /checks/unit` on the company) and, in his own company, the administrator's;
  anyone else is 403 `not-a-writer`, and the administrator in another company 403 `forbidden`. A keeper of another company, or standing in none, is 422 `not-in-company`.
- **A booking** is `id`, `realm`, `company`, `resourceId`, `title`, `start`, `end` (local ISO datetimes,
  `end` after `start`), `allDay`, `repeat` (as an event's), `occurrence` (as an event's), `booker`,
  `state` (`requested`, `booked` or `refused`), `decidedBy`, `decidedAt`, `reason` (each null where none),
  `createdAt`, `changedAt`. A write's title is one line, not blank; its `id` as an event's. An all-day
  booking holds its days whole.
- **Who books.** Any account of the company books, as the booker; the booker and a keeper of the resource
  change and cancel; a keeper alone decides. Anyone else is 403 `not-a-writer`, and a decision by one who
  keeps no such resource 403 `not-a-keeper`. A booking is made in its booker's own company: one written in
  another is 403 `not-a-writer`. The administrator reads every company's bookings, and his every write in
  another company is 403 `forbidden` (「What the administrator writes」).
- **The state a write leaves.** On a resource whose `approval` is `true`, a booking made by anyone but a
  keeper stands `requested`, and a change of its `start`, `end`, `allDay`, `repeat` or `resourceId` by
  anyone but a keeper makes it `requested` again; otherwise it stands `booked`. A decision approving makes
  it `booked`; one refusing makes it `refused`, holding no span, until its booker is told, when it is moved
  to `done`.
- **No overlap.** A write that would make a standing (`requested` or `booked`) booking of the resource overlap
  another standing one is 409 `overlap`, its `error` naming the booking it meets in the words `the booking
  "<title>" (<id>) of the same resource`, which the screen reads the met booking's title from, and its first
  occurrence that meets. Spans are half-open: one may start where another ends. Repeating bookings are checked
  occurrence by occurrence over 28 years from the later of the two starts. The check and the write run under
  one writer for every booking of the unit, so of two writes made at once one is refused.
- **Idempotent on `id`,** as an event's write is: a POST naming an `id` the company already holds answers 200
  with the booking as it stands after the write.
- **Refusals.** An end not after its start is 422 `end-before-start`; a span over 92 days or ending before it
  starts 422 `span-too-long`; a resource the company does not hold live 422 `unknown-resource`; an
  `occurrence` that is not one of the booking's 422 `not-an-occurrence`; a decision on a booking that is not
  `requested` 409 `not-requested`; a resource or a booking the company does not hold live 404 `not-found`;
  a field outside its shape or its words 400 `bad-request`.
- **Who is told** (「Alarms」 for how): the keepers of the resource, once each time a booking comes to stand
  `requested`, the instruction's kind `booking:<bookingId>:requested:<requestedAt>` and its order opening
  on the marker `CALENDAR BOOKING REQUEST.`; the booker, once a keeper decides, the kind
  `booking:<bookingId>:decided:<decidedAt>` and the marker `CALENDAR BOOKING DECISION.`. Each order carries
  the booking's data alone: its title, its resource and place, its span and repeat, its booker, the
  decision and its reason where there is one. A telling is marked told in the store
  (`told:<bookingId>:<the kind's last two parts>`) and one the receiver does not take is tried again at the
  next beat.

## Alarms

| Method | Path | Does |
|---|---|---|
| GET | `/{realm}/alarms?from=&to=` | The alarm moments in the span (epoch ms, at most 92 days), by moment — `{items, more}`, each `realm`, `eventId`, `occurrence`, `at`, `told` |
| POST | `/beats` | Service token only (403 `forbidden` to the console): runs one beat now (the minute's work: tell what is due) — 200, `told`, each alarm told: `realm`, `company` (null in the `mine` realm), `eventId`, `occurrence`, `at`, `lateMs`, `rooms`; and `bookings`, each booking telling made: `company`, `bookingId`, `kind`, `rooms` |

The beat runs every minute on the service's own clock and once at start. It tells each due alarm once, and
each booking telling not yet told (「Resources and bookings」):
the mark `told:<eventId>:<occurrence>` is kept per realm in the unit's own store, and the event's `told` is
read from it. An alarm is due when its moment has come and it came after the last beat the store
remembers, or when its event was written after its moment and has not started yet. A due alarm found at
start after the service was down is told with how late it is (`lateMs`, 0 where it is on time).

How an alarm is told (「Calls the calendar makes」): one `POST /instructions` on the receiver for each room it
reaches, sort `calendar`, kind `calendar:<eventId>:<occurrence>`, `cause` `reply`, `summary` the event's
title, `said.order` opening on the marker `CALENDAR ALARM.` and then the data alone, in English: the event's
title, its start, its calendar and realm, how late it is where it is late, that a reminder of it was asked
for, and whom it is for. The turn carries no instruction; how a quiet instruction is answered stands in the
receiving session's own rules. Who is told: in the `mine` realm the account; in the `organization` realm the
calendar's `tell`: `creator` the event's creator, `everyone` every account holding a `mine` realm that stands
in the calendar's company (the owner alone in the `platform` company), `nobody` none. Where each is told: the
owner's private room (the receiver's `GET /owner`) for the owner's account (the account the account service
links to the owner's sender); for a member, the room numbered as one of the senders the account service links
to the member's account (`GET /users/{user}`) that the receiver holds registered (`GET /rooms/{roomId}`), else
the private room the receiver keeps for the member's own BMO (of the rooms `GET /accounts/{account}/rooms`
lists, the lowest numbered whose `GET /rooms/{roomId}` names the account as its `privateOf`), else the owner's
private room naming the member. One room is told once, naming every account it is told for. A
telling the receiver refuses or does not answer keeps no mark, and the next beat tells it again.

## A person's Outlook day

One day of the Outlook calendar of the person a tool session serves, read on that person's own node
(`docs/design.md`, 「A person's Outlook day」). The service token alone calls it; the console's credential is 403
`forbidden`.

| Method | Path | Does |
|---|---|---|
| GET | `/outlook/day?holderName=&date=` | The day's events, by start — `{date, zone, road, node, items}` |

- **Whose.** `holderName`, the tool session's holder, names the account as the tool's calls name it (「The tool」);
  the route takes no `account`, so it reads no person's day but the holder's. A call naming no holder, or one
  whose account cannot be found, is 422 `no-account`.
- **The day.** `date`, `YYYY-MM-DD`, a date of the PC's own wall clock; today on the PC where none is given. A
  date of another shape is 400 `bad-request`.
- **The node.** The account's record in agency's `GET /person-nodes`, where it is not marked `shared`; for the
  deployment's own account, the account linked to the receiver's owner, the node `[outlook] owner_node` names
  where it names one. An account with neither is 422 `no-node`.
- **The account the run and the page token name.** The account itself; for the deployment's own account the one
  `[outlook] owner_account` names where it names one, the account his node and his page token are known by
  (`docs/design.md`, 「A person's Outlook day」).
- **The answer:** `date`; `zone`, the PC's time zone as Windows names it; `road`, `outlook` where the running
  Outlook was read and `graph` where Microsoft Graph was; `node`; and `items`, each `title`, `start` and `end`
  (local ISO datetimes on the PC's wall clock), `allDay`, `organizer` (a name, empty where none), `attendees` (how
  many required and optional attendees, the organizer and the rooms aside), `room` (the location, empty where
  none) and `titleParts`: `levels`, the work code's levels one to seven where the title opens on one (each as
  written, the first five always together), else none; `group`, the bracketed group, or null; and `task`, the
  rest of the title, the whole title where it is in no such form.
- **Failures.** A node service that refuses the run, a node out of reach, or a command that ends with no answer
  that reads is 502 `upstream-failed`, saying which. A node whose Outlook is not running, where no page-token service is named, it
  lends no token or Graph refuses the read, is 422 `no-road`, saying why. No answer is kept, and nothing is published.

## Console

The calendar's screen is a console bundle under bmo-ui's console plugin contract (bmo-ui
`docs/console-bundles.md`), which states the manifest, the bundle's files, the layer a bundle is built on
and the guard of every route a console reaches. The service serves its console face through bmo-chat's
`bmo-console-face` crate (bmo-chat `docs/console-face.md`), which assembles the manifest, answers the
bundle's files and guards them and the screens' routes as the contract states. This section names only what
the calendar chooses within it.

- **The bundle.** Named `calendar`, its title the word `calendar.title` and a month icon, its screens under
  `/calendar`, relayed as the service `calendar` with the whole API under the prefix `/`.
- **Its readers.** Every member and administrator, each with an owner key: the `mine` realm is the reader's
  own account (「Realms」).
- **What it reads.** The event stream (「Events the calendar publishes」), whose `alarm.told` and
  `booking.decided` a console raises as a notice, and the console's people directory, where a resource's
  keepers are chosen. No WebSocket, no share page, no sidebar count, no other service called and no route
  relayed outside the prefix.
- **Its files.** The folder `bun run bundle` builds in `client/` with bmo-ui's bundle build (`[console]
  folder`), read once at start: the module `bundle.js`, the stylesheet `bundle.css` and the words
  `words.json` (one pack to a locale: `en`, `ko`, `ja` and `zh-CN`, each holding every key), beside the
  layer that build wrote. A folder the face cannot be built from leaves the service
  with no console face, and the console's routes answer 404.
- **The screens' routes.** The companies, the calendars, the events, the sources, the schedules, the
  resources, the bookings, the alarms and the event stream, held to the manifest's role and owner key for a
  call carrying the console credential, and then to this document's own terms.

## Events the calendar publishes

`GET /events` is a server-sent stream in the platform's shape (`after` or `Last-Event-ID`, `kinds`; the
same events as a page to `Accept: application/json`, with `after` and `limit`): `calendar.changed`
(`realm`, `company`, `calendarId`), `event.changed` (`realm`, `company`, `eventId`, `occurrence`, null where
the change is the whole event's), `alarm.told` (`realm`, `company`, `eventId`, `occurrence`, `at`),
`source.changed` (`sourceId`), `schedule.changed` (`company`, `sourceId`, `itemId`), `resource.changed`
(`company`, `resourceId`), `booking.changed` (`company`, `bookingId`, `booker`) and `booking.decided`
(`company`, `bookingId`, `state`, `title`), `company` null in the `mine` realm. A console reader is given every
source's events, its own account's `mine` events, its own company's `organization` events and the
`booking.decided` of its own bookings; the administrator every company's; the service token reads every
one. A private schedule's `schedule.changed` is given to its assignee alone, and to the company's readers as
well where the change makes it private or opens it to the company. The service keeps the events since it started, at most the last 1000, numbered upward from its start's
moment in milliseconds times 1000; a resume from before them is 410 `gone`. The console refreshes the
visible span on `event.changed`, `schedule.changed` and `booking.changed`, reads the calendars, sources and
resources again on `calendar.changed`, `source.changed` and `resource.changed`, and rings `alarm.told` and
`booking.decided`.

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
| `calendar_resources` | The resources of the company, `kind?` narrowing them |
| `calendar_free_resources` | The resources free for a span: `start`, `end?` (an hour where none), `seats?`, `repeat?`, `kind?` |
| `calendar_book` | Books a resource: `resourceId`, `title`, `start`, `end?` (an hour where none, the day where `allDay`), `allDay?`, `repeat?`; answers the booking with its `state` |
| `calendar_cancel_booking` | Cancels a booking or one occurrence: `bookingId`, `occurrence?` |
| `calendar_outlook_day` | One day of the session's person's own Outlook calendar, read on their own node: `date?`, today on the PC where none (「A person's Outlook day」) |

The `mine` realm of a tool call is the account the session's holder acts for: the tool hands the session's
`holderName` to the service, which answers it with the account agency names as the holder's owner, else the
owner's account (「Realms」); a call that cannot name an account is refused `no-account`. The organization
realm, the resources and the bookings of a tool call are that account's company's, and the bookings it makes
are that account's. `calendar_events` with `n` and no span reads from now to 92 days ahead and answers the
first `n`. The tool reaches the service with the service token from `env/calendar.env` under the data root
(`CALENDAR_ADDRESS`, `CALENDAR_TOKEN`; the same names in the call's environment come first), which the
deployment writes before the flip with `bmo-calendar tool-env`.

## Calls the calendar makes

| What | Route |
|---|---|
| the realm's brain, made on first write | brain `GET /brains?accountKey=&agentKey=`, `POST /brains` |
| a realm removed for good | brain `DELETE /brains/{brainId}` |
| calendars, events, sources, schedules, resources, bookings and the move's record read, written, versioned, retired | brain `GET /memories`, `POST /memories`, `POST /memories/{id}/versions`, `PATCH /memories/{id}` |
| the brain's changes followed | brain `GET /events` (`memory.written`, `memory.moved`, `scope=brain:<id>`) |
| an alarm told | receiver `POST /instructions`; the owner's sender and room from `GET /owner`; a member's room by `GET /rooms/{roomId}`, else their own BMO's by `GET /accounts/{account}/rooms` |
| whose account a tool session's holder acts for | agency `GET /supervisors/{supervisorName}`, then `GET /role-agents` (each `ownerAccount`) |
| whose account is whose | the account service `GET /senders/{sender}/user` (the owner's account), `GET /users/{user}` (a member's name and senders) |
| an account's company | the account service `GET /users/{user}/unit` (its `company`; a 404 is no company) |
| the companies the administrator reads | the account service `GET /units`, with the caller's owner key and role |
| whether an account heads its company | the account service `POST /checks/unit` (`unit` the company's unit id as a number, `user` the account key as a string, as the account service's contract types them) |
| which node is an account's own | agency `GET /person-nodes` (each `ownerAccount`, `node`, `shared`) |
| a person's Outlook day read on their node | the node service `POST /nodes/{nodeName}/runs` (`account`, `command`, `shell` `powershell`, `env`), as the caller its own token names |
| a person's page token | the optional page-token service `POST /page-tokens` (`account` as a number, `scopes` `Calendars.Read`, `Calendars.ReadWrite`, `Calendars.Read.Shared`, `Calendars.ReadWrite.Shared`), as the caller its own token names |
| a person's day from Microsoft Graph | Graph `GET /me/calendarView` (`startDateTime`, `endDateTime`, the fields read alone, `Prefer: outlook.timezone`), with the person's page token, following `@odata.nextLink` |

The brain's address and token are `BRAIN_ADDRESS` and `BRAIN_TOKEN` from `env/brain.env`; the receiver's
`listen` from `env/receiver.toml` and its token from `env/receiver.token`; the account service's `[server]
listen` and `token` from `env/web.toml`; agency's control port from `env/agency.env` (`AGENCY_CONTROL_PORT`,
3220 where it names none, on the loopback) and its token from the same file (`AGENCY_CONTROL_TOKEN`, or the
file `AGENCY_CONTROL_TOKEN_FILE` names); the node service's `listen` from `env/node.toml` and the calendar's own
caller token from `env/calendar-node.token`;
each under the data root, read afresh for every call, and each overridden by the settings where they name one
(「Settings」). Graph is reached at `[outlook] graph_address`. Every call waits for its answer; no clock cuts
one. A brain or receiver that refuses the connection is asked again two seconds later; the brain's event
stream, once broken, is opened again from its last event two seconds later, and a resume the brain answers
410 `gone` reads the realm's memories whole again.

## Codes

Beside the common codes of bmo-chat's `docs/conventions.md` (`bad-request`, `unauthorized`, `forbidden`,
`no-route`, `not-found`, `gone`, `failed`, `upstream-failed`, `not-ready`), each answered 422:

| Code | When |
|---|---|
| `no-account` | a `mine` call names no account, or its holder's account cannot be found |
| `no-company` | an `organization` write with no company, named or its own |
| `not-a-keeper` | a decision by one who keeps no such resource (answered 403) |
| `not-in-company` | a resource's keeper who stands in another company or none |
| `unknown-resource` | a booking written on a resource the company does not hold live |
| `overlap` | a booking that would meet another standing booking of its resource (answered 409) |
| `not-requested` | a decision on a booking that is not `requested` (answered 409) |
| `not-a-writer` | a write the caller may not make (answered 403) |
| `end-before-start` | an end not after its start |
| `unknown-calendar` | an event written under a calendar the realm does not hold live |
| `span-too-long` | a span over 92 days, or one ending before it starts |
| `not-an-occurrence` | an `occurrence` that is not one of the event's |
| `no-node` | an Outlook day asked for an account with no node of its own |
| `no-road` | an Outlook day whose node runs no Outlook, where no page token is lent or Graph refuses the read |

`upstream-failed` (502) is the brain, the receiver, the account service, agency or the node service answering a
failure, or a node's command answering nothing that reads;
`not-ready` (503) is a realm asked for before the service has read the brain once and finished the move
into companies (`docs/design.md`, 「The move into companies」).

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
| `calendars` | `platform_name` | the name the platform's own organization realm is listed by among the companies |
| `brain`, `receiver`, `accounts`, `agency` | `address`, `token` | a peer's base address and token where the data root's files are not to be read; empty, the files are read |
| `node` | `address`, `token` | the node service's base address, and the calendar's own caller token there, where the data root's files are not to be read; empty, the files are read |
| `page_tokens` | `address`, `token` | the optional page-token service's base address, and the calendar's own caller token there; empty, none is asked and an Outlook day is read only from the running Outlook |
| `outlook` | `owner_node` | the node the deployment's own account's Outlook day is read on; empty, its agency record is read as any account's |
| `outlook` | `owner_account` | the account the deployment's own account is known by to the node service and the page-token service, which the run and the page token for its day name; empty, its own account |
| `outlook` | `graph_address` | Microsoft Graph's base address, `https://graph.microsoft.com/v1.0` |
