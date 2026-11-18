# bmo-calendar

The platform's dated events as one service: calendars and events kept as memories in the brain service (the
brain the master, this unit the subordinate), in two realms kept apart, each account's own and its company's
organization realm; a repeating event answered one row per occurrence; an alarm told once to the supervisor
through the receiver when it comes due; a company's rooms and other resources booked, two bookings of one
never meeting; and the console bundle that shows them under its sidebar entry.
`docs/design.md` states what it is and `docs/api.md` its boundary: every route, the events it publishes, the
tool it places, the calls it makes and its settings. Every other unit is reached through its API.

## Tests

`cargo test` (through the testbed) runs the crate's one harness, `tests/main.rs`, over stand-ins of the brain,
the receiver, the account service and agency on a loopback port: the calendars made on first read and kept per
realm (`calendars.rs`), the administrator reading a person's own realm and writing and making nothing there
(`administrator.rs`), each company's realm read by its own and the administrator alone, his writes in another
company's refused, a tool session's holder acting for its owner's account, and the one move into companies
(`companies.rs`), resources and
bookings, overlaps refused, approvals and their tellings (`bookings.rs`), whether two bookings ever meet
(`overlap.rs`), received schedules kept in their companies (`schedules.rs`), events written, changed and
cancelled with their labels, idempotent by id, refused as documented and refreshed by the brain's stream
(`events.rs`), repeating events expanded in a span and one occurrence changed or cancelled on its own
(`repeats.rs`), the beat telling a due alarm once, a late one with its lateness and none twice across a
restart, each in the room its account reaches (`alarms.rs`), the console's guard, the console face's manifest
read through bmo-ui's own reading of the contract, and its events (`console.rs`), the OpenAPI document
(`openapi.rs`) and the repository's own checks (`repository.rs`). The tool's crate carries its own harness,
`tools/calendar/tests/main.rs`, against a stand-in calendar service. The console bundle's check is `bun run
check` inside `client/`.

## Screen check

The console bundle's screens are driven in a browser without the console or any live unit:

1. `cargo test --test main preview -- --ignored --nocapture` starts the service over the test stand-ins with a
   company's rooms, bookings and an organization event in it, prints the address it answers at, and serves
   until it is stopped.
2. From `client/preview/`, `PREVIEW_SERVICE=<that address> PREVIEW_KEY=<account> PREVIEW_ROLE=<member or
   administrator> bun x nuxi dev --port <port>` serves the screens as a Nuxt app over it: the client as a
   layer, the bundle's words installed and its relay calls proxied with the console's credential and that
   reader's key and role. The stand-ins' accounts are the owner `1` (company `10`), member `2` (a head of
   it), member `3` (keeper of the rooms that wait for approval), member `4` (another company) and `5` (none).
3. A browser driver reads the screens' accessibility snapshot and measures them inside the page, at a
   desktop width and at a phone's (360 by 780).

What a sweep measures is kept in the console bundle's browser tests, `client/tests/browser/` (declared under
`bmo.browser` in `client/package.json` and run by the testbed): the screens built into one harness page with no
console or service around them, opened in a real browser at a desktop's width and a Galaxy S25's (412 by 915), a
stand-in answering the service as it answers the platform administrator.

## Layout

| Folder | Holds |
|---|---|
| `src/` | The service and its command: the routes, the cache of each realm's brain, the companies and the move into them, resources and bookings, the alarms and the beat, the told marks' store, the console face's declaration and the OpenAPI document |
| `tools/` | The calendar tool the deployment places on the tool host: its crate, its manifest, its input schemas and its manual |
| `client/` | The console plugin bundle, a project of its own |
| `deploy/` | The service's user unit |
| `docs/` | The canonical statements of what the service is and its boundary, and the OpenAPI document |
| `tests/` | The crate's one test harness and the stand-ins it runs over |

At the root stand the Cargo manifest, `Cargo.lock`, `testbed.toml`, `rustfmt.toml`, `settings.toml` (the
defaults built into the binary), `CLAUDE.md`, `PERSONAL-DATA.md` and `LICENSE`.

## Commands

- `bmo-calendar serve` reads the brain, beats once a minute and answers the routes until it is told to stop.
- `bmo-calendar health` ends 0 where the service its settings name answers ready.
- `bmo-calendar openapi` prints the OpenAPI document, which `docs/openapi.json` holds.
- `bmo-calendar tool-env` writes the data root's `env/calendar.env`, where the tool finds the service.

The settings are read from the binary's built-in `settings.toml`, then the deployment's `settings.toml` and
`settings.local.toml` beside the release (the data root's `env/calendar.toml`), then `BMO_CALENDAR_<TABLE>__<KEY>`
in the environment (`docs/api.md`, 「Settings」).
