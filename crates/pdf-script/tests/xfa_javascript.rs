//! An XFA form's JavaScript run against a host that answers for a small form (ADR 2545).
//!
//! The form is a `Receipt` subform holding `Units`, `Price` and `Total`, and a `Detail` subform
//! holding two `Row`s; every expected value is one the script computes from the values this host
//! states, or a sentence the object model is documented to refuse with (XFA 3.3 pages 106–107 and
//! 412–415).

#![cfg(feature = "engine")]
#![expect(
    clippy::expect_used,
    reason = "test code: an explanatory panic is the intended failure"
)]

use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use pdf_model::xfa::{Occurrence, Scalar, Step};
use pdf_script::Budget;
use pdf_script::xfa::{FormHost, FormRan, FormRealm, MAX_NODES_HELD, som};

/// Container numbers: 0 `Receipt`, 1 `Units`, 2 `Price`, 3 `Total`, 4 `Detail`, 5 and 6 its two
/// `Row`s.
const NAMES: [&str; 7] = ["Receipt", "Units", "Price", "Total", "Detail", "Row", "Row"];
/// Each container's parent.
const PARENTS: [Option<u32>; 7] = [None, Some(0), Some(0), Some(0), Some(0), Some(4), Some(4)];

/// A form held in this test, which counts what it was asked.
#[derive(Debug)]
struct Receipt {
    values: RefCell<Vec<Scalar>>,
    asked: RefCell<usize>,
    assigned: RefCell<Vec<(u32, Scalar)>>,
}

impl Receipt {
    fn new() -> Rc<Self> {
        Rc::new(Self {
            values: RefCell::new(vec![
                Scalar::Null,
                Scalar::Number(5.0),
                Scalar::Number(3.0),
                Scalar::Null,
                Scalar::Null,
                Scalar::Text("a".to_owned()),
                Scalar::Text("b".to_owned()),
            ]),
            asked: RefCell::new(0),
            assigned: RefCell::new(Vec::new()),
        })
    }

    /// Counts a question.
    fn ask(&self) {
        let mut asked = self.asked.borrow_mut();
        *asked = asked.saturating_add(1);
    }

    /// The children of `parent` called `name`, in document order.
    fn children(parent: Option<u32>, name: &str) -> Vec<u32> {
        (0u32..7)
            .filter(|&node| PARENTS[node as usize] == parent && NAMES[node as usize] == name)
            .collect()
    }
}

impl FormHost for Receipt {
    fn resolve(&self, from: Option<u32>, path: &[Step]) -> Result<Vec<u32>, String> {
        self.ask();
        let Some((first, rest)) = path.split_first() else {
            return Ok(Vec::new());
        };
        if first.name.starts_with('$') && first.name != "$" {
            return Err(format!(
                "the qualified reference {} (page 114) other than $",
                first.name
            ));
        }
        // The script's container is `Total`; a search from it finds its siblings, then the root.
        if first.name == "Many" {
            return Ok(match first.occurrence {
                Occurrence::At(n) => vec![u32::try_from(n).unwrap_or(0).saturating_add(1000)],
                _ => Vec::new(),
            });
        }
        let mut current = match (from, first.name.as_str()) {
            (_, "$") => vec![from.unwrap_or(3)],
            (Some(base), name) => Self::children(Some(base), name),
            (None, name) => {
                let siblings = Self::children(Some(0), name);
                if siblings.is_empty() {
                    Self::children(None, name)
                } else {
                    siblings
                }
            }
        };
        if first.name != "$" {
            current = select(&current, first.occurrence);
        }
        for step in rest {
            let next: Vec<u32> = current
                .iter()
                .flat_map(|&node| Self::children(Some(node), &step.name))
                .collect();
            current = select(&next, step.occurrence);
        }
        Ok(current)
    }

    fn value(&self, node: u32) -> Result<Scalar, String> {
        self.ask();
        Ok(self
            .values
            .borrow()
            .get(node as usize)
            .cloned()
            .unwrap_or(Scalar::Null))
    }

    fn assign(&self, node: u32, value: Scalar) -> Result<(), String> {
        self.ask();
        if !(1..=3).contains(&node) {
            return Err("the container takes no value".to_owned());
        }
        self.assigned.borrow_mut().push((node, value.clone()));
        if let Some(slot) = self.values.borrow_mut().get_mut(node as usize) {
            *slot = value;
        }
        Ok(())
    }
}

/// The matches an occurrence selects; an omitted index takes the first.
fn select(matches: &[u32], occurrence: Occurrence) -> Vec<u32> {
    match occurrence {
        Occurrence::Every => matches.to_vec(),
        Occurrence::At(index) => usize::try_from(index)
            .ok()
            .and_then(|index| matches.get(index).copied())
            .into_iter()
            .collect(),
        Occurrence::Same | Occurrence::Relative(_) => {
            matches.first().copied().into_iter().collect()
        }
    }
}

