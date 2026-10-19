//! The transfer functions ISO 19005's graphics states may not state, removed, and what each said.
//!
//! # Two answers, decided by what the file states
//!
//! ISO 19005-2 section 6.2.5 and ISO 19005-4 section 6.2.5 forbid a graphics state's `/TR`
//! entry. ISO 32000-2 §10.5 is drawn by this reader (`doc/todo/13`), so what removing the entry
//! costs is a fact about the file rather than about the requirement:
//!
//! - **Every `/TR` the file states is the name `Identity`.** §8.4.5's Table 57: "The name
//!   Identity may be used to represent the Identity function." The function in force is then the
//!   identity before and after every `gs`, and removing the entry moves no colour on this
//!   reader's device, whose initial transfer function §8.4.1's Table 52 leaves device dependent
//!   and which is the identity here — a statement, not a loss (`doc/adr/1952` section 3).
//! - **Any other value** is a function this reader applies, so removing it moves the colours a
//!   reader saw, and the removal is [`super::Loss::TransferFunction`], which the caller
//!   authorises. §10.5 recommends against the entry,
//!
//! > Because transfer functions produce device-dependent effects, a page description that is
//! > intended to be device-independent should not define a current transfer function in the
//! > graphics state, or define TransferFunction in any halftone dictionaries.
//!
//! and a file that defines one all the same may depend on it for what its page shows, on a
//! printer as on a screen: the corpus's one witness draws its logo as a near-black negative
//! without it. So neither shipped profile authorises the loss (`doc/adr/1964`).
//!
//! # What is kept when the function goes
//!
//! **The function itself is data** (`doc/pdf-a-mitigations.md` section 4.3), so the removal
//! writes it into the output's own `xmpMM:History` before it goes: each graphics state's `/TR`
//! as stated, and every object it reaches written in clause 7's syntax, a stream's data decoded
//! and in hexadecimal. A reader of the archive can rebuild what the producer specified though
//! the archive no longer applies it. The record is a condition of the removal: a packet that will
//! not take it withdraws the removal (`doc/adr/1964`).
//!
//! Baking the function into the colours instead would rewrite every colour operand, image sample
//! and shading drawn under the state — content, behind ADR 0816's fence — so it is not offered.
//!
//! # `/TR2`, which the same subclause admits only as `Default`
//!
//! §8.4.5's Table 57 makes `TR2` "[s]ame as TR except that the value may also be the name
//! Default , denoting the transfer function that was in effect at the start of the page", and has
//! it "take precedence" over a `TR` beside it. So a `TR2` other than `Default` is written
//! `Default` under the same two answers: the name `Identity` is the initial function on this
//! reader's device and the statement is free, and a function is the same loss, recorded the same
//! way before it goes (`doc/adr/1978` section 6).
//!
//! # A halftone's `TransferFunction`, which the same subclause admits only where required
//!
//! ISO 19005-2 and ISO 19005-4 section 6.2.5 admit the entry only where ISO 32000-2 requires it,
//! and §10.6.5.6 requires it only of a type 5 halftone's component for a nonprimary or nonstandard
//! colourant. §10.5's second bullet is drawn here as its first is — "[a] transfer function
//! specified in a halftone dictionary shall override the corresponding one specified by the current
//! transfer function parameter in the graphics state" — so the same two answers apply, with one
//! difference in the first (`doc/adr/2007`):
//!
//! - **`Identity` is free wherever it stands.** It overrides `/TR` with the identity for its
//!   component, and a conversion that completes has removed every `/TR` and written every `/TR2`
//!   `Default` — the identity on this reader's device — so the component is drawn through the
//!   identity before and after.
//! - **A function** is the loss `transfer-function`, kept in `xmpMM:History` before it goes.
//!
//! A component that omits a `TransferFunction` it is required to state is refused: what the
//! function would be is the producer's, and nothing in the file says it.

use std::collections::{BTreeSet, VecDeque};
use std::fmt::Write as _;

use pdf_archive::Outcome;
use pdf_syntax::Document;
use pdf_syntax::object::{Object, ObjectId};

use super::decision::Because;

/// The requirement this module answers, by the identifier the validator reports it under.
const SITE: &str = "graphics/no-transfer-function-in-a-graphics-state";

/// The `/TR2` requirement beside it, answered by the same two answers.
const SECOND: &str = "graphics/second-transfer-function-is-default";

/// The halftone requirement beside both, answered by the same two answers (`doc/adr/2007`).
const HALFTONE: &str = "graphics/halftone-transfer-function-only-where-required";

