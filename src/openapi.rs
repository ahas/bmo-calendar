//! The OpenAPI document, `docs/openapi.json`: every route `docs/api.md` states, with its parameters, its body,
//! its answers and the codes it can give, the schemas generated from the service's own types, the events it
//! publishes, and the tool's entries with the input schemas the tool's folder holds.

use std::collections::BTreeMap;

use schemars::generate::{SchemaGenerator, SchemaSettings};
use schemars::{JsonSchema, Schema};
use serde_json::{Map, Value, json};

use crate::error::CODES;
use crate::events::KINDS;
use crate::wire::{
  AccountQuery, AlarmMoment, AlarmTold, AlarmsQuery, Beat, Booking, BookingChange, BookingChanged,
  BookingDecided, BookingsQuery, Calendar, CalendarChange, CalendarChanged, CancelQuery, Company, Decision,
  Event, EventChange, EventChanged, EventsQuery, FreeQuery, Health, NewBooking, NewCalendar, NewEvent,
  NewResource, Page, Reconciled, Resource, ResourceChange, ResourceChanged, ResourcesQuery, Schedule,
  ScheduleChanged, ScheduleSet, ScheduleWrite, Source, SourceChange, SourceChanged, SourceWrite, SpanQuery,
  StreamEvent, WireError,
};

const SCHEMAS: &str = "/components/schemas";

const DESCRIPTION: &str = "The platform's dated events, kept as memories in the brain service (the brain the \
  master, this unit the subordinate), in two realms kept apart: each account's own (mine) and the organization's, \
  one for each company. Calendars and events are read and written by realm; a repeating event answers one row per \
  occurrence in a span; an alarm is told once to the supervisor through the receiver when it comes due. A company's \
  resources are booked, two bookings of one resource never meeting, some only once a keeper approves. Bodies are JSON with camelCase \
  keys, times are epoch milliseconds unless a field says it is a local ISO datetime, lists are pages of items and \
  more, and every failure is the error shape with its status. The tool's entries, placed on the tool host, are \
  stated under x-tools.";

/// The tool's entries: name, description, the route it calls, its input file and the answer it carries.
pub const TOOLS: [(&str, &str, &str, &str); 10] = [
  ("calendar_list", "The calendars of a realm.", "GET /{realm}/calendars", "calendars"),
  (
    "calendar_events",
    "The events of a realm in a span, or the next n from now.",
    "GET /{realm}/events",
    "events",
  ),
  ("calendar_show", "One event.", "GET /{realm}/events/{eventId}", "event"),
  (
    "calendar_add",
    "Adds an event, with an alarm where the owner asks to be reminded.",
    "POST /{realm}/events",
    "event",
  ),
  (
    "calendar_change",
    "Changes the fields named of an event or one occurrence.",
    "PATCH /{realm}/events/{eventId}",
    "event",
  ),
  (
    "calendar_cancel",
    "Cancels an event or one occurrence.",
    "DELETE /{realm}/events/{eventId}",
    "none",
  ),
  (
    "calendar_resources",
    "The resources of the company: its rooms and whatever else it books.",
    "GET /{realm}/resources",
    "resources",
  ),
  (
    "calendar_free_resources",
    "The company's resources free for a span, with at least the seats asked.",
    "GET /{realm}/resources/free",
    "resources",
  ),
  (
    "calendar_book",
    "Books a resource for a span; on a resource that waits for approval the booking stands requested.",
    "POST /{realm}/bookings",
    "booking",
  ),
  (
    "calendar_cancel_booking",
    "Cancels a booking or one occurrence.",
    "DELETE /{realm}/bookings/{bookingId}",
    "none",
  ),
];

