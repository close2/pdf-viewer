//! One walk of a document's content, answering the questions several requirements share.
//!
//! # Why one walk rather than several
//!
//! A handful of ISO 19005 requirements are not about a dictionary at all. They are about what a
//! page's content *does*: which font a text-showing operator ran with, which colour space was in
//! force when a colour was set, whether anything on the page was involved in a transparency
//! operation. Every one of those needs the same traversal — every page's `/Contents`, the form
//! `XObject`s it invokes, the patterns it paints with, the glyph procedures of the Type 3 fonts
//! it selects, and the appearance streams of its annotations — carrying the same two pieces of
//! context: the resource dictionary in force, and the graphics state.
//!
//! So the traversal is written once, here, and the requirements read its product. The
//! alternative — a walk per rule — would have each rule re-deriving the resource dictionary in
//! force, and the rules would drift apart in exactly the place they must agree.
//!
//! # What is walked, and what is deliberately not
//!
//! Walked: each page's content streams; the form `XObject`s reached through `Do`; the tiling
//! patterns selected through `scn`/`SCN`; the glyph procedures of a Type 3 font a show operator
//! ran with; the appearance streams under a page annotation's `/AP`.
//!
//! Not walked: the group of a soft mask, the image an `SMask` entry names, and the content of a
//! shading's function. A colour reached only through one of those is not reported, which is this
//! crate's standing direction of error — **under-report rather than mis-report**, because a
//! requirement that invents a failure tells a user their conforming file does not conform.
//!
//! The soft-mask image is the one of the three the standard has since answered, and a round that
//! widens the walk to it has to bring the answer along: `TechNote 0010` A026 resolves that
//! `DeviceGray` as the `ColorSpace` of a soft-mask image dictionary needs neither a default space
//! nor an output intent, so reaching one without exempting it would turn today's silence into a
//! false failure. `crate::table::graphics`'s `device_gray_under_part_two` carries the reading.
//!
//! # What it costs, and where the fix landed
//!
//! **A survey is one pass over the document's content, and it used to be walked once per
//! requirement that read one.** `examples/survey_cost` measured that: ISO 32000-2's own
//! specification — 1 023 pages, 8 029 592 content-stream tokens — cost 574 ms to read the tokens
//! of and 1.0 s to survey, and a whole PDF/A-4 report over it took 19 s because thirteen of its
//! requirements each asked for a survey of their own.
//!
//! **That fix has been made, and it is [`crate::Examination`]**: a predicate is handed the
//! examination rather than the document, and the survey is an `OnceCell` on it, so a report
//! walks the content once however many of its requirements read it. `examples/cost` prints the
//! survey as its own line for that reason. Caching on the document instead was considered and
//! rejected: a `&Document` has no identity a cache can key on that a later document cannot
//! reuse, and a validator that answers about the wrong file is worse than a slow one.
//!
//! What *is* still done here is to keep the pass itself honest. Observations are deduplicated
//! as they are made — the same specification selects a device colour space 317 127 times and
//! says 2 565 distinct things by doing so, because [`Where`] names a page rather than an
//! operator — and
//! §8.6.5.6's defaults are resolved once per content stream rather than once per colour operator,
//! since the resource dictionary in force cannot change within one.
//!
//! # What the marked-content stack costs, and what it bought
//!
//! [`Survey::of`] carries §14.6.1's open sequences on a stack, reads each `BDC` operand as
//! §14.6.2's property list, and records against every string a text-showing operator drew what
//! stood between it and §14.9.4's replacement text ([`Replacement`]). That is the one thing here
//! that reads a construction the walk used to step over, so it was measured with
//! `examples/cost` on ISO 32000-2's own specification — 1 023 pages and 8 029 592
//! content-stream tokens.
//!
//! **About 6%, and the ratio is what to trust rather than either figure.** Back to back in one
//! session, fastest of five runs each, the survey went from 547 ms to 580 ms; the same
//! comparison earlier the same day, on a quieter machine and fastest of three, put it at 521 ms
//! and 596 ms. The spread between sessions is twice the difference either measured, which is
//! the honest thing to write down rather than the one number that flattered the change.
//!
//! The 6% divides in half, measured by taking each side out on the quieter machine: about
//! 36 ms of the 75 was building the property lists
//! (`pdf_model::content::reader::inline_dictionary`, which is the interpreter's own reader of
//! that construction rather than a second one), and about 40 ms was the walk reading the
//! operands *inside* them for ISO 19005-2 section 6.1.13's limits — which the token walk used
//! to see for itself and would otherwise have stopped seeing. Neither half is optional: the
//! first is the only route to a `BDC` property list, and dropping the second would have
//! narrowed one rule silently while widening another.
//!
//! What it bought: the three corpus documents this crate had been missing under
//! ISO 19005-2 section 6.2.11.7.3 and section 6.7.4, and — because the property list is now
//! read rather than guessed at — an `/ActualText` value that is no longer paired with its name
//! by adjacency alone.
//!
//! # What the three per-token measurements cost, and what they bought
//!
//! [`ContentLiterals`], [`Survey::deepest_graphics_state_nesting`],
//! [`Survey::unlisted_operators`] and [`Survey::inline_actual_texts`] are the only things here
//! that touch *every* token rather than every observation, so they were measured before and after
//! with `examples/cost` on ISO 32000-2's own specification — 1 023 pages and 8 029 592
//! content-stream tokens, the largest document this tree holds. **The fastest of ten runs was
//! 640 ms before and 643 ms after**, so what they add — five comparisons on each operand and
//! thirty-nine more arms on a `match` the walk already ran — is below what this instrument can
//! resolve on this machine. It is not free; it is smaller than the run-to-run spread, which is
//! the honest thing to record rather than a figure the noise would have invented either way.
//!
//! What it bought, and this is the part worth reading twice:
//!
//! - Three rows that were `Unchecked` — ISO 19005-2 section 6.1.13's limits on the values written
//!   *inside* a content stream, its `q`/`Q` nesting limit, and section 6.2.2's ban on an operator
//!   the base standard does not define — and eleven corpus documents this crate had been missing.
//!   The three predicates that read them cost 40 ns, 360 ns and 500 ns on that same document,
//!   because all the work is here and none of it is repeated.
//! - **A requirement that had been decoding these streams a second time stopped.**
//!   `fonts/actual-text-states-no-private-use` needs ISO 32000-2 §14.9.4's entry where a producer
//!   wrote it into a `BDC` operator's operands, which is no object; it was decoding and lexing
//!   every content stream again to find one, at 118 ms on top of its object walk, and was the
//!   dearest single requirement in a whole part 4 report. Reading the two fields here instead
//!   took it to 44 ms and out of the ten dearest altogether.
//!
//! That second one is the pattern to reach for. **A rule that decodes a content stream is a rule
//! that should be reading this file**, and the arithmetic is one-sided: a field costs one pass a
//! few nanoseconds a token, and a second walk costs a whole pass per requirement that wants one.
//!
//! # The bounds, and why they are here
//!
//! A content stream is untrusted input. Three bounds keep a hostile document from turning this
//! walk into an unbounded one: a token budget, a form nesting depth, and a cap on how many
//! streams one walk opens. Reaching any of them stops the walk, which under-reports in the same
//! direction as everything else.

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use pdf_model::Pages;
use pdf_model::content::reader::{ContentReader, LOOKAHEAD, NestedContent, WINDOW};
use pdf_model::page::Page;
use pdf_syntax::{Dictionary, Document, Name, Object, ObjectId, Stream, Token};

use crate::finding::Where;

/// How many content-stream tokens one document's walk may read.
///
/// `examples/token_window_census` counted 225 775 555 content-stream tokens across 39 976
/// documents, so this is four orders of magnitude above what an ordinary document costs and is
/// here to stop a hostile one, not a large one.
const TOKEN_BUDGET: u64 = 20_000_000;

/// How deep a chain of form `XObject`s, patterns and glyph procedures is followed.
const MAX_FORM_DEPTH: u32 = 8;

/// How many nested content streams one document's walk may open.
const MAX_STREAMS: usize = 4096;

/// How deep a `q` stack is kept. ISO 19005-2 section 6.1.13 limits a conforming file to 28.
const MAX_NESTING: usize = 512;

/// How deep a colour space array may nest before [`classify`] gives up.
const MAX_SPACE_DEPTH: usize = 16;

/// How many bytes of distinct shown strings one survey keeps, across every font in it.
///
/// [`SelectedFont::shown`] is what the two rules about a *code* rest on — ISO 19005-2 section
/// 6.2.11.8 and section 6.2.11.4.1, and their part 4 numbers — and it is the one thing this walk
/// keeps that a document can make large honestly rather than only maliciously. So it is bounded in
/// bytes, and a font whose strings did not fit says so ([`SelectedFont::shown_complete`]) instead
/// of being reported on a prefix: a rule that asked "does any code reach `.notdef`" of half a
/// page's text would answer *no* about a page it had not finished reading.
///
/// Four mebibytes is two orders of magnitude above what an ordinary document's *distinct*
/// strings come to — they are deduplicated, and a page of prose repeats itself heavily — and it
/// is a fixed ceiling rather than a per-font one so that no count of fonts can multiply it.
///
/// **What keeping them costs**, measured with `examples/cost` on ISO 32000-2's own specification
/// — 1 023 pages, 8 029 592 content-stream tokens, and the largest document this tree holds: the
/// survey goes from about 480 ms to about 660 ms, which is 5% of that document's whole PDF/A-4
/// report. It is a ceiling that file reaches, so its later fonts are marked incomplete and the
/// two rules that need every code stay silent about them — which is the bound doing its job
/// rather than failing at it.
const SHOWN_BUDGET: usize = 4 << 20;

/// How many bytes of distinct `/ActualText` strings one survey keeps.
///
/// ISO 32000-2 §14.9.4's entry is a text string a producer writes, so it is the second thing
/// here a document can make large honestly rather than only maliciously. An entry that does not
/// fit is **dropped whole rather than truncated**, and that is the point of the bound rather
/// than an implementation detail: a text string is UTF-16BE, and half of one can split a
/// surrogate pair into a character the file never states — which the rule that reads these would
/// then report as a private-use character. Under-reporting is this crate's direction of error;
/// inventing a code point is not.
///
/// A quarter of [`SHOWN_BUDGET`], because these are entries a producer writes by hand where
/// those are every string a page draws.
const ACTUAL_TEXT_BUDGET: usize = 1 << 20;

/// How deep a dictionary a content stream wrote inline is read for its operands.
///
/// `pdf_model::content::reader::inline_dictionary` bounds what it *builds*; this bounds what
/// this walk then reads out of it, and the two are separate numbers because they answer to
/// separate callers.
const MAX_INLINE_DEPTH: u32 = 8;

/// How many marked-content sequences one walk keeps open at once.
///
/// §14.6.1 makes `BMC`/`BDC` … `EMC` nest, and a hostile stream can open them and never close
/// one. A sequence opened past this is read for its property list like any other and then
/// forgotten, which loses a coverage that would have made a rule *quieter* — so the bound
/// under-reports in the same direction as everything else here.
const MAX_MARK_DEPTH: usize = 64;

/// How many marked-content identifiers one shown string keeps.
///
/// [`Replacement::identifiers`] is a list the rule reading it must have **whole**: it stays
/// silent where any one of them reaches §14.9.4's entry, so a prefix would make it speak where
/// the entry it did not keep was the covering one. A string shown under more sequences than
/// this therefore records [`Replacement::elided`] and is not reported at all.
const MAX_IDENTIFIERS: usize = 8;

/// How many *distinct* observations of one kind a survey keeps.
///
/// The lists below are deduplicated, so a page that sets `DeviceRGB` ten thousand times under
/// one resource dictionary contributes one entry — which is all a report can point at, since
/// [`Where`] names a page and not an operator. This cap is the second bound, for a document
/// that manufactures distinct observations rather than repeated ones. ISO 32000-2 has 1 023
/// pages and 1 018 transparency groups, so it is two orders of magnitude above a large
/// document's honest need.
const MAX_OBSERVATIONS: usize = 100_000;

/// One of ISO 32000-2 §8.6.4's three device colour spaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DeviceFamily {
    /// `DeviceGray`, one component.
    Gray,
    /// `DeviceRGB`, three components.
    Rgb,
    /// `DeviceCMYK`, four components.
    Cmyk,
}

impl DeviceFamily {
    /// The name a report prints for this family.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Gray => "DeviceGray",
            Self::Rgb => "DeviceRGB",
            Self::Cmyk => "DeviceCMYK",
        }
    }

    /// The `/DefaultGray`, `/DefaultRGB` or `/DefaultCMYK` key §8.6.5.6 pairs with this family.
    #[must_use]
    pub const fn default_key(self) -> &'static str {
        match self {
            Self::Gray => "DefaultGray",
            Self::Rgb => "DefaultRGB",
            Self::Cmyk => "DefaultCMYK",
        }
    }

    /// Where this family sits in a fixed array of three, for the defaults a context carries.
    const fn index(self) -> usize {
        match self {
            Self::Gray => 0,
            Self::Rgb => 1,
            Self::Cmyk => 2,
        }
    }

    /// The four-character ICC colour space signature a destination profile of this family has.
    #[must_use]
    pub const fn profile_signature(self) -> &'static [u8] {
        match self {
            Self::Gray => b"GRAY",
            Self::Rgb => b"RGB ",
            Self::Cmyk => b"CMYK",
        }
    }
}

/// What a colour space is, as far as ISO 19005's colour subclauses need to tell them apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SpaceKind {
    /// One of the three device colour spaces, which ISO 19005 section 6.2.4.3 restricts.
    Device(DeviceFamily),
    /// A CIE-based space — `CalGray`, `CalRGB`, `Lab` or `ICCBased` — with the device family
    /// its component count corresponds to, where it has one.
    ///
    /// `Lab` carries `None`: it has three components and is not an RGB-based space, and
    /// ISO 32000-2 §11.3.4 excludes it from being a blending colour space at all.
    Independent(Option<DeviceFamily>),
    /// A `Separation`, `DeviceN` or `NChannel` space, whose components are colourants.
    Colourant,
    /// A `Pattern` space with no underlying space, or bytes this crate could not read as one.
    Unknown,
}

impl SpaceKind {
    /// Whether this space is device-independent, which is what every licence in section 6.2.4.3
    /// turns on.
    #[must_use]
    pub const fn is_independent(self) -> bool {
        matches!(self, Self::Independent(_))
    }

    /// Whether this space is a device-independent one whose components are of `family`.
    #[must_use]
    pub fn is_independent_family(self, family: DeviceFamily) -> bool {
        self == Self::Independent(Some(family))
    }
}

/// How a device colour space was reached from the space a content stream actually named.
///
/// The three ISO 19005 subclauses that restrict colour do not restrict the same population. section
/// 6.2.4.3 is about the device space itself, wherever it turns up; section 6.2.4.4 is about the
/// alternate space of a `Separation` or `DeviceN`; section 6.2.4.5 is about the space underlying an
/// `Indexed` or a `Pattern`. One walk finds all three, so each use carries the route that says
/// which subclauses it answers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Route {
    /// The content named the device colour space itself.
    Direct,
    /// It is the base of an `Indexed` space or the underlying space of a `Pattern`: section
    /// 6.2.4.5.
    Underlying,
    /// It is the alternate space of a `Separation` or `DeviceN` space: section 6.2.4.4.
    Alternate,
}

impl Route {
    /// The route reached by descending into an `Indexed` base or a `Pattern`'s underlying space.
    ///
    /// An alternate space stays an alternate space: descending further does not stop section
    /// 6.2.4.4 from being the clause that put the restriction there.
    const fn under(self) -> Self {
        match self {
            Self::Alternate => Self::Alternate,
            Self::Direct | Self::Underlying => Self::Underlying,
        }
    }
}

/// §8.6.5.6's default colour space for one family, under each part's reading of that clause.
///
/// Two readings rather than one, because `TechNote 0010` A028 names ISO 19005-2 and ISO 19005-3
/// and does not name ISO 19005-4. The working group resolved that parts 2 and 3 are read as if
/// section 6.2.2 required any default colour space to be defined in the *explicitly associated*
/// resources dictionary, and as if a processor ignored what that dictionary does not define.
/// Part 4's own section 6.2.2 states the resource sentences in its own words and says nothing
/// about defaults, so it keeps the base standard's reading, where a form `XObject` with no
/// `Resources` entry of its own is read against the dictionary it falls back on.
///
/// `crate::clarification` carries the record; `crate::table::graphics` is where each part picks
/// the field its own clause reads.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct DefaultSpace {
    /// What the resources in force name, which is what ISO 19005-4 reads.
    pub in_force: Option<SpaceKind>,
    /// The same, where those resources are the stream's own explicitly associated dictionary.
    ///
    /// `None` where the stream states no `Resources` entry of its own — there is then no
    /// explicitly associated dictionary at all, so A028 leaves no default in force.
    pub explicit: Option<SpaceKind>,
}

impl DefaultSpace {
    /// The default a stream reads, given whether the dictionary it was read from was its own.
    const fn read(in_force: Option<SpaceKind>, explicitly_associated: bool) -> Self {
        Self {
            in_force,
            explicit: if explicitly_associated {
                in_force
            } else {
                None
            },
        }
    }
}

/// One place a content stream selected a device colour space, with what licensed it there.
///
/// The licences are recorded rather than judged, because the two owned parts license the same
/// use differently: ISO 19005-2 section 6.2.4.3 admits a default space or the output intent, and
/// ISO 19005-4 section 6.2.4.3 admits the current transparency blending space as well.
#[derive(Debug, Clone)]
pub struct DeviceColour {
    /// Which device colour space was selected.
    pub family: DeviceFamily,
    /// The zero-based index of the page whose content selected it.
    pub page: usize,
    /// Where a report points a reader.
    pub place: Where,
    /// What kind of site selected it, for the sentence a finding prints.
    pub what: &'static str,
    /// How it was reached from the space the content named.
    pub via: Route,
    /// §8.6.5.6's matching default colour space, under each part's reading of that clause.
    pub default: DefaultSpace,
    /// ISO 32000-2 §11.3.4's current transparency blending colour space, where there is one.
    pub blending: Option<SpaceKind>,
}

