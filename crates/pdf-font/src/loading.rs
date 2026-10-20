//! A font dictionary, loaded into something that can draw.
//!
//! [`LoadedFont`] is what the rest of the program holds. It resolves ISO 32000-2 clause 9's
//! two routes — a simple font's one-byte codes (§9.6) and a composite font's `CMap` (§9.7) —
//! once, when the font is loaded, and then answers a character code with a glyph outline, an
//! advance and the text the code stands for.
//!
//! Each step it delegates has a module of its own: which reader understands the embedded
//! program is [`crate::program`]'s, what glyph name a code selects is [`crate::glyph_names`]'s,
//! how a name or a code reaches a glyph is [`crate::name_keyed`]'s and [`crate::truetype`]'s,
//! what stands in for a program the document did not embed is [`crate::substituted`]'s, and
//! what the document says a code is wide is [`crate::metrics`]'s.

use crate::cmap::{CMap, Code};
use crate::composite::{CidToGlyph, composite_cmap};
use crate::glyph_class::Decision;
use crate::glyph_names::GlyphNames;
use crate::metrics::{
    NO_STRETCH, Vertical, composite_widths, missing_width, narrow, vertical_extent,
};
use crate::name_keyed::NameKeyed;
use crate::program::{Program, parsed_type1, simple_units_per_em};
use crate::runs::Runs;
use crate::substituted::script_sample;
use crate::vertical::Downward;
use crate::{cff, encoding, metrics, predefined, substitute, tounicode, type1};
use pdf_render::{Path, PathCommand, Point};
use pdf_syntax::{Dictionary, Document, Object};
use skrifa::outline::{HintingInstance, OutlinePen};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, OnceLock};

mod advances;
mod codes;
mod drawing;
mod findings;
mod glyphs;
mod opening;
mod program;
mod substitution;

/// A character code's glyph, for each of the 256 codes a simple font can use.
pub(crate) type CodeTable = [Option<u16>; 256];

/// A set of the one-byte character codes a simple font can use, as a bitmask.
///
/// Thirty-two bytes rather than a `[bool; 256]` or a `BTreeSet`, because a [`LoadedFont`] holds
/// one whether or not anything ever asks and the page-one path loads every font a page uses
/// (`CLAUDE.md` principle 2). Nothing allocates and nothing is computed twice: the one place
/// that fills it is already walking all 256 codes.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CodeSet([u64; 4]);

impl CodeSet {
    /// Records a code. A value above 255 is not a simple font's code and is ignored.
    pub(crate) fn insert(&mut self, code: u32) {
        let (word, bit) = (code / 64, code % 64);
        if let Ok(word) = usize::try_from(word)
            && let Some(slot) = self.0.get_mut(word)
        {
            *slot |= 1_u64 << bit;
        }
    }

    /// Whether the code is in the set.
    pub(crate) fn contains(&self, code: u32) -> bool {
        let (word, bit) = (code / 64, code % 64);
        usize::try_from(word)
            .ok()
            .and_then(|word| self.0.get(word))
            .is_some_and(|slot| slot & (1_u64 << bit) != 0)
    }
}

/// The glyph index a font program answers with when it has no glyph for a code.
///
/// Glyph 0 is `.notdef` in every format Table 124 admits, and both of clause 9's selection
/// routes end there when they fail: §9.6.5.2 substitutes it where "an encoding maps to a
/// character name that does not exist in the Type 1 font program", and §9.7.6.3 substitutes
/// "the glyph for CID 0 (which shall be present)" where "no glyph exists for that CID". So a
/// code that reached it reached a statement of absence rather than a glyph, and the two
/// instruments that ask this question — `pdf_model::Interpretation::codes_without_a_glyph` and
/// `crate::substituted::substitute_face`'s comparison of two faces — both have to say so the
/// same way.
pub const NOTDEF_GLYPH: u16 = 0;

/// How many codes [`LoadedFont::addressable_codes`] will walk before declining a font.
///
/// **A bound rather than a rule, and it is measured rather than picked.** §9.7.5.2's Table 116
/// says of `Identity-H` that "\[i]t maps 2-byte character codes ranging from 0 to 65,535", which
/// is the most any `CMap` of one- and two-byte codes can state and the largest inverse this
/// program has reason to build; the registered `CMap`s this binary carries are sparse and state
/// far fewer, which `every_registered_cmap_is_inside_the_addressable_bound` checks over all of
/// them rather than asserting here. Twice the Identity figure therefore admits every one of them
/// with a whole one-byte set to spare.
///
/// What it excludes is a `CMap` whose ranges cover a three- or four-byte codespace densely.
/// Those are refused as a whole and reported (§12.7.4.3's `Owed::FontUnusable`), because
/// principle 3's rule is that a document's own numbers may not drive an unbounded loop and
/// because a table that stopped early would answer with the wrong reason.
const MAX_ADDRESSABLE_CODES: u64 = 1 << 17;

/// How a font maps character codes to glyphs.
#[derive(Debug)]
pub(crate) enum CodeMapping {
    /// One byte per code, mapped through a table resolved when the font was loaded.
    ///
    /// Both routes a simple font can take end here. A bare CFF has no `cmap` and reaches a
    /// glyph by name through its charset (§9.6.5.2); a `TrueType` or `OpenType` program has
    /// one and reaches a glyph through the algorithm of §9.6.5.4. Neither resolution can be
    /// done by the font program alone — both need the PDF `/Encoding` — so both happen once
    /// at load time rather than per glyph drawn.
    Named(Box<CodeTable>),
    /// A composite font: a `CMap` from codes to CIDs, and the `CIDFont`'s own route to glyphs.
    ///
    /// The two halves are §9.7.5's and §9.7.4.2's and they are independent — a `CMap` says
    /// nothing about glyph indices and a `CIDToGIDMap` says nothing about codes — so keeping
    /// them apart is what stops the Identity case from being the only one that works.
    Composite {
        /// Codes to CIDs (§9.7.6.2).
        cmap: Box<CMap>,
        /// CIDs to glyph indices (§9.7.4.2).
        glyphs: CidToGlyph,
    },
    /// A composite font with no usable program, resolved through what its codes *mean*.
    ///
    /// The only route to a substitute for a composite font: a CID indexes the glyphs of
    /// the font that defined it, so it says nothing about any other font. The `CMap` is
    /// still needed, to split the string into codes — §9.7.4.2 is explicit that a CID plays
    /// no part here: "In this case, CIDs shall not participate in glyph selection".
    ///
    /// What a code *means* is not held here, because the font already holds it:
    /// [`LoadedFont::to_unicode`] is §9.10.2's first method and [`LoadedFont::collection`] its
    /// third, and [`LoadedFont::substituted_character`] asks them in the clause's order for
    /// each code. This variant holds no copy of either, so a code the producer's table omits
    /// still reaches the collection's, exactly as it does for the readback beside it (ADR 1002).
    Substituted {
        /// Codes to CIDs, used for the code boundaries, for `/W`'s widths, and for §9.10.2's
        /// step (a) on the way to the collection's table.
        cmap: Box<CMap>,
        /// The vertical forms this substitute draws, for a `CMap` in writing mode 1.
        ///
        /// `None` for every horizontal font and for a vertical one whose collection Table 116
        /// publishes no vertical `CMap` for — see [`crate::vertical`], which holds the whole
        /// argument and both of its halves. **`Some` for a face that states no vertical form at
        /// all**, which is not the same thing: there the question arises and the answer is that
        /// the reader loses the shape, which is what `Form::Unsupplied` says and what
        /// `LoadedFont::unsupplied_vertical_form` counts (ADR 0764).
        downward: Option<Box<Downward>>,
    },
    /// A font whose program this crate refused, carrying only what the *document* states.
    ///
    /// ISO 32000-2 §9.2.4 puts a glyph's displacement in two places at once — "\[t]he width
    /// information for each glyph shall be stored both in the font dictionary and in the font
    /// program itself" — and its NOTE 2 says what the redundancy buys:
    ///
    /// > Storing this information in the font dictionary, although redundant, enables a PDF
    /// > processor to determine glyph positioning without having to look inside the font
    /// > program.
    ///
    /// So a refusal that reached the *program* has not reached the widths, and §9.4.4's update —
    /// "\[a]fter the glyph is painted, the text matrix shall be updated according to the glyph
    /// displacement and any spacing parameters that apply" — is still owed for every code the
    /// page shows. No glyph is reachable through this mapping and no code is named through it;
    /// what it answers is where the *next* glyph goes, which on a line that continues in another
    /// font is that font's position. See [`LoadedFont::metrics_only`] and ADR 1094.
    MetricsOnly {
        /// Codes to CIDs (§9.7.6.2), which is what splits the string into codes and what keys
        /// §9.7.4.3's `/W`.
        ///
        /// `None` for a simple font, whose codes are single bytes (§9.6.1) and index Table 109's
        /// `/Widths` directly.
        cmap: Option<Box<CMap>>,
    },
}

/// Everything a font dictionary says about where its glyphs go, read without a font program.
///
/// The four entries [`LoadedFont::metrics_only`] needs, gathered in one place so that the two
/// kinds of font — a simple one keyed by code, a composite one keyed by CID — answer through one
/// struct literal rather than two. Each field is read by the same function the ordinary load
/// reads it with, because a second reader for the same table is a second reading of clause 9.
struct StatedMetrics {
    /// How the string is split into codes, and how each code keys `widths`.
    mapping: CodeMapping,
    /// Table 109's `/Widths` by code, or §9.7.4.3's `/W` by CID, in thousandths of an em.
    widths: Runs<f32>,
    /// Table 120's `/MissingWidth`, or Table 115's `/DW`.
    default_width: f32,
    /// Table 120's `/Ascent` and `/Descent`, in ems.
    extent: (f32, f32),
    /// §9.7.4.3's `/W2` and `/DW2`, for a `CMap` whose writing mode is 1.
    vertical: Option<Vertical>,
}

impl StatedMetrics {
    /// Reads them, or answers `None` for a font whose codes cannot be delimited at all.
    fn read(document: &Document, dict: &Dictionary, name: &str) -> Option<Self> {
        let composite = document
            .get_key(dict, "Subtype")
            .as_name()
            .is_some_and(|value| value.as_bytes() == b"Type0");
        if !composite {
            // §9.6.1: "Glyphs in the font shall be selected by single-byte character codes",
            // and Table 109 indexes `/Widths` by exactly those codes.
            let descriptor_object = document.get_key(dict, "FontDescriptor");
            let descriptor = descriptor_object.as_dict();
            return Some(Self {
                mapping: CodeMapping::MetricsOnly { cmap: None },
                widths: Runs::from_map(&metrics::stated_widths(document, dict)),
                default_width: missing_width(document, descriptor),
                extent: vertical_extent(document, descriptor),
                // §9.2.4: a second set of metrics "is available only for composite fonts".
                vertical: None,
            });
        }

        // The `CMap` is the whole of what a composite font's outer dictionary states about
        // placement (§9.7.5), and without it a string cannot be split into codes.
        let cmap = composite_cmap(document, dict, name).ok()?;
        let vertical = cmap.wmode() == 1;
        // §9.7.6.1's Table 119: the one-element array naming the CIDFont whose `/W`, `/DW` and
        // descriptor these come from. A file that selects none still places its codes, by the
        // defaults those two entries carry.
        let descendant = document
            .get_key(dict, "DescendantFonts")
            .as_array()
            .and_then(<[Object]>::first)
            .map(|item| document.resolve(item))
            .and_then(|item| item.as_dict().cloned());
        let descriptor_object = descendant
            .as_ref()
            .map(|descendant| document.get_key(descendant, "FontDescriptor"));
        let descriptor = descriptor_object.as_ref().and_then(Object::as_dict);
        Some(Self {
            mapping: CodeMapping::MetricsOnly {
                cmap: Some(Box::new(cmap)),
            },
            widths: descendant
                .as_ref()
                .map(|descendant| composite_widths(document, descendant))
                .unwrap_or_default(),
            default_width: descendant
                .as_ref()
                .and_then(|descendant| document.get_key(descendant, "DW").as_number())
                .map_or(DEFAULT_CID_WIDTH, narrow),
            extent: vertical_extent(document, descriptor),
            vertical: match (&descendant, vertical) {
                (Some(descendant), true) => Some(Vertical::read(document, descendant)),
                _ => None,
            },
        })
    }
}

/// Table 115's default for `/DW`, "\[t]he default width for glyphs in the `CIDFont`".
const DEFAULT_CID_WIDTH: f32 = 1000.0;

