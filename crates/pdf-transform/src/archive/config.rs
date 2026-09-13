//! A remedy configuration: a refusal answered in advance, read by the caller and handed in as data.
//!
//! # What this is, and the seam it keeps
//!
//! `doc/rfc/0007` is the design. Its sentence is the whole of the motivation — *a refusal would
//! mean that a human has to intervene* — and the reframe is that a refusal is a question the
//! converter asks, so the answer can be given in advance in a file. This module is the file's
//! reader and nothing more: it turns TOML ([`super::toml`]) into a [`Configuration`], a data value
//! the caller builds and hands to [`ArchivePlan`]. **Nothing here opens a path, reads a clock or
//! spawns a process** — RFC 0002 section 5's rule that `apply` is a pure function of its inputs is
//! why the configuration is *read by the caller*, exactly as `Policy` and `Budget` are.
//!
//! [`ArchivePlan`]: super::ArchivePlan
//!
//! # What it does today, and what it is shaped for
//!
//! The owner's answers `A54`–`A60` (2026-09-12) draw the line this file builds to. Four remedies
//! reach a real conversion:
//!
//! - **`discard`**, where the site is one of the seven `doc/pdf-a-conversion-limits.md` section 3
//!   losses — it becomes the [`Authorisations`] entry a caller would otherwise type on the command
//!   line, so a configuration authorising `image-smoothing` and `--authorise image-smoothing`
//!   convert the same document identically.
//! - **`derive`** at [`DERIVABLE`]'s two sites: an embedded file the target refuses becomes what a
//!   program the configuration declares made from it. `A55` is the permission and its four terms;
//!   [`super::remedies`] carries them out and `A54`'s request-and-executor shape is why nothing
//!   here starts a process.
//! - **`supply`** at [`SUPPLIABLE`]'s one: the operator states the media type of their own
//!   attachments, which nothing in a file specification states and which this converter may not
//!   infer from an extension.
//! - **`preserve`**, in either of the two mechanisms `doc/rfc/0007` section 4.6.1 separates and
//!   the owner's answer of 2026-09-11 is the reason for: `placement = "append"` lays the content
//!   out on pages appended to the document, which every target admits and which `doc/adr/1014`'s
//!   amendment to `CLAUDE.md` permits; `placement = "attach"` keeps it an attachment, unchanged at
//!   a target that holds the original and derived into a conforming one where the target will not
//!   — the same [`Derivation`] a `derive` row builds, wired to this word rather than written
//!   twice. Neither is a fallback for the other, so a `preserve` stating no mechanism is an error.
//! - **A departure** (section 4.7), the first of which accepts XML and only XML attachments when
//!   the target is `PDF/A-2` — `A60`'s case, `ZUGFeRD` and `Factur-X`'s, and the only route an operator
//!   has now that part 3 is not a target.
//!
//! Every remedy at a site whose rewrite is unbuilt — `preserve` by page anywhere but
//! [`PRESERVABLE_BY_PAGE`], and `derive`, `supply` or `discard` outside their own lists — is
//! **recognised, validated and enumerated but not applied**: naming one leaves that site's
//! requirement refused with the sentence it already carries, so the configuration promises nothing
//! it cannot keep (`CLAUDE.md` principle 1, and the round's own rule — *a promise nothing will
//! keep is worse than a refusal with a sentence*). The format is built so each slots in without a
//! reader change.
//!
//! Installing a configuration therefore changes no pipeline unless it authorises a built loss or
//! names a departure — which is `doc/adr/0954`'s requirement that `stop` stay every site's default.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::time::Duration;

use pdf_archive::{Target, table};
use pdf_syntax::Document;
use pdf_syntax::object::ObjectId;

use crate::tool::{Bounds, PLACEHOLDERS, Tool};

use super::decision::{Answer, Loss, REMEDIES};
use super::toml::{self, Value};

/// `doc/rfc/0007` section 2's remedy vocabulary, closed and small so a configuration is legible.
///
/// The order is what happens to the document's content, which is the sort the RFC's section 2
/// argues is load-bearing: nothing, a loss, a move, a new representation — and section 0.2 of
/// `doc/pdf-a-mitigations.md` adds the fifth, where the operator rather than the document is the
/// source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// The conversion refuses, as today. Every site's default.
    Stop,
    /// The information is lost, with the operator's authorisation — section 3's authorised loss.
    Discard,
    /// The information survives, somewhere the target admits.
    Preserve,
    /// A new representation is made from content the document already has.
    Derive,
    /// The operator states a fact the document does not — section 0.2's fifth kind.
    Supply,
}

impl Kind {
    /// The word a configuration names this kind by.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Stop => "stop",
            Self::Discard => "discard",
            Self::Preserve => "preserve",
            Self::Derive => "derive",
            Self::Supply => "supply",
        }
    }

    /// The kind a configuration's word names.
    #[must_use]
    pub fn parse(word: &str) -> Option<Self> {
        [
            Self::Stop,
            Self::Discard,
            Self::Preserve,
            Self::Derive,
            Self::Supply,
        ]
        .into_iter()
        .find(|kind| kind.word() == word)
    }

    /// Whether a remedy of this kind reaches a real conversion at the sites that admit it.
    ///
    /// `stop` is a no-op, `discard` is an authorised loss, `derive` runs a declared tool over one
    /// of [`DERIVABLE`]'s two sites, `supply` writes the operator's own fact at [`SUPPLIABLE`]'s
    /// one, and `preserve` moves content the target will not admit — onto pages appended to the
    /// document ([`PRESERVABLE_BY_PAGE`], under `doc/adr/1014`'s amendment) or into an attachment
    /// the target does admit ([`DERIVABLE`], through the same derivation `derive` uses).
    ///
    /// *Which sites* admit each is the question [`Configuration::unbuilt`] answers; this is only
    /// the kind.
    #[must_use]
    pub const fn is_built(self) -> bool {
        !matches!(self, Self::Stop)
    }
}

