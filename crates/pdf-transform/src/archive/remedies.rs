//! The two configured remedies that reach a conversion: `derive` and `supply`.
//!
//! # What this file is
//!
//! [`super::config`] reads a configuration; [`super::decision`] answers requirements the converter
//! knows how to answer on its own. This file is the third thing: **an answer the *operator* gave**,
//! carried out over one document. Two kinds reach a conversion here, and they are the two the
//! owner's answers of 2026-09-12 opened:
//!
//! | kind | the owner's answer | what happens |
//! |---|---|---|
//! | `derive` | `A54` and `A55` | a declared program makes a new representation from content the document has |
//! | `supply` | `doc/pdf-a-mitigations.md` section 0.2 | the operator states a fact the document does not |
//!
//! # `derive` never runs anything here
//!
//! `A54`: [`crate::apply`] returns a [`ToolRequest`] and the **caller** runs it. So this file
//! builds requests out of the document and reads results the caller recorded; it opens no path,
//! reads no clock and starts no process. A pass with no recorded result for a request puts the
//! request in [`crate::Report::requested`] and leaves the requirement refused with
//! [`Because::AwaitingTool`] — which is not a failure, it is the first half of a two-pass
//! conversion the command-line program does in one command.
//!
//! # `derive` is impossible to get by accident
//!
//! `A55`, and the guardrails are in three places so that no one of them is the only one:
//!
//! - it is **never a default** — a site absent from the configuration is `stop`, and a site
//!   present without a `remedy` word is `stop`;
//! - it is **unreachable without the configuration naming the site *and* the tool** —
//!   [`super::ConfigError::DeriveWithoutTool`] refuses a site that names one and not the other, and
//!   [`super::Derivation`] cannot be constructed without a [`Tool`];
//! - what it produced is **reported per document in the words *this is derived, not original***
//!   ([`DERIVED_NOT_ORIGINAL`]) and **recorded in the file's own `xmpMM:History`**
//!   ([`derived_history`]).

use std::collections::{BTreeMap, BTreeSet};

use pdf_archive::{Outcome, Target};
use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId, Stream};

use crate::tool::{Tool, ToolOutcome, ToolOutputs, ToolRequest, ToolResult};

use super::config::{Derivation, Supplied, Supply, Winner, specification_name};
use super::report::{Derived, DerivedOutcome, SuppliedFact};
use super::rewrite::Rewrite;
use super::sites::{SeparationUse, separation_uses};

/// The sentence `doc/questions/A55` requires a derived document's report to carry.
///
/// The owner's answer asks for it *in those words*: a derived artefact is "reported per document in
/// the words 'this is derived, not original'". A PDF/A file whose attachment has become a rendering
/// of itself is a *different document* from the one that went in; the converter may produce it, and
/// it may not pretend otherwise.
pub const DERIVED_NOT_ORIGINAL: &str = "this is derived, not original";

/// The sentence a `supply` remedy's report carries.
///
/// `doc/rfc/0007` section 5b.1's obligation, which is `supply`'s alone: nothing was lost, nothing
/// moved and nothing was computed — a person put their own knowledge into the file, and the report
/// and the file's own provenance both have to say so, because it is the one remedy *the converter*
/// cannot get wrong and *the operator* can.
pub const SUPPLIED_BY_THE_OPERATOR: &str = "this value is the operator's, not the document's";

/// Why a requirement whose tool has not been run yet is refused for this pass.
pub(super) const AWAITING_TOOL: &str = "a remedy in this configuration answers this requirement \
     with a program, and that program has not been run yet. `apply` returns the invocation as data \
     rather than starting a process (doc/questions/A54); the caller runs it through the shared \
     executor and applies again, which is what `quorra-transform archive` does in one command";

