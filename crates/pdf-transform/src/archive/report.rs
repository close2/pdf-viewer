//! What one conversion did, as a report rather than a log.
//!
//! `doc/adr/0927`: the owner's four permissions to write something a producer did not all carry one
//! condition, that **what was written is reported**, named per document rather than inferable from
//! a diff. [`Conversion`] is that report — what conformed already, what was decided and under which
//! clause, what was refused and why, what was lost with authorisation, and what holding the
//! *output* to the target found — and it reaches a caller through [`crate::Report::archive`]
//! whether or not a file was written.
//!
//! Nothing here changes a document or takes a decision. It words what the other three files did,
//! for a person and for `--json` alike.
use std::collections::BTreeSet;

use pdf_archive::{Judgement, MisusedProperty, Outcome, Target};

use crate::json::Value;

use super::actions::RemovedAction;
use super::decision::Decision;
use super::fonts::{RestatedFont, SubstitutedFont};
use super::prepare::{DestinationProfile, RemovedAnnotation, WrittenAppearance};
use super::signatures::SourceSignature;

/// What the conversion decided about the signatures the source carries, and what they were.
///
/// `doc/pdf-a-conversion-limits.md` section 3.6's report, and a field of its own rather than a
/// row of [`Conversion::decided`] because it is not the answer to a requirement: a rewrite
/// invalidates every signature whatever requirement asked for it, so the question is put by the
/// conversion itself and is put whenever a non-conforming source carries one. The decision is
/// the same kind of answer a requirement gets — [`Decision::Authorised`],
/// [`Decision::Unauthorised`] or [`Decision::Refused`] — and stops the conversion the same way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureDecision {
    /// What was decided: authorised, waiting for `--authorise signature-assertion`, or refused.
    pub decision: Decision,
    /// Each signature the source carries, named and verified over the source.
    ///
    /// Filled whatever the decision, so that a refused document's report still says what it
    /// carried — which is the half of section 3.6's report a person can act on.
    pub each: Vec<SourceSignature>,
    /// How many places the rewrite touched: each value removed, each permissions entry, the
    /// form's flag.
    pub changed: usize,
}

/// What one departure did to the conversion.
///
/// `doc/rfc/0007` section 4.7. A departure is named per requirement and carries a narrowing
/// predicate; whether it *applies* to a given document is a question about that document's
/// attachments, so a departure the caller named can either cover the file — the requirement is
/// tolerated and the output does not conform on purpose — or leave a file its predicate does not
/// admit in place, where the requirement stays refused as it would with no departure at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DepartureOutcome {
    /// The predicate covered the document; the requirement is departed from.
    Applied {
        /// Whether the output still claims the target (`--claim-conformance`, `A59`).
        claimed: bool,
    },
    /// An embedded file the predicate does not admit; the requirement is not departed from.
    LeftInPlace {
        /// The offending file's name.
        file: String,
        /// The media type it stated, or `None` where it stated none.
        media_type: Option<String>,
    },
}

/// One departure the caller's configuration named, and what it did to this document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Departed {
    /// The requirement identifier departed from.
    pub requirement: &'static str,
    /// Its clause, as cited for the target asked for.
    pub citation: String,
    /// The media types the departure admits.
    pub media_types: Vec<String>,
    /// The `/AFRelationship` values it admits, where it narrowed by relationship.
    pub relationships: Vec<String>,
    /// The operator's stated reason, copied verbatim.
    pub reason: String,
    /// What the departure did.
    pub outcome: DepartureOutcome,
}

/// The `xmpMM:History` parameters recording every applied departure, where any applied.
///
/// `doc/rfc/0007` section 4.7.3. `None` where nothing was departed from, so the packet gains no
/// entry. The string names the requirement, the predicate that narrowed the departure, and the
/// reason — everything a reader of the archive needs to know the file goes against the standard on
/// purpose and why.
#[must_use]
pub(super) fn departure_history(departures: &[Departed]) -> Option<String> {
    use std::fmt::Write as _;
    let mut applied = departures
        .iter()
        .filter(|departed| matches!(departed.outcome, DepartureOutcome::Applied { .. }))
        .peekable();
    applied.peek()?;
    let mut out = String::from(
        "this file departs from ISO 19005 on purpose, so it does not conform to the target: ",
    );
    for (index, departed) in applied.enumerate() {
        if index > 0 {
            out.push_str("; ");
        }
        let _ = write!(out, "{} accepted for", departed.requirement);
        if departed.media_types.is_empty() {
            out.push_str(" attachments of any media type");
        } else {
            let _ = write!(out, " {} attachments", departed.media_types.join(", "));
        }
        let _ = write!(out, " ({})", departed.reason);
    }
    Some(out)
}

/// What became of one attempt to derive an artefact with an external program.
///
/// `doc/questions/A55` and `doc/rfc/0007` section 4.1: a tool can produce a result, decline one, or
/// fail — and a fourth outcome is this converter's own, where what came back is not what the tool
/// promised (section 4.2, *what comes back is not trusted*).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DerivedOutcome {
    /// The tool produced what it promised, and the derived artefact is in the output.
    Attached,
    /// The tool exited with the declining status, which is *not this one* rather than a failure.
    Declined,
    /// The tool's declared media type is not what it returned, so nothing was used.
    NotWhatItPromised,
    /// The tool could not be run, ran and failed, overran its timeout or its output limit.
    Failed(String),
}

impl DerivedOutcome {
    /// A stable word for the report's machine-readable form.
    #[must_use]
    pub const fn word(&self) -> &'static str {
        match self {
            Self::Attached => "attached",
            Self::Declined => "declined",
            Self::NotWhatItPromised => "not-what-it-promised",
            Self::Failed(_) => "failed",
        }
    }
}

/// One artefact a `derive` remedy made, and everything `doc/questions/A55` asks be said about it.
///
/// The answer's condition, in its own words: *the report says, per document, what was derived, from
/// what, and by which tool*. The digest and the resolved program path are `doc/rfc/0007` section
/// 4.4's, so a re-run can be **checked** even though it cannot be **guaranteed**.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Derived {
    /// The requirement the derivation answered.
    pub site: &'static str,
    /// The embedded file's own name, as the source states it.
    pub attachment: String,
    /// The media type the source stated for it, where it stated one.
    pub was: Option<String>,
    /// The `[tool.…]` block's name.
    pub tool: String,
    /// The program as it was declared and, once run, as the operating system resolved it.
    pub program: String,
    /// The media type the tool promised.
    pub expects: String,
    /// The SHA-256 of what came back, lower-case hexadecimal.
    pub digest: String,
    /// How many bytes came back.
    pub bytes: usize,
    /// What the tool wrote to standard error, bounded. Section 4.1: a tool's own explanation of
    /// why it declined is the most useful thing a report can carry.
    pub stderr: String,
    /// What became of it.
    pub outcome: DerivedOutcome,
}