/// One thing a configuration got wrong, named so a person can fix it.
///
/// **An error naming both**, which is `doc/rfc/0007` section 4.6's rule and `doc/adr/0954`'s: a
/// configuration that silently did less than it said is the failure mode the whole feature exists
/// to remove, so an unknown site, an unknown remedy, or a remedy a target cannot admit is stated
/// rather than ignored.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConfigError {
    /// The file is not the subset of TOML a configuration is written in.
    #[error("the configuration is not readable: {0}")]
    Toml(String),
    /// A `[site."…"]` names something no requirement of ISO 19005 is called.
    #[error(
        "line {line}: the site {site:?} is no requirement this converter knows; \
         `--remedy-sites --to <target>` lists every one"
    )]
    UnknownSite {
        /// The 1-based line.
        line: usize,
        /// The site the configuration named.
        site: String,
    },
    /// A `remedy` names a word outside the closed vocabulary.
    #[error(
        "line {line}: the site {site:?} states the remedy {remedy:?}, which is none of \
         stop, discard, preserve, derive or supply"
    )]
    UnknownRemedy {
        /// The 1-based line.
        line: usize,
        /// The site.
        site: String,
        /// The word it named.
        remedy: String,
    },
    /// A key held the wrong kind of value.
    #[error("line {line}: {key} in {site:?} wants {wanted}, and this is {found}")]
    WrongValue {
        /// The 1-based line.
        line: usize,
        /// The table the key is in.
        site: String,
        /// The key.
        key: String,
        /// What the key wanted.
        wanted: &'static str,
        /// What was there.
        found: &'static str,
    },
    /// `on-failure` was a list, which `A57` rules out.
    #[error(
        "line {line}: on-failure in {site:?} takes one remedy and not a chain — the owner's \
         answer A57 — so it is a string, not a list"
    )]
    FallbackChain {
        /// The 1-based line.
        line: usize,
        /// The site.
        site: String,
    },
    /// A target qualifier names something that is not a target.
    #[error(
        "line {line}: {qualifier:?} in {site:?} is no target; the targets are 2b, 2u, 2a, 4, 4f, 4e"
    )]
    UnknownTargetQualifier {
        /// The 1-based line.
        line: usize,
        /// The site.
        site: String,
        /// The qualifier.
        qualifier: String,
    },
    /// A `[depart."…"]` names a requirement this round's departure cannot carry.
    #[error(
        "line {line}: a departure from {requirement:?} is not built — the only departure this \
         version carries is from embedded-files/embedded-file-is-itself-pdfa (and its plain-profile \
         sibling), which is A60's XML-attachment case"
    )]
    UnsupportedDeparture {
        /// The 1-based line.
        line: usize,
        /// The requirement named.
        requirement: String,
    },
    /// A departure states no `reason`, which section 4.7.2 requires.
    #[error(
        "line {line}: the departure from {requirement:?} states no reason, and one is required — \
         a departure nobody wrote a reason for is one nobody will be able to explain in two years"
    )]
    DepartureWithoutReason {
        /// The 1-based line.
        line: usize,
        /// The requirement.
        requirement: String,
    },
    /// A `derive` site names no `tool`, which `A55` makes a condition rather than a convenience.
    ///
    /// **The guardrail, as an error.** The owner's answer requires a `derive` remedy to be
    /// unreachable "without the configuration naming the site *and* the tool": a site alone is
    /// half of an instruction, and a converter that read it as *derive somehow* would be deriving
    /// content by accident, which is the one thing `A55` rules out.
    #[error(
        "line {line}: the site {site:?} answers with `derive` and names no tool. Deriving content \
         makes something that was not in the document before, so the owner's answer A55 requires \
         the configuration to name the site *and* the tool: add `tool = \"<name>\"` and a \
         [tool.<name>] block declaring the program"
    )]
    DeriveWithoutTool {
        /// The 1-based line.
        line: usize,
        /// The site.
        site: String,
    },
    /// A site references a tool no `[tool.…]` block declares.
    #[error(
        "line {line}: the site {site:?} names the tool {tool:?}, and this configuration declares \
         no [tool.{tool}] block. Every external program a configuration can run is declared in \
         one place so a reviewer can read them all at once — doc/rfc/0007 section 3"
    )]
    UndeclaredTool {
        /// The 1-based line.
        line: usize,
        /// The site.
        site: String,
        /// The name it referenced.
        tool: String,
    },
    /// A `[tool.…]` block omits a key it cannot have a default for.
    #[error("line {line}: the tool {tool:?} states no {key}. {why}")]
    ToolWithout {
        /// The 1-based line.
        line: usize,
        /// The tool's name.
        tool: String,
        /// The missing key.
        key: &'static str,
        /// Why it has no default.
        why: &'static str,
    },
    /// A `[tool.…]` key holds a value this reader cannot make sense of.
    #[error("line {line}: the tool {tool:?} states {key} as {value:?}, which is not {wanted}")]
    ToolValue {
        /// The 1-based line.
        line: usize,
        /// The tool's name.
        tool: String,
        /// The key.
        key: &'static str,
        /// What was written.
        value: String,
        /// What the key wants.
        wanted: &'static str,
    },
    /// An argument holds a placeholder outside section 4.1's vocabulary.
    #[error(
        "line {line}: the tool {tool:?} writes {placeholder} in its args, which is no placeholder \
         this converter substitutes. The vocabulary is {{in}}, {{out}}, {{site}} and {{target}}; \
         {{media-type}} is deliberately not among them, because the only media type available at \
         a site that runs a tool is the document's own, and doc/rfc/0007 section 4.1 keeps \
         document-derived strings out of args"
    )]
    UnknownPlaceholder {
        /// The 1-based line.
        line: usize,
        /// The tool's name.
        tool: String,
        /// What was written.
        placeholder: String,
    },
    /// A site's remedy is one this version carries out, and one of its keys asks for a shape of it
    /// that is not built.
    ///
    /// **A refusal with a sentence rather than a promise nothing keeps.** A configuration key this
    /// converter read and then ignored would be the worse failure of the two, and it is the one the
    /// whole feature exists to remove.
    #[error("line {line}: the site {site:?} asks for {asked}, and {why}")]
    NotBuiltThatWay {
        /// The 1-based line.
        line: usize,
        /// The site.
        site: String,
        /// What the configuration asked for.
        asked: String,
        /// What is built instead.
        why: &'static str,
    },
}

/// A departure from one requirement, narrowed by a declarative predicate.
///
/// `doc/rfc/0007` section 4.7. **Not a remedy**: every remedy produces a file that conforms, and a
/// departure produces one that does not — so it is named per requirement, carries a narrowing
/// predicate (the media-type form is enough to start), states a `reason`, and by default leaves the
/// PDF/A identification off the output so the file does not claim what it has not earned (`A59`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Departure {
    /// The requirement identifier departed from.
    pub requirement: String,
    /// The media types the departure admits — section 4.7.2's `(and only xml)`. Empty admits any.
    pub media_types: Vec<String>,
    /// The `/AFRelationship` values the departure admits, narrowing further. Empty admits any.
    pub relationships: Vec<String>,
    /// Why, copied verbatim into the report and the file's `xmpMM:History`.
    pub reason: String,
}

/// Whether a departure's predicate covers a document, and why not where it does not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Coverage {
    /// Every embedded file the requirement is about matches the predicate; the departure applies.
    Covers,
    /// An embedded file the predicate does not admit, so the requirement stays refused.
    Leaves {
        /// The file's own name.
        file: String,
        /// The media type it stated, or `None` where it stated none.
        media_type: Option<String>,
    },
}

impl Departure {
    /// The two requirement identifiers a built departure may name.
    ///
    /// `embedded-files/embedded-file-is-itself-pdfa` binds PDF/A-2's three levels;
    /// `…-in-the-plain-profile` binds PDF/A-4 plain. Both are the embedding rule the owner's XML
    /// case departs from; the 4f and 4e flavours lift the rule outright and need no departure.
    const SUPPORTED: [&'static str; 2] = [
        "embedded-files/embedded-file-is-itself-pdfa",
        "embedded-files/embedded-file-is-itself-pdfa-in-the-plain-profile",
    ];

    /// Whether this round can carry this departure out.
    #[must_use]
    pub fn is_built(&self) -> bool {
        Self::SUPPORTED.contains(&self.requirement.as_str())
    }

    /// Whether the departure's predicate covers this document.
    ///
    /// The predicate is `(and only xml)`, so the question is asked of **every** embedded file the
    /// requirement is about — the same population ISO 19005-2 section 6.8 binds: every file
    /// specification carrying an `/EF`. The departure covers the document only where every one of
    /// them matches, which is the difference between *accept XML attachments* and *accept XML and
    /// only XML attachments* — the second is what makes a departure narrower than any target.
    #[must_use]
    pub fn covers(&self, document: &Document) -> Coverage {
        for (name, media_type, relationship) in embedded_files(document) {
            let type_ok = self.media_types.is_empty()
                || media_type
                    .as_deref()
                    .is_some_and(|stated| self.media_types.iter().any(|want| want == stated));
            let relationship_ok =
                self.relationships.is_empty() || self.relationships.contains(&relationship);
            if !type_ok || !relationship_ok {
                return Coverage::Leaves {
                    file: name,
                    media_type,
                };
            }
        }
        Coverage::Covers
    }
}

/// A site the configuration named whose remedy this round does not carry out.
///
/// Reported so the operator sees their intent was read, not ignored — the requirement itself is
/// refused with the sentence it already carries, which names what the remedy waits on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unbuilt {
    /// The site.
    pub site: String,
    /// The remedy the configuration named.
    pub remedy: Kind,
}

/// One `derive` remedy the configuration named, with the program it names and its fallback.
///
/// `doc/questions/A55`, and the type is the guardrail: it cannot be built without a [`Tool`], so
/// "never reachable without the configuration naming the site *and* the tool" is a property of
/// the shape rather than of a check somebody has to remember to run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Derivation {
    /// The requirement identifier the derivation answers.
    pub site: String,
    /// The program, as the `[tool.…]` block declares it.
    pub tool: Tool,
    /// What is done where the tool declines or fails — `A57`: one alternative, never a chain.
    pub on_failure: Kind,
}

/// One fact the operator states that the document does not.
///
/// `doc/pdf-a-mitigations.md` section 0.2's fifth remedy kind. **The operator is the source**, which
/// is the whole of what separates it from every other kind: nothing is lost, nothing moves, and
/// nothing is computed from the document — a person has put their own knowledge into the file. So
/// it carries an obligation the other four do not, and section 5b.1 of `doc/rfc/0007` states it:
/// the report names the supplied value beside the requirement it answered, and `xmpMM:History`
/// records that a human rather than the document is its source.
///
/// One variant today. A second site adds a second variant, which is a compile error everywhere the
/// kind must be answered — the same construction `Authorisations` uses, and for the same reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Supplied {
    /// `embedded-files/associated-file-media-type`: what the operator's own attachments are.
    ///
    /// ISO 19005-4 section 6.9 asks an associated file's stream for a `/Subtype` that is a MIME
    /// media type, and nothing in a file specification states one — an extension is a convention
    /// rather than a declaration, so reading it as one would be this converter asserting what the
    /// bytes are. An operator whose pipeline produces the attachments does know.
    MediaTypes {
        /// Extension (with its full stop) to media type, in the order the file wrote them.
        by_extension: Vec<(String, String)>,
    },
}

/// One `supply` remedy the configuration named.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Supply {
    /// The requirement identifier it answers.
    pub site: String,
    /// The operator's fact.
    pub fact: Supplied,
}

/// Which of `preserve`'s two mechanisms a row asks for.
///
/// `doc/rfc/0007` section 4.6.1's finding is that the six targets differ about what may be
/// *attached*, not about what may be a page — so `preserve` is not one operation with a fallback
/// but two mechanisms an operator chooses between, and the configuration has to say which. A row
/// that named neither would leave this converter guessing what an archive is for: somebody
/// archiving to PDF/A-4 may reasonably prefer the content visible in the document over an
/// attachment a reader has to go looking for, and only they know that.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    /// The content is laid out on pages appended to the document (available at all six targets).
    Append,
    /// The content stays an attachment, unchanged where the target admits it and derived into one
    /// the target admits where it does not.
    Attach,
}