/// One `[tool.…]` invocation this conversion needs, and what came back where anything did.
pub(super) struct Remedies {
    /// Invocations the caller has to perform before the conversion can proceed.
    pub(super) pending: Vec<ToolRequest>,
    /// The replacement bytes for each embedded file stream a tool derived.
    pub(super) derived: BTreeMap<ObjectId, Object>,
    /// The requirements a derivation answered at every place they failed.
    pub(super) derive_sites: BTreeSet<&'static str>,
    /// The `/Subtype` the operator supplied for each embedded file stream.
    pub(super) supplied: BTreeMap<ObjectId, String>,
    /// The definition every `Separation` array naming one colourant is to agree on.
    pub(super) separations: BTreeMap<Vec<u8>, (Object, Object)>,
    /// The requirements a supplied fact answered, each with the rewrite that carries it out.
    pub(super) supply_sites: BTreeMap<&'static str, Rewrite>,
    /// What was derived, from what, and by which tool — `A55`'s per-document report.
    pub(super) derived_report: Vec<Derived>,
    /// What the operator stated, beside the requirement it answered.
    pub(super) supplied_report: Vec<SuppliedFact>,
}

impl Remedies {
    /// Nothing configured, which is every conversion until an operator's file says otherwise.
    pub(super) fn none() -> Self {
        Self {
            pending: Vec::new(),
            derived: BTreeMap::new(),
            derive_sites: BTreeSet::new(),
            supplied: BTreeMap::new(),
            separations: BTreeMap::new(),
            supply_sites: BTreeMap::new(),
            derived_report: Vec::new(),
            supplied_report: Vec::new(),
        }
    }

    /// What the configuration's `derive` and `supply` answers do to this document.
    ///
    /// `input` is stage one's validation, and the population every remedy runs over is **its
    /// findings** rather than a walk of this converter's: which embedded file failed which rule is
    /// `pdf_archive`'s reading, and a second walk here would be a second reading of ISO 19005.
    pub(super) fn of(
        document: &Document,
        input: &pdf_archive::Report,
        target: Target,
        derivations: &[Derivation],
        supplies: &[Supply],
        recorded: &ToolOutputs,
    ) -> Self {
        let mut out = Self::none();
        for derivation in derivations {
            out.derive(document, input, target, derivation, recorded);
        }
        for supply in supplies {
            out.supply(document, input, supply);
        }
        out
    }

    /// One `derive` site, over every place its requirement failed.
    fn derive(
        &mut self,
        document: &Document,
        input: &pdf_archive::Report,
        target: Target,
        derivation: &Derivation,
        recorded: &ToolOutputs,
    ) {
        let Some(judgement) = input
            .failures()
            .find(|judgement| judgement.id == derivation.site)
        else {
            // The requirement is met; there is nothing to derive.
            return;
        };
        let streams = embedded_streams(document, judgement);
        if streams.is_empty() {
            return;
        }
        let mut answered = true;
        for (at, name) in streams {
            let Some(stream) = document.get(at).as_stream().cloned() else {
                answered = false;
                continue;
            };
            let Some(bytes) = document.decoded_stream_data(&stream) else {
                // A stream this reader cannot decode is a fact about this program, and there is
                // nothing to hand a tool. The requirement keeps its own refusal.
                answered = false;
                continue;
            };
            let request = request_for(&derivation.tool, &derivation.site, target, at, &name, bytes);
            let Some(result) = recorded.get(&request.id) else {
                self.pending.push(request);
                answered = false;
                continue;
            };
            answered &= self.place(
                document,
                judgement.id,
                &derivation.tool,
                &stream,
                at,
                &name,
                result,
            );
        }
        if answered {
            self.derive_sites.insert(judgement.id);
        }
    }