/// One stream whose data an operator's tool fetched from outside the document.
///
/// `doc/adr/1209`. The rewrite makes the bytes indistinguishable from any other stream's, so the
/// report and the file's own `xmpMM:History` are the only places a reader of the archive can
/// learn that they came from outside it. Every field is what that reader needs to check the
/// claim: what the document said its data was, which object holds it, which program was run, and
/// `doc/rfc/0007` section 4.4's digest of what came back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved {
    /// The requirement the fetch answered.
    pub site: &'static str,
    /// The file specification the document wrote, as a person reads it.
    pub names: String,
    /// The stream object, in the source's own numbering.
    pub object: String,
    /// The `[tool.…]` block's name.
    pub tool: String,
    /// The program as the operating system resolved it.
    pub program: String,
    /// How many bytes came back.
    pub bytes: usize,
    /// The SHA-256 of what came back, lower-case hexadecimal.
    pub digest: String,
    /// What the tool wrote to standard error, bounded.
    pub stderr: String,
    /// What became of it.
    pub outcome: DerivedOutcome,
}

impl Resolved {
    /// The row in the report's machine-readable form.
    #[must_use]
    pub fn to_json(&self) -> Value {
        Value::Object(vec![
            ("site".to_owned(), Value::text(self.site.to_owned())),
            ("names".to_owned(), Value::text(self.names.clone())),
            ("object".to_owned(), Value::text(self.object.clone())),
            ("tool".to_owned(), Value::text(self.tool.clone())),
            ("program".to_owned(), Value::text(self.program.clone())),
            (
                "bytes".to_owned(),
                Value::Integer(i64::try_from(self.bytes).unwrap_or(i64::MAX)),
            ),
            ("digest".to_owned(), Value::text(self.digest.clone())),
            ("stderr".to_owned(), Value::text(self.stderr.clone())),
            (
                "outcome".to_owned(),
                Value::text(self.outcome.word().to_owned()),
            ),
        ])
    }
}

/// One piece of the document's own content a `preserve` remedy kept on an appended page.
///
/// **`doc/adr/1014` section 5's fourth bullet, which is the condition every step onto that line
/// carries**: *a page was appended, carrying this, from there, placed so*. The three are
/// [`Self::pages`], [`Self::subject`] and [`Self::placement`], and together they are what makes the
/// page's content checkable against the input — the property a page composed of the document's own
/// content has and a watermark has not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preserved {
    /// The requirement whose refusal the remedy answered.
    pub site: &'static str,
    /// What was preserved, named as the document names it.
    pub subject: String,
    /// The zero-based pages of the output it now sits on.
    pub pages: Vec<usize>,
    /// How it was placed, in one sentence — every choice in it argued in `doc/adr/1025` section 4.
    pub placement: &'static str,
    /// The typeface the appended page is set in, where anything on it is set at all.
    ///
    /// **`doc/questions/A47`'s condition**, which the appended page inherits along with the
    /// permission: a face this program ships rather than one the document carried is named, so a
    /// reader can see that the letters on the page are this program's shapes and the words are the
    /// document's.
    pub face: SetIn,
    /// Why an appearance went onto an appended page rather than back onto the producer's own.
    ///
    /// **`doc/adr/1123`, the two honest costs made legible.** `None` for content relocated onto
    /// the producer's page and for a text packet, which was never on a page to relocate onto.
    /// `Some(sentence)` names the refusal that made the appended page the answer — a producer whose
    /// `q`/`Q` do not balance, or a remaining annotation over where the marks would land — so the
    /// report says which of the two constructions each preserved thing got.
    pub declined: Option<&'static str>,
}

/// What an appended page's content is set in.
///
/// Three states rather than an `Option`, because the third is not the absence of the second:
/// a page carrying an annotation's normal appearance sets no text at all, so no face was chosen
/// for it and none could have been named. `doc/adr/1099`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetIn {
    /// Nothing on the page is text: it carries marks the producer already drew.
    NoText,
    /// A face the document itself embeds, which puts nothing on the page from outside the file.
    ///
    /// What this prefers, and `doc/adr/1014`'s rule read strictly.
    TheDocumentsOwnFace,
    /// A face this program ships, named — `doc/questions/A47`'s permission and its condition.
    AFaceThisProgramShips(String),
}

impl SetIn {
    /// The clause the report adds after the placement sentence.
    fn sentence(&self) -> String {
        match self {
            Self::NoText => String::new(),
            Self::TheDocumentsOwnFace => ", in a face this document itself embeds".to_owned(),
            Self::AFaceThisProgramShips(face) => format!(
                ", in a face this document does not carry: {face}, embedded for it \
                 (doc/questions/A47)"
            ),
        }
    }

    /// The face's name as JSON, which is `null` for either face this program did not embed.
    fn to_json(&self) -> Value {
        match self {
            Self::NoText | Self::TheDocumentsOwnFace => Value::Null,
            Self::AFaceThisProgramShips(face) => Value::text(face.clone()),
        }
    }
}

impl Preserved {
    /// One preserved thing as JSON.
    fn to_json(&self) -> Value {
        Value::Object(vec![
            ("requirement".to_owned(), Value::text(self.site)),
            ("subject".to_owned(), Value::text(self.subject.clone())),
            (
                "pages".to_owned(),
                Value::Array(self.pages.iter().copied().map(Value::count).collect()),
            ),
            ("placement".to_owned(), Value::text(self.placement)),
            ("face".to_owned(), self.face.to_json()),
            (
                "declined_relocation".to_owned(),
                self.declined.map_or(Value::Null, Value::text),
            ),
            (
                "warns".to_owned(),
                Value::text(crate::archive::PRESERVED_AS_A_PAGE),
            ),
        ])
    }
}

/// One fact the operator stated that the document does not.
///
/// `doc/rfc/0007` section 5b.1's obligation: the report states the supplied value beside the
/// requirement it answered, because `supply` is the one remedy kind the converter cannot get wrong
/// and the person can.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuppliedFact {
    /// The requirement it answered.
    pub site: &'static str,
    /// What it is about — an embedded file's own name.
    pub subject: String,
    /// What the operator said it is.
    pub value: String,
}

impl Derived {
    /// One derived artefact as JSON.
    fn to_json(&self) -> Value {
        Value::Object(vec![
            ("requirement".to_owned(), Value::text(self.site)),
            (
                "attachment".to_owned(),
                Value::text(self.attachment.clone()),
            ),
            (
                "was".to_owned(),
                self.was.as_ref().map_or(Value::Null, Value::text),
            ),
            ("tool".to_owned(), Value::text(self.tool.clone())),
            ("program".to_owned(), Value::text(self.program.clone())),
            ("expects".to_owned(), Value::text(self.expects.clone())),
            ("digest".to_owned(), Value::text(self.digest.clone())),
            ("bytes".to_owned(), Value::count(self.bytes)),
            ("stderr".to_owned(), Value::text(self.stderr.clone())),
            ("outcome".to_owned(), Value::text(self.outcome.word())),
            (
                "warns".to_owned(),
                Value::text(crate::archive::DERIVED_NOT_ORIGINAL),
            ),
        ])
    }
}

impl SuppliedFact {
    /// One supplied fact as JSON.
    fn to_json(&self) -> Value {
        Value::Object(vec![
            ("requirement".to_owned(), Value::text(self.site)),
            ("subject".to_owned(), Value::text(self.subject.clone())),
            ("value".to_owned(), Value::text(self.value.clone())),
            (
                "warns".to_owned(),
                Value::text(crate::archive::SUPPLIED_BY_THE_OPERATOR),
            ),
        ])
    }
}

