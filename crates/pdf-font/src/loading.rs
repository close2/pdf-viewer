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

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::{Mutex, OnceLock};

use pdf_render::{Path, PathCommand, Point};
use pdf_syntax::{Dictionary, Document, Object};
use skrifa::outline::{
    DrawSettings, Engine, HintingInstance, HintingOptions, OutlineGlyphCollection, OutlinePen,
    Target,
};
use skrifa::prelude::{LocationRef, Size};
use skrifa::raw::TableProvider;
use skrifa::{FontRef, GlyphId, MetadataProvider};

use crate::cff::{self, CodeToGlyph};
use crate::cmap::{CMap, Code};
use crate::composite::{
    CidToGlyph, cid_to_glyph, collection_gap, collection_table, composite_cmap,
};
use crate::encoding;
use crate::glyph_names::GlyphNames;
use crate::metrics;
use crate::metrics::{
    NO_STRETCH, SimpleMetrics, Vertical, composite_widths, missing_width, narrow, simple_advances,
    vertical_extent,
};
use crate::name_keyed::{NameKeyed, simple_code_table};
use crate::predefined;
use crate::program::{Embedded, Program, embedded_program, parsed_type1, simple_units_per_em};
use crate::substitute;
use crate::substituted::{
    script_sample, substitute_code_table, substitute_encoding_names, substitute_face, symbolic_set,
    wound_counter_clockwise,
};
use crate::tounicode;
use crate::truetype;
use crate::truetype::{invert_charmap, truetype_code_table};
use crate::type1;
use crate::vertical::{Downward, Form, VerticalForms};

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
/// [`crate::substituted::substitute_face`]'s comparison of two faces — both have to say so the
/// same way.
pub const NOTDEF_GLYPH: u16 = 0;

/// How many codes [`LoadedFont::addressable_codes`] will walk before declining a font.
///
/// **A bound rather than a rule, and it is measured rather than picked.** §9.7.5.2's Table 116
/// says of `Identity-H` that "[i]t maps 2-byte character codes ranging from 0 to 65,535", which
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
    /// each code. This variant carried a copy of one of the two until session 981, chosen once
    /// at load — the `/ToUnicode` whenever it stated anything at all — so a code the producer's
    /// table omitted never reached the collection's, while the readback beside it did (ADR
    /// 1002).
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
}

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
    /// §9.10.3's `beginbfchar` "define[s] the mapping from character codes to Unicode character
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
    /// the program's own `post` table and `cmap` (see [`LoadedFont::text_from_program`]) named
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
    #[error("font /{name} has no embedded font program")]
    NotEmbedded {
        /// The resource name, for diagnosis.
        name: String,
    },
    /// The embedded program is in a format this crate does not read.
    #[error("font /{name} uses unsupported program type {kind}")]
    UnsupportedProgram {
        /// The resource name.
        name: String,
        /// Which kind of program it was.
        kind: &'static str,
    },
    /// The font program could not be parsed.
    #[error("font /{name} could not be parsed: {detail}")]
    Malformed {
        /// The resource name.
        name: String,
        /// What went wrong.
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
/// reads the program — and what that cost, for [`LoadedFont::program_shortfall`].
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
    lock: &Mutex<BTreeMap<u16, Option<Arc<Path>>>>,
) -> std::sync::MutexGuard<'_, BTreeMap<u16, Option<Arc<Path>>>> {
    lock.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// A font ready to produce glyph outlines.