    /// One recorded result, checked and turned into the stream that replaces the attachment.
    ///
    /// Returns whether this place is answered. `doc/rfc/0007` section 4.2 is the whole of the
    /// checking: **what comes back is not trusted**, so the media type the tool declared is held
    /// against what it produced, and whatever survives that still goes through `doc/adr/0947`'s
    /// third stage, which re-opens the assembled output and holds it to the same target.
    #[expect(
        clippy::too_many_arguments,
        reason = "every one is a different fact about the place this result answers — which \
                  requirement, which tool, which stream, where it is, what it is called — and a \
                  struct bundling them would be this function's arguments under another name"
    )]
    fn place(
        &mut self,
        document: &Document,
        site: &'static str,
        tool: &Tool,
        stream: &Stream,
        at: ObjectId,
        name: &str,
        result: &ToolResult,
    ) -> bool {
        let was = document
            .get_key(&stream.dict, "Subtype")
            .as_name()
            .and_then(|subtype| subtype.as_str().map(str::to_owned));
        let mut row = Derived {
            site,
            attachment: name.to_owned(),
            was,
            tool: tool.name.clone(),
            program: tool.program.display().to_string(),
            expects: tool.expects.clone(),
            digest: result.digest.clone(),
            bytes: result.output.len(),
            stderr: result.stderr.clone(),
            outcome: DerivedOutcome::Attached,
        };
        match &result.outcome {
            ToolOutcome::Produced => {}
            ToolOutcome::Declined => {
                row.outcome = DerivedOutcome::Declined;
                self.derived_report.push(row);
                return false;
            }
            ToolOutcome::Failed(sentence) => {
                row.outcome = DerivedOutcome::Failed(sentence.clone());
                self.derived_report.push(row);
                return false;
            }
        }
        if !produced_what_it_promised(&tool.expects, &result.output) {
            row.outcome = DerivedOutcome::NotWhatItPromised;
            self.derived_report.push(row);
            return false;
        }
        self.derived.insert(
            at,
            derived_stream(&stream.dict, &tool.expects, &result.output),
        );
        self.derived_report.push(row);
        true
    }

    /// One `supply` site, over every place its requirement failed.
    fn supply(&mut self, document: &Document, input: &pdf_archive::Report, supply: &Supply) {
        let Some(judgement) = input
            .failures()
            .find(|judgement| judgement.id == supply.site)
        else {
            return;
        };
        let by_extension = match &supply.fact {
            Supplied::MediaTypes { by_extension } => by_extension,
            Supplied::SeparationWinner(winner) => {
                self.agree_on_one_definition(document, judgement.id, *winner);
                return;
            }
        };
        let streams = embedded_streams(document, judgement);
        if streams.is_empty() {
            return;
        }
        let mut answered = true;
        for (at, name) in streams {
            let Some(media_type) = for_extension(&name, by_extension) else {
                // The format's `unlisted` key is the operator's answer to an attachment their own
                // table does not name, and `stop` is the only value built — the configuration
                // reader refuses any other, because dropping an attachment is a rewrite nobody has
                // written. So an attachment outside the table leaves its requirement refused,
                // which is what `stop` is, and a converter that guessed the type here would be
                // doing the one thing `supply` exists to avoid.
                answered = false;
                continue;
            };
            self.supplied.insert(at, media_type.clone());
            self.supplied_report.push(SuppliedFact {
                site: judgement.id,
                subject: name,
                value: media_type,
            });
        }
        if answered {
            self.supply_sites
                .insert(judgement.id, Rewrite::SuppliedMediaType);
        }
    }

    /// `graphics/separations-of-one-name-agree`, answered by the operator's choice of definition.
    ///
    /// ISO 19005-2 section 6.2.4.4 and ISO 19005-4 section 6.2.4.4 require every `Separation`
    /// array naming one colourant to state the same alternate space and the same tint transform.
    /// A file that states two has defined one ink twice, and ISO 32000-2 §8.6.6.4 makes the
    /// difference real on a screen: an additive device "never applies a process colourant
    /// directly; it always reverts to the alternate colour space", so the tint the page paints is
    /// whatever the transform in force says it is.
    ///
    /// **Which definition is right is not in the file**, which is why this is a `supply` and never
    /// a default. What the operator states is *which of the document's own definitions* the
    /// archive means; every byte written below is the producer's.
    fn agree_on_one_definition(&mut self, document: &Document, site: &'static str, winner: Winner) {
        let uses = separation_uses(document);
        let mut agreed: BTreeMap<Vec<u8>, (Object, Object)> = BTreeMap::new();
        for colourant in uses
            .iter()
            .map(|use_| use_.colourant.clone())
            .collect::<BTreeSet<Vec<u8>>>()
        {
            let of_this_ink: Vec<&SeparationUse> = uses
                .iter()
                .filter(|use_| use_.colourant == colourant)
                .collect();
            // One definition, or several that are all the same object by the reading the
            // requirement is failed on: nothing disagrees, so nothing is written.
            let Some(first) = of_this_ink.first() else {
                continue;
            };
            if of_this_ink
                .iter()
                .all(|use_| same_definition(document, first, use_))
            {
                continue;
            }
            let chosen = match winner {
                Winner::First => first,
                Winner::MostUsed => most_used(document, &of_this_ink),
            };
            agreed.insert(
                colourant.clone(),
                (chosen.alternate.clone(), chosen.transform.clone()),
            );
            self.supplied_report.push(SuppliedFact {
                site,
                subject: format!(
                    "the colourant {} is defined {} time(s) in this file, in {} different ways",
                    String::from_utf8_lossy(&colourant),
                    of_this_ink.len(),
                    distinct(document, &of_this_ink),
                ),
                value: format!(
                    "the {} of them, which every Separation array naming it now states",
                    winner.word()
                ),
            });
        }
        if agreed.is_empty() {
            // Every colourant agrees with itself, so the failure is at something this reading of
            // §8.6.6.4's array shape did not reach — an array inside a stream this program cannot
            // decode, or one past the depth bound. The requirement keeps its own refusal.
            return;
        }
        self.separations = agreed;
        self.supply_sites.insert(site, Rewrite::SeparationAgreed);
    }
}