/// One requirement the input failed, with what was decided and what was done about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decided {
    /// The `pdf_archive` requirement identifier.
    pub requirement: &'static str,
    /// Its clause, as cited for the target that was asked for.
    pub citation: String,
    /// What it asks, in `pdf_archive`'s own words.
    pub asks: &'static str,
    /// How many places the input failed it, including any past the validator's own bound.
    pub places: usize,
    /// What the converter decided.
    pub decision: Decision,
    /// How many places the rewrite actually touched.
    ///
    /// Zero for a decision that performed none, and for a requirement the writer met by
    /// construction — there is no place to count when the whole file is the rewrite.
    pub changed: usize,
}

/// One requirement the validator did not check, carried into the conversion's report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotChecked {
    /// The requirement identifier.
    pub requirement: &'static str,
    /// Its clause.
    pub citation: String,
    /// Why it was not checked, in the validator's own words.
    ///
    /// **Never shortened and never softened.** `doc/questions/A20`: the reason *is* the report,
    /// and a conversion that trimmed it would be making its own verdict look better than the
    /// evidence for it.
    pub because: &'static str,
}

/// What holding the *output* to the target found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Achieved {
    /// Whether every requirement the validator checks was met by the output.
    pub conforms: bool,
    /// Every requirement the output still fails, by identifier.
    pub still_failing: Vec<&'static str>,
    /// Every requirement the *input* met and the output does not.
    ///
    /// A regression is what stops the file being written at all: a conversion that broke a
    /// requirement its source satisfied has not converted the document, it has damaged it.
    pub regressions: Vec<&'static str>,
    /// How many requirements the validator checked, of those that bind the target and are
    /// about a file at all.
    pub checked: usize,
}

/// What one conversion did, per document.
///
/// The report `doc/adr/0927` makes the condition of the owner's four permissions, and section 7 of
/// `doc/pdf-a-conversion-limits.md` the promise it keeps: what conformed already, what was
/// changed and under which clause, what was refused and why, what was lost with authorisation
/// — and the list of requirements the verdict is a verdict *over*.
// `Eq` is deliberately absent: `removed_boundaries` carries the rectangles a reader computed
// for a page boundary before and after its entry went, which are §7.9.5 real numbers, and a
// type holding one has no reflexive equality to claim (`doc/adr/1210`).
#[derive(Debug, Clone, PartialEq)]
pub struct Conversion {
    /// Which source.
    pub source: usize,
    /// The target asked for.
    pub target: Target,
    /// Every requirement the input already met, by identifier.
    pub conformed: Vec<&'static str>,
    /// Every requirement the input failed, with the decision taken about it.
    pub decided: Vec<Decided>,
    /// Every requirement the validator did not check.
    pub not_checked: Vec<NotChecked>,
    /// What the output was found to be, where one was written.
    ///
    /// `None` means no file was written: some decision refused, and [`Conversion::decided`]
    /// says which.
    pub achieved: Option<Achieved>,
    /// The destination profile the output intent this conversion added names.
    ///
    /// `None` where no output intent was added. `doc/questions/A18` makes this half of the
    /// report the condition on the permission: a conversion that adds an output intent has
    /// changed what every device colour in the file means to a conforming reader, and a user is
    /// entitled to be told which profile decided that and whose profile it is.
    ///
    /// **Named for either of the two rewrites that use it**: the output intent states it as its
    /// destination profile, and the `/DefaultCMYK` states the same object as the alternate space
    /// its tint transform's result is interpreted in.
    pub profile: Option<DestinationProfile>,
    /// What this conversion wrote into the document's own `xmpMM:History`, where it wrote
    /// anything.
    ///
    /// `doc/questions/A48`'s second condition, and the reason it is a field rather than a line of
    /// prose: the permission to write a `/DefaultCMYK` is conditional on the clause being named
    /// in the file's own provenance, so a report that could not say whether the entry was written
    /// could not say whether the permission had been honoured. `doc/adr/0927` has the argument.
    pub recorded: Option<String>,
    /// Every metadata property this conversion removed, with the value that was there.
    ///
    /// `doc/pdf-a-conversion-limits.md` section 3.9's condition on the loss, and the reason it is
    /// a list rather than a count: a property that is gone leaves nothing in the output to notice,
    /// so the report is the only place a user can read what the file used to say — and it names
    /// the namespace as well as the property, because two schemas may spell a local name the same
    /// way. With those three a user can put the property back by hand, or go back to the source
    /// and correct it there.
    pub removed: Vec<MisusedProperty>,
    /// Every annotation appearance this conversion constructed.
    ///
    /// `doc/questions/A21`'s condition on the permission, in the answer's own words: report every
    /// appearance written, so the difference between the producer's file and ours is visible in
    /// the report rather than only in the bytes.
    pub appearances: Vec<WrittenAppearance>,
    /// Every annotation this conversion removed for its subtype.
    ///
    /// `doc/pdf-a-conversion-limits.md` section 3.2's condition on the loss, in its own words:
    /// *remove the annotation and report it*, and *where the removed annotation had a normal
    /// appearance the page loses that mark, and the report says which page*. So each row names the
    /// page, the subtype and whether it drew anything — and where `remedy = "preserve"` answered
    /// the site, [`Self::preserved`] says which appended page the marks went to instead.
    pub removed_annotations: Vec<RemovedAnnotation>,
    /// Every stream whose data this file keeps outside itself, with what its `/F` names.
    ///
    /// ISO 19005-2 section 6.1.7.1 and ISO 19005-4 section 6.1.6.1 forbid the keys that put a
    /// stream's data elsewhere, and nothing in the document supplies what they point at — so this
    /// list is what a caller has to resolve before the conversion can embed anything, and what it
    /// resolved goes back in [`super::ArchivePlan::external_data`]. **The same shape as a tool
    /// request** (`doc/questions/A54`): a pass says what it needs, the caller performs the act,
    /// and the next pass is a pure function of what came back. Empty for every document whose
    /// streams keep their own bytes (`doc/adr/1199`).
    pub external_data: Vec<super::ExternalStream>,
    /// Every action, and every action-holding entry, this conversion removed.
    ///
    /// `doc/pdf-a-conversion-limits.md` section 3.3's condition on the loss it classes *Ask*: an
    /// action that is gone leaves nothing in the output to notice, so each row names the
    /// dictionary it was written in, the entry it was reached through and what it was. Empty for
    /// every conversion whose target admitted what the document held (`doc/adr/1175`).
    pub removed_actions: Vec<RemovedAction>,
    /// Every out-of-range page boundary entry this conversion removed.
    ///
    /// ISO 19005-2 section 6.1.13's limit, answered by ISO 32000-2 §7.7.3.3's Table 31 and
    /// §14.11.2.1's defaults. Each row names the page, the entry, and the rectangle a conforming
    /// reader computed for that boundary before and after — which is what says whether the
    /// removal cost anything, since §14.11.2.1 had already made an over-sized box its
    /// intersection with the media box. Empty for every conversion whose pages met the limit
    /// (`doc/adr/1210`).
    pub removed_boundaries: Vec<super::RemovedBoundary>,
    /// Every font this conversion embedded a face for, with what was asked for and what was used.
    ///
    /// `doc/pdf-a-conversion-limits.md` section 4.9's condition, in its own words: report per
    /// font, naming the face requested, the face used and which of the two metric routes was
    /// taken. `doc/questions/A47` makes substitution the default, and this list is what keeps
    /// that default honest — a reader of the output can see which of its typefaces are the
    /// producer's and which are this program's.
    pub substituted: Vec<SubstitutedFont>,
    /// Every embedded font program whose stated advances this conversion restated.
    ///
    /// Section 4.9's second metric route, listed for the same reason: nothing on the page moved,
    /// and all the same the bytes of somebody's font program are not the bytes they were.
    pub restated: Vec<RestatedFont>,
    /// The signatures the source carries and what was decided about them.
    ///
    /// `None` where the source carries none, or where it was copied rather than rewritten and
    /// so keeps every one it has.
    pub signatures: Option<SignatureDecision>,
    /// The departures the caller's configuration named, and what each did.
    ///
    /// `doc/rfc/0007` section 4.7. Empty for every conversion that departs from nothing, which is
    /// every conversion until an operator names one and it covers the document. A departure makes
    /// the output *not* conform on purpose, so the report says so in those words — the whole of
    /// what keeps a departed file from passing as a conforming one it is not.
    pub departures: Vec<Departed>,
    /// Every artefact a `derive` remedy made, with the tool that made it.
    ///
    /// **`doc/questions/A55`'s condition on the whole mode.** Empty for every conversion that
    /// derives nothing, which is every conversion until an operator's configuration names both a
    /// site and a tool. Where it is not empty the report says, in the answer's own words, that
    /// *this is derived, not original* — and the same sentence goes into the file's `xmpMM:History`,
    /// so the archive carries the fact rather than relying on a report nobody kept.
    pub derived: Vec<Derived>,
    /// Every stream whose data an operator's tool fetched from outside the document.
    ///
    /// `doc/adr/1209`. Empty for every conversion whose configuration names no fetching tool,
    /// which is every conversion until an operator's file does. The rewrite leaves the fetched
    /// bytes indistinguishable from any other stream's, so this list and the file's own
    /// `xmpMM:History` are where the archive says they came from outside it.
    pub resolved: Vec<Resolved>,
    /// Every fact the operator's configuration stated that the document does not.
    ///
    /// `doc/rfc/0007` section 5b.1. Empty until a configuration supplies one.
    pub supplied: Vec<SuppliedFact>,
    /// Everything a `preserve` remedy kept on a page appended to the document.
    ///
    /// **`doc/adr/1014`'s condition on the permission.** Empty for every conversion that appends
    /// nothing, which is every conversion until an operator's configuration names a `preserve`
    /// remedy whose site this document failed. Where it is not empty the report names the page and
    /// what it carries, and the same is recorded in the file's own `xmpMM:History`.
    pub preserved: Vec<Preserved>,
    /// What the source's encryption asserted about its reader, where the source carried one.
    ///
    /// **`doc/pdf-a-mitigations.md` section 2's `preserve` beside the `discard`**, and
    /// `doc/adr/1187`'s condition on the loss: no conforming output can be encrypted, so none can
    /// enforce ISO 32000-2 §7.6.4.2's Table 22 flags — what the producer claimed about printing,
    /// extraction and modification would otherwise leave the archive without a trace. `None` for a
    /// source that carried no `/Encrypt`, which is nearly every document.
    pub protection: Option<crate::archive::SourceProtection>,
}