/// The input schema of a tool entry, as the tool's folder holds it.
pub fn tool_input(name: &str) -> Value {
  let text = match name {
    "calendar_list" => include_str!("../tools/calendar/calendar_list.json"),
    "calendar_events" => include_str!("../tools/calendar/calendar_events.json"),
    "calendar_show" => include_str!("../tools/calendar/calendar_show.json"),
    "calendar_add" => include_str!("../tools/calendar/calendar_add.json"),
    "calendar_change" => include_str!("../tools/calendar/calendar_change.json"),
    "calendar_cancel" => include_str!("../tools/calendar/calendar_cancel.json"),
    "calendar_resources" => include_str!("../tools/calendar/calendar_resources.json"),
    "calendar_free_resources" => include_str!("../tools/calendar/calendar_free_resources.json"),
    "calendar_book" => include_str!("../tools/calendar/calendar_book.json"),
    "calendar_cancel_booking" => include_str!("../tools/calendar/calendar_cancel_booking.json"),
    _ => "null",
  };
  serde_json::from_str(text).unwrap_or(Value::Null)
}

struct Route {
  method: &'static str,
  path: &'static str,
  id: &'static str,
  tag: &'static str,
  summary: &'static str,
  parameters: Vec<Value>,
  body: Option<Schema>,
  answers: Vec<(u16, Option<Value>)>,
  codes: Vec<&'static str>,
}

fn error_answer(error: &Schema, codes: &[&str]) -> Value {
  let description = codes
    .iter()
    .map(|code| {
      let meaning =
        CODES.iter().find(|(name, ..)| name == code).map(|(.., meaning)| *meaning).unwrap_or("");
      format!("`{code}`: {meaning}")
    })
    .collect::<Vec<_>>()
    .join("\n");
  json!({ "description": description, "content": { "application/json": { "schema": error } } })
}

impl Route {
  fn operation(self, error: &Schema) -> Value {
    let mut codes = vec!["unauthorized", "forbidden"];
    codes.extend(self.codes);
    codes.sort_by_key(|code| CODES.iter().position(|(name, ..)| name == code));
    codes.dedup();
    let mut statuses: BTreeMap<u16, Vec<&str>> = BTreeMap::new();
    for code in &codes {
      let status = CODES
        .iter()
        .find(|(name, ..)| name == code)
        .map(|(_, status, _)| *status)
        .unwrap_or(500);
      statuses.entry(status).or_default().push(code);
    }
    let mut responses = Map::new();
    for (status, content) in self.answers {
      let description = match status {
        201 => "Created.",
        204 => "Done; no content.",
        _ => "Answered.",
      };
      let mut answer = json!({ "description": description });
      if let Some(content) = content {
        answer["content"] = content;
      }
      responses.insert(status.to_string(), answer);
    }
    for (status, codes) in statuses {
      responses.insert(status.to_string(), error_answer(error, &codes));
    }
    let mut operation = json!({
      "operationId": self.id,
      "tags": [self.tag],
      "summary": self.summary,
      "responses": responses,
      "x-codes": codes,
    });
    if !self.parameters.is_empty() {
      operation["parameters"] = self.parameters.into();
    }
    if let Some(body) = self.body {
      operation["requestBody"] =
        json!({ "required": true, "content": { "application/json": { "schema": body } } });
    }
    operation
  }
}

fn content<T: JsonSchema>(g: &mut SchemaGenerator) -> Option<Value> {
  Some(json!({ "application/json": { "schema": g.subschema_for::<T>() } }))
}

fn path_parameter(g: &mut SchemaGenerator, name: &str, description: &str) -> Value {
  json!({
    "name": name,
    "in": "path",
    "required": true,
    "description": description,
    "schema": g.subschema_for::<String>(),
  })
}

fn realm(g: &mut SchemaGenerator) -> Value {
  let mut parameter = path_parameter(g, "realm", "The realm: mine or organization.");
  parameter["schema"] = json!({ "type": "string", "enum": ["mine", "organization"] });
  parameter
}