/// Why a halftone's transfer function is not removed.
const HALFTONE_NOT_REACHED: &str = "a halftone in this file states a TransferFunction where ISO \
     32000-2 10.6.5.6 requires none, and this conversion cannot reach every halftone that does: \
     the validator's list was shortened, or the halftone or the type 5 halftone holding it is \
     written inside another object or as a stream. Removing the entries it can reach would \
     leave the requirement failing at the others, so none is removed (doc/adr/2007)";

/// Why a component lacking the function it is required to state is not answered.
const HALFTONE_FUNCTION_MISSING: &str = "a type 5 halftone in this file has a component for a \
     nonstandard colourant that states no TransferFunction, which ISO 32000-2 10.6.5.6 requires \
     of it. What that function would be is the producer's, and nothing in the file states it, \
     so the requirement is refused (doc/adr/2007)";

/// Why a graphics state's transfer function is not removed at all.
const STATE_NOT_REACHED: &str = "a graphics state in this file states a transfer function \
     where this conversion cannot reach every graphics state that does: the validator's list of \
     them was shortened, or one is written inside another object rather than as an object of \
     its own. Removing the entries it can reach would leave the requirement failing at the \
     others, so none is removed";

/// Why a function whose data cannot be decoded is not removed.
const FUNCTION_NOT_DECODED: &str = "a transfer function in this file is a stream whose data this \
     tree cannot decode, so the statement of it the removal has to keep in the file's own \
     xmpMM:History cannot be written, and the function is not removed either (doc/adr/1964)";

/// Why a function past the record's bound is not removed.
const FUNCTION_PAST_THE_BOUND: &str = "the transfer functions this file states are larger than \
     the statement this conversion writes into xmpMM:History before removing them (16 384 bytes \
     of decoded data across 64 objects), so they are not removed rather than removed without \
     their record (doc/adr/1964)";

/// The decoded stream data the record holds, across every function it states.
///
/// A bound rather than a reading (principle 3): the record is text in the metadata packet, two
/// characters a byte. A 256-entry eight-bit sampled function, the shape the corpus's one witness
/// states three times, is 256 bytes, so this holds sixty-four of them.
const MOST_BYTES: usize = 16_384;

/// The objects the record states, across every graphics state.
const MOST_OBJECTS: usize = 64;

/// One graphics state or halftone whose transfer function this conversion removed, with what it
/// said.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemovedTransferFunction {
    /// The graphics state parameter dictionary, or the halftone, in the source's numbering.
    pub state: ObjectId,
    /// Its `/TR` or `/TransferFunction` as stated, then every object that reaches, in ISO 32000-2
    /// clause 7's syntax: `/TR 8 0 R, where 8 0 obj << … >>`, a stream's data decoded and in
    /// hexadecimal.
    pub statement: String,
}

/// The graphics states whose `/TR` goes, and which of them stated a function other than the
/// identity.
#[derive(Debug, Default)]
pub(super) struct TransferFunctions {
    /// The requirement's clause, as the validator cited it for the target asked for.
    pub(super) citation: String,
    /// Every graphics state stating `/TR`.
    pub(super) at: BTreeSet<ObjectId>,
    /// Every graphics state whose `/TR2` is written `Default`.
    pub(super) second: BTreeSet<ObjectId>,
    /// The ones whose `/TR` is not the name `Identity`, each with its statement.
    pub(super) stated: Vec<RemovedTransferFunction>,
    /// Every halftone whose `TransferFunction` goes.
    pub(super) halftones: BTreeSet<ObjectId>,
    /// The ones whose `TransferFunction` is not the name `Identity`, each with its statement.
    pub(super) halftones_stated: Vec<RemovedTransferFunction>,
}

impl TransferFunctions {
    /// Whether the removal moves no colour: every function the file states is the identity.
    pub(super) fn costs_nothing(&self) -> bool {
        self.stated.is_empty() && self.halftones_stated.is_empty()
    }

    /// Every removed function's record, the graphics states' then the halftones'.
    pub(super) fn removed(&self) -> Vec<RemovedTransferFunction> {
        self.stated
            .iter()
            .chain(&self.halftones_stated)
            .cloned()
            .collect()
    }