/// Runs `script` as `Total`'s.
fn run(realm: &mut FormRealm, host: &Rc<Receipt>, script: &str) -> FormRan {
    let shared: Rc<dyn FormHost> = Rc::clone(host) as Rc<dyn FormHost>;
    realm.run(script, 3, shared, Budget::FIELD_EVENT.steps)
}

fn realm() -> FormRealm {
    FormRealm::new(Budget::FIELD_EVENT).expect("a realm is built")
}

/// Page 106's `rawValue`, page 413's naked references, and page 379's value of a calculation.
#[test]
fn a_calculation_multiplies_two_fields_by_their_naked_names() {
    let (mut realm, host) = (realm(), Receipt::new());
    let ran = run(&mut realm, &host, "Units.rawValue * Price.rawValue");
    assert_eq!(ran.outcome, Ok(Scalar::Number(15.0)));
    let ran = run(
        &mut realm,
        &host,
        "this.rawValue = Receipt.Units.rawValue + 1;",
    );
    assert_eq!(ran.outcome, Ok(Scalar::Number(6.0)));
    assert_eq!(
        host.assigned.borrow().as_slice(),
        &[(3, Scalar::Number(6.0))]
    );
    let ran = run(
        &mut realm,
        &host,
        "if (Total.isNull) { 1 } else { Total.rawValue * 2 }",
    );
    assert_eq!(ran.outcome, Ok(Scalar::Number(12.0)));
}

/// Page 379's "evaluated as a function": a top level that returns is a function's body, and its
/// locals do not outlive it; a global of a script's own does (page 414).
#[test]
fn a_script_that_returns_is_a_function_and_a_global_persists() {
    let (mut realm, host) = (realm(), Receipt::new());
    let ran = run(&mut realm, &host, "var n = Units.rawValue; return n + 2;");
    assert_eq!(ran.outcome, Ok(Scalar::Number(7.0)));
    run(&mut realm, &host, "kept = 41;")
        .outcome
        .expect("a global is set");
    assert_eq!(
        run(&mut realm, &host, "kept + 1").outcome,
        Ok(Scalar::Number(42.0))
    );
    assert_eq!(
        run(&mut realm, &host, "undefined").outcome,
        Ok(Scalar::Null)
    );
    assert_eq!(
        run(&mut realm, &host, "true").outcome,
        Ok(Scalar::Number(1.0))
    );
}

/// Page 107: `resolveNode` and `resolveNodes` take an expression in a string, on `xfa` and on a
/// container.
#[test]
fn resolve_node_and_resolve_nodes_take_a_som_expression() {
    let (mut realm, host) = (realm(), Receipt::new());
    let ran = run(
        &mut realm,
        &host,
        "xfa.resolveNode(\"Detail.Row[1]\").rawValue",
    );
    assert_eq!(ran.outcome, Ok(Scalar::Text("b".to_owned())));
    let ran = run(
        &mut realm,
        &host,
        "var rows = Detail.resolveNodes(\"Row[*]\"); rows.length + ':' + rows.item(0).rawValue + rows.item(1).rawValue + (rows.item(2) === null)",
    );
    assert_eq!(ran.outcome, Ok(Scalar::Text("2:abtrue".to_owned())));
    let ran = run(&mut realm, &host, "$xfa.resolveNode(\"Nothing\") === null");
    assert_eq!(ran.outcome, Ok(Scalar::Number(1.0)));
    let ran = run(&mut realm, &host, "xfa.resolveNode(\"Detail.Row[*]\")");
    let refused = ran.outcome.expect_err("two containers are not one");
    assert!(refused.contains("resolveNodes"), "{refused}");
}

/// Everything outside the subset is refused by name, a refusal the script caught included, and a
/// container is never assigned by its name (page 106).
#[test]
fn what_the_object_model_does_not_offer_is_refused_by_name() {
    let (mut realm, host) = (realm(), Receipt::new());
    for (script, named) in [
        ("xfa.host.messageBox('hello')", "xfa.host"),
        ("this.presence = 'hidden'", "presence"),
        ("Units.formattedValue", "formattedValue"),
        ("$form.Receipt.Units.rawValue", "$form"),
        ("Units = 4", "Units names a container"),
        ("try { xfa.event.newText } catch (e) {} 5", "xfa.event"),
        ("xfa.resolveNode('Receipt..Units')", "descendant"),
        ("Units + 1", "valueOf"),
    ] {
        let ran = run(&mut realm, &host, script);
        let refused = ran.outcome.expect_err(script);
        assert!(refused.contains(named), "{script}: {refused}");
    }
    assert!(
        host.assigned.borrow().is_empty(),
        "nothing refused was assigned"
    );
}