impl Conversion {
    /// Whether every decision lets the conversion proceed.
    #[must_use]
    pub fn proceeds(&self) -> bool {
        self.decided
            .iter()
            .all(|decided| decided.decision.proceeds())
            && self
                .signatures
                .as_ref()
                .is_none_or(|signed| signed.decision.proceeds())
    }

    /// The report as RFC 0002 section 4.5's JSON.
    #[expect(
        clippy::too_many_lines,
        reason = "one entry per field of the report, in the order the rendered form states them.                   The entries that need a shape of their own are functions; what is left is a                   flat list, and splitting a flat list into two would put a reader of the schema                   in two places to read one object"
    )]
    #[must_use]
    pub fn to_json(&self) -> Value {
        Value::Object(vec![
            ("source".to_owned(), Value::count(self.source)),
            ("target".to_owned(), Value::text(self.target.to_string())),
            (
                "conformed".to_owned(),
                Value::Array(self.conformed.iter().map(|id| Value::text(*id)).collect()),
            ),
            (
                "decided".to_owned(),
                Value::Array(self.decided.iter().map(Decided::to_json).collect()),
            ),
            (
                "not_checked".to_owned(),
                Value::Array(self.not_checked.iter().map(NotChecked::to_json).collect()),
            ),
            (
                "achieved".to_owned(),
                self.achieved
                    .as_ref()
                    .map_or(Value::Null, Achieved::to_json),
            ),
            (
                "icc_profile".to_owned(),
                self.profile
                    .as_ref()
                    .map_or(Value::Null, DestinationProfile::to_json),
            ),
            (
                "recorded_in_xmp_history".to_owned(),
                self.recorded
                    .as_ref()
                    .map_or(Value::Null, |action| Value::text(action.clone())),
            ),
            (
                "removed_metadata_properties".to_owned(),
                Value::Array(self.removed.iter().map(removed_to_json).collect()),
            ),
            (
                "removed_annotations".to_owned(),
                Value::Array(
                    self.removed_annotations
                        .iter()
                        .map(RemovedAnnotation::to_json)
                        .collect(),
                ),
            ),
            (
                "external_stream_data".to_owned(),
                Value::Array(self.external_data.iter().map(external_to_json).collect()),
            ),
            (
                "removed_actions".to_owned(),
                Value::Array(
                    self.removed_actions
                        .iter()
                        .map(RemovedAction::to_json)
                        .collect(),
                ),
            ),
            (
                "resolved_external_data".to_owned(),
                Value::Array(self.resolved.iter().map(Resolved::to_json).collect()),
            ),
            (
                "removed_page_boundaries".to_owned(),
                Value::Array(
                    self.removed_boundaries
                        .iter()
                        .map(boundary_to_json)
                        .collect(),
                ),
            ),
            (
                "constructed_appearances".to_owned(),
                Value::Array(
                    self.appearances
                        .iter()
                        .map(WrittenAppearance::to_json)
                        .collect(),
                ),
            ),
            (
                "substituted_fonts".to_owned(),
                Value::Array(self.substituted.iter().map(substituted_to_json).collect()),
            ),
            (
                "restated_font_metrics".to_owned(),
                Value::Array(self.restated.iter().map(restated_to_json).collect()),
            ),
            (
                "signatures".to_owned(),
                self.signatures
                    .as_ref()
                    .map_or(Value::Null, SignatureDecision::to_json),
            ),
            (
                "departures".to_owned(),
                Value::Array(self.departures.iter().map(Departed::to_json).collect()),
            ),
            (
                "derived".to_owned(),
                Value::Array(self.derived.iter().map(Derived::to_json).collect()),
            ),
            (
                "supplied".to_owned(),
                Value::Array(self.supplied.iter().map(SuppliedFact::to_json).collect()),
            ),
            (
                "preserved".to_owned(),
                Value::Array(self.preserved.iter().map(Preserved::to_json).collect()),
            ),
            (
                "protection".to_owned(),
                protection_json(self.protection.as_ref()),
            ),
        ])
    }

    /// The report as text, for a person.
    ///
    /// The shape is the one section 7 of that document asks for: what the target was, what
    /// conformed, what was done and under which clause, what was refused and why — and the
    /// not-checked list last, never
    /// omitted, so that a reader who has learned to look for it can see it is empty rather than
    /// absent.
    #[must_use]
    #[expect(
        clippy::too_many_lines,
        reason = "one report, in the order a person reads it: what was already met, what was \
                  decided, the signatures, the profile, the provenance, what was written, the \
                  departures, the configured remedies, the verdict. The sections that carry a \
                  list of their own are functions; the spine is this, and a spine split in two \
                  would be a reader's problem rather than a writer's"
    )]
    pub fn render(&self) -> String {
        use std::fmt::Write as _;
        let mut out = String::new();
        let _ = writeln!(
            out,
            "{}: {} of the requirements this file was held to were already met",
            self.target,
            self.conformed.len()
        );
        // One rewrite can answer a dozen requirements — an output intent answers most of the
        // colour subclause at once — and its sentence about what the file now asserts is the same
        // sentence every time. Said once, so that a reader meets it rather than skims past it;
        // the JSON keeps it against each decision, where a machine wants it.
        let mut said: BTreeSet<&'static str> = BTreeSet::new();
        for decided in &self.decided {
            let repeated = match decided.decision {
                Decision::Stated { reinterprets, .. } => !said.insert(reinterprets),
                _ => false,
            };
            let _ = writeln!(
                out,
                "  {} ({}) — {} place(s)\n      {}",
                decided.requirement,
                decided.citation,
                decided.places,
                describe_decision(decided, repeated)
            );
        }
        if let Some(signed) = &self.signatures {
            let _ = writeln!(
                out,
                "  the source carries {} signature(s), and a conversion rewrites every byte a \
                 signature covered, so no output of it can carry them as signatures (ISO \
                 32000-2 \u{a7}12.8.1)\n      {}",
                signed.each.len(),
                describe(signed.decision, signed.changed, false)
            );
            for signature in &signed.each {
                let _ = writeln!(out, "      {}", describe_signature(signature));
            }
        }
        if let Some(profile) = &self.profile {
            let _ = writeln!(
                out,
                "  the ICC profile this conversion states is {}{}, over {}",
                profile.source.describe(),
                profile
                    .describes
                    .as_ref()
                    .map_or_else(String::new, |name| format!(" ({name})")),
                profile.space
            );
            if let Some(copyright) = &profile.copyright {
                let _ = writeln!(out, "      its copyright tag says: {copyright}");
            }
        }
        if let Some(protection) = &self.protection {
            let _ = writeln!(
                out,
                "  {} \u{2014} {}",
                crate::archive::PERMISSIONS_NO_LONGER_ASSERTED,
                protection.sentence()
            );
        }
        if let Some(recorded) = &self.recorded {
            let _ = writeln!(
                out,
                "  recorded in this file's own xmpMM:History: {recorded}"
            );
        }
        out.push_str(&self.render_what_was_written());
        out.push_str(&self.render_departures());
        out.push_str(&self.render_configured());
        if let Some(achieved) = &self.achieved {
            let verdict = if achieved.conforms {
                "conforms"
            } else {
                "does not conform"
            };
            let _ = writeln!(
                out,
                "  the output was held to {} again and {verdict}, over {} requirements",
                self.target, achieved.checked
            );
            for id in &achieved.still_failing {
                let sharper = if achieved.regressions.contains(id) {
                    " — which this document met, so the conversion broke it"
                } else {
                    ""
                };
                let _ = writeln!(out, "      still failed: {id}{sharper}");
            }
            if !achieved.conforms && !self.departed_output_stands() {
                let _ = writeln!(
                    out,
                    "  no file was written, because a conversion whose result is not {} has not \
                     converted the document",
                    self.target
                );
            } else if !achieved.conforms {
                let _ = writeln!(
                    out,
                    "  the file is written all the same: it departs from ISO 19005 on purpose \
                     (above), so every requirement it still fails is one the configuration named"
                );
            }
        } else {
            let _ = writeln!(out, "  no file was written");
        }
        let _ = writeln!(out, "  not checked ({}):", self.not_checked.len());
        for row in &self.not_checked {
            let _ = writeln!(out, "      {} ({})", row.requirement, row.citation);
        }
        out
    }
}