/// Whether two uses of one colourant define it the same way.
fn same_definition(document: &Document, left: &SeparationUse, right: &SeparationUse) -> bool {
    pdf_archive::same_parameter(document, &left.alternate, &right.alternate)
        && pdf_archive::same_parameter(document, &left.transform, &right.transform)
}

/// How many distinct definitions one colourant has, by the requirement's own reading of sameness.
fn distinct(document: &Document, uses: &[&SeparationUse]) -> usize {
    let mut seen: Vec<&SeparationUse> = Vec::new();
    for use_ in uses {
        if !seen
            .iter()
            .any(|held| same_definition(document, held, use_))
        {
            seen.push(use_);
        }
    }
    seen.len()
}

/// The definition the most `Separation` arrays state, ties going to the first in object order.
///
/// **Counted by definitions written, not by marks painted**, which is what
/// [`Winner::MostUsed`] documents and what an operator has to know: the file says how many times
/// each definition appears and says nothing about how much of any page each one covers.
fn most_used<'a>(document: &Document, uses: &[&'a SeparationUse]) -> &'a SeparationUse {
    let mut best: Option<(&'a SeparationUse, usize)> = None;
    for candidate in uses {
        let count = uses
            .iter()
            .filter(|other| same_definition(document, candidate, other))
            .count();
        if best.is_none_or(|(_, held)| count > held) {
            best = Some((candidate, count));
        }
    }
    // `uses` is never empty at the one call site, which checked `first()` before choosing; the
    // fallback keeps that a local fact rather than a panic waiting for a second caller.
    best.map_or(uses[0], |(chosen, _)| chosen)
}

/// Every embedded file stream one judgement's findings name, with the file's own name.
///
/// A finding for an embedded file names the **file specification** object and the `/EF` key the
/// stream sits under, which is how `pdf_archive` records both rules this file answers. Resolving
/// the pair here rather than walking the document again is what keeps the population the
/// validator's.
fn embedded_streams(
    document: &Document,
    judgement: &pdf_archive::Judgement,
) -> Vec<(ObjectId, String)> {
    let Outcome::Failed { places, .. } = &judgement.outcome else {
        return Vec::new();
    };
    let mut out: Vec<(ObjectId, String)> = Vec::new();
    for finding in places {
        let Some(specification) = finding.place.object else {
            continue;
        };
        let object = document.get(specification);
        let Some(dict) = object.as_dict() else {
            continue;
        };
        let files = document.get_key(dict, "EF");
        let Some(files) = files.as_dict() else {
            continue;
        };
        let name = specification_name(document, dict);
        // The finding names the key where it knows it; where it does not, every key Table 43
        // admits is taken, which is the same population the requirement was failed over.
        let keys: Vec<&str> = match finding.place.name.as_deref() {
            Some(named) if files.get(named).is_some() => vec![named],
            _ => vec!["F", "UF", "DOS", "Mac", "Unix"],
        };
        for key in keys {
            let Some(at) = files.get(key).and_then(Object::as_reference) else {
                continue;
            };
            if out.iter().any(|(held, _)| *held == at) {
                continue;
            }
            out.push((at, name.clone()));
        }
    }
    out
}