impl Placement {
    /// The word a configuration names this mechanism by.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Append => "append",
            Self::Attach => "attach",
        }
    }

    /// The mechanism a configuration's word names.
    #[must_use]
    pub fn parse(word: &str) -> Option<Self> {
        [Self::Append, Self::Attach]
            .into_iter()
            .find(|placement| placement.word() == word)
    }
}

/// One `preserve` remedy this conversion carries out by appending pages.
///
/// `doc/adr/1014`: a page composed solely of content the document already holds is on the near
/// side of `CLAUDE.md`'s authoring exclusion, and what such a page owes — the report's sentence,
/// the page labels, the structure entries, the `xmpMM:History` record — is inside the same
/// permission. [`super::preserve`] is where each is discharged or refused by name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preservation {
    /// The requirement identifier it answers.
    pub site: String,
}

/// The one requirement a built `preserve` may answer by appending pages.
///
/// ISO 19005-2 section 6.6.2.3.1's, whose only other answer is
/// `doc/pdf-a-conversion-limits.md` section 3.9's authorised loss: the properties come out of the
/// packet either way, and this keeps what the producer wrote where a person can still read it.
/// It is the owner's own example in `doc/rfc/0007` section 2 — *instead of losing metadata it
/// could be appended or prefixed as an extra page*.
const PRESERVABLE_BY_PAGE: [&str; 1] = ["metadata/properties-use-known-schemas"];

/// Why a `preserve` row states no mechanism.
const PRESERVE_WITHOUT_PLACEMENT: &str = "preserve has two mechanisms and neither is a fallback \
     for the other (doc/rfc/0007 section 4.6.1): `placement = \"append\"` lays the content out on \
     pages appended to the document, which every target admits, and `placement = \"attach\"` keeps \
     it an attachment, which only some do. Which of those an archive wants is the operator's \
     decision and this converter will not take it";

/// Why a `preserve` by attachment at a target that admits no such attachment needs a tool.
const ATTACHING_NEEDS_A_CONFORMING_FILE: &str = "this target admits an embedded file only where \
     that file itself conforms to a part of ISO 19005, so preserving this attachment by keeping it \
     an attachment means attaching one derived from it — name the tool that makes it (`tool = \
     \"<name>\"` and a [tool.<name>] block), or choose `placement = \"append\"` to keep the \
     content in the document's body instead. PDF/A-4f and PDF/A-4e are the targets that hold the \
     original unchanged, and doc/pdf-a-conversion-limits.md section 9 is why this converter will \
     not switch to one for you";

/// The two requirements a built `derive` may name.
///
/// `doc/pdf-a-mitigations.md` section 11's flagship entry and `doc/rfc/0007` section 5's first row:
/// an embedded file that is not itself PDF/A, derived into one with a declared tool. The plain
/// PDF/A-4 profile states the same rule under its own identifier; 4f and 4e lift it outright and
/// need no remedy at all.
const DERIVABLE: [&str; 2] = [
    "embedded-files/embedded-file-is-itself-pdfa",
    "embedded-files/embedded-file-is-itself-pdfa-in-the-plain-profile",
];

/// Why a `derive` at an embedded-file site needs a tool that makes a PDF.
const WANTS_A_PDF: &str = "the requirement it answers is that the embedded file itself conform to \
     a part of ISO 19005, so the only artefact that can answer it is a PDF: declare the tool with \
     expects = \"application/pdf\"";

/// Why `on-failure` at a built `derive` site takes only `stop` this version.
const ONLY_STOP_ON_FAILURE: &str = "the only on-failure this version carries out is `stop`. \
     Dropping an attachment is a rewrite of the embedded-file name tree that nobody has written, \
     and offering it here would be selling a permanent hole in somebody's archive to get past an \
     afternoon of ours (doc/pdf-a-mitigations.md section 0.2)";

/// Why `on-failure` at a `preserve` by appended page takes only `stop` this version.
const ONLY_STOP_WHEN_A_PAGE_CANNOT_BE_COMPOSED: &str = "the only on-failure this version carries \
     out is `stop`: a preservation this converter cannot compose leaves the requirement refused \
     with the composition's own reason, which says what stopped it. Falling back to `discard` \
     would quietly lose the very content the remedy was named to keep, which is the one outcome \
     an operator choosing `preserve` has ruled out";

/// Why `unlisted` at a built `supply` site takes only `stop` this version.
const ONLY_STOP_UNLISTED: &str = "the only unlisted this version carries out is `stop`: an \
     attachment whose extension your table does not name leaves its requirement refused, which is \
     what `stop` is. Naming its extension is the answer, and guessing the type would be the one \
     thing `supply` exists to avoid";

/// The one requirement a built `supply` may name.
///
/// `doc/pdf-a-mitigations.md` section 11's `embedded-files/associated-file-media-type`, which its own
/// entry calls "a good test of the owner's question" and says passes cleanly: the operator needs to
/// know what their own attachments are, types nothing about any individual document, and can read
/// the cost in one line — *the archive asserts these media types on our authority*.
const SUPPLIABLE: [&str; 1] = ["embedded-files/associated-file-media-type"];

/// One remedy configuration, read from a file and validated against a target.
///
/// The two things a conversion consumes are [`Self::authorisations`] and [`Self::departures`]; the
/// rest is what the report tells the operator about answers this round could not yet act on.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Configuration {
    /// The profile's own name, where the file is a shipped profile (`[profile] name`).
    pub name: Option<String>,
    /// Every site the file named, in file order, with the remedy it chose and its keys.
    sites: Vec<Row>,
    /// Every external program the file declares, by the name a site references it under.
    tools: BTreeMap<String, Tool>,
    /// The departures the file states.
    pub departures: Vec<Departure>,
}

/// One `[site."…"]` table, read.
///
/// The site-specific keys live here rather than in five parallel maps because
/// `doc/rfc/0007` section 3 makes them a property of the site: a reader that split them would have
/// to put them back together to answer any question about one row.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Row {
    /// The requirement identifier.
    site: String,
    /// The remedy word.
    remedy: Kind,
    /// The `[tool.…]` block this row references, where it references one.
    tool: Option<String>,
    /// `on-failure` — `A57`: one alternative, never a chain. `stop` where the row states none.
    on_failure: Kind,
    /// `placement` — which of `preserve`'s two mechanisms, where the row is a `preserve`.
    placement: Option<Placement>,
    /// `media-types`, the `supply` table this version reads.
    media_types: Vec<(String, String)>,
    /// `unlisted` — what happens to a subject the supplied table does not name.
    unlisted: Kind,
    /// The 1-based line its header sits on, for an error a person can find.
    line: usize,
}

impl Configuration {
    /// Reads and validates a configuration against the target it will be used for.
    ///
    /// # Errors
    ///
    /// [`ConfigError`], naming the line and what was wrong. A site the target does not bind is not
    /// an error — the refusal never arises, so the answer is never consulted (a profile works with
    /// all six targets for exactly this reason) — but a site no requirement is called, a remedy
    /// outside the vocabulary, and an `on-failure` chain are all stated rather than ignored.
    pub fn read(text: &str, target: Target) -> Result<Self, ConfigError> {
        let tables = toml::parse(text).map_err(|error| ConfigError::Toml(error.to_string()))?;
        let requirements: BTreeSet<&'static str> = table::requirements()
            .map(|requirement| requirement.id)
            .collect();
        let mut name = None;
        let mut sites: Vec<Row> = Vec::new();
        let mut seen_sites: BTreeSet<String> = BTreeSet::new();
        let mut departures = Vec::new();
        let mut tools: BTreeMap<String, Tool> = BTreeMap::new();
        for tbl in &tables {
            match tbl.path.first().map(String::as_str) {
                Some("profile") => {
                    name = tbl.get("name").and_then(Value::as_text).map(str::to_owned);
                    // `doc/pdf-a-mitigations.md` section 14's ninth finding: a `default` may only be
                    // `stop` — today's behaviour, stated deliberately so `refuse-any-loss` is not
                    // the empty file. A default of anything else would authorise a loss for every
                    // site at once, which is exactly what a per-site vocabulary exists to prevent.
                    if let Some(default) = tbl.get("default")
                        && default.as_text() != Some("stop")
                    {
                        return Err(ConfigError::WrongValue {
                            line: tbl.line,
                            site: "profile".to_owned(),
                            key: "default".to_owned(),
                            wanted: "the string \"stop\" (the only default a profile may state)",
                            found: default.kind(),
                        });
                    }
                }
                Some("site") => {
                    let site = site_identifier(tbl, &requirements)?;
                    let remedy = site_remedy(tbl, &site)?;
                    check_fallback(tbl, &site)?;
                    check_target_qualifier(tbl, &site)?;
                    check_key_shapes(tbl, &site)?;
                    let row = row(tbl, site, remedy)?;
                    // The target qualifier decides *whether this row applies* to the conversion:
                    // an unqualified row is the default, a qualified one wins for its target. A
                    // row for another target is read (and validated) but does not answer here.
                    if applies_to(tbl, target) && seen_sites.insert(row.site.clone()) {
                        sites.push(row);
                    }
                }
                Some("depart") => {
                    if let Some(departure) = departure(tbl, target)? {
                        departures.push(departure);
                    }
                }
                Some("tool") => {
                    let declared = declared_tool(tbl)?;
                    tools.insert(declared.name.clone(), declared);
                }
                _ => {}
            }
        }
        check_rows(&sites, &tools, target)?;
        Ok(Self {
            name,
            sites,
            tools,
            departures,
        })
    }