fn query<Q: JsonSchema>(g: &mut SchemaGenerator) -> Vec<Value> {
  let schema = Q::json_schema(g).to_value();
  let required: Vec<&str> = schema["required"]
    .as_array()
    .map(|names| names.iter().filter_map(Value::as_str).collect())
    .unwrap_or_default();
  let Some(properties) = schema["properties"].as_object() else {
    return Vec::new();
  };
  properties
    .iter()
    .map(|(name, property)| {
      let mut parameter = json!({
        "name": name,
        "in": "query",
        "required": required.contains(&name.as_str()),
        "schema": property,
      });
      if let Some(description) = property.get("description") {
        parameter["description"] = description.clone();
      }
      parameter
    })
    .collect()
}

fn with(mut first: Vec<Value>, rest: Vec<Value>) -> Vec<Value> {
  first.extend(rest);
  first
}

fn routes(g: &mut SchemaGenerator) -> Vec<Route> {
  let realm_and = |g: &mut SchemaGenerator, name: &str, description: &str| {
    vec![realm(g), path_parameter(g, name, description)]
  };
  vec![
    Route {
      method: "get",
      path: "/health",
      id: "readHealth",
      tag: "health",
      summary: "Whether the service has read the brain, and its version.",
      parameters: vec![],
      body: None,
      answers: vec![(200, content::<Health>(g))],
      codes: vec![],
    },
    Route {
      method: "get",
      path: "/{realm}/calendars",
      id: "listCalendars",
      tag: "calendars",
      summary: "The realm's calendars, the caller's hidden mark on each; one is made where the realm holds none and the caller writes in it.",
      parameters: with(vec![realm(g)], query::<AccountQuery>(g)),
      body: None,
      answers: vec![(200, content::<Page<Calendar>>(g))],
      codes: vec!["bad-request", "not-found", "no-account", "upstream-failed", "not-ready"],
    },
    Route {
      method: "post",
      path: "/{realm}/calendars",
      id: "createCalendar",
      tag: "calendars",
      summary: "Make a calendar.",
      parameters: with(vec![realm(g)], query::<AccountQuery>(g)),
      body: Some(g.subschema_for::<NewCalendar>()),
      answers: vec![(201, content::<Calendar>(g))],
      codes: vec![
        "bad-request",
        "not-a-writer",
        "not-found",
        "no-account",
        "no-company",
        "upstream-failed",
        "not-ready",
      ],
    },
    Route {
      method: "patch",
      path: "/{realm}/calendars/{calendarId}",
      id: "changeCalendar",
      tag: "calendars",
      summary: "Change a calendar's name, colour, writers or tell, or hide or show it for the caller.",
      parameters: with(realm_and(g, "calendarId", "The calendar's id."), query::<AccountQuery>(g)),
      body: Some(g.subschema_for::<CalendarChange>()),
      answers: vec![(200, content::<Calendar>(g))],
      codes: vec![
        "bad-request",
        "not-a-writer",
        "not-found",
        "no-account",
        "no-company",
        "upstream-failed",
        "not-ready",
      ],
    },
    Route {
      method: "delete",
      path: "/{realm}/calendars/{calendarId}",
      id: "retireCalendar",
      tag: "calendars",
      summary: "Retire a calendar and its events.",
      parameters: with(realm_and(g, "calendarId", "The calendar's id."), query::<AccountQuery>(g)),
      body: None,
      answers: vec![(204, None)],
      codes: vec![
        "bad-request",
        "not-a-writer",
        "not-found",
        "no-account",
        "no-company",
        "upstream-failed",
        "not-ready",
      ],
    },
    Route {
      method: "get",
      path: "/{realm}/events",
      id: "listEvents",
      tag: "events",
      summary: "The events overlapping a span of at most 92 days, a repeating event one row per occurrence.",
      parameters: with(vec![realm(g)], query::<SpanQuery>(g)),
      body: None,
      answers: vec![(200, content::<Page<Event>>(g))],
      codes: vec![
        "bad-request",
        "not-found",
        "no-account",
        "span-too-long",
        "upstream-failed",
        "not-ready",
      ],
    },
    Route {
      method: "post",
      path: "/{realm}/events",
      id: "createEvent",
      tag: "events",
      summary: "Write an event; a POST naming an id the realm holds writes over it and answers 200.",
      parameters: with(vec![realm(g)], query::<AccountQuery>(g)),
      body: Some(g.subschema_for::<NewEvent>()),
      answers: vec![(201, content::<Event>(g)), (200, content::<Event>(g))],
      codes: vec![
        "bad-request",
        "not-a-writer",
        "not-found",
        "no-account",
        "no-company",
        "end-before-start",
        "unknown-calendar",
        "upstream-failed",
        "not-ready",
      ],
    },
    Route {
      method: "get",
      path: "/{realm}/events/{eventId}",
      id: "readEvent",
      tag: "events",
      summary: "One event.",
      parameters: with(realm_and(g, "eventId", "The event's id."), query::<AccountQuery>(g)),
      body: None,
      answers: vec![(200, content::<Event>(g))],
      codes: vec!["bad-request", "not-found", "no-account", "upstream-failed", "not-ready"],
    },
    Route {
      method: "patch",
      path: "/{realm}/events/{eventId}",
      id: "changeEvent",
      tag: "events",
      summary: "Change the fields named; with an occurrence, that occurrence alone, which becomes an event of its own.",
      parameters: with(realm_and(g, "eventId", "The event's id."), query::<AccountQuery>(g)),
      body: Some(g.subschema_for::<EventChange>()),
      answers: vec![(200, content::<Event>(g))],
      codes: vec![
        "bad-request",
        "not-a-writer",
        "not-found",
        "no-account",
        "end-before-start",
        "unknown-calendar",
        "not-an-occurrence",
        "upstream-failed",
        "not-ready",
      ],
    },
    Route {
      method: "delete",
      path: "/{realm}/events/{eventId}",
      id: "cancelEvent",
      tag: "events",
      summary: "Cancel an event, or one occurrence of a repeating event.",
      parameters: with(realm_and(g, "eventId", "The event's id."), query::<CancelQuery>(g)),
      body: None,
      answers: vec![(204, None)],
      codes: vec![
        "bad-request",
        "not-a-writer",
        "not-found",
        "no-account",
        "not-an-occurrence",
        "upstream-failed",
        "not-ready",
      ],
    },
    Route {
      method: "put",
      path: "/sources/{sourceId}",
      id: "putSource",
      tag: "sources",
      summary: "Name and colour a source, every account's hidden mark kept. The service token alone.",
      parameters: vec![path_parameter(g, "sourceId", "The source's id.")],
      body: Some(g.subschema_for::<SourceWrite>()),
      answers: vec![(201, content::<Source>(g)), (200, content::<Source>(g))],
      codes: vec!["bad-request", "upstream-failed", "not-ready"],
    },
    Route {
      method: "put",
      path: "/sources/{sourceId}/schedules/{itemId}",
      id: "putSchedule",
      tag: "sources",
      summary: "Write or change one schedule of a source, the source made where none stands. The service token alone.",
      parameters: vec![
        path_parameter(g, "sourceId", "The source's id."),
        path_parameter(g, "itemId", "The item's id, the unit's own."),
      ],
      body: Some(g.subschema_for::<ScheduleWrite>()),
      answers: vec![(201, content::<Schedule>(g)), (200, content::<Schedule>(g))],
      codes: vec!["bad-request", "end-before-start", "upstream-failed", "not-ready"],
    },
    Route {
      method: "delete",
      path: "/sources/{sourceId}/schedules/{itemId}",
      id: "removeSchedule",
      tag: "sources",
      summary: "Remove one schedule of a source; one that does not stand is no failure. The service token alone.",
      parameters: vec![
        path_parameter(g, "sourceId", "The source's id."),
        path_parameter(g, "itemId", "The item's id, the unit's own."),
      ],
      body: None,
      answers: vec![(204, None)],
      codes: vec!["bad-request", "upstream-failed", "not-ready"],
    },
    Route {
      method: "put",
      path: "/sources/{sourceId}/schedules",
      id: "putScheduleSet",
      tag: "sources",
      summary: "A source's whole set: each item that differs written, each standing one the set does not name removed. The service token alone.",
      parameters: vec![path_parameter(g, "sourceId", "The source's id.")],
      body: Some(g.subschema_for::<ScheduleSet>()),
      answers: vec![(200, content::<Reconciled>(g))],
      codes: vec!["bad-request", "end-before-start", "upstream-failed", "not-ready"],
    },
    Route {
      method: "get",
      path: "/{realm}/sources",
      id: "listSources",
      tag: "sources",
      summary: "Every source, the caller's hidden mark for the realm on each.",
      parameters: with(vec![realm(g)], query::<AccountQuery>(g)),
      body: None,
      answers: vec![(200, content::<Page<Source>>(g))],
      codes: vec!["bad-request", "not-found", "no-account", "upstream-failed", "not-ready"],
    },
    Route {
      method: "patch",
      path: "/{realm}/sources/{sourceId}",
      id: "changeSource",
      tag: "sources",
      summary: "Hide or show a source's schedules in the realm for the caller's account.",
      parameters: with(realm_and(g, "sourceId", "The source's id."), query::<AccountQuery>(g)),
      body: Some(g.subschema_for::<SourceChange>()),
      answers: vec![(200, content::<Source>(g))],
      codes: vec!["bad-request", "not-found", "no-account", "upstream-failed", "not-ready"],
    },
    Route {
      method: "get",
      path: "/{realm}/schedules",
      id: "listSchedules",
      tag: "sources",
      summary: "The schedules overlapping a span of at most 92 days: every one in the organization realm, those assigned to the account in the mine realm.",
      parameters: with(vec![realm(g)], query::<SpanQuery>(g)),
      body: None,
      answers: vec![(200, content::<Page<Schedule>>(g))],
      codes: vec![
        "bad-request",
        "not-found",
        "no-account",
        "span-too-long",
        "upstream-failed",
        "not-ready",
      ],
    },
    Route {
      method: "get",
      path: "/{realm}/companies",
      id: "listCompanies",
      tag: "companies",
      summary: "The companies the caller reads, its own first; for the administrator every company and the platform's own last. The organization realm alone.",
      parameters: with(vec![realm(g)], query::<AccountQuery>(g)),
      body: None,
      answers: vec![(200, content::<Page<Company>>(g))],
      codes: vec!["bad-request", "not-found", "no-account", "upstream-failed", "not-ready"],
    },
    Route {
      method: "get",
      path: "/{realm}/resources",
      id: "listResources",
      tag: "bookings",
      summary: "The company's live resources, by kind, place and name. The organization realm alone.",
      parameters: with(vec![realm(g)], query::<ResourcesQuery>(g)),
      body: None,
      answers: vec![(200, content::<Page<Resource>>(g))],
      codes: vec!["bad-request", "not-found", "no-account", "upstream-failed", "not-ready"],
    },
    Route {
      method: "post",
      path: "/{realm}/resources",
      id: "createResource",
      tag: "bookings",
      summary: "Make a resource: the administrator's and a head of the company's.",
      parameters: with(vec![realm(g)], query::<AccountQuery>(g)),
      body: Some(g.subschema_for::<NewResource>()),
      answers: vec![(201, content::<Resource>(g)), (200, content::<Resource>(g))],
      codes: vec![
        "bad-request",
        "not-a-writer",
        "not-found",
        "no-account",
        "no-company",
        "not-in-company",
        "upstream-failed",
        "not-ready",
      ],
    },
    Route {
      method: "get",
      path: "/{realm}/resources/free",
      id: "listFreeResources",
      tag: "bookings",
      summary: "The resources with at least the seats asked that no standing booking meets over the span and its repeat, fewest seats first.",
      parameters: with(vec![realm(g)], query::<FreeQuery>(g)),
      body: None,
      answers: vec![(200, content::<Page<Resource>>(g))],
      codes: vec![
        "bad-request",
        "not-found",
        "no-account",
        "end-before-start",
        "upstream-failed",
        "not-ready",
      ],
    },
    Route {
      method: "patch",
      path: "/{realm}/resources/{resourceId}",
      id: "changeResource",
      tag: "bookings",
      summary: "Change the fields named of a resource.",
      parameters: with(realm_and(g, "resourceId", "The resource's id."), query::<AccountQuery>(g)),
      body: Some(g.subschema_for::<ResourceChange>()),
      answers: vec![(200, content::<Resource>(g))],
      codes: vec![
        "bad-request",
        "not-a-writer",
        "not-found",
        "no-account",
        "no-company",
        "not-in-company",
        "upstream-failed",
        "not-ready",
      ],
    },
    Route {
      method: "delete",
      path: "/{realm}/resources/{resourceId}",
      id: "retireResource",
      tag: "bookings",
      summary: "Retire a resource and cancel its standing bookings.",
      parameters: with(realm_and(g, "resourceId", "The resource's id."), query::<AccountQuery>(g)),
      body: None,
      answers: vec![(204, None)],
      codes: vec![
        "bad-request",
        "not-a-writer",
        "not-found",
        "no-account",
        "no-company",
        "upstream-failed",
        "not-ready",
      ],
    },
    Route {
      method: "get",
      path: "/{realm}/bookings",
      id: "listBookings",
      tag: "bookings",
      summary: "The bookings overlapping a span of at most 92 days, one row per occurrence: the company's standing ones in the organization realm, the account's own in the mine realm.",
      parameters: with(vec![realm(g)], query::<BookingsQuery>(g)),
      body: None,
      answers: vec![(200, content::<Page<Booking>>(g))],
      codes: vec![
        "bad-request",
        "not-found",
        "no-account",
        "span-too-long",
        "upstream-failed",
        "not-ready",
      ],
    },
    Route {
      method: "post",
      path: "/{realm}/bookings",
      id: "createBooking",
      tag: "bookings",
      summary: "Book a resource; a POST naming an id the company holds writes over it and answers 200.",
      parameters: with(vec![realm(g)], query::<AccountQuery>(g)),
      body: Some(g.subschema_for::<NewBooking>()),
      answers: vec![(201, content::<Booking>(g)), (200, content::<Booking>(g))],
      codes: vec![
        "bad-request",
        "not-a-writer",
        "not-found",
        "overlap",
        "no-account",
        "no-company",
        "unknown-resource",
        "end-before-start",
        "upstream-failed",
        "not-ready",
      ],
    },
    Route {
      method: "get",
      path: "/{realm}/bookings/requests",
      id: "listBookingRequests",
      tag: "bookings",
      summary: "The requested bookings of the resources the caller keeps, oldest request first.",
      parameters: with(vec![realm(g)], query::<AccountQuery>(g)),
      body: None,
      answers: vec![(200, content::<Page<Booking>>(g))],
      codes: vec!["bad-request", "not-found", "no-account", "upstream-failed", "not-ready"],
    },
    Route {
      method: "get",
      path: "/{realm}/bookings/{bookingId}",
      id: "readBooking",
      tag: "bookings",
      summary: "One booking.",
      parameters: with(realm_and(g, "bookingId", "The booking's id."), query::<AccountQuery>(g)),
      body: None,
      answers: vec![(200, content::<Booking>(g))],
      codes: vec!["bad-request", "not-found", "no-account", "upstream-failed", "not-ready"],
    },
    Route {
      method: "patch",
      path: "/{realm}/bookings/{bookingId}",
      id: "changeBooking",
      tag: "bookings",
      summary: "Change the fields named; with an occurrence, that occurrence alone, which becomes a booking of its own.",
      parameters: with(realm_and(g, "bookingId", "The booking's id."), query::<AccountQuery>(g)),
      body: Some(g.subschema_for::<BookingChange>()),
      answers: vec![(200, content::<Booking>(g))],
      codes: vec![
        "bad-request",
        "not-a-writer",
        "not-found",
        "overlap",
        "no-account",
        "unknown-resource",
        "end-before-start",
        "not-an-occurrence",
        "upstream-failed",
        "not-ready",
      ],
    },
    Route {
      method: "delete",
      path: "/{realm}/bookings/{bookingId}",
      id: "cancelBooking",
      tag: "bookings",
      summary: "Cancel a booking, or one occurrence of a repeating booking.",
      parameters: with(realm_and(g, "bookingId", "The booking's id."), query::<CancelQuery>(g)),
      body: None,
      answers: vec![(204, None)],
      codes: vec![
        "bad-request",
        "not-a-writer",
        "not-found",
        "no-account",
        "not-an-occurrence",
        "upstream-failed",
        "not-ready",
      ],
    },
    Route {
      method: "post",
      path: "/{realm}/bookings/{bookingId}/decision",
      id: "decideBooking",
      tag: "bookings",
      summary: "A keeper approves or refuses a requested booking; its booker is told.",
      parameters: with(realm_and(g, "bookingId", "The booking's id."), query::<AccountQuery>(g)),
      body: Some(g.subschema_for::<Decision>()),
      answers: vec![(200, content::<Booking>(g))],
      codes: vec![
        "bad-request",
        "not-a-keeper",
        "not-found",
        "not-requested",
        "no-account",
        "upstream-failed",
        "not-ready",
      ],
    },
    Route {
      method: "get",
      path: "/{realm}/alarms",
      id: "listAlarms",
      tag: "alarms",
      summary: "The alarm moments in a span of at most 92 days, each with whether it was told.",
      parameters: with(vec![realm(g)], query::<AlarmsQuery>(g)),
      body: None,
      answers: vec![(200, content::<Page<AlarmMoment>>(g))],
      codes: vec![
        "bad-request",
        "not-found",
        "no-account",
        "span-too-long",
        "upstream-failed",
        "not-ready",
      ],
    },
    Route {
      method: "post",
      path: "/beats",
      id: "runBeat",
      tag: "alarms",
      summary: "Run one beat now: tell every alarm due and every booking telling not yet made, once each. The service token alone.",
      parameters: vec![],
      body: None,
      answers: vec![(200, content::<Beat>(g))],
      codes: vec!["failed", "not-ready"],
    },
    Route {
      method: "get",
      path: "/events",
      id: "readEvents",
      tag: "events-stream",
      summary: "The events the calendar publishes, as a server-sent stream, or a page to Accept: application/json.",
      parameters: with(
        query::<EventsQuery>(g),
        vec![json!({
          "name": "Last-Event-ID",
          "in": "header",
          "required": false,
          "description": "The last event number seen, as a reconnecting stream sends it.",
          "schema": { "type": "integer", "format": "int64" },
        })],
      ),
      body: None,
      answers: vec![(
        200,
        Some(json!({
          "text/event-stream": { "schema": { "type": "string", "description": "Each event's data is a StreamEvent." } },
          "application/json": { "schema": g.subschema_for::<Page<StreamEvent>>() },
        })),
      )],
      codes: vec!["bad-request", "gone"],
    },
    Route {
      method: "get",
      path: "/console-manifest",
      id: "readConsoleManifest",
      tag: "console",
      summary: "The console face's manifest, by bmo-ui's console plugin contract, to the console's credential alone.",
      parameters: vec![],
      body: None,
      answers: vec![(200, Some(json!({ "application/json": { "schema": { "type": "object" } } })))],
      codes: vec![],
    },
    Route {
      method: "get",
      path: "/console/{file}",
      id: "readConsoleFile",
      tag: "console",
      summary: "A file of the console bundle at the path the manifest names: the words to the console's credential alone, the module, the stylesheet and every other file to it with the manifest's Owner-Role and Owner-Key.",
      parameters: vec![path_parameter(g, "file", "The file's name, as the manifest gives it.")],
      body: None,
      answers: vec![(200, Some(json!({ "*/*": { "schema": { "type": "string" } } })))],
      codes: vec!["not-found"],
    },
  ]
}