///
/// # Shareable between threads, and *outliving* one page
///
/// The four memos below were `RefCell` and `OnceCell` until the seven-hundred-and-seventieth
/// session, which is what confined a loaded font to the interpretation that loaded it: a
/// `Rc<LoadedFont>` cannot cross a thread, and `viewer_core`'s per-document state does cross
/// one — ADR 0182 opens the document on a thread of its own and moves the whole viewer back, so
/// anything held beside the document has to be `Send`. That is the only reason these are locks;
/// nothing in this crate uses more than one thread.
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
    /// Glyph advances by character code, in thousandths of an em.
    widths: BTreeMap<u32, f32>,
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
    /// [`crate::metrics::substitute_stretch`] for what it is derived from and why.
    stretch: f32,
    /// Whether the glyphs are a stand-in rather than the font the document named.
    substituted: bool,
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
    /// Cached outlines: a page reuses the same few dozen glyphs constantly, and
    /// re-extracting each one would dominate the render.
    outlines: Mutex<BTreeMap<u16, Option<Arc<Path>>>>,
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
    /// [`OutlineGlyphCollection::require_interpreter`], the `FT_FACE_FLAG_TRICKY` list — and
    /// its documentation states the contract this cell follows: when it answers true, hint
    /// through [`Engine::Interpreter`] with [`Target::Mono`].
    ///
    /// Built once because `require_interpreter` reads the name table and may checksum font
    /// programs, and because [`HintingInstance::new`] runs the font's `fpgm` and `prep`
    /// programs — for the fonts in this family, the bulk of the machinery. `None` for every
    /// other font, which therefore draws exactly as before. Witness: page one of a crawled
    /// class list (`doc/checks/fixed-documents.toml`, `7803013.pdf`) whose embedded
    /// DFKaiShu-SB subset drew thin, misassembled strokes 2.25 ink points lighter than
    /// either reference at 8×. ADR 0727.
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
    /// Since the hundred-and-forty-eighth session that availability is a fact about the binary
    /// rather than about the machine ([`crate::standard`], ADR 0133), which is what makes this
    /// worth having: an interface drawn in one of the fourteen looks the same on a machine with
    /// no fonts installed at all.
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

    /// Loads a simple font: one byte per code.
    fn load_simple(document: &Document, dict: &Dictionary, name: &str) -> Result<Self, FontError> {
        let descriptor_object = document.get_key(dict, "FontDescriptor");
        // A descriptor is required of every font except the standard 14, which is exactly
        // the case that most often needs substituting, so its absence is not an error yet.
        let descriptor = descriptor_object.as_dict();
        let embedded = match descriptor {
            Some(descriptor) => embedded_program(document, descriptor, name),
            None => Err(FontError::NotEmbedded {
                name: name.to_owned(),
            }),
        };

        let (data, program, substituted) = match embedded {
            Ok(Embedded { data, program }) => (data, program, None),
            // Nothing usable is embedded. A substitute renders the text in the wrong
            // shapes; refusing renders it not at all, and the document's own `/Widths`
            // keep the layout right either way.
            Err(FontError::NotEmbedded { .. } | FontError::UnsupportedProgram { .. }) => {
                let request = substitute::Request::derive(document, dict, descriptor);
                // The encoding is read before the face is chosen, because *which* face is
                // usable is decided by the characters the encoding names: see
                // `substitute_face`. It is handed on to the code table below rather than read
                // twice.
                let names = substitute_encoding_names(document, dict, request, name)?;
                let (data, format) = substitute_face(document, dict, request, &names, name);
                (data, Program::from(format), Some((request, names)))
            }
            Err(other) => return Err(other),
        };
        // The request outlives the match below, which consumes the encoding beside it.
        let requested = substituted.as_ref().map(|(request, _)| *request);
        let (type1, units_per_em) = parsed_program(program, &data, name)?;

        // Kept for text extraction: a glyph name is what a code means when a font carries
        // no `/ToUnicode`, which is common in older documents.
        let names;
        let mut notdef = None; // §9.6.5.2's substitute; only a name-keyed program has one
        // Empty unless the route below is a simple sfnt one, which is the only route §9.6.5.4
        // governs.
        let mut reader_chosen = CodeSet::default();
        let mapping = match (program, substituted) {
            // A substitute shares no glyph order with the font the document meant, so its
            // glyphs are reached by what each code *means* rather than by index.
            (_, Some((request, encoded))) => {
                let (table, resolved) =
                    substitute_code_table(document, dict, request, encoded, &data, program, name)?;
                names = Some(resolved);
                CodeMapping::Named(Box::new(table))
            }
            (Program::Sfnt, None) => {
                let (table, resolved, chosen) =
                    truetype_code_table(document, dict, descriptor, &data, name)?;
                names = Some(resolved);
                reader_chosen = chosen;
                CodeMapping::Named(Box::new(table))
            }
            (Program::BareCff, None) => {
                let cff = CodeToGlyph::read(&data).map_err(|e| FontError::Malformed {
                    name: name.to_owned(),
                    detail: e.to_string(),
                })?;
                // A CID-keyed program has no glyph names for `/Encoding` to address, and
                // §9.7.4.2 puts one in a *composite* font, so a simple font naming one is
                // malformed rather than unsupported.
                let CodeToGlyph::Named(keyed) = cff else {
                    return Err(FontError::UnsupportedEncoding {
                        name: name.to_owned(),
                        encoding: "CID-keyed CFF in a simple font".to_owned(),
                    });
                };
                let (table, resolved) = simple_code_table(document, dict, &keyed, name)?;
                (names, notdef) = (Some(resolved), keyed.by_name.get(".notdef").copied());
                CodeMapping::Named(Box::new(table))
            }
            (Program::Type1, None) => {
                let keyed = type1_code_to_glyph(type1.as_ref(), name)?;
                let (table, resolved) = simple_code_table(document, dict, &keyed, name)?;
                (names, notdef) = (Some(resolved), keyed.by_name.get(".notdef").copied());
                CodeMapping::Named(Box::new(table))
            }
        };

        let metrics = SimpleMetrics {
            substituted: requested,
            names: names.as_ref(),
            data: &data,
            program,
            mapping: &mapping,
            units_per_em,
        };
        let (widths, stretch) = simple_advances(document, dict, metrics);
        let default_width = missing_width(document, descriptor);

        Ok(Self {
            data,
            program,
            font_dicts: None, // name-keyed, so there are no Font DICTs to replace
            type1,
            mapping,
            widths,
            default_width,
            extent: vertical_extent(document, descriptor),
            // §9.2.4: a second set of metrics "is available only for composite fonts".
            vertical: None,
            units_per_em,
            stretch,
            substituted: requested.is_some(),
            to_unicode: to_unicode(document, dict),
            collection: None,
            symbolic_set: requested.and_then(|request| symbolic_set(request.family)),
            glyph_names: names,
            notdef,
            reader_chosen,
            outlines: Mutex::new(BTreeMap::new()),
            codes_by_character: OnceLock::new(),
            agl_by_code: OnceLock::new(),
            hinting: OnceLock::new(),
            program_by_glyph: OnceLock::new(),
        })
    }

    /// Loads a composite (Type0) font.
    fn load_composite(
        document: &Document,
        dict: &Dictionary,
        name: &str,
    ) -> Result<Self, FontError> {
        let cmap = composite_cmap(document, dict, name)?;
        // §9.7.5.1: "A CMap shall specify the writing mode … for any CIDFont with which the
        // CMap is combined", and §9.2.4 makes the writing mode the choice between two sets of
        // metrics rather than anything about the glyphs.
        let vertical = cmap.wmode() == 1;

        let descendants = document.get_key(dict, "DescendantFonts");
        let descendant = descendants
            .as_array()
            .and_then(<[Object]>::first)
            .map(|item| document.resolve(item))
            .and_then(|item| item.as_dict().cloned())
            .ok_or_else(|| FontError::Malformed {
                name: name.to_owned(),
                detail: "no descendant font".to_owned(),
            })?;

        let descriptor_object = document.get_key(&descendant, "FontDescriptor");
        let descriptor = descriptor_object.as_dict();

        let embedded = match descriptor {
            Some(descriptor) => embedded_program(document, descriptor, name),
            None => Err(FontError::NotEmbedded {
                name: name.to_owned(),
            }),
        };

        let (data, program, substituted) = match embedded {
            Err(FontError::NotEmbedded { .. } | FontError::UnsupportedProgram { .. }) => {
                let request = substitute::Request::derive(document, &descendant, descriptor);
                // Characters the collection's own script requires, so that a face is chosen
                // by what it can *draw* and not only by the family a descriptor implies.
                let wanted = script_sample(document, &descendant);
                // **`installed` rather than `find`, and the difference is §9.7.4.2's.** A
                // substituted composite font is reachable only through `/ToUnicode`, so its face
                // has to answer *by character* — which an `sfnt`'s `cmap` does and the
                // compiled-in name-keyed CFF faces cannot. Handing them to this path would refuse
                // five corpus documents a machine font draws.
                let data = substitute::installed_covering(request, wanted).ok_or_else(|| {
                    FontError::NoSubstitute {
                        name: name.to_owned(),
                        reason: format!(
                            "no {:?} face this machine offers can be addressed by character, which \
                         is the only way §9.7.4.2 leaves to reach a substitute for a composite \
                         font",
                            request.family
                        ),
                    }
                })?;
                (data, Program::Sfnt, true)
            }
            Ok(Embedded { data, program }) => (data, program, false),
            Err(other) => return Err(other),
        };
        let (data, font_dicts) = with_readable_font_dicts(data, program);
        // Kept parsed for the same measured reason a simple font keeps it (`type1::Program`),
        // and read for its scale the same way: a Type 1 program states it in a `/FontMatrix`
        // rather than in an `sfnt` header, so `FontRef` cannot be asked.
        let (type1, units_per_em) = parsed_program(program, &data, name)?;

        // §9.10.2's first and third methods, read once each: the producer's `/ToUnicode`, keyed
        // by code, and the collection's `registry-ordering-UCS2` table, keyed by CID. Both serve
        // the readback of every composite font and, for a substituted one, glyph selection too.
        let to_unicode = to_unicode(document, dict);
        let collection = collection_table(document, &descendant);

        let mapping = if substituted {
            // A CID is meaningless outside the font that defined it — it is an index into
            // that font's glyphs, not a character — so a substitute can only be reached
            // through what the codes *mean*. `/ToUnicode` is the only thing that says so,
            // and a composite font without one cannot be substituted at all. §9.7.4.2 says
            // the same thing from the other side: with the program absent, "CIDs shall not
            // participate in glyph selection", and a `/CIDToGIDMap` "shall be ignored, since
            // it is not meaningful to refer to glyph indices in an external font program".
            // §9.10.2's first method, then its third. The third became reachable in the
            // hundred-and-fifty-sixth session, when this binary started carrying the
            // collections' own tables; before it, a CJK font without a `/ToUnicode` was
            // refused whatever its `/CIDSystemInfo` said.
            //
            // **And where the CMap is one of the two identity ones, §9.7.5.2 has already
            // forbidden the file rather than left the reader a choice**: "The Identity-H and
            // Identity-V CMaps shall not be used with a non-embedded font. Only standardized
            // character sets may be used." A document that states `/Identity-H` over a
            // descendant with no font program has broken that sentence, so the refusal below
            // is not a gap in this reader — it is the one honest answer to a combination the
            // standard says shall not exist. ADR 0433 measures what the other renderers do
            // with it instead, and they do four different things.
            //
            // **Both tables are kept, and the choice between them is made per code.** The
            // clause ranks its methods and a method that "fail[s] to produce a Unicode value"
            // for one code has failed for that code only, so a `/ToUnicode` that omits a code
            // the collection names has not settled the font's route — `substituted_character`
            // asks the second table where the first says nothing (ADR 1002). What is refused
            // here is the font with *neither*, which is the one with no question left to ask.
            if to_unicode.is_empty() && collection.is_none() {
                // **[`FontError::NoSubstitute`] rather than
                // [`FontError::UnsupportedEncoding`], and the encoding is why**: `Identity-H`
                // is read perfectly well here — [`composite_cmap`] built `cmap` out of it
                // above — so nothing about the *encoding* is unsupported. What failed is
                // reaching a substitute through it, which is the case that variant's own doc
                // comment describes and which the sibling refusal thirty lines up already
                // uses. [`collection_gap`] says which of its four facts this file is.
                return Err(FontError::NoSubstitute {
                    name: name.to_owned(),
                    reason: collection_gap(
                        document,
                        &descendant,
                        encoding_name(document, dict).as_deref(),
                    ),
                });
            }
            CodeMapping::Substituted {
                cmap: Box::new(cmap),
                // §9.7.5.1's NOTE: a vertical `CMap` names *different CIDs*, so a substituted
                // face has to be asked for a different glyph too.
                downward: Downward::read(document, &descendant, &data, vertical),
            }
        } else {
            CodeMapping::Composite {
                cmap: Box::new(cmap),
                glyphs: cid_to_glyph(document, &descendant, &data, program, name)?,
            }
        };

        let default_width = document
            .get_key(&descendant, "DW")
            .as_number()
            .map_or(1000.0, narrow);

        Ok(Self {
            data,
            program,
            font_dicts,
            // §9.9's Table 124 gives a CIDFont `/FontFile2` and `/FontFile3` and never
            // `/FontFile` — but a descriptor writing one is read rather than refused; see
            // the `mapping` above for the clause's own analogy that decides how.
            type1,
            mapping,
            substituted,
            to_unicode,
            collection,
            // §9.6.5.1 gives the two symbolic standard-14 fonts *simple* built-in encodings, and
            // a composite font has none of them.
            symbolic_set: None,
            glyph_names: None,
            // A composite font's substitute is §9.7.6.3's CID 0, applied in `glyph_for`.
            notdef: None,
            // §9.6.5.4 is a *simple* font's subclause: a composite font's codes reach glyphs
            // through §9.7.6.2 and §9.7.4.2, neither of which offers the processor a choice.
            reader_chosen: CodeSet::default(),
            widths: composite_widths(document, &descendant),
            default_width,
            extent: vertical_extent(document, descriptor),
            vertical: vertical.then(|| Vertical::read(document, &descendant)),
            units_per_em,
            // A composite font's substitute is addressed by *character* through `/ToUnicode`
            // (§9.7.4.2), so there is no code whose `/W` entry and whose face advance are two
            // statements about one glyph — which is the comparison `substitute_stretch` is.
            // ADR 0358 states the restriction and what would lift it.
            stretch: NO_STRETCH,
            outlines: Mutex::new(BTreeMap::new()),
            codes_by_character: OnceLock::new(),
            agl_by_code: OnceLock::new(),
            hinting: OnceLock::new(),
            program_by_glyph: OnceLock::new(),
        })
    }

    /// Table 120's `/Ascent` and `/Descent`, in ems, for a caller measuring a line's height.
    ///
    /// See [`crate::metrics::vertical_extent`], including what a font that states neither gets
    /// and why.
    #[must_use]
    pub fn extent(&self) -> (f32, f32) {
        self.extent
    }

    /// Whether these glyphs stand in for a font the document did not embed.
    ///
    /// The shapes are then not the ones the producer chose, though the metrics still are
    /// wherever the document stated them.
    #[must_use]
    pub fn is_substituted(&self) -> bool {
        self.substituted
    }

    /// Whether the face this font draws from states vertical forms of its own glyphs.
    ///
    /// A question about the **face** and about nothing else: it reads the program's `GSUB` for
    /// the OpenType `vert` and `vrt2` features, whatever the font's writing mode is and whether
    /// or not the glyphs are a substitute. See [`crate::vertical`] for what those features are
    /// and why this crate reads them at all.
    ///
    /// **Public for a reason worth stating, because it is the only one.** Which face
    /// [`crate::substitute`] finds is a property of the machine, so a test of the vertical route
    /// has to be able to say *this machine's face has no vertical forms* — and it may not say it
    /// by observing that the route changed no glyph, because that is also exactly what a broken
    /// route looks like. A skip condition read off the output of the thing under test turns a
    /// defect into a green run, which is `doc/traps/instruments-and-reports.md`'s trap 13. This
    /// answers from the bytes of the face instead, so deleting the route cannot make it `false`.
    ///
    /// Reads the table on every call rather than from a memo, deliberately: nothing on a drawing
    /// path asks it, and a cache would be an optimisation of a question the program does not put.
    #[must_use]
    pub fn face_states_vertical_forms(&self) -> bool {
        FontRef::new(&self.data).is_ok_and(|font| !VerticalForms::read(&font).is_empty())
    }

    /// The horizontal scale this font's outlines are drawn at, one being the face as it is.
    ///
    /// A substituted face's glyphs are as wide as *its* designer drew them, inside advances the
    /// document states for the face it meant, so a condensed font substituted by a normal one
    /// collides where the file says there is a gap. This is the number that closes that, derived
    /// from the file's own `/Widths` against the chosen face's own advances — see
    /// [`crate::metrics::substitute_stretch`], which holds the clauses and the argument.
    ///
    /// Always one for a font whose program the document embedded: those outlines are the
    /// producer's own and nothing here may reshape them.
    #[must_use]
    pub fn stretch(&self) -> f32 {
        self.stretch
    }

    /// Appends the text a character code represents, reporting whether any was found.
    ///
    /// Three sources, in order of authority, and the first two are §9.10.2's own first two
    /// methods. `/ToUnicode` is the producer's own statement of what a code means and is
    /// preferred. Failing that, the glyph name the encoding selects identifies a character
    /// through the Adobe Glyph List — which describes what was actually drawn, and so stays
    /// right even when a producer's `/ToUnicode` is not. The third is not a method of the
    /// clause but the choice it explicitly permits where its methods fail; see
    /// [`Self::text_from_program`].
    ///
    /// Takes the destination by reference because extraction calls this once per character
    /// on the page, and returning a `String` would allocate for every one.
    pub fn text(&self, code: Code, out: &mut String) -> bool {
        if self.to_unicode.append(code.value(), out) {
            return true;
        }
        // §9.10.2's third method, in the clause's own position: after `/ToUnicode` and before
        // the permission it grants where its methods fail. It applies to an *embedded*
        // composite font as much as to a substituted one — the collection says what a CID
        // means whether or not the program that defines the CID is present.
        if let Some(table) = self.collection.as_ref()
            && let Some(cid) = match &self.mapping {
                CodeMapping::Composite { cmap, .. } | CodeMapping::Substituted { cmap, .. } => {
                    cmap.cid(code)
                }
                CodeMapping::Named(_) => None,
            }
            && table.append(cid, out)
        {
            return true;
        }
        let Some(names) = self.glyph_names.as_ref() else {
            return self.text_from_program(code, out);
        };
        let cells = self
            .agl_by_code
            .get_or_init(|| Box::new(std::array::from_fn(|_| OnceLock::new())));
        let Some((slot, name)) = usize::try_from(code.value())
            .ok()
            .and_then(|index| cells.get(index).zip(names.get(index)))
        else {
            return self.text_from_program(code, out);
        };
        let resolved = slot.get_or_init(|| {
            Some(Cow::as_ref(name))
                .filter(|name| !name.is_empty())
                .and_then(|name| {
                    // The clause's own second method first, and Annex D's character set
                    // where that list does not hold the name — which for `ZapfDingbats` is
                    // every name it has. See `SymbolicEncoding::character_for`, and ADR 0318
                    // for why the annex is a route to a character and not a convention.
                    encoding::text_for(name).or_else(|| {
                        self.symbolic_set
                            .and_then(|set| set.character_for(name))
                            .map(String::from)
                    })
                })
        });
        if let Some(text) = resolved {
            out.push_str(text);
            return true;
        }
        self.text_from_program(code, out)
    }

    /// Which of §9.10.2's methods could have named a code and did not, or `None` where one did.
    ///
    /// [`Self::text`] answers *whether* the clause named a code; this answers **why not**, and
    /// it exists because the refusal is a population rather than a case: ADR 0311 counted the
    /// codes a corpus page shows that no method can name, and the count on its own cannot say
    /// whether a reader lost them to a question the standard leaves unanswerable or to a route
    /// this program does not walk. Those two have different consequences, which is
    /// `CLAUDE.md` principle 5's distinction between misreading a clause and a clause defining
    /// nothing.
    ///
    /// The answer is **the highest-priority method the clause states that this font could have
    /// answered with**, because the clause itself ranks them: "[a] PDF processor can use these
    /// methods, in the priority given". A font carrying a `/ToUnicode` that omits the code has
    /// failed at the first method whatever its glyph names say, and reporting the last thing
    /// tried would describe every gap the same way — every route ends at the same declined
    /// permission.
    ///
    /// Asked by [`Self::text`]'s own route rather than by a copy of it, so the census cannot come
    /// to describe an extraction the code no longer does. The readback it produces is thrown
    /// away, and its `String` allocates only where a method answered — which is the branch that
    /// returns `None` and counts nothing.
    #[must_use]
    pub fn naming_gap(&self, code: Code) -> Option<NamingGap> {
        let mut discarded = String::new();
        if self.text(code, &mut discarded) {
            // A method answered, and a caller reading text back has characters only if the
            // answer had any: §9.10.3's destination may be a sequence, and a producer may write
            // an empty one.
            return discarded.is_empty().then_some(NamingGap::EmptyMapping);
        }
        // The clause's first method, and a font that carries the table has taken it: the
        // producer stated what its codes mean and left this one out. Distinguished from a font
        // with no table at all because they are different facts about the file — one is a
        // producer's incomplete statement, the other is no statement.
        if !self.to_unicode.is_empty() {
            return Some(NamingGap::IncompleteToUnicode);
        }
        // The clause's second method: "[i]f the font is a simple font and the glyph selection
        // algorithm … uses a glyph name, that name can be looked up in the Adobe Glyph List and
        // Adobe Glyph List for New Fonts". A name was used and neither list holds it.
        if let Some(name) = self.selected_glyph_name(code) {
            return Some(NamingGap::UnlistedName(name.to_owned()));
        }
        match &self.mapping {
            // The clause's third method, whose own first sentence says which composite fonts it
            // is for — the predefined `CMap`s "(except Identity -H and Identity -V )" and the
            // registered collections. `self.collection` is exactly that test, resolved when the
            // font was loaded.
            CodeMapping::Composite { .. } | CodeMapping::Substituted { .. } => {
                Some(if self.collection.is_some() {
                    NamingGap::UnnamedCid
                } else {
                    NamingGap::UnaddressableCid
                })
            }
            // A simple font that used no name at all: a symbolic `TrueType` selecting by code
            // through a `cmap` subtable (§9.6.5.4), whose program then named nothing either.
            CodeMapping::Named(_) => Some(NamingGap::UnnamedGlyph),
        }
    }

    /// The glyph name a simple font's encoding selects for a code, where it selects one.
    ///
    /// §9.6.5's glyph selection for a simple font is by name, and this is the name that was
    /// used — the same table [`Self::text`] takes §9.10.2's second method from, so the two
    /// cannot disagree about whether a name existed.
    ///
    /// **Public because *which name was used* is a different question from *what the code
    /// means*, and only the first one some callers are asking.** [`Self::text`] and
    /// [`Self::naming_gap`] both answer the second: a code whose name neither Adobe list holds
    /// still gets an answer out of them, because §9.10.2 ends by permitting a processor to
    /// choose one where its three methods fail, and [`Self::text_from_program`] takes that
    /// permission. A caller asking whether the *name* is a listed one therefore cannot read the
    /// answer off either — the permission has already hidden it — and this is what it needs.
    ///
    /// `None` for a composite font, which selects by CID rather than by name (§9.7.4.2), and
    /// for a code the encoding in force leaves unnamed.
    #[must_use]
    pub fn selected_glyph_name(&self, code: Code) -> Option<&str> {
        self.glyph_names
            .as_ref()?
            .get(usize::try_from(code.value()).ok()?)
            .map(Cow::as_ref)
            .filter(|name| !name.is_empty())
    }

    /// §9.10.2's last resort: the name the *font program* gives the glyph that was drawn.
    ///
    /// The clause's three methods are tried first and in its order. Where all three fail it
    /// states an outcome and a permission in one sentence:
    ///
    /// > If these methods fail to produce a Unicode value, there is no way to determine what
    /// > the character code represents in which case a PDF processor may choose a character
    /// > code of their choosing.
    ///
    /// This is that choice, and it is a choice rather than a fourth method — the second method
    /// asks for "the glyph name the glyph selection algorithm uses", and a symbolic `TrueType`
    /// selects its glyph by *code* through a `cmap` subtable, so no name was used. What is
    /// available instead is the program's own `post` table, which states what the glyph it drew
    /// is called, and the Adobe Glyph List, which states what that name means. Neither is a
    /// guess: both are read from data the file itself carries, which is the same instrument
    /// §9.6.5.4's own last resort uses for the *forward* direction.
    ///
    /// `issue15910.pdf` is the case. Its `/F10` is a symbolic `TrueType` Arial subset with no
    /// `/Encoding` and no `/ToUnicode`, drawing `(Allgäu)` and `(Käferhofen 10)`; both methods
    /// above return nothing and the page read back as though those two lines were not there.
    ///
    /// It cannot invent text where a font does not name its glyphs: a `post` table of version
    /// 3.0 holds no names at all, and a name outside the Adobe Glyph List answers `None`. That
    /// is what keeps it from being the fallback-that-fills-the-page this project forbids —
    /// measured over the pdf.js corpus rather than assumed, in the sixty-fourth session.
    ///
    /// **Two statements the program makes, in that order.** The `post` table names the glyph,
    /// which the Adobe Glyph List turns into a character — the same step §9.10.2's own second
    /// method takes, from a name the file supplies rather than one the encoding chose. Failing
    /// that, the program's Unicode `cmap` subtable is inverted: an entry mapping U+00E4 to
    /// glyph 74 is the font saying that glyph 74 is `ä`, whichever direction it is read in.
    /// `issue15910.pdf` needs the second, because its `post` is version 2.0 with every name an
    /// empty string — a table that satisfies the format and states nothing.
    ///
    /// **A composite font reaches this too, and it did not until the four-hundred-and-twenty-third
    /// session.** This function used to refuse one outright, on the note that "a composite one
    /// selects by CID through a `CMap`, and §9.10.2's third method is the route the clause states
    /// for those" — which reads the clause's third method as though it applied to every composite
    /// font. It does not, and the clause says so in its own first line:
    ///
    /// > If the font is a composite font that uses one of the predefined CMaps listed in
    /// > "Table 116 -Predefined CJK CMap names" (except Identity -H and Identity -V ) or whose
    /// > descendant CIDFont uses the Adobe-GB1, Adobe-CNS1, Adobe-Japan1, Adobe-Korea1
    /// > (deprecated in PDF 2.0 (2020)) or Adobe-KR (added in PDF 2.0 (2020)) character
    /// > collection
    ///
    /// An `Identity-H` font whose descendant is `Adobe-Identity` is excluded by name from the
    /// third method and cannot use the second, so a `/ToUnicode` that answers nothing leaves
    /// *every* method failed — which is the precondition of the permission quoted above, and the
    /// refusal declined it. Three documents in `doc/corpora/pdfbox` are that shape and all three
    /// read back short or blank while reporting nothing: `PDFBOX-4322-Empty-ToUnicode-reduced.pdf`
    /// (a `/ToUnicode` that is a copy of the `Identity-H` CID `CMap`, so it holds no `bfchar` or
    /// `bfrange` at all and §9.10.3 requires those), `PDFBOX-5838-0024320-reduced.pdf` (a
    /// `/ToUnicode` covering 8 of its 15 codes, reading `H Reeach Pec` for
    /// `Honors Research Project`) and `sample_fonts_solidconvertor.pdf` (two fonts whose
    /// `/ToUnicode` is the *name* `/Identity-H`, two whole lines of the page read back as
    /// nothing).
    ///
    /// The route is the same data in the same order, one step longer: the `CMap` gives a CID,
    /// §9.7.4.2's `/CIDToGIDMap` gives the glyph, and the program then names it. Nothing here is
    /// a guess about a code — it is the program's statement about a glyph the file's own tables
    /// selected.
    fn text_from_program(&self, code: Code, out: &mut String) -> bool {
        let glyph = match &self.mapping {
            CodeMapping::Named(_) => self.glyph_for_selector(code.value()),
            // §9.7.6.3's notdef fallbacks are deliberately not taken here, which is why this
            // is not `glyph_for`: a code that reached CID 0 drew a substitute, and naming
            // what the substitute is called would put a character on a page that shows none.
            CodeMapping::Composite { cmap, .. } => {
                cmap.cid(code).and_then(|cid| self.glyph_for_selector(cid))
            }
            // A substitute is reached through what a code *means*, so there is no glyph of
            // the document's own to ask about.
            CodeMapping::Substituted { .. } => None,
        };
        match glyph.and_then(|glyph| self.program_characters().get(&glyph).copied()) {
            Some(character) => {
                out.push(character);
                true
            }
            None => Self::text_from_the_code(code, out),
        }
    }

    /// What the embedded program calls each glyph it defines, built once.
    ///
    /// [`LoadedFont::text_from_program`] states the two sources and their order; this is where
    /// they are read. The `post` table is applied second so that it overwrites the inverted
    /// `cmap`, which is that order.
    ///
    /// **Both are read at once, where the `cmap` used to be inverted only for a glyph the `post`
    /// table left unanswered.** The laziness was worth having while only a simple font arrived
    /// here and its `post` usually answered; a subset embedded for a composite font is normally
    /// `post` version 3.0, which holds no names at all, so the second source is needed for
    /// essentially every glyph and deferring it buys a branch rather than a table walk. The walk
    /// is one pass over the mappings the font states, once per font, and only for a font that
    /// got this far.
    fn program_characters(&self) -> &BTreeMap<u16, char> {
        self.program_by_glyph.get_or_init(|| {
            let Ok(font) = FontRef::new(&self.data) else {
                return BTreeMap::new();
            };
            let mut by_glyph = invert_charmap(&font);
            let Ok(post) = font.post() else {
                return by_glyph;
            };
            let glyphs = font.maxp().map_or(0, |maxp| maxp.num_glyphs());
            for glyph in 0..glyphs {
                if let Some(character) = post
                    .glyph_name(skrifa::raw::types::GlyphId16::new(glyph))
                    .filter(|name| !name.is_empty())
                    .and_then(read_fonts::ps::agl::name_to_char)
                {
                    by_glyph.insert(glyph, character);
                }
            }
            by_glyph
        })
    }

    /// §9.10.2's last resort, once the program has been asked and has said nothing.
    ///
    /// The clause states the outcome and the licence in one sentence — see
    /// [`LoadedFont::text_from_program`], which quotes it — and this is the second thing that
    /// sentence permits: the **code itself**, where it is a printable ASCII byte.
    ///
    /// 0x21 to 0x7E is the range in which a byte and a Unicode code point mean the same character
    /// under every encoding §9.6.5 states, so a code outside it is one this declines rather than
    /// guesses at. Space is excluded because a readback of whitespace is what `Interpretation` uses
    /// to tell a missing mark from a blank one.
    ///
    /// `issue2017r.pdf` is the witness: a symbolic `TrueType` subset with no `/Encoding` at all,
    /// whose `post` table names nothing and whose `cmap` is a (3, 0) symbolic subtable — inverting
    /// that gives *codes* rather than Unicode, so every method above correctly declines and a page
    /// reading `ABCDEFGHIJKLMNOPQRSTUVWYZ` read back as nothing.
    ///
    /// **A code of more than one byte is declined**, and the guard is written down because the
    /// argument above is entirely about bytes: §9.6.5's encodings are one byte per code, so a
    /// two-byte code whose *value* happens to be 0x004A is not "the letter J spelled as a byte",
    /// it is a `CMap`'s two-byte code that no encoding of the standard's has anything to say
    /// about. Only a simple font could reach here until composite fonts joined
    /// [`LoadedFont::text_from_program`], which is why this costs nothing today and would have
    /// been wrong tomorrow — `PDFBOX-4322-Empty-ToUnicode-reduced.pdf` shows `<004a0075…>` and
    /// would read back `Justin` from the arithmetic rather than from its font.
    fn text_from_the_code(code: Code, out: &mut String) -> bool {
        if code.length() != 1 {
            return false;
        }
        let Ok(byte) = u8::try_from(code.value()) else {
            return false;
        };
        if !(0x21..=0x7E).contains(&byte) {
            return false;
        }
        out.push(char::from(byte));
        true
    }

    /// Splits a PDF string into character codes.
    ///
    /// One byte per code for a simple font (§9.7.1: "each byte of a string to be shown selects
    /// one glyph"); for a composite font, whatever its `CMap`'s codespace ranges say, which
    /// may be one to four bytes and may differ from code to code within one string. Getting
    /// this wrong does not merely shift text, it reads entirely different glyphs.
    #[must_use]
    pub fn decode(&self, bytes: &[u8]) -> Vec<Code> {
        match &self.mapping {
            CodeMapping::Named(_) => bytes.iter().copied().map(Code::single_byte).collect(),
            CodeMapping::Composite { cmap, .. } | CodeMapping::Substituted { cmap, .. } => {
                let mut codes = Vec::new();
                let mut rest = bytes;
                while !rest.is_empty() {
                    let code = cmap.next_code(rest);
                    // `next_code` never reports fewer than one byte, so this terminates.
                    let taken = usize::from(code.length()).clamp(1, rest.len());
                    rest = rest.get(taken..).unwrap_or_default();
                    codes.push(code);
                }
                codes
            }
        }
    }

    /// Returns a code's advance width in text-space units, where one em is 1.0.
    ///
    /// A simple font's `/Widths` is indexed by character code; a composite font's `/W` is
    /// indexed by CID (§9.7.4.3), so the code goes through the `CMap` first. A code the `CMap`
    /// does not define takes CID 0's width, because CID 0's glyph is what §9.7.6.3 says is
    /// drawn.
    #[must_use]
    pub fn advance(&self, code: Code) -> f32 {
        self.widths
            .get(&self.selector(code))
            .copied()
            .unwrap_or(self.default_width)
            / 1000.0
    }

    /// How many bytes the glyph program this font draws from occupies.
    ///
    /// What a caller keeping fonts across pages charges itself for — `pdf_model::FontCache` is
    /// the one, and its `FONT_BUDGET` is what it bounds. The *decoded*
    /// program rather than the stream in the file, because that is what is held; and for a
    /// substituted font it is the face this crate stood in with, which is held just the same.
    ///
    /// **The program is not the whole of what a loaded font retains** — the widths, the
    /// `CMap`s, the glyph names and the outlines built on demand sit beside it — and this
    /// deliberately does not guess at those. It is the term that dominates and the only one
    /// that is exact; what the rest add is measured rather than estimated, as peak resident
    /// memory over a sweep (ADR 0710).
    #[must_use]
    pub fn program_bytes(&self) -> usize {
        self.data.len()
    }

    /// The advance the *embedded font program* states for a code, in the units
    /// [`Self::advance`] answers in: one em is 1.0.
    ///
    /// The two are the two halves of ISO 32000-2 §9.6.2.1's Table 109 —
    ///
    /// > These widths shall be consistent with the actual widths given in the font program.
    ///
    /// — and [`Self::advance`] is the *document's* half: it resolves `/Widths` or `/W`, because
    /// that is what a page is laid out by and what a renderer must obey. This is the other
    /// half, read out of the program's `hmtx` record or its charstring's leading width operand,
    /// so that a caller comparing the two is comparing two statements rather than one with
    /// itself.
    ///
    /// It exists for the callers that have to make exactly that comparison: ISO 19005-2 section
    /// 6.2.11.5 and ISO 19005-4 section 6.2.10.5 require the two to agree to within a thousandth of
    /// a text-space unit, and `pdf-model/tests/composite_fonts.rs` asks the same question of the
    /// corpus by reading `hmtx` by hand. A conformance check reading it a second time would have
    /// been a second font reader in a crate whose whole design says it has none.
    ///
    /// # `None` is an answer about this reader, never about the file
    ///
    /// Every refusal below exists so that a caller reporting a disagreement is reporting the
    /// producer's, and each one is a way this could otherwise accuse a sound file:
    ///
    /// - **A substituted font.** The advances would be the stand-in face's, which is a fact
    ///   about this machine ([`Self::is_substituted`]).
    /// - **A repaired CID-keyed CFF.** [`Self::repair_shortfall`] says what was done: the Font
    ///   DICTs that could not be read were replaced by empty ones, and a Type 2 charstring's
    ///   advance is decided by its Private DICT's `defaultWidthX` and `nominalWidthX` — so the
    ///   number that came back would be this crate's rather than the program's.
    /// - **A bare Type 1 program**, whose `hsbw` width this crate does not read.
    /// - **A code that reaches no glyph, or reaches `.notdef`.** §9.7.6.3's CID-0 fallback and
    ///   §9.6.5.2's `.notdef` substitution are what happens when the font is *drawn*; here they
    ///   are not applied, and a code the tables send to glyph 0 outright is declined for the
    ///   same reason [`crate::metrics`]'s substitute scale declines it — that glyph is what a
    ///   character the program does not have resolves to, so its advance is a statement about
    ///   absence rather than about this code.
    ///
    /// A Type 3 font never reaches this method at all — [`Self::load`] refuses one with
    /// [`FontError::Type3`], its glyphs being content streams rather than a program.
    #[must_use]
    pub fn program_advance(&self, code: Code) -> Option<f32> {
        let glyph = self.program_glyph(code)?;
        let advance = metrics::program_advance(self.program, &self.data, glyph)?;
        self.in_ems(advance)
    }

    /// The vertical displacement the *embedded font program* states for a code, in the units
    /// [`Self::vertical_metrics`] answers in and with its sign convention: downward is negative.
    ///
    /// The program-side counterpart of the first number [`Self::vertical_metrics`] returns. That
    /// method resolves §9.7.4.3's `/DW2` and `/W2`, which are the *`CIDFont` dictionary's*
    /// statement of `w1`; this reads OpenType's `vmtx`, which is the program's. ISO 19005-4 section
    /// 6.2.10.5 requires the two to agree where the program states them at all, and there is no way
    /// to ask that question of one number.
    ///
    /// `None` where the program states nothing — which is the common case, since a face never
    /// meant to be set vertically carries no `vmtx` — and for every refusal
    /// [`Self::program_advance`] lists, for the same reasons. It is deliberately *not* gated on
    /// [`Self::is_vertical`]: whether the file shows the font in writing mode 1 is the caller's
    /// question, and this answers only what the program says.
    #[must_use]
    pub fn program_vertical_advance(&self, code: Code) -> Option<f32> {
        let glyph = self.program_glyph(code)?;
        let height = metrics::program_advance_height(self.program, &self.data, glyph)?;
        // `vmtx` states a distance and §9.7.4.3 states a displacement, which for writing mode 1
        // runs down the page; the sign is PDF's convention rather than the table's.
        self.in_ems(-height)
    }

    /// The `cmap` subtables the embedded program carries, as platform and encoding ID pairs.
    ///
    /// ISO 32000-2 §9.6.5.4 names the subtables it uses by exactly those two numbers, and so
    /// do the requirements written on top of it — ISO 19005-2 section 6.2.11.6 and ISO 19005-4
    /// Section 6.2.10.6 among them. This crate's own reader keeps only the three the subclause's
    /// algorithm needs; a caller asking what a font *contains* rather than what it can draw
    /// needs the whole list, in the table's order.
    ///
    /// `None` for a font whose program is not an sfnt — a bare CFF or Type 1 program has
    /// nowhere to put a `cmap` — and for a substituted one, whose tables are this machine's.
    /// An sfnt carrying no `cmap` at all is an empty list, which is a statement about the file.
    #[must_use]
    pub fn program_cmap_subtables(&self) -> Option<Vec<(u16, u16)>> {
        if self.substituted {
            return None;
        }
        truetype::cmap_subtables(&self.data)
    }

    /// Every glyph name the embedded **name-keyed** program defines, in the program's own order.
    ///
    /// The set a Type 1 or Type 1C program's charset assigns, which is what a font descriptor's
    /// `/CharSet` string (ISO 32000-2 §9.8.1, Table 122) claims to list. Asked by a caller
    /// checking that claim, so the answer has to be the *program's* set rather than the set the
    /// document's encoding can reach — a name in the program that no code selects is exactly the
    /// case such a claim gets wrong.
    ///
    /// `.notdef` is included: it is a glyph the program defines like any other, and whether the
    /// clause counting them means to count it is the caller's reading rather than this crate's.
    ///
    /// `None` where the question does not apply or cannot be answered honestly: a substituted
    /// font, whose program is this machine's; an sfnt, which keys its glyphs by index; and a
    /// CID-keyed CFF, whose charset assigns CIDs rather than names (see
    /// [`Self::program_character_identifiers`]).
    #[must_use]
    pub fn program_glyph_names(&self) -> Option<Vec<String>> {
        if self.substituted {
            return None;
        }
        let names = |keyed: &NameKeyed| {
            keyed
                .by_name
                .keys()
                .map(|name| name.as_ref().to_owned())
                .collect()
        };
        match self.program {
            Program::Type1 => Some(names(&self.type1.as_ref()?.code_to_glyph().ok()?)),
            Program::BareCff => match CodeToGlyph::read(&self.data).ok()? {
                CodeToGlyph::Named(keyed) => Some(names(&keyed)),
                CodeToGlyph::Keyed { .. } => None,
            },
            Program::Sfnt => None,
        }
    }

    /// Every character identifier the embedded **CID-keyed** program defines.
    ///
    /// The counterpart of [`Self::program_glyph_names`] for a composite font, and what a font
    /// descriptor's `/CIDSet` stream (§9.8.3.1, Table 124) claims to mark. The two formats state
    /// it differently and both are read:
    ///
    /// - A **CID-keyed CFF** — a `CIDFontType0`'s `/FontFile3` — assigns a CID to each glyph in
    ///   its charset, so the set is that charset's values.
    /// - An **sfnt** — a `CIDFontType2`'s `/FontFile2` — has no CIDs of its own. §9.7.4.2 says a
    ///   CID reaches a glyph through the `CIDFont`'s `/CIDToGIDMap`, and the identity map is
    ///   both the default and what all but a handful of files state, so the identifiers are the
    ///   glyph indices the program holds. A caller for which that assumption is wrong has the
    ///   document's `/CIDToGIDMap` in front of it and this crate does not.
    ///
    /// `None` for a substituted font, for a name-keyed program, and where the program cannot be
    /// read at all.
    #[must_use]
    pub fn program_character_identifiers(&self) -> Option<Vec<u16>> {
        if self.substituted {
            return None;
        }
        match self.program {
            Program::BareCff => match CodeToGlyph::read(&self.data).ok()? {
                CodeToGlyph::Keyed { by_cid } => Some(by_cid.into_keys().collect()),
                CodeToGlyph::Named(_) => None,
            },
            Program::Sfnt => {
                let font = FontRef::new(&self.data).ok()?;
                let count = font.maxp().ok()?.num_glyphs();
                Some((0..count).collect())
            }
            Program::Type1 => None,
        }
    }

    /// The glyph a code selects **in the producer's own program**, or nothing.
    ///
    /// Deliberately stricter than [`Self::glyph_for`], which answers what gets *drawn* and
    /// therefore applies §9.7.6.3's fallbacks to CID 0. The two advance methods above compare a
    /// number the program states against a number the document states, and a fallback glyph's
    /// metrics are a statement about a code the font does not have. See
    /// [`Self::program_advance`] for the whole list of what this declines and why.
    fn program_glyph(&self, code: Code) -> Option<u16> {
        if self.substituted || self.font_dicts.is_some() {
            return None;
        }
        let glyph = match &self.mapping {
            CodeMapping::Named(table) => *table.get(usize::try_from(code.value()).ok()?)?,
            CodeMapping::Composite { cmap, glyphs } => {
                cmap.cid(code).and_then(|cid| glyphs.glyph(cid))
            }
            CodeMapping::Substituted { .. } => None,
        }?;
        (glyph != NOTDEF_GLYPH).then_some(glyph)
    }

    /// One of the program's own units as a fraction of the em, for the two advance methods above.
    ///
    /// `None` for a program stating no em square, which no reader here produces and which would
    /// otherwise divide by zero.
    fn in_ems(&self, units: f32) -> Option<f32> {
        (self.units_per_em > 0.0).then(|| units / self.units_per_em)
    }

    /// What was done to read the program at all, where its own Font DICTs could not be.
    ///
    /// A CID-keyed CFF some of whose Font DICTs could not be read draws the glyphs under them
    /// against an empty Private DICT ([`cff::FontDictRepair`]), and the sentence says so and how
    /// many of those glyphs call a local subroutine that DICT cannot hold. It is a fact about the
    /// program; whether the page shows one of those glyphs is [`Self::glyph_lost_to_repair`]'s
    /// question, asked per code, and a page that shows none has lost nothing and reports nothing.
    #[must_use]
    pub fn repair_shortfall(&self) -> Option<String> {
        let repair = self.font_dicts.as_ref()?;
        Some(format!(
            "{} of the {} Font DICTs its CID-keyed CFF selects cannot be read (FDArray at offset \
             {}), so the glyphs under them are drawn against an empty Private DICT, and the {} \
             of its {} glyphs that call a local subroutine that DICT cannot hold are not drawn",
            repair.replaced,
            repair.font_dicts,
            repair.fd_array_at,
            repair.lost.len(),
            repair.glyphs
        ))
    }

    /// Whether `code` reaches a glyph the repaired program cannot draw — one that calls a local
    /// subroutine the replacement Private DICT does not hold. `false` for a program that needed
    /// no repair.
    #[must_use]
    pub fn glyph_lost_to_repair(&self, code: Code) -> bool {
        self.font_dicts.as_ref().is_some_and(|repair| {
            self.glyph_for(code)
                .is_some_and(|glyph| repair.lost.contains(&glyph))
        })
    }

    /// Which of `pdf-font`'s `CMap` bounds discarded a mapping this font's `CMap` stated.
    ///
    /// `None` for every simple font, which has no `CMap`, and for every composite font whose
    /// `CMap` was read whole. `Some` names the bound in [`crate::cmap::CUT_BY_RANGES`]'s
    /// vocabulary and means §9.7.6.2's lookup — "[t]he code extracted from the string shall be
    /// looked up in the character code mappings for codes of that length" — will fail for codes
    /// the producer mapped, so §9.7.6.3's CID 0 is drawn in their place. It is a fact about the
    /// font rather than about a code, which is why it is asked once where the font is loaded.
    #[must_use]
    pub fn cmap_truncated(&self) -> Option<&'static str> {
        match &self.mapping {
            CodeMapping::Composite { cmap, .. } | CodeMapping::Substituted { cmap, .. } => {
                cmap.truncated()
            }
            CodeMapping::Named(_) => None,
        }
    }

    /// Which of `pdf-font`'s `/ToUnicode` bounds discarded a mapping one of this font's
    /// Unicode tables stated.
    ///
    /// `None` where every table was read whole, which is every font that states none. `Some`
    /// names the bound in [`crate::tounicode::CUT_BY_SINGLES`]'s vocabulary.
    ///
    /// **Two tables are asked, because a font can hold two** and they answer different
    /// questions: the producer's own `/ToUnicode` (§9.10.2's first method) and the character
    /// collection's `registry-ordering-UCS2` table (its third). For a composite font with no
    /// usable program the same two are what [`CodeMapping::Substituted`] selects glyphs through,
    /// which is why this is not only about extracted text: §9.7.4.2 says that with the program
    /// absent "CIDs shall not participate in glyph selection", so the substitute's glyph is
    /// reached *through* the character, and a mapping the bound discarded is a glyph the page
    /// does not draw. (This said *three* while the substituted variant held a copy of one of
    /// the two; it holds none since session 981.)
    #[must_use]
    pub fn to_unicode_truncated(&self) -> Option<&'static str> {
        self.to_unicode.truncated().or_else(|| {
            self.collection
                .as_ref()
                .and_then(tounicode::ToUnicode::truncated)
        })
    }

    /// Whether this font is shown in §9.2.4's writing mode 1, one glyph below the next.
    ///
    /// Set by the `CMap`'s `/WMode` (§9.7.5.1) and available only to a composite font, which
    /// is the clause's own restriction: "this feature is available only for composite fonts".
    #[must_use]
    pub fn is_vertical(&self) -> bool {
        self.vertical.is_some()
    }

    /// §9.7.4.3's vertical displacement `w1` and position vector `v`, in text-space units.
    ///
    /// `w1`'s horizontal component is 0 and `v` is the offset from the horizontal origin to
    /// the vertical one — so a glyph drawn in writing mode 1 is placed at `-v` from the
    /// current text position, and the position then moves by `w1`.
    ///
    /// Returns the horizontal metrics' degenerate form — no displacement, no offset — for a
    /// font in writing mode 0, so a caller that asks without checking gets a glyph that does
    /// not move rather than one that moves wrongly.
    #[must_use]
    pub fn vertical_metrics(&self, code: Code) -> ([f32; 2], [f32; 2]) {
        let Some(vertical) = self.vertical.as_ref() else {
            return ([0.0, 0.0], [0.0, 0.0]);
        };
        let cid = self.selector(code);
        let width = self.widths.get(&cid).copied().unwrap_or(self.default_width);
        let (displacement, position) = vertical.metrics(cid, width);
        (
            [displacement[0] / 1000.0, displacement[1] / 1000.0],
            [position[0] / 1000.0, position[1] / 1000.0],
        )
    }

    /// Returns the outline for a character code, with one em as one unit.
    ///
    /// That is the space PDF's text matrix expects, so the caller multiplies by the font
    /// size and nothing else.
    ///
    /// Returns `None` when the code has no glyph, which includes the ordinary case of a
    /// space in a font with no space outline.
    ///
    /// # §9.6.5.2's `.notdef`, and why it is applied here rather than in the code table
    ///
    /// > If an encoding maps to a character name that does not exist in the Type 1 font program,
    /// > the .notdef glyph shall be substituted.
    ///
    /// The condition is exactly what it says: the *encoding named a glyph* and the program does
    /// not have it. A code the encoding says nothing about is not this sentence's subject and
    /// still reaches nothing.
    ///
    /// It is applied at the drawing step and deliberately not written into the code table,
    /// because the table is what [`Self::glyph_index`] answers with and three of this project's
    /// instruments read that answer: `codes_without_a_glyph` counts the codes a page showed that
    /// reached none (ADR 0152), `simple_code_table` refuses a font whose every code resolved to
    /// nothing, and the whitespace check tells a blank glyph from an absent one. Substituting in
    /// the table would tell all three that every unresolved code had been drawn — a `shall`
    /// obeyed by blinding the gates that watch it.
    #[must_use]
    pub fn outline(&self, code: Code) -> Option<Arc<Path>> {
        let glyph = match self.glyph_for(code) {
            Some(glyph) => glyph,
            None => self.notdef.filter(|_| self.substitutes_notdef(code))?,
        };
        self.cached_outline(glyph)
    }

    /// One glyph's outline, through the cache both routes into this font share.
    ///
    /// Separated from [`Self::outline`] because [`Self::character_glyph`] arrives at a glyph
    /// without a code and must not build a second cache to do it: a face drawing an interface's
    /// own text reuses the same few dozen glyphs exactly as a page does.
    fn cached_outline(&self, glyph: u16) -> Option<Arc<Path>> {
        if let Some(cached) = outlines(&self.outlines).get(&glyph) {
            return cached.clone();
        }
        let built = self.build_outline(glyph);
        outlines(&self.outlines).insert(glyph, built.clone());
        built
    }

    /// What this font's *program* draws for a character, for a caller that has no code.
    ///
    /// **This is not a route a document's text may take, and the distinction is the whole of why
    /// it exists.** A document selects a glyph by character code — §9.6.5's encoding for a simple
    /// font, §9.7.6's `CMap` for a composite one — and drawing a glyph the file did not select
    /// would be inventing what the page says. What has no code at all is the text a *program*
    /// draws for itself: a panel of §12.3.3's outline titles, §8.11.4.3's layer names, §12.4.2's
    /// page labels. There is no font dictionary behind those and therefore no encoding, so the
    /// question they ask is the one this answers — which glyph does this face state for this
    /// character — and [`Self::code_for`] cannot answer it, because a simple font's encoding is
    /// 256 codes wide and a panel's text is not.
    ///
    /// `None` where the program states no glyph for the character, and `None` for every program
    /// with no `cmap` at all: a bare CFF or a Type 1 program is keyed by glyph *name*, which is a
    /// different question, and none of the compiled-in faces so keyed carries anything outside
    /// the standard Latin character set for it to find (ADR 0270). A caller reports the absence —
    /// `viewer_ui::chrome` draws a box for it (ADR 0195).
    ///
    /// The advance is the program's own, from `hmtx`, because there is no `/Widths` array to
    /// disagree with: an interface's text is nothing a document stated a width for.
    ///
    /// **A `cmap` entry naming [`NOTDEF_GLYPH`] answers nothing**, on this crate's own reading of
    /// what glyph 0 is: a statement of absence rather than a glyph. Drawing it would put a
    /// designer's box on the screen while telling the caller the character was set, which is
    /// exactly the confident wrong mark a placeholder exists to avoid.
    #[must_use]
    pub fn character_glyph(&self, character: char) -> Option<CharacterGlyph> {
        let program = FontRef::new(&self.data).ok()?;
        let glyph = program.charmap().map(character)?;
        if glyph.to_u32() == u32::from(NOTDEF_GLYPH) {
            return None;
        }
        let advance = program
            .glyph_metrics(Size::unscaled(), LocationRef::default())
            .advance_width(glyph)?
            / self.units_per_em;
        Some(CharacterGlyph {
            outline: u16::try_from(glyph.to_u32())
                .ok()
                .and_then(|glyph| self.cached_outline(glyph)),
            advance,
        })
    }

    /// Whether §9.6.5.2's substitution applies to `code`.
    ///
    /// Two conditions, and the second is a documented departure.
    ///
    /// **The clause's own**: the encoding "maps to a character name" — so a code no encoding
    /// names is not this sentence's subject and still reaches nothing.
    ///
    /// **And not for a space.** A subset font routinely omits `space` because it has no marks,
    /// and a designer's `.notdef` is routinely a box: `PDF-Declarations.pdf`'s bare CFF fonts
    /// have one of 27 path commands. Obeying the sentence for a code that *means whitespace*
    /// would put a box where every reader expects a gap, which is trap 1's shape — a confident
    /// wrong mark rather than an honest absence — and it is the same distinction ADR 0157 drew
    /// when it exempted a whitespace readback from the missing-glyph count. The clause's
    /// permission is about a designer choosing what a *missing character* looks like; nobody
    /// designs the appearance of an absent space.
    ///
    /// **Measured before it was written**: over the 974 corpus documents' first pages and the 14
    /// specification PDFs, applying the substitution changes not one pixel — the oracle's 1794
    /// verdicts, the corpus's report list and the text gate are all unmoved. So this is a clause
    /// implemented for the documents that will arrive rather than for the ones already here.
    fn substitutes_notdef(&self, code: Code) -> bool {
        let Some(names) = self.glyph_names.as_ref() else {
            return false;
        };
        let encoded = usize::try_from(code.value())
            .ok()
            .and_then(|code| names.get(code))
            .is_some_and(|name| !name.is_empty());
        let mut meaning = String::new();
        let whitespace = self.text(code, &mut meaning)
            && !meaning.is_empty()
            && meaning.chars().all(char::is_whitespace);
        encoded && !whitespace
    }

    /// The character selector a code resolves to, which for a composite font is a CID.
    ///
    /// Only the two lookups §9.7.6.2 names, in its order: the character mappings, then the
    /// notdef mappings. Failing both, CID 0, which §9.7.6.3's NOTE states:
    ///
    /// > If the `CMap` does not contain either a character mapping or a notdef mapping for the
    /// > code, descendant 0 shall be selected and the glyph for CID 0 shall be substituted from
    /// > the associated `CIDFont`.
    ///
    /// A simple font has no CID and its code indexes both its glyph table and its `/Widths`
    /// directly, so the code is its own selector.
    fn selector(&self, code: Code) -> u32 {
        match &self.mapping {
            CodeMapping::Named(_) => code.value(),
            CodeMapping::Composite { cmap, .. } | CodeMapping::Substituted { cmap, .. } => cmap
                .cid(code)
                .or_else(|| cmap.notdef_cid(code))
                .unwrap_or(0),
        }
    }

    /// Resolves a character code to a glyph index.
    ///
    /// Deliberately not memoised. Two of the mappings build a `FontRef` here, which looks
    /// like a per-character cost worth caching — but measuring it on a dense specification
    /// page (3587 lookups, 211 distinct codes, two thirds of them through the character
    /// map) moved the interpretation pass by less than the run-to-run noise. `FontRef` is
    /// a zero-copy view over the table directory, not a parse. A cache here would be
    /// unmeasured cleverness, and `CLAUDE.md` forbids that.
    fn glyph_for(&self, code: Code) -> Option<u16> {
        match &self.mapping {
            CodeMapping::Composite { cmap, glyphs } => {
                // §9.7.6.3's two fallbacks, in its order. "If a code maps to a CID for which
                // no such glyph exists in the descendant CIDFont, the notdef mappings in the
                // CMap shall be consulted … If no glyph exists for that CID, the glyph for
                // CID 0 (which shall be present) shall be substituted." The second is also
                // the sentence about a `/CIDToGIDMap` stream too short for a CID: "if a
                // (character) code does not have a corresponding GID in the CIDtoGIDMap
                // stream, the glyph for CID 0 shall be substituted".
                if let Some(glyph) = cmap.cid(code).and_then(|cid| glyphs.glyph(cid)) {
                    return Some(glyph);
                }
                if let Some(glyph) = cmap.notdef_cid(code).and_then(|cid| glyphs.glyph(cid)) {
                    return Some(glyph);
                }
                glyphs.glyph(0)
            }
            // The substitute has no notion of this document's CIDs, so the code is taken
            // to the character it stands for and that character is looked up.
            //
            // **And then the writing mode is asked, because the character is not the whole of
            // what the producer chose.** §9.7.5.1's NOTE says a vertical `CMap` names a
            // different CID where the shape differs, and the CID-to-Unicode table this arm
            // reaches the substitute through is keyed to the *character* — Adobe-Japan1's 7911
            // and 686 are both U+300C. So where the collection says this CID is a vertical form
            // and the face states one for the glyph, that is the glyph the file asked for.
            // [`crate::vertical`] holds the argument and names what neither half is derived
            // from.
            CodeMapping::Substituted { .. } => {
                let (_, glyph, form) = self.substituted_glyph(code)?;
                Some(match form {
                    Form::Rotated(rotated) => rotated,
                    Form::Upright | Form::Unsupplied => glyph,
                })
            }
            // Resolved when the font was loaded. A code with no entry has no glyph, and
            // that is final: falling back to the code as a glyph index here is exactly
            // how a font draws plausible, wrong text.
            CodeMapping::Named(table) => *table.get(usize::try_from(code.value()).ok()?)?,
        }
    }

    /// One code's whole answer from a substituted composite font: what it means, which glyph of
    /// the face that is, and what the writing mode makes of it.
    ///
    /// One function because two callers must not be able to disagree. [`Self::glyph_for`] draws
    /// what this returns and [`Self::unsupplied_vertical_form`] counts what it could not draw,
    /// and a count taken from a second walk of the same route would be a measurement of that
    /// walk (trap 13). `None` for every font that is not a substituted composite one, and for a
    /// code §9.10.2 gives no character or the face has no glyph for — that second silence is
    /// [`Self::uncovered_character`]'s and is deliberately not this one's.
    fn substituted_glyph(&self, code: Code) -> Option<(char, u16, Form)> {
        let CodeMapping::Substituted { cmap, downward } = &self.mapping else {
            return None;
        };
        let font = FontRef::new(&self.data).ok()?;
        let character = self.substituted_character(cmap, code)?;
        let glyph = u16::try_from(font.charmap().map(character)?.to_u32()).ok()?;
        let form = downward.as_ref().map_or(Form::Upright, |downward| {
            cmap.cid(code)
                .map_or(Form::Upright, |cid| downward.form_of(character, cid, glyph))
        });
        Some((character, glyph, form))
    }

    /// The single character a substituted composite font's code stands for, by ISO 32000-2
    /// §9.10.2's methods in the clause's order.
    ///
    /// > A PDF processor can use these methods, in the priority given, to map a character code
    /// > to a Unicode value.
    ///
    /// The first method is the producer's `/ToUnicode` and the third is the collection's own
    /// table, and **the ranking is per code**: the clause's closing sentence speaks of methods
    /// that "fail to produce a Unicode value", and a `/ToUnicode` that omits a code has failed
    /// for that code however many others it answers. So the `/ToUnicode` is asked first and is
    /// final where it *states* the code — even with a sequence, which addresses no single glyph
    /// and answers `None` here, because the producer's statement is not to be second-guessed by
    /// a table that outranks nothing — and the collection is asked, through §9.10.2's step (a),
    /// only where the producer's table says nothing at all. That is the same order and the same
    /// two tables as [`Self::text`]'s first two branches, which is the point: until session 981
    /// this route took one table for the whole font and the readback took both per code, and
    /// a page could name a character it did not draw (ADR 1002).
    ///
    /// One character rather than a string because this is what *substitution* needs: a
    /// substitute face is addressed by character, so a code standing for a cluster has no glyph
    /// to look up.
    fn substituted_character(&self, cmap: &CMap, code: Code) -> Option<char> {
        if self.to_unicode.states(code.value()) {
            return self.to_unicode.char_for(code.value());
        }
        self.collection
            .as_ref()
            .and_then(|table| table.char_for(cmap.cid(code)?))
    }

    /// The character whose *vertical form* this code named and the substituted face did not have.
    ///
    /// `Some` in one situation, which is ISO 32000-2 §9.7.5.1's NOTE going unhonoured: the font
    /// is substituted, its `CMap` is in writing mode 1, the character collection calls this code's
    /// CID that character's vertical form, the face has a glyph for the character — and states no
    /// `vert` or `vrt2` substitution for it. The page then draws the producer's character in the
    /// producer's place in whatever shape the substitute had, which is a mark made and not a mark
    /// missed.
    ///
    /// **A count and not a report**, on ADR 0152's arithmetic: this says something about a face
    /// rather than about a file, and a report would take a page off the oracle's judged set for
    /// it. **And disjoint from [`Self::uncovered_character`]**, which is the distinction ADR 0764
    /// exists to draw: that one is a character the face cannot draw at all, this one is a
    /// character it draws in the wrong shape, and until they were counted apart they were one
    /// silence with one number.
    #[must_use]
    pub fn unsupplied_vertical_form(&self, code: Code) -> Option<char> {
        let (character, _, form) = self.substituted_glyph(code)?;
        (form == Form::Unsupplied).then_some(character)
    }

    /// The character a code stands for, where the substitute cannot draw it.
    ///
    /// `Some` only in one situation, and it is the one worth reporting: the font was
    /// **substituted**, §9.10.2 gave the code a character, and the face this machine offered
    /// has no glyph for that character. Everything else answers `None` — an embedded font, a
    /// code the mapping does not cover (which §9.7.6.3 answers with CID 0), or a character the
    /// face draws.
    ///
    /// It exists because a font is otherwise reported as a whole: [`FontError`] is the only
    /// channel a font has, so a substitute that draws *some* of a document's characters draws
    /// those and says nothing about the rest — and one that draws none of them, on a page
    /// whose every code is Chinese, drew a blank page in silence. `issue8372.pdf` is that
    /// page: `AdobeHeitiStd-Regular`, not embedded, `Adobe-GB1` through `UniGB-UTF16-H`, and
    /// the substitute a family match finds is a Latin face with no 目 in it.
    ///
    /// A space is why this asks the question in this direction rather than by looking for a
    /// missing outline: U+0020 is in every face, so a blank glyph and an absent one stay
    /// distinguishable without the caller having to know which is which.
    #[must_use]
    pub fn uncovered_character(&self, code: Code) -> Option<char> {
        let CodeMapping::Substituted { cmap, .. } = &self.mapping else {
            return None;
        };
        let font = FontRef::new(&self.data).ok()?;
        let character = self.substituted_character(cmap, code)?;
        font.charmap().map(character).is_none().then_some(character)
    }

    /// Whether this code reached its glyph only by a mapping ISO 32000-2 §9.6.5.4 leaves open.
    ///
    /// That subclause sets out how a simple `TrueType` or `OpenType` font's codes reach glyphs
    /// and then closes the list with a licence:
    ///
    /// > If a character cannot be mapped in any of the ways described previously, a PDF
    /// > processor may supply a mapping of its choosing.
    ///
    /// [`crate::truetype`] supplies two such mappings, and this says which codes they answered.
    /// `false` for a code the subclause's own steps reached, for a code that reaches no glyph at
    /// all, and for every font of another kind — a composite font, a Type 1 or bare CFF program,
    /// or a substitute — because none of those is what that sentence is about.
    ///
    /// # Why this is exposed
    ///
    /// Because a *conforming file* is the one thing the licence is not addressed to. ISO 19005-2
    /// Section 6.2.11.6 and ISO 19005-4 section 6.2.10.6 require a rendered TrueType font's codes
    /// to reach glyphs by the base standard's procedure without a mapping the reader chooses — so a
    /// validator has to be able to ask which route drew a code, and the route is knowable only
    /// where the table is built. It costs a viewer thirty-two bytes per loaded font and no work at
    /// all: the bit is set inside a loop that was already running.
    #[must_use]
    pub fn glyph_needed_a_reader_chosen_mapping(&self, code: Code) -> bool {
        self.reader_chosen.contains(code.value())
    }

    /// The glyph index a character code reaches, or `None` where it reaches none.
    ///
    /// Public for one reason: the strongest check in this tree is that the document's stated
    /// width for a code and the font program's own advance for the glyph that code reaches
    /// agree, and those two statements travel through completely separate structures — so they
    /// agree only if the whole chain (§9.7.6.2's `CMap`, §9.7.4.2's `CIDToGIDMap` or charset)
    /// landed on the glyph the producer meant. That check needs the glyph index, and it
    /// verifies the mapping without consulting the mapping. See
    /// `pdf-model/tests/composite_fonts.rs`.
    #[must_use]
    pub fn glyph_index(&self, code: Code) -> Option<u16> {
        self.glyph_for(code)
    }

    /// Whether [`Self::code_for`] can answer for this font at all.
    ///
    /// The distinction is public so a caller can report *which* of the two refusals it hit —
    /// "this font lacks that character" and "this font cannot be addressed by character" are not
    /// the same statement.
    ///
    /// **This answered `false` for every composite font until the five-hundred-and-second
    /// session**, on the true observation that a `CMap`'s codespace ranges decide a code's length
    /// (§9.7.6.2) and the false conclusion that nothing could invert them.
    /// [`crate::cmap::CMap::each_addressable_code`] does, and the one case left is a `CMap`
    /// stating more codes than that walk will visit.
    #[must_use]
    pub fn addresses_characters(&self) -> bool {
        self.addressable_codes().is_some()
    }

    /// The code that draws a character, for a font this crate can address that way.
    ///
    /// The inverse of [`Self::text`], and deliberately built *by running it*: every code the
    /// font defines is asked what it means and what glyph it reaches, and a character is
    /// answered with the first code that both means it and has a glyph. That construction is
    /// what makes the answer trustworthy — a code this returns is a code that draws the
    /// character asked for, because the two directions traverse the same tables.
    ///
    /// Needed by ISO 32000-2 §12.7.4.3, where a processor writes the content stream itself: a
    /// field's value arrives as a §7.9.2.2 text string and has to leave as bytes in the
    /// font's own encoding. Every other route through this crate starts from a code the
    /// document already wrote.
    #[must_use]
    pub fn code_for(&self, character: char) -> Option<Code> {
        self.addressable_codes()?.get(&character).copied()
    }

    /// The codes this font can be addressed by, keyed by the character each one draws.
    ///
    /// Two populations, and which one a font has is the whole of the difference between a
    /// simple font and a composite one. A simple font's codes are the 256 §9.7.1 gives it —
    /// "each byte of a string to be shown selects one glyph" — and the whole set can be walked.
    /// A composite font's are whatever its `CMap` states and its codespace admits, which is
    /// [`crate::cmap::CMap::each_addressable_code`].
    ///
    /// `None` where the walk declined, which is one case: a `CMap` stating more codes than
    /// [`MAX_ADDRESSABLE_CODES`]. Nothing is answered from a table that stopped early, because a
    /// caller cannot tell "no code for this character" from "the search gave up" and would
    /// report the first while the second is true.
    fn addressable_codes(&self) -> Option<&BTreeMap<char, Code>> {
        self.codes_by_character
            .get_or_init(|| self.build_addressable_codes())
            .as_ref()
    }

    /// Builds the table [`Self::addressable_codes`] hands out, once.
    fn build_addressable_codes(&self) -> Option<BTreeMap<char, Code>> {
        let mut map = BTreeMap::new();
        let mut meaning = String::new();
        let mut consider = |code: Code| {
            meaning.clear();
            if !self.text(code, &mut meaning) {
                return;
            }
            let mut characters = meaning.chars();
            // A code standing for more than one character — an `ffi` ligature's `/ToUnicode`
            // entry — cannot answer "which code draws this character", because using it would
            // draw the other characters too.
            let (Some(single), None) = (characters.next(), characters.next()) else {
                return;
            };
            if self.glyph_for(code).is_none() {
                return;
            }
            map.entry(single).or_insert(code);
        };
        match &self.mapping {
            CodeMapping::Named(_) => {
                for byte in 0..=u8::MAX {
                    consider(Code::single_byte(byte));
                }
            }
            CodeMapping::Composite { cmap, .. } | CodeMapping::Substituted { cmap, .. } => {
                if !cmap.each_addressable_code(MAX_ADDRESSABLE_CODES, &mut consider) {
                    return None;
                }
            }
        }
        Some(map)
    }

    /// The glyph a *character selector* reaches, skipping the code that selected it.
    ///
    /// A simple font's `/Widths` is indexed by character code and a composite font's `/W` by
    /// CID (§9.7.4.3), so this is the key both tables use — which is what the
    /// widths-against-charstrings cross-check needs, since it walks those tables rather than a
    /// string. Drawing always starts from a code; the two callers that do not are that check
    /// and §9.10.2's last resort in [`Self::text_from_program`], which needs the glyph in order
    /// to ask the program what it is called.
    fn glyph_for_selector(&self, selector: u32) -> Option<u16> {
        match &self.mapping {
            CodeMapping::Named(table) => *table.get(usize::try_from(selector).ok()?)?,
            CodeMapping::Composite { glyphs, .. } => glyphs.glyph(selector),
            // A substitute is reached through what a code *means*, so a selector alone
            // cannot name a glyph in it.
            CodeMapping::Substituted { .. } => None,
        }
    }

    /// The interpreter instance for a hint-reliant font program, built on first use.
    ///
    /// `None` for every font the library does not place in that family, and for a family
    /// member whose `fpgm`/`prep` programs fail — both of which draw unhinted, exactly as
    /// every font did before ADR 0727.
    ///
    /// The size is one pixel per design unit, and that is the decision this method exists to
    /// hold: an instruction that rounds to the pixel grid then rounds to the grid the outline
    /// was designed on — a near no-op — while the constructive moves this family's programs
    /// exist for are carried out in full. So the outline stays resolution-independent, which
    /// is what [`Self::outlines`]' cache and every caller rely on, and the grid-fitting half
    /// of hinting stays declined, which is this tree's standing scan-conversion position
    /// (`doc/todo/_scan-conversion.md`). [`Target::Mono`] and [`Engine::Interpreter`] are the
    /// combination [`OutlineGlyphCollection::require_interpreter`]'s own documentation
    /// prescribes for this family.
    fn hinting(&self, glyphs: &OutlineGlyphCollection) -> Option<&HintingInstance> {
        self.hinting
            .get_or_init(|| {
                if !glyphs.require_interpreter() {
                    return None;
                }
                HintingInstance::new(
                    glyphs,
                    Size::new(self.units_per_em),
                    LocationRef::default(),
                    HintingOptions {
                        engine: Engine::Interpreter,
                        target: Target::Mono,
                    },
                )
                .ok()
            })
            .as_ref()
    }

    /// Extracts and normalises one glyph outline.
    fn build_outline(&self, glyph: u16) -> Option<Arc<Path>> {
        let mut pen = PathPen {
            path: Path::new(),
            scale: 1.0 / self.units_per_em,
            stretch: self.stretch,
            last: None,
        };

        match self.program {
            Program::BareCff => cff::draw(&self.data, glyph, &mut pen).ok()?,
            Program::Type1 => self.type1.as_ref()?.draw(glyph, &mut pen).ok()?,
            Program::Sfnt => {
                let font = FontRef::new(&self.data).ok()?;
                let glyphs = font.outline_glyphs();
                let outline = glyphs.get(GlyphId::from(glyph))?;
                // Unhinted and unscaled by default: hinting is a device-resolution decision,
                // and this outline is resolution-independent because the text matrix scales
                // it later. The exception is the hint-reliant family — see [`Self::hinting`]
                // — whose instructions *construct* the contours rather than grid-fit them;
                // those draw through the interpreter, and a bytecode failure falls back to
                // the unhinted skeleton, which is the picture every glyph of the family drew
                // before ADR 0727 rather than no glyph at all.
                let unhinted = || DrawSettings::unhinted(Size::unscaled(), LocationRef::default());
                match self.hinting(&glyphs) {
                    Some(instance) => {
                        if outline
                            .draw(DrawSettings::hinted(instance, false), &mut pen)
                            .is_err()
                        {
                            pen.path = Path::new();
                            pen.last = None;
                            outline.draw(unhinted(), &mut pen).ok()?;
                        }
                    }
                    None => {
                        outline.draw(unhinted(), &mut pen).ok()?;
                    }
                }
            }
        }

        if pen.path.is_empty() {
            return None;
        }
        // §9.3.6 NOTE 2 makes a glyph's contour direction visible where two glyphs overlap
        // inside one clipping path, and a face this program chose has no direction the
        // document stated — so every substituted outline is wound the same way, and an
        // embedded program's is left exactly as its producer drew it.
        if self.substituted {
            return Some(Arc::new(wound_counter_clockwise(pen.path)));
        }
        Some(Arc::new(pen.path))
    }
}