/// Why ISO 32000-2 §9.10.2 could not say what a character code represents.
///
/// The clause states three methods a processor can use "in the priority given" and ends by naming
/// the outcome when all of them fail:
///
/// > If these methods fail to produce a Unicode value, there is no way to determine what the
/// > character code represents in which case a PDF processor may choose a character code of
/// > their choosing.
///
/// A variant here says **which method was the highest-priority one this font could have answered
/// with**, so that a population of unnamed codes can be read as the sum of its causes rather than
/// as one number — and the two kinds are what a reader of that population needs to tell apart:
/// [`Self::UnaddressableCid`] and [`Self::UnlistedName`] are the sentence above happening, and
/// [`Self::IncompleteToUnicode`] and [`Self::EmptyMapping`] are a statement the file made and left
/// short. See [`LoadedFont::naming_gap`].
/// **Exhaustive on purpose**, where the errors beside it are not: a caller of this counts a
/// population, and a variant added later must break every such tally rather than be folded into
/// a wildcard arm that would silently mis-attribute it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum NamingGap {
    /// A mapping answered for this code with no characters at all.
    ///
    /// §9.10.3's `beginbfchar` "define\[s] the mapping from character codes to Unicode character
    /// sequences expressed in UTF-16BE encoding", and a sequence of length zero is a mapping that
    /// says the code means nothing. Kept apart from [`Self::IncompleteToUnicode`] because the
    /// producer *did* state something about this code: a reader has an answer and it is empty.
    EmptyMapping,
    /// The font carries a `/ToUnicode` `CMap` with no entry for this code.
    ///
    /// §9.10.2's first method applied and the producer's own table did not answer. §9.10.3 makes
    /// that table a statement about the codes it holds and requires nothing about completeness,
    /// so this is a gap in the *file* rather than in the clause.
    IncompleteToUnicode,
    /// A simple font's glyph selection used a name that neither list §9.10.2 names holds.
    ///
    /// The clause's second method sends a reader to the Adobe Glyph List and the Adobe Glyph
    /// List for New Fonts, and a producer's private label — pdfTeX's `/aNNN` after the code — is
    /// in neither. Carries the name, because what a name *is* decides whether the gap is the
    /// clause's or ours.
    UnlistedName(String),
    /// A composite font using a registered character collection whose table has no character for
    /// the CID this code selects.
    ///
    /// §9.10.2's third method applied — the collection's `registry-ordering-UCS2` `CMap` was
    /// found and read — and it holds nothing for this CID.
    UnnamedCid,
    /// A composite font §9.10.2's third method excludes by name, with no `/ToUnicode` at all.
    ///
    /// The clause's third method is for a font using a predefined `CMap` "(except Identity -H and
    /// Identity -V )" or one of the registered collections; an `Identity` ordering is neither, and
    /// §9.7.4.2 states why nothing else can be asked — a CID indexes the glyphs of the font that
    /// defined it and says nothing about any character. This is the clause's "there is no way",
    /// and no reading of the standard closes it.
    UnaddressableCid,
    /// A simple font that selected its glyph by code, and whose program does not name it.
    ///
    /// §9.6.5.4's route for a symbolic `TrueType` uses no glyph name, so the clause's second
    /// method has nothing to look up; §9.10.2's closing permission is then all that is left, and
    /// the program's own `post` table and `cmap` (see `LoadedFont::text_from_program`) named
    /// nothing either — a `post` of version 3.0 holds no names, and a `(3, 0)` subtable inverts
    /// to codes rather than to characters.
    UnnamedGlyph,
}

/// Why a font could not be used.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum FontError {
    /// The font program is not embedded in the document.
    ///
    /// Substituting a system font would change metrics and therefore layout, so this is
    /// reported rather than guessed at.
    ///
    /// **This is the only way into substitution, and a refusal downstream relies on that.**
    /// `composite::collection_gap`'s first branch says of a `/Encoding /Identity-H` over a
    /// substituted descendant that the *file* broke §9.7.5.2's "shall not be used with a
    /// non-embedded font" — a claim about the document rather than about this reader — and what
    /// entitles it is that `program::embedded_program` answers this variant exactly when Table
    /// 120's three keys supplied no program. A second error kind routed to substitution would
    /// carry that sentence to a file that *did* embed one, so either it does not, or
    /// `collection_gap` reads the descriptor for itself.
    #[error("font /{name} has no embedded font program")]
    NotEmbedded {
        /// The resource name, for diagnosis.
        name: String,
    },
    /// The font program could not be parsed.
    #[error("font /{name} could not be parsed: {detail}")]
    Malformed {
        /// The resource name.
        name: String,
        /// What went wrong.
        detail: String,
    },
    /// The font **dictionary** states something its own table does not admit.
    ///
    /// Separate from [`FontError::Malformed`], and the separation is what the message buys: that
    /// one says the program "could not be parsed", which is a claim that bytes a `/FontFile`
    /// supplied were read and rejected. Here no program is reached at all — the entry that would
    /// lead to one names nothing, or is not the type its table gives it, or is a stream that will
    /// not decode — so a reader told the program failed goes looking for damage in a font file
    /// that is not the fault.
    ///
    /// Two tables reach this today. ISO 32000-2 §9.7.6.1's Table 119 says of `/DescendantFonts`:
    ///
    /// > A one-element array specifying the CIDFont dictionary that is the descendant of this
    /// > Type 0 font.
    ///
    /// and §9.7.6.2 makes that array what the `CMap`'s font number indexes to select a `CIDFont`,
    /// so an array selecting no dictionary leaves the mapping with nothing to reach. The other is
    /// §9.7.4.2's `/CIDToGIDMap`, which Table 115 types as a stream or a name: a value that is
    /// neither states no route from a CID to a glyph index.
    #[error("font /{name}: {detail}")]
    MalformedDictionary {
        /// The resource name.
        name: String,
        /// What the dictionary states, and which table does not admit it.
        detail: String,
    },
    /// The encoding is one this crate does not implement.
    #[error("font /{name} uses unsupported encoding {encoding}")]
    UnsupportedEncoding {
        /// The resource name.
        name: String,
        /// The encoding named in the font dictionary.
        encoding: String,
    },
    /// The font is a Type 3 font, whose glyphs are content streams rather than outlines.
    ///
    /// Not a program this crate could read and does not: §9.6.4 defines a Type 3 glyph as a
    /// content stream in `/CharProcs`, so drawing one means running the interpreter, which
    /// lives a layer above this crate. Substituting is not available either — the glyph
    /// names in such a font's `/Differences` name procedures, and mean nothing anywhere
    /// else.
    #[error("font /{name} is a Type 3 font, whose glyphs are /CharProcs content streams")]
    Type3 {
        /// The resource name.
        name: String,
    },
    /// The font's own program is unusable and nothing can stand in for it.
    ///
    /// Distinct from [`FontError::NotEmbedded`], which no longer reaches a caller: a
    /// missing program is now substituted. This is the case where substitution itself
    /// failed — either because the machine has no such face, or because the face it has
    /// draws none of the codes the document uses.
    ///
    /// The reason is spelled out by the caller rather than by a second variant, because the
    /// two failures are the same fact to everyone above: the text will not be drawn.
    #[error("font /{name} cannot be substituted: {reason}")]
    NoSubstitute {
        /// The resource name.
        name: String,
        /// Why substitution failed, in the caller's own words.
        reason: String,
    },
}

/// One glyph reached by character rather than by character code.
///
/// What [`LoadedFont::character_glyph`] answers with, and the two halves are returned together
/// deliberately: a caller drawing text of its own has to place the next character, so a route
/// that gave it an outline without an advance would leave it measuring in one space and drawing
/// in another.
#[derive(Debug, Clone)]
pub struct CharacterGlyph {
    /// The outline in em units, y upwards, or `None` where the glyph makes no mark.
    ///
    /// A blank glyph and an absent one are different statements — the face has a `space` and has
    /// no 日 — which is why this is `Some(CharacterGlyph)` with no outline rather than `None`.
    pub outline: Option<Arc<Path>>,
    /// The advance the font program states for it, in ems.
    pub advance: f32,
}

/// Reads [`LoadedFont::outlines`] past a lock a panicking thread poisoned.
///
/// The same reasoning `pdf_syntax::Document`'s five locks are read under and for the same
/// reason: this holds no invariant across fields. It is a memoisation of a pure function of
/// the glyph index, every write is one `insert`, and the worst a poisoned map can cost is a
/// glyph built twice. Propagating a `PoisonError` would turn a panic anywhere in the process
/// into a font that can no longer draw.
/// What both loads read out of a program's bytes before anything else: its parsed Type 1 form,
/// where that is what it is, and its scale.
fn parsed_program(
    program: Program,
    data: &[u8],
    name: &str,
) -> Result<(Option<type1::Program>, f32), FontError> {
    let type1 = parsed_type1(program, data, name)?;
    let units_per_em = simple_units_per_em(type1.as_ref(), data, program, name)?;
    Ok((type1, units_per_em))
}

/// A CID-keyed CFF whose Font DICTs cannot be read, given ones that can be before anything
/// reads the program — and what that cost, for [`LoadedFont::repair_shortfall`].
///
/// A program that does not open at all is left as it is, for `cid_to_glyph` to refuse with
/// the reader's own words; an `sfnt` or a Type 1 has no CFF Font DICTs. See
/// [`cff::FontDictRepair`]. The bytes are kept once: the repair's copy becomes `data` and the
/// record keeps its figures only.
fn with_readable_font_dicts(
    data: Arc<[u8]>,
    program: Program,
) -> (Arc<[u8]>, Option<cff::FontDictRepair>) {
    let repair = match program {
        Program::BareCff => cff::readable_font_dicts(&data).ok().flatten(),
        Program::Sfnt | Program::Type1 => None,
    };
    match repair {
        Some(repair) => (
            Arc::from(repair.data.as_slice()),
            Some(cff::FontDictRepair {
                data: Vec::new(),
                ..repair
            }),
        ),
        None => (data, None),
    }
}

fn outlines(
    lock: &Mutex<BTreeMap<Placed, Option<Arc<Path>>>>,
) -> std::sync::MutexGuard<'_, BTreeMap<Placed, Option<Arc<Path>>>> {
    lock.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// A glyph, and which of a font's faces holds it.
///
/// Every font in this crate has exactly one face but a substituted `CIDFont` whose descriptor
/// states ISO 32000-2 §9.8.3.3's `/FD`, where each glyph class the file names may have chosen a
/// different one. [`Face::Own`] is the font's own face and is what every other route answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Face {
    /// The face the font dictionary's own descriptor chose.
    Own,
    /// The face one of `/FD`'s classes chose, indexed as the file states them.
    Class(usize),
}

/// A glyph index together with the face it indexes.
///
/// The pair travels as one value because a glyph index means nothing without its face: two faces
/// number their glyphs independently, and an outline cache keyed by the index alone would hand
/// one face's contours back for the other's glyph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Placed {
    face: Face,
    glyph: u16,
}

/// ISO 32000-2 §9.8.3.3's per-class substitute.
///
/// > The font descriptor shall define a set of default attributes that apply to all glyphs in
/// > the CIDFont. The FD entry in the font descriptor shall contain exceptions to these
/// > defaults.
///
/// This tree reads a descriptor for one thing a page can see — which installed face stands in
/// for a font the document did not embed (ADR 0007) — so an exception to the defaults is a
/// second face, chosen from the main descriptor with this class's entries laid over it. The
/// glyphs [`crate::glyph_class::Decision`] shows to be in the class are drawn from it, and every
/// other glyph from the font's own face, which is what "for that class of glyphs" asks for.
struct ClassFace {
    /// The face this class's overriding descriptor chose.
    data: Arc<[u8]>,
    /// Its own units per em, which is a property of the face rather than of the font.
    units_per_em: f32,
    /// §9.7.5.1's vertical forms, read from *this* face: a second face states its own.
    downward: Option<Box<Downward>>,
}

/// The machine face a composite font with no embedded program is drawn from, with the request it
/// was found by.
///
/// **`installed_covering` rather than `find`, and the difference is §9.7.4.2's.** A substituted
/// composite font is reachable only by character, so its face has to answer a `cmap` lookup —
/// which an `sfnt`'s does and the compiled-in name-keyed CFF faces cannot. Handing those to this
/// path would refuse five corpus documents a machine font draws.
///
/// The request comes back as well as the answer because the refusal names the family it looked
/// for, and because [`LoadedFont::machine_offers_a_substitute`] has to run *this* search rather
/// than a second one written to look like it (ADR 1154).
fn composite_substitute(
    document: &Document,
    descendant: &Dictionary,
    descriptor: Option<&Dictionary>,
) -> (substitute::Request, Option<Arc<[u8]>>) {
    let request = substitute::Request::derive(document, descendant, descriptor);
    // Characters the collection's own script requires, so that a face is chosen by what it can
    // *draw* and not only by the family a descriptor implies; among the faces that draw as much,
    // Table 120's weight, slope and width choose (ADR 1441).
    let wanted = script_sample(document, descendant);
    let style = substitute::Style::derive(document, descendant, descriptor);
    let found = substitute::installed_covering_styled(request, style, wanted);
    (request, found)
}

/// The pairs Adobe published for the face a simple font's `/BaseFont` names, where it is one of
/// §9.6.2.2's fourteen spelled as the clause spells it (ADR 1916).
///
/// Asked only of a font whose program the document did not embed: an embedded program is the
/// document's face, kerned by what it states.
fn published_pairs(document: &Document, dict: &Dictionary) -> Option<crate::pairs::Pairs> {
    let base_font = document.get_key(dict, "BaseFont");
    let name = std::str::from_utf8(base_font.as_name()?.as_bytes()).ok()?;
    crate::pairs::Pairs::published(name)
}