/// One transparency group's `CS` entry, which ISO 19005 makes subject to the colour subclauses.
#[derive(Debug, Clone)]
pub struct GroupSpace {
    /// What the `CS` entry names.
    pub kind: SpaceKind,
    /// The zero-based index of the page the group is on or is painted onto.
    pub page: usize,
    /// Where a report points a reader.
    pub place: Where,
    /// §8.6.5.6's matching default colour space, where the entry names a device space.
    pub default: DefaultSpace,
}

/// A resource a content stream named that the resource dictionary in force does not define.
///
/// ISO 19005-4 section 6.2.2 requires the associated resource dictionary to define every named
/// resource its content stream references. ISO 19005-2 states no such sentence in its own text —
/// and binds it anyway, because `TechNote 0010` A002 is the working group resolving that parts 2
/// and 3 are read as if it did. So both parts' rows read this; `crate::table::graphics`'s
/// `named_resources_are_defined` carries the argument.
#[derive(Debug, Clone)]
pub struct MissingResource {
    /// The zero-based index of the page whose content named it.
    pub page: usize,
    /// The subdictionary of the resources it should have been in — `Font`, `XObject`, and so on.
    pub category: &'static str,
    /// The name the content stream used.
    pub name: String,
    /// What kind of stream named it, for the sentence a finding prints.
    pub what: &'static str,
}

/// One `/Lang` a marked-content property list stated, with the page that stated it.
///
/// ISO 19005-2 section 6.7.4 binds a `/Lang` in three places — the catalog, a structure element
/// and a property list — and only this one is written inside a content stream. A property list
/// named through the resources (ISO 32000-2 §14.6.2) is an object as well, and is recorded here
/// too: the requirement that reads this walks the *structure tree* for the other two places, so
/// a `/Properties` entry is reached by nothing else.
#[derive(Debug, Clone)]
pub struct MarkedLanguage {
    /// The zero-based index of the page whose content stated it.
    pub page: usize,
    /// The value as the property list wrote it, or the name of its type where it is not a text
    /// string — §14.9.2.2 makes a language identifier a text string, so that is a failure of
    /// its own and one a report has to be able to describe.
    pub value: Result<Vec<u8>, &'static str>,
}

/// One content stream the walk opened, and the two facts ISO 19005 section 6.2.2 turns on.
///
/// The clause requires a content stream that references other objects to have a resource
/// dictionary *explicitly* associated with it, and ISO 32000-2 §7.8.3 is where "associated"
/// is defined: a page's content stream is associated with the dictionary the page dictionary's
/// `Resources` entry designates **or one it inherits**, while a form `XObject`, a pattern, an
/// annotation appearance and a Type 3 font's glyph procedures are each required to carry the
/// entry themselves. So the two facts are: did this stream name a resource at all, and was the
/// dictionary it was read against stated on it rather than inherited.
#[derive(Debug, Clone)]
pub struct OpenedStream {
    /// The zero-based index of the page whose rendering reached it.
    pub page: usize,
    /// The object it is, where the walk reached it as one.
    ///
    /// `None` for a page's own `/Contents`, which §7.7.3.3 lets be an array of streams and which
    /// is therefore not one object. Every other content stream a walk opens — a form `XObject`,
    /// a tiling pattern, a Type 3 glyph procedure, an annotation appearance — is a stream
    /// object, and this is how a rule that wants to read the bytes again finds them without
    /// walking the resource dictionaries a second time.
    pub object: Option<ObjectId>,
    /// Where a report points a reader.
    pub place: Where,
    /// What kind of content stream it is, for the sentence a finding prints.
    pub what: &'static str,
    /// Whether it named at least one resource, whether or not the resources defined it.
    pub referenced: bool,
    /// Whether the resource dictionary it was read against was its own rather than inherited.
    pub own_resources: bool,
}

/// The profile stream of an `ICCBased` colour space: ISO 32000-2 §8.6.5.5, Table 66.
#[derive(Debug, Clone)]
pub struct IccProfile {
    /// The object the profile stream is, where the colour space array reaches it by reference.
    ///
    /// ISO 19005-4 section 6.2.4.2 makes two profiles identical when the colour space and the
    /// output intent reach the same embedded stream by indirect reference, so the reference itself
    /// — not what it resolves to — is one of the two things this carries.
    pub id: Option<ObjectId>,
    /// The stream, so a profile written directly into the array is comparable too.
    pub stream: Arc<Stream>,
    /// Which device family the stream dictionary's `N` corresponds to: §8.6.5.5, Table 66.
    pub family: Option<DeviceFamily>,
}

/// One place a content stream selected an `ICCBased` colour space.
///
/// ISO 19005-4 section 6.2.4.2's last requirement binds a profile that is *used*, which is the
/// whole difference between the corpus's `6-2-4-2-t03-fail-a` and its `t03-pass-b`: the same
/// profile sits in the same resource dictionary in both, and only one of them names it from
/// content.
#[derive(Debug, Clone)]
pub struct IccSelection {
    /// The zero-based index of the page whose content selected it.
    pub page: usize,
    /// Where a report points a reader.
    pub place: Where,
    /// What kind of site selected it, for the sentence a finding prints.
    pub what: &'static str,
    /// How the space carrying the profile was reached from the space the content named.
    ///
    /// The same reason [`DeviceColour::via`] carries one: section 6.2.4.2's restrictions reach a
    /// `Separation`'s alternate space only because section 6.2.4.4 sends them there, so a report
    /// has to be able to cite the clause that actually put the restriction where it found it.
    pub via: Route,
    /// The profile the selected space is formed from.
    pub profile: IccProfile,
    /// The profile of ISO 32000-2 §11.3.4's current blending colour space, where that space is
    /// itself an `ICCBased` one.
    pub blending: Option<IccProfile>,
}

/// One painting operator that marked the page in an `ICCBased` CMYK colour space.
///
/// ISO 32000-2 §8.6.7 is why the record is made at the operator rather than at the selection:
///
/// > Non-zero overprint mode shall apply only to painting operations that use the current colour
/// > in the graphics state when the current colour space is DeviceCMYK (or is implicitly
/// > converted to DeviceCMYK ; see (8.6.5.7, "Implicit conversion of CIE-Based colour spaces").
/// > It shall not, however, apply to the painting of images or shadings (8.7.4, "Shading
/// > patterns").
///
/// So each side of a painting operator is its own record: a stream may have an `ICCBased` CMYK
/// space in force for stroking and never stroke with it.
#[derive(Debug, Clone)]
pub struct IccCmykPaint {
    /// The zero-based index of the page the mark was made on.
    pub page: usize,
    /// Where a report points a reader.
    pub place: Where,
    /// What kind of content stream painted, for the sentence a finding prints.
    pub what: &'static str,
    /// Whether the space in force was the stroking one rather than the non-stroking one.
    pub stroking: bool,
    /// Whether Table 58's `OP` — or `op`, for the non-stroking side — was then true.
    pub overprinting: bool,
    /// Table 51's overprint mode, as `OPM` last set it. Its initial value is 0.
    pub mode: i64,
}

/// An extreme one document's content streams reached, with the page that reached it.
///
/// Every field of [`ContentLiterals`] is one of these, and so is the deepest `q` nesting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Extreme<T> {
    /// The value itself.
    pub value: T,
    /// The zero-based index of the page whose content reached it.
    pub page: usize,
}

/// How far a document's content-stream operands reach in each direction.
///
/// # Why extremes rather than every operand that breaks a limit
///
/// The rule that reads this — ISO 19005-2 section 6.1.13, applied to the values written *inside* a
/// content stream — is a set of bounds, and a bound is broken by the operand furthest out.
/// Keeping the extreme of each kind answers it in six words of state, where keeping every
/// breach would need the walk to know the standard's numbers and would let a hostile document
/// choose how much memory a survey costs.
///
/// It costs a report one finding per kind rather than one per operand. That is what a reader
/// can act on anyway: the page and the value are what send them to the right place, and a file
/// whose content states one out-of-range integer usually states thousands.
///
/// A real number is kept as its **magnitude**, because both of section 6.1.13's real-number bounds
/// are stated as magnitudes and a signed extreme would answer neither.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ContentLiterals {
    /// The largest integer any operand stated.
    pub largest_integer: Option<Extreme<i64>>,
    /// The smallest integer any operand stated.
    pub smallest_integer: Option<Extreme<i64>>,
    /// The largest magnitude any real operand stated.
    pub largest_real_magnitude: Option<Extreme<f64>>,
    /// The smallest non-zero magnitude any real operand stated.
    pub smallest_real_magnitude: Option<Extreme<f64>>,
    /// The length in bytes of the longest string operand.
    pub longest_string: Option<Extreme<usize>>,
    /// The length in bytes of the longest name operand, its escapes already expanded.
    pub longest_name: Option<Extreme<usize>>,
}

/// One content-stream keyword that ISO 32000's operator summary does not list.
///
/// See [`keyword`] for what "does not list" is decided against, and why the walk filters here
/// rather than reporting every operator it read.
#[derive(Debug, Clone)]
pub struct UnlistedOperator {
    /// The zero-based index of the page whose content read it.
    pub page: usize,
    /// Where a report points a reader.
    pub place: Where,
    /// What kind of content stream it stood in, for the sentence a finding prints.
    pub what: &'static str,
    /// The keyword itself, as it was written.
    pub spelling: Vec<u8>,
}

/// A font a content stream selected, and whether anything was drawn with it.
#[derive(Debug, Clone)]
pub struct SelectedFont {
    /// The font dictionary itself.
    pub dict: Dictionary,
    /// The object it is, where it is an indirect one.
    pub id: Option<ObjectId>,
    /// The zero-based index of the page whose content reached it first.
    pub page: usize,
    /// The resource name it was selected by, for the report.
    pub name: String,
    /// Whether a text-showing operator ran with it in a rendering mode other than 3.
    pub rendered: bool,
    /// The distinct byte strings text-showing operators drew with it, in any rendering mode,
    /// each with what stood between it and §14.9.4's replacement text.
    ///
    /// **Any mode, deliberately**, because the clause that most needs this says so: ISO 19005-2
    /// Section 6.2.11.8 and ISO 19005-4 section 6.2.10.9 forbid a reference to `.notdef` from a
    /// text-showing operator *regardless of text rendering mode*. A caller that wants only the
    /// rendered ones has [`Self::rendered`] beside it; one that filtered these by it would miss the
    /// case the corpus tests.
    ///
    /// Bytes rather than codes, because a code's length is the font's own `CMap`'s answer
    /// (ISO 32000-2 §9.7.6.2) and this walk holds no fonts — `pdf_font::LoadedFont::decode` is
    /// what turns these into codes, in the crate that already loads the font to ask about it.
    ///
    /// **A map rather than a set** because ISO 19005-2 section 6.2.11.7.3 asks about a
    /// *character* and not about a font: whether a character mapped into the Private Use Area
    /// is covered by a replacement text depends on where in the marked-content nesting the
    /// string holding it was drawn. [`Replacement`] is that, kept beside the string rather than
    /// in a second map, so the bytes are stored once.
    pub shown: BTreeMap<Vec<u8>, Replacement>,
    /// Whether [`Self::shown`] is all of what was drawn, or a prefix cut off by [`SHOWN_BUDGET`].
    ///
    /// A rule that asks whether *any* shown code is faulty may read a prefix; a rule that asks
    /// whether *every* shown code is sound may not. This is the flag that tells the second kind
    /// to stay silent.
    pub shown_complete: bool,
}

/// What stood between one string a font drew and ISO 32000-2 §14.9.4's replacement text.
///
/// §14.9.4 puts an `ActualText` in two places and the walk can only see one of them: the
/// property list of an enclosing marked-content sequence is here in the content stream, while a
/// structure element's entry is an object away — reached from §14.7.5.4's `/MCID` through the
/// page's parent tree, which is a document walk and not a content walk. So this records what
/// the content stream said and leaves the second route to the requirement that reads it.
///
/// **A showing an enclosing sequence covered outright contributes nothing.** Where an open
/// sequence stated `/ActualText` in its property list — or stated a property list this walk
/// could not read, which is the same answer for the purpose of not inventing a failure — that
/// showing of the string is settled where it stands and is not recorded here.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Replacement {
    /// Whether some showing of the string stood inside no marked-content sequence at all.
    ///
    /// The one case no later lookup can change: nothing encloses the string, so no property
    /// list and no structure element is attached to it.
    pub bare: bool,
    /// The `/MCID`s of the sequences enclosing the showings that were not settled inline, each
    /// with the page whose parent tree indexes it.
    ///
    /// The page travels with the identifier because §14.7.5.4 makes the identifier an index
    /// into *a page's* parent tree entry, and one font is shown on many pages.
    pub identifiers: Vec<(usize, i64)>,
    /// Whether an identifier was dropped for [`MAX_IDENTIFIERS`], so the list above is a prefix.
    pub elided: bool,
}

/// How one selected font is told from another.
///
/// By object where the resources name one, because two pages sharing a font share its object.
/// A font dictionary written directly into a resource dictionary has no object, so it is told
/// apart by the page and resource name that reached it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum FontKey {
    /// A font that is an indirect object.
    Indirect(ObjectId),
    /// A font written directly into a resource dictionary, on this page under this name.
    Direct(usize, Vec<u8>),
}

/// What one document's content streams do, as the requirements that read content need it.
#[derive(Debug, Default)]
pub struct Survey {
    /// The fonts the content selected, keyed so that a shared font is one entry.
    fonts: BTreeMap<FontKey, SelectedFont>,
    /// Every device colour space selection the walk reached.
    colours: Vec<DeviceColour>,
    /// Every transparency group `CS` entry the walk reached.
    groups: Vec<GroupSpace>,
    /// The pages the method finds transparency on.
    transparent: BTreeSet<usize>,
    /// Every named resource a content stream referenced and its resources did not define.
    missing: Vec<MissingResource>,
    /// Every inline image the walk read, with the page that draws it.
    inline_images: Vec<(usize, Dictionary)>,
    /// Every operand the rendering intent operator was given, with the page it ran on.
    rendering_intents: Vec<(usize, Vec<u8>)>,
    /// Every content stream the walk opened, in the order it opened them.
    streams: Vec<OpenedStream>,
    /// Every `ICCBased` colour space selection the walk reached.
    icc: Vec<IccSelection>,
    /// Every painting operator that marked the page in an `ICCBased` CMYK colour space.
    icc_paints: Vec<IccCmykPaint>,
    /// How far the operands of the content streams reached, in each direction.
    literals: ContentLiterals,
    /// The deepest `q` nesting any one content stream reached.
    nesting: Option<Extreme<usize>>,
    /// Every keyword the content used that ISO 32000's operator summary does not list.
    unlisted: Vec<UnlistedOperator>,
    /// Every `/ActualText` string a content stream wrote into a marked-content operator.
    actual_texts: Vec<(usize, Vec<u8>)>,
    /// Whether any content stream named `/ActualText` at all, whatever it gave the name.
    names_actual_text: bool,
    /// Every `/Lang` a marked-content property list stated.
    languages: Vec<MarkedLanguage>,
    /// How many pages the walk covered, so a caller can iterate them.
    pages: usize,
}

impl Survey {
    /// Walks one document's content and reports what it found.
    #[must_use]
    pub fn of(document: &Document) -> Self {
        let mut walk = Walk {
            document,
            visited: BTreeSet::new(),
            opened: 0,
            budget: TOKEN_BUDGET,
            shown_budget: SHOWN_BUDGET,
            actual_text_budget: ACTUAL_TEXT_BUDGET,
            marks: Vec::new(),
            seen: BTreeSet::new(),
            survey: Self::default(),
        };
        let pages = Pages::new(document);
        walk.survey.pages = pages.len();
        for index in 0..pages.len() {
            let Some(page) = pages.get(index) else {
                continue;
            };
            walk.page(&page, index);
        }
        walk.survey
    }

    /// The fonts the content streams selected.
    pub fn fonts(&self) -> impl Iterator<Item = &SelectedFont> {
        self.fonts.values()
    }

    /// Every device colour space selection, in the order the walk reached them.
    #[must_use]
    pub fn device_colours(&self) -> &[DeviceColour] {
        &self.colours
    }

    /// Every transparency group colour space the walk reached.
    #[must_use]
    pub fn group_spaces(&self) -> &[GroupSpace] {
        &self.groups
    }

    /// Every named resource a content stream referenced that its resources did not define.
    #[must_use]
    pub fn missing_resources(&self) -> &[MissingResource] {
        &self.missing
    }

    /// Every inline image the walk read, as the image dictionary an `XObject` would have had.
    ///
    /// `pdf_model::inline_image` expands §8.9.7's abbreviations on the way, so `/I` arrives as
    /// `Interpolate` and `/CS` as `ColorSpace` — the rules read one spelling rather than two.
    #[must_use]
    pub fn inline_images(&self) -> &[(usize, Dictionary)] {
        &self.inline_images
    }

    /// Every operand ISO 32000-2 §8.6.5.8's rendering intent operator was given.
    #[must_use]
    pub fn rendering_intents(&self) -> &[(usize, Vec<u8>)] {
        &self.rendering_intents
    }

    /// Every content stream the walk opened, with what ISO 19005 section 6.2.2 asks about each.
    #[must_use]
    pub fn opened_streams(&self) -> &[OpenedStream] {
        &self.streams
    }

    /// Every `ICCBased` colour space a content stream selected.
    #[must_use]
    pub fn icc_selections(&self) -> &[IccSelection] {
        &self.icc
    }