/// What the source's encryption asserted, as `doc/rfc/0002` section 4.5's JSON.
///
/// `null` where the source carried none, which is nearly every document.
fn protection_json(protection: Option<&crate::archive::SourceProtection>) -> Value {
    protection.map_or(Value::Null, |protection| {
        Value::Object(vec![
            ("handler".to_owned(), Value::text(&protection.handler)),
            (
                "revision".to_owned(),
                Value::text(protection.revision.to_string()),
            ),
            (
                "withheld".to_owned(),
                Value::Array(protection.withheld().into_iter().map(Value::text).collect()),
            ),
            ("says".to_owned(), Value::text(protection.sentence())),
        ])
    })
}

impl Conversion {
    /// What a `derive` remedy made and what a `supply` remedy stated, per document.
    ///
    /// **`doc/questions/A55`'s condition, in the answer's own words**: the report says *what was
    /// derived, from what, and by which tool*, and it says *this is derived, not original* rather
    /// than leaving a reader to infer it — a PDF/A file whose attachment has become something a
    /// program made is a different document from the one that went in. The digest and the resolved
    /// program are `doc/rfc/0007` section 4.4's, so a re-run can be checked.
    ///
    /// And `doc/rfc/0007` section 5b.1's, which is `supply`'s alone: the supplied value is stated
    /// beside the requirement it answered, because it is the one remedy the converter cannot get
    /// wrong and the person can.
    fn render_configured(&self) -> String {
        use std::fmt::Write as _;
        let mut out = String::new();
        if !self.derived.is_empty() {
            let _ = writeln!(
                out,
                "  {} artefact(s) a configured tool derived — {}:",
                self.derived.len(),
                crate::archive::DERIVED_NOT_ORIGINAL
            );
            for row in &self.derived {
                let _ = writeln!(
                    out,
                    "      {} ({}) {} by the tool {:?} ({}), answering {}",
                    row.attachment,
                    row.was.as_deref().unwrap_or("of no stated media type"),
                    match &row.outcome {
                        DerivedOutcome::Attached => format!(
                            "became {} of {} byte(s), SHA-256 {}",
                            row.expects, row.bytes, row.digest
                        ),
                        DerivedOutcome::Declined =>
                            "was declined by the tool, so it is unchanged and the requirement \
                             stays refused"
                                .to_owned(),
                        DerivedOutcome::NotWhatItPromised => format!(
                            "was not converted: what came back is not the {} the tool promised, \
                             and doc/rfc/0007 section 4.2 does not trust it",
                            row.expects
                        ),
                        DerivedOutcome::Failed(sentence) =>
                            format!("was not converted: {sentence}"),
                    },
                    row.tool,
                    row.program,
                    row.site
                );
                if !row.stderr.is_empty() {
                    let _ = writeln!(out, "          the tool said: {}", row.stderr.trim_end());
                }
            }
        }
        if !self.preserved.is_empty() {
            let _ = writeln!(
                out,
                "  {} thing(s) this conversion preserved rather than lost — {}:",
                self.preserved.len(),
                crate::archive::PRESERVED_AS_A_PAGE
            );
            for row in &self.preserved {
                let _ = writeln!(
                    out,
                    "      {} is on page(s) {}, answering {} — {}{}",
                    row.subject,
                    row.pages
                        .iter()
                        .map(|page| page.saturating_add(1).to_string())
                        .collect::<Vec<_>>()
                        .join(", "),
                    row.site,
                    row.placement,
                    row.face.sentence()
                );
                if let Some(declined) = row.declined {
                    let _ = writeln!(
                        out,
                        "          onto an appended page rather than the producer's own, because \
                         {declined}"
                    );
                }
            }
        }
        if !self.supplied.is_empty() {
            let _ = writeln!(
                out,
                "  {} value(s) this configuration supplied — {}:",
                self.supplied.len(),
                crate::archive::SUPPLIED_BY_THE_OPERATOR
            );
            for row in &self.supplied {
                let _ = writeln!(
                    out,
                    "      {} is stated to be {}, answering {}",
                    row.subject, row.value, row.site
                );
            }
        }
        out
    }

