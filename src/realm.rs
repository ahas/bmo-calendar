//! A realm as a brain is keyed (`docs/design.md`, 「Two realms, kept apart」): an account's own, a company's
//! organization realm, or the platform's own, which holds what belongs to no company.

use crate::model::{BOOKING_TYPE, CALENDAR_TYPE, EVENT_TYPE, RESOURCE_TYPE, SCHEDULE_TYPE, SOURCE_TYPE};
use crate::wire::Realm;

/// The word the platform's own organization realm is named by where a company is named.
pub const PLATFORM: &str = "platform";

/// A realm as a brain is keyed, or the organization realm of a caller that stands in no company.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RealmKey {
  /// An account's own calendars.
  Mine(String),
  /// One company's organization realm, by the company's unit id.
  Company(String),
  /// The platform's own organization realm.
  Platform,
  /// The organization realm of a call with no company: it holds nothing and takes no write.
  NoCompany,
}

impl RealmKey {
  /// The realm's word.
  pub fn realm(&self) -> Realm {
    match self {
      Self::Mine(_) => Realm::Mine,
      Self::Company(_) | Self::Platform | Self::NoCompany => Realm::Organization,
    }
  }

  /// Whether it is an organization realm.
  pub fn organization(&self) -> bool {
    self.realm() == Realm::Organization
  }

  /// The account whose realm it is, for a `mine` realm.
  pub fn account(&self) -> Option<&str> {
    match self {
      Self::Mine(account) => Some(account),
      _ => None,
    }
  }

  /// The company a row read in it names: the company's id, or `platform`; none in a `mine` realm.
  pub fn company(&self) -> Option<String> {
    match self {
      Self::Company(company) => Some(company.clone()),
      Self::Platform => Some(PLATFORM.to_owned()),
      Self::Mine(_) | Self::NoCompany => None,
    }
  }

  /// The realm a company's word names: `platform`, else that company's.
  pub fn of_company(company: &str) -> Self {
    if company == PLATFORM { Self::Platform } else { Self::Company(company.to_owned()) }
  }

  /// The brain's `accountKey`, none for a call with no company.
  pub fn account_key(&self) -> Option<String> {
    match self {
      Self::Mine(account) => Some(format!("account:{account}")),
      Self::Company(company) => Some(format!("company:{company}")),
      Self::Platform => Some("organization".to_owned()),
      Self::NoCompany => None,
    }
  }

  /// The memory types its brain holds: every realm its calendars and events; a company's also the schedules
  /// received for it, its resources and their bookings; the platform's the sources and the schedules that name
  /// no company (`docs/design.md`, 「What a calendar and an event are in the brain」).
  pub fn kinds(&self) -> &'static [&'static str] {
    match self {
      Self::Mine(_) => &[CALENDAR_TYPE, EVENT_TYPE],
      Self::Company(_) => &[CALENDAR_TYPE, EVENT_TYPE, SCHEDULE_TYPE, RESOURCE_TYPE, BOOKING_TYPE],
      Self::Platform => &[CALENDAR_TYPE, EVENT_TYPE, SOURCE_TYPE, SCHEDULE_TYPE],
      Self::NoCompany => &[],
    }
  }

  /// The realm a brain's `accountKey` names, where it names one.
  pub fn of_account_key(key: &str) -> Option<Self> {
    if key == "organization" {
      return Some(Self::Platform);
    }
    if let Some(company) = key.strip_prefix("company:").filter(|company| !company.is_empty()) {
      return Some(Self::Company(company.into()));
    }
    key
      .strip_prefix("account:")
      .filter(|account| !account.is_empty())
      .map(|account| Self::Mine(account.into()))
  }
}