    /// Every painting operator that marked a page in an `ICCBased` CMYK colour space.
    #[must_use]
    pub fn icc_cmyk_paints(&self) -> &[IccCmykPaint] {
        &self.icc_paints
    }

    /// Whether the method finds transparency on this page.
    ///
    /// **Which method, for which part, is not one answer.** ISO 32000-2 Annex Q states it and is
    /// PDF/A-4's base standard; ISO 19005-2 states it itself, in its own normative Annex A, and
    /// *its* base standard has no Annex Q at all — ISO 32000-1:2008 states no such method. The
    /// two texts give the same steps and the same four graphics-state conditions, so one walk
    /// serves both; what would have been wrong is to say that a PDF/A-2 page's transparency is
    /// decided by a clause of a standard that part does not cite. ADR 0964 found the same shape
    /// one clause over, in the ICC edition rule, and this comment is written the way that one had
    /// to be corrected.
    #[must_use]
    pub fn page_is_transparent(&self, page: usize) -> bool {
        self.transparent.contains(&page)
    }

    /// How far the operands written inside the content streams reach, in each direction.
    #[must_use]
    pub const fn content_literals(&self) -> &ContentLiterals {
        &self.literals
    }

    /// The deepest `q` nesting any single content stream reached, and the page it was on.
    ///
    /// **Within one stream, deliberately.** ISO 32000-2 §8.10.1 makes a form `XObject`'s content
    /// run inside the graphics state the `Do` operator saved, so a `q` inside a form is nested
    /// inside whatever the invoking stream had open, and a depth summed across the invocation
    /// would be the graphics state stack's true depth. This does not sum it, for the reason
    /// every bound in this file is set the way it is: the walk follows a form once per page and
    /// a stream reached from two places has two depths, so a summed figure would depend on which
    /// invocation the walk happened to take — and a rule that failed a conforming file on that
    /// would be worse than one that missed a nesting split across two streams.
    ///
    /// **`TechNote 0010` A004 is the working group saying the same thing as a reading of the
    /// limit**, rather than as a caution: parts 1 to 3 are read as if the nesting limit assumed
    /// each content stream considered in isolation, ignoring the cumulative effect of nested form
    /// `XObject`s. So the figure this keeps is the one the limit is about, and the paragraph above
    /// is why it would have been kept that way regardless.
    ///
    /// Capped at [`MAX_NESTING`], which is an order of magnitude above the limit any rule reads
    /// it against.
    #[must_use]
    pub const fn deepest_graphics_state_nesting(&self) -> Option<Extreme<usize>> {
        self.nesting
    }

    /// Every keyword the content streams used that ISO 32000's operator summary does not list.
    #[must_use]
    pub fn unlisted_operators(&self) -> &[UnlistedOperator] {
        &self.unlisted
    }

    /// Every `/ActualText` string a content stream wrote inline, with the page that drew it.
    ///
    /// ISO 32000-2 §14.6.2 lets a marked-content property list be written straight into the
    /// `BDC` operator's operands rather than named as a resource, and such an entry is no object:
    /// nothing that walks the cross-reference table can see it. This is where it is seen, because
    /// the walk is reading those operands anyway.
    ///
    /// Read out of the property list itself: the entry is the value of an `ActualText` key,
    /// wherever inside the dictionary that key stands. This used to be a *shallow* pairing — the
    /// name operand and the string after it — which was sound only because nothing else in a
    /// content stream spells that name; the walk now builds the dictionary anyway, for
    /// §14.7.5.4's `/MCID`, so the entry is read rather than inferred.
    ///
    /// Bounded by [`ACTUAL_TEXT_BUDGET`], and an entry that did not fit is absent rather than
    /// short. A caller that needs to know whether the document states one *at all* asks
    /// [`Self::names_actual_text`], which no bound can make wrong.
    #[must_use]
    pub fn inline_actual_texts(&self) -> &[(usize, Vec<u8>)] {
        &self.actual_texts
    }

    /// Every `/Lang` a marked-content property list stated, in the order the walk read them.
    ///
    /// Deduplicated by page and value, because [`Where`] names a page rather than an operator
    /// and a page that states the same identifier in a hundred sequences says one thing.
    #[must_use]
    pub fn marked_languages(&self) -> &[MarkedLanguage] {
        &self.languages
    }

    /// Whether any content stream named `/ActualText`, whatever it gave the name.
    ///
    /// Wider than [`Self::inline_actual_texts`] on purpose, and a `bool` so that no budget can
    /// narrow it: the rule that reads this stays *silent* where a document states an
    /// `ActualText` anywhere, so an answer that missed one would turn a silence into a finding
    /// against a file that has what the clause asks for.
    #[must_use]
    pub const fn names_actual_text(&self) -> bool {
        self.names_actual_text
    }

    /// How many pages the walk covered.
    #[must_use]
    pub const fn pages(&self) -> usize {
        self.pages
    }
}

/// The part of the graphics state this walk needs.
#[derive(Clone, Default)]
struct State {
    /// The resource name of the current font, from `Tf`.
    font: Option<Vec<u8>>,
    /// ISO 32000-2 §9.3.6's text rendering mode, from `Tr`.
    mode: i64,
    /// What the fill colour space is, as far as this walk tells them apart.
    fill: Selected,
    /// The same for the stroke colour space.
    stroke: Selected,
    /// Whether the fill colour space is an `ICCBased` one whose profile has four components.
    fill_icc_cmyk: bool,
    /// The same for the stroke colour space.
    stroke_icc_cmyk: bool,
    /// ISO 32000-2 §8.6.7's overprint parameters, which decide what a mark does to the
    /// colourants it does not name.
    overprint: Overprint,
    /// ISO 32000-2 §11.3.4's current blending colour space, where a group established one.
    blending: Blending,
}

/// ISO 32000-2 §8.6.7's overprint parameters, as §8.4.5's Table 58 sets them.
///
/// [`Default`] is §8.4.1's Table 51: the overprint parameters start false and the overprint mode
/// starts at 0, so a stream that never runs `gs` paints with all three at their initial values.
#[derive(Clone, Copy, Default)]
struct Overprint {
    /// Table 58's `OP`, the overprint parameter for stroking.
    stroke: bool,
    /// Table 58's `op`, the overprint parameter for every other painting operation.
    fill: bool,
    /// Table 51's overprint mode, as `OPM` last set it.
    mode: i64,
}

/// ISO 32000-2 §11.3.4's current blending colour space, as much of it as the rules need.
///
/// Two fields rather than one because the colour subclauses ask two different questions of the same
/// space: section 6.2.4.3 asks which family it is device independent in, and section 6.2.4.2 asks
/// which profile it is, so that a colour space using the same profile can be told from one that
/// does not.
#[derive(Clone, Default)]
struct Blending {
    /// What kind of space it is.
    kind: Option<SpaceKind>,
    /// Its profile, where the space is an `ICCBased` one.
    profile: Option<IccProfile>,
}

/// What kind of colour space a `cs` or `CS` operator left in force.
///
/// Three cases rather than the space itself, because only three things are asked of it: whether
/// a `scn` operand names a pattern, whether a painting operator is painting in ISO 32000-2
/// §8.6.8's initial `DeviceGray`, and — recorded at selection time — which device space it was.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Selected {
    /// The initial colour space of §8.4.1's graphics state, which no operator has replaced.
    ///
    /// Table 51 gives the colour space parameter an initial value of `DeviceGray`, so a
    /// painting operator that runs before any `cs`, `g`, `rg` or `k` is painting in a device
    /// colour space the file never named — and ISO 19005 section 6.2.4.3 restricts a use, not a
    /// mention.
    #[default]
    Initial,
    /// A `Pattern` space, so the last name operand of `scn` names a pattern resource.
    Pattern,
    /// Any other space an operator selected.
    Plain,
}

/// The operators this walk acts on. Everything else is a boundary that clears the operands.
enum Operator {
    /// `q`
    Save,
    /// `Q`
    Restore,
    /// `Tf`
    SetFont,
    /// `Tr`
    SetRenderMode,
    /// `Tj`, `TJ`, `'` or `"`.
    Show,
    /// `Do`
    Invoke,
    /// `gs`
    SetGraphicsState,
    /// `cs` or `CS`, with which of the two.
    SetSpace { stroking: bool },
    /// `sc`, `scn`, `SC` or `SCN`, with which of the two.
    SetColour { stroking: bool },
    /// `g`, `G`, `rg`, `RG`, `k` or `K`.
    SetDeviceColour {
        /// Which device space the operator names.
        family: DeviceFamily,
        /// Whether it is the upper-case, stroking form.
        stroking: bool,
    },
    /// `sh`
    Shade,
    /// `ri`
    SetRenderingIntent,
    /// A path-painting operator, and which of fill and stroke it marks with.
    Paint {
        /// Whether it fills.
        fill: bool,
        /// Whether it strokes.
        stroke: bool,
    },
    /// `BI`, whose data is not a token stream.
    InlineImage,
    /// `BMC` or `BDC`, which open one of §14.6.1's marked-content sequences.
    ///
    /// The two are one case because what they open is the same; the difference is only whether
    /// a property list stands between the tag and the operator, which the operands say.
    BeginMarked,
    /// `EMC`, which closes the innermost open sequence.
    EndMarked,
    /// `DP`, §14.6.1's marked-content *point* with a property list.
    ///
    /// A point encloses nothing, so it opens no sequence and covers no string. It is here for
    /// its property list alone, which ISO 19005-2 section 6.7.4 binds exactly as a sequence's.
    MarkedPoint,
    /// Any other operator.
    Other,
}

/// One step of the walk: an operand worth keeping, an operator, or the end.
enum Step {
    /// A name operand.
    Name(Vec<u8>),
    /// An integer operand.
    Integer(i64),
    /// A real operand, kept for its magnitude alone; see [`ContentLiterals`].
    Real(f64),
    /// A string operand, which only a text-showing operator takes.
    Text(Vec<u8>),
    /// An operator, which consumes the operands before it.
    Operator(Operator),
    /// A keyword ISO 32000's operator summary does not list, which consumes them too.
    Unlisted(Vec<u8>),
    /// A dictionary operand's `<<`, which only §14.6.2's property list is written as.
    ///
    /// The tokens of it are not steps: [`Walk::run`] reads them into a dictionary with
    /// `pdf_model::content::reader::inline_dictionary`, which is the interpreter's own reader of
    /// this construction rather than a second one.
    Dictionary,
    /// An operand of a kind no rule here reads.
    Other,
    /// The content stream ended.
    End,
}

/// Reads one token and reduces it to what the walk needs.
fn next_step(reader: &mut ContentReader<'_>) -> Step {
    reader.with_token(|token| match token {
        None => Step::End,
        Some(Token::Name(name)) => Step::Name(name),
        Some(Token::Integer(value)) => Step::Integer(value),
        Some(Token::Real(value)) => Step::Real(value),
        Some(Token::String(text)) => Step::Text(text),
        Some(Token::Keyword(word)) => keyword_step(word),
        Some(Token::DictOpen) => Step::Dictionary,
        Some(_) => Step::Other,
    })
}

/// One bare keyword, as an operator, an operand, or a word no operator summary lists.
fn keyword_step(word: &[u8]) -> Step {
    if let Some(operator) = keyword(word) {
        return Step::Operator(operator);
    }
    // §7.3.2's booleans and §7.3.9's null are *objects*, and the lexer hands every keyword over
    // the same way — so they arrive here looking like operators and are not. A `BDC` property
    // list written inline is where a content stream states one.
    if matches!(word, b"true" | b"false" | b"null") {
        return Step::Other;
    }
    Step::Unlisted(word.to_vec())
}

/// Which operator a content-stream keyword is, or `None` where no operator summary lists it.
///
/// # Why every operator is named here, including the ones this walk ignores
///
/// The arms below fall into two halves. The first is the operators the survey *acts* on. The second
/// names every remaining operator of the base standard's operator summary and does nothing with it
/// — and it is there because the `None` case is itself an answer somebody asks for: ISO 19005-2
/// Section 6.2.2 and ISO 19005-4 section 6.2.2 forbid a content stream to use an operator the base
/// standard does not define, and [`Survey::unlisted_operators`] is how that is reported.
///
/// **The two editions' summaries hold the same operators**, checked entry by entry: ISO 32000-2
/// Annex A's Table A.1 and ISO 32000-1:2008 Annex A's Table A.1 each list the same 73, differing
/// only in how they annotate `F` — obsolete in the first edition, deprecated in the second. So
/// one table serves both parts of ISO 19005, and a caller does not have to ask which edition its
/// target points at. Were a later edition to add or drop one, this would become two tables and
/// the reporting would have to carry which.
///
/// Deciding it *here* rather than in the requirement is a measured choice. A rule that judged the
/// operators would need the walk to report every one it read, and deduplicating a set of
/// operators per page over ISO 32000-2's own eight million content tokens costs more than the
/// whole survey does; filtering in the one `match` the walk already runs costs the arms below and
/// an allocation per word that no summary lists, which an ordinary document never pays at all.
fn keyword(word: &[u8]) -> Option<Operator> {
    let operator = match word {
        b"q" => Operator::Save,
        b"Q" => Operator::Restore,
        b"Tf" => Operator::SetFont,
        b"Tr" => Operator::SetRenderMode,
        b"Tj" | b"TJ" | b"'" | b"\"" => Operator::Show,
        b"Do" => Operator::Invoke,
        b"gs" => Operator::SetGraphicsState,
        b"cs" => Operator::SetSpace { stroking: false },
        b"CS" => Operator::SetSpace { stroking: true },
        b"sc" | b"scn" => Operator::SetColour { stroking: false },
        b"SC" | b"SCN" => Operator::SetColour { stroking: true },
        b"g" => device_colour(DeviceFamily::Gray, false),
        b"G" => device_colour(DeviceFamily::Gray, true),
        b"rg" => device_colour(DeviceFamily::Rgb, false),
        b"RG" => device_colour(DeviceFamily::Rgb, true),
        b"k" => device_colour(DeviceFamily::Cmyk, false),
        b"K" => device_colour(DeviceFamily::Cmyk, true),
        b"sh" => Operator::Shade,
        b"ri" => Operator::SetRenderingIntent,
        b"f" | b"F" | b"f*" => Operator::Paint {
            fill: true,
            stroke: false,
        },
        b"S" | b"s" => Operator::Paint {
            fill: false,
            stroke: true,
        },
        b"B" | b"B*" | b"b" | b"b*" => Operator::Paint {
            fill: true,
            stroke: true,
        },
        b"BI" => Operator::InlineImage,
        b"BDC" | b"BMC" => Operator::BeginMarked,
        b"EMC" => Operator::EndMarked,
        b"DP" => Operator::MarkedPoint,
        // The rest of Table A.1, in the order the table prints them. Nothing here changes the
        // state this walk carries; they are listed so that the fall-through means what it says.
        b"BT" | b"BX" | b"c" | b"cm" | b"d" | b"d0" | b"d1" | b"EI" | b"ET" | b"EX" | b"h"
        | b"i" | b"ID" | b"j" | b"J" | b"l" | b"m" | b"M" | b"MP" | b"n" | b"re" | b"T*"
        | b"Tc" | b"Td" | b"TD" | b"TL" | b"Tm" | b"Ts" | b"Tw" | b"Tz" | b"v" | b"w" | b"W"
        | b"W*" | b"y" => Operator::Other,
        _ => return None,
    };
    Some(operator)
}

/// One of the six operators that names a device colour space and sets a colour in it.
const fn device_colour(family: DeviceFamily, stroking: bool) -> Operator {
    Operator::SetDeviceColour { family, stroking }
}

/// One open marked-content sequence, as much of its property list as the rules here read.
///
/// ISO 32000-2 §14.6.1's sequences nest, and what encloses a string is what decides whether
/// §14.9.4's replacement text covers it — so this is a stack entry and not a flag.
#[derive(Clone, Copy, Default)]
struct Mark {
    /// Whether this sequence settles the question for what it encloses.
    ///
    /// True where its property list states `/ActualText`, and **also** where the list is one
    /// this walk could not read — a named property list the resources do not define, or a value
    /// that is no dictionary. The two are one field because they have one consequence: a rule
    /// that may not invent a failure has to stay silent for both.
    settles: bool,
    /// §14.7.5.4's `/MCID`, where the property list states one.
    mcid: Option<i64>,
    /// How many sequences opened under this one were not given a stack entry of their own.
    ///
    /// [`MAX_MARK_DEPTH`] bounds the stack, and a bound that simply dropped an open sequence
    /// would leave its `EMC` to close somebody else's. So the drop is counted here and the
    /// `EMC` spends the count instead — and [`Mark::settles`] is set at the same moment, which
    /// makes everything under the bound silent rather than reported against a sequence this
    /// walk stopped following.
    suppressed: u32,
}

/// The state one document's walk carries across its content streams.
struct Walk<'a> {
    /// The document being walked.
    document: &'a Document,
    /// Which nested streams have been opened on which page, so one reached twice is read once.
    ///
    /// Keyed by page as well as by object, because a form placed on two pages puts its
    /// transparency and its colours on *both* of them, and a set keyed by object alone would
    /// report the second page as if the form were not there.
    visited: BTreeSet<(usize, ObjectId)>,
    /// How many nested streams have been opened.
    opened: usize,
    /// How many more tokens may be read.
    budget: u64,
    /// How many more bytes of distinct shown strings may be kept; see [`SHOWN_BUDGET`].
    shown_budget: usize,
    /// How many more bytes of `/ActualText` may be kept; see [`ACTUAL_TEXT_BUDGET`].
    actual_text_budget: usize,
    /// The marked-content sequences open where the walk is standing, outermost first.
    ///
    /// One stack for the whole walk rather than one per stream, because a form `XObject`
    /// invoked between a `BDC` and its `EMC` draws *inside* that sequence: §8.10.1 runs its
    /// content where the `Do` stands. Each stream remembers the depth it found and truncates
    /// back to it when it ends, so an unbalanced stream cannot close its caller's sequences.
    marks: Vec<Mark>,
    /// Which observations have already been recorded, so that a repeat costs nothing.
    ///
    /// A page that paints ten thousand red rectangles selects `DeviceRGB` ten thousand times
    /// under one resource dictionary, and every one of those observations carries the same
    /// answer to every question a requirement asks. Keeping them all cost ISO 32000-2's own
    /// specification 317 127 colour records where 25 distinct ones say the same thing; the set
    /// is what turns that into an allocation nobody pays for.
    seen: BTreeSet<Observation>,
    /// What has been found so far.
    survey: Survey,
}