    /// The two lists the owner's conditions make part of the report rather than of a diff.
    ///
    /// `doc/questions/A21` asks for every appearance written and
    /// `doc/pdf-a-conversion-limits.md` section 3.9 for every property removed, each named rather
    /// than counted. They are one function because they are one obligation seen twice: what this
    /// program wrote that the producer did not, and what it took away that the producer did.
    fn render_what_was_written(&self) -> String {
        use std::fmt::Write as _;
        let mut out = String::new();
        if !self.removed_annotations.is_empty() {
            let drew = self
                .removed_annotations
                .iter()
                .filter(|removed| removed.appearance.is_some())
                .count();
            let _ = writeln!(
                out,
                "  {} annotation(s) removed for a subtype ISO 19005 does not admit, {drew} of \
                 which drew marks the page no longer carries:",
                self.removed_annotations.len()
            );
            for removed in &self.removed_annotations {
                let _ = writeln!(
                    out,
                    "      a {} annotation on page {}, which {}",
                    removed.subtype,
                    removed.page.saturating_add(1),
                    if removed.appearance.is_some() {
                        "had a normal appearance"
                    } else {
                        "drew nothing of its own"
                    }
                );
            }
        }
        if !self.appearances.is_empty() {
            let _ = writeln!(
                out,
                "  {} appearance stream(s) constructed, each from the entries the annotation's \
                 own subtype clause states:",
                self.appearances.len()
            );
            for appearance in &self.appearances {
                let _ = writeln!(
                    out,
                    "      a {} annotation on page {}",
                    appearance.subtype,
                    appearance.page.saturating_add(1)
                );
            }
        }
        if !self.substituted.is_empty() {
            let _ = writeln!(
                out,
                "  {} font(s) this file names and does not carry now embed a face this program \
                 ships; no glyph moved, because the widths the font dictionaries state were not \
                 touched:",
                self.substituted.len()
            );
            for font in &self.substituted {
                let _ = writeln!(
                    out,
                    "      {} (resource {}) is drawn from {} — {}",
                    font.requested,
                    font.resource,
                    font.face,
                    font.route.describe()
                );
            }
        }
        if !self.restated.is_empty() {
            let _ = writeln!(
                out,
                "  {} embedded font program(s) had their own stated advances restated to the \
                 widths their font dictionary already states, which is what ISO 32000-2 \
                 \u{a7}9.2.4 makes a reader position glyphs by:",
                self.restated.len()
            );
            for font in &self.restated {
                let _ = writeln!(
                    out,
                    "      {} (resource {}): {} glyph(s) restated across the page and {} down \
                     it, no outline changed",
                    font.requested, font.resource, font.glyphs, font.heights
                );
            }
        }
        if !self.removed.is_empty() {
            let _ = writeln!(
                out,
                "  {} metadata propert(ies) removed, because the predefined schema each names \
                 does not define the value it held:",
                self.removed.len()
            );
            for property in &self.removed {
                let _ = writeln!(
                    out,
                    "      {} in {} held {} — {}",
                    property.spelled, property.name.namespace, property.stated, property.because
                );
            }
        }
        out
    }

    /// The departures the configuration named, worded for a person.
    ///
    /// `doc/rfc/0007` section 4.7: reported per document, in the words that keep the difference
    /// between a departed file and a conforming one visible — a departed file is a PDF that meets
    /// the target in every respect but the ones listed, and does not claim to be PDF/A unless the
    /// operator demanded the claim with a second switch.
    fn render_departures(&self) -> String {
        use std::fmt::Write as _;
        let mut out = String::new();
        for departed in &self.departures {
            match &departed.outcome {
                DepartureOutcome::Applied { claimed } => {
                    let _ = writeln!(
                        out,
                        "  departed from {} ({}): the requirement is not enforced for {}",
                        departed.requirement,
                        departed.citation,
                        predicate(departed),
                    );
                    let _ = writeln!(out, "      because: {}", departed.reason);
                    let _ = writeln!(
                        out,
                        "      {}",
                        if *claimed {
                            "the output claims to be PDF/A anyway, on your instruction — a \
                             validator will fail it, and it stated the claim before it failed"
                        } else {
                            "the output does not claim to be PDF/A: its identification schema is \
                             omitted, so it meets the target in every respect but this one"
                        }
                    );
                }
                DepartureOutcome::LeftInPlace { file, media_type } => {
                    let described = media_type.as_deref().map_or_else(
                        || "no declared media type".to_owned(),
                        |it| format!("of {it}"),
                    );
                    let _ = writeln!(
                        out,
                        "  the departure from {} does not cover this document: {file:?} is {}, \
                         which the predicate {} does not admit, so the requirement stays refused",
                        departed.requirement,
                        described,
                        predicate(departed),
                    );
                }
            }
        }
        out
    }

    /// Whether a departed conversion's output was written, from the report alone.
    ///
    /// The same reckoning as `super::stands_as_departed`, made here so the report is self-contained:
    /// the file was written where every requirement it still fails is one an applied departure
    /// named, or — where the identification was omitted — one the omission costs.
    fn departed_output_stands(&self) -> bool {
        let departed: BTreeSet<&str> = self
            .departures
            .iter()
            .filter(|d| matches!(d.outcome, DepartureOutcome::Applied { .. }))
            .map(|d| d.requirement)
            .collect();
        if departed.is_empty() {
            return false;
        }
        let omit = self
            .departures
            .iter()
            .any(|d| matches!(d.outcome, DepartureOutcome::Applied { claimed: false }));
        self.achieved.as_ref().is_some_and(|achieved| {
            achieved.still_failing.iter().all(|id| {
                departed.contains(id)
                    || (omit && super::decision::IDENTIFICATION_CLAIM.contains(id))
            })
        })
    }
}