/// The OpenAPI document.
pub fn openapi() -> Value {
  let mut generator = SchemaSettings::draft2020_12()
    .for_serialize()
    .with(|settings| {
      settings.definitions_path = SCHEMAS.into();
      settings.meta_schema = None;
    })
    .into_generator();
  let g = &mut generator;
  let error = g.subschema_for::<WireError>();
  let mut tags: Vec<&str> = Vec::new();
  let mut paths = Map::new();
  for route in routes(g) {
    if !tags.contains(&route.tag) {
      tags.push(route.tag);
    }
    let (path, method) = (route.path, route.method);
    paths.entry(path).or_insert_with(|| json!({}))[method] = route.operation(&error);
  }
  let events = json!({
    "calendar.changed": g.subschema_for::<CalendarChanged>(),
    "event.changed": g.subschema_for::<EventChanged>(),
    "alarm.told": g.subschema_for::<AlarmTold>(),
    "source.changed": g.subschema_for::<SourceChanged>(),
    "schedule.changed": g.subschema_for::<ScheduleChanged>(),
    "resource.changed": g.subschema_for::<ResourceChanged>(),
    "booking.changed": g.subschema_for::<BookingChanged>(),
    "booking.decided": g.subschema_for::<BookingDecided>(),
  });
  let calendars = g.subschema_for::<Page<Calendar>>();
  let event_page = g.subschema_for::<Page<Event>>();
  let event = g.subschema_for::<Event>();
  let resources = g.subschema_for::<Page<Resource>>();
  let booking = g.subschema_for::<Booking>();
  let tools: Vec<Value> = TOOLS
    .iter()
    .map(|(name, description, route, answer)| {
      let output = match *answer {
        "calendars" => json!(calendars),
        "events" => json!(event_page),
        "event" => json!(event),
        "resources" => json!(resources),
        "booking" => json!(booking),
        _ => json!({ "type": "object", "properties": { "cancelled": { "type": "boolean" } } }),
      };
      json!({
        "name": name,
        "namespace": "bmo",
        "capabilities": ["room.tools"],
        "description": description,
        "route": route,
        "input": tool_input(name),
        "output": output,
      })
    })
    .collect();
  json!({
    "openapi": "3.1.0",
    "info": { "title": "bmo-calendar", "version": env!("CARGO_PKG_VERSION"), "description": DESCRIPTION },
    "servers": [{ "url": "/", "description": "the calendar service" }],
    "security": [{ "bearer": [] }],
    "x-layer": "downstream",
    "x-calls": ["bmo-brain", "bmo-receiver", "the account service", "bmo-agency"],
    "x-events": KINDS,
    "x-event-data": events,
    "tags": tags.iter().map(|tag| json!({ "name": tag })).collect::<Vec<_>>(),
    "paths": paths,
    "components": {
      "schemas": generator.take_definitions(true),
      "securitySchemes": {
        "bearer": {
          "type": "http",
          "scheme": "bearer",
          "description": "The service token, or the console's credential with Owner-Key and Owner-Role; the console bundle's routes take the console's credential alone.",
        },
      },
    },
    "x-tools": tools,
  })
}

/// The document as the repository keeps it: two-space indented JSON with a closing newline.
pub fn openapi_json() -> String {
  let mut text = serde_json::to_string_pretty(&openapi()).expect("a JSON value always renders");
  text.push('\n');
  text
}