/// What makes one observation different from another, for the set above.
///
/// Deliberately *not* the witness: two `DeviceRGB` selections on one page differ in where they
/// stand in the content stream and in nothing a requirement or a report can read, because
/// [`Where`] names a page rather than an operator.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Observation {
    /// A device colour space selection.
    Colour(
        usize,
        DeviceFamily,
        Route,
        &'static str,
        DefaultSpace,
        Option<SpaceKind>,
    ),
    /// A transparency group's colour space.
    Group(usize, SpaceKind, DefaultSpace),
    /// A named resource the resources in force did not define.
    Missing(usize, &'static str, String),
    /// A rendering intent operator's operand.
    Intent(usize, Vec<u8>),
    /// An `ICCBased` colour space selection, by the route, the profile and the blending profile.
    Icc(
        usize,
        &'static str,
        Route,
        Option<ObjectId>,
        Option<ObjectId>,
    ),
    /// A painting operator marking in an `ICCBased` CMYK space, with the overprint parameters.
    Overprint(usize, &'static str, bool, bool, i64),
    /// A keyword no operator summary lists, by the page and kind of stream that used it.
    Operator(usize, &'static str, Vec<u8>),
    /// An `/ActualText` value written inline, by the page that drew it.
    ActualText(usize, Vec<u8>),
    /// A `/Lang` a marked-content property list stated, by the page that stated it.
    Language(usize, Result<Vec<u8>, &'static str>),
}

/// Which of [`ContentLiterals`]' two length extremes an operand widens.
#[derive(Debug, Clone, Copy)]
enum Extent {
    /// A string operand's length in bytes.
    String,
    /// A name operand's length in bytes, its escapes already expanded.
    Name,
}

/// The content stream [`Walk::run`] is reading.
///
/// Two of the fields travel together because [`Walk::look_up`] needs both: the description a
/// finding prints, and the entry in [`Survey::streams`] to mark when the stream names a resource.
#[derive(Clone, Copy)]
struct Origin {
    /// What kind of content stream it is, for the sentence a finding prints.
    what: &'static str,
    /// Its entry in [`Survey::streams`], where one fitted within [`MAX_OBSERVATIONS`].
    record: Option<usize>,
    /// Whether the resources [`Walk::run`] reads it against are its own explicitly associated
    /// dictionary rather than one it fell back on. See [`DefaultSpace`].
    explicitly_associated: bool,
}

impl Walk<'_> {
    /// One page: its group, its content, and its annotations' appearances.
    fn page(&mut self, page: &Page, index: usize) {
        // §7.8.3: a page's content is associated with the dictionary its own `Resources` entry
        // designates *or* one it inherits, and only the first of those is explicit. Read the
        // same way a form XObject's entry is, so that an entry reaching no dictionary counts
        // as no association rather than as a broken one.
        let own = self
            .document
            .get_key(&page.dict, "Resources")
            .as_dict()
            .is_some();
        let blending = self.page_group(page, index, own);
        let state = State {
            blending: blending.clone(),
            ..State::default()
        };
        let origin = self.open_record(index, None, Where::page(index), "the page content", own);
        let mut reader = ContentReader::for_page(self.document, page);
        self.run(&mut reader, &page.resources, index, 0, &state, origin);
        self.annotations(page, index, &blending);
    }

    /// Notes that a content stream has been opened, and answers with what [`Walk::run`] needs.
    fn open_record(
        &mut self,
        page: usize,
        object: Option<ObjectId>,
        place: Where,
        what: &'static str,
        own_resources: bool,
    ) -> Origin {
        let record = (self.survey.streams.len() < MAX_OBSERVATIONS).then(|| {
            self.survey.streams.push(OpenedStream {
                page,
                object,
                place,
                what,
                referenced: false,
                own_resources,
            });
            self.survey.streams.len().saturating_sub(1)
        });
        Origin {
            what,
            record,
            explicitly_associated: own_resources,
        }
    }

    /// A page's `/Group`, which ISO 32000-2 §11.4.7 makes the page's blending colour space.
    ///
    /// The clause is explicit that a page group's `CS` is honoured whatever its `I` entry says,
    /// because a page imposed on the output medium is effectively isolated — so, unlike a form
    /// `XObject`'s group below, no isolation test guards this one.
    fn page_group(&mut self, page: &Page, index: usize, explicitly_associated: bool) -> Blending {
        let group = self.document.get_key(&page.dict, "Group");
        let Some(group) = group.as_dict() else {
            return Blending::default();
        };
        if !self.is_transparency_group(group) {
            return Blending::default();
        }
        self.record_group_space(
            group,
            &page.resources,
            index,
            Where::page(index),
            explicitly_associated,
        )
    }

    /// Whether a group attributes dictionary is a transparency group: §11.6.6, Table 147.
    fn is_transparency_group(&self, group: &Dictionary) -> bool {
        self.document
            .get_key(group, "S")
            .as_name()
            .is_some_and(|name| name.as_bytes() == b"Transparency")
    }

    /// Records one group's `CS` entry and answers with the blending space it establishes.
    fn record_group_space(
        &mut self,
        group: &Dictionary,
        resources: &Dictionary,
        page: usize,
        place: Where,
        explicitly_associated: bool,
    ) -> Blending {
        let Some(stated) = group.get("CS") else {
            return Blending::default();
        };
        let stated = self.document.resolve(stated);
        let kind = classify(self.document, &stated, resources, 0);
        let in_force = match kind {
            SpaceKind::Device(family) => self.default_space(resources, family),
            _ => None,
        };
        let default = DefaultSpace::read(in_force, explicitly_associated);
        if self.first_time(Observation::Group(page, kind, default)) {
            self.survey.groups.push(GroupSpace {
                kind,
                page,
                place: place.named("CS"),
                default,
            });
        }
        Blending {
            kind: Some(kind),
            profile: icc_profile(self.document, &stated, resources, 0),
        }
    }

    /// Walks every appearance stream a page's annotations carry.
    ///
    /// ISO 32000-2 §12.5.5 makes an `/AP` entry either a stream or a dictionary of states, and
    /// each of the three entries is drawn under some condition, so all of them are content the
    /// document renders. §12.5.2's own `BM` entry is part of the graphics state the appearance
    /// is drawn in, so Annex Q's blend-mode test applies to it as it does to a `gs`.
    fn annotations(&mut self, page: &Page, index: usize, blending: &Blending) {
        let Object::Array(annotations) = self.document.get_key(&page.dict, "Annots") else {
            return;
        };
        let state = State {
            blending: blending.clone(),
            ..State::default()
        };
        for annotation in &annotations {
            let Object::Dictionary(annotation) = self.document.resolve(annotation) else {
                continue;
            };
            if self.annotation_is_transparent(&annotation) {
                self.survey.transparent.insert(index);
            }
            let appearances = self.document.get_key(&annotation, "AP");
            let Some(appearances) = appearances.as_dict() else {
                continue;
            };
            for (_, entry) in appearances.iter() {
                self.appearance(entry, page, index, &state);
            }
        }
    }

    /// Whether an annotation's own dictionary puts transparency in the graphics state.
    fn annotation_is_transparent(&self, annotation: &Dictionary) -> bool {
        blends(self.document, annotation) || below_one(&self.document.get_key(annotation, "CA"))
    }

    /// One `/AP` entry, which is either the stream itself or a dictionary of appearance states.
    fn appearance(&mut self, entry: &Object, page: &Page, index: usize, state: &State) {
        let id = entry.as_reference();
        match self.document.resolve(entry) {
            Object::Stream(stream) => self.form(&stream, id, &page.resources, index, 0, state),
            Object::Dictionary(states) => {
                for (_, one) in states.iter() {
                    let id = one.as_reference();
                    if let Object::Stream(stream) = self.document.resolve(one) {
                        self.form(&stream, id, &page.resources, index, 0, state);
                    }
                }
            }
            _ => {}
        }
    }

    /// Runs one content stream, in the state it inherits.
    ///
    /// ISO 32000-2 §8.10.1 starts a form `XObject` in the graphics state in force at `Do`, so
    /// the inherited state is a copy and what the form does to it does not return.
    fn run(
        &mut self,
        reader: &mut ContentReader<'_>,
        resources: &Dictionary,
        page: usize,
        depth: u32,
        initial: &State,
        origin: Origin,
    ) {
        let mut state = initial.clone();
        // `TechNote 0010` A028: parts 2 and 3 are read as if a default colour space had to be
        // defined in the *explicitly associated* resources dictionary, and as if a processor
        // ignored what that dictionary does not define. Both readings are recorded here because
        // the resolution does not name ISO 19005-4; see [`DefaultSpace`].
        let explicit = origin.explicitly_associated;
        let defaults = [
            DefaultSpace::read(self.default_space(resources, DeviceFamily::Gray), explicit),
            DefaultSpace::read(self.default_space(resources, DeviceFamily::Rgb), explicit),
            DefaultSpace::read(self.default_space(resources, DeviceFamily::Cmyk), explicit),
        ];
        let mut stack: Vec<State> = Vec::new();
        let mut names: Vec<Vec<u8>> = Vec::new();
        // Every string operand since the last operator: `Tj` takes one, `TJ` takes the strings
        // of its array, and `\"` takes one after two numbers. Keeping all of them and letting
        // the operator decide is what makes those three the same case here.
        let mut strings: Vec<Vec<u8>> = Vec::new();
        let mut integer: Option<i64> = None;
        // §14.6.2's property list, where the operator being assembled was given one written
        // into its operands rather than named through the resources.
        let mut property: Option<Dictionary> = None;
        // Where this stream found the marked-content stack. A stream that opens more sequences
        // than it closes leaves them here, and one that closes more would otherwise close its
        // caller's; §14.6.1 makes neither a thing a conforming stream does, so the depth is
        // restored on the way out rather than the imbalance reported.
        let base = self.marks.len();
        loop {
            if self.budget == 0 {
                self.marks.truncate(base);
                return;
            }
            self.budget = self.budget.saturating_sub(1);
            let step = next_step(reader);
            let names_it = matches!(&step, Step::Name(name) if name == b"ActualText");
            match step {
                Step::End => {
                    self.marks.truncate(base);
                    return;
                }
                Step::Other => {}
                Step::Dictionary => {
                    let dict = pdf_model::content::reader::inline_dictionary(reader);
                    self.note_inline_dictionary(&dict, page);
                    property = Some(dict);
                }
                Step::Name(name) => {
                    self.note_length(name.len(), page, Extent::Name);
                    self.survey.names_actual_text |= names_it;
                    // At most two are ever wanted: `Tf` and `Do` take the first, and `scn`
                    // takes the last. Keeping two rather than every operand is what stops a
                    // stream of names from growing this vector without bound.
                    if names.len() < 2 {
                        names.push(name);
                    } else {
                        names[1] = name;
                    }
                }
                // The last integer, because `Tr` takes exactly one and it comes last.
                Step::Integer(value) => {
                    self.note_integer(value, page);
                    integer = Some(value);
                }
                Step::Real(value) => self.note_real(value, page),
                Step::Text(text) => {
                    self.note_length(text.len(), page, Extent::String);
                    // Bounded by the same budget the kept strings are, so a stream of literals
                    // between two operators cannot grow this vector without one.
                    if text.len() <= self.shown_budget {
                        strings.push(text);
                    }
                }
                Step::Unlisted(word) => {
                    self.note_unlisted_operator(&word, page, origin);
                    names.clear();
                    strings.clear();
                    integer = None;
                    property = None;
                }
                Step::Operator(operator) => {
                    let context = Context {
                        resources,
                        defaults,
                        page,
                        depth,
                        what: origin.what,
                        record: origin.record,
                        marks_base: base,
                    };
                    self.apply(
                        &operator,
                        &mut state,
                        &mut stack,
                        &Operands {
                            names: &names,
                            strings: &strings,
                            integer,
                            property: property.as_ref(),
                        },
                        &context,
                        reader,
                    );
                    names.clear();
                    strings.clear();
                    integer = None;
                    property = None;
                }
            }
        }
    }

    /// One operator, against the state it changes.
    fn apply(
        &mut self,
        operator: &Operator,
        state: &mut State,
        stack: &mut Vec<State>,
        operands: &Operands<'_>,
        context: &Context<'_>,
        reader: &mut ContentReader<'_>,
    ) {
        match *operator {
            Operator::Save => {
                if stack.len() < MAX_NESTING {
                    stack.push(state.clone());
                }
                // After the push, so that the first `q` of a stream counts as one level deep.
                self.note_nesting(stack.len(), context.page);
            }
            Operator::Restore => {
                if let Some(previous) = stack.pop() {
                    *state = previous;
                }
            }
            Operator::SetFont => {
                if let Some(name) = operands.first() {
                    state.font = Some(name.clone());
                }
            }
            Operator::SetRenderMode => {
                if let Some(mode) = operands.integer {
                    state.mode = mode;
                }
            }
            Operator::Show => self.show(state, operands, context),
            Operator::Invoke => {
                if let Some(name) = operands.first().cloned() {
                    self.invoke(&name, state, context);
                }
            }
            Operator::SetGraphicsState => {
                if let Some(name) = operands.first() {
                    self.graphics_state(name, state, context);
                }
            }
            Operator::SetSpace { stroking } => {
                if let Some(name) = operands.first().cloned() {
                    self.set_space(&name, stroking, state, context);
                }
            }
            Operator::SetColour { stroking } => {
                let selected = if stroking { state.stroke } else { state.fill };
                if selected == Selected::Pattern
                    && let Some(name) = operands.last()
                {
                    self.pattern(name, state, context);
                }
            }
            Operator::SetDeviceColour { family, stroking } => {
                self.record_colour(
                    (family, Route::Direct),
                    context,
                    "a colour operator",
                    state.blending.kind,
                );
                if stroking {
                    state.stroke = Selected::Plain;
                    state.stroke_icc_cmyk = false;
                } else {
                    state.fill = Selected::Plain;
                    state.fill_icc_cmyk = false;
                }
            }
            Operator::Shade => {
                if let Some(name) = operands.first() {
                    self.shading_resource(name, state, context);
                }
            }
            Operator::SetRenderingIntent => {
                if let Some(name) = operands.first()
                    && self.first_time(Observation::Intent(context.page, name.clone()))
                {
                    self.survey
                        .rendering_intents
                        .push((context.page, name.clone()));
                }
            }
            Operator::Paint { fill, stroke } => self.paint(fill, stroke, state, context),
            Operator::InlineImage => self.inline_image(reader, context),
            Operator::BeginMarked => self.begin_marked(operands, context),
            Operator::EndMarked => {
                if let Some(top) = self.marks.last_mut()
                    && top.suppressed > 0
                {
                    top.suppressed = top.suppressed.saturating_sub(1);
                } else if self.marks.len() > context.marks_base {
                    self.marks.pop();
                }
            }
            Operator::MarkedPoint => {
                // A point encloses nothing, so its property list is read for §14.9.2's `/Lang`
                // and nothing is pushed.
                if let PropertyList::Read(list) = self.property_list(operands, context) {
                    self.note_language(&list, context.page);
                }
            }
            Operator::Other => {}
        }
    }

    /// A text-showing operator: the font it ran with, and the marks it made.
    fn show(&mut self, state: &State, operands: &Operands<'_>, context: &Context<'_>) {
        let Some(name) = state.font.clone() else {
            return;
        };
        // Mode 3 alone is exempt: mode 7 adds the glyphs to the clipping path, which is a mark
        // on the page by way of everything drawn afterwards.
        let font = self.select(&name, context, state.mode != 3, operands.strings);
        // ISO 32000-2 §9.3.6: modes 0, 2, 4 and 6 fill and modes 1, 2, 5 and 6 stroke, so the
        // colour space in force is used exactly as a path-painting operator would use it.
        let fills = matches!(state.mode, 0 | 2 | 4 | 6);
        let strokes = matches!(state.mode, 1 | 2 | 5 | 6);
        self.paint(fills, strokes, state, context);
        if let Some(font) = font {
            self.type3(&font, state, context);
        }
    }

    /// Records that a content stream selected the font a resource name stands for.
    fn select(
        &mut self,
        name: &[u8],
        context: &Context<'_>,
        rendered: bool,
        strings: &[Vec<u8>],
    ) -> Option<Dictionary> {
        let entry = self.look_up("Font", name, context, context.what)?;
        let id = entry.as_reference();
        let Object::Dictionary(dict) = self.document.resolve(&entry) else {
            return None;
        };
        let key = id.map_or_else(
            || FontKey::Direct(context.page, name.to_vec()),
            FontKey::Indirect,
        );
        // Read before the entry is borrowed, and once rather than per string: every string a
        // single text-showing operator draws stands inside the same sequences. The stack is
        // lent to `widen` rather than collected into a list, because collecting one would be a
        // heap allocation per text-showing operator on a tagged page — which is most of them.
        let settled = self.marks.iter().any(|mark| mark.settles);
        let bare = !settled && !self.marks.iter().any(|mark| mark.mcid.is_some());
        let enclosing: &[Mark] = if settled { &[] } else { &self.marks };
        let page = context.page;
        let entry = self
            .survey
            .fonts
            .entry(key)
            .or_insert_with(|| SelectedFont {
                dict: dict.clone(),
                id,
                page: context.page,
                name: String::from_utf8_lossy(name).into_owned(),
                rendered: false,
                shown: BTreeMap::new(),
                shown_complete: true,
            });
        entry.rendered |= rendered;
        for text in strings {
            if let Some(replacement) = entry.shown.get_mut(text) {
                widen(replacement, bare, enclosing, page);
                continue;
            }
            if text.len() > self.shown_budget {
                // The font is not reported on a prefix of its text; see `SHOWN_BUDGET`.
                entry.shown_complete = false;
                continue;
            }
            self.shown_budget = self.shown_budget.saturating_sub(text.len());
            let mut replacement = Replacement::default();
            widen(&mut replacement, bare, enclosing, page);
            entry.shown.insert(text.clone(), replacement);
        }
        Some(dict)
    }

    /// ISO 32000-2 §9.6.5: a Type 3 font's glyphs are content streams of their own.
    ///
    /// Annex Q.5 says the same thing for transparency, and section 6.2.4.3's corpus says it for
    /// colour: what a glyph procedure paints is painted on the page that showed the glyph.
    fn type3(&mut self, font: &Dictionary, state: &State, context: &Context<'_>) {
        let subtype = self.document.get_key(font, "Subtype");
        if subtype
            .as_name()
            .is_none_or(|name| name.as_bytes() != b"Type3")
        {
            return;
        }
        // §9.6.5.4: a Type 3 font's own `/Resources` are what its glyph procedures are read
        // against, and a font that states none falls back on the invoking stream's.
        let own = self.document.get_key(font, "Resources");
        let stated = own.as_dict().is_some();
        let resources = own
            .as_dict()
            .cloned()
            .unwrap_or_else(|| context.resources.clone());
        let procedures = self.document.get_key(font, "CharProcs");
        let Some(procedures) = procedures.as_dict() else {
            return;
        };
        for (_, procedure) in procedures.iter() {
            let id = procedure.as_reference();
            let Object::Stream(stream) = self.document.resolve(procedure) else {
                continue;
            };
            self.nested(
                &stream,
                id,
                &resources,
                context.page,
                context.depth,
                state,
                "a Type 3 glyph procedure",
                stated,
            );
        }
    }

    /// Follows `Do` into a form or image `XObject`.
    fn invoke(&mut self, name: &[u8], state: &State, context: &Context<'_>) {
        let Some(entry) = self.look_up("XObject", name, context, context.what) else {
            return;
        };
        let id = entry.as_reference();
        let Object::Stream(stream) = self.document.resolve(&entry) else {
            return;
        };
        let subtype = self.document.get_key(&stream.dict, "Subtype");
        let subtype = subtype.as_name().map(|name| name.as_bytes().to_vec());
        match subtype.as_deref() {
            Some(b"Form") => self.form(
                &stream,
                id,
                context.resources,
                context.page,
                context.depth,
                state,
            ),
            Some(b"Image") => self.image(&stream, id, context),
            _ => {}
        }
    }

    /// One form `XObject`: its transparency group, and then its content.
    ///
    /// ISO 32000-2 §11.6.6 makes an isolated group's own `CS` the blending colour space for
    /// what it contains; a non-isolated group, or one that states no `CS`, inherits the space of
    /// the page or group it is painted into.
    fn form(
        &mut self,
        stream: &Stream,
        id: Option<ObjectId>,
        fallback: &Dictionary,
        page: usize,
        depth: u32,
        state: &State,
    ) {
        let mut state = state.clone();
        let own = self.document.get_key(&stream.dict, "Resources");
        let stated_resources = own.as_dict().is_some();
        let resources = own.as_dict().cloned().unwrap_or_else(|| fallback.clone());
        let group = self.document.get_key(&stream.dict, "Group");
        if let Some(group) = group.as_dict()
            && self.is_transparency_group(group)
        {
            self.survey.transparent.insert(page);
            let place = id.map_or_else(|| Where::page(page), Where::object);
            let stated = self.record_group_space(group, &resources, page, place, stated_resources);
            if self.document.get_key(group, "I") == Object::Boolean(true) && stated.kind.is_some() {
                state.blending = stated;
            }
        }
        self.nested(
            stream,
            id,
            &resources,
            page,
            depth,
            &state,
            "a form XObject",
            stated_resources,
        );
    }

    /// An image `XObject`: its colour space, and Annex Q.4's two transparency conditions.
    fn image(&mut self, stream: &Stream, id: Option<ObjectId>, context: &Context<'_>) {
        let place = id.map_or_else(|| Where::page(context.page), Where::object);
        if matches!(
            self.document.get_key(&stream.dict, "SMask"),
            Object::Stream(_)
        ) || self
            .document
            .get_key(&stream.dict, "SMaskInData")
            .as_integer()
            .is_some_and(|value| value > 0)
        {
            self.survey.transparent.insert(context.page);
        }
        self.image_space(&stream.dict, &place, context, "an image XObject");
    }

    /// One image dictionary's `/ColorSpace`, recorded where it names a device space.
    ///
    /// §8.6.5.6's remapping reaches an image's colour space through the resource dictionary the
    /// image was *drawn* from, which is the one in force here rather than any the image carries.
    fn image_space(
        &mut self,
        dict: &Dictionary,
        place: &Where,
        context: &Context<'_>,
        what: &'static str,
    ) {
        let Some(stated) = dict.get("ColorSpace") else {
            return;
        };
        let stated = self.document.resolve(stated);
        self.record_space(&stated, place, context, what, None, None);
    }

    /// ISO 32000-2 §8.9.7's inline image: its colour space, and then the bytes stepped over.
    fn inline_image(&mut self, reader: &mut ContentReader<'_>, context: &Context<'_>) {
        let bound = LOOKAHEAD.min(self.document.limits().max_stream_len);
        let mut want = WINDOW;
        loop {
            let settled = {
                let (ahead, complete) = reader.lookahead(want);
                let scanned = pdf_model::inline_image::scan(
                    self.document,
                    ahead,
                    0,
                    context.resources,
                    complete,
                );
                if complete || scanned.image.is_ok() || want >= bound {
                    Some((scanned.resume, scanned.image.ok()))
                } else {
                    None
                }
            };
            if let Some((resume, image)) = settled {
                reader.skip(resume);
                if let Some(image) = image {
                    let place = Where::page(context.page);
                    self.image_space(&image.dict, &place, context, "an inline image");
                    if self.survey.inline_images.len() < MAX_OBSERVATIONS {
                        self.survey
                            .inline_images
                            .push((context.page, image.dict.clone()));
                    }
                }
                return;
            }
            want = want.saturating_mul(2).min(bound);
        }
    }

    /// A `gs` operator: Annex Q.2's four conditions on the graphics state it sets.
    fn graphics_state(&mut self, name: &[u8], state: &mut State, context: &Context<'_>) {
        let Some(entry) = self.look_up("ExtGState", name, context, context.what) else {
            return;
        };
        let Object::Dictionary(dict) = self.document.resolve(&entry) else {
            return;
        };
        if self.is_transparent_state(&dict) {
            self.survey.transparent.insert(context.page);
        }
        self.overprint(&dict, state);
    }

    /// ISO 32000-2 §8.4.5, Table 58's `OP`, `op` and `OPM`, as a `gs` operator sets them.
    ///
    /// `OP` sets both parameters or only the stroking one, and which of the two it does is
    /// decided by the same dictionary rather than by what any earlier one said:
    ///
    /// > Specifying an OP entry shall set both parameters unless there is also an op entry in
    /// > the same graphics state parameter dictionary, in which case the OP entry shall set only
    /// > the overprint parameter for stroking.
    fn overprint(&self, dict: &Dictionary, state: &mut State) {
        let stated = |key: &str| dict.get(key).is_some();
        let on = |object: &Object| *object == Object::Boolean(true);
        if stated("OP") {
            let stroking = on(&self.document.get_key(dict, "OP"));
            state.overprint.stroke = stroking;
            if !stated("op") {
                state.overprint.fill = stroking;
            }
        }
        if stated("op") {
            state.overprint.fill = on(&self.document.get_key(dict, "op"));
        }
        if let Some(mode) = self.document.get_key(dict, "OPM").as_integer() {
            state.overprint.mode = mode;
        }
    }

    /// ISO 32000-2 §Q.2's four tests on one graphics state parameter dictionary.
    ///
    /// > - SMask key is present and its value is of type dictionary;
    /// > - ca key is present and its value is less than one (1);
    /// > - CA key is present and its value is less than one (1);
    /// > - BM key is present and its value is not Normal.
    fn is_transparent_state(&self, dict: &Dictionary) -> bool {
        if dict.get("SMask").is_some() && self.document.get_key(dict, "SMask").as_dict().is_some() {
            return true;
        }
        if below_one(&self.document.get_key(dict, "ca"))
            || below_one(&self.document.get_key(dict, "CA"))
        {
            return true;
        }
        blends(self.document, dict)
    }

    /// A `cs` or `CS` operator, which selects a colour space by name.
    ///
    /// §8.6.8 lets the operand be one of the four family names outright; anything else is a key
    /// into the `/ColorSpace` subdictionary of the resources in force, and a key that reaches
    /// nothing is a resource the stream referenced and its resources did not define.
    fn set_space(&mut self, name: &[u8], stroking: bool, state: &mut State, context: &Context<'_>) {
        let object = Object::Name(Name::new(name.to_vec()));
        let resolved = if names_a_family(name) {
            object
        } else {
            match self.look_up("ColorSpace", name, context, context.what) {
                Some(entry) => resolve_space(self.document, &entry, context.resources, 0),
                None => return,
            }
        };
        let selected = if is_pattern(self.document, &resolved) {
            Selected::Pattern
        } else {
            Selected::Plain
        };
        let profile = icc_profile(self.document, &resolved, context.resources, 0);
        let cmyk = profile
            .as_ref()
            .is_some_and(|profile| profile.family == Some(DeviceFamily::Cmyk));
        if stroking {
            state.stroke = selected;
            state.stroke_icc_cmyk = cmyk;
        } else {
            state.fill = selected;
            state.fill_icc_cmyk = cmyk;
        }
        let place = Where::page(context.page);
        self.record_space(
            &resolved,
            &place,
            context,
            "a colour space operator",
            state.blending.kind,
            state.blending.profile.as_ref(),
        );
    }

    /// A painting operator, which marks the page in whatever space is in force.
    ///
    /// The only case this adds to what selection already recorded is §8.6.8's initial space: a
    /// stream that paints without ever naming a colour space is painting in `DeviceGray`, and
    /// the file never wrote the operator that would have said so.
    fn paint(&mut self, fill: bool, stroke: bool, state: &State, context: &Context<'_>) {
        let implicit = (fill && state.fill == Selected::Initial)
            || (stroke && state.stroke == Selected::Initial);
        if implicit {
            self.record_colour(
                (DeviceFamily::Gray, Route::Direct),
                context,
                "the initial colour space, which no operator replaced",
                state.blending.kind,
            );
        }
        if fill && state.fill_icc_cmyk {
            self.record_icc_paint(false, state.overprint.fill, state, context);
        }
        if stroke && state.stroke_icc_cmyk {
            self.record_icc_paint(true, state.overprint.stroke, state, context);
        }
    }

    /// One side of a painting operator that marked the page in an `ICCBased` CMYK space.
    fn record_icc_paint(
        &mut self,
        stroking: bool,
        overprinting: bool,
        state: &State,
        context: &Context<'_>,
    ) {
        let mode = state.overprint.mode;
        if !self.first_time(Observation::Overprint(
            context.page,
            context.what,
            stroking,
            overprinting,
            mode,
        )) {
            return;
        }
        self.survey.icc_paints.push(IccCmykPaint {
            page: context.page,
            place: Where::page(context.page),
            what: context.what,
            stroking,
            overprinting,
            mode,
        });
    }

    /// A `scn` or `SCN` operand naming a pattern, which is content or a shading of its own.
    fn pattern(&mut self, name: &[u8], state: &State, context: &Context<'_>) {
        let Some(entry) = self.look_up("Pattern", name, context, context.what) else {
            return;
        };
        let id = entry.as_reference();
        match self.document.resolve(&entry) {
            // A tiling pattern is a content stream, read against its own resources where it
            // states them — which is why a `/DefaultCMYK` on the page does not reach into one.
            Object::Stream(stream) => {
                let own = self.document.get_key(&stream.dict, "Resources");
                let stated = own.as_dict().is_some();
                let resources = own
                    .as_dict()
                    .cloned()
                    .unwrap_or_else(|| context.resources.clone());
                self.nested(
                    &stream,
                    id,
                    &resources,
                    context.page,
                    context.depth,
                    state,
                    "a tiling pattern",
                    stated,
                );
            }
            Object::Dictionary(dict) => self.shading(&dict, state, context),
            _ => {}
        }
    }

    /// A `sh` operator's operand, which names a shading in the resources.
    fn shading_resource(&mut self, name: &[u8], state: &State, context: &Context<'_>) {
        let Some(entry) = self.look_up("Shading", name, context, context.what) else {
            return;
        };
        let dict = match self.document.resolve(&entry) {
            Object::Dictionary(dict) => dict,
            Object::Stream(stream) => stream.dict.clone(),
            _ => return,
        };
        self.record_shading_space(&dict, state, context);
    }

    /// A shading pattern's `/Shading`, which is where its colour space is stated.
    fn shading(&mut self, pattern: &Dictionary, state: &State, context: &Context<'_>) {
        let shading = self.document.get_key(pattern, "Shading");
        let dict = match shading {
            Object::Dictionary(dict) => dict,
            Object::Stream(stream) => stream.dict.clone(),
            _ => return,
        };
        self.record_shading_space(&dict, state, context);
    }

    /// One shading dictionary's `/ColorSpace`: §8.7.4.3, Table 77.
    fn record_shading_space(&mut self, shading: &Dictionary, state: &State, context: &Context<'_>) {
        let Some(stated) = shading.get("ColorSpace") else {
            return;
        };
        let stated = self.document.resolve(stated);
        let place = Where::page(context.page);
        self.record_space(
            &stated,
            &place,
            context,
            "a shading",
            state.blending.kind,
            state.blending.profile.as_ref(),
        );
    }

    /// Records one device colour space selection at the site the walk is standing on.
    fn record_colour(
        &mut self,
        used: (DeviceFamily, Route),
        context: &Context<'_>,
        what: &'static str,
        blending: Option<SpaceKind>,
    ) {
        let place = Where::page(context.page);
        self.push_colour(used, place, context, what, blending);
    }

    /// Every device colour space one named space reaches, recorded at this site.
    fn record_space(
        &mut self,
        space: &Object,
        place: &Where,
        context: &Context<'_>,
        what: &'static str,
        blending: Option<SpaceKind>,
        blending_profile: Option<&IccProfile>,
    ) {
        for (profile, via) in icc_uses(self.document, space, context.resources) {
            self.push_icc(profile, via, place.clone(), context, what, blending_profile);
        }
        for used in device_uses(self.document, space, context.resources) {
            self.push_colour(used, place.clone(), context, what, blending);
        }
    }

    /// Records one `ICCBased` colour space selection, with the blending profile then in force.
    fn push_icc(
        &mut self,
        profile: IccProfile,
        via: Route,
        place: Where,
        context: &Context<'_>,
        what: &'static str,
        blending: Option<&IccProfile>,
    ) {
        let blending_id = blending.and_then(|blending| blending.id);
        if !self.first_time(Observation::Icc(
            context.page,
            what,
            via,
            profile.id,
            blending_id,
        )) {
            return;
        }
        let mut place = place.named("ICCBased");
        place.page = Some(context.page);
        self.survey.icc.push(IccSelection {
            page: context.page,
            place,
            what,
            via,
            profile,
            blending: blending.cloned(),
        });
    }

    /// The same, where the caller has a better witness than the page index.
    fn push_colour(
        &mut self,
        used: (DeviceFamily, Route),
        place: Where,
        context: &Context<'_>,
        what: &'static str,
        blending: Option<SpaceKind>,
    ) {
        let (family, via) = used;
        let default = context.defaults[family.index()];
        if !self.first_time(Observation::Colour(
            context.page,
            family,
            via,
            what,
            default,
            blending,
        )) {
            return;
        }
        let mut place = place.named(family.name());
        place.page = Some(context.page);
        self.survey.colours.push(DeviceColour {
            family,
            page: context.page,
            place,
            what,
            via,
            default,
            blending,
        });
    }

    /// Looks one named resource up in the resources in force, recording a miss.
    ///
    /// ISO 32000-2 §7.8.3 makes a resource dictionary a table of subdictionaries keyed by
    /// category, so a name that reaches no entry is a name the content stream referenced and its
    /// resources did not define — which is the whole of ISO 19005-4 section 6.2.2's third sentence.
    fn look_up(
        &mut self,
        category: &'static str,
        name: &[u8],
        context: &Context<'_>,
        what: &'static str,
    ) -> Option<Object> {
        // The name was referenced whether or not the resources define it, which is the fact
        // ISO 19005 section 6.2.2's second requirement turns on.
        if let Some(record) = context.record
            && let Some(opened) = self.survey.streams.get_mut(record)
        {
            opened.referenced = true;
        }
        let table = self.document.get_key(context.resources, category);
        let found = table
            .as_dict()
            .and_then(|table| table.get_by_name(&Name::new(name.to_vec())))
            .cloned();
        if found.is_none() {
            let printed = String::from_utf8_lossy(name).into_owned();
            if self.first_time(Observation::Missing(
                context.page,
                category,
                printed.clone(),
            )) {
                self.survey.missing.push(MissingResource {
                    page: context.page,
                    category,
                    name: printed,
                    what,
                });
            }
        }
        found
    }

    /// Whether an observation is new, and worth the entry it would cost.
    fn first_time(&mut self, observation: Observation) -> bool {
        self.seen.len() < MAX_OBSERVATIONS && self.seen.insert(observation)
    }

    /// Widens [`ContentLiterals`]' two integer extremes by one operand.
    ///
    /// Two comparisons per integer token, which is what putting this on the walk's hottest path
    /// costs; `examples/cost` is where that was checked rather than assumed.
    fn note_integer(&mut self, value: i64, page: usize) {
        let literals = &mut self.survey.literals;
        if literals
            .largest_integer
            .is_none_or(|held| value > held.value)
        {
            literals.largest_integer = Some(Extreme { value, page });
        }
        if literals
            .smallest_integer
            .is_none_or(|held| value < held.value)
        {
            literals.smallest_integer = Some(Extreme { value, page });
        }
    }

    /// Widens [`ContentLiterals`]' two real extremes by one operand, as magnitudes.
    ///
    /// A magnitude that is not finite is left out: no PDF real *literal* spells one, so an
    /// infinity here is what the lexer did with a run of digits too long for a `f64` rather
    /// than what the file said, and reading a bound off it would report the reader's arithmetic.
    fn note_real(&mut self, value: f64, page: usize) {
        let value = value.abs();
        if !value.is_finite() {
            return;
        }
        let literals = &mut self.survey.literals;
        if literals
            .largest_real_magnitude
            .is_none_or(|held| value > held.value)
        {
            literals.largest_real_magnitude = Some(Extreme { value, page });
        }
        if value > 0.0
            && literals
                .smallest_real_magnitude
                .is_none_or(|held| value < held.value)
        {
            literals.smallest_real_magnitude = Some(Extreme { value, page });
        }
    }

    /// Widens the string or name extreme of [`ContentLiterals`] by one operand's length.
    fn note_length(&mut self, value: usize, page: usize, of: Extent) {
        let literals = &mut self.survey.literals;
        let held = match of {
            Extent::String => &mut literals.longest_string,
            Extent::Name => &mut literals.longest_name,
        };
        if held.is_none_or(|held| value > held.value) {
            *held = Some(Extreme { value, page });
        }
    }

    /// Records how deep a `q` stack stood, where it is deeper than anything seen before.
    fn note_nesting(&mut self, depth: usize, page: usize) {
        if self.survey.nesting.is_none_or(|held| depth > held.value) {
            self.survey.nesting = Some(Extreme { value: depth, page });
        }
    }

    /// Keeps one `/ActualText` value, once per page and string, within its byte budget.
    ///
    /// The budget is spent rather than compared, so a document cannot make this vector large by
    /// stating many distinct entries any more than by stating one enormous one.
    fn note_actual_text(&mut self, text: &[u8], page: usize) {
        if text.len() > self.actual_text_budget
            || !self.first_time(Observation::ActualText(page, text.to_vec()))
        {
            return;
        }
        self.actual_text_budget = self.actual_text_budget.saturating_sub(text.len());
        self.survey.actual_texts.push((page, text.to_vec()));
    }

    /// Opens one of §14.6.1's marked-content sequences, with what its property list says.
    fn begin_marked(&mut self, operands: &Operands<'_>, context: &Context<'_>) {
        let mark = match self.property_list(operands, context) {
            // `BMC`, whose sequence carries no property list at all: §14.6.1 gives it a tag and
            // nothing else, so nothing it encloses is covered by anything.
            PropertyList::Absent => Mark::default(),
            PropertyList::Unreadable => Mark {
                settles: true,
                mcid: None,
                suppressed: 0,
            },
            PropertyList::Read(list) => {
                self.note_language(&list, context.page);
                Mark {
                    // §14.9.4 puts the entry in the property list of a sequence, and what it
                    // replaces is everything the sequence encloses. The *value* is not read
                    // here: whether it is a text string or a name is that rule's question, and
                    // this one is only whether the entry stands.
                    settles: !self.document.get_key(&list, "ActualText").is_null(),
                    mcid: self.document.get_key(&list, "MCID").as_integer(),
                    suppressed: 0,
                }
            }
        };
        if self.marks.len() < MAX_MARK_DEPTH {
            self.marks.push(mark);
        } else if let Some(top) = self.marks.last_mut() {
            top.suppressed = top.suppressed.saturating_add(1);
            top.settles = true;
        }
    }

    /// §14.6.2's property list, written into the operands or named through the resources.
    ///
    /// **Not read through [`Walk::look_up`], deliberately.** That reader records a name the
    /// resources do not define and marks the stream as having referenced a resource, and both
    /// of those are read by ISO 19005 section 6.2.2's rows. Reaching a property list is a new
    /// route to both facts, so taking it here would change what two other requirements say
    /// about documents nobody has looked at; it is a change worth making on its own evidence
    /// rather than as a side effect of this one.
    fn property_list<'o>(
        &self,
        operands: &Operands<'o>,
        context: &Context<'_>,
    ) -> PropertyList<'o> {
        if let Some(inline) = operands.property {
            return PropertyList::Read(Cow::Borrowed(inline));
        }
        // §14.6.2's second form: the tag and then a name, so the name is the last of the two.
        if operands.names.len() < 2 {
            return PropertyList::Absent;
        }
        let Some(name) = operands.names.last() else {
            return PropertyList::Absent;
        };
        let table = self.document.get_key(context.resources, "Properties");
        let found = table
            .as_dict()
            .and_then(|table| table.get_by_name(&Name::new(name.clone())))
            .cloned();
        let Some(found) = found else {
            return PropertyList::Unreadable;
        };
        match self.document.resolve(&found) {
            Object::Dictionary(dict) => PropertyList::Read(Cow::Owned(dict)),
            _ => PropertyList::Unreadable,
        }
    }

    /// Keeps a `/Lang` a marked-content property list stated, once per page and value.
    ///
    /// ISO 19005-2 section 6.7.4 requires a `/Lang` that is present to be a language identifier
    /// wherever it stands, and a property list is one of the three places it can stand.
    fn note_language(&mut self, list: &Dictionary, page: usize) {
        let stated = self.document.get_key(list, "Lang");
        if stated.is_null() {
            return;
        }
        let value = stated
            .as_string()
            .map(<[u8]>::to_vec)
            .ok_or_else(|| stated.type_name());
        if !self.first_time(Observation::Language(page, value.clone())) {
            return;
        }
        self.survey.languages.push(MarkedLanguage { page, value });
    }

    /// Reads a dictionary a content stream wrote inline, for the three things it can hold.
    ///
    /// Its operands are operands: ISO 19005-2 section 6.1.13's limits are on the values written
    /// inside a content stream, and a number written inside a property list is one of them —
    /// which is why this walks the whole dictionary rather than its top level. §14.9.4's entry
    /// is here where a producer wrote the list inline, which is no object at all: nothing that
    /// walks the cross-reference table can see it. And the *name* `ActualText` is the wider
    /// fact [`Survey::names_actual_text`] keeps, which no budget may narrow.
    ///
    /// Each entry costs one token of the walk's budget. That is an under-count of what the
    /// tokens of a dictionary really were — a key and its value are at least two — and it is
    /// the direction that keeps a bound from ending a walk early over an estimate.
    fn note_inline_dictionary(&mut self, dict: &Dictionary, page: usize) {
        self.note_inline_entries(dict, page, 0);
    }

    /// The entries of one dictionary written inline, at a stated nesting depth.
    fn note_inline_entries(&mut self, dict: &Dictionary, page: usize, depth: u32) {
        for (name, value) in dict.iter() {
            self.survey.names_actual_text |= name.as_bytes() == b"ActualText";
            self.note_length(name.as_bytes().len(), page, Extent::Name);
            self.note_inline_object(value, page, Some(name), depth.saturating_add(1));
        }
    }

    /// One value inside a dictionary a content stream wrote inline; see above.
    fn note_inline_object(&mut self, object: &Object, page: usize, key: Option<&Name>, depth: u32) {
        if depth >= MAX_INLINE_DEPTH || self.budget == 0 {
            return;
        }
        self.budget = self.budget.saturating_sub(1);
        match object {
            Object::Integer(value) => self.note_integer(*value, page),
            Object::Real(value) => self.note_real(*value, page),
            Object::Name(name) => self.note_length(name.as_bytes().len(), page, Extent::Name),
            Object::String(text) => {
                self.note_length(text.len(), page, Extent::String);
                if key.is_some_and(|key| key.as_bytes() == b"ActualText") {
                    self.note_actual_text(text, page);
                }
            }
            Object::Array(items) => {
                for item in items {
                    self.note_inline_object(item, page, None, depth.saturating_add(1));
                }
            }
            Object::Dictionary(dict) => self.note_inline_entries(dict, page, depth),
            _ => {}
        }
    }

    /// Records a keyword no operator summary lists, once per page, stream kind and spelling.
    fn note_unlisted_operator(&mut self, word: &[u8], page: usize, origin: Origin) {
        if !self.first_time(Observation::Operator(page, origin.what, word.to_vec())) {
            return;
        }
        let place = origin
            .record
            .and_then(|at| self.survey.streams.get(at))
            .map_or_else(|| Where::page(page), |stream| stream.place.clone());
        self.survey.unlisted.push(UnlistedOperator {
            page,
            place,
            what: origin.what,
            spelling: word.to_vec(),
        });
    }

    /// §8.6.5.6's default colour space for one family, as the resources in force state it.
    fn default_space(&self, resources: &Dictionary, family: DeviceFamily) -> Option<SpaceKind> {
        let table = self.document.get_key(resources, "ColorSpace");
        let stated = table.as_dict()?.get(family.default_key())?.clone();
        let stated = self.document.resolve(&stated);
        Some(classify(self.document, &stated, resources, 0))
    }

    /// Opens one nested content stream and runs it against the resources given.
    #[expect(
        clippy::too_many_arguments,
        reason = "the walk's whole context travels together, and naming a struct for one call \
                  site would hide which of the seven a caller changed"
    )]
    fn nested(
        &mut self,
        stream: &Stream,
        id: Option<ObjectId>,
        resources: &Dictionary,
        page: usize,
        depth: u32,
        state: &State,
        description: &'static str,
        own_resources: bool,
    ) {
        if depth >= MAX_FORM_DEPTH || self.opened >= MAX_STREAMS {
            return;
        }
        // A stream reached twice on one page draws the same marks twice, and following it once
        // is what stops a `/Resources` cycle from becoming an unbounded walk.
        if let Some(id) = id
            && !self.visited.insert((page, id))
        {
            return;
        }
        let Ok(content) = NestedContent::of(self.document, stream, description.to_owned()) else {
            return;
        };
        self.opened = self.opened.saturating_add(1);
        let place = id.map_or_else(|| Where::page(page), Where::object);
        let origin = self.open_record(page, id, place, description, own_resources);
        let mut reader = content.reader();
        self.run(
            &mut reader,
            resources,
            page,
            depth.saturating_add(1),
            state,
            origin,
        );
    }
}

