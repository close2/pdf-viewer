//! Fuzzes `pdf_formcalc`: a document's `FormCalc` script parsed and evaluated over a small form
//! under a budget (RFC 0011 section 5's step 6, ADR 2482).
//!
//! **The script is the producer's, every byte of it.** XFA 3.3 chapter 25's grammar is parsed by a
//! hand-written lexer and a recursive-descent parser, and evaluated by a tree walk with loops,
//! user-defined functions and string builders — each of them a way for a short input to cost a
//! great deal, so the target holds the evaluator to the properties its budgets promise and not
//! only to never panicking (overflow checks stay on in this profile).
//!
//! The input is the script, read lossily so that every input is one, then after a NUL byte, where
//! the input has one, the values of three fields: `A`, `B` and `C`, split on further NUL bytes,
//! each a number where it reads as one and text otherwise. The form also holds a repeated field
//! `Row`, so `Row[*]` and an index reach occurrences, and a field `Out` a script may assign.
//!
//! Three properties beyond not panicking:
//!
//! - **The budgets hold.** A run ends in a value or in one of [`pdf_formcalc::Stop`]'s reasons, a
//!   value's text is at most the budget's string bytes, and the two runs together take less than
//!   [`ESCAPED`] of wall time, so that a ceiling the evaluator did not enforce is found as a failure
//!   and not as libFuzzer's timeout.
//! - **A run is a function of its input.** Running the same script on the same form twice gives
//!   the same outcome and leaves the same fields.
//! - **A script that did not parse assigns nothing.**

#![no_main]
#![expect(
    clippy::panic,
    reason = "a fuzz target states its properties by failing: `panic!` is how a violated one reaches libFuzzer, and each message names the property"
)]

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use libfuzzer_sys::fuzz_target;
use pdf_formcalc::{Budget, Host, Occurrence, Segment, Stop, Value, run};

/// The most the two runs may take together: far above what the budget's steps cost even on a
/// machine busy with builds, so that only a ceiling the evaluator failed to enforce reaches it.
const ESCAPED: Duration = Duration::from_secs(5);

/// A small budget, so that a campaign runs many inputs and the ceilings are reached often.
const BUDGET: Budget = Budget {
    steps: 20_000,
    calls: 8,
    string_bytes: 1 << 16,
};

/// A form of named values; the key of an occurrence is `name[n]`.
#[derive(Clone, PartialEq)]
struct Form {
    fields: BTreeMap<String, Value>,
}

impl Host for Form {
    type Node = String;

    fn resolve(&self, path: &[Segment]) -> Result<Vec<String>, String> {
        let name = path
            .iter()
            .map(|segment| segment.name.as_str())
            .collect::<Vec<_>>()
            .join(".");
        let Some(last) = path.last() else {
            return Ok(Vec::new());
        };
        Ok(match last.occurrence {
            Occurrence::Every => self
                .fields
                .keys()
                .filter(|key| key.starts_with(&format!("{name}[")))
                .cloned()
                .collect(),
            Occurrence::At(at) => {
                let key = format!("{name}[{at}]");
                if self.fields.contains_key(&key) {
                    vec![key]
                } else {
                    Vec::new()
                }
            }
            _ if self.fields.contains_key(&name) => vec![name],
            _ => Vec::new(),
        })
    }

    fn value(&self, node: &String) -> Value {
        self.fields.get(node).cloned().unwrap_or(Value::Null)
    }

    fn assign(&mut self, node: &String, value: Value) -> Result<(), String> {
        self.fields.insert(node.clone(), value);
        Ok(())
    }
}

/// A field value as the input states it: a number where the text reads as one, else the text.
fn field(text: &str) -> Value {
    text.trim()
        .parse::<f64>()
        .ok()
        .filter(|n| n.is_finite())
        .map_or_else(|| Value::String(text.to_owned()), Value::Number)
}

fn form(values: &[&str]) -> Form {
    let mut fields = BTreeMap::new();
    for (name, text) in ["A", "B", "C"].into_iter().zip(values) {
        fields.insert(name.to_owned(), field(text));
    }
    fields.insert("Out".to_owned(), Value::Null);
    for (at, value) in [1.0, 2.0, 3.0].into_iter().enumerate() {
        fields.insert(format!("Row[{at}]"), Value::Number(value));
    }
    Form { fields }
}

fuzz_target!(|data: &[u8]| {
    let text = String::from_utf8_lossy(data);
    let mut parts = text.split('\0');
    let script = parts.next().unwrap_or("");
    let values: Vec<&str> = parts.collect();
    let mut first = form(&values);
    let started = Instant::now();
    let outcome = run(script, &mut first, BUDGET);
    let mut second = form(&values);
    let again = run(script, &mut second, BUDGET);
    assert!(
        started.elapsed() <= ESCAPED,
        "a run took {:?}, past the budgets' reach",
        started.elapsed()
    );
    assert!(
        outcome == again && first == second,
        "the same script on the same form gave two outcomes"
    );
    match &outcome {
        Ok(Value::String(text)) if text.len() > BUDGET.string_bytes => {
            panic!("a value of {} bytes passed the string budget", text.len());
        }
        Err(Stop::Parse(_)) if first != form(&values) => {
            panic!("a script that did not parse assigned a field");
        }
        _ => {}
    }
});