    /// The `xmpMM:History` parameters keeping every function the removal takes, where it takes one.
    pub(super) fn history(&self) -> Option<String> {
        if self.costs_nothing() {
            return None;
        }
        let mut out = format!(
            "{}: the transfer function stated by {} graphics state(s) removed, or a TR2 \
             written Default; ISO 32000-2 10.5's function each applied is kept here as data",
            self.citation,
            self.stated.len()
        );
        if !self.halftones_stated.is_empty() {
            let _ = write!(
                out,
                ", and by {} halftone(s) where ISO 32000-2 10.6.5.6 requires none",
                self.halftones_stated.len()
            );
        }
        for removed in &self.stated {
            let _ = write!(
                out,
                "; graphics state {} {} R: {}",
                removed.state.number, removed.state.generation, removed.statement
            );
        }
        for removed in &self.halftones_stated {
            let _ = write!(
                out,
                "; halftone {} {} R: {}",
                removed.state.number, removed.state.generation, removed.statement
            );
        }
        Some(out)
    }
}

/// Every graphics state stating `/TR`, and the statement of each one that is not the identity.
///
/// The findings are the whole population — every graphics state stating the key fails the
/// requirement — so a list the validator shortened leaves the site refused rather than answered
/// for the part of it this can see; so does a finding naming an object that is not itself the
/// graphics state, a dictionary written inside another object being one this rewrite does not
/// reach (`doc/adr/1952`).
pub(super) fn removal(
    document: &Document,
    input: &pdf_archive::Report,
) -> Result<TransferFunctions, Because> {
    let refused = Because::NotBuiltYet(STATE_NOT_REACHED);
    let mut out = TransferFunctions::default();
    let mut budget = Budget::default();
    for judgement in input.failures() {
        let key = match judgement.id {
            SITE => "TR",
            SECOND => "TR2",
            HALFTONE => {
                halftone_removal(document, judgement, &mut out, &mut budget)?;
                continue;
            }
            _ => continue,
        };
        let Outcome::Failed { places, total } = &judgement.outcome else {
            continue;
        };
        if places.len() != *total {
            return Err(refused);
        }
        if out.citation.is_empty() || key == "TR" {
            out.citation.clone_from(&judgement.citation);
        }
        for finding in places {
            let id = finding.place.object.ok_or(refused)?;
            let object = document.get(id);
            let state = object.as_dict().ok_or(refused)?;
            let value = state.get(key).ok_or(refused)?;
            if !matches!(value, Object::Name(name) if name.as_bytes() == b"Identity") {
                out.stated.push(RemovedTransferFunction {
                    state: id,
                    statement: statement(document, key, value, &mut budget)?,
                });
            }
            if key == "TR" {
                out.at.insert(id);
            } else {
                out.second.insert(id);
            }
        }
    }
    Ok(out)
}

/// The halftones the halftone requirement names, each one's `TransferFunction` to go.
///
/// A finding names a type 5 halftone and the colourant whose component states the entry, or a
/// graphics state whose current halftone does; either way the halftone that states it must be a
/// dictionary object of its own, which is what the rewrite reaches. A component that *omits* the
/// entry it is required to state is the other half of the requirement, and is refused.
fn halftone_removal(
    document: &Document,
    judgement: &pdf_archive::Judgement,
    out: &mut TransferFunctions,
    budget: &mut Budget,
) -> Result<(), Because> {
    let refused = Because::NotBuiltYet(HALFTONE_NOT_REACHED);
    let Outcome::Failed { places, total } = &judgement.outcome else {
        return Ok(());
    };
    if places.len() != *total {
        return Err(refused);
    }
    if out.citation.is_empty() {
        out.citation.clone_from(&judgement.citation);
    }
    for finding in places {
        let holder_id = finding.place.object.ok_or(refused)?;
        let holder = document.get(holder_id);
        let holder = holder.as_dict().ok_or(refused)?;
        let key = finding.place.name.as_deref().ok_or(refused)?;
        // The finding's key is `HT` on a graphics state, and a colourant's name on a type 5
        // halftone: in both, the value is the halftone that states the entry.
        let at = holder
            .get(key)
            .and_then(Object::as_reference)
            .ok_or(refused)?;
        let object = document.get(at);
        let halftone = object.as_dict().ok_or(refused)?;
        let Some(value) = halftone.get("TransferFunction") else {
            return Err(Because::NotBuiltYet(HALFTONE_FUNCTION_MISSING));
        };
        if !matches!(value, Object::Name(name) if name.as_bytes() == b"Identity") {
            out.halftones_stated.push(RemovedTransferFunction {
                state: at,
                statement: statement(document, "TransferFunction", value, budget)?,
            });
        }
        out.halftones.insert(at);
    }
    Ok(())
}

/// What the record has spent of its two bounds.
#[derive(Debug, Default)]
struct Budget {
    /// Decoded stream bytes written.
    bytes: usize,
    /// Objects written.
    objects: usize,
}