/// Widens what is known about one string by one showing of it.
///
/// A showing an enclosing sequence settled contributes nothing at all: `bare` is false and
/// `identifiers` is empty, so this leaves the record as it found it. See [`Replacement`].
fn widen(replacement: &mut Replacement, bare: bool, enclosing: &[Mark], page: usize) {
    replacement.bare |= bare;
    for identifier in enclosing.iter().filter_map(|mark| mark.mcid) {
        let identifier = (page, identifier);
        if replacement.identifiers.contains(&identifier) {
            continue;
        }
        if replacement.identifiers.len() >= MAX_IDENTIFIERS {
            replacement.elided = true;
            break;
        }
        replacement.identifiers.push(identifier);
    }
}

/// The operands standing before an operator that this walk keeps.
struct Operands<'a> {
    /// The names, at most the first two.
    names: &'a [Vec<u8>],
    /// Every string operand, which is what a text-showing operator draws.
    strings: &'a [Vec<u8>],
    /// The last integer.
    integer: Option<i64>,
    /// §14.6.2's property list, where one was written into the operands rather than named.
    property: Option<&'a Dictionary>,
}

impl Operands<'_> {
    /// The first name operand, which `Tf`, `Do`, `gs`, `cs` and `sh` each take.
    fn first(&self) -> Option<&Vec<u8>> {
        self.names.first()
    }

    /// The last name operand, which is where `scn` states a pattern.
    fn last(&self) -> Option<&Vec<u8>> {
        self.names.last()
    }
}