/// ISO 32000-2 §9.8.3.3's `/FD`, resolved to a decision and a face per class.
///
/// Answers for a `CIDFont` whose program the document did not embed and for no other font, which
/// is the whole of where this entry can reach: it overrides "the font-wide attributes for that
/// class only", and the attribute this tree acts on — which installed face stands in — is a
/// question an embedded program has already answered. Empty, too, for the
/// descriptor that states no `/FD`, which is all but 129 of the 617 696 font descriptors on this
/// disk (`pdf-model/examples/glyph_class_census`).
///
/// A class's face is chosen exactly as the font's own is: [`substitute::Request`] derived from a
/// descriptor — here the main one with this class's entries laid over it
/// ([`substitute::overridden`]) — and then `installed_covering` against the characters that class
/// has to be able to draw. Where this machine offers no such face the class is left `None` and
/// its glyphs keep the font's own face, which is the picture the page had before the entry was
/// read; a refusal cannot lose a mark.
///
/// # What it costs
///
/// One `Request` and one catalogue search per class, for a font that states the entry at all.
/// The corpus's `/FD` dictionaries name exactly one class each, and the search is the same one
/// the font's own face already paid for and is memoised on the characters asked for
/// (`substitute::covering_path`).
fn class_faces(
    document: &Document,
    descendant: &Dictionary,
    substituted: bool,
    vertical: bool,
    name: &str,
) -> (Decision, Vec<Option<ClassFace>>) {
    // The descendant rather than its descriptor, because both halves of the question are there:
    // §9.8.3.3's `/FD` is in the descriptor and §9.7.3's character collection, which the clause
    // makes the class names depend on, is in the `CIDFont` dictionary above it.
    let descriptor = document.get_key(descendant, "FontDescriptor");
    let Some(descriptor) = descriptor.as_dict().filter(|_| substituted) else {
        return (Decision::default(), Vec::new());
    };
    let stated = substitute::glyph_classes(document, descriptor);
    if stated.is_empty() {
        return (Decision::default(), Vec::new());
    }
    let collection = crate::composite::collection_names(document, descendant);
    let names: Vec<String> = stated.iter().map(|(name, _)| name.clone()).collect();
    let decision = Decision::read(
        &names,
        collection
            .as_ref()
            .map(|(registry, ordering)| (registry.as_str(), ordering.as_str())),
    );

    // The request the font's own descriptor derives, which is what a class has to differ from to
    // be an exception at all (see below). Derived once: every class is compared with it.
    let font_wide = substitute::Request::derive(document, descendant, Some(descriptor));
    let faces = stated
        .iter()
        .map(|(class, over)| {
            // A class no character can be shown to be in never draws, so no face is looked for.
            if decision
                .refused()
                .iter()
                .any(|(refused, _)| refused == class)
            {
                return None;
            }
            let merged = substitute::overridden(document, descriptor, over);
            let request = substitute::Request::derive(document, descendant, Some(&merged));
            // §9.8.3.3: the `/FD` entry "shall contain exceptions to these defaults", and a
            // class descriptor that derives the request the main one already derived has stated
            // no exception this tree can act on. Looking a face up for it anyway would change
            // the class's glyphs for nothing the file said — the characters asked for below are
            // the class's rather than the collection's, so the search could land elsewhere — and
            // that is the case the corpus actually holds: all 129 of its class descriptors
            // restate `/Flags` and `/ItalicAngle` unchanged and none states a `/FontWeight`
            // (`pdf-model/examples/glyph_class_census`).
            if request == font_wide {
                return None;
            }
            let wanted: &[char] = if crate::glyph_class::holds_the_collections_script(class) {
                script_sample(document, descendant)
            } else {
                &[]
            };
            let style = substitute::Style::derive(document, descendant, Some(&merged));
            let data = substitute::installed_covering_styled(request, style, wanted)?;
            let (_, units_per_em) = parsed_program(Program::Sfnt, &data, name).ok()?;
            let downward = Downward::read(document, descendant, &data, vertical);
            Some(ClassFace {
                data,
                units_per_em,
                downward,
            })
        })
        .collect();
    (decision, faces)
}

/// A font ready to produce glyph outlines.
///
/// # Shareable between threads, and *outliving* one page
///
/// The four memos below are locks rather than `RefCell` and `OnceCell` so that a loaded font
/// outlives the interpretation that loaded it (ADR 0710): a `Rc<LoadedFont>` cannot cross a thread,
/// and `viewer_core`'s per-document state does cross one — ADR 0182 opens the document on a thread
/// of its own and moves the whole viewer back, so anything held beside the document has to be
/// `Send`. That is the only reason these are locks; nothing in this crate uses more than one
/// thread.
///
/// **It is not free and the price was measured rather than assumed.** `examples/callgrind_pages`
/// and `examples/callgrind_interpret`, one sitting, two arms from one tree: **+0.468%** on fifty
/// interpretations of page 101 of ISO 32000-2 and **+0.140%** over twenty distinct pages of it,
/// the display lists identical command for command. What it buys is `pdf_model::FontCache`
/// and −14.9% of interpreting those same twenty pages. ADR 0710.
pub struct LoadedFont {
    /// The embedded font program, which the reader borrows from on each use.
    data: Arc<[u8]>,
    program: Program,
    /// What was done to `data` before it was read, where its Font DICTs could not be: a
    /// CID-keyed CFF drawn against an empty Private DICT, with the glyphs that needed the
    /// real one counted. See [`cff::FontDictRepair`].
    font_dicts: Option<cff::FontDictRepair>,
    /// The parsed Type 1 program, when that is what was embedded.
    ///
    /// The one program kept in parsed form rather than re-read per glyph, because it is the
    /// one whose parse is expensive; see [`type1::Program`].
    type1: Option<type1::Program>,
    mapping: CodeMapping,
    /// ISO 32000-2 §9.8.3.3's glyph classes, decided once against Table 123 and the collection.
    ///
    /// Empty of assignments and of refusals for every font whose descriptor states no `/FD`,
    /// which is all but 129 font descriptors of the 617 696 on this disk.
    class_of: Decision,
    /// The face each stated class chose, in the file's own order.
    ///
    /// `None` at an index where the class was refused ([`Decision::refused`]) or where this
    /// machine offered no face for the request its descriptor derives — in both cases the glyph
    /// keeps the font's own face, which is what it had before `/FD` was read.
    classes: Vec<Option<ClassFace>>,
    /// Glyph advances by character code, in thousandths of an em.
    widths: Runs<f32>,
    /// Advance for a code with no entry.
    default_width: f32,
    /// Table 120's `/Ascent` and `/Descent`, in ems.
    extent: (f32, f32),
    /// §9.7.4.3's second set of metrics, for a composite font in writing mode 1.
    vertical: Option<Vertical>,
    units_per_em: f32,
    /// The horizontal scale a substituted face's outlines are drawn at.
    ///
    /// [`crate::metrics::NO_STRETCH`] for every font whose program the document embedded, and
    /// for a substitute the file states no widths to compare against. See
    /// `crate::metrics::substitute_stretch` for what it is derived from and why.
    stretch: f32,
    /// Whether the glyphs are a stand-in rather than the font the document named.
    substituted: bool,
    /// The pairs Adobe published for the face, where the document names one of §9.6.2.2's
    /// fourteen by the clause's own spelling and embeds no program for it (ADR 1916).
    published_pairs: Option<crate::pairs::Pairs>,
    /// What the producer said each code means, when the font says so.
    to_unicode: tounicode::ToUnicode,
    /// §9.10.2's third method, for a composite font whose descendant names a registered
    /// character collection: the collection's own CID table, keyed by CID.
    ///
    /// Held beside `to_unicode` rather than folded into it because the two are keyed
    /// differently — by code and by CID, and folding one into the other would mean enumerating
    /// every code the `CMap` defines, which for a UTF-32 codespace is not a finite thing to do
    /// at load time — and because the clause ranks them, `/ToUnicode` first. Both
    /// [`Self::text`] and [`Self::substituted_character`] read the pair in that order.
    collection: Option<tounicode::ToUnicode>,
    /// The character set this font's own encoding belongs to, for the two symbolic standard-14
    /// fonts.
    ///
    /// `Some` only where this crate resolved the font *as* §9.6.2.2's `Symbol` or `ZapfDingbats` —
    /// a document naming one and embedding nothing — because that is the font whose character set
    /// Annex D documents. An embedded program brings its own built-in encoding (§9.6.5.1), which
    /// the annex says nothing about, so it is left `None` rather than assumed to be this one.
    symbolic_set: Option<encoding::SymbolicEncoding>,
    /// The glyph name each code selects, for simple fonts.
    ///
    /// The fallback for extraction when there is no `/ToUnicode`: a glyph name identifies
    /// a character through the Adobe Glyph List, and it is what actually selected the
    /// glyph, so it describes what was drawn rather than what the producer claimed.
    glyph_names: Option<GlyphNames>,
    /// The codes whose glyph was reached only by a mapping ISO 32000-2 §9.6.5.4 leaves open.
    ///
    /// That subclause's last sentence — "if a character cannot be mapped in any of the ways
    /// described previously, a PDF processor may supply a mapping of its choosing" — is a
    /// licence, and [`crate::truetype`] takes it twice. Empty for every font that never reached
    /// it, and for every route other than a simple `TrueType`/`OpenType` program, because no
    /// other route has that sentence behind it.
    ///
    /// Recorded rather than recomputed because it is knowable only while the table is being
    /// built: what comes out is 256 glyph indices with no record of which rule supplied each.
    /// [`Self::glyph_needed_a_reader_chosen_mapping`] is what asks.
    reader_chosen: CodeSet,
    /// §9.6.5.2's substitute: the glyph this program itself calls `.notdef`.
    ///
    /// > If an encoding maps to a character name that does not exist in the Type 1 font program,
    /// > the .notdef glyph shall be substituted.
    ///
    /// The clause requires every Type 1 program to contain a glyph of that name, and leaves what
    /// showing it looks like to the font's designer — usually nothing, sometimes a box.
    ///
    /// Kept as the program's *own* answer rather than as glyph 0, because the sentence is about
    /// a glyph with a name and this crate's readers number glyphs themselves. `None` for a
    /// program that has none, which the NOTE under that sentence leaves implementation
    /// dependent and which this crate answers by drawing nothing — the picture it drew before.
    notdef: Option<u16>,
    /// Whether a simple font's descriptor sets both Table 121's Symbolic and Nonsymbolic flags.
    ///
    /// §9.8.2: "This flag and the Nonsymbolic flag shall not both be set or both be clear", and
    /// "\[a] PDF processor should always check the Symbolic flag". So this crate reads such a
    /// font as symbolic, which for a `TrueType` program makes §9.6.5.4 ignore its `/Encoding` —
    /// and the fact is kept so that a page drawn short by that reading can say the file asked for
    /// both (ADR 1411). Always false for a composite font, whose descriptor selects no encoding.
    symbolic_flags_conflict: bool,
    /// Cached outlines: a page reuses the same few dozen glyphs constantly, and
    /// re-extracting each one would dominate the render.
    outlines: Mutex<BTreeMap<Placed, Option<Arc<Path>>>>,
    /// The inverse of the code-to-character mapping, built on first use by [`Self::code_for`].
    ///
    /// Lazy rather than built at load time because nothing on a page needs it: only a
    /// constructed appearance (§12.7.4.3) writes a string this crate has to encode, and a
    /// document may load hundreds of fonts without containing one form field.
    ///
    /// `None` inside the cell is the answer for a font that cannot be addressed this way at all
    /// — see [`Self::addressable_codes`] — and is cached like any other, because the walk that
    /// establishes it is the expensive one.
    codes_by_character: OnceLock<Option<BTreeMap<char, Code>>>,
    /// Each code's character through the Adobe Glyph List, resolved once.
    ///
    /// §9.10.2's second method — a glyph name looked up in the AGL — runs for every character
    /// a page shows in a font with no `/ToUnicode`, and `read_fonts::ps::agl::name_to_char`
    /// searches a four-thousand-entry list before trying the specification's algorithmic
    /// forms. A font has at most 256 codes and a page shows thousands of characters, so the
    /// same searches were being repeated all day.
    ///
    /// **Measured, on `examples/callgrind_interpret`**: 2 013.8 M instructions before,
    /// 1 989.1 M after — 1.2% of the whole of interpretation for a cache of 256 entries.
    /// The AGL's own share went from 4.26% to 3.35%.
    ///
    /// Lazy rather than built at load, because a font whose `/ToUnicode` covers its codes
    /// never reaches the list at all, and 256 AGL searches is not a cost to pay on the page-one
    /// path for nothing (`CLAUDE.md` principle 2).
    ///
    /// **And the same sentence applies one level down, which is why there are two cells.** The
    /// laziness used to be the table's alone: the first code that reached this route resolved
    /// all 256, and a page shows a few dozen of them — so the cost the paragraph above declines
    /// to pay on the page-one path was paid in full by the first character extracted from the
    /// font. Each cell now declines a different one. The outer declines the array itself for a
    /// font that never extracts text; the inner declines the Adobe Glyph List search for a code
    /// the page does not show. Neither changes an answer: the function and its argument are
    /// what they were, so the table this converges to is the one it used to build in one go.
    /// Page 101 of ISO 32000-2 asked [`encoding::text_for`] 67 200 times over fifty
    /// interpretations and asks it **8 850** — the codes it shows — which is **5.10%** of the
    /// whole of interpretation (ADR 0694). The outer cell's own price is one check per glyph,
    /// **+0.22%** on that page, and it is paid because the page whose fonts all reach this
    /// route is the arm where it loses: the eager array would allocate and zero 8 KiB at load
    /// for a font whose `/ToUnicode` answers every code and that never arrives here at all.
    agl_by_code: OnceLock<Box<[OnceLock<Option<String>>; 256]>>,
    /// The TrueType interpreter, for the one family of fonts whose *shapes* depend on it.
    ///
    /// Most TrueType instructions grid-fit an outline the `glyf` table already states, and
    /// this crate deliberately draws unhinted — a resolution decision, argued at
    /// [`Self::build_outline`]. A small family of fonts (`DynaLab`'s DFKai-SB among them)
    /// stores stroke *skeletons* and computes each glyph's finished contours in its
    /// instruction programs, so skipping the interpreter there draws contours the font never
    /// meant as a picture. `skrifa` carries `FreeType`'s detection of exactly that family —
    /// [`OutlineGlyphCollection::require_interpreter`][tricky], the `FT_FACE_FLAG_TRICKY` list — and
    /// its documentation states the contract this cell follows: when it answers true, hint
    /// through [`skrifa::outline::Engine::Interpreter`] with [`Target::Mono`][mono].
    ///
    /// Built once because `require_interpreter` reads the name table and may checksum font
    /// programs, and because [`HintingInstance::new`] runs the font's `fpgm` and `prep`
    /// programs — for the fonts in this family, the bulk of the machinery. `None` for every
    /// other font, which therefore draws exactly as before. Witness: page one of a crawled
    /// class list (`doc/checks/fixed-documents.toml`, `7803013.pdf`) whose embedded
    /// DFKaiShu-SB subset drew thin, misassembled strokes 2.25 ink points lighter than
    /// either reference at 8×. ADR 0727.
    ///
    /// [tricky]: skrifa::outline::OutlineGlyphCollection::require_interpreter
    /// [mono]: skrifa::outline::Target::Mono
    hinting: OnceLock<Option<HintingInstance>>,
    /// §9.10.2's last resort: what the *program* calls each glyph it defines.
    ///
    /// Keyed by glyph index rather than by character code, which is what lets one table serve
    /// both routes into it: a simple font arrives by code through `/Encoding`, and a composite
    /// font by CID through its `CMap` and `/CIDToGIDMap`. Built once and only for a font that
    /// reaches this far — see [`LoadedFont::text_from_program`], which is the only reader and
    /// explains the choice the clause permits.
    program_by_glyph: OnceLock<BTreeMap<u16, char>>,
}