/// A script that never ends is stopped inside the realm, and reports itself past what it was
/// handed so that the activation it belongs to ends there (page 380); the realm runs the next one.
#[test]
fn a_script_that_never_ends_is_stopped_and_the_realm_goes_on() {
    let (mut realm, host) = (realm(), Receipt::new());
    for script in [
        "for (;;) {}",
        "while (true) { for (var i = 0; i < 90000; i++) {} }",
        "function f() { return f(); } f()",
        "'x'.repeat(1 << 30)",
    ] {
        let began = Instant::now();
        let ran = realm.run(script, 3, Rc::clone(&host) as Rc<dyn FormHost>, 1_000_000);
        assert!(ran.outcome.is_err(), "{script}");
        assert!(
            ran.steps > 1_000_000,
            "{script} reports itself over: {}",
            ran.steps
        );
        assert!(
            began.elapsed() < Duration::from_secs(5),
            "{script} took {:?}",
            began.elapsed()
        );
    }
    assert_eq!(
        run(&mut realm, &host, "Units.rawValue").outcome,
        Ok(Scalar::Number(5.0))
    );
}

/// A script that stays under every loop's limit and is handed every step there is is stopped by
/// the realm's wall clock, and says so.
#[test]
fn a_script_that_spends_no_limit_but_time_is_stopped_by_the_clock() {
    let host = Receipt::new();
    let mut realm = FormRealm::new(Budget {
        steps: u64::MAX,
        ..Budget::FIELD_EVENT
    })
    .expect("a realm is built");
    let began = Instant::now();
    let ran = realm.run(
        "for (var i = 0; i < 90000; i++) { for (var j = 0; j < 90000; j++) {} }",
        3,
        Rc::clone(&host) as Rc<dyn FormHost>,
        u64::MAX,
    );
    let stopped = ran.outcome.expect_err("the clock stops it");
    assert!(stopped.contains("wall-time budget of 100 ms"), "{stopped}");
    assert!(
        began.elapsed() < Duration::from_secs(2),
        "{:?}",
        began.elapsed()
    );
}

/// The containers one run may hold an object for are bounded before the object is made, and the
/// questions a script puts are one a name a run.
#[test]
fn the_containers_and_the_questions_of_a_run_are_bounded() {
    let (mut realm, host) = (realm(), Receipt::new());
    run(
        &mut realm,
        &host,
        "var s = 0; for (var i = 0; i < 1000; i++) { s += Units.rawValue; } s",
    )
    .outcome
    .expect("a loop over a naked reference runs");
    let asked = *host.asked.borrow();
    assert!(
        asked < 1100,
        "a name is resolved once a run and its value read each time: {asked}"
    );
    // A host of more containers than a run holds objects for: `Many[n]` is container 1000 + n.
    // The script's own container is one of those a run holds.
    let mut wide = FormRealm::new(Budget {
        wall: Duration::from_mins(1),
        ..Budget::FIELD_EVENT
    })
    .expect("a realm is built");
    let script = format!(
        "for (var i = 0; i <= {MAX_NODES_HELD}; i++) {{ xfa.resolveNode('Many[' + i + ']'); }} 1"
    );
    let ran = wide.run(&script, 3, Rc::clone(&host) as Rc<dyn FormHost>, u64::MAX);
    let refused = ran.outcome.expect_err("one container past the bound");
    assert!(
        refused.contains(&format!("more than {MAX_NODES_HELD} containers")),
        "{refused}"
    );
    let script = format!(
        "for (var i = 1; i < {MAX_NODES_HELD}; i++) {{ xfa.resolveNode('Many[' + i + ']'); }} 1"
    );
    let ran = wide.run(&script, 3, Rc::clone(&host) as Rc<dyn FormHost>, u64::MAX);
    assert_eq!(
        ran.outcome,
        Ok(Scalar::Number(1.0)),
        "at the bound, `this` included"
    );
}

/// The expression parser's bounds and refusals (pages 94–105).
#[test]
fn a_som_expression_in_a_string_is_read_within_its_bounds() {
    let steps = som("$.Detail.Row[+1]").expect("an expression");
    assert_eq!(steps.len(), 3);
    assert_eq!(steps[2].occurrence, Occurrence::Relative(1));
    assert_eq!(
        som("Row[-2]").expect("a step")[0].occurrence,
        Occurrence::Relative(-2)
    );
    assert!(som(&vec!["a"; 65].join(".")).is_err(), "65 names");
    assert!(som(&vec!["a"; 64].join(".")).is_ok(), "64 names");
    assert!(som(&"n".repeat(4097)).is_err(), "a name of 4097 bytes");
    for refused in [
        "a..b",
        "a.#field",
        "a.[x > 1]",
        "a\\.b",
        "a[",
        "a[x]",
        "a.",
        "",
    ] {
        assert!(som(refused).is_err(), "{refused}");
    }
}