/// What stood where a marked-content operator's property list belongs.
///
/// The read case is a [`Cow`] so that the common one costs nothing: a property list written into
/// the operands is already built and is lent, where one named through the resources has to be
/// resolved out of the document and is owned.
enum PropertyList<'a> {
    /// None was written: `BMC`, or a `BDC` whose operands state no list.
    Absent,
    /// One this walk read, whether it was written inline or named through the resources.
    Read(Cow<'a, Dictionary>),
    /// One it could not: a name the resources do not define, or a value that is no dictionary.
    Unreadable,
}

/// Where in the document the walk is standing.
struct Context<'a> {
    /// The resource dictionary in force.
    resources: &'a Dictionary,
    /// §8.6.5.6's three default colour spaces, as the resources in force state them.
    ///
    /// Read once when the stream is opened rather than once per colour operator, and that is a
    /// measured decision: ISO 32000-2's own specification selects a device colour space 317 127
    /// times over its 1 023 pages, and resolving `/ColorSpace` out of the resources at each of
    /// them clones a dictionary that cannot have changed — the resource dictionary in force is
    /// fixed for the length of one content stream. Indexed by [`DeviceFamily::index`].
    defaults: [DefaultSpace; 3],
    /// The zero-based index of the page the content is drawn on.
    page: usize,
    /// How many nested streams deep the walk is.
    depth: u32,
    /// What kind of content stream is being read, for the sentence a finding prints.
    what: &'static str,
    /// The stream's entry in [`Survey::streams`], for [`Walk::look_up`] to mark.
    record: Option<usize>,
    /// How deep [`Walk::marks`] was when this content stream began; see [`Walk::run`].
    marks_base: usize,
}

/// Whether a dictionary's `BM` entry sets a blend mode other than `Normal`.
///
/// ISO 32000-2 Annex Q.2's fourth condition, applied to the two dictionaries that carry the
/// entry: a graphics state parameter dictionary (§8.4.5) and, since PDF 2.0, an annotation
/// (§12.5.2). `Compatible` is treated as `Normal` because §11.3.5 defines it as that mode under
/// its older name, so a file that states it has not asked for a blend at all. The array form is
/// the older one, whose entries a processor picks the first recognised mode from.
fn blends(document: &Document, dict: &Dictionary) -> bool {
    let other_than_normal = |object: &Object| {
        object
            .as_name()
            .is_some_and(|mode| mode.as_bytes() != b"Normal" && mode.as_bytes() != b"Compatible")
    };
    match &document.get_key(dict, "BM") {
        Object::Array(modes) => modes
            .iter()
            .any(|mode| other_than_normal(&document.resolve(mode))),
        stated => other_than_normal(stated),
    }
}

/// Whether a number-valued entry is present and less than one, for Annex Q.2's `ca` and `CA`.
fn below_one(object: &Object) -> bool {
    object.as_number().is_some_and(|value| value < 1.0)
}

/// Resolves a colour space operand to the object that defines it.
///
/// A `cs` operand is either one of §8.6.8's four family names — which name the space directly —
/// or a key into the `/ColorSpace` subdictionary of the resources (§7.8.3).
fn resolve_space(
    document: &Document,
    object: &Object,
    resources: &Dictionary,
    depth: usize,
) -> Object {
    if depth > MAX_SPACE_DEPTH {
        return Object::Null;
    }
    let resolved = document.resolve(object);
    let Some(name) = resolved.as_name() else {
        return resolved;
    };
    if names_a_family(name.as_bytes()) {
        return resolved;
    }
    let table = document.get_key(resources, "ColorSpace");
    let Some(entry) = table.as_dict().and_then(|table| table.get_by_name(name)) else {
        return resolved;
    };
    resolve_space(document, entry, resources, depth.saturating_add(1))
}

/// Whether a `cs` operand is one of the family names §8.6.8 lets a content stream use directly.
fn names_a_family(name: &[u8]) -> bool {
    matches!(
        name,
        b"DeviceGray" | b"DeviceRGB" | b"DeviceCMYK" | b"Pattern" | b"G" | b"RGB" | b"CMYK"
    )
}

/// Whether a resolved colour space is a `Pattern` space: §8.7.3.
fn is_pattern(document: &Document, space: &Object) -> bool {
    match space {
        Object::Name(name) => name.as_bytes() == b"Pattern",
        Object::Array(items) => items
            .first()
            .map(|first| document.resolve(first))
            .and_then(|first| first.as_name().map(|name| name.as_bytes() == b"Pattern"))
            .unwrap_or(false),
        _ => false,
    }
}

/// What a colour space object is, for the requirements that tell device spaces from CIE-based
/// ones.
///
/// Names are looked up in the resources' `/ColorSpace` subdictionary, an `Indexed` space answers
/// with its base and a `Pattern` space with its underlying space, because ISO 19005 section 6.2.4.5
/// makes the requirements of section 6.2.4 apply to exactly those.
pub(crate) fn classify(
    document: &Document,
    space: &Object,
    resources: &Dictionary,
    depth: usize,
) -> SpaceKind {
    if depth > MAX_SPACE_DEPTH {
        return SpaceKind::Unknown;
    }
    let space = resolve_space(document, space, resources, 0);
    match &space {
        Object::Name(name) => match name.as_bytes() {
            b"DeviceGray" | b"G" => SpaceKind::Device(DeviceFamily::Gray),
            b"DeviceRGB" | b"RGB" => SpaceKind::Device(DeviceFamily::Rgb),
            b"DeviceCMYK" | b"CMYK" => SpaceKind::Device(DeviceFamily::Cmyk),
            _ => SpaceKind::Unknown,
        },
        Object::Array(items) => classify_array(document, items, resources, depth),
        _ => SpaceKind::Unknown,
    }
}