    /// The command-line authorisations the built `discard` remedies stand for.
    ///
    /// A `discard` at a site that is one of the seven section 3 losses is exactly
    /// `--authorise <that loss>`, so a configuration and the flags reach the same conversion. A
    /// `discard` at any other site authorises nothing: its lossless-or-loss rewrite is unbuilt, so
    /// the requirement stays refused with the sentence it carries, and [`Self::unbuilt`] names it.
    #[must_use]
    pub fn authorisations(&self, target: Target) -> Authorisations {
        let losses = loss_sites();
        let mut authorised = Authorisations::default();
        for row in &self.sites {
            if row.remedy == Kind::Discard
                && let Some(loss) = losses.get(row.site.as_str())
                && requirement_binds(&row.site, target)
            {
                authorised.authorise(*loss);
            }
        }
        authorised
    }

    /// The sites whose remedy this round recognised but cannot carry out.
    #[must_use]
    pub fn unbuilt(&self, target: Target) -> Vec<Unbuilt> {
        let losses = loss_sites();
        let mut out = Vec::new();
        for row in &self.sites {
            if row.remedy == Kind::Stop || !requirement_binds(&row.site, target) {
                continue;
            }
            let built = match row.remedy {
                Kind::Discard => losses.contains_key(row.site.as_str()),
                Kind::Derive => self.derivation(row).is_some(),
                Kind::Supply => supplied(row).is_some(),
                // A `preserve` by appended page is built at the one site `PRESERVABLE_BY_PAGE`
                // names; one by attachment is carried out by the same derivation a `derive` row
                // would be, so it is built wherever that is.
                Kind::Preserve => match row.placement {
                    Some(Placement::Append) => PRESERVABLE_BY_PAGE.contains(&row.site.as_str()),
                    Some(Placement::Attach) => self.derivation(row).is_some(),
                    None => false,
                },
                Kind::Stop => false,
            };
            if !built {
                out.push(Unbuilt {
                    site: row.site.clone(),
                    remedy: row.remedy,
                });
            }
        }
        out
    }

    /// Every `derive` remedy this conversion carries out.
    ///
    /// A row naming a site outside [`DERIVABLE`] is not one: its requirement stays refused with the
    /// sentence it already carries, and [`Self::unbuilt`] names it so the operator sees their
    /// intent was read rather than ignored.
    #[must_use]
    pub fn derivations(&self, target: Target) -> Vec<Derivation> {
        self.sites
            .iter()
            .filter(|row| requirement_binds(&row.site, target))
            .filter_map(|row| self.derivation(row))
            .collect()
    }

    /// Every `supply` remedy this conversion carries out.
    #[must_use]
    pub fn supplies(&self, target: Target) -> Vec<Supply> {
        self.sites
            .iter()
            .filter(|row| requirement_binds(&row.site, target))
            .filter_map(supplied)
            .collect()
    }

    /// Every external program this configuration can run, in name order.
    ///
    /// What a reviewer reads to see the whole of what a configuration may start —
    /// `doc/rfc/0007` section 3's reason for declaring tools once and referencing them by name.
    pub fn tools(&self) -> impl Iterator<Item = &Tool> {
        self.tools.values()
    }

    /// One row as a built derivation, where it is one.
    fn derivation(&self, row: &Row) -> Option<Derivation> {
        if !answers_by_derivation(row) {
            return None;
        }
        // The tool is required of *every* `derive` row by [`ConfigError::DeriveWithoutTool`], so
        // reaching here without one is impossible; the `?` states that rather than asserting it.
        let tool = self.tools.get(row.tool.as_ref()?)?;
        Some(Derivation {
            site: row.site.clone(),
            tool: tool.clone(),
            on_failure: row.on_failure,
        })
    }

    /// Every `preserve` remedy this conversion carries out by appending pages.
    ///
    /// A row naming a site outside [`PRESERVABLE_BY_PAGE`], or one whose mechanism is
    /// [`Placement::Attach`], is not one: the first stays refused with the sentence it carries and
    /// [`Self::unbuilt`] names it, and the second is carried out as a [`Derivation`] instead —
    /// which is `doc/rfc/0007` section 4.6's table read as the two mechanisms it is rather than as
    /// two implementations of one.
    #[must_use]
    pub fn preservations(&self, target: Target) -> Vec<Preservation> {
        self.sites
            .iter()
            .filter(|row| requirement_binds(&row.site, target))
            .filter(|row| {
                row.remedy == Kind::Preserve
                    && row.placement == Some(Placement::Append)
                    && PRESERVABLE_BY_PAGE.contains(&row.site.as_str())
            })
            .map(|row| Preservation {
                site: row.site.clone(),
            })
            .collect()
    }

    /// The departures that bind the target and this round can carry out.
    #[must_use]
    pub fn built_departures(&self, target: Target) -> Vec<Departure> {
        self.departures
            .iter()
            .filter(|departure| {
                departure.is_built() && requirement_binds(&departure.requirement, target)
            })
            .cloned()
            .collect()
    }
}

/// Every check a row needs once the whole file has been read.
///
/// **Separated from the reader because it is a different question**, and one the reader cannot ask
/// as it goes: a `[tool.…]` block may be written below the site that names it, so an error that
/// depended on the order lines appear in would be an error about this reader rather than about the
/// configuration.
fn check_rows(
    sites: &[Row],
    tools: &BTreeMap<String, Tool>,
    target: Target,
) -> Result<(), ConfigError> {
    // **The second half of `A55`'s guardrail**, and it is checked after every table has been
    // read rather than at the row: a `[tool.…]` block may be written below the site that
    // names it, and an error that depended on the order lines appear in would be an error
    // about this reader rather than about the configuration.
    for row in sites {
        if let Some(named) = &row.tool {
            let Some(tool) = tools.get(named) else {
                return Err(ConfigError::UndeclaredTool {
                    line: row.line,
                    site: row.site.clone(),
                    tool: named.clone(),
                });
            };
            // The requirement a built `derive` answers is *that the embedded file conform to a
            // part of ISO 19005*, so a tool that promises anything else could not answer it —
            // and a configuration whose tool makes a spreadsheet into a spreadsheet is a
            // mistake worth naming before a conversion rather than after one.
            if answers_by_derivation(row) && tool.expects != "application/pdf" {
                return Err(ConfigError::NotBuiltThatWay {
                    line: row.line,
                    site: row.site.clone(),
                    asked: format!("a tool promising {}", tool.expects),
                    why: WANTS_A_PDF,
                });
            }
        }
        // **`doc/rfc/0007` section 4.6.1, as a refusal rather than a default.** Appending is an
        // operator's choice and not a fallback, so a `preserve` that names no mechanism is half an
        // instruction and the half it leaves out is the one only an archive's owner can give.
        //
        // **Asked only where this converter carries a `preserve` out**, which is the narrowness
        // the catalogue's own entries argue for: `preserve` at an ICC profile site is
        // section 8.6.5.5's alternate space and at an annotation site it is the appearance the
        // producer wrote, and neither is a page or an attachment. A row for a site with no built
        // preserve stays inert and [`Configuration::unbuilt`] names it, as every other unbuilt
        // remedy does.
        if row.remedy == Kind::Preserve
            && row.placement.is_none()
            && (PRESERVABLE_BY_PAGE.contains(&row.site.as_str())
                || DERIVABLE.contains(&row.site.as_str()))
        {
            return Err(ConfigError::NotBuiltThatWay {
                line: row.line,
                site: row.site.clone(),
                asked: "remedy = \"preserve\" with no placement".to_owned(),
                why: PRESERVE_WITHOUT_PLACEMENT,
            });
        }
        // **`doc/rfc/0007` section 4.6's error naming both.** At a target that admits an
        // embedded file only where the file itself conforms, keeping the attachment *as* an
        // attachment means attaching a conforming one derived from it — which is `derive`'s
        // machinery, wired to this row rather than written twice. Without a tool there is
        // nothing to attach, and a configuration that silently did less than it said is the
        // failure this whole feature exists to remove.
        if row.remedy == Kind::Preserve
            && row.placement == Some(Placement::Attach)
            && DERIVABLE.contains(&row.site.as_str())
            && requirement_binds(&row.site, target)
            && row.tool.is_none()
        {
            return Err(ConfigError::NotBuiltThatWay {
                line: row.line,
                site: row.site.clone(),
                asked: format!(
                    "remedy = \"preserve\" with placement = \"attach\" at the target {target}"
                ),
                why: ATTACHING_NEEDS_A_CONFORMING_FILE,
            });
        }
        if answers_by_derivation(row) && row.on_failure != Kind::Stop {
            return Err(ConfigError::NotBuiltThatWay {
                line: row.line,
                site: row.site.clone(),
                asked: format!("on-failure = \"{}\"", row.on_failure.word()),
                why: ONLY_STOP_ON_FAILURE,
            });
        }
        if row.remedy == Kind::Preserve
            && row.placement == Some(Placement::Append)
            && PRESERVABLE_BY_PAGE.contains(&row.site.as_str())
            && row.on_failure != Kind::Stop
        {
            return Err(ConfigError::NotBuiltThatWay {
                line: row.line,
                site: row.site.clone(),
                asked: format!("on-failure = \"{}\"", row.on_failure.word()),
                why: ONLY_STOP_WHEN_A_PAGE_CANNOT_BE_COMPOSED,
            });
        }
        if row.remedy == Kind::Supply
            && SUPPLIABLE.contains(&row.site.as_str())
            && row.unlisted != Kind::Stop
        {
            return Err(ConfigError::NotBuiltThatWay {
                line: row.line,
                site: row.site.clone(),
                asked: format!("unlisted = \"{}\"", row.unlisted.word()),
                why: ONLY_STOP_UNLISTED,
            });
        }
    }
    Ok(())
}