impl std::fmt::Debug for LoadedFont {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoadedFont")
            .field("bytes", &self.data.len())
            .field("program", &self.program)
            .field("mapping", &self.mapping)
            .field("units_per_em", &self.units_per_em)
            .field("stretch", &self.stretch)
            .field("substituted", &self.substituted)
            .finish_non_exhaustive()
    }
}

/// A parsed Type 1 program's own code-to-glyph tables, or the error that says why there are none.
///
/// Split out of [`LoadedFont::load_simple`] because the two failures it distinguishes are the
/// whole of it: a `/Subtype /Type1` font whose program never parsed is one the document did not
/// embed usably, and one whose charset and encoding could not be read is malformed. Both are
/// reported as themselves rather than as a font that maps nothing.
fn type1_code_to_glyph(
    program: Option<&type1::Program>,
    name: &str,
) -> Result<NameKeyed, FontError> {
    program
        .ok_or_else(|| FontError::NotEmbedded {
            name: name.to_owned(),
        })?
        .code_to_glyph()
        .map_err(|error| FontError::Malformed {
            name: name.to_owned(),
            detail: error.to_string(),
        })
}

impl LoadedFont {
    /// One of ISO 32000-2 §9.6.2.2's fourteen, for text this program generates rather than reads.
    ///
    /// A viewer draws text of its own — an outline panel's titles, a layer's `/Name`, an About
    /// box — and none of it comes from a content stream, so there is no font dictionary to load
    /// and no document to load it from. What there *is* is §9.6.2.2's fourteen names, which
    /// Table 109 lets a file use without carrying the font — and which Errata Collection 3 has
    /// turned from a `shall` on a processor into an informative NOTE (Issue #47 and #48, `/State`
    /// `Review` `Completed`; see [`crate::standard`] for what moved and ADR 0253 for why
    /// `doc/md/` cannot show it).
    ///
    /// That availability is a fact about the binary rather than about the machine
    /// ([`crate::standard`], ADR 0133), which is what makes this worth having: an interface drawn
    /// in one of the fourteen looks the same on a machine with no fonts installed at all.
    ///
    /// **The route is the ordinary one, deliberately.** A `/Type1` dictionary naming
    /// `base_font` is assembled here and handed to [`Self::load`] against
    /// [`Document::empty`], so the encoding is §9.6.5.2's, the widths are §9.6.2.2's own
    /// metrics and the face is [`crate::standard::face`]'s — the same three answers a document
    /// naming `/Helvetica` gets. A second path would be a second reading of clause 9.
    ///
    /// # Errors
    ///
    /// As [`Self::load`]. A `base_font` that is not one of the fourteen is not an error: it
    /// falls through to [`crate::substitute::find`] exactly as a document's unrecognised
    /// `/BaseFont` does, and what comes back is a substitute rather than a refusal.
    pub fn standard(base_font: &str) -> Result<Self, FontError> {
        let name = |value: &str| Object::Name(pdf_syntax::Name::new(value.as_bytes().to_vec()));
        let mut dict = Dictionary::new();
        for (key, value) in [
            ("Type", "Font"),
            ("Subtype", "Type1"),
            ("BaseFont", base_font),
        ] {
            dict.insert(pdf_syntax::Name::new(key.as_bytes().to_vec()), name(value));
        }
        Self::load(&Document::empty(), &dict, base_font)
    }

    /// Loads a font from a PDF font dictionary.
    ///
    /// # Errors
    ///
    /// See [`FontError`]. Every failure names the font, because a page may use dozens and
    /// "unsupported font" without a name is not actionable.
    pub fn load(document: &Document, dict: &Dictionary, name: &str) -> Result<Self, FontError> {
        let subtype = document
            .get_key(dict, "Subtype")
            .as_name()
            .map(|value| value.as_bytes().to_vec())
            .unwrap_or_default();

        // A Type 3 font has no font program at all: each glyph is a content stream in
        // `/CharProcs`, run by the interpreter (§9.6.4), so nothing in this crate can draw
        // one. It is refused here rather than falling into the substitution path below,
        // where it used to arrive silently — a Type 3 `/Differences` array names
        // *procedures*, and `french_diacritics.pdf` names them `/a192`, `/a199`, `/a224`,
        // which are also ZapfDingbats glyph names, so a substitute drew whatever those
        // reached and reported nothing.
        if subtype == b"Type3" {
            return Err(FontError::Type3 {
                name: name.to_owned(),
            });
        }

        // A Type0 font delegates almost everything to a descendant; the outer dictionary
        // carries only the encoding.
        if subtype == b"Type0" {
            Self::load_composite(document, dict, name)
        } else {
            Self::load_simple(document, dict, name)
        }
    }

    /// A font whose program was refused, carrying the metrics the **document** states.
    ///
    /// # What a refusal is about, and what it is not about
    ///
    /// [`Self::load`] refuses a font when nothing can draw its glyphs: §9.7.5.2 forbids the file
    /// outright ("\[t]he Identity-H and Identity-V `CMaps` shall not be used with a non-embedded
    /// font"), or Table 120's three keys supplied no program and no face stands in, or the bytes
    /// a `/FontFile` supplied will not parse. Every one of those is a statement about the *glyph
    /// program*, and ISO 32000-2 §9.2.4 keeps the glyph's displacement somewhere else as well:
    ///
    /// > The width information for each glyph shall be stored both in the font dictionary and in
    /// > the font program itself.
    ///
    /// > Storing this information in the font dictionary, although redundant, enables a PDF
    /// > processor to determine glyph positioning without having to look inside the font program.
    ///
    /// So the file still says how far each code moves the pen — Table 109's `/Widths` and Table
    /// 120's `/MissingWidth` for a simple font, and for a `CIDFont` the two entries §9.7.4.3
    /// names: "\[w]idths for a `CIDFont` are defined using the DW and W entries in the `CIDFont`
    /// dictionary". §9.4.4 still requires the update — "\[a]fter the glyph is painted, the text
    /// matrix shall be updated according to the glyph displacement and any spacing parameters
    /// that apply" — so a reader that drops the displacement with the program moves every glyph
    /// that follows on the same line, including the glyphs of a font it loaded perfectly well.
    /// ADR 1094.
    ///
    /// # What it carries, and what it deliberately does not
    ///
    /// The widths, the default width, Table 120's extent and — for a `CMap` in writing mode 1 —
    /// §9.7.4.3's second set of metrics. Nothing else: no glyph is reachable
    /// ([`Self::outline`] and [`Self::glyph_index`] answer `None` for every code) and no `CMap`
    /// of §9.10.2's is read, so [`Self::text`] names nothing. Both omissions are the same
    /// sentence: this font marks no part of the page, and a reader that named its codes would
    /// put selectable text under blank paper while the page's own report says the font was
    /// refused.
    ///
    /// # When there is not even that
    ///
    /// `None` where the document states nothing that could place a code: a composite font whose
    /// `/Encoding` names no `CMap` this crate reads cannot even say where one code ends and the
    /// next begins, so it has no displacement to offer.
    #[must_use]
    pub fn metrics_only(document: &Document, dict: &Dictionary, name: &str) -> Option<Self> {
        let stated = StatedMetrics::read(document, dict, name)?;
        Some(Self {
            data: Arc::from(Vec::new()),
            // No program was read, and every route that would consult this one is closed by
            // `CodeMapping::MetricsOnly` before it is reached.
            program: Program::Sfnt,
            font_dicts: None,
            type1: None,
            mapping: stated.mapping,
            class_of: Decision::default(),
            classes: Vec::new(),
            widths: stated.widths,
            default_width: stated.default_width,
            extent: stated.extent,
            vertical: stated.vertical,
            // A scale for a program there is none of; `units_per_em` divides no outline here.
            units_per_em: 1.0,
            stretch: NO_STRETCH,
            // §9.7.4.2's substitution is a *face* standing in for the glyphs, which is exactly
            // what did not happen: this font draws nothing rather than the wrong thing.
            substituted: false,
            published_pairs: None,
            to_unicode: tounicode::ToUnicode::default(),
            collection: None,
            symbolic_set: None,
            glyph_names: None,
            notdef: None,
            symbolic_flags_conflict: false,
            reader_chosen: CodeSet::default(),
            outlines: Mutex::new(BTreeMap::new()),
            codes_by_character: OnceLock::new(),
            agl_by_code: OnceLock::new(),
            hinting: OnceLock::new(),
            program_by_glyph: OnceLock::new(),
        })
    }
}

/// Whether a descriptor sets both of Table 121's Symbolic (bit 3) and Nonsymbolic (bit 6) flags,
/// which §9.8.2 says "shall not both be set"; see [`LoadedFont::states_both_symbolic_flags`].
fn both_symbol_flags(document: &Document, descriptor: Option<&Dictionary>) -> bool {
    /// Bit 3 and bit 6, counting from one as the specification does.
    const BOTH: [u32; 2] = [1 << 2, 1 << 5];
    descriptor.is_some_and(|descriptor| {
        BOTH.iter()
            .all(|mask| metrics::flag(document, descriptor, *mask))
    })
}

/// Reads a font dictionary's `/ToUnicode` `CMap` and whatever it builds on (§9.10.3).
/// A Type 0 font's `/Encoding` where it is one of §9.7.5.2's predefined names, as text.
///
/// Only [`collection_gap`] asks, and only for the one of its four facts that is stated of the
/// `CMap` rather than of the descendant: §9.7.5.2's "shall not be used with a non-embedded font"
/// is about `Identity-H` and `Identity-V` by name. A stream `/Encoding` — §9.7.5.3's embedded
/// `CMap` — has no name and answers `None`, which is correct: the prohibition names two `CMap`s.
///
/// [`collection_gap`]: crate::composite::collection_gap
fn encoding_name(document: &Document, dict: &Dictionary) -> Option<String> {
    let name = document.get_key(dict, "Encoding");
    let name = name.as_name()?;
    std::str::from_utf8(name.as_bytes()).ok().map(str::to_owned)
}

fn to_unicode(document: &Document, dict: &Dictionary) -> tounicode::ToUnicode {
    read_to_unicode(document, &document.get_key(dict, "ToUnicode"), 0)
}

/// Bounds the chain of `CMap`s a `/ToUnicode` may build on, which a document could make cyclic.
///
/// The same bound `crate::composite::read_cmap` puts on the `/Encoding` form, for the same
/// reason.
const MAX_TO_UNICODE_DEPTH: u32 = 4;

/// One `/ToUnicode` stream, with the `CMap` it states only its differences from beneath it.
///
/// §9.10.3 names the one dictionary entry that means anything here, and it is `/UseCMap`, which
/// "may be used if the `CMap` is based on another `ToUnicode` `CMap`".
///
/// **The sentence that names it has been rewritten** (ADR 0426). Errata Collection 3's
/// Issue #462 (`/State` `Review` `Completed`) strikes everything in front of the entry's name —
/// the clause used to introduce it as the only pertinent entry of a `CMap` stream dictionary and
/// point at Table 118 for the rest — and inserts a table of the `/ToUnicode` stream's own entries
/// instead. `doc/md/` carries neither change, because the sponsored copy records EC3 as review
/// markup and the conversion dropped every annotation (ADR 0252). Nothing this function does
/// moves: `/UseCMap` is the entry under both readings, and §9.10.3's ledger row carries the rest.
///
/// A **name** is one of Adobe's published files, which this binary carries (see
/// [`predefined::unicode_cmap`]); a **stream** is another `/ToUnicode` `CMap`, read the same way.
///
/// **And the file's own `usecmap` operator is read where the dictionary is silent**, which is
/// what `issue5010.pdf` needs. §9.7.5.4 a) requires the two statements to agree —
///
/// > If the embedded CMap file contains a usecmap reference, the CMap indicated there shall
/// > also be identified by the UseCMap entry in the CMap stream dictionary.
///
/// — so following the operator can never contradict a conforming file, and it is the only
/// statement a file that omits the entry has made. `issue5010.pdf` is that file: a Korean
/// `Identity-H` font whose `/ToUnicode` states five mappings of its own and `/Adobe-Korea1-UCS2
/// usecmap` for the rest, with no `/UseCMap` in the stream dictionary. Every code its page shows
/// is in the *rest*, and §9.10.2's third method cannot help — the descendant's registry is
/// `Unidocs`, so there is no `Unidocs-Korea1-UCS2` to construct — so the page read back as
/// nothing at all.
fn read_to_unicode(document: &Document, object: &Object, depth: u32) -> tounicode::ToUnicode {
    if depth > MAX_TO_UNICODE_DEPTH {
        return tounicode::ToUnicode::default();
    }
    let Some(stream) = object.as_stream() else {
        return tounicode::ToUnicode::default();
    };
    let Some(bytes) = document.decoded_stream_data(stream) else {
        return tounicode::ToUnicode::default();
    };

    let base = match document.get_key(&stream.dict, "UseCMap") {
        Object::Name(named) => predefined::unicode_cmap(&String::from_utf8_lossy(named.as_bytes())),
        Object::Null => {
            predefined::used_by(&bytes).and_then(|name| predefined::unicode_cmap(&name))
        }
        referenced => Some(read_to_unicode(
            document,
            &referenced,
            depth.saturating_add(1),
        )),
    };
    tounicode::ToUnicode::parse_on(&bytes, base)
}