/// Every device colour space one colour space object reaches, and by which route.
///
/// A device space names itself; an `Indexed` or `Pattern` space is asked for the space beneath it,
/// which ISO 19005 section 6.2.4.5 makes subject to the same restrictions; a `Separation` or
/// `DeviceN` space is asked for its alternate, which section 6.2.4.4 makes subject to section
/// 6.2.4.3.
pub(crate) fn device_uses(
    document: &Document,
    space: &Object,
    resources: &Dictionary,
) -> Vec<(DeviceFamily, Route)> {
    let mut found = Vec::new();
    collect_uses(document, space, resources, Route::Direct, 0, &mut found);
    found
}

/// One colour space object's contribution to [`device_uses`].
fn collect_uses(
    document: &Document,
    space: &Object,
    resources: &Dictionary,
    route: Route,
    depth: usize,
    found: &mut Vec<(DeviceFamily, Route)>,
) {
    if depth > MAX_SPACE_DEPTH {
        return;
    }
    let space = resolve_space(document, space, resources, 0);
    let deeper = depth.saturating_add(1);
    let items = match &space {
        Object::Name(_) => {
            if let SpaceKind::Device(family) = classify(document, &space, resources, depth) {
                found.push((family, route));
            }
            return;
        }
        Object::Array(items) => items,
        _ => return,
    };
    let Some(family) = items.first().map(|first| document.resolve(first)) else {
        return;
    };
    let Some(family) = family.as_name().map(|name| name.as_bytes().to_vec()) else {
        return;
    };
    match family.as_slice() {
        b"DeviceGray" | b"G" => found.push((DeviceFamily::Gray, route)),
        b"DeviceRGB" | b"RGB" => found.push((DeviceFamily::Rgb, route)),
        b"DeviceCMYK" | b"CMYK" => found.push((DeviceFamily::Cmyk, route)),
        b"Indexed" | b"I" | b"Pattern" => {
            if let Some(base) = items.get(1) {
                collect_uses(document, base, resources, route.under(), deeper, found);
            }
        }
        // §8.6.6.4, Table 74: the alternate space is the third element of both arrays.
        b"Separation" | b"DeviceN" => {
            if let Some(alternate) = items.get(2) {
                collect_uses(
                    document,
                    alternate,
                    resources,
                    Route::Alternate,
                    deeper,
                    found,
                );
            }
        }
        _ => {}
    }
}

/// The array form of a colour space: §8.6, Table 61.
fn classify_array(
    document: &Document,
    items: &[Object],
    resources: &Dictionary,
    depth: usize,
) -> SpaceKind {
    let Some(family) = items.first().map(|first| document.resolve(first)) else {
        return SpaceKind::Unknown;
    };
    let Some(family) = family.as_name().map(|name| name.as_bytes().to_vec()) else {
        return SpaceKind::Unknown;
    };
    let deeper = depth.saturating_add(1);
    match family.as_slice() {
        b"DeviceGray" | b"G" => SpaceKind::Device(DeviceFamily::Gray),
        b"DeviceRGB" | b"RGB" => SpaceKind::Device(DeviceFamily::Rgb),
        b"DeviceCMYK" | b"CMYK" => SpaceKind::Device(DeviceFamily::Cmyk),
        b"CalGray" => SpaceKind::Independent(Some(DeviceFamily::Gray)),
        b"CalRGB" => SpaceKind::Independent(Some(DeviceFamily::Rgb)),
        // §8.6.5.4's Lab has three components and is not an RGB-based space, and §11.3.4 bars
        // it from being a blending space at all — so it is independent of no family.
        b"Lab" => SpaceKind::Independent(None),
        b"ICCBased" => SpaceKind::Independent(icc_components(document, items.get(1))),
        b"Indexed" | b"I" => items.get(1).map_or(SpaceKind::Unknown, |base| {
            classify(document, base, resources, deeper)
        }),
        b"Pattern" => items.get(1).map_or(SpaceKind::Unknown, |base| {
            classify(document, base, resources, deeper)
        }),
        b"Separation" | b"DeviceN" => SpaceKind::Colourant,
        _ => SpaceKind::Unknown,
    }
}

/// Which family an `ICCBased` space's component count corresponds to: §8.6.5.5, Table 66's `N`.
fn icc_components(document: &Document, stream: Option<&Object>) -> Option<DeviceFamily> {
    let stream = document.resolve(stream?);
    let Object::Stream(stream) = stream else {
        return None;
    };
    icc_family(document, &stream.dict)
}

/// The same, from the profile stream's dictionary.
fn icc_family(document: &Document, dict: &Dictionary) -> Option<DeviceFamily> {
    match document.get_key(dict, "N").as_integer() {
        Some(1) => Some(DeviceFamily::Gray),
        Some(3) => Some(DeviceFamily::Rgb),
        Some(4) => Some(DeviceFamily::Cmyk),
        _ => None,
    }
}

/// Every `ICCBased` profile one colour space object reaches, and by which route.
///
/// The `ICCBased` twin of [`device_uses`], and deliberately the same shape: the three subclauses
/// that restrict a colour space restrict it wherever it stands, and each of the three routes is a
/// different clause's business. Section 6.2.4.5 sends the base of an `Indexed` and the underlying
/// space of a `Pattern` to the rest of section 6.2.4; section 6.2.4.4 sends a `Separation`'s or
/// `DeviceN`'s alternate space there in the same words.
///
/// **The alternate used to be left out, and that was a misreading.** The argument for leaving it
/// out was that a colourant space paints through its tint transform rather than in the alternate
/// directly, so an alternate profile is a use that never happened. But ISO 32000-2 §8.6.6.4 is
/// explicit that the tint transform's output *is* interpreted in the alternate space, which is why
/// the sibling rule over device colours has always counted an alternate `DeviceCMYK` as a use of
/// `DeviceCMYK` — and ISO 19005-4 section 6.2.4.4 says in one sentence that the alternate space
/// shall obey all the restrictions of section 6.2.4.2 and section 6.2.4.3, without distinguishing
/// them. The corpus's `6-2-4-4-t01-fail-i` and `-fail-j` are exactly this case, and were missed for
/// as long as the two halves of section 6.2.4.4 were read differently.
fn icc_uses(
    document: &Document,
    space: &Object,
    resources: &Dictionary,
) -> Vec<(IccProfile, Route)> {
    let mut found = Vec::new();
    collect_icc(document, space, resources, Route::Direct, 0, &mut found);
    found
}

/// One colour space object's contribution to [`icc_uses`].
fn collect_icc(
    document: &Document,
    space: &Object,
    resources: &Dictionary,
    route: Route,
    depth: usize,
    found: &mut Vec<(IccProfile, Route)>,
) {
    if depth > MAX_SPACE_DEPTH {
        return;
    }
    let space = resolve_space(document, space, resources, 0);
    let Object::Array(items) = &space else {
        return;
    };
    let Some(family) = items.first().map(|first| document.resolve(first)) else {
        return;
    };
    let Some(family) = family.as_name().map(|name| name.as_bytes().to_vec()) else {
        return;
    };
    let deeper = depth.saturating_add(1);
    match family.as_slice() {
        b"ICCBased" => {
            if let Some(profile) = icc_entry(document, items.get(1)) {
                found.push((profile, route));
            }
        }
        b"Indexed" | b"I" | b"Pattern" => {
            if let Some(base) = items.get(1) {
                collect_icc(document, base, resources, route.under(), deeper, found);
            }
        }
        // §8.6.6.4, Table 74: the alternate space is the third element of both arrays.
        b"Separation" | b"DeviceN" => {
            if let Some(alternate) = items.get(2) {
                collect_icc(
                    document,
                    alternate,
                    resources,
                    Route::Alternate,
                    deeper,
                    found,
                );
            }
        }
        _ => {}
    }
}

/// The second element of an `ICCBased` array, as the profile it names.
fn icc_entry(document: &Document, entry: Option<&Object>) -> Option<IccProfile> {
    let entry = entry?;
    let id = entry.as_reference();
    let Object::Stream(stream) = document.resolve(entry) else {
        return None;
    };
    let family = icc_family(document, &stream.dict);
    Some(IccProfile { id, stream, family })
}

