//! The validation state of a form's containers, kept from one activation to the next, which
//! decides when a `validationState` event fires (XFA 3.3 pages 396–397; ADR 2706).
//!
//! Page 396 fires the event for each field, exclusion group and subform when the object is first
//! initialized, and afterwards only when the object changes from valid to invalid or back, or the
//! reason it is invalid changes — from the null test to the script test, the page's example. So
//! the state is a set of tests per container, and the event fires where the set changed.
//!
//! **A container's state is its content's** (page 397): a container becomes invalid where any of
//! its content fails a validation, and valid where none of it currently fails one, which includes
//! the case where no test was evaluated. So a subform's set is the union of its own and of every
//! container inside it, and an exclusion group's of its own and its buttons'.
//!
//! **A test's verdict stands until the test runs again** (page 396): the null test runs only in a
//! pre-event, so a field a person clears stays valid until the next one finds it empty, and the
//! script test runs whenever content changes, where the dependency tracking reaches. This module
//! keeps what the last run of each test said of each container, and the activation that ran the
//! tests records their verdicts here.

use std::collections::BTreeMap;

use crate::xfa::template::{Class, Template};

/// One of the tests of page 381's table a container's content can fail, which page 396 calls the
/// reason for invalidity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Test {
    /// The null-content test (`nullTest`).
    Null,
    /// The datatype test: a value the field's content type cannot hold.
    Datatype,
    /// The format test (`formatTest`).
    Format,
    /// The script test (`scriptTest`).
    Script,
}

impl Test {
    /// The test's bit in a [`Failing`] set.
    fn bit(self) -> u8 {
        match self {
            Self::Null => 1,
            Self::Datatype => 2,
            Self::Format => 4,
            Self::Script => 8,
        }
    }
}

/// The tests a container's content fails, as a set: empty is valid.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Failing(u8);

/// What each container of a form fails, as the tests last run found it, and what its last
/// `validationState` event announced.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Validity {
    /// Each container's own failing tests, by template node; a container absent fails none.
    own: BTreeMap<usize, Failing>,
    /// The state of each field, exclusion group and subform when its `validationState` event last
    /// fired, those valid absent; `None` until the merge initializes the objects (page 396), so
    /// that no change is announced before the first state is.
    announced: Option<BTreeMap<usize, Failing>>,
}

impl Validity {
    /// Records that `test` failed or passed on the container `node`.
    pub(super) fn record(&mut self, node: usize, test: Test, failed: bool) {
        let entry = self.own.entry(node).or_default();
        if failed {
            entry.0 |= test.bit();
        } else {
            entry.0 &= !test.bit();
        }
        if entry.0 == 0 {
            self.own.remove(&node);
        }
    }

    /// The state of every field, exclusion group and subform: its own failing tests and those of
    /// every container inside it (page 397). The walk goes up from each failing container, at
    /// most as many steps as the template is deep, which `xml::MAX_DEPTH` bounds.
    fn states(&self, template: &Template) -> BTreeMap<usize, Failing> {
        let mut states: BTreeMap<usize, Failing> = BTreeMap::new();
        for (&node, failing) in &self.own {
            let mut at = Some(node);
            while let Some(here) = at {
                let Some(container) = template.get(here) else {
                    break;
                };
                if announces(container.class) {
                    states.entry(here).or_default().0 |= failing.0;
                }
                at = container.parent;
            }
        }
        states
    }

    /// The objects first initialized, every field, exclusion group and subform in document order,
    /// whose `validationState` events fire now (page 396), and their state held as announced.
    pub(super) fn initialize(&mut self, template: &Template) -> Vec<usize> {
        self.announced = Some(self.states(template));
        (0..template.container_count())
            .filter(|&node| template.get(node).is_some_and(|c| announces(c.class)))
            .collect()
    }

    /// The objects whose state changed since their `validationState` event last fired, in
    /// document order, their new state held as announced; empty before the merge's
    /// initialization, which announces every state first.
    pub(super) fn changed(&mut self, template: &Template) -> Vec<usize> {
        let states = self.states(template);
        let Some(announced) = self.announced.as_mut() else {
            return Vec::new();
        };
        let mut changed: Vec<usize> = states
            .iter()
            .filter(|&(node, state)| announced.get(node) != Some(state))
            .map(|(&node, _)| node)
            .chain(
                announced
                    .keys()
                    .filter(|node| !states.contains_key(node))
                    .copied(),
            )
            .collect();
        changed.sort_unstable();
        *announced = states;
        changed
    }
}

/// Whether page 396 fires the event on a container of `class`.
fn announces(class: Class) -> bool {
    matches!(class, Class::Field | Class::ExclGroup | Class::Subform)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::xfa::xml::parse;

    fn template() -> Template {
        let text = "<template><subform name=\"outer\"><field name=\"one\"/>\
                    <exclGroup name=\"group\"><field name=\"button\"/></exclGroup>\
                    </subform></template>";
        Template::read(&parse(text.as_bytes()).expect("well formed"), 0)
    }

    /// Page 397: a container is invalid where any of its content fails, and valid again where
    /// none does; page 396: a change of reason is a change of state.
    #[test]
    fn a_subform_follows_its_content_and_a_new_reason_is_a_change() {
        let template = template();
        let mut validity = Validity::default();
        validity.record(1, Test::Script, true);
        assert_eq!(validity.changed(&template), Vec::<usize>::new());
        assert_eq!(validity.initialize(&template), vec![0, 1, 2, 3]);
        assert_eq!(validity.changed(&template), Vec::<usize>::new());
        validity.record(1, Test::Script, false);
        validity.record(1, Test::Null, true);
        assert_eq!(validity.changed(&template), vec![0, 1]);
        validity.record(1, Test::Null, false);
        validity.record(3, Test::Null, true);
        assert_eq!(validity.changed(&template), vec![1, 2, 3]);
    }
}