/// Reads the `[site."<id>"]` identifier, or the error that names the unknown one.
fn site_identifier(
    tbl: &toml::Table,
    requirements: &BTreeSet<&'static str>,
) -> Result<String, ConfigError> {
    let site = tbl.path.get(1).cloned().unwrap_or_default();
    if !requirements.contains(site.as_str()) {
        return Err(ConfigError::UnknownSite {
            line: tbl.line,
            site,
        });
    }
    Ok(site)
}

/// Reads a site's `remedy`, or the error naming the unknown word.
fn site_remedy(tbl: &toml::Table, site: &str) -> Result<Kind, ConfigError> {
    let Some(value) = tbl.get("remedy") else {
        // A site table stating no remedy at all defaults to `stop`, like an absent site.
        return Ok(Kind::Stop);
    };
    let Some(word) = value.as_text() else {
        return Err(ConfigError::WrongValue {
            line: tbl.line,
            site: site.to_owned(),
            key: "remedy".to_owned(),
            wanted: "a string",
            found: value.kind(),
        });
    };
    Kind::parse(word).ok_or_else(|| ConfigError::UnknownRemedy {
        line: tbl.line,
        site: site.to_owned(),
        remedy: word.to_owned(),
    })
}

/// One `[site."…"]` table read into a [`Row`], with `A55`'s guardrail applied.
fn row(tbl: &toml::Table, site: String, remedy: Kind) -> Result<Row, ConfigError> {
    let tool = tbl.get("tool").and_then(Value::as_text).map(str::to_owned);
    // **`A55`, as a refusal rather than a warning.** A `derive` row naming no tool is half an
    // instruction, and the half it states is the dangerous one: *make something that was not in
    // the document*. The owner's answer requires the site **and** the tool, so the site alone is
    // an error naming the site.
    if remedy == Kind::Derive && tool.is_none() {
        return Err(ConfigError::DeriveWithoutTool {
            line: tbl.line,
            site,
        });
    }
    let on_failure = tbl
        .get("on-failure")
        .and_then(Value::as_text)
        .and_then(Kind::parse)
        .unwrap_or(Kind::Stop);
    let placement = match tbl.get("placement").and_then(Value::as_text) {
        Some(word) => Some(
            Placement::parse(word).ok_or_else(|| ConfigError::WrongValue {
                line: tbl.line,
                site: site.clone(),
                key: "placement".to_owned(),
                wanted: "either \"append\" or \"attach\"",
                found: "another word",
            })?,
        ),
        None => None,
    };
    let unlisted = tbl
        .get("unlisted")
        .and_then(Value::as_text)
        .and_then(Kind::parse)
        .unwrap_or(Kind::Stop);
    let media_types = tbl
        .get("media-types")
        .and_then(Value::as_map)
        .map(<[(String, String)]>::to_vec)
        .unwrap_or_default();
    Ok(Row {
        site,
        remedy,
        tool,
        on_failure,
        placement,
        media_types,
        unlisted,
        line: tbl.line,
    })
}

/// One row as a built `supply`, where it is one.
///
/// An empty table is **not** an error: `doc/profiles/keep-everything.toml` writes `role-map = { }`
/// with the note *an empty map stops*, and the same reading holds here — an operator who has not
/// yet filled the table in has stated no fact, so the site keeps its refusal and [`Configuration::unbuilt`]
/// says so. Erroring instead would make a half-written profile unloadable rather than inert.
fn supplied(row: &Row) -> Option<Supply> {
    if row.remedy != Kind::Supply || !SUPPLIABLE.contains(&row.site.as_str()) {
        return None;
    }
    if row.media_types.is_empty() {
        return None;
    }
    Some(Supply {
        site: row.site.clone(),
        fact: Supplied::MediaTypes {
            by_extension: row.media_types.clone(),
        },
    })
}

/// One `[tool."…"]` block read into a [`Tool`].
///
/// Every key but `args` is required, and `doc/rfc/0007` section 4.3 states the reason for the two
/// that look like they could have defaults: *the right value is a property of the tool and a wrong
/// default is worse than an absent one*.
fn declared_tool(tbl: &toml::Table) -> Result<Tool, ConfigError> {
    let name = tbl.path.get(1).cloned().unwrap_or_default();
    let missing = |key: &'static str, why: &'static str| ConfigError::ToolWithout {
        line: tbl.line,
        tool: name.clone(),
        key,
        why,
    };
    let wrong = |key: &'static str, value: String, wanted: &'static str| ConfigError::ToolValue {
        line: tbl.line,
        tool: name.clone(),
        key,
        value,
        wanted,
    };
    let program = tbl
        .get("program")
        .and_then(Value::as_text)
        .ok_or_else(|| missing("program", NO_PROGRAM))?;
    let args: Vec<String> = tbl
        .get("args")
        .and_then(Value::as_list)
        .map(|items| items.into_iter().map(str::to_owned).collect())
        .unwrap_or_default();
    for argument in &args {
        if let Some(unknown) = unknown_placeholder(argument) {
            return Err(ConfigError::UnknownPlaceholder {
                line: tbl.line,
                tool: name.clone(),
                placeholder: unknown,
            });
        }
    }
    let expects = tbl
        .get("expects")
        .and_then(Value::as_text)
        .ok_or_else(|| missing("expects", NO_EXPECTS))?;
    if !is_media_type(expects) {
        return Err(wrong("expects", expects.to_owned(), "a MIME media type"));
    }
    let timeout = tbl
        .get("timeout")
        .and_then(Value::as_text)
        .ok_or_else(|| missing("timeout", NO_TIMEOUT))?;
    let timeout = duration(timeout).ok_or_else(|| {
        wrong(
            "timeout",
            timeout.to_owned(),
            "a time such as 60s, 5m or 2h",
        )
    })?;
    let limit = tbl
        .get("output-limit")
        .and_then(Value::as_text)
        .ok_or_else(|| missing("output-limit", NO_OUTPUT_LIMIT))?;
    let output_limit = size(limit).ok_or_else(|| {
        wrong(
            "output-limit",
            limit.to_owned(),
            "a size such as 64MiB, 512KiB or 1GiB",
        )
    })?;
    Ok(Tool {
        name,
        program: PathBuf::from(program),
        args,
        expects: expects.to_owned(),
        bounds: Bounds {
            timeout,
            output_limit,
        },
    })
}

/// Why `program` has no default.
const NO_PROGRAM: &str = "a tool is a program on this operator's machine and there is nothing to \
     guess; write the path the operating system will execute";

/// Why `expects` has no default.
const NO_EXPECTS: &str = "doc/rfc/0007 section 4.2: what comes back is not trusted, so the tool \
     declares what it promises to produce and the converter checks it — a tool returning something \
     other than what it promised would otherwise put arbitrary bytes into an archive";

/// Why `timeout` has no default.
const NO_TIMEOUT: &str = "doc/rfc/0007 section 4.3: an external program is unbounded by nature, \
     and the right timeout is a property of the tool, so a wrong default would be worse than an \
     absent one";

/// Why `output-limit` has no default.
const NO_OUTPUT_LIMIT: &str = "doc/rfc/0007 section 4.3, the same argument as the timeout's: what \
     a tool may produce is a property of the tool, and a result past the limit is a failure named \
     rather than a truncation carried into an archive";