/// `/TR` — or `/TR2`, as `key` says — as the graphics state states it, then every object it
/// reaches, once each.
///
/// Breadth first from the value, so the record reads in the order a reader resolves it: a
/// stitching function's own entry before the functions it stitches. A reference already written
/// is not written twice, which is also what ends a cycle.
fn statement(
    document: &Document,
    key: &str,
    value: &Object,
    budget: &mut Budget,
) -> Result<String, Because> {
    let mut out = format!("/{key} {}", written(value));
    let mut queue = VecDeque::new();
    let mut seen = BTreeSet::new();
    references(value, &mut queue);
    while let Some(id) = queue.pop_front() {
        if !seen.insert(id) {
            continue;
        }
        budget.objects = budget.objects.saturating_add(1);
        if budget.objects > MOST_OBJECTS {
            return Err(Because::NotBuiltYet(FUNCTION_PAST_THE_BOUND));
        }
        let object = document.get(id);
        references(&object, &mut queue);
        let separator = if seen.len() == 1 { ", where" } else { "," };
        let _ = write!(out, "{separator} {} {} obj ", id.number, id.generation);
        if let Object::Stream(stream) = &object {
            let data = document
                .decoded_stream_data(stream)
                .ok_or(Because::NotBuiltYet(FUNCTION_NOT_DECODED))?;
            budget.bytes = budget.bytes.saturating_add(data.len());
            if budget.bytes > MOST_BYTES {
                return Err(Because::NotBuiltYet(FUNCTION_PAST_THE_BOUND));
            }
            // The entries that say how the data was stored rather than what it is: the data
            // below is decoded, so they would describe bytes the record does not hold.
            let mut dict = stream.dict.clone();
            for container in ["Length", "Filter", "DecodeParms", "DL"] {
                dict.remove(container);
            }
            out.push_str(&written(&Object::Dictionary(dict)));
            out.push_str(" decoded data <");
            for byte in data.iter() {
                let _ = write!(out, "{byte:02X}");
            }
            out.push('>');
        } else {
            out.push_str(&written(&object));
        }
    }
    Ok(out)
}

/// Every reference one value holds, without following any, appended to `queue`.
fn references(value: &Object, queue: &mut VecDeque<ObjectId>) {
    match value {
        Object::Reference(id) => queue.push_back(*id),
        Object::Array(items) => {
            for item in items {
                references(item, queue);
            }
        }
        Object::Dictionary(dict) => {
            for (_, item) in dict.iter() {
                references(item, queue);
            }
        }
        Object::Stream(stream) => {
            for (_, item) in stream.dict.iter() {
                references(item, queue);
            }
        }
        _ => {}
    }
}

/// One value in clause 7's syntax, as `pdf_syntax::write` puts it in a file.
///
/// That writer escapes every name and writes every string in hexadecimal, so its output is
/// ASCII; the lossy conversion is the type's, and replaces nothing.
fn written(value: &Object) -> String {
    let mut bytes = Vec::new();
    pdf_syntax::write::object(value, &mut bytes);
    String::from_utf8_lossy(&bytes).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_record_names_each_removed_state_and_what_its_function_said() {
        let functions = TransferFunctions {
            citation: "ISO 19005-2 section 6.2.5".to_owned(),
            at: BTreeSet::from([ObjectId::new(9, 0), ObjectId::new(10, 0)]),
            second: BTreeSet::new(),
            halftones: BTreeSet::new(),
            halftones_stated: Vec::new(),
            stated: vec![RemovedTransferFunction {
                state: ObjectId::new(10, 0),
                statement: "/TR 11 0 R, where 11 0 obj << /FunctionType 2 >>".to_owned(),
            }],
        };
        let history = functions
            .history()
            .expect("a function other than the identity is recorded");
        assert!(history.starts_with("ISO 19005-2 section 6.2.5: "));
        assert!(history.contains("by 1 graphics state(s)"));
        assert!(history.ends_with(
            "; graphics state 10 0 R: /TR 11 0 R, where 11 0 obj << /FunctionType 2 >>"
        ));
    }

    #[test]
    fn an_identity_only_removal_records_nothing() {
        let functions = TransferFunctions {
            citation: "ISO 19005-2 section 6.2.5".to_owned(),
            at: BTreeSet::from([ObjectId::new(9, 0)]),
            second: BTreeSet::new(),
            stated: Vec::new(),
            halftones: BTreeSet::from([ObjectId::new(12, 0)]),
            halftones_stated: Vec::new(),
        };
        assert!(functions.costs_nothing());
        assert_eq!(functions.history(), None);
    }
}
