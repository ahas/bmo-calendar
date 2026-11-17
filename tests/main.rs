//! The crate's one test harness (bmo-chat's docs/conventions.md, 「Rust builds」): each file of tests a module
//! of it, sharing the process.

mod support;

mod alarms;
mod bookings;
mod calendars;
mod companies;
mod console;
mod events;
mod openapi;
mod overlap;
mod preview;
mod repeats;
mod repository;
mod schedules;
