# The calendar tool

Every entry names a realm: `mine`, the owner's own calendars, or `organization`, the organization's. Times are
local datetimes in the shape `YYYY-MM-DDTHH:mm:ss`, on the calendar host's clock.

- **`calendar_list`** answers the realm's calendars, one line each: its id, its name and its colour.
- **`calendar_events`** answers the events overlapping a span (`start` and `end`, at most 92 days apart), or
  with `n` alone the next `n` from now. A repeating event answers one line per occurrence, which names the
  occurrence's start; that start is what `calendar_change` and `calendar_cancel` take as `occurrence`.
- **`calendar_show`** answers one event by its id.
- **`calendar_add`** adds an event. `title` and `start` are required; `end` is an hour after the start where
  none is given, or the whole day where `allDay` is true; `calendarId` is the realm's first calendar where none
  is given. Set `alarm` (minutes before the start: `0`, `5`, `15`, `30`, `60` or `1440`) when the owner asks
  to be reminded: the calendar tells this room's supervisor when the moment comes.
- **`calendar_change`** changes the fields named. With `occurrence`, only that occurrence of a repeating event
  changes, and it becomes an event of its own.
- **`calendar_cancel`** cancels an event, or with `occurrence` one occurrence of a repeating event.

Examples:

- The owner asks to be reminded of a dentist appointment tomorrow at three in the afternoon:
  `calendar_add` with `{"realm": "mine", "title": "Dentist", "start": "2026-11-04T15:00:00", "alarm": "60"}`.
- What stands this week: `calendar_events` with `{"realm": "mine", "start": "2026-11-02T00:00:00", "end":
  "2026-11-09T00:00:00"}`.
- The next three of the organization's: `calendar_events` with `{"realm": "organization", "n": 3}`.
