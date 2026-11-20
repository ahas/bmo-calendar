//! An Outlook event's title read as the work-code form (`docs/design.md`, 「A person's Outlook day」): a work
//! code of seven digits and a letter, up to two more levels each after an underscore, a bracketed group, then the
//! task's words. The form is the company's own way of writing a title, not a general format.

use std::sync::LazyLock;

use regex::Regex;

use crate::wire::TitleParts;

/// The work-code form: levels one to five, six and seven, the group and the task, in that order of captures.
static WORK_CODE: LazyLock<Regex> = LazyLock::new(|| {
  Regex::new(
    r"^(\d)(\d{2})(\d{2})(\d{2})([A-Za-z])(?:_([^_ ]+))?(?:_([^_ ]+))?(?:\s*\[([^\]]+)\])?\s*(.*)$",
  )
  .unwrap_or_else(|why| unreachable!("the work-code form is a fixed pattern that reads: {why}"))
});

/// How many levels a work code can carry.
const LEVELS: usize = 7;
/// The capture of the group, after the levels.
const GROUP: usize = LEVELS + 1;
/// The capture of the task, after the group.
const TASK: usize = GROUP + 1;

/// A title's parts: its work code's levels, its group and its task, or the whole title as the task.
pub fn parts(title: &str) -> TitleParts {
  let Some(found) = WORK_CODE.captures(title) else {
    return TitleParts {
      levels: Vec::new(),
      group: None,
      task: title.to_owned(),
    };
  };
  let text = |index: usize| found.get(index).map(|part| part.as_str().to_owned());
  TitleParts {
    levels: (1..=LEVELS).map_while(text).collect(),
    group: text(GROUP),
    task: text(TASK).unwrap_or_default(),
  }
}