/// Reads a font dictionary's `/ToUnicode` `CMap` and whatever it builds on (§9.10.3).
/// A Type 0 font's `/Encoding` where it is one of §9.7.5.2's predefined names, as text.
///
/// Only [`collection_gap`] asks, and only for the one of its four facts that is stated of the
/// `CMap` rather than of the descendant: §9.7.5.2's "shall not be used with a non-embedded font"
/// is about `Identity-H` and `Identity-V` by name. A stream `/Encoding` — §9.7.5.3's embedded
/// `CMap` — has no name and answers `None`, which is correct: the prohibition names two `CMap`s.
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
/// The same bound [`crate::composite::read_cmap`] puts on the `/Encoding` form, for the same
/// reason.
const MAX_TO_UNICODE_DEPTH: u32 = 4;

/// One `/ToUnicode` stream, with the `CMap` it states only its differences from beneath it.
///
/// §9.10.3 names the one dictionary entry that means anything here, and it is `/UseCMap`, which
/// "may be used if the `CMap` is based on another `ToUnicode` `CMap`".
///
/// **The sentence that names it has been rewritten, and this comment quoted the retired half of
/// it as a blockquote until the five-hundred-and-ninety-first session.** Errata Collection 3's
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
    /// The horizontal scale of [`crate::metrics::substitute_stretch`], applied to x alone.
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
    /// **The sibling of [`no_registered_cmap_is_cut_by_these_bounds`], over the second parser
    /// that reads the same files.** Session 960 measured Adobe's 240 published `CMap`s against
    /// `cmap.rs`'s four bounds and found one of them below what the data states; the same files
    /// also go through [`crate::tounicode`], which has three bounds of its own, and nothing
    /// walked them. §9.10.2's third method reads `registry-ordering-UCS2` for every composite
    /// font whose program is absent, and §9.10.3 lets a producer's `/ToUnicode` name any of the
    /// rest in `/UseCMap` — so the population is the whole set, not the five collection tables.
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
    /// registered `CMap`s — 239 files, compiled in since the hundred-and-fifty-sixth session — and
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
    /// (This sentence quoted §9.6.2.2's "these fonts, or their font metrics and suitable
    /// substitution fonts" until the four-hundred-and-thirty-first session; Errata Collection 3
    /// struck the whole sentence — Issue #47 and #48, `/State` `Review` `Completed` — and
    /// [`crate::standard`] carries the reading that replaces it. `tools/spec-errata` could not
    /// see this one because the quotation lowers the sentence's first letter and the comparison
    /// kept case; it folds case now, and found it.)
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

    /// Loads every first-page font in the corpus, keeping the ones backed by a bare CFF.
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
    /// bytes go back through [`LoadedFont::decode`] and must come out as that one code. This
    /// half of the test was `code_for(…).is_none()` — the refusal composite fonts got until the
    /// five-hundred-and-second session — and the corpus fonts it skipped are now its subject.
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

            for (&code, &declared) in &font.widths {
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
    /// **The assertion moved from `outline` to `glyph_index` in the two-hundred-and-eighty-seventh
    /// session**, when §9.6.5.2's last sentence was implemented: an uncovered code may now draw
    /// the program's own `.notdef`, so the property that catches the fall-through is the one
    /// about the *table* — which is also the answer all three of this project's missing-glyph
    /// instruments read. The second assertion is what keeps that from becoming a hole: every
    /// outline an uncovered code produces must be the **same** outline, because there is one
    /// `.notdef` per program and a fall-through would produce a different glyph per code.
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
    /// **Calibrated both ways** (session 981): with `substituted_character` reading the
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