/// Collects glyph outlines into a [`Path`], scaling to em-normalised coordinates.
struct PathPen {
    path: Path,
    scale: f32,
    /// The horizontal scale of `crate::metrics::substitute_stretch`, applied to x alone.
    stretch: f32,
    /// The current point, needed to elevate quadratic curves to cubics.
    last: Option<Point>,
}

impl PathPen {
    fn at(&self, x: f32, y: f32) -> Point {
        Point::new(x * self.scale * self.stretch, y * self.scale)
    }
}

impl OutlinePen for PathPen {
    fn move_to(&mut self, x: f32, y: f32) {
        let point = self.at(x, y);
        self.last = Some(point);
        self.path.push(PathCommand::MoveTo(point));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let point = self.at(x, y);
        self.last = Some(point);
        self.path.push(PathCommand::LineTo(point));
    }

    /// Elevates a quadratic curve to a cubic.
    ///
    /// `TrueType` outlines are quadratic and PDF has no quadratic operator, so the whole
    /// pipeline handles exactly one curve type. The elevation is *exact*, not an
    /// approximation: a quadratic is the cubic whose control points sit two-thirds of the
    /// way from each endpoint toward the quadratic's control point.
    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        let control = self.at(cx, cy);
        let end = self.at(x, y);
        let start = self.last.unwrap_or(control);

        let first = Point::new(
            start.x + 2.0 / 3.0 * (control.x - start.x),
            start.y + 2.0 / 3.0 * (control.y - start.y),
        );
        let second = Point::new(
            end.x + 2.0 / 3.0 * (control.x - end.x),
            end.y + 2.0 / 3.0 * (control.y - end.y),
        );

        self.last = Some(end);
        self.path.push(PathCommand::CurveTo(first, second, end));
    }

    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        let end = self.at(x, y);
        self.last = Some(end);
        self.path.push(PathCommand::CurveTo(
            self.at(cx0, cy0),
            self.at(cx1, cy1),
            end,
        ));
    }

    fn close(&mut self) {
        self.path.push(PathCommand::Close);
    }
}

#[cfg(test)]
mod tests {
    use super::{CidToGlyph, Code, CodeMapping, LoadedFont, MAX_ADDRESSABLE_CODES, Program};
    use pdf_syntax::{Dictionary, Document};

    /// No registered `CMap` loses a mapping to one of `cmap.rs`'s bounds, in either direction.
    ///
    /// §9.7.5.2's Table 116 names the `CMap`s a reader resolves by name; Adobe publishes their
    /// files and this binary carries them, so a bound of ours below what one of them states is a
    /// reader that answers §9.7.6.2's lookup wrongly for codes the standard's own data defines.
    /// One was: `UniCNS-UCS2-H` states 16 418 `cidrange` entries of two-byte codes and
    /// `MAX_RANGES` was 16 384, so its last 34 — U+FF02 to U+FFE4, the fullwidth digits, Latin
    /// letters and punctuation of Adobe-CNS1 — were dropped in silence (ADR 0963).
    ///
    /// **The second half is what makes it a gate rather than a formality**: the fullwidth
    /// letters are checked against the CIDs Adobe's own file gives them, so a bound raised far
    /// enough to stop the flag firing while the parse still lost them would fail here. The CIDs
    /// are read off `data/cmaps/UniCNS-UCS2-H`, which is the file this binary compiles in, and
    /// are not a second implementation's output.
    #[test]
    fn no_registered_cmap_is_cut_by_these_bounds() {
        let mut counted = 0_u32;
        for name in crate::predefined::names() {
            let Some(cmap) = crate::predefined::cmap(name) else {
                panic!("/{name} is listed and does not parse");
            };
            assert_eq!(
                cmap.truncated(),
                None,
                "the registered /{name} lost a mapping to one of cmap.rs's bounds"
            );
            counted = counted.saturating_add(1);
        }
        assert!(counted > 200, "only {counted} registered CMaps were walked");

        let cns = crate::predefined::cmap("UniCNS-UCS2-H").expect("UniCNS-UCS2-H is carried");
        // `<ff10> <ff19> 333`, `<ff21> <ff3a> 365` and `<ff41> <ff5a> 391` are the last three
        // runs of data/cmaps/UniCNS-UCS2-H, and `<ff01> <ff01> 108` is the entry that sat one
        // inside the old bound — the control that says the CMap was being read at all.
        for (code, cid) in [
            ([0xff, 0x01], 108),
            ([0xff, 0x10], 333),
            ([0xff, 0x19], 342),
            ([0xff, 0x21], 365),
            ([0xff, 0x3a], 390),
            ([0xff, 0x41], 391),
            ([0xff, 0x5a], 416),
        ] {
            let read = cns.next_code(&code);
            assert_eq!(
                cns.cid(read),
                Some(cid),
                "UniCNS-UCS2-H maps {code:02x?} to CID {cid} and this reader disagrees"
            );
        }
    }

    /// No carried `CMap` read as a `/ToUnicode` loses a mapping to one of `tounicode.rs`'s
    /// bounds.
    ///
    /// **The sibling of [`no_registered_cmap_is_cut_by_these_bounds`], over the second parser that
    /// reads the same files (ADR 0971).** ADR 0963 measured Adobe's 240 published `CMap`s against
    /// `cmap.rs`'s four bounds and found one of them below what the data states; the same files
    /// also go through [`crate::tounicode`], which has three bounds of its own, and nothing walked
    /// them. §9.10.2's third method reads `registry-ordering-UCS2` for every composite font whose
    /// program is absent, and §9.10.3 lets a producer's `/ToUnicode` name any of the rest in
    /// `/UseCMap` — so the population is the whole set, not the five collection tables.
    ///
    /// Measured: the widest is `UCS2-ETen-B5` at 13 291 `bfrange` entries against a bound of
    /// 16 384, and `Adobe-Japan1-UCS2` at 17 387 individual mappings against 65 536. Nothing is
    /// cut, which is what this asserts; ADR 0971 has the finding, which is that being cut was
    /// silent.
    ///
    /// **The second half is what makes it a gate rather than a formality**, exactly as in its
    /// sibling: the *last* entry of each of those two files is checked against the value the
    /// file itself gives it, so a bound raised only far enough to stop the flag firing while
    /// the parse still lost the tail fails here.
    #[test]
    fn no_carried_unicode_cmap_is_cut_by_these_bounds() {
        let mut counted = 0_u32;
        let mut read = 0_u32;
        for name in crate::predefined::names() {
            counted = counted.saturating_add(1);
            // A name whose file states no `bfchar` or `bfrange` parses to an empty map and is
            // answered as `None` — every CID `CMap` in the set — so this is not a failure.
            let Some(table) = crate::predefined::unicode_cmap(name) else {
                continue;
            };
            read = read.saturating_add(1);
            assert_eq!(
                table.truncated(),
                None,
                "the carried /{name}, read as a /ToUnicode, lost a mapping to one of \
                 tounicode.rs's bounds"
            );
        }
        assert!(counted > 200, "only {counted} carried CMaps were walked");
        assert!(
            read > 40,
            "only {read} of them stated any bfchar or bfrange"
        );

        // `<FFE5> <FFE5> <A244>` is the last line of data/cmaps/UCS2-ETen-B5's only bfrange
        // section, and `<5a13> <32ff>` the last of data/cmaps/Adobe-Japan1-UCS2's last bfchar
        // section — the two entries a bound one short would drop first.
        let eten =
            crate::predefined::unicode_cmap("UCS2-ETen-B5").expect("UCS2-ETen-B5 is carried");
        assert_eq!(eten.char_for(0xffe5), char::from_u32(0xa244));
        let japan = crate::predefined::unicode_cmap("Adobe-Japan1-UCS2")
            .expect("Adobe-Japan1-UCS2 is carried");
        assert_eq!(japan.char_for(0x5a13), char::from_u32(0x32ff));
        // And the last bfrange of the same file, which is the entry after it.
        assert_eq!(japan.char_for(0x5a0e), char::from_u32(0x9f92));
    }

    /// Every registered `CMap` this binary carries can be inverted, which is the bound's evidence.
    ///
    /// [`MAX_ADDRESSABLE_CODES`] is a number in a source file, and what makes it the right number
    /// is that the population it has to admit fits inside it. That population is §9.7.5.2's
    /// registered `CMap`s — 239 files, compiled in (ADR 0140) — and
    /// this walks all of them rather than asserting anything about their contents. A round that
    /// adds a `CMap` to the binary and pushes one past the limit hears about it here instead of
    /// in a field that silently reports the wrong reason.
    ///
    /// `Identity-H` and `Identity-V` are not among the names and are the largest of all: Table
    /// 116 gives each 65 536 codes, which is asserted beside them.
    #[test]
    fn every_registered_cmap_is_inside_the_addressable_bound() {
        let mut counted = 0_u32;
        for name in crate::predefined::names() {
            let Some(cmap) = crate::predefined::cmap(name) else {
                panic!("/{name} is listed and does not parse");
            };
            let mut codes = 0_u64;
            assert!(
                cmap.each_addressable_code(MAX_ADDRESSABLE_CODES, |_| codes =
                    codes.saturating_add(1)),
                "/{name} states more codes than MAX_ADDRESSABLE_CODES"
            );
            counted = counted.saturating_add(1);
        }
        assert!(counted > 200, "only {counted} registered CMaps were walked");
        for identity in [
            crate::cmap::CMap::identity(),
            crate::cmap::CMap::identity_vertical(),
        ] {
            let mut codes = 0_u64;
            assert!(
                identity.each_addressable_code(MAX_ADDRESSABLE_CODES, |_| codes =
                    codes.saturating_add(1))
            );
            assert_eq!(codes, 1 << 16, "Table 116's own count");
        }
    }

    /// [`LoadedFont::standard`] answers with §9.6.2.2's metrics and drawable glyphs, no file.
    ///
    /// Three things at once, and each has failed somewhere else in this tree. The widths are
    /// the clause's own — Helvetica's `M` is 833 thousandths of an em and its space 278, which
    /// is the AFM's number and not something a substitute face happened to have — so it is the
    /// published metrics that answered rather than the substitute program's own advances.
    /// [`LoadedFont::advance`] states them in ems, which is why these are thousandths of one.
    /// Errata Collection 3 strikes §9.6.2.2's sentence on these fonts and their metrics — Issue
    /// #47 and #48, `/State` `Review` `Completed` — and [`crate::standard`] carries the reading
    /// that replaces it (ADR 0267).
    /// Every code a Latin label uses has an outline with segments in it, because a font that
    /// maps a code and draws nothing is the silent failure trap 1 is about. And the whole of it
    /// runs against [`Document::empty`], which is the point: there is no file here.
    #[test]
    fn the_fourteen_are_available_without_a_document() {
        let font = LoadedFont::standard("Helvetica").expect("one of §9.6.2.2's fourteen");
        for (character, width) in [('M', 0.833), (' ', 0.278), ('i', 0.222)] {
            let code = font
                .code_for(character)
                .unwrap_or_else(|| panic!("{character:?} has a code in StandardEncoding"));
            assert!(
                (font.advance(code) - width).abs() < 0.0005,
                "{character:?} advances {} em where §9.6.2.2's metrics say {width}",
                font.advance(code)
            );
        }
        for character in "Outline".chars() {
            let code = font.code_for(character).expect("a code");
            let outline = font
                .outline(code)
                .unwrap_or_else(|| panic!("{character:?} draws nothing"));
            assert!(
                !outline.commands().is_empty(),
                "{character:?} is an empty path"
            );
        }
        // A face the clause does not name is a substitute rather than a refusal, which is the
        // same answer a document's own unrecognised `/BaseFont` gets.
        LoadedFont::standard("Garamond").expect("a substitute, not an error");
    }

    /// The compiled-in face states characters no encoding of it can name.
    ///
    /// The whole of [`LoadedFont::character_glyph`]'s reason, pinned so that it cannot be
    /// re-optimised away: a simple font's encoding is 256 codes wide, so [`LoadedFont::code_for`]
    /// answers for the Latin set §9.6.5.2 names and for nothing else — while the face behind it
    /// is Liberation Sans, whose `cmap` states Greek and Cyrillic too. A caller with a character
    /// and no code (a panel of §12.3.3's titles) was being told the face has no Д.
    ///
    /// The negative half matters as much: the same face states no CJK, so a route that answered
    /// *something* for 日 would be drawing a wrong glyph rather than reporting an absence.
    #[test]
    fn a_character_no_encoding_can_name_may_still_be_in_the_face() {
        let font = LoadedFont::standard("Helvetica").expect("one of §9.6.2.2's fourteen");
        for character in ['Д', 'щ', 'Ω', 'ż', 'é'] {
            assert!(
                font.code_for(character).is_none(),
                "{character:?} has a code, so it is not this test's subject any more"
            );
            let glyph = font
                .character_glyph(character)
                .unwrap_or_else(|| panic!("the compiled-in face states no {character:?}"));
            assert!(
                glyph
                    .outline
                    .is_some_and(|outline| !outline.commands().is_empty()),
                "{character:?} is an empty path"
            );
            assert!(
                glyph.advance > 0.0,
                "{character:?} advances {} em",
                glyph.advance
            );
        }
        for character in ['日', 'ก', 'א'] {
            assert!(
                font.character_glyph(character).is_none(),
                "the compiled-in face claims a glyph for {character:?}"
            );
        }
        // Ten of the fourteen are bare CFF programs with no `cmap` at all, and this route has
        // nothing to ask them: they are keyed by glyph name, and their charsets hold the standard
        // Latin character set and nothing else (ADR 0270).
        let mono = LoadedFont::standard("Courier").expect("one of §9.6.2.2's fourteen");
        assert!(mono.character_glyph('Д').is_none());
    }

