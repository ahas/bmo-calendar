# The calendar tool

The calendar acts for the person this session serves: the account that owns the supervisor holding it, or the
owner where none does. Every event entry names a realm: `mine`, that person's own calendars, or `organization`,
the calendars of that person's company. The resource and booking entries stand in that company. Times are local
datetimes in the shape `YYYY-MM-DDTHH:mm:ss`, on the calendar host's clock.

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
- **`calendar_resources`** answers the company's resources, one line each: its name, its kind, its seats, its
  id, its place, and whether a booking of it waits for a keeper's approval. `kind` narrows them.
- **`calendar_free_resources`** answers the resources free for a span (`start`, and `end`, an hour after the
  start where none is given) with at least `seats`, fewest seats first; `repeat` asks for a span that repeats.
- **`calendar_book`** books a resource for the person: `resourceId`, `title` and `start` are required; `end` is
  an hour after the start where none is given, or the whole day where `allDay` is true. Two bookings of one
  resource never meet: a span another booking holds is refused, naming that booking. On a resource that waits
  for approval the booking stands `requested`; its keepers are told, and the person is told their answer.
- **`calendar_cancel_booking`** cancels a booking by its `bookingId`, or with `occurrence` one occurrence of a
  repeating booking.
- **`calendar_outlook_day`** answers one day of the person's own Outlook calendar, read on their own PC: the
  running Outlook where it runs there, else Microsoft Graph with the sign-in of their Teams page. `date` is the
  day on that PC's clock, today there where none is given. Each event is one line: its time, its title, its
  organizer, how many attendees it has and its room; where the title opens on a work code, its levels and its
  group are named and the rest is the task. Its body is never read. A person with no PC of their own, or a PC
  where neither road stands, is refused saying why.

Examples:

- The owner asks to be reminded of a dentist appointment tomorrow at three in the afternoon:
  `calendar_add` with `{"realm": "mine", "title": "Dentist", "start": "2026-11-04T15:00:00", "alarm": "60"}`.
- What stands this week: `calendar_events` with `{"realm": "mine", "start": "2026-11-02T00:00:00", "end":
  "2026-11-09T00:00:00"}`.
- The next three of the organization's: `calendar_events` with `{"realm": "organization", "n": 3}`.
- What the person's work calendar held yesterday: `calendar_outlook_day` with `{"date": "2026-11-03"}`.
- A room for eight tomorrow from ten to eleven: `calendar_free_resources` with `{"start": "2026-11-04T10:00:00",
  "end": "2026-11-04T11:00:00", "seats": 8}`, then `calendar_book` with `{"resourceId": "<an id it answered>",
  "title": "Planning", "start": "2026-11-04T10:00:00", "end": "2026-11-04T11:00:00"}`.