/// The first `{…}` in an argument that is not one this converter substitutes.
fn unknown_placeholder(argument: &str) -> Option<String> {
    let mut rest = argument;
    while let Some(open) = rest.find('{') {
        let after = rest.get(open..)?;
        let close = after.find('}')?;
        let placeholder = after.get(..=close)?;
        if !PLACEHOLDERS.contains(&placeholder) {
            return Some(placeholder.to_owned());
        }
        rest = after.get(close.saturating_add(1)..)?;
    }
    None
}

/// Whether a string is a MIME media type as Internet RFC 2046 composes one: a type and a subtype.
fn is_media_type(value: &str) -> bool {
    let mut halves = value.splitn(2, '/');
    let (Some(top), Some(sub)) = (halves.next(), halves.next()) else {
        return false;
    };
    let token = |part: &str| {
        !part.is_empty()
            && part
                .bytes()
                .all(|byte| byte.is_ascii_graphic() && byte != b'/')
    };
    token(top) && token(sub)
}

/// A time written as a count and a unit: `500ms`, `60s`, `5m`, `2h`.
fn duration(value: &str) -> Option<Duration> {
    for (suffix, scale) in [
        ("ms", Duration::from_millis(1)),
        ("s", Duration::from_secs(1)),
        ("m", Duration::from_mins(1)),
        ("h", Duration::from_hours(1)),
    ] {
        let Some(count) = value.strip_suffix(suffix) else {
            continue;
        };
        let count: u32 = count.parse().ok()?;
        return scale.checked_mul(count);
    }
    None
}

/// A size written as a count and a binary unit: `4096B`, `512KiB`, `256MiB`, `1GiB`.
fn size(value: &str) -> Option<u64> {
    // Longest suffix first: every one of the three binary units ends in `B`, so testing `B`
    // before them would read `512KiB` as a count of `512Ki` bytes and refuse it.
    for (suffix, scale) in [
        ("KiB", 1_u64 << 10),
        ("MiB", 1 << 20),
        ("GiB", 1 << 30),
        ("B", 1),
    ] {
        let Some(count) = value.strip_suffix(suffix) else {
            continue;
        };
        let count: u64 = count.parse().ok()?;
        return count.checked_mul(scale);
    }
    None
}

/// `A57`: `on-failure` picks one alternative, not a chain.
fn check_fallback(tbl: &toml::Table, site: &str) -> Result<(), ConfigError> {
    match tbl.get("on-failure") {
        None | Some(Value::Text(_)) => Ok(()),
        Some(Value::List(_)) => Err(ConfigError::FallbackChain {
            line: tbl.line,
            site: site.to_owned(),
        }),
        Some(other) => Err(ConfigError::WrongValue {
            line: tbl.line,
            site: site.to_owned(),
            key: "on-failure".to_owned(),
            wanted: "a remedy word",
            found: other.kind(),
        }),
    }
}

/// Well-known site keys whose value is a boolean, checked so a typo is caught rather than ignored.
///
/// A site documents its own keys (`doc/rfc/0007` section 3), and the reader does not know most of
/// them — but the ones the shipped profiles use most are flags, and `fresh-packet = "yes"` is a
/// mistake worth naming rather than passing over. The keys a remedy this round cannot yet carry
/// out are still validated for shape, so a configuration written against the format is well formed
/// before the remedy that reads it exists.
const BOOLEAN_KEYS: &[&str] = &[
    "keep-attachment",
    "construct",
    "fresh-packet",
    "attach-source",
];

/// Well-known site keys whose value is an inline table (the `supply` kind's, section 0.2).
const MAP_KEYS: &[&str] = &["media-types", "role-map", "colourants", "schemas"];

/// Checks the well-known keys hold the shape they are documented to.
fn check_key_shapes(tbl: &toml::Table, site: &str) -> Result<(), ConfigError> {
    for entry in &tbl.entries {
        let wanted = if BOOLEAN_KEYS.contains(&entry.key.as_str()) {
            Some(("a boolean", entry.value.as_bool().is_some()))
        } else if MAP_KEYS.contains(&entry.key.as_str()) {
            Some(("an inline table", entry.value.as_map().is_some()))
        } else {
            None
        };
        if let Some((wanted, ok)) = wanted
            && !ok
        {
            return Err(ConfigError::WrongValue {
                line: entry.line,
                site: site.to_owned(),
                key: entry.key.clone(),
                wanted,
                found: entry.value.kind(),
            });
        }
    }
    Ok(())
}

/// What narrows a site row beyond its identifier.
///
/// Two qualifiers, both `doc/rfc/0007` and the mitigations catalogue argue the site key needs:
/// `doc/rfc/0007` section 4.6's **target** (a remedy differs in kind by target) and section 14's
/// first finding's **shape** (one requirement splits into two shapes with different answers — a
/// blend mode written as an array against a bare name, an inline image's `LZWDecode` against its
/// `Crypt` filter). The target qualifier is applied today; the shape qualifier is parsed and
/// validated so a configuration can be written against it, and a shape-qualified row is inert
/// until the shape-aware remedies exist, exactly as a row for another target is.
enum Qualifier<'a> {
    /// `[site."x"]` — applies to every target.
    None,
    /// `[site."x".target."<t>"]`.
    Target(&'a str),
    /// `[site."x".shape."<s>"]`. The shape string is parsed and discarded until a shape-aware
    /// remedy reads it; the row is inert meanwhile.
    Shape,
}

/// The qualifier a site header carries, or the error naming a malformed one.
fn qualifier<'a>(tbl: &'a toml::Table, site: &str) -> Result<Qualifier<'a>, ConfigError> {
    match tbl.path.as_slice() {
        [_, _] => Ok(Qualifier::None),
        [_, _, marker, value] if marker == "target" => Ok(Qualifier::Target(value.as_str())),
        [_, _, marker, _] if marker == "shape" => Ok(Qualifier::Shape),
        _ => Err(ConfigError::UnknownTargetQualifier {
            line: tbl.line,
            site: site.to_owned(),
            qualifier: tbl.path.get(2).cloned().unwrap_or_default(),
        }),
    }
}

/// Validates a site header's qualifier: a target one must name a target.
fn check_target_qualifier(tbl: &toml::Table, site: &str) -> Result<(), ConfigError> {
    if let Qualifier::Target(value) = qualifier(tbl, site)?
        && Target::parse(value).is_none()
    {
        return Err(ConfigError::UnknownTargetQualifier {
            line: tbl.line,
            site: site.to_owned(),
            qualifier: value.to_owned(),
        });
    }
    Ok(())
}

/// Whether a site row applies to this conversion's target.
///
/// An unqualified row applies to every target; a target-qualified one only to the target it names
/// (`doc/rfc/0007` section 4.6, why the same profile works with all six targets — the 4f-only answer
/// sits behind a qualifier the other five skip); a shape-qualified row is inert until a shape-aware
/// remedy reads it, because no conversion carries a shape yet.
fn applies_to(tbl: &toml::Table, target: Target) -> bool {
    match qualifier(tbl, "") {
        Ok(Qualifier::None) => true,
        Ok(Qualifier::Target(value)) => Target::parse(value) == Some(target),
        Ok(Qualifier::Shape) | Err(_) => false,
    }
}

/// Reads a `[depart."<id>"]` table into a [`Departure`], where the target binds the requirement.
fn departure(tbl: &toml::Table, target: Target) -> Result<Option<Departure>, ConfigError> {
    let requirement = tbl.path.get(1).cloned().unwrap_or_default();
    let media_types = string_list(tbl, "media-type");
    let relationships = string_list(tbl, "relationship");
    let Some(reason) = tbl.get("reason").and_then(Value::as_text) else {
        return Err(ConfigError::DepartureWithoutReason {
            line: tbl.line,
            requirement,
        });
    };
    let departure = Departure {
        requirement: requirement.clone(),
        media_types,
        relationships,
        reason: reason.to_owned(),
    };
    if !departure.is_built() {
        return Err(ConfigError::UnsupportedDeparture {
            line: tbl.line,
            requirement,
        });
    }
    // A departure from a requirement the target does not bind is inert — the rule is not in force,
    // so there is nothing to depart from. It is read and validated, and simply does not apply.
    Ok(requirement_binds(&requirement, target).then_some(departure))
}

/// A key's value read as a list of strings, empty where the key is absent.
fn string_list(tbl: &toml::Table, key: &str) -> Vec<String> {
    tbl.get(key)
        .and_then(Value::as_list)
        .map(|items| items.into_iter().map(str::to_owned).collect())
        .unwrap_or_default()
}

/// The requirement identifier → [`Loss`] map, read off the decision table.
///
/// The one place a `discard` becomes a built authorisation. Read off [`REMEDIES`] so it cannot
/// disagree with what the converter actually authorises — a site is a built `discard` exactly
/// where the decision table answers its requirement with `Answer::Loses`.
fn loss_sites() -> BTreeMap<&'static str, Loss> {
    REMEDIES
        .iter()
        .filter_map(|remedy| match remedy.answer {
            Answer::Loses(loss, _) => Some((remedy.requirement, loss)),
            _ => None,
        })
        .collect()
}