    /// Every PDF in `doc/`, which is the corpus these tests are written against.
    fn corpus() -> Vec<std::path::PathBuf> {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc");
        let mut files: Vec<_> = std::fs::read_dir(dir)
            .expect("the corpus directory is readable")
            .filter_map(|entry| entry.ok().map(|e| e.path()))
            .filter(|path| path.extension().is_some_and(|ext| ext == "pdf"))
            .collect();
        files.sort();
        files
    }

    /// The font dictionaries reachable from a document's first page.
    fn first_page_fonts(document: &Document) -> Vec<(String, Dictionary)> {
        fn first_page(document: &Document, node: &Dictionary) -> Option<Dictionary> {
            let kids = document.get_key(node, "Kids");
            let Some(list) = kids.as_array() else {
                return Some(node.clone());
            };
            let child = document.resolve(list.first()?);
            first_page(document, child.as_dict()?)
        }

        let Ok(catalog) = document.catalog() else {
            return Vec::new();
        };
        let tree = document.get_key(&catalog, "Pages");
        let Some(page) = tree.as_dict().and_then(|t| first_page(document, t)) else {
            return Vec::new();
        };
        let resources = document.get_key(&page, "Resources");
        let Some(resources) = resources.as_dict() else {
            return Vec::new();
        };
        let fonts = document.get_key(resources, "Font");
        let Some(fonts) = fonts.as_dict() else {
            return Vec::new();
        };
        fonts
            .iter()
            .filter_map(|(name, value)| {
                let dict = document.resolve(value).as_dict()?.clone();
                Some((String::from_utf8_lossy(name.as_bytes()).into_owned(), dict))
            })
            .collect()
    }

    /// Loads every first-page font in the corpus whose producer *embedded* a bare CFF program.
    ///
    /// **The `substituted` half of the filter is the population and not a detail**, because all
    /// three tests below reason about the program the producer shipped: that its charstrings'
    /// advances agree with the document's `/Widths`, that the corpus exercises both kinds of bare
    /// CFF, and that a code the encoding does not cover reaches no glyph in it. A *substitute* is
    /// a different designer's face reached through the characters the codes mean, and
    /// `metrics::substitute_stretch` says so from the other side — "[o]nly a substituted font is
    /// scaled … a disagreement between [an embedded program's outlines] and `/Widths` is the
    /// producer's own".
    ///
    /// Without it the population is a function of this machine's font collection. §9.6.2.2's
    /// fourteen are compiled in as bare CFF, so every simple font this machine cannot find a face
    /// for arrives here looking like an embedded one: on a machine with no fonts installed nine
    /// corpus fonts did, and `ICC-1_1998-09.pdf` `/F19` failed the widths test with a Foxit face's
    /// advances against a producer's array. Five join the population on *this* machine and passed
    /// only by luck of which face was picked. ADR 1154.
    fn corpus_bare_cff_fonts() -> Vec<(String, String, LoadedFont)> {
        let mut found = Vec::new();
        for path in corpus() {
            let bytes = std::fs::read(&path).expect("corpus file is readable");
            let file = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            let Ok(document) = Document::open(bytes) else {
                continue;
            };
            for (name, dict) in first_page_fonts(&document) {
                if let Ok(font) = LoadedFont::load(&document, &dict, &name)
                    && font.program == Program::BareCff
                    && !font.substituted
                {
                    found.push((file.clone(), name, font));
                }
            }
        }
        found
    }

    /// A code `code_for` returns must mean the character it was asked for.
    ///
    /// The property that matters for §12.7.4.3, stated the only way it can be: the inverse
    /// is not a bijection — two codes may draw the same character, and the first one wins —
    /// so the round trip has to start and end at the *character*. Run over every font on a
    /// first page of the specification corpus, so it is a statement about real encodings rather
    /// than about one constructed table.
    ///
    /// **And for a composite font the string is checked as well as the code**, because there the
    /// two are different claims: the code has a length, and a decoder splits the bytes by
    /// §9.7.6.2's codespace ranges rather than by what the writer intended. So each code's own
    /// bytes go back through [`LoadedFont::decode`] and must come out as that one code, over
    /// every corpus composite font (ADR 0337).
    #[test]
    fn a_code_for_a_character_means_that_character() {
        let mut checked = 0usize;
        let mut composite = 0usize;
        for path in corpus() {
            let bytes = std::fs::read(&path).expect("corpus file is readable");
            let Ok(document) = Document::open(bytes) else {
                continue;
            };
            for (name, dict) in first_page_fonts(&document) {
                let Ok(font) = LoadedFont::load(&document, &dict, &name) else {
                    continue;
                };
                let simple = matches!(font.mapping, CodeMapping::Named(_));
                let Some(table) = font.addressable_codes() else {
                    continue;
                };
                for (&character, &code) in table {
                    let mut back = String::new();
                    assert!(
                        font.text(code, &mut back),
                        "{name}: code {code:?} means nothing"
                    );
                    assert_eq!(
                        back.chars().collect::<Vec<_>>(),
                        vec![character],
                        "{name}: code {code:?} does not draw {character:?}"
                    );
                    let string: Vec<u8> = code
                        .value()
                        .to_be_bytes()
                        .into_iter()
                        .skip(4usize.saturating_sub(usize::from(code.length())))
                        .collect();
                    assert_eq!(
                        font.decode(&string),
                        vec![code],
                        "{name}: the string for {character:?} does not decode back to its code"
                    );
                    checked = checked.saturating_add(1);
                    if !simple {
                        composite = composite.saturating_add(1);
                    }
                }
            }
        }
        assert!(
            checked > 1000,
            "only {checked} codes checked; the corpus is not present"
        );
        assert!(
            composite > 100,
            "only {composite} composite codes checked; the corpus does not exercise the inverse \
             this test is half about"
        );
    }

    /// An embedded program's outlines are the producer's own, at every scale but its own.
    ///
    /// The other half of ADR 0358's rule, and the half no fixture can state: a substituted face
    /// is drawn to the widths the file states, and a font the document *carried* is drawn as the
    /// producer drew it, however its `/Widths` and its charstrings disagree. Over every font
    /// reachable from a first page in `doc/` — hundreds of real embedded programs across the
    /// fourteen specifications — the stretch is exactly one.
    #[test]
    fn an_embedded_program_is_never_reshaped() {
        let mut embedded = 0usize;
        for path in corpus() {
            let bytes = std::fs::read(&path).expect("corpus file is readable");
            let Ok(document) = Document::open(bytes) else {
                continue;
            };
            for (name, dict) in first_page_fonts(&document) {
                let Ok(font) = LoadedFont::load(&document, &dict, &name) else {
                    continue;
                };
                if font.substituted {
                    continue;
                }
                embedded = embedded.saturating_add(1);
                assert!(
                    (font.stretch() - 1.0).abs() < f32::EPSILON,
                    "{}: /{name} carries its own program and was drawn at {}",
                    path.display(),
                    font.stretch()
                );
            }
        }
        assert!(embedded > 20, "only {embedded} embedded fonts were checked");
    }

    /// The corpus must actually exercise both routes, or the tests below prove nothing.
    #[test]
    fn the_corpus_contains_both_kinds_of_bare_cff_font() {
        let fonts = corpus_bare_cff_fonts();
        let named = fonts
            .iter()
            .filter(|(_, _, f)| matches!(f.mapping, CodeMapping::Named(_)))
            .count();
        let keyed = fonts
            .iter()
            .filter(|(_, _, f)| {
                matches!(
                    &f.mapping,
                    CodeMapping::Composite {
                        glyphs: CidToGlyph::Charset(_),
                        ..
                    }
                )
            })
            .count();

        assert!(named > 0, "no name-keyed bare CFF font in the corpus");
        assert!(keyed > 0, "no CID-keyed bare CFF font in the corpus");
    }

    /// The document's `/Widths` and the font program's own advances must agree.
    ///
    /// This is the check that a character code reaches the *right* glyph, and it is worth
    /// more than any of the others because it does not consult the mapping to verify the
    /// mapping. The PDF states a width per character code; the CFF charstring states an
    /// advance per glyph. They are written by the same producer from the same font but
    /// travel through completely separate structures, so they agree only if the code
    /// reached the glyph the producer meant. An off-by-one charset, a misread encoding or
    /// a code silently used as a glyph index all break the agreement immediately.
    ///
    /// Widths that disagree are counted rather than tolerated one by one: a producer may
    /// legitimately override a glyph's advance in `/Widths`, so a handful of mismatches is
    /// normal and a systematic mismatch is the defect being looked for.
    #[test]
    fn the_pdf_widths_agree_with_the_font_programs_own_advances() {
        use skrifa::raw::ps::cff::CffFontRef;

        let mut checked = 0usize;
        let mut disagreed = 0usize;

        for (file, name, font) in corpus_bare_cff_fonts() {
            let cff = CffFontRef::new_cff(&font.data, 0, None).expect("the font already loaded");

            for (code, declared) in font.widths.iter() {
                // A subset font's `/Widths` is padded with zeros for every code the
                // document does not use, so a zero means "no opinion", not "zero wide".
                // Comparing those would flag correct mappings: code 173 resolves to
                // `hyphen` under WinAnsiEncoding note 5 and reaches the same real glyph
                // code 45 does, while its `/Widths` entry is a padding zero.
                if declared == 0.0 {
                    continue;
                }
                let Some(glyph) = font.glyph_for_selector(code) else {
                    continue;
                };
                let id = skrifa::GlyphId::from(glyph);
                let Some(index) = cff.subfont_index(id) else {
                    continue;
                };
                let Ok(subfont) = cff.subfont(index, &[]) else {
                    continue;
                };
                let mut sink = NoPen;
                let Ok(Some(advance)) = cff.draw(&subfont, id, &[], None, &mut sink) else {
                    continue;
                };

                // `/Widths` is in thousandths of an em; the charstring is in font units.
                let from_program = advance / font.units_per_em * 1000.0;
                checked += 1;
                if (from_program - declared).abs() > 1.0 {
                    disagreed += 1;
                    assert!(
                        disagreed < 8,
                        "{file} /{name}: code {code} is {declared} wide in /Widths but glyph \
                         {glyph} advances {from_program} — the code is reaching the wrong glyph"
                    );
                }
            }
        }

        assert!(checked > 200, "only {checked} widths were comparable");
        // A wrong mapping does not produce a few stragglers, it produces mostly-wrong.
        assert!(
            disagreed * 20 < checked,
            "{disagreed} of {checked} widths disagree with the font program"
        );
    }

    /// A code the encoding does not cover must reach no glyph, and draw at most `.notdef`.
    ///
    /// This is the regression test for the defect that motivated the work: a CFF font
    /// whose lookup falls through to treating the character code as a glyph index loads
    /// cleanly, reports nothing unsupported, and draws whatever glyph happens to sit at
    /// that index. Every subset font in the corpus has far fewer glyphs than codes, so a
    /// fall-through would show up here as a glyph where there should be none.
    ///
    /// **The assertion is on `glyph_index` rather than `outline`**, because §9.6.5.2's last
    /// sentence lets an uncovered code draw the program's own `.notdef`, so the property that
    /// catches the fall-through is the one about the *table* — which is also the answer all three
    /// of this project's missing-glyph instruments read. The second assertion is what keeps that
    /// from becoming a hole: every outline an uncovered code produces must be the **same** outline,
    /// because there is one `.notdef` per program and a fall-through would produce a different
    /// glyph per code.
    #[test]
    fn an_uncovered_code_has_no_glyph_rather_than_a_guessed_one() {
        let mut fonts_with_gaps = 0usize;

        for (file, name, font) in corpus_bare_cff_fonts() {
            let CodeMapping::Named(table) = &font.mapping else {
                continue;
            };
            let covered = table.iter().filter(|slot| slot.is_some()).count();
            if covered == 256 {
                continue;
            }
            fonts_with_gaps += 1;

            // Every outline an uncovered code produces, by identity — `outline` caches per
            // glyph, so one `.notdef` is one pointer however many codes reach it.
            let mut drawn_by_uncovered: Vec<*const pdf_render::Path> = Vec::new();
            for (code, slot) in table.iter().enumerate() {
                let Ok(byte) = u8::try_from(code) else {
                    continue;
                };
                if slot.is_some() {
                    continue;
                }
                let selector = Code::single_byte(byte);
                assert!(
                    font.glyph_index(selector).is_none(),
                    "{file} /{name}: code {code} has no glyph in the encoding but reaches one"
                );
                if let Some(drawn) = font.outline(selector) {
                    let identity = std::sync::Arc::as_ptr(&drawn);
                    if !drawn_by_uncovered.contains(&identity) {
                        drawn_by_uncovered.push(identity);
                    }
                }
            }
            assert!(
                drawn_by_uncovered.len() <= 1,
                "{file} /{name}: uncovered codes draw {} different glyphs, so this is a \
                 fall-through rather than one .notdef",
                drawn_by_uncovered.len()
            );
        }

        assert!(
            fonts_with_gaps > 0,
            "no font in the corpus has an uncovered code, so this proves nothing"
        );
    }

    /// A pen that discards everything, for when only a charstring's advance is wanted.
    struct NoPen;