/// A departure's narrowing predicate, worded for a person.
fn predicate(departed: &Departed) -> String {
    let mut parts = Vec::new();
    if !departed.media_types.is_empty() {
        parts.push(format!(
            "attachments of {}",
            departed.media_types.join(", ")
        ));
    }
    if !departed.relationships.is_empty() {
        parts.push(format!(
            "relationship {}",
            departed.relationships.join(", ")
        ));
    }
    if parts.is_empty() {
        "attachments of any media type".to_owned()
    } else {
        parts.join(", ")
    }
}

/// One substituted font as JSON.
fn substituted_to_json(font: &SubstitutedFont) -> Value {
    Value::Object(vec![
        ("resource".to_owned(), Value::text(font.resource.clone())),
        ("requested".to_owned(), Value::text(font.requested.clone())),
        ("face".to_owned(), Value::text(font.face.clone())),
        (
            "face_authority".to_owned(),
            Value::text(font.authority.word()),
        ),
        ("metric_route".to_owned(), Value::text(font.route.word())),
    ])
}

/// One font whose program's advances were restated, as JSON.
fn restated_to_json(font: &RestatedFont) -> Value {
    Value::Object(vec![
        ("resource".to_owned(), Value::text(font.resource.clone())),
        ("base_font".to_owned(), Value::text(font.requested.clone())),
        ("glyphs".to_owned(), Value::count(font.glyphs)),
        ("vertical_glyphs".to_owned(), Value::count(font.heights)),
    ])
}

/// One removed property as JSON.
fn removed_to_json(property: &MisusedProperty) -> Value {
    Value::Object(vec![
        ("property".to_owned(), Value::text(property.spelled.clone())),
        (
            "namespace".to_owned(),
            Value::text(property.name.namespace.clone()),
        ),
        ("local".to_owned(), Value::text(property.name.local.clone())),
        ("stated".to_owned(), Value::text(property.stated.clone())),
        ("because".to_owned(), Value::text(property.because.clone())),
    ])
}

/// One stream whose data is outside the file: where it is, and where the file says the bytes are.
fn external_to_json(stream: &super::ExternalStream) -> Value {
    let (form, names) = match &stream.data {
        super::ExternalData::Named { shown, .. } => ("file-specification", shown.clone()),
        super::ExternalData::AtUrl(url) => ("url", url.clone()),
        super::ExternalData::NoneNamed => ("no-file-named", String::new()),
        super::ExternalData::Unreadable => ("unreadable", String::new()),
    };
    Value::Object(vec![
        (
            "object".to_owned(),
            Value::text(format!("{} {}", stream.at.number, stream.at.generation)),
        ),
        ("form".to_owned(), Value::text(form.to_owned())),
        ("names".to_owned(), Value::text(names)),
    ])
}

/// One removed page boundary entry, with what a reader computed for it before and after.
///
/// The two rectangles are what says whether the removal cost anything: equal means
/// §14.11.2.1 had already made the entry its intersection with the media box and no reader used
/// what it said (`doc/adr/1210`).
fn boundary_to_json(removed: &super::RemovedBoundary) -> Value {
    let rect = |sides: [f32; 4]| {
        Value::Array(
            sides
                .iter()
                .map(|side| Value::Number(f64::from(*side)))
                .collect(),
        )
    };
    Value::Object(vec![
        (
            "page".to_owned(),
            Value::Integer(i64::try_from(removed.page).unwrap_or(i64::MAX)),
        ),
        ("entry".to_owned(), Value::text(removed.entry.to_owned())),
        ("removed".to_owned(), Value::Bool(removed.removed)),
        ("was".to_owned(), rect(removed.was)),
        ("now".to_owned(), rect(removed.now)),
        (
            "costs_nothing".to_owned(),
            Value::Bool(removed.costs_nothing()),
        ),
    ])
}

/// One signature of the source, in one line: where, who, when, and what verifying it found.
///
/// `doc/pdf-a-conversion-limits.md` section 3.6's line, worded without the word *valid* for the
/// reason `pdf_signature::signature` gives: what was checked is that the value, the certificate and
/// the bytes belong together, and not who the signer is.
fn describe_signature(signature: &SourceSignature) -> String {
    use std::fmt::Write as _;
    let mut out = signature.at.clone();
    out.push_str(if signature.timestamp {
        ", a document timestamp"
    } else if signature.permitted.is_some() {
        ", a certification signature"
    } else {
        ", a signature"
    });
    if let Some(name) = &signature.name {
        let _ = write!(out, " by {name}");
    }
    if let Some(when) = &signature.signed_at {
        let _ = write!(out, " at {when}");
    }
    if let Some(reason) = &signature.reason {
        let _ = write!(out, ", stating the reason {reason:?}");
    }
    if let Some(permitted) = &signature.permitted {
        let _ = write!(
            out,
            ", whose DocMDP entry had every processor permit {permitted} and nothing else"
        );
    }
    let _ = write!(
        out,
        ": over the source, {}; {}; {}",
        signature.coverage, signature.integrity, signature.authenticity
    );
    out
}

/// The signature decision and each signature, worded for a refusal a person reads.
pub(super) fn describe_signatures(signed: &SignatureDecision) -> String {
    let mut out = describe(signed.decision, signed.changed, false);
    for signature in &signed.each {
        out.push_str("; ");
        out.push_str(&describe_signature(signature));
    }
    out
}

/// One decision, worded for a person.
pub(super) fn describe_decision(decided: &Decided, repeated: bool) -> String {
    describe(decided.decision, decided.changed, repeated)
}

/// One decision, worded for a person, wherever it was taken.
fn describe(decision: Decision, changed: usize, repeated: bool) -> String {
    match decision {
        Decision::Mechanical(rewrite) => {
            format!(
                "changed, losing nothing: {} ({} done)",
                rewrite.describe(),
                changed
            )
        }
        Decision::Stated {
            rewrite,
            reinterprets,
        } => {
            let sentence = if repeated {
                "the same interpretation as above"
            } else {
                reinterprets
            };
            format!(
                "changed, stating an interpretation the standard defines: {} ({} done)\n      {}",
                rewrite.describe(),
                changed,
                sentence
            )
        }
        Decision::Authorised { loss, rewrite } => format!(
            "changed with your authorisation: {} — {} ({} done)",
            rewrite.describe(),
            loss.describe(),
            changed
        ),
        Decision::Unauthorised { loss, rewrite } => format!(
            "not done, because it loses something nobody authorised: {} — {}; \
             --authorise {} allows it",
            rewrite.describe(),
            loss.describe(),
            loss.word()
        ),
        Decision::Refused(because) => format!("refused: {}", because.sentence()),
        Decision::Configured {
            kind,
            rewrite,
            warns,
        } => format!(
            "changed, because this configuration answered it with `{}`: {} ({} done)\n      {}",
            kind.word(),
            rewrite.describe(),
            changed,
            warns
        ),
    }
}