/// Whether a requirement identifier names a requirement the target binds.
/// Whether one row is answered by handing an attachment to a declared program.
///
/// Two remedy words reach the same mechanism, and that is deliberate rather than an accident of
/// the reader: `derive` says *make a new representation*, and `preserve` with
/// [`Placement::Attach`] says *keep this attachment an attachment* — which at a target admitting
/// only conforming attachments is the same act, and the report says it was derived because it
/// was. `doc/rfc/0007` section 4.6's table is the argument, and wiring the two together is what
/// keeps the second from being a second implementation of the first.
fn answers_by_derivation(row: &Row) -> bool {
    if !DERIVABLE.contains(&row.site.as_str()) {
        return false;
    }
    row.remedy == Kind::Derive
        || (row.remedy == Kind::Preserve && row.placement == Some(Placement::Attach))
}

fn requirement_binds(id: &str, target: Target) -> bool {
    table::requirements().any(|requirement| requirement.id == id && requirement.binds(target))
}

/// Every embedded file the departure predicate is asked about: its name, media type and
/// relationship.
///
/// The population is ISO 19005-2 section 6.8's — every file specification dictionary carrying an
/// `/EF`. The media type is the embedded stream's own `/Subtype`, which §7.11.4.1's Table 44 makes a MIME
/// media type; the relationship is §7.11.3's Table 43 `/AFRelationship`, defaulting to
/// `Unspecified`. Walking every object with an `/EF` mirrors what the validator's predicate does,
/// so *only XML* means only XML by the same reckoning the requirement was failed on.
fn embedded_files(document: &Document) -> Vec<(String, Option<String>, String)> {
    let mut out = Vec::new();
    for number in document.xref().object_numbers() {
        let object = document.get(ObjectId::new(number, 0));
        let Some(specification) = object.as_dict() else {
            continue;
        };
        let files = document.get_key(specification, "EF");
        let Some(files) = files.as_dict() else {
            continue;
        };
        let name = specification_name(document, specification);
        let media_type = ["F", "UF", "DOS", "Mac", "Unix"]
            .into_iter()
            .find_map(|key| {
                let stream = document.get_key(files, key);
                let stream = stream.as_stream()?;
                let subtype = document.get_key(&stream.dict, "Subtype");
                subtype.as_name()?.as_str().map(str::to_owned)
            });
        let relationship = document
            .get_key(specification, "AFRelationship")
            .as_name()
            .and_then(|name| name.as_str().map(str::to_owned))
            .unwrap_or_else(|| "Unspecified".to_owned());
        out.push((name, media_type, relationship));
    }
    out
}

/// A file specification's own name, for a report a person reads.
///
/// §7.11.3's Table 43 makes `/UF` and `/F` the same file name written twice in two types, and the
/// Unicode one is preferred where the file states both — `pdf_syntax::text_string` is §7.9.2.2's
/// decoding. Shared with [`super::remedies`], which needs the same name for the same file.
pub(super) fn specification_name(
    document: &Document,
    specification: &pdf_syntax::object::Dictionary,
) -> String {
    for key in ["UF", "F"] {
        if let Some(bytes) = document.get_key(specification, key).as_string() {
            return pdf_syntax::text_string(bytes);
        }
    }
    "an unnamed embedded file".to_owned()
}

use super::decision::Authorisations;

/// One refusal site a configuration may answer, for `--remedy-sites --to <target>`.
///
/// `doc/rfc/0007` section 3.1: every site is enumerable, printed *from the same table the converter
/// decides from* so a site cannot exist undocumented and a configuration naming one that does not
/// exist is an error rather than an ignored line.
#[derive(Debug, Clone, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "one flag per remedy a configuration may answer this site with, and they are \
              independent of each other: a site can take a discard and a page, or a tool and a \
              departure. An enumeration would say they were exclusive, which is the thing \
              doc/rfc/0007 section 3.1's listing must not say"
)]
pub struct Site {
    /// The requirement identifier — the site's key in a configuration.
    pub requirement: &'static str,
    /// Its clause, as cited for the target asked for.
    pub citation: String,
    /// The remedy a built `discard` at this site authorises, where one is built.
    pub built_discard: Option<Loss>,
    /// Whether this site is one of the two the XML departure may name.
    pub departable: bool,
    /// Whether a configuration may answer this site with `derive`, which runs an external program.
    ///
    /// **The flag `--remedy-sites` prints [`UNTRUSTED_INPUT_WARNING`] beside**, which is
    /// `doc/questions/A56`'s answer: the warning lives where an operator configures a tool rather
    /// than in a security document nobody opens, and the two places an operator meets a tool are
    /// the shipped profiles' `[tool.…]` blocks and this list.
    pub takes_a_tool: bool,
    /// Whether a configuration may answer this site with `supply` — the operator's own fact.
    pub takes_a_supplied_fact: bool,
    /// Whether a configuration may answer this site with `preserve` by an appended page.
    ///
    /// `doc/adr/1014`'s amendment, and the listing says so because `doc/rfc/0007` section 3.1
    /// makes enumerability a gate: a remedy a configuration may name and this listing does not
    /// mention is a site that exists undocumented.
    pub takes_a_page: bool,
}

/// What an operator is agreeing to when they declare a `[tool.…]` block.
///
/// **`doc/questions/A56`, in the owner's chosen shape.** Of the options put, the owner chose *no
/// offer in the first version, warn at the configuration site*: this project cannot confine
/// somebody else's program to the standard `CLAUDE.md` principle 3 holds its own renderer to —
/// `LibreOffice` will not run under our seccomp profile — and a confinement profile written against
/// no particular program is a guess. So the honest thing is the sentence, in the place where the
/// decision is actually made.
///
/// It is printed by `--remedy-sites` for every site that takes a tool, and written above every
/// `[tool.…]` block in `doc/profiles/`.
pub const UNTRUSTED_INPUT_WARNING: &str =
    "this runs a program you chose, on a document you did not write";