/// The media type the operator's table gives a file name, matched on its extension.
///
/// The comparison is on the last full stop onwards and is case-insensitive, because a file name is
/// a producer's string rather than a token: `INVOICE.XML` and `invoice.xml` are the same
/// attachment to everyone but a byte comparison.
fn for_extension(name: &str, table: &[(String, String)]) -> Option<String> {
    let at = name.rfind('.')?;
    let extension = name.get(at..)?.to_ascii_lowercase();
    table
        .iter()
        .find(|(suffix, _)| suffix.to_ascii_lowercase() == extension)
        .map(|(_, media_type)| media_type.clone())
}

/// One invocation, built from the tool's declaration and this attachment's bytes.
///
/// `{site}` and `{target}` are substituted here because both are the configuration's and the
/// caller's own words; `{in}` and `{out}` are left for the executor, which is the only thing with a
/// directory. **Nothing from the document reaches `args`** — `doc/rfc/0007` section 4.1 — and the
/// bytes travel in [`ToolRequest::input`].
fn request_for(
    tool: &Tool,
    site: &str,
    target: Target,
    at: ObjectId,
    name: &str,
    bytes: std::sync::Arc<[u8]>,
) -> ToolRequest {
    let args = tool
        .args
        .iter()
        .map(|argument| {
            argument
                .replace("{site}", site)
                .replace("{target}", &target.to_string())
        })
        .collect();
    ToolRequest {
        id: format!("{site}/{}/{}", at.number, at.generation),
        site: site.to_owned(),
        tool: tool.name.clone(),
        program: tool.program.clone(),
        args,
        input: bytes,
        expects: tool.expects.clone(),
        bounds: tool.bounds,
        delivery: tool.delivery(),
        subject: name.to_owned(),
    }
}

/// Whether what came back is what the tool promised.
///
/// `doc/rfc/0007` section 4.2's rule, and this version can check exactly one media type, which is
/// the only one the built site admits: a derived embedded file has to be a PDF, so `application/pdf`
/// is checked by opening it. Any other declared type is checked no further here — the tool's word
/// is taken for the bytes' *type*, and stage three's re-validation is what the file as a whole is
/// still held to.
fn produced_what_it_promised(expects: &str, bytes: &[u8]) -> bool {
    if expects != "application/pdf" {
        return true;
    }
    Document::open(std::sync::Arc::<[u8]>::from(bytes)).is_ok()
}