impl SignatureDecision {
    /// The signatures and the decision as JSON.
    fn to_json(&self) -> Value {
        let mut fields = vec![
            ("decision".to_owned(), Value::text(self.decision.word())),
            ("changed".to_owned(), Value::count(self.changed)),
        ];
        match self.decision {
            Decision::Authorised { loss, rewrite } | Decision::Unauthorised { loss, rewrite } => {
                fields.push(("rewrite".to_owned(), Value::text(rewrite.word())));
                fields.push(("loss".to_owned(), Value::text(loss.word())));
                fields.push(("loses".to_owned(), Value::text(loss.describe())));
            }
            Decision::Refused(because) => {
                fields.push(("because".to_owned(), Value::text(because.word())));
                fields.push(("reason".to_owned(), Value::text(because.sentence())));
            }
            Decision::Mechanical(rewrite)
            | Decision::Stated { rewrite, .. }
            | Decision::Configured { rewrite, .. } => {
                fields.push(("rewrite".to_owned(), Value::text(rewrite.word())));
            }
        }
        fields.push((
            "each".to_owned(),
            Value::Array(self.each.iter().map(signature_to_json).collect()),
        ));
        Value::Object(fields)
    }
}

/// One signature of the source as JSON.
fn signature_to_json(signature: &SourceSignature) -> Value {
    Value::Object(vec![
        ("reached".to_owned(), Value::text(signature.reached.word())),
        ("at".to_owned(), Value::text(signature.at.clone())),
        (
            "name".to_owned(),
            signature
                .name
                .as_ref()
                .map_or(Value::Null, |name| Value::text(name.clone())),
        ),
        (
            "signed_at".to_owned(),
            signature
                .signed_at
                .as_ref()
                .map_or(Value::Null, |when| Value::text(when.clone())),
        ),
        (
            "reason".to_owned(),
            signature
                .reason
                .as_ref()
                .map_or(Value::Null, |reason| Value::text(reason.clone())),
        ),
        ("timestamp".to_owned(), Value::Bool(signature.timestamp)),
        (
            "permitted".to_owned(),
            signature
                .permitted
                .as_ref()
                .map_or(Value::Null, |permitted| Value::text(permitted.clone())),
        ),
        (
            "byte_range".to_owned(),
            Value::text(signature.coverage.clone()),
        ),
        (
            "digest".to_owned(),
            Value::text(signature.integrity.clone()),
        ),
        (
            "verification".to_owned(),
            Value::text(signature.authenticity.clone()),
        ),
    ])
}

impl Decided {
    /// One decision as JSON.
    fn to_json(&self) -> Value {
        let mut fields = vec![
            ("requirement".to_owned(), Value::text(self.requirement)),
            ("clause".to_owned(), Value::text(self.citation.clone())),
            ("asks".to_owned(), Value::text(self.asks)),
            ("places".to_owned(), Value::count(self.places)),
            ("decision".to_owned(), Value::text(self.decision.word())),
            ("changed".to_owned(), Value::count(self.changed)),
        ];
        match self.decision {
            Decision::Mechanical(rewrite) => {
                fields.push(("rewrite".to_owned(), Value::text(rewrite.word())));
            }
            Decision::Stated {
                rewrite,
                reinterprets,
            } => {
                fields.push(("rewrite".to_owned(), Value::text(rewrite.word())));
                fields.push(("reinterprets".to_owned(), Value::text(reinterprets)));
            }
            Decision::Authorised { loss, rewrite } | Decision::Unauthorised { loss, rewrite } => {
                fields.push(("rewrite".to_owned(), Value::text(rewrite.word())));
                fields.push(("loss".to_owned(), Value::text(loss.word())));
                fields.push(("loses".to_owned(), Value::text(loss.describe())));
            }
            Decision::Refused(because) => {
                fields.push(("because".to_owned(), Value::text(because.word())));
                fields.push(("reason".to_owned(), Value::text(because.sentence())));
            }
            Decision::Configured {
                kind,
                rewrite,
                warns,
            } => {
                fields.push(("rewrite".to_owned(), Value::text(rewrite.word())));
                fields.push(("remedy".to_owned(), Value::text(kind.word())));
                fields.push(("warns".to_owned(), Value::text(warns)));
            }
        }
        Value::Object(fields)
    }
}

impl NotChecked {
    /// One of the validator's unchecked rows, carried across.
    pub(super) fn of(judgement: &Judgement) -> Self {
        Self {
            requirement: judgement.id,
            citation: judgement.citation.clone(),
            because: match judgement.outcome {
                Outcome::Unchecked(why) => why,
                // `Report::unchecked` yields only `Outcome::Unchecked`, so this arm is
                // unreachable; it names the bug rather than panicking.
                _ => "the validator reported no reason",
            },
        }
    }

    /// One unchecked requirement as JSON.
    fn to_json(&self) -> Value {
        Value::Object(vec![
            ("requirement".to_owned(), Value::text(self.requirement)),
            ("clause".to_owned(), Value::text(self.citation.clone())),
            ("not_checked_because".to_owned(), Value::text(self.because)),
        ])
    }
}

impl Achieved {
    /// The output's verdict as JSON.
    fn to_json(&self) -> Value {
        Value::Object(vec![
            ("conforms".to_owned(), Value::Bool(self.conforms)),
            ("checked".to_owned(), Value::count(self.checked)),
            (
                "still_failing".to_owned(),
                Value::Array(
                    self.still_failing
                        .iter()
                        .map(|id| Value::text(*id))
                        .collect(),
                ),
            ),
            (
                "regressions".to_owned(),
                Value::Array(self.regressions.iter().map(|id| Value::text(*id)).collect()),
            ),
        ])
    }
}

impl Departed {
    /// One departure as JSON.
    fn to_json(&self) -> Value {
        let (outcome, claimed, left) = match &self.outcome {
            DepartureOutcome::Applied { claimed } => ("applied", Some(*claimed), None),
            DepartureOutcome::LeftInPlace { file, .. } => {
                ("left-in-place", None, Some(file.clone()))
            }
        };
        Value::Object(vec![
            ("requirement".to_owned(), Value::text(self.requirement)),
            ("clause".to_owned(), Value::text(self.citation.clone())),
            (
                "media_types".to_owned(),
                Value::Array(
                    self.media_types
                        .iter()
                        .map(|it| Value::text(it.clone()))
                        .collect(),
                ),
            ),
            (
                "relationships".to_owned(),
                Value::Array(
                    self.relationships
                        .iter()
                        .map(|it| Value::text(it.clone()))
                        .collect(),
                ),
            ),
            ("reason".to_owned(), Value::text(self.reason.clone())),
            ("outcome".to_owned(), Value::text(outcome)),
            (
                "claims_conformance".to_owned(),
                claimed.map_or(Value::Null, Value::Bool),
            ),
            (
                "left_in_place".to_owned(),
                left.map_or(Value::Null, Value::text),
            ),
        ])
    }
}