    impl skrifa::outline::OutlinePen for NoPen {
        fn move_to(&mut self, _x: f32, _y: f32) {}
        fn line_to(&mut self, _x: f32, _y: f32) {}
        fn quad_to(&mut self, _a: f32, _b: f32, _c: f32, _d: f32) {}
        fn curve_to(&mut self, _a: f32, _b: f32, _c: f32, _d: f32, _e: f32, _f: f32) {}
        fn close(&mut self) {}
    }
}

/// ISO 32000-2 §9.6's simple fonts are one path, and the `/Subtype` name does not fork it.
///
/// §9.6.2.3 is the reason this is worth a test rather than a comment: a multiple master
/// instance carries `/Subtype /MMType1` and, when it is embedded, "shall be an ordinary
/// Type 1 font program" — a snapshot with the design coordinates already chosen. So the
/// name distinguishes nothing a reader must do, and neither does `/TrueType`, because
/// §9.6.3's font may equally be an `OpenType` file holding CFF outlines. What decides is
/// the program, which `embedded_program` reads by signature.
#[cfg(test)]
mod simple_font_subtype_tests {
    use super::{FontError, LoadedFont};
    use crate::fixture::font_with_program;

    /// Every simple `/Subtype` reaches the same loader and fails identically on one program.
    #[test]
    fn a_simple_fonts_subtype_selects_nothing() {
        let load = |subtype: &str| {
            let (document, dict) = font_with_program(subtype, b"not a font program at all");
            LoadedFont::load(&document, &dict, "F1").err()
        };

        let type1 = load("Type1");
        assert!(
            matches!(type1, Some(FontError::Malformed { .. })),
            "an unreadable program should be reported as malformed, not {type1:?}"
        );
        assert_eq!(load("MMType1"), type1);
        assert_eq!(load("TrueType"), type1);
    }

    /// The two subtypes that *are* different clauses still are.
    #[test]
    fn type0_and_type3_are_not_simple_fonts() {
        let (document, dict) = font_with_program("Type3", b"not a font program at all");
        assert!(matches!(
            LoadedFont::load(&document, &dict, "F1"),
            Err(FontError::Type3 { .. })
        ));

        let (document, dict) = font_with_program("Type0", b"not a font program at all");
        let composite = LoadedFont::load(&document, &dict, "F1");
        assert!(
            composite.is_err(),
            "a Type0 dictionary with no /Encoding and no descendant cannot load"
        );
        assert!(
            !matches!(composite, Err(FontError::Malformed { .. })),
            "and it fails on the composite path, not on the program: {composite:?}"
        );
    }

    /// Assembles a minimal sfnt whose one drawable glyph carries constructive instructions.
    ///
    /// The glyph's `glyf` data states a 300-unit square at (100, 100); its instruction program
    /// moves point 3 — the square's top-left corner — up by 100 units (`SVTCA[y]`, `PUSHB` the
    /// point, `PUSHW` 6400 sixty-fourths, `SHPIX`). A renderer that draws the stated outline and
    /// one that runs the program therefore disagree about one y coordinate by a tenth of an em —
    /// the hint-reliant construction of ADR 0727's witness, reduced to one point. `family` goes
    /// into the name table, which is what `skrifa`'s `require_interpreter` reads; `fpgm` is
    /// prepended verbatim when given, so a test can hand the interpreter a program that cannot run.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        clippy::arithmetic_side_effects,
        reason = "a fixture builder over a font of two glyphs: every quantity is a few \
                  hundred, and a wrong table is what the assertions exist to catch"
    )]
    fn instructed_sfnt(family: &str, fpgm: &[u8]) -> Vec<u8> {
        let be16 = |v: i32| (v as i16).to_be_bytes();

        let mut head = Vec::new();
        head.extend_from_slice(&0x0001_0000_u32.to_be_bytes()); // version
        head.extend_from_slice(&[0; 8]); // fontRevision, checkSumAdjustment
        head.extend_from_slice(&0x5F0F_3CF5_u32.to_be_bytes()); // magic
        head.extend_from_slice(&be16(0)); // flags
        head.extend_from_slice(&be16(1000)); // unitsPerEm
        head.extend_from_slice(&[0; 16]); // created, modified
        for v in [100, 100, 400, 500] {
            head.extend_from_slice(&be16(v)); // xMin, yMin, xMax, yMax
        }
        head.extend_from_slice(&be16(0)); // macStyle
        head.extend_from_slice(&be16(8)); // lowestRecPPEM
        head.extend_from_slice(&be16(2)); // fontDirectionHint
        head.extend_from_slice(&be16(1)); // indexToLocFormat: long
        head.extend_from_slice(&be16(0)); // glyphDataFormat

        let mut maxp = Vec::new();
        maxp.extend_from_slice(&0x0001_0000_u32.to_be_bytes());
        for v in [2, 8, 1, 0, 0, 2, 4, 8, 8, 0, 32, 32, 0, 0] {
            maxp.extend_from_slice(&be16(v));
        }

        let mut hhea = Vec::new();
        hhea.extend_from_slice(&0x0001_0000_u32.to_be_bytes());
        for v in [800, -200, 0, 500, 0, 0, 500, 1, 0, 0, 0, 0, 0, 0, 0, 2] {
            hhea.extend_from_slice(&be16(v));
        }

        let mut hmtx = Vec::new();
        for _ in 0..2 {
            hmtx.extend_from_slice(&be16(500));
            hmtx.extend_from_slice(&be16(0));
        }

        // One (1, 0) format 0 subtable mapping code 0x41 to glyph 1.
        let mut cmap = Vec::new();
        cmap.extend_from_slice(&be16(0)); // version
        cmap.extend_from_slice(&be16(1)); // one subtable
        cmap.extend_from_slice(&be16(1)); // platform: Macintosh
        cmap.extend_from_slice(&be16(0)); // encoding: Roman
        cmap.extend_from_slice(&12_u32.to_be_bytes()); // offset
        cmap.extend_from_slice(&be16(0)); // format 0
        cmap.extend_from_slice(&be16(262)); // length
        cmap.extend_from_slice(&be16(0)); // language
        let mut codes = [0_u8; 256];
        codes[0x41] = 1;
        cmap.extend_from_slice(&codes);

        let mut glyph = Vec::new();
        glyph.extend_from_slice(&be16(1)); // one contour
        for v in [100, 100, 400, 400] {
            glyph.extend_from_slice(&be16(v)); // bounds
        }
        glyph.extend_from_slice(&be16(3)); // endPtsOfContours
        // SVTCA[y]; PUSHB[0] 0 (the point); PUSHW[0] 6400 (100 units in F26Dot6); SHPIX.
        let instructions: &[u8] = &[0x00, 0xB0, 0x03, 0xB8, 0x19, 0x00, 0x38];
        glyph.extend_from_slice(&be16(instructions.len() as i32));
        glyph.extend_from_slice(instructions);
        glyph.extend_from_slice(&[0x01; 4]); // four on-curve points, long deltas
        for delta in [100, 300, 0, -300] {
            glyph.extend_from_slice(&be16(delta)); // x
        }
        for delta in [100, 0, 300, 0] {
            glyph.extend_from_slice(&be16(delta)); // y
        }

        let mut loca = Vec::new();
        for offset in [0_u32, 0, glyph.len() as u32] {
            loca.extend_from_slice(&offset.to_be_bytes()); // glyph 0 is empty
        }

        // Format 0, one record: the family name, Macintosh Roman, English.
        let name_bytes = family.as_bytes();
        let mut name = Vec::new();
        for v in [0, 1, 18, 1, 0, 0, 1, name_bytes.len() as i32, 0] {
            name.extend_from_slice(&be16(v));
        }
        name.extend_from_slice(name_bytes);

        let mut tables: Vec<([u8; 4], Vec<u8>)> = vec![
            (*b"cmap", cmap),
            (*b"glyf", glyph),
            (*b"head", head),
            (*b"hhea", hhea),
            (*b"hmtx", hmtx),
            (*b"loca", loca),
            (*b"maxp", maxp),
            (*b"name", name),
        ];
        if !fpgm.is_empty() {
            tables.insert(1, (*b"fpgm", fpgm.to_vec()));
        }

        let mut out = Vec::new();
        out.extend_from_slice(&0x0001_0000_u32.to_be_bytes());
        out.extend_from_slice(&(tables.len() as u16).to_be_bytes());
        out.extend_from_slice(&[0; 6]); // binary-search hints, which nothing reads
        let directory = 12 + 16 * tables.len();
        let mut body = Vec::new();
        for (tag, data) in &tables {
            out.extend_from_slice(tag);
            out.extend_from_slice(&0_u32.to_be_bytes());
            out.extend_from_slice(&((directory + body.len()) as u32).to_be_bytes());
            out.extend_from_slice(&(data.len() as u32).to_be_bytes());
            body.extend_from_slice(data);
            while body.len() % 4 != 0 {
                body.push(0);
            }
        }
        out.extend_from_slice(&body);
        out
    }

    /// The highest y any point of the fixture glyph's outline reaches, in ems.
    fn top_of_a(document: &pdf_syntax::Document, dict: &pdf_syntax::Dictionary) -> f32 {
        let font = LoadedFont::load(document, dict, "F1").expect("the fixture font loads");
        let outline = font
            .outline(crate::Code::single_byte(0x41))
            .expect("code 0x41 reaches the fixture's one glyph");
        let mut top = f32::MIN;
        let mut consider = |point: &pdf_render::Point| top = top.max(point.y);
        for command in outline.commands() {
            match command {
                pdf_render::PathCommand::MoveTo(p) | pdf_render::PathCommand::LineTo(p) => {
                    consider(p);
                }
                pdf_render::PathCommand::CurveTo(a, b, c) => {
                    consider(a);
                    consider(b);
                    consider(c);
                }
                pdf_render::PathCommand::Close => {}
            }
        }
        top
    }

    /// A hint-reliant face draws what its instructions construct, not what `glyf` states.
    ///
    /// ISO 32000-2 §9.6.3 requires TrueType font programs, and for the family `FreeType` calls
    /// tricky — DFKai-SB among them — the program's instructions *are* the artwork: ADR
    /// 0727's witness stores stroke skeletons and assembles every glyph in bytecode. The
    /// fixture reduces that to one point: its instructions move the square's top-left corner up
    /// a tenth of an em, so the interpreter's answer and the stated outline's differ by a
    /// quantity nothing else in the fixture produces. The same bytes under a family name on
    /// nobody's list draw the stated square, which pins that the interpreter runs *only* for
    /// the family — every other font draws exactly as before ADR 0727.
    #[test]
    fn a_hint_reliant_familys_instructions_construct_the_outline() {
        let (document, dict) = crate::fixture::symbolic_font_with_binary_program(
            "TrueType",
            &instructed_sfnt("DFKai-SB", &[]),
        );
        let moved = top_of_a(&document, &dict);
        assert!(
            (moved - 0.5).abs() < 1e-3,
            "the instructed corner reaches half an em: {moved}"
        );

        let (document, dict) = crate::fixture::symbolic_font_with_binary_program(
            "TrueType",
            &instructed_sfnt("Ordinary", &[]),
        );
        let stated = top_of_a(&document, &dict);
        assert!(
            (stated - 0.4).abs() < 1e-3,
            "an ordinary family draws the stated square: {stated}"
        );
    }

    /// A family font whose interpreter cannot start still draws — the stated outline.
    ///
    /// `HintingInstance::new` runs the font's `fpgm`; a program that pops an empty stack
    /// cannot. The fallback is the picture every glyph of the family drew before ADR 0727,
    /// rather than no glyph: a refusal here would take the whole page's text with it for a
    /// defect in one table.
    #[test]
    fn a_family_font_with_a_broken_program_falls_back_to_the_stated_outline() {
        let (document, dict) = crate::fixture::symbolic_font_with_binary_program(
            "TrueType",
            &instructed_sfnt("DFKai-SB", &[0x38]), // SHPIX with nothing on the stack
        );
        let top = top_of_a(&document, &dict);
        assert!(
            (top - 0.4).abs() < 1e-3,
            "the skeleton is drawn when the program cannot run: {top}"
        );
    }

    /// The program's own advance is a second number, and it is not the dictionary's.
    ///
    /// The whole reason [`LoadedFont::program_advance`] exists: ISO 32000-2 §9.6.2.1's Table 109
    /// requires a file's stated widths to be "consistent with the actual widths given in the
    /// font program", and a check of that needs both statements. The fixture states no
    /// `/Widths` at all, so Table 120's `/MissingWidth` default of 0 is the dictionary's answer
    /// while the program's `hmtx` says 500 of its 1000 units per em — half an em apart, which
    /// is what a caller comparing them has to be able to see.
    #[test]
    fn the_program_states_an_advance_of_its_own() {
        let (document, dict) = crate::fixture::symbolic_font_with_binary_program(
            "TrueType",
            &instructed_sfnt("Ordinary", &[]),
        );
        let font = LoadedFont::load(&document, &dict, "F1").expect("the fixture font loads");
        let code = crate::Code::single_byte(0x41);
        assert!(
            (font.advance(code) - 0.0).abs() < 1e-6,
            "the dictionary states none"
        );
        let program = font
            .program_advance(code)
            .expect("the hmtx record is there");
        assert!(
            (program - 0.5).abs() < 1e-6,
            "half an em, from hmtx: {program}"
        );
    }

    /// A code the program's own tables do not reach has no advance to compare against.
    ///
    /// The fixture's `cmap` is a format 0 table listing code 0x41 and sending every other code
    /// to glyph 0, which is where §9.6.5.2 puts a character the program does not have.
    /// Answering with that glyph's advance would be a statement about `.notdef` dressed up as
    /// one about this character, and the fixture makes the mistake invisible on purpose: its
    /// `hmtx` gives glyph 0 the same 500 units as the glyph that is really there.
    #[test]
    fn a_code_the_program_does_not_reach_has_no_program_advance() {
        let (document, dict) = crate::fixture::symbolic_font_with_binary_program(
            "TrueType",
            &instructed_sfnt("Ordinary", &[]),
        );
        let font = LoadedFont::load(&document, &dict, "F1").expect("the fixture font loads");
        assert_eq!(font.program_advance(crate::Code::single_byte(0x42)), None);
    }

    /// The `cmap` inventory is the pairs the table states, in its own order.
    ///
    /// ISO 32000-2 §9.6.5.4 names its subtables by platform and encoding ID, and so do the
    /// requirements written on top of it; the fixture carries exactly one, Macintosh Roman.
    /// A program with no `vmtx` states no vertical advance, which is a different answer from a
    /// disagreement and is what the third assertion pins.
    #[test]
    fn the_program_reports_the_cmap_subtables_it_carries() {
        let (document, dict) = crate::fixture::symbolic_font_with_binary_program(
            "TrueType",
            &instructed_sfnt("Ordinary", &[]),
        );
        let font = LoadedFont::load(&document, &dict, "F1").expect("the fixture font loads");
        assert_eq!(font.program_cmap_subtables(), Some(vec![(1, 0)]));
        assert_eq!(
            font.program_vertical_advance(crate::Code::single_byte(0x41)),
            None,
            "the fixture has no vmtx"
        );
    }
}

