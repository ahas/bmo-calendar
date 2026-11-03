# bmo-calendar

The calendar unit keeps the platform's dated events — appointments, deadlines, anything with a day and a
time — and tells the supervisor when one is due. It is a view and an alarm over the brain: **the brain is
the master and the calendar is the subordinate** (the owner's words: 「캘린더는 비모의 브레인 API를 사용한다.
주가 브레인. 종이 캘린더」). Every calendar and every event is a memory in the brain service; the calendar
unit keeps no truth of its own, only what can be rebuilt from the brain: a cache of what it last read and
the marks of which alarms it has already told.

## Two realms, kept apart

The owner's words: 「일정은 조직 공유가 가능했으면 함」 and 「내 일정과 조직 일정의 분리 필요」.

- **Mine (`mine`):** each console account's own calendars. Only that account reads and writes them; its
  alarms reach that person.
- **Organization (`organization`):** the organization's calendars. Every console account reads them;
  who may write and who is told are settings of each calendar (by default the administrator and the
  event's creator write; the creator is told).

The realms are two kinds of brain in the brain service, never one: an account's realm is the brain keyed
`accountKey = account:<Owner-Key>`, `agentKey = calendar`; the organization's realm is the brain keyed
`accountKey = organization`, `agentKey = calendar`. The calendar unit makes a brain the first time a realm
is written. A screen shows both realms on one grid and tells them apart by colour and a mark; a tool call
names the realm it reads or writes.

## What a calendar and an event are in the brain

- **A calendar** is a memory of type `calendar`: `key` `calendar:<id>`, `text` its name, labels `color`
  (one of the console's six: `primary`, `secondary`, `info`, `success`, `warning`, `error`), `realm`,
  `writers` (organization only: `administrator`, `creator` or `everyone`), `tell` (organization only:
  `creator`, `everyone` or `nobody`), `hidden` (the accounts that hid it from their grid, their Owner-Keys
  joined by commas, empty where none did), `creator` (the account that made it) and `createdAt` (epoch ms,
  the moment its first version was written).
- **An event** is a memory of type `event`: `key` `event:<id>`, `text` its title followed, after a blank
  line, by its description; `validFrom` its start (epoch ms); labels `calendar` (the calendar's id),
  `start` and `end` (local ISO `YYYY-MM-DDTHH:mm:ss`, the shape the console's template uses), `allDay`
  (`true`/`false`), `alarm` (`none`, or minutes before the start: `0`, `5`, `15`, `30`, `60`, `1440`),
  `repeat` (`none`, `daily`, `weekly`, `monthly`, `yearly`), `creator` (the account's Owner-Key), `realm`,
  `createdAt` (epoch ms, the moment its first version was written) and `skip` (the occurrences of a repeating
  event that no longer stand in it, each its local start, joined by commas, empty where none: one cancelled,
  or one changed on its own and so made an event of its own).
- **Local times** are the wall clock of the time zone the calendar unit's host keeps; `validFrom` is the
  start read in that zone. A daily or weekly event repeats on the same wall-clock time; a monthly or yearly
  one on the same day of the month, skipping a month that has no such day.
- A change is a new version of the memory (`POST /memories/{id}/versions`); a cancellation moves the
  memory to `done` with the reason; nothing is deleted. The calendar reads the brain by `type`, the realm
  and `order=validFrom`, pages through, and keeps what it read in its cache, which the brain's event
  stream (`memory.written`, `memory.moved`) refreshes.

## Alarms

An event with an alarm is told once, at the moment its alarm comes (start less the minutes). The calendar
unit beats once a minute and reads the due alarms from its cache; the moment it tells one it keeps the mark
`told:<event id>:<occurrence>` (per realm) so a restart tells nothing twice, and a late beat tells what came
due while it was down, once each, naming how late. Beside the marks the store keeps the moment of the last
beat, which is how far back a beat looks; an event written after its alarm's moment and before its start is
told at the next beat. The telling is the receiver's instruction route: an order for the
supervisor in the recipient's room — the owner's private room for the owner; for a member, the room numbered
as one of the senders the account service links to the member's account, where the receiver holds it
registered, and the owner's private room naming the member where it holds none — so the supervisor says it
in its own words (「비모가 알려준다」). A
repeating event's next occurrence is told when its moment comes; a cancelled event's pending alarm is told
no more.

## The tool

The chat supervisor and the owner's tools reach the calendar through the tool host, namespace `bmo`,
capability `room.tools`: list the events of a span, show one, add, change, cancel, and list the calendars,
each naming its realm. The supervisor sets an alarm when the owner asks to be reminded of an appointment.
Every holder of the tool acts for the owner: its `mine` realm is the owner's account, the account the
account service links to the owner's sender.

## The console

The console screen follows the Nuxt UI calendar template (day, week, month and year views, drag to move
and resize, a popover to edit, a quick natural-language entry, colour-coded calendars with visibility, a
mini calendar), stood as a console bundle under the sidebar entry 일정. The sidebar lists 내 일정 and 조직
일정 calendars apart; one grid shows both.