/// The profile an `ICCBased` colour space is formed from, where the space is one.
///
/// The single-answer form [`record_group_space`](Walk::record_group_space) needs for §11.3.4's
/// blending colour space, which is a device or CIE-based space and never a colourant one — so
/// unlike [`icc_uses`] it does not follow an alternate, and the two are not the same question.
fn icc_profile(
    document: &Document,
    space: &Object,
    resources: &Dictionary,
    depth: usize,
) -> Option<IccProfile> {
    if depth > MAX_SPACE_DEPTH {
        return None;
    }
    let space = resolve_space(document, space, resources, 0);
    let Object::Array(items) = &space else {
        return None;
    };
    let family = items.first().map(|first| document.resolve(first))?;
    let family = family.as_name()?.as_bytes().to_vec();
    match family.as_slice() {
        b"ICCBased" => {
            let entry = items.get(1)?;
            let id = entry.as_reference();
            let Object::Stream(stream) = document.resolve(entry) else {
                return None;
            };
            let family = icc_family(document, &stream.dict);
            Some(IccProfile { id, stream, family })
        }
        b"Indexed" | b"I" | b"Pattern" => {
            icc_profile(document, items.get(1)?, resources, depth.saturating_add(1))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use pdf_syntax::{Dictionary, Document, Name, Object};

    use super::{DeviceFamily, Replacement, Route, SpaceKind, Survey, classify, device_uses};

    /// A file assembled from object bodies, with a cross-reference table over them.
    ///
    /// The walk's whole subject is what a real content stream does, so the shortest honest
    /// fixture is a real file the real parser opens.
    fn document_of(objects: &[&str]) -> Document {
        use std::fmt::Write as _;
        let mut out = String::from("%PDF-2.0\n");
        let mut offsets = Vec::new();
        for (index, body) in objects.iter().enumerate() {
            offsets.push(out.len());
            let number = index.saturating_add(1);
            let _ = writeln!(out, "{number} 0 obj {body} endobj");
        }
        let start = out.len();
        let size = objects.len().saturating_add(1);
        let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
        for offset in &offsets {
            let _ = writeln!(out, "{offset:010} 00000 n ");
        }
        let _ = write!(
            out,
            "trailer << /Size {size} /Root 1 0 R >>\nstartxref\n{start}\n%%EOF\n"
        );
        Document::open(out.into_bytes()).unwrap_or_else(|_| Document::empty())
    }

    /// One page whose content and resources are the test's, with a stream of the right length.
    fn page_with(content: &str, page_extra: &str, more: &[&str]) -> Document {
        let stream = format!(
            "<< /Length {} >> stream\n{content}\nendstream",
            content.len()
        );
        let page = format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Contents 4 0 R {page_extra} >>"
        );
        let mut objects = vec![
            "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
            page,
            stream,
        ];
        objects.extend(more.iter().map(|body| (*body).to_owned()));
        let borrowed: Vec<&str> = objects.iter().map(String::as_str).collect();
        document_of(&borrowed)
    }

    /// An array object written as source, for the classifier's tests.
    fn parsed(document: &Document, index: usize) -> Object {
        document.get(pdf_syntax::ObjectId::new(
            u32::try_from(index).unwrap_or(1),
            0,
        ))
    }

    /// Section 6.2.4.4's alternate space and section 6.2.4.5's base are found, and each says how it
    /// was reached — which is what makes the three subclauses three rows over one walk.
    #[test]
    fn a_device_space_carries_the_route_that_reached_it() {
        let document = document_of(&[
            "<< /Type /Catalog >>",
            "[/Indexed /DeviceRGB 1 <00>]",
            "[/Separation /Spot /DeviceCMYK 5 0 R]",
            "[/ICCBased 6 0 R]",
            "<< /FunctionType 2 /Domain [0 1] /C0 [0] /C1 [1] /N 1 >>",
            "<< /N 3 /Length 0 >> stream\n\nendstream",
        ]);
        let resources = Dictionary::new();
        assert_eq!(
            device_uses(&document, &parsed(&document, 2), &resources),
            vec![(DeviceFamily::Rgb, Route::Underlying)],
            "an Indexed base is section 6.2.4.5's underlying space"
        );
        assert_eq!(
            device_uses(&document, &parsed(&document, 3), &resources),
            vec![(DeviceFamily::Cmyk, Route::Alternate)],
            "a Separation's third element is section 6.2.4.4's alternate space"
        );
        assert!(
            device_uses(&document, &parsed(&document, 4), &resources).is_empty(),
            "an ICCBased space is not a device space at all"
        );
        assert_eq!(
            classify(&document, &parsed(&document, 4), &resources, 0),
            SpaceKind::Independent(Some(DeviceFamily::Rgb)),
            "three components make it the RGB-based space section 6.2.4.3's second licence names"
        );
    }

    /// A name is a key into the resources' `/ColorSpace`, and one that reaches nothing is a
    /// resource ISO 19005-4 section 6.2.2 says the dictionary should have defined.
    #[test]
    fn a_colour_space_name_is_resolved_through_the_resources_and_a_miss_is_reported() {
        let document = page_with(
            "/CS0 cs 0 0 0 sc /Missing cs",
            "/Resources << /ColorSpace << /CS0 /DeviceRGB >> >>",
            &[],
        );
        let survey = Survey::of(&document);
        assert_eq!(
            survey
                .device_colours()
                .iter()
                .filter(|colour| colour.family == DeviceFamily::Rgb)
                .count(),
            1,
            "the named space is DeviceRGB"
        );
        let missing: Vec<&str> = survey
            .missing_resources()
            .iter()
            .map(|entry| entry.name.as_str())
            .collect();
        assert_eq!(missing, vec!["Missing"]);
    }

    /// §8.6.5.6's default is read from the resources in force, and only its own family's key.
    #[test]
    fn the_default_in_force_is_reported_beside_the_colour_it_would_license() {
        let document = page_with(
            "1 0 0 rg 0 0 1 1 re B",
            "/Resources << /ColorSpace << /DefaultRGB [/CalRGB << /WhitePoint [1 1 1] >>] >> >>",
            &[],
        );
        let survey = Survey::of(&document);
        let rgb = survey
            .device_colours()
            .iter()
            .find(|colour| colour.family == DeviceFamily::Rgb)
            .expect("the rg operator selects DeviceRGB");
        let calibrated = Some(SpaceKind::Independent(Some(DeviceFamily::Rgb)));
        assert_eq!(rgb.default.in_force, calibrated);
        assert_eq!(
            rgb.default.explicit, calibrated,
            "the page states its own Resources, so the two readings agree"
        );
        let gray = survey
            .device_colours()
            .iter()
            .find(|colour| colour.family == DeviceFamily::Gray)
            .expect("B strokes as well as fills, in §8.6.8's initial DeviceGray");
        assert_eq!(gray.default.in_force, None, "no DefaultGray is in force");
    }

    /// `TechNote 0010` A028: a form `XObject` stating no `Resources` reads no default at all.
    ///
    /// The two fields of [`DefaultSpace`] part company exactly here, which is the whole reason
    /// the type has two: the page's `DefaultGray` is what the resources in force say, and it is
    /// not defined in any dictionary explicitly associated with the form's own content stream.
    #[test]
    fn a_form_that_states_no_resources_reads_no_default_colour_space() {
        let inner = "0 g 0 0 1 1 re f";
        let form = format!(
            "<< /Type /XObject /Subtype /Form /BBox [0 0 1 1] /Length {} >> \
             stream\n{inner}\nendstream",
            inner.len()
        );
        let document = page_with(
            "/Fm0 Do",
            "/Resources << /ColorSpace << /DefaultGray [/CalGray << /WhitePoint [1 1 1] >>] >> \
             /XObject << /Fm0 5 0 R >> >>",
            &[&form],
        );
        let survey = Survey::of(&document);
        let gray = survey
            .device_colours()
            .iter()
            .find(|colour| colour.family == DeviceFamily::Gray)
            .expect("the form's `g` operator selects DeviceGray");
        assert_eq!(
            gray.default.in_force,
            Some(SpaceKind::Independent(Some(DeviceFamily::Gray))),
            "the page's dictionary is what §8.6.5.6 alone would read"
        );
        assert_eq!(
            gray.default.explicit, None,
            "A028: the form has no explicitly associated resources dictionary"
        );
    }

    /// ISO 32000-2 Annex Q.2's conditions, and the one that is not a condition.
    #[test]
    fn annex_q_finds_transparency_in_a_graphics_state_and_not_in_a_normal_blend() {
        let transparent = page_with(
            "/GS0 gs 0 0 1 1 re f",
            "/Resources << /ExtGState << /GS0 << /ca 0.5 >> >> >>",
            &[],
        );
        assert!(Survey::of(&transparent).page_is_transparent(0));

        let opaque = page_with(
            "/GS0 gs 0 0 1 1 re f",
            "/Resources << /ExtGState << /GS0 << /ca 1 /BM /Normal >> >> >>",
            &[],
        );
        assert!(!Survey::of(&opaque).page_is_transparent(0));
    }

    /// Annex Q.3: a form `XObject` that is a transparency group puts transparency on the page
    /// it is placed on, whatever its own content does.
    #[test]
    fn a_transparency_group_xobject_makes_its_page_transparent() {
        let document = page_with(
            "/Fm0 Do",
            "/Resources << /XObject << /Fm0 5 0 R >> >>",
            &["<< /Type /XObject /Subtype /Form /BBox [0 0 1 1] \
               /Group << /S /Transparency /CS /DeviceRGB >> /Length 0 >> stream\n\nendstream"],
        );
        let survey = Survey::of(&document);
        assert!(survey.page_is_transparent(0));
        assert_eq!(
            survey.group_spaces().len(),
            1,
            "the group's CS is recorded so section 6.2.9's third paragraph can judge it"
        );
        assert_eq!(
            survey.group_spaces()[0].kind,
            SpaceKind::Device(DeviceFamily::Rgb)
        );
    }

    /// ISO 32000-2 §11.6.6: an isolated group's own `CS` becomes the blending space, and a
    /// non-isolated one inherits the page's instead.
    #[test]
    fn only_an_isolated_group_replaces_the_blending_colour_space() {
        let page_group = "/Group << /S /Transparency /CS [/CalRGB << /WhitePoint [1 1 1] >>] >>";
        let form = |isolated: &str| {
            format!(
                "<< /Type /XObject /Subtype /Form /BBox [0 0 1 1] \
                 /Group << /S /Transparency /CS /DeviceCMYK {isolated} >> \
                 /Length 9 >> stream\n1 0 0 rg\nendstream"
            )
        };
        for (isolated, expected) in [
            ("/I true", SpaceKind::Device(DeviceFamily::Cmyk)),
            ("", SpaceKind::Independent(Some(DeviceFamily::Rgb))),
        ] {
            let body = form(isolated);
            let document = page_with(
                "/Fm0 Do",
                &format!("/Resources << /XObject << /Fm0 5 0 R >> >> {page_group}"),
                &[body.as_str()],
            );
            let survey = Survey::of(&document);
            let inside = survey
                .device_colours()
                .iter()
                .find(|colour| colour.family == DeviceFamily::Rgb)
                .expect("the form's rg operator");
            assert_eq!(inside.blending, Some(expected), "with {isolated:?}");
        }
    }

    /// §8.6.8's initial colour space is a `DeviceGray` the file never named, and a page that
    /// paints without selecting one is painting in it.
    #[test]
    fn painting_without_selecting_a_space_is_a_use_of_the_initial_one() {
        let document = page_with("0 0 1 1 re f", "", &[]);
        let survey = Survey::of(&document);
        assert!(
            survey
                .device_colours()
                .iter()
                .any(|colour| colour.family == DeviceFamily::Gray && colour.via == Route::Direct),
            "the initial DeviceGray is what the fill was painted in"
        );
    }

    /// A font is recorded only where a show operator ran with it, which is the population
    /// `super::super::table::fonts`' embedding rule needs.
    #[test]
    fn a_font_is_rendered_only_when_a_show_operator_ran_with_it() {
        let document = page_with(
            "BT /F0 12 Tf (a) Tj 3 Tr /F1 12 Tf (b) Tj ET",
            "/Resources << /Font << /F0 5 0 R /F1 6 0 R >> >>",
            &[
                "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
                "<< /Type /Font /Subtype /Type1 /BaseFont /Courier >>",
            ],
        );
        let survey = Survey::of(&document);
        let rendered: Vec<(&str, bool)> = survey
            .fonts()
            .map(|font| (font.name.as_str(), font.rendered))
            .collect();
        assert!(rendered.contains(&("F0", true)));
        assert!(
            rendered.contains(&("F1", false)),
            "rendering mode 3 marks nothing"
        );
    }

    /// A name that is not a colour space family is looked up, and `Name` is compared by bytes.
    /// ISO 19005 section 6.2.2 turns on two facts per stream, and ISO 32000-2 §7.8.3 is where the
    /// second of them is defined: only a page dictionary may reach its resources by
    /// inheritance, and even there the entry is not explicit.
    #[test]
    fn a_stream_is_reported_with_whether_it_named_a_resource_and_owned_its_dictionary() {
        // The page states its own resources; the form does not, and names a colour space
        // through them; the second form states none and names nothing.
        let document = page_with(
            "/X0 Do /X1 Do",
            "/Resources << /XObject << /X0 5 0 R /X1 6 0 R >> \
             /ColorSpace << /CS0 /DeviceRGB >> >>",
            &[
                "<< /Type /XObject /Subtype /Form /BBox [0 0 1 1] /Length 12 >> stream\n\
                 /CS0 cs 0 sc\nendstream",
                "<< /Type /XObject /Subtype /Form /BBox [0 0 1 1] /Length 8 >> stream\n\
                 0 0 1 rg\nendstream",
            ],
        );
        let survey = Survey::of(&document);
        let opened: Vec<(&str, bool, bool)> = survey
            .opened_streams()
            .iter()
            .map(|stream| (stream.what, stream.referenced, stream.own_resources))
            .collect();
        assert_eq!(
            opened,
            vec![
                ("the page content", true, true),
                ("a form XObject", true, false),
                ("a form XObject", false, false),
            ]
        );
    }

    /// A page that inherits its resources from an ancestor of the page tree has no `Resources`
    /// entry of its own, which is what ISO 32000-2 §7.8.3's first bullet permits and ISO 19005
    /// Section 6.2.2 withdraws.
    #[test]
    fn a_page_that_inherits_its_resources_does_not_own_them() {
        let document = document_of(&[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 \
             /Resources << /ColorSpace << /CS0 /DeviceRGB >> >> >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Contents 4 0 R >>",
            "<< /Length 12 >> stream\n/CS0 cs 0 sc\nendstream",
        ]);
        let survey = Survey::of(&document);
        let opened = survey.opened_streams();
        assert_eq!(opened.len(), 1);
        assert!(opened[0].referenced, "the content names /CS0");
        assert!(
            !opened[0].own_resources,
            "the entry is on the page tree node rather than on the page"
        );
    }

    /// ISO 32000-2 §8.6.7 makes overprint mode act on a mark rather than on a selection, and
    /// Table 58 pairs `OP` with stroking and `op` with everything else.
    #[test]
    fn an_icc_cmyk_mark_carries_the_overprint_parameters_of_the_side_that_painted() {
        // Both spaces are the four-component ICCBased one; only the fill is painted.
        let document = page_with(
            "/GS0 gs /CS0 CS /CS0 cs 0 0 0 0 scn 0 0 1 1 re f",
            "/Resources << /ColorSpace << /CS0 [/ICCBased 5 0 R] >> \
             /ExtGState << /GS0 << /OP true /op false /OPM 1 >> >> >>",
            &["<< /N 4 /Length 4 >> stream\nabcd\nendstream"],
        );
        let survey = Survey::of(&document);
        let marks: Vec<(bool, bool, i64)> = survey
            .icc_cmyk_paints()
            .iter()
            .map(|paint| (paint.stroking, paint.overprinting, paint.mode))
            .collect();
        assert_eq!(
            marks,
            vec![(false, false, 1)],
            "the stroking space was never stroked with, and `op` governs the fill"
        );
    }

    /// Table 58: an `OP` entry with no `op` beside it sets both overprint parameters.
    #[test]
    fn an_overprint_entry_without_its_lower_case_twin_sets_both_parameters() {
        let document = page_with(
            "/GS0 gs /CS0 cs 0 0 0 0 scn 0 0 1 1 re f",
            "/Resources << /ColorSpace << /CS0 [/ICCBased 5 0 R] >> \
             /ExtGState << /GS0 << /OP true /OPM 1 >> >> >>",
            &["<< /N 4 /Length 4 >> stream\nabcd\nendstream"],
        );
        let survey = Survey::of(&document);
        let marks: Vec<(bool, bool, i64)> = survey
            .icc_cmyk_paints()
            .iter()
            .map(|paint| (paint.stroking, paint.overprinting, paint.mode))
            .collect();
        assert_eq!(marks, vec![(false, true, 1)]);
    }

    /// The selection is what ISO 19005-4 section 6.2.4.2's last requirement binds, so a profile
    /// that only sits in a resource dictionary is not one.
    #[test]
    fn an_icc_space_is_recorded_where_the_content_selects_it_and_not_where_it_only_sits() {
        let selected = page_with(
            "/CS0 cs 0 0 0 0 scn",
            "/Resources << /ColorSpace << /CS0 [/ICCBased 5 0 R] >> >>",
            &["<< /N 4 /Length 4 >> stream\nabcd\nendstream"],
        );
        let survey = Survey::of(&selected);
        assert_eq!(survey.icc_selections().len(), 1);
        assert_eq!(
            survey.icc_selections()[0].profile.family,
            Some(DeviceFamily::Cmyk)
        );
        let unused = page_with(
            "0 0 0 0 k",
            "/Resources << /ColorSpace << /CS0 [/ICCBased 5 0 R] >> >>",
            &["<< /N 4 /Length 4 >> stream\nabcd\nendstream"],
        );
        assert!(Survey::of(&unused).icc_selections().is_empty());
    }

    #[test]
    fn the_family_names_are_the_ones_the_base_standard_lets_a_stream_use_directly() {
        assert!(super::names_a_family(b"DeviceCMYK"));
        assert!(super::names_a_family(b"Pattern"));
        assert!(!super::names_a_family(b"CS0"));
        let _ = Name::new(b"CS0".to_vec());
    }

    /// The three cases the operator table has to tell apart, and the one that is neither.
    #[test]
    fn a_keyword_is_an_operator_the_walk_acts_on_an_operator_it_ignores_or_neither() {
        let listed = page_with("q 1 0 0 1 0 0 cm 0 0 5 5 re f BX EX Q", "", &[]);
        assert!(
            Survey::of(&listed).unlisted_operators().is_empty(),
            "every one of these is in ISO 32000's operator summary, whether or not this walk \
             acts on it"
        );
        let unlisted = page_with("q BX UnknownOperator EX Q", "", &[]);
        let survey = Survey::of(&unlisted);
        assert_eq!(
            survey
                .unlisted_operators()
                .iter()
                .map(|found| found.spelling.clone())
                .collect::<Vec<_>>(),
            vec![b"UnknownOperator".to_vec()],
            "the compatibility brackets exempt nothing, which is what section 6.2.2 says in as \
            many \
             words"
        );
        // §7.3.2's booleans and §7.3.9's null reach the walk as keywords and are operands.
        let objects = page_with("/OC << /Off true /On false /Nul null >> BDC EMC", "", &[]);
        assert!(Survey::of(&objects).unlisted_operators().is_empty());
    }

    /// One unlisted operator on one page in one kind of stream is one observation, however
    /// often it is written.
    #[test]
    fn an_unlisted_operator_is_reported_once_however_often_a_page_uses_it() {
        let repeated = page_with("Xyzzy Xyzzy Xyzzy Plugh", "", &[]);
        let survey = Survey::of(&repeated);
        assert_eq!(survey.unlisted_operators().len(), 2);
    }

    /// The extremes section 6.1.13's limits are read against, taken from the operands themselves.
    #[test]
    fn the_operands_extremes_are_kept_in_each_direction() {
        let document = page_with("7 -3 0.5 0 -12.25 /LongEnough (text) Tj", "", &[]);
        let survey = Survey::of(&document);
        let literals = survey.content_literals();
        assert_eq!(literals.largest_integer.map(|held| held.value), Some(7));
        assert_eq!(literals.smallest_integer.map(|held| held.value), Some(-3));
        assert_eq!(
            literals.largest_real_magnitude.map(|held| held.value),
            Some(12.25),
            "the largest magnitude, so that a bound stated as a magnitude can read it"
        );
        assert_eq!(
            literals.smallest_real_magnitude.map(|held| held.value),
            Some(0.5),
            "and the smallest non-zero one: a literal 0 is exactly representable and breaks no \
             limit, so it is not the extreme this keeps"
        );
        assert_eq!(literals.longest_string.map(|held| held.value), Some(4));
        assert_eq!(
            literals.longest_name.map(|held| held.value),
            Some(10),
            "measured on the decoded name, as the sibling rule over the objects measures it"
        );
    }

    /// §14.9.4's entry written into a `BDC` operator's operands, which is no object at all.
    #[test]
    fn an_actual_text_written_inline_is_kept_with_the_page_that_drew_it() {
        let inline = page_with("/Span << /ActualText (ffi) >> BDC EMC", "", &[]);
        let survey = Survey::of(&inline);
        assert!(survey.names_actual_text());
        assert_eq!(
            survey.inline_actual_texts(),
            [(0usize, b"ffi".to_vec())],
            "the string after the name is the entry, whatever dictionary stands around it"
        );

        // The name alone is the wider fact, and the rule that reads it needs the wider one:
        // a property list naming a resource states an `ActualText` this walk cannot value.
        let named_only = page_with("/Span /P0 BDC EMC /ActualText", "", &[]);
        let survey = Survey::of(&named_only);
        assert!(survey.names_actual_text());
        assert!(survey.inline_actual_texts().is_empty());

        let neither = page_with("/Span << /Lang (en) >> BDC EMC", "", &[]);
        let survey = Survey::of(&neither);
        assert!(!survey.names_actual_text());
        assert!(survey.inline_actual_texts().is_empty());
    }

    /// ISO 19005-4 section 6.3.3 sends an appearance dictionary's graphics to clause 6.2, and
    /// this is what carries that delegation: §12.5.5 makes an appearance stream a form
    /// `XObject`, and the walk follows every `/AP` entry of every page annotation into one.
    #[test]
    fn a_colour_selected_only_inside_an_annotation_appearance_is_recorded() {
        let marks = "1 0 0 rg 0 0 5 5 re f";
        let appearance = format!(
            "<< /Type /XObject /Subtype /Form /BBox [0 0 5 5] /Resources << >> /Length {} >>\n\
             stream\n{marks}\nendstream",
            marks.len()
        );
        let annotation =
            "<< /Type /Annot /Subtype /Square /Rect [0 0 5 5] /F 4 /AP << /N 6 0 R >> >>";
        let document = page_with("", "/Annots [5 0 R]", &[annotation, &appearance]);
        let survey = Survey::of(&document);
        let families: Vec<DeviceFamily> = survey
            .device_colours()
            .iter()
            .map(|colour| colour.family)
            .collect();
        assert_eq!(
            families,
            vec![DeviceFamily::Rgb],
            "the page's own content selects nothing, so this is the appearance stream's"
        );
    }

    /// §14.6.1's sequences nest, and what encloses a shown string is what may replace it.
    #[test]
    fn a_shown_string_carries_the_marked_content_that_encloses_it() {
        let font = "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>";
        let resources = "/Resources << /Font << /F1 5 0 R >> \
                         /Properties << /P0 << /MCID 7 >> /P1 << /ActualText (ab) >> >> >>";
        let shown = |content: &str| {
            let document = page_with(&format!("BT /F1 12 Tf {content} ET"), resources, &[font]);
            let survey = Survey::of(&document);
            survey
                .fonts()
                .flat_map(|used| used.shown.values().cloned())
                .collect::<Vec<Replacement>>()
        };

        assert_eq!(
            shown("(x) Tj"),
            vec![Replacement {
                bare: true,
                identifiers: Vec::new(),
                elided: false,
            }],
            "nothing encloses it, so nothing can replace it"
        );
        assert_eq!(
            shown("/Span << /MCID 3 >> BDC (x) Tj EMC"),
            vec![Replacement {
                bare: false,
                identifiers: vec![(0, 3)],
                elided: false,
            }],
            "§14.7.5.4's identifier is the route to the structure element, and it is kept"
        );
        assert_eq!(
            shown("/Span /P0 BDC (x) Tj EMC"),
            vec![Replacement {
                bare: false,
                identifiers: vec![(0, 7)],
                elided: false,
            }],
            "§14.6.2's property list may be named through the resources instead"
        );
        assert_eq!(
            shown("/Span << /ActualText (ab) >> BDC /Span << /MCID 3 >> BDC (x) Tj EMC EMC"),
            vec![Replacement::default()],
            "an enclosing sequence states the entry, so the showing is settled where it stands"
        );
        assert_eq!(
            shown("/Span /P1 BDC (x) Tj EMC"),
            vec![Replacement::default()],
            "and the same where that sequence named its property list"
        );
        assert_eq!(
            shown("/Span /Absent BDC (x) Tj EMC"),
            vec![Replacement::default()],
            "a property list this walk cannot read settles it too, because a rule that may not \
             invent a failure has to stay silent for both"
        );
        assert_eq!(
            shown("/Artifact BMC (x) Tj EMC"),
            vec![Replacement {
                bare: true,
                identifiers: Vec::new(),
                elided: false,
            }],
            "`BMC` opens a sequence with no property list at all, which can replace nothing"
        );
        assert_eq!(
            shown("/Span << /MCID 3 >> BDC EMC (x) Tj"),
            vec![Replacement {
                bare: true,
                identifiers: Vec::new(),
                elided: false,
            }],
            "the sequence closed before the string was shown"
        );
    }

    /// ISO 19005-2 section 6.7.4 binds a `/Lang` in a property list as well as in an element.
    #[test]
    fn a_language_in_a_marked_content_property_list_is_recorded_once_per_page_and_value() {
        let inline = "/Span << /Lang (en-GB) >> BDC EMC ";
        let document = page_with(
            &format!("{inline}{inline}/Span /P0 BDC EMC /Span << /Lang /en >> DP"),
            "/Resources << /Properties << /P0 << /Lang (de) >> >> >>",
            &[],
        );
        let survey = Survey::of(&document);
        let stated: Vec<(usize, Result<Vec<u8>, &str>)> = survey
            .marked_languages()
            .iter()
            .map(|found| (found.page, found.value.clone()))
            .collect();
        assert_eq!(
            stated,
            vec![
                (0, Ok(b"en-GB".to_vec())),
                (0, Ok(b"de".to_vec())),
                (0, Err("name")),
            ],
            "the repeat says nothing new, a named property list is read like an inline one, and \
             a value that is no text string is kept as what it is"
        );
    }

    /// The depth is the stack's after the push, so one `q` is one level.
    #[test]
    fn the_deepest_q_nesting_is_the_stack_at_its_deepest() {
        let document = page_with("q q q Q Q q Q Q", "", &[]);
        assert_eq!(
            Survey::of(&document)
                .deepest_graphics_state_nesting()
                .map(|held| held.value),
            Some(3)
        );
        let flat = page_with("0 0 5 5 re f", "", &[]);
        assert_eq!(Survey::of(&flat).deepest_graphics_state_nesting(), None);
    }
}