#[cfg(test)]
mod substituted_composite_tests {
    use super::{FontError, LoadedFont};
    use crate::fixture::document_of;

    /// A `/ToUnicode` stream stating one mapping, for a font whose `CMap` addresses many codes.
    ///
    /// It says that code `<3042>` — U+3042 あ under `UniJIS-UCS2-H`, and Adobe-Japan1's CID 843
    /// — means U+3044 い, which is *not* what the collection says. That disagreement is
    /// deliberate: it is what shows which table answered.
    const TO_UNICODE: &[u8] = b"/CIDInit /ProcSet findresource begin\n\
        12 dict begin\n\
        begincmap\n\
        /CMapName /Fixture def\n\
        1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n\
        1 beginbfchar\n<3042> <3044>\nendbfchar\n\
        endcmap\n\
        CMapName currentdict /CMap defineresource pop\n\
        end end\n";

    /// A non-embedded Adobe-Japan1 font under a Unicode `CMap`, with the stream above.
    fn fixture() -> Result<LoadedFont, FontError> {
        let font = b"1 0 obj\n<< /Type /Font /Subtype /Type0 /BaseFont /KozMinPr6N-Regular \
            /Encoding /UniJIS-UCS2-H /DescendantFonts [2 0 R] /ToUnicode 3 0 R >>\nendobj\n";
        let descendant = b"2 0 obj\n<< /Type /Font /Subtype /CIDFontType0 \
            /BaseFont /KozMinPr6N-Regular \
            /CIDSystemInfo << /Registry (Adobe) /Ordering (Japan1) /Supplement 6 >> \
            /FontDescriptor << /Type /FontDescriptor /FontName /KozMinPr6N-Regular /Flags 4 \
            /FontBBox [0 -120 1000 880] /ItalicAngle 0 /Ascent 880 /Descent -120 \
            /CapHeight 700 /StemV 80 >> /DW 1000 >>\nendobj\n";
        let mut stream =
            format!("3 0 obj\n<< /Length {} >>\nstream\n", TO_UNICODE.len()).into_bytes();
        stream.extend_from_slice(TO_UNICODE);
        stream.extend_from_slice(b"\nendstream\nendobj\n");
        let (document, dict) = document_of(&[font, descendant, &stream]);
        LoadedFont::load(&document, &dict, "F1")
    }

    /// ISO 32000-2 §9.10.2's methods are ranked per code, and glyph selection for a substituted
    /// composite font is §9.10.2 (§9.7.4.2: "CIDs shall not participate in glyph selection").
    ///
    /// Three codes, one string. `<3042>` is stated by the `/ToUnicode` and means what the
    /// producer said, い, whatever Adobe-Japan1 says CID 843 is — the first method answered and
    /// is final. `<3044>` is omitted by the `/ToUnicode`, so the first method "fail[ed] to
    /// produce a Unicode value" for it and the third is next: `UniJIS-UCS2-H` gives CID 845 and
    /// `Adobe-Japan1-UCS2` gives い. `<3046>` the same way, to う. So the first two codes draw one
    /// glyph and the third another.
    ///
    /// **Machine-independent by construction.** Which face stands in is this machine's business
    /// (§9.5 NOTE 5, ADR 0133), so the test states relations between the three codes' glyphs
    /// and no glyph index: any face `installed_covering` accepts covers あ, and a face covering
    /// あ covers い and う. Where the machine has no such face the load refuses and the test says
    /// so rather than passing.
    ///
    /// **Calibrated both ways** (ADR 1002): with `substituted_character` reading the
    /// `/ToUnicode` alone — the route before ADR 1002 — the second and third codes reach no
    /// glyph and the first assertion fails; with the collection alone, the first code draws あ
    /// and the equality fails.
    #[test]
    fn a_code_the_to_unicode_omits_is_selected_through_the_collection() {
        let font = match fixture() {
            Ok(font) => font,
            Err(FontError::NoSubstitute { reason, .. }) => {
                println!("skipped: {reason}");
                return;
            }
            Err(other) => {
                panic!("the fixture should load or be refused for want of a face: {other}")
            }
        };
        assert!(font.is_substituted());
        let codes = font.decode(&[0x30, 0x42, 0x30, 0x44, 0x30, 0x46]);
        assert_eq!(codes.len(), 3, "UniJIS-UCS2-H is two bytes a code");
        let stated = font.glyph_index(codes[0]);
        let omitted = font.glyph_index(codes[1]);
        let another = font.glyph_index(codes[2]);
        assert!(
            omitted.is_some(),
            "a code the /ToUnicode omits reaches a glyph through §9.10.2's third method"
        );
        assert_eq!(
            stated, omitted,
            "the producer's /ToUnicode outranks the collection where it states the code, so \
             <3042> draws the い the file says and not the あ the collection says"
        );
        assert!(another.is_some());
        assert_ne!(
            another, omitted,
            "う and い are two glyphs of any face that has them"
        );

        // And the readback names what was drawn, through the same two tables in the same order.
        let mut text = String::new();
        for code in &codes {
            assert!(font.text(*code, &mut text));
        }
        assert_eq!(text, "いいう");
        assert!(
            font.uncovered_character(codes[1]).is_none(),
            "a character the face draws is not an uncovered one"
        );
    }
}

/// ISO 32000-2 §9.8.3.3's `/FD`, where its override is consumed: the face a glyph is drawn from.
///
/// # Why a fixture rather than a corpus document
///
/// `pdf-model/examples/glyph_class_census` counts what 91 800 documents write. 129 font
/// descriptors of 614 068 state `/FD` at all; every one of the 129 names exactly one class,
/// `Proportional` 123 times and `Alphabetic` six; 127 belong to a `CIDFont` that embeds its
/// program, where §9.8.3.3's substitution hint has nothing to hint at.
///
/// **And all 129 restate the main descriptor's `/Flags` and `/ItalicAngle` unchanged, and not one
/// states a `/FontWeight`** — which are the three entries a face is chosen by. What they do
/// override is `/StemV` (128 of 129), `/StemH` (126), `/XHeight` (108), `/CapHeight` (109) and
/// the `/Ascent`-`/Descent` pair (108 each), none of which this tree reads as an input to
/// anything. So no document on this disk states an override that *could* choose a different
/// face, and a fixture is the only thing that can show the route working. That is trap 8 stated
/// rather than hidden: the entry is implemented from the clause, and the census is what says no
/// corpus page moves because of it.
#[cfg(test)]
mod glyph_class_tests {
    use crate::cmap::CMap;
    use crate::fixture::document_of;
    use crate::{Code, LoadedFont};

    /// The two-byte code `Identity-H` makes of a CID (Table 116).
    fn code(cid: u16) -> Code {
        CMap::identity().next_code(&cid.to_be_bytes())
    }

    /// A non-embedded `Adobe-Japan1` `CIDFont`, with `/FD` as `classes` states it.
    ///
    /// Nothing in the name is a family word, so [`crate::substitute::Request`] falls through to
    /// Table 121's flags — which is the entry under test, since `/FD`'s descriptor overrides
    /// them for its class alone.
    fn japanese(classes: &str) -> (pdf_syntax::Document, pdf_syntax::Dictionary) {
        let descriptor = format!(
            "3 0 obj\n<< /Type /FontDescriptor /FontName /KozMinPr6N-Regular /Flags 6 \
             /StemV 62 /Ascent 752 /Descent -271 {classes} >>\nendobj\n"
        );
        document_of(&[
            b"1 0 obj\n<< /Type /Font /Subtype /Type0 /BaseFont /KozMinPr6N-Regular \
              /Encoding /Identity-H /DescendantFonts [ 2 0 R ] >>\nendobj\n",
            b"2 0 obj\n<< /Type /Font /Subtype /CIDFontType0 /BaseFont /KozMinPr6N-Regular \
              /CIDSystemInfo << /Registry (Adobe) /Ordering (Japan1) /Supplement 6 >> \
              /FontDescriptor 3 0 R /DW 1000 >>\nendobj\n",
            descriptor.as_bytes(),
            b"4 0 obj\n<< /Type /FontDescriptor /FontName /KozMinPr6N-Proportional /Flags 1 \
              /StemV 83 >>\nendobj\n",
        ])
    }

    /// The first CID of `Adobe-Japan1` the compiled-in table calls `character`.
    ///
    /// Searched rather than written down: which CID is which is Adobe's registered data, carried
    /// in `crate::collection`, and a number typed here would be a second copy of it.
    fn cid_for(font: &LoadedFont, character: char) -> u16 {
        for cid in 1..=8000_u16 {
            let mut out = String::new();
            if font.text(code(cid), &mut out) && out.chars().eq(std::iter::once(character)) {
                return cid;
            }
        }
        panic!("Adobe-Japan1 has a CID for {character:?}");
    }

    /// A class's descriptor decides the face its own glyphs are drawn from, and no others'.
    ///
    /// The main descriptor states Table 121's Serif bit and `/FD`'s `/Proportional` states
    /// `FixedPitch`, so §9.8.3.3's "exceptions to these defaults" ask for a monospaced face for
    /// the proportional Latin glyphs and leave the kanji on the serif one. Both halves are
    /// asserted, because an override that moved *every* glyph would satisfy the first alone.
    ///
    /// **Calibrated (trap 13) by deleting the `class_face` arm of `substituted_glyph`**: the
    /// first assertion fails and the second still passes.
    ///
    /// The skip is read off `crate::substitute` rather than off the route under test, which is
    /// the same trap: a machine whose catalogue answers both requests with one face cannot show
    /// a difference, and a skip taken from "the two outlines are equal" would pass with the
    /// whole route deleted.
    #[test]
    fn a_glyph_class_descriptor_chooses_the_face_for_its_own_class_alone() {
        let (document, dict) = japanese("/FD << /Proportional 4 0 R >>");
        let Ok(overridden) = LoadedFont::load(&document, &dict, "F1") else {
            println!("skipped: this machine offers no face for a non-embedded Adobe-Japan1 font");
            return;
        };
        let (plain, plain_dict) = japanese("");
        let plain = LoadedFont::load(&plain, &plain_dict, "F1")
            .expect("the same font without /FD loads exactly as it did before");

        let latin = cid_for(&plain, 'A');
        let kanji = cid_for(&plain, '\u{4e00}');

        let serif = crate::substitute::Request {
            family: crate::substitute::Family::Serif,
            bold: false,
            italic: false,
            standard: false,
        };
        let monospace = crate::substitute::Request {
            family: crate::substitute::Family::Monospace,
            ..serif
        };
        let one_face = match (
            crate::substitute::installed_covering(serif, &['\u{3042}']),
            crate::substitute::installed_covering(monospace, &[]),
        ) {
            (Some(a), Some(b)) => a == b,
            _ => true,
        };
        if one_face {
            println!("skipped: this machine answers both requests with the same face");
            return;
        }

        assert_ne!(
            overridden.outline(code(latin)),
            plain.outline(code(latin)),
            "the class states FixedPitch where the main descriptor states Serif, so its own \
             glyphs are drawn from another face"
        );
        assert_eq!(
            overridden.outline(code(kanji)),
            plain.outline(code(kanji)),
            "a kanji is in none of the classes /FD names, so it keeps the main descriptor's face"
        );
    }

    /// A class no character can be shown to be in is refused by name, and nothing moves.
    ///
    /// `Dingbats` is Table 123's "Special symbols", which no Unicode value states, so the
    /// override is declined rather than applied to a guess — and the font draws what it drew
    /// before the entry was read.
    #[test]
    fn a_class_no_character_decides_leaves_every_glyph_on_the_main_face() {
        let (document, dict) = japanese("/FD << /Dingbats 4 0 R >>");
        let Ok(refused) = LoadedFont::load(&document, &dict, "F1") else {
            println!("skipped: this machine offers no face for a non-embedded Adobe-Japan1 font");
            return;
        };
        let (plain, plain_dict) = japanese("");
        let plain = LoadedFont::load(&plain, &plain_dict, "F1").expect("the same font loads");
        let latin = cid_for(&plain, 'A');
        assert_eq!(refused.outline(code(latin)), plain.outline(code(latin)));
        let names: Vec<&str> = refused
            .refused_glyph_classes()
            .iter()
            .map(|(name, _)| *name)
            .collect();
        assert_eq!(
            names,
            ["Dingbats"],
            "the class is declined by name rather than guessed at: {:?}",
            refused.refused_glyph_classes()
        );
    }
}