/// The embedded file stream a derived artefact replaces the original with.
///
/// Four edits, each for a reason the base standard states:
///
/// - the **data** is the tool's, unfiltered, with `/Filter` and `/DecodeParms` removed: what those
///   described was the producer's encoding of bytes that are no longer there;
/// - **`/Subtype`** becomes the media type the tool declared, which §7.11.4.1's Table 44 makes the entry's
///   value and which is now true of the bytes;
/// - **`/Params` `/Size`** is restated where the specification stated one, because Table 45 makes it
///   "[t]he size of the uncompressed embedded file, in bytes" and the file's size has changed;
/// - **`/Params` `/CheckSum`** is removed: Table 45 makes it a checksum of the *uncompressed*
///   bytes, and the bytes it was taken over are not in the file any more. Recomputing it would be
///   writing this converter's digest where the producer's stood.
///
/// **The file's `/F` and `/UF` names are left exactly as the producer wrote them**, and that is a
/// choice with a cost. A derived attachment may now be a PDF under a name ending `.docx`, which is
/// misleading; renaming it would be this converter inventing a file name, which `doc/questions/A48`
/// puts on the far side of the line. The report and the file's own `xmpMM:History` say the
/// attachment is derived, which is the honest version of the same information.
fn derived_stream(from: &Dictionary, media_type: &str, bytes: &[u8]) -> Object {
    let mut dict = from.clone();
    dict.remove("Filter");
    dict.remove("DecodeParms");
    dict.insert(
        Name::new(&b"Subtype"[..]),
        Object::Name(Name::new(media_type.as_bytes())),
    );
    dict.insert(
        Name::new(&b"Length"[..]),
        Object::Integer(i64::try_from(bytes.len()).unwrap_or(i64::MAX)),
    );
    if let Some(Object::Dictionary(params)) = dict.get("Params") {
        let mut params = params.clone();
        params.remove("CheckSum");
        if params.get("Size").is_some() {
            params.insert(
                Name::new(&b"Size"[..]),
                Object::Integer(i64::try_from(bytes.len()).unwrap_or(i64::MAX)),
            );
        }
        dict.insert(Name::new(&b"Params"[..]), Object::Dictionary(params));
    }
    Object::Stream(std::sync::Arc::new(Stream {
        dict,
        data: bytes.to_vec().into(),
        decryption_failed: false,
    }))
}

/// The `xmpMM:History` parameters recording every derived artefact, where anything was derived.
///
/// `doc/questions/A55`: the fact that part of an archive is derived rather than original belongs in
/// the *file*, not only in a report somebody may not have kept. The sentence is
/// [`DERIVED_NOT_ORIGINAL`] verbatim, followed by what was derived, from what, and by which tool.
pub(super) fn derived_history(rows: &[Derived]) -> Option<String> {
    use std::fmt::Write as _;
    let mut attached = rows
        .iter()
        .filter(|row| row.outcome == DerivedOutcome::Attached)
        .peekable();
    attached.peek()?;
    let mut out = format!("{DERIVED_NOT_ORIGINAL}: ");
    for (index, row) in attached.enumerate() {
        if index > 0 {
            out.push_str("; ");
        }
        let _ = write!(
            out,
            "the embedded file {} was replaced by {} derived from it by the tool {} ({}), \
             SHA-256 {}",
            row.attachment, row.expects, row.tool, row.program, row.digest
        );
    }
    Some(out)
}

/// The `xmpMM:History` parameters recording every fact the operator supplied.
///
/// `doc/rfc/0007` section 5b.1: `supply` carries an obligation the other four kinds do not, and this
/// is half of it — the file itself records that a human, not the document, is the value's source.
pub(super) fn supplied_history(rows: &[SuppliedFact]) -> Option<String> {
    use std::fmt::Write as _;
    if rows.is_empty() {
        return None;
    }
    let mut out = format!("{SUPPLIED_BY_THE_OPERATOR}: ");
    for (index, row) in rows.iter().enumerate() {
        if index > 0 {
            out.push_str("; ");
        }
        let _ = write!(
            out,
            "the value {} was stated by the converting operator's configuration for {}, \
             answering {}",
            row.value, row.subject, row.site
        );
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_extension_is_matched_without_regard_to_case() {
        let table = vec![(".xml".to_owned(), "application/xml".to_owned())];
        assert_eq!(
            for_extension("INVOICE.XML", &table).as_deref(),
            Some("application/xml")
        );
        assert_eq!(for_extension("invoice.csv", &table), None);
        assert_eq!(for_extension("no-extension", &table), None);
    }

    #[test]
    fn a_tool_that_promised_a_pdf_and_returned_something_else_is_caught() {
        assert!(!produced_what_it_promised("application/pdf", b"not a pdf"));
        // A type this version cannot check is taken on the tool's word, and the output's own
        // verdict is what the file is still held to.
        assert!(produced_what_it_promised("text/csv", b"a,b,c"));
    }
}
