//! A refusal or a failure, answered in the platform's error shape (`docs/api.md`, 「Codes」).

use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};

use crate::wire::WireError;

/// Every code the service answers, each with its status and what it means.
pub const CODES: [(&str, u16, &str); 21] = [
  ("bad-request", 400, "the body or the query is not the shape the route takes"),
  ("unauthorized", 401, "the token is missing or wrong"),
  ("forbidden", 403, "the token is right but the caller may not do this"),
  ("not-a-writer", 403, "a write the caller may not make"),
  ("not-a-keeper", 403, "a decision by one who keeps no such resource"),
  ("no-route", 404, "no route answers this method and path"),
  ("not-found", 404, "nothing stands at the key the path names"),
  ("overlap", 409, "a booking that would meet another standing booking of its resource"),
  ("not-requested", 409, "a decision on a booking that is not requested"),
  ("gone", 410, "a resume from before the events the service keeps"),
  (
    "no-account",
    422,
    "a mine call names no account, or its holder's account cannot be found",
  ),
  ("no-company", 422, "an organization write with no company, named or its own"),
  ("not-in-company", 422, "a resource's keeper who stands in another company or none"),
  (
    "unknown-resource",
    422,
    "a booking written on a resource the company does not hold live",
  ),
  ("end-before-start", 422, "an end not after its start"),
  (
    "unknown-calendar",
    422,
    "an event written under a calendar the realm does not hold live",
  ),
  ("span-too-long", 422, "a span over 92 days, or one ending before it starts"),
  ("not-an-occurrence", 422, "an occurrence that is not one of the event's"),
  ("failed", 500, "the service failed"),
  (
    "upstream-failed",
    502,
    "the brain, the receiver, the account service or agency answered a failure",
  ),
  ("not-ready", 503, "the service has not read the brain and moved it into companies yet"),
];

/// A refusal or a failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApiError {
  pub status: u16,
  pub code: &'static str,
  pub error: String,
}

impl ApiError {
  fn of(code: &'static str, error: impl Into<String>) -> Self {
    let status = CODES
      .iter()
      .find(|(name, ..)| *name == code)
      .map(|(_, status, _)| *status)
      .unwrap_or(500);

    Self {
      status,
      code,
      error: error.into(),
    }
  }

  pub fn bad_request(error: impl Into<String>) -> Self {
    Self::of("bad-request", error)
  }

  pub fn unauthorized() -> Self {
    Self::of("unauthorized", "The bearer token is missing or wrong.")
  }

  pub fn forbidden(error: impl Into<String>) -> Self {
    Self::of("forbidden", error)
  }

  pub fn not_a_writer(error: impl Into<String>) -> Self {
    Self::of("not-a-writer", error)
  }

  pub fn no_route() -> Self {
    Self::of("no-route", "No route answers this method and path.")
  }

  pub fn not_found(error: impl Into<String>) -> Self {
    Self::of("not-found", error)
  }

  pub fn gone(error: impl Into<String>) -> Self {
    Self::of("gone", error)
  }

  pub fn no_account(error: impl Into<String>) -> Self {
    Self::of("no-account", error)
  }

  pub fn no_company() -> Self {
    Self::of("no-company", "The call names no company and the caller stands in none.")
  }

  pub fn not_a_keeper(error: impl Into<String>) -> Self {
    Self::of("not-a-keeper", error)
  }

  pub fn not_in_company(error: impl Into<String>) -> Self {
    Self::of("not-in-company", error)
  }

  pub fn unknown_resource(id: &str) -> Self {
    Self::of("unknown-resource", format!("The company holds no live resource {id}."))
  }

  pub fn overlap(error: impl Into<String>) -> Self {
    Self::of("overlap", error)
  }

  pub fn not_requested(id: &str) -> Self {
    Self::of("not-requested", format!("The booking {id} is not waiting for a keeper."))
  }

  pub fn end_before_start() -> Self {
    Self::of("end-before-start", "The end is not after the start.")
  }

  pub fn unknown_calendar(id: &str) -> Self {
    Self::of("unknown-calendar", format!("The realm holds no live calendar {id}."))
  }

  pub fn span_too_long() -> Self {
    Self::of("span-too-long", "The span is over 92 days, or ends before it starts.")
  }

  pub fn not_an_occurrence(occurrence: &str) -> Self {
    Self::of("not-an-occurrence", format!("{occurrence} is not an occurrence of the event."))
  }

  pub fn failed(error: impl Into<String>) -> Self {
    Self::of("failed", error)
  }

  pub fn upstream(error: impl Into<String>) -> Self {
    Self::of("upstream-failed", error)
  }

  pub fn not_ready(error: impl Into<String>) -> Self {
    Self::of("not-ready", error)
  }
}

impl IntoResponse for ApiError {
  fn into_response(self) -> Response {
    let status = StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let body = WireError {
      code: self.code.to_owned(),
      error: self.error,
    };
    let mut response = (status, axum::Json(body)).into_response();
    if status == StatusCode::SERVICE_UNAVAILABLE {
      response.headers_mut().insert(header::RETRY_AFTER, HeaderValue::from_static("2"));
    }
    response
  }
}