/// Every refusal site the target binds, in clause order, for the enumeration.
///
/// A refusal site is a requirement the converter answers with a refusal or an authorised loss —
/// the ones a configuration has anything to say about. A requirement the writer meets by
/// construction, or that is already answered without a choice, is not a site: there is no refusal
/// to configure. The list is generated from [`super::census`], so it is exactly the table the
/// converter decides from.
#[must_use]
pub fn sites(target: Target) -> Vec<Site> {
    use super::census::{Kind as Standing, Standing as Row};
    let losses = loss_sites();
    super::census::census(target)
        .filter_map(|(requirement, standing)| {
            let is_site = matches!(standing, Row::Refused(_) | Row::Remedy(Standing::Loses));
            if !is_site {
                return None;
            }
            Some(Site {
                requirement: requirement.id,
                citation: requirement
                    .clauses
                    .citation(target)
                    .unwrap_or_else(|| requirement.id.to_owned()),
                built_discard: losses.get(requirement.id).copied(),
                departable: Departure::SUPPORTED.contains(&requirement.id),
                takes_a_tool: DERIVABLE.contains(&requirement.id),
                takes_a_supplied_fact: SUPPLIABLE.contains(&requirement.id),
                takes_a_page: PRESERVABLE_BY_PAGE.contains(&requirement.id),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdf_archive::{Flavour, Level};

    const TWO_B: Target = Target::Two(Level::B);

    #[test]
    fn a_discard_at_a_loss_site_becomes_the_authorisation_the_flag_would() {
        let text = "\
[site.\"graphics/image-interpolation-is-off\"]
remedy = \"discard\"
";
        let config = Configuration::read(text, TWO_B).expect("it reads");
        let authorised = config.authorisations(TWO_B);
        assert!(authorised.grants(Loss::ImageSmoothing));
        assert!(config.unbuilt(TWO_B).is_empty());
    }

    #[test]
    fn a_discard_at_an_unbuilt_site_authorises_nothing_and_is_named() {
        let text = "\
[site.\"file-structure/no-encryption\"]
remedy = \"discard\"
";
        let config = Configuration::read(text, TWO_B).expect("it reads");
        assert_eq!(config.authorisations(TWO_B), Authorisations::default());
        let unbuilt = config.unbuilt(TWO_B);
        assert_eq!(unbuilt.len(), 1);
        assert_eq!(unbuilt[0].site, "file-structure/no-encryption");
    }

    #[test]
    fn an_unknown_site_is_an_error_naming_it() {
        let text = "[site.\"graphics/not-a-requirement\"]\nremedy = \"discard\"\n";
        let error = Configuration::read(text, TWO_B).expect_err("unknown site");
        assert!(matches!(error, ConfigError::UnknownSite { .. }), "{error}");
    }

    #[test]
    fn an_unknown_remedy_is_an_error_naming_it() {
        let text = "[site.\"file-structure/no-encryption\"]\nremedy = \"ignore\"\n";
        let error = Configuration::read(text, TWO_B).expect_err("unknown remedy");
        assert!(
            matches!(error, ConfigError::UnknownRemedy { .. }),
            "{error}"
        );
    }

    #[test]
    fn an_on_failure_chain_is_refused_per_a57() {
        let text = "\
[site.\"annotations/three-dimensional-stream-format\"]
remedy = \"derive\"
on-failure = [\"preserve\", \"stop\"]
";
        let error = Configuration::read(text, Target::Four(Flavour::E)).expect_err("a chain");
        assert!(
            matches!(error, ConfigError::FallbackChain { .. }),
            "{error}"
        );
    }

    #[test]
    fn a_target_qualifier_decides_which_row_applies() {
        let text = "\
[site.\"embedded-files/embedded-file-is-itself-pdfa-in-the-plain-profile\"]
remedy = \"discard\"

[site.\"embedded-files/embedded-file-is-itself-pdfa-in-the-plain-profile\".target.\"4f\"]
remedy = \"preserve\"
";
        // At PDF/A-4 the unqualified `discard` row applies; the 4f row is read but does not.
        let four = Configuration::read(text, Target::Four(Flavour::Plain)).expect("reads");
        assert_eq!(four.unbuilt(Target::Four(Flavour::Plain)).len(), 1);
    }

    #[test]
    fn a_shape_qualifier_reads_and_is_inert_until_a_shape_aware_remedy_exists() {
        // `doc/pdf-a-mitigations.md` section 14's first finding: a site is finer than a requirement.
        // The format carries the shape qualifier so a configuration can be written against it; a
        // shape-qualified row is inert this round, like a row for another target.
        let text = "\
[site.\"graphics/graphics-state-blend-modes-are-defined\".shape.\"array\"]
remedy = \"discard\"
";
        let config = Configuration::read(text, TWO_B).expect("a shape qualifier reads");
        assert!(
            config.unbuilt(TWO_B).is_empty(),
            "a shape-qualified row does not apply yet"
        );
    }

    #[test]
    fn an_unknown_header_marker_is_an_error() {
        let text = "[site.\"file-structure/no-encryption\".flavour.\"x\"]\nremedy = \"discard\"\n";
        let error = Configuration::read(text, TWO_B).expect_err("unknown marker");
        assert!(
            matches!(error, ConfigError::UnknownTargetQualifier { .. }),
            "{error}"
        );
    }

    #[test]
    fn a_departure_without_a_reason_is_refused() {
        let text = "\
[depart.\"embedded-files/embedded-file-is-itself-pdfa\"]
media-type = [\"application/xml\"]
";
        let error = Configuration::read(text, TWO_B).expect_err("no reason");
        assert!(
            matches!(error, ConfigError::DepartureWithoutReason { .. }),
            "{error}"
        );
    }

    #[test]
    fn the_xml_departure_reads_and_binds_part_two() {
        let text = "\
[depart.\"embedded-files/embedded-file-is-itself-pdfa\"]
media-type = [\"application/xml\", \"text/xml\"]
relationship = [\"Alternative\"]
reason = \"Factur-X invoices; our archive accepts them\"
";
        let config = Configuration::read(text, TWO_B).expect("reads");
        let built = config.built_departures(TWO_B);
        assert_eq!(built.len(), 1);
        assert_eq!(built[0].media_types, vec!["application/xml", "text/xml"]);
        // At PDF/A-4f the embedding rule is lifted, so the departure does not apply.
        assert!(config.built_departures(Target::Four(Flavour::F)).is_empty());
    }

    #[test]
    fn a_derive_naming_only_the_site_is_refused_by_name() {
        // **`doc/questions/A55`'s guardrail, as a test.** The owner's answer makes `derive`
        // unreachable "without the configuration naming the site *and* the tool": a site alone
        // states half an instruction, and the half it states is *make something that was not in
        // the document*. So it is an error naming the site rather than a remedy taken on faith.
        let text = "\
[site.\"embedded-files/embedded-file-is-itself-pdfa-in-the-plain-profile\"]
remedy = \"derive\"
";
        let error = Configuration::read(text, Target::Four(Flavour::Plain))
            .expect_err("a derive with no tool");
        let ConfigError::DeriveWithoutTool { site, .. } = &error else {
            panic!("the site is named: {error}");
        };
        assert_eq!(
            site,
            "embedded-files/embedded-file-is-itself-pdfa-in-the-plain-profile"
        );
        assert!(error.to_string().contains("A55"), "{error}");
    }

    #[test]
    fn a_site_naming_a_tool_no_block_declares_is_refused() {
        let text = "\
[site.\"embedded-files/embedded-file-is-itself-pdfa-in-the-plain-profile\"]
remedy = \"derive\"
tool = \"office-to-pdf\"
";
        let error = Configuration::read(text, Target::Four(Flavour::Plain))
            .expect_err("an undeclared tool");
        assert!(
            matches!(error, ConfigError::UndeclaredTool { .. }),
            "{error}"
        );
    }

    #[test]
    fn a_tool_declaration_reads_and_the_keys_with_no_default_are_required() {
        let text = "\
[site.\"embedded-files/embedded-file-is-itself-pdfa-in-the-plain-profile\"]
remedy = \"derive\"
tool = \"office-to-pdf\"

[tool.office-to-pdf]
program = \"/usr/bin/soffice\"
args = [\"--headless\", \"--convert-to\", \"pdf\", \"--outdir\", \"{out}\", \"{in}\"]
expects = \"application/pdf\"
timeout = \"60s\"
output-limit = \"256MiB\"
";
        let config =
            Configuration::read(text, Target::Four(Flavour::Plain)).expect("the declaration reads");
        let derivations = config.derivations(Target::Four(Flavour::Plain));
        assert_eq!(derivations.len(), 1);
        let tool = &derivations[0].tool;
        assert_eq!(tool.bounds.timeout, Duration::from_mins(1));
        assert_eq!(tool.bounds.output_limit, 256 << 20);
        // A tool given `{out}` writes there; one that is not writes to standard output.
        assert_eq!(tool.delivery(), crate::tool::Delivery::OutputDirectory);
        assert!(config.unbuilt(Target::Four(Flavour::Plain)).is_empty());

        for missing in ["program", "expects", "timeout", "output-limit"] {
            let without: String = text
                .lines()
                .filter(|line| !line.starts_with(missing))
                .collect::<Vec<_>>()
                .join("\n");
            let error = Configuration::read(&without, Target::Four(Flavour::Plain))
                .expect_err("a key with no default");
            assert!(
                matches!(error, ConfigError::ToolWithout { key, .. } if key == missing),
                "{missing}: {error}"
            );
        }
    }

    #[test]
    fn a_placeholder_outside_the_vocabulary_is_refused_and_names_the_one_left_out() {
        let text = "\
[site.\"embedded-files/embedded-file-is-itself-pdfa-in-the-plain-profile\"]
remedy = \"derive\"
tool = \"t\"

[tool.t]
program = \"/bin/true\"
args = [\"{media-type}\"]
expects = \"application/pdf\"
timeout = \"1s\"
output-limit = \"1MiB\"
";
        let error = Configuration::read(text, Target::Four(Flavour::Plain))
            .expect_err("an unknown placeholder");
        let ConfigError::UnknownPlaceholder { placeholder, .. } = &error else {
            panic!("{error}");
        };
        assert_eq!(placeholder, "{media-type}");
        // The sentence says why that one in particular is not substituted: the only media type
        // available at a site running a tool is the document's own, and section 4.1 keeps
        // document-derived strings out of args.
        assert!(error.to_string().contains("document-derived"), "{error}");
    }

    #[test]
    fn a_supply_whose_table_is_empty_stops_rather_than_erroring() {
        // `doc/profiles/keep-everything.toml` writes `role-map = { }` with the note *an empty map
        // stops*, and the same reading holds here: an operator who has not filled the table in has
        // stated no fact, so the site keeps its refusal and the configuration still loads.
        let text = "[site.\"embedded-files/associated-file-media-type\"]\nremedy = \"supply\"\n";
        let target = Target::Four(Flavour::F);
        let config = Configuration::read(text, target).expect("it reads");
        assert!(config.supplies(target).is_empty());
        assert_eq!(config.unbuilt(target).len(), 1);
    }

    #[test]
    fn a_derive_whose_tool_promises_something_other_than_a_pdf_is_refused() {
        let text = "\
[site.\"embedded-files/embedded-file-is-itself-pdfa-in-the-plain-profile\"]
remedy = \"derive\"
tool = \"t\"

[tool.t]
program = \"/bin/true\"
args = []
expects = \"text/csv\"
timeout = \"1s\"
output-limit = \"1MiB\"
";
        let error = Configuration::read(text, Target::Four(Flavour::Plain))
            .expect_err("a tool that cannot answer the requirement");
        assert!(
            matches!(error, ConfigError::NotBuiltThatWay { .. }),
            "{error}"
        );
    }

    #[test]
    fn a_departure_this_round_cannot_carry_is_an_error() {
        let text = "\
[depart.\"file-structure/no-encryption\"]
reason = \"we accept it\"
";
        let error = Configuration::read(text, TWO_B).expect_err("unsupported departure");
        assert!(
            matches!(error, ConfigError::UnsupportedDeparture { .. }),
            "{error}"
        );
    }
}
