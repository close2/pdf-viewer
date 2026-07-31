//! Font loading and glyph outline extraction.
//!
//! Maps a PDF font dictionary plus a character code to a glyph outline, applying the
//! encoding the document specifies.
//!
//! Outline extraction is delegated to `skrifa`, a memory-safe replacement for `FreeType` —
//! historically a steady source of vulnerabilities in every viewer that used it.
//!
//! # Text shaping is deliberately absent
//!
//! A PDF content stream carries glyphs the producer already positioned. Re-shaping them
//! would move glyphs away from the coordinates the document specifies, and would do so most
//! visibly on the complex-script documents where shaping seems most helpful. See
//! `CLAUDE.md` on `rustybuzz`.
//!
//! # What is implemented, and what says so
//!
//! Embedded `TrueType` and CFF outlines, for simple fonts and for composite (Type0) fonts
//! under the Identity encoding or an embedded `CMap` (§9.7.5.3) — which between them cover
//! the overwhelming majority of modern documents. A font this crate cannot load returns an
//! error naming why, so the caller reports the text as undrawn rather than silently omitting
//! it. What is left is the predefined `CMap`s of Table 116, whose data is not in the tree.

#![forbid(unsafe_code)]

pub mod cff;
pub mod cmap;
pub mod encoding;
pub mod name_keyed;
pub mod panose;
pub mod standard_metrics;
pub mod substitute;
pub mod tounicode;
pub mod type1;

use std::borrow::Cow;
use std::cell::{OnceCell, RefCell};
use std::collections::BTreeMap;
use std::sync::Arc;

use pdf_render::{Path, PathCommand, Point};
use pdf_syntax::{Dictionary, Document, Object};
use skrifa::outline::{DrawSettings, OutlinePen};
use skrifa::prelude::{LocationRef, Size};
use skrifa::raw::TableProvider;
use skrifa::raw::tables::cmap::{Cmap, CmapSubtable, PlatformId};
use skrifa::{FontRef, GlyphId, MetadataProvider};

use crate::cff::CodeToGlyph;
use crate::cmap::CMap;
use crate::encoding::BaseEncoding;
use crate::name_keyed::NameKeyed;

pub use crate::cmap::Code;

/// A character code's glyph, for each of the 256 codes a simple font can use.
type CodeTable = [Option<u16>; 256];

/// The glyph name each of a simple font's 256 character codes selects.
///
/// Borrowed for a name one of the specifications lists, which is nearly every name a real
/// document writes, and owned for one only the document's own font program carries — a
/// subsetter's `/gid2436`, say. Both have to be kept: an unrecognised name is *not* an
/// unencoded code, and the font's own `post` table or CFF charset may hold the glyph under
/// exactly that spelling. Dropping them was a defect; see [`apply_differences`].
type GlyphNames = Box<[Cow<'static, str>; 256]>;

/// A table of 256 unencoded codes, which every encoding starts from.
fn no_names() -> GlyphNames {
    Box::new(std::array::from_fn(|_| Cow::Borrowed("")))
}

/// Width of a simple font's code that neither `/Widths` nor `/MissingWidth` covers.
///
/// ISO 32000-2 §9.8.1's Table 120 gives `/MissingWidth` the default value 0, and §9.6.2's
/// Table 109 sends every code outside the declared range to it:
///
/// > For character codes outside the range FirstChar to LastChar , the value of
/// > MissingWidth from the FontDescriptor entry for this font shall be used.
///
/// It used to be half an em here, on the reasoning that spacing degrades more gracefully
/// than it does collapsing to zero. That is a plausible thing to want and not a reading of
/// anything: the clause states the default, and a producer wanting half an em can write
/// it. `issue7439.pdf` is what the difference costs — its one line of text shows code 2
/// six times against a `/FirstChar` of 3 and a descriptor with no `/MissingWidth`, so half
/// an em of invented space opened six times between `Issue` and `7439`. The page was
/// contradicted by the reference consensus and now agrees.
const DEFAULT_WIDTH: f32 = 0.0;

/// How a font maps character codes to glyphs.
#[derive(Debug)]
enum CodeMapping {
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
    /// the font that defined it, so it says nothing about any other font, and only
    /// `/ToUnicode` records what the producer meant by it. The `CMap` is still needed, to
    /// split the string into codes — §9.7.4.2 is explicit that a CID plays no part here:
    /// "In this case, CIDs shall not participate in glyph selection".
    Substituted {
        /// Codes to CIDs, used for the code boundaries and for `/W`'s widths.
        cmap: Box<CMap>,
        /// What the producer said each code means.
        text: Box<tounicode::ToUnicode>,
    },
}

/// How a `CIDFont` turns a CID into a glyph index (ISO 32000-2 §9.7.4.2).
///
/// The clause gives one route per kind of embedded program, and the three arms below are
/// exactly those routes. Which one applies is decided by what the program *is* rather than by
/// the `/Subtype` name, because a `CIDFontType0` may arrive wrapped in an `OpenType`
/// container and glyph selection then still runs through the CFF's charset.
#[derive(Debug)]
enum CidToGlyph {
    /// The CID is the glyph index.
    ///
    /// Two of the clause's cases reach this. A `TrueType` program whose `/CIDToGIDMap` is
    /// `Identity` (§9.7.5.2):
    ///
    /// > the 2-byte CID values shall be identical glyph indices for the glyph descriptions in
    /// > the `TrueType` font program
    ///
    /// And a CFF program whose Top DICT carries no `CIDFont` operators (§9.7.4.2):
    ///
    /// > The CIDs shall be used directly as GID values
    Identity,
    /// A CID-keyed CFF's charset, inverted (§9.7.4.2).
    ///
    /// > The CIDs shall be used to determine the GID value for the glyph procedure using the
    /// > charset table in the CFF program.
    Charset(BTreeMap<u16, u16>),
    /// A `/CIDToGIDMap` stream (§9.7.4.1 Table 115).
    ///
    /// > the glyph index for a particular CID value c shall be a 2-byte value stored in bytes
    /// > 2 × 𝑐 and 2 × 𝑐 + 1 , where the first byte shall be the high-order byte.
    Stream(Arc<[u8]>),
}

impl CidToGlyph {
    /// The glyph index a CID selects, or `None` where the `CIDFont` has none for it.
    fn glyph(&self, cid: u32) -> Option<u16> {
        match self {
            // Glyph indices are 16 bits wide, so a larger CID cannot name one.
            Self::Identity => u16::try_from(cid).ok(),
            Self::Charset(by_cid) => by_cid.get(&u16::try_from(cid).ok()?).copied(),
            Self::Stream(bytes) => {
                let at = usize::try_from(cid).ok()?.checked_mul(2)?;
                let pair = bytes.get(at..at.checked_add(2)?)?;
                let (Some(&high), Some(&low)) = (pair.first(), pair.get(1)) else {
                    return None;
                };
                Some(u16::from_be_bytes([high, low]))
            }
        }
    }
}

/// Which reader extracts outlines from the embedded font program.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Program {
    /// An sfnt container — `TrueType` or `OpenType` — read through `skrifa`'s `FontRef`.
    Sfnt,
    /// A bare Type 1 program — §9.9's `/FontFile` — read through `read-fonts`' Type 1
    /// reader. Name-keyed exactly as a bare CFF is, which is why both reach
    /// [`simple_code_table`] with the same value; see [`crate::name_keyed`].
    Type1,
    /// A bare CFF program, read through `read-fonts`' CFF reader directly.
    ///
    /// Wrapping it in a synthesised sfnt so that `FontRef` would accept it is possible,
    /// but pointless: the CFF reader draws from the bare program, and a synthesised
    /// container would be one more thing to get right for no gain.
    BareCff,
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

/// A font ready to produce glyph outlines.
pub struct LoadedFont {
    /// The embedded font program, which the reader borrows from on each use.
    data: Arc<[u8]>,
    program: Program,
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
    /// §9.7.4.3's second set of metrics, for a composite font in writing mode 1.
    vertical: Option<Vertical>,
    units_per_em: f32,
    /// Whether the glyphs are a stand-in rather than the font the document named.
    substituted: bool,
    /// What the producer said each code means, when the font says so.
    to_unicode: tounicode::ToUnicode,
    /// The glyph name each code selects, for simple fonts.
    ///
    /// The fallback for extraction when there is no `/ToUnicode`: a glyph name identifies
    /// a character through the Adobe Glyph List, and it is what actually selected the
    /// glyph, so it describes what was drawn rather than what the producer claimed.
    glyph_names: Option<GlyphNames>,
    /// Cached outlines: a page reuses the same few dozen glyphs constantly, and
    /// re-extracting each one would dominate the render.
    outlines: RefCell<BTreeMap<u16, Option<Arc<Path>>>>,
    /// The inverse of the code-to-character mapping, built on first use by [`Self::code_for`].
    ///
    /// Lazy rather than built at load time because nothing on a page needs it: only a
    /// constructed appearance (§12.7.4.3) writes a string this crate has to encode, and a
    /// document may load hundreds of fonts without containing one form field.
    codes_by_character: OnceCell<BTreeMap<char, u8>>,
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
    /// The AGL's own share went from 4.26% to 3.35%, and what remains is *not* this path: it
    /// is §9.6.5.4's, which asks the list once per code when a `TrueType` font is loaded, and
    /// which is already once per font.
    ///
    /// Lazy rather than built at load, because a font whose `/ToUnicode` covers its codes
    /// never reaches the list at all, and 256 AGL searches is not a cost to pay on the page-one
    /// path for nothing (`CLAUDE.md` principle 2).
    agl_by_code: OnceCell<Box<[Option<char>; 256]>>,
    /// §9.10.2's last resort: what the *program* calls the glyph a code selected.
    ///
    /// One entry per code of a simple font, built once and only for a font that reaches this
    /// far — see [`LoadedFont::text`], which is the only reader and explains the choice.
    post_by_code: OnceCell<Box<[Option<char>; 256]>>,
}

impl std::fmt::Debug for LoadedFont {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoadedFont")
            .field("bytes", &self.data.len())
            .field("program", &self.program)
            .field("mapping", &self.mapping)
            .field("units_per_em", &self.units_per_em)
            .field("substituted", &self.substituted)
            .finish_non_exhaustive()
    }
}

impl LoadedFont {
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
                let data = substitute::find(request).ok_or_else(|| FontError::NoSubstitute {
                    name: name.to_owned(),
                    reason: format!("no {:?} face is installed", request.family),
                })?;
                (data, Program::Sfnt, Some(request))
            }
            Err(other) => return Err(other),
        };
        let type1 = parsed_type1(program, &data, name)?;
        let units_per_em = match &type1 {
            Some(parsed) => parsed.units_per_em().map_err(|e| FontError::Malformed {
                name: name.to_owned(),
                detail: e.to_string(),
            })?,
            None => units_per_em(&data, program, name)?,
        };

        // Kept for text extraction: a glyph name is what a code means when a font carries
        // no `/ToUnicode`, which is common in older documents.
        let names;
        let mapping = match (program, substituted) {
            // A substitute shares no glyph order with the font the document meant, so its
            // glyphs are reached by what each code *means* rather than by index.
            (_, Some(request)) => {
                let (table, resolved) =
                    substitute_code_table(document, dict, descriptor, request, &data, name)?;
                names = Some(resolved);
                CodeMapping::Named(Box::new(table))
            }
            (Program::Sfnt, None) => {
                let (table, resolved) =
                    truetype_code_table(document, dict, descriptor, &data, name)?;
                names = Some(resolved);
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
                names = Some(resolved);
                CodeMapping::Named(Box::new(table))
            }
            (Program::Type1, None) => {
                let keyed = type1
                    .as_ref()
                    .ok_or_else(|| FontError::NotEmbedded {
                        name: name.to_owned(),
                    })?
                    .code_to_glyph()
                    .map_err(|e| FontError::Malformed {
                        name: name.to_owned(),
                        detail: e.to_string(),
                    })?;
                let (table, resolved) = simple_code_table(document, dict, &keyed, name)?;
                names = Some(resolved);
                CodeMapping::Named(Box::new(table))
            }
        };

        let metrics = SimpleMetrics {
            substituted,
            names: names.as_ref(),
            data: &data,
            mapping: &mapping,
            units_per_em,
        };
        let widths = simple_widths(document, dict, metrics);
        let default_width = missing_width(document, descriptor);

        Ok(Self {
            data,
            program,
            type1,
            mapping,
            widths,
            default_width,
            // §9.2.4: a second set of metrics "is available only for composite fonts".
            vertical: None,
            units_per_em,
            substituted: substituted.is_some(),
            to_unicode: to_unicode(document, dict),
            glyph_names: names,
            outlines: RefCell::new(BTreeMap::new()),
            codes_by_character: OnceCell::new(),
            agl_by_code: OnceCell::new(),
            post_by_code: OnceCell::new(),
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
                let data = substitute::find(request).ok_or_else(|| FontError::NoSubstitute {
                    name: name.to_owned(),
                    reason: format!("no {:?} face is installed", request.family),
                })?;
                (data, Program::Sfnt, true)
            }
            Ok(Embedded { data, program }) => (data, program, false),
            Err(other) => return Err(other),
        };
        // Kept parsed for the same measured reason a simple font keeps it (`type1::Program`),
        // and read for its scale the same way: a Type 1 program states it in a `/FontMatrix`
        // rather than in an `sfnt` header, so `FontRef` cannot be asked.
        let type1 = parsed_type1(program, &data, name)?;
        let units_per_em = match &type1 {
            Some(parsed) => parsed.units_per_em().map_err(|e| FontError::Malformed {
                name: name.to_owned(),
                detail: e.to_string(),
            })?,
            None => units_per_em(&data, program, name)?,
        };

        let mapping = if substituted {
            // A CID is meaningless outside the font that defined it — it is an index into
            // that font's glyphs, not a character — so a substitute can only be reached
            // through what the codes *mean*. `/ToUnicode` is the only thing that says so,
            // and a composite font without one cannot be substituted at all. §9.7.4.2 says
            // the same thing from the other side: with the program absent, "CIDs shall not
            // participate in glyph selection", and a `/CIDToGIDMap` "shall be ignored, since
            // it is not meaningful to refer to glyph indices in an external font program".
            let text = to_unicode(document, dict);
            if text.is_empty() {
                return Err(FontError::UnsupportedEncoding {
                    name: name.to_owned(),
                    encoding: "no /ToUnicode, so a substitute cannot be addressed".to_owned(),
                });
            }
            CodeMapping::Substituted {
                cmap: Box::new(cmap),
                text: Box::new(text),
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
            // §9.9's Table 124 gives a CIDFont `/FontFile2` and `/FontFile3` and never
            // `/FontFile` — but a descriptor writing one is read rather than refused; see
            // the `mapping` above for the clause's own analogy that decides how.
            type1,
            mapping,
            substituted,
            to_unicode: to_unicode(document, dict),
            glyph_names: None,
            widths: composite_widths(document, &descendant),
            default_width,
            vertical: vertical.then(|| Vertical::read(document, &descendant)),
            units_per_em,
            outlines: RefCell::new(BTreeMap::new()),
            codes_by_character: OnceCell::new(),
            agl_by_code: OnceCell::new(),
            post_by_code: OnceCell::new(),
        })
    }

    /// Whether these glyphs stand in for a font the document did not embed.
    ///
    /// The shapes are then not the ones the producer chose, though the metrics still are
    /// wherever the document stated them.
    #[must_use]
    pub fn is_substituted(&self) -> bool {
        self.substituted
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
        let Some(names) = self.glyph_names.as_ref() else {
            return self.text_from_program(code, out);
        };
        let table = self.agl_by_code.get_or_init(|| {
            let mut table = Box::new([None; 256]);
            for (code, slot) in table.iter_mut().enumerate() {
                *slot = names
                    .get(code)
                    .map(Cow::as_ref)
                    .filter(|name| !name.is_empty())
                    .and_then(read_fonts::ps::agl::name_to_char);
            }
            table
        });
        if let Some(character) = usize::try_from(code.value())
            .ok()
            .and_then(|code| table.get(code))
            .copied()
            .flatten()
        {
            out.push(character);
            return true;
        }
        self.text_from_program(code, out)
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
    /// Only for a simple font: a composite one selects by CID through a `CMap`, and §9.10.2's
    /// third method is the route the clause states for those.
    fn text_from_program(&self, code: Code, out: &mut String) -> bool {
        if !matches!(self.mapping, CodeMapping::Named(_)) {
            return false;
        }
        let table = self.post_by_code.get_or_init(|| {
            let mut table = Box::new([None; 256]);
            let Ok(font) = FontRef::new(&self.data) else {
                return table;
            };
            let Ok(post) = font.post() else {
                return table;
            };
            // The program's own Unicode subtable, inverted. Built once and only where the
            // `post` table left something unanswered, because it walks every mapping the font
            // states — a few hundred for a subset, and a few thousand for a full CJK face.
            let mut by_glyph: Option<BTreeMap<u16, char>> = None;
            for (code, slot) in table.iter_mut().enumerate() {
                let Some(glyph) = u32::try_from(code)
                    .ok()
                    .and_then(|code| self.glyph_for_selector(code))
                else {
                    continue;
                };
                *slot = post
                    .glyph_name(skrifa::raw::types::GlyphId16::new(glyph))
                    .filter(|name| !name.is_empty())
                    .and_then(read_fonts::ps::agl::name_to_char)
                    .or_else(|| {
                        by_glyph
                            .get_or_insert_with(|| invert_charmap(&font))
                            .get(&glyph)
                            .copied()
                    });
            }
            table
        });
        match usize::try_from(code.value())
            .ok()
            .and_then(|code| table.get(code))
            .copied()
            .flatten()
        {
            Some(character) => {
                out.push(character);
                true
            }
            None => false,
        }
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
    #[must_use]
    pub fn outline(&self, code: Code) -> Option<Arc<Path>> {
        let glyph = self.glyph_for(code)?;

        if let Some(cached) = self.outlines.borrow().get(&glyph) {
            return cached.clone();
        }
        let built = self.build_outline(glyph);
        self.outlines.borrow_mut().insert(glyph, built.clone());
        built
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
            CodeMapping::Substituted { text, .. } => {
                let font = FontRef::new(&self.data).ok()?;
                let character = text.char_for(code.value())?;
                let id = font.charmap().map(character)?;
                u16::try_from(id.to_u32()).ok()
            }
            // Resolved when the font was loaded. A code with no entry has no glyph, and
            // that is final: falling back to the code as a glyph index here is exactly
            // how a font draws plausible, wrong text.
            CodeMapping::Named(table) => *table.get(usize::try_from(code.value()).ok()?)?,
        }
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
    /// A simple font's codes are one byte and the whole set can be walked; a composite font's
    /// lengths come from its `CMap`'s codespace ranges (§9.7.6.2) and inverting that is a
    /// different question. The distinction is public so a caller can report *which* of the two
    /// refusals it hit — "this font lacks that character" and "this font cannot be addressed by
    /// character" are not the same statement.
    #[must_use]
    pub const fn addresses_characters(&self) -> bool {
        matches!(self.mapping, CodeMapping::Named(_))
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
    ///
    /// `None` for a composite font. A `CMap`'s codespace ranges decide how many bytes a code
    /// occupies (§9.7.6.2), and inverting the code-to-CID mapping is a different question from
    /// inverting a 256-entry table; a caller reports the refusal rather than guessing a length.
    #[must_use]
    pub fn code_for(&self, character: char) -> Option<Code> {
        let CodeMapping::Named(_) = &self.mapping else {
            return None;
        };
        self.codes_by_character
            .get_or_init(|| {
                let mut map = BTreeMap::new();
                let mut meaning = String::new();
                for byte in 0..=u8::MAX {
                    let code = Code::single_byte(byte);
                    if self.glyph_for(code).is_none() {
                        continue;
                    }
                    meaning.clear();
                    if !self.text(code, &mut meaning) {
                        continue;
                    }
                    let mut characters = meaning.chars();
                    // A code standing for more than one character — an `ffi` ligature's
                    // `/ToUnicode` entry — cannot answer "which code draws this character",
                    // because using it would draw the other characters too.
                    if let (Some(single), None) = (characters.next(), characters.next()) {
                        map.entry(single).or_insert(byte);
                    }
                }
                map
            })
            .get(&character)
            .copied()
            .map(Code::single_byte)
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

    /// Extracts and normalises one glyph outline.
    fn build_outline(&self, glyph: u16) -> Option<Arc<Path>> {
        let mut pen = PathPen {
            path: Path::new(),
            scale: 1.0 / self.units_per_em,
            last: None,
        };

        match self.program {
            Program::BareCff => cff::draw(&self.data, glyph, &mut pen).ok()?,
            Program::Type1 => self.type1.as_ref()?.draw(glyph, &mut pen).ok()?,
            Program::Sfnt => {
                let font = FontRef::new(&self.data).ok()?;
                let outline = font.outline_glyphs().get(GlyphId::from(glyph))?;
                // Unhinted and unscaled: hinting is a device-resolution decision, and this
                // outline is resolution-independent because the text matrix scales it
                // later.
                outline
                    .draw(
                        DrawSettings::unhinted(Size::unscaled(), LocationRef::default()),
                        &mut pen,
                    )
                    .ok()?;
            }
        }

        (!pen.path.is_empty()).then(|| Arc::new(pen.path))
    }
}

/// Narrows a PDF number to `f32`.
fn narrow(value: f64) -> f32 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a glyph advance outside f32's range is not an advance"
    )]
    {
        value as f32
    }
}

/// Resolves a Type 0 font's `/Encoding` to the `CMap` that decodes its strings.
///
/// ISO 32000-2 §9.7.6.1 Table 119 gives the two forms:
///
/// > The name of a predefined `CMap`, or a stream containing a `CMap` that maps character codes
/// > to font numbers and CIDs.
///
/// Of the predefined names, Table 116's two Identity `CMap`s are the only ones this tree can
/// build, because the other seventy-odd *are* data — the registered `CMap` files — and
/// vendoring them is a licensing decision rather than a coding one. Guessing at one would map
/// codes to the wrong glyphs, which is plausible-looking wrong text and the worst kind of
/// rendering error, so a font naming one is refused and reported.
///
/// Vertical writing is refused whichever way it arrives, because §9.7.5.1 makes it a property
/// of the `CMap` and it decides the metrics:
///
/// > A `CMap` shall specify the writing mode … for any `CIDFont` with which the `CMap` is
/// > combined. The writing mode determines which metrics shall be used when glyphs are painted
/// > from that font.
///
/// §9.2.4 and §9.7.4.3 give those metrics as `/W2` and `/DW2`, which nothing here reads, and a
/// vertical run drawn horizontally is not a near miss: `vertical.pdf` should set two columns
/// down the right edge of a page and came out as one overlapping line across the top,
/// reporting nothing.
fn composite_cmap(document: &Document, dict: &Dictionary, name: &str) -> Result<CMap, FontError> {
    let unsupported = |encoding: &str| FontError::UnsupportedEncoding {
        name: name.to_owned(),
        encoding: encoding.to_owned(),
    };

    let encoding = document.get_key(dict, "Encoding");
    let cmap = match &encoding {
        Object::Name(named) => match named.as_bytes() {
            b"Identity-H" => return Ok(CMap::identity()),
            // §9.7.5.2: the two Identity `CMap`s differ only in their writing mode, and
            // the vertical one carries `/WMode 1`, which `CMap::identity_vertical` states.
            b"Identity-V" => return Ok(CMap::identity_vertical()),
            other => return Err(unsupported(&String::from_utf8_lossy(other))),
        },
        Object::Stream(_) => read_cmap(document, &encoding, name, 0)?,
        _ => return Err(unsupported("no /Encoding naming a CMap")),
    };

    // Table 118's `/WMode` and the file's own must agree — "The value of this entry shall be
    // the same as the value of WMode in the CMap file" — so a font is refused if *either*
    // asks for vertical writing rather than only the one a reader happens to consult.
    let dictionary_wmode = encoding
        .as_stream()
        .map(|stream| document.get_key(&stream.dict, "WMode"))
        .and_then(|value| value.as_integer())
        .unwrap_or(0);
    if i64::from(cmap.wmode()) != dictionary_wmode.clamp(0, 1) {
        return Err(unsupported(
            "a CMap whose /WMode disagrees with the one in its own stream",
        ));
    }

    // Without codespace ranges no code can be extracted at all (§9.7.6.2), and without
    // character mappings every code is `.notdef` (§9.7.6.3) — a page of empty boxes drawn in
    // silence. Both are refused rather than approximated, per the rule that unsupported input
    // stays loud. Every one of the corpus's fourteen embedded `CMap`s states both.
    if !cmap.has_codespace() {
        return Err(unsupported("a CMap stream with no codespace ranges"));
    }
    if !cmap.has_mappings() {
        return Err(unsupported("a CMap stream with no character mappings"));
    }
    Ok(cmap)
}

/// Reads one embedded `CMap` stream and the chain of `/UseCMap`s beneath it (§9.7.5.3).
///
/// Table 118 states what it is:
///
/// > The name of a predefined `CMap`, or a stream containing a `CMap`. If this entry is
/// > present, the referencing `CMap` shall specify only the character mappings that differ from
/// > the referenced `CMap`.
///
/// A stream is read and built upon; a *name* is a predefined `CMap`, whose data this tree does
/// not have, so the referencing map's mappings would be missing an unknown share of their
/// codes — refused rather than half-applied.
///
/// §9.7.5.4 a) requires a `usecmap` operator inside the file to be named by `/UseCMap` as well:
///
/// > If the embedded `CMap` file contains a `usecmap` reference, the `CMap` indicated there
/// > shall also be identified by the `UseCMap` entry in the `CMap` stream dictionary.
///
/// A file that breaks that rule is refused, because what it inherits cannot be found.
///
/// No corpus document exercises any of this — none of the fourteen embedded `CMap`s references
/// another — so the tests are synthetic, which is trap 8's advice rather than an accident.
fn read_cmap(
    document: &Document,
    object: &Object,
    name: &str,
    depth: u32,
) -> Result<CMap, FontError> {
    /// Bounds the `/UseCMap` chain, which a document could otherwise make cyclic.
    const MAX_DEPTH: u32 = 4;

    let unsupported = |encoding: &str| FontError::UnsupportedEncoding {
        name: name.to_owned(),
        encoding: encoding.to_owned(),
    };

    if depth > MAX_DEPTH {
        return Err(unsupported("a /UseCMap chain deeper than four"));
    }
    let stream = object
        .as_stream()
        .ok_or_else(|| unsupported("an /Encoding that is neither a name nor a stream"))?;
    let data = document
        .decoded_stream_data(stream)
        .ok_or_else(|| FontError::Malformed {
            name: name.to_owned(),
            detail: "the CMap stream could not be decoded".to_owned(),
        })?;

    let used = match document.get_key(&stream.dict, "UseCMap") {
        Object::Null => None,
        Object::Name(named) if named.as_bytes() == b"Identity-H" => Some(CMap::identity()),
        Object::Name(named) => {
            return Err(unsupported(&format!(
                "a CMap built on the predefined {}",
                String::from_utf8_lossy(named.as_bytes())
            )));
        }
        referenced => Some(read_cmap(
            document,
            &referenced,
            name,
            depth.saturating_add(1),
        )?),
    };

    let cmap = CMap::parse(&data, used.as_ref());
    if cmap.references_another() && used.is_none() {
        return Err(unsupported(
            "a CMap whose usecmap reference is not named by /UseCMap (§9.7.5.4 a))",
        ));
    }
    Ok(cmap)
}

/// Resolves how a `CIDFont`'s CIDs reach glyph indices (§9.7.4.2).
///
/// Three routes, in the order the clauses put them.
///
/// **A CID-keyed CFF's charset first**, because §9.7.4.2 states that route outright and does
/// so for the program rather than for the `/Subtype` name — a `CIDFontType0` may arrive as a
/// CFF wrapped in an `OpenType` container, and the clause's two CFF cases are about what the
/// Top DICT contains.
///
/// **Then the dictionary's own `/CIDToGIDMap` stream**, whatever the `CIDFont`'s subtype.
/// Table 115 conditions the entry's *presence* on Type 2 — "Required for Type 2 `CIDFonts` with
/// embedded font programs" — and defines its meaning unconditionally: "A specification of the
/// mapping from CIDs to glyph indices." §9.7.4.2's other CFF sentence, that a program whose
/// Top DICT has no `CIDFont` operators uses "the CIDs … directly as GID values", describes what
/// such a program offers on its own; it cannot outrank a mapping the font dictionary states,
/// because that would make the stated mapping mean nothing.
///
/// **The identity last**, which is what remains when neither the program nor the dictionary
/// says otherwise.
///
/// The one corpus font that settles the middle rule is `issue7901.pdf`'s: a `CIDFontType0`
/// whose `/FontFile3` is an `OpenType` wrapper around a *name*-keyed CFF, carrying a
/// `/CIDToGIDMap` stream of 230 entries. Ignoring the stream there — the first reading of
/// Table 115 written here — drew "üãÍ†Ë œÍ†ÿ¨ Ì{«" where four renderers draw "The Free Software
/// Definition", because the producer's CIDs are not that CFF's glyph indices and the stream is
/// the only thing in the file that says what they are.
fn cid_to_glyph(
    document: &Document,
    descendant: &Dictionary,
    data: &[u8],
    program: Program,
    name: &str,
) -> Result<CidToGlyph, FontError> {
    // A CFF, bare or wrapped. `read` reports whether the Top DICT uses CIDFont operators,
    // which is precisely the distinction §9.7.4.2 draws between its two CFF cases.
    let cff = match program {
        Program::BareCff => Some(Cow::Borrowed(data)),
        Program::Sfnt => FontRef::new(data).ok().and_then(|font| {
            font.table_data(skrifa::Tag::new(b"CFF "))
                .map(|table| Cow::Owned(table.as_bytes().to_vec()))
        }),
        // §9.9's Table 124 gives a CIDFont two programs, `/FontFile2` and `/FontFile3`, and
        // a bare Type 1 is neither — so nothing is read out of it *here*, and the
        // `/CIDToGIDMap` route below decides, which for a CIDFontType0 is the identity.
        //
        // That identity is the clause's own answer rather than a guess, by two sentences
        // read together. §9.7.4.2, on a CFF whose Top DICT does not use CIDFont operators:
        //
        // > The CIDs shall be used directly as GID values, and the glyph procedure shall be
        // > retrieved using the CharStrings INDEX
        //
        // and §9.6.2.1's NOTE 1, on what a CFF *is*:
        //
        // > an alternative, more compact but functionally equivalent representation of a
        // > Type 1 font program
        //
        // A bare Type 1 program is a name-keyed program whose charstrings are in an order,
        // exactly as a non-CID-keyed CFF's are, so "used directly as GID values" transfers
        // without inventing anything. `issue11740_reduced.pdf` is the witness, and before
        // this it was substituted for and drawn through its own `/ToUnicode` — which
        // records the Windows-1251 *bytes* of its Russian text as Latin-1 code points, so
        // the page came out as mojibake with nothing reported.
        Program::Type1 => None,
    };
    if let Some(cff) = cff {
        // A bare CFF has already been read once, to decide it was one; a wrapped one is read
        // here for the first time. Either way a program this crate cannot parse is a font it
        // cannot draw, so the error is propagated rather than falling through to the identity.
        let read = CodeToGlyph::read(&cff).map_err(|e| FontError::Malformed {
            name: name.to_owned(),
            detail: e.to_string(),
        })?;
        if let CodeToGlyph::Keyed { by_cid } = read {
            return Ok(CidToGlyph::Charset(by_cid));
        }
    }

    Ok(match document.get_key(descendant, "CIDToGIDMap") {
        // Absent, or `Identity`: the CID is the glyph index. A producer that omits the entry
        // has said nothing to prefer over the identity — which is also what §9.7.5.2 describes
        // for `Identity-H` with an embedded `TrueType` program.
        Object::Null => CidToGlyph::Identity,
        Object::Name(map) if map == "Identity" => CidToGlyph::Identity,
        Object::Name(other) => {
            return Err(FontError::UnsupportedEncoding {
                name: name.to_owned(),
                encoding: format!(
                    "/CIDToGIDMap /{}, which Table 115 says shall be Identity",
                    String::from_utf8_lossy(other.as_bytes())
                ),
            });
        }
        stream => {
            /// A glyph index is 16 bits, so no CID above 65 535 can have one and a map longer
            /// than two bytes per such CID describes nothing.
            const MAX_MAP: usize = 2 * (1 << 16);

            let stream = stream.as_stream().ok_or_else(|| FontError::Malformed {
                name: name.to_owned(),
                detail: "/CIDToGIDMap is neither a name nor a stream".to_owned(),
            })?;
            let bytes =
                document
                    .decoded_stream_data(stream)
                    .ok_or_else(|| FontError::Malformed {
                        name: name.to_owned(),
                        detail: "the /CIDToGIDMap stream could not be decoded".to_owned(),
                    })?;
            let kept = bytes.get(..bytes.len().min(MAX_MAP)).unwrap_or(&bytes);
            CidToGlyph::Stream(Arc::from(kept))
        }
    })
}

/// Collects `/W` widths for a composite font.
///
/// The array mixes two forms: `c [w1 w2 ...]` gives consecutive codes, and `c1 c2 w` gives
/// one width for a whole range.
///
/// ISO 32000-2 §9.7.4.3 on a CID that appears twice: "specifying a given CID value more than
/// once should not be done. In the case where it is done, the first specification is the one
/// that shall be used." So an entry that already exists is kept, rather than overwritten.
fn composite_widths(document: &Document, descendant: &Dictionary) -> BTreeMap<u32, f32> {
    /// Ranges are bounded so a hostile `/W` cannot ask for four billion entries.
    const MAX_RANGE: u32 = 1 << 16;

    let mut widths = BTreeMap::new();
    let array = document.get_key(descendant, "W");
    let Some(items) = array.as_array() else {
        return widths;
    };

    let resolved: Vec<Object> = items.iter().map(|item| document.resolve(item)).collect();
    let mut index = 0usize;

    while index < resolved.len() {
        let Some(first) = resolved.get(index).and_then(Object::as_integer) else {
            break;
        };
        let Ok(first) = u32::try_from(first) else {
            break;
        };

        match resolved.get(index.saturating_add(1)) {
            Some(Object::Array(list)) => {
                for (offset, item) in list.iter().enumerate() {
                    let Some(width) = document.resolve(item).as_number() else {
                        continue;
                    };
                    let Ok(offset) = u32::try_from(offset) else {
                        continue;
                    };
                    widths
                        .entry(first.saturating_add(offset))
                        .or_insert(narrow(width));
                }
                index = index.saturating_add(2);
            }
            Some(second) => {
                let Some(last) = second
                    .as_integer()
                    .and_then(|value| u32::try_from(value).ok())
                else {
                    break;
                };
                let Some(width) = resolved
                    .get(index.saturating_add(2))
                    .and_then(Object::as_number)
                else {
                    break;
                };
                let span = last.saturating_sub(first).min(MAX_RANGE);
                for offset in 0..=span {
                    widths
                        .entry(first.saturating_add(offset))
                        .or_insert(narrow(width));
                }
                index = index.saturating_add(3);
            }
            None => break,
        }
    }

    widths
}

/// The glyph name each character code selects, according to the PDF encoding alone.
///
/// This is the half of the mapping the *document* determines, shared by every route to a
/// glyph: a bare CFF resolves these names through its charset, a substitute resolves them
/// through the Adobe Glyph List, and text extraction falls back to them when a font has no
/// `/ToUnicode`. An empty name means the code is unencoded and the font's own encoding
/// applies.
fn encoding_names(
    document: &Document,
    dict: &Dictionary,
    name: &str,
    symbolic_font: Option<encoding::SymbolicEncoding>,
    fall_back_to_standard: bool,
) -> Result<GlyphNames, FontError> {
    let mut names = no_names();

    if let Some(symbolic) = symbolic_font {
        // The two symbolic standard-14 fonts have their own encoding and no Latin base.
        for (code, slot) in names.iter_mut().enumerate() {
            if let Ok(code) = u8::try_from(code) {
                *slot = Cow::Borrowed(symbolic.glyph_name(code));
            }
        }
    } else {
        let base = base_encoding(document, dict, name)?
            .or(fall_back_to_standard.then_some(BaseEncoding::Standard));
        if let Some(base) = base {
            for (code, slot) in names.iter_mut().enumerate() {
                if let Ok(code) = u8::try_from(code) {
                    *slot = Cow::Borrowed(base.glyph_name(code));
                }
            }
        }
    }

    apply_differences(document, dict, &mut names);
    Ok(names)
}

/// Reads a font's `/ToUnicode` `CMap`, which is absent more often than not.
fn to_unicode(document: &Document, dict: &Dictionary) -> tounicode::ToUnicode {
    let object = document.get_key(dict, "ToUnicode");
    let Some(stream) = object.as_stream() else {
        return tounicode::ToUnicode::default();
    };
    document
        .decoded_stream_data(stream)
        .map(|bytes| tounicode::ToUnicode::parse(&bytes))
        .unwrap_or_default()
}

/// Resolves a simple font's character codes to glyphs in a *substitute* font.
///
/// The substitute shares nothing with the font the document named except the characters
/// it can draw, so every code is resolved to the character it stands for and that
/// character is looked up in the substitute's `cmap`.
///
/// The character comes from the glyph name the encoding gives — through the Adobe Glyph
/// List — rather than from `/ToUnicode`, because the encoding is what the *rendering*
/// model uses. `/ToUnicode` is a statement about extracted text and producers are known
/// to leave it wrong; a code's glyph name is what actually selected the glyph.
fn substitute_code_table(
    document: &Document,
    dict: &Dictionary,
    descriptor: Option<&Dictionary>,
    request: substitute::Request,
    data: &[u8],
    name: &str,
) -> Result<(CodeTable, GlyphNames), FontError> {
    let _ = descriptor;
    let font = FontRef::new(data).map_err(|e| FontError::Malformed {
        name: name.to_owned(),
        detail: format!("substitute font: {e}"),
    })?;
    let charmap = font.charmap();

    // A symbolic standard-14 font carries its own encoding; everything else starts from a
    // Latin base, defaulting to StandardEncoding when the document names none.
    let symbolic = match request.family {
        substitute::Family::Symbol => Some(encoding::SymbolicEncoding::Symbol),
        substitute::Family::ZapfDingbats => Some(encoding::SymbolicEncoding::ZapfDingbats),
        _ => None,
    };
    let names = encoding_names(document, dict, name, symbolic, true)?;

    let mut table: CodeTable = [None; 256];
    for (code, slot) in table.iter_mut().enumerate() {
        let Some(glyph_name) = names.get(code).map(Cow::as_ref).filter(|n| !n.is_empty()) else {
            continue;
        };
        let Some(character) = read_fonts::ps::agl::name_to_char(glyph_name) else {
            continue;
        };
        *slot = charmap
            .map(character)
            .and_then(|id| u16::try_from(id.to_u32()).ok());
    }

    if !declared_codes(document, dict).any(|code| table.get(code).is_some_and(Option::is_some)) {
        return Err(FontError::NoSubstitute {
            name: name.to_owned(),
            reason: format!(
                "the {:?} face draws none of the codes the document declares",
                request.family
            ),
        });
    }

    Ok((table, names))
}

/// The character codes the font dictionary says the document uses.
///
/// `/FirstChar` and `/LastChar` bound `/Widths`, so between them they are the producer's own
/// statement of which codes appear in the content stream. That is what makes them the right
/// range to judge a mapping by: a substitute that reaches a glyph for two hundred codes the
/// document never shows, and for none of the four it does, is not a usable substitute — and
/// counting *all* the codes it happens to cover says it is.
///
/// `issue20504.pdf` is the case that showed it. Its Chinese line uses a Type 1 program this
/// crate cannot read, so the font is substituted, and `/Differences [33 /gid2436 …]` names
/// four glyphs only the original font had. Every one of the 252 codes the document does not
/// use resolved through `StandardEncoding` and mapped, so the substitute looked usable, and
/// the four codes actually shown drew nothing at all — in silence.
///
/// A dictionary stating neither yields every code, which keeps the judgement no weaker than
/// it was for a font that says nothing about its range.
fn declared_codes(document: &Document, dict: &Dictionary) -> std::ops::RangeInclusive<usize> {
    let bound = |key: &str| {
        document
            .get_key(dict, key)
            .as_integer()
            .and_then(|value| usize::try_from(value).ok())
    };
    match (bound("FirstChar"), bound("LastChar")) {
        (Some(first), Some(last)) if first <= last => first..=last.min(255),
        _ => 0..=255,
    }
}

/// Reads the base encoding a font dictionary names, if it names one.
fn base_encoding(
    document: &Document,
    dict: &Dictionary,
    name: &str,
) -> Result<Option<BaseEncoding>, FontError> {
    let encoding = document.get_key(dict, "Encoding");
    let named = encoding
        .as_name()
        .map(|value| value.as_bytes().to_vec())
        .or_else(|| {
            encoding
                .as_dict()
                .map(|d| document.get_key(d, "BaseEncoding"))
                .and_then(|value| value.as_name().map(|n| n.as_bytes().to_vec()))
        });

    match named {
        None => Ok(None),
        Some(named) => {
            BaseEncoding::by_name(&named)
                .map(Some)
                .ok_or_else(|| FontError::UnsupportedEncoding {
                    name: name.to_owned(),
                    encoding: String::from_utf8_lossy(&named).into_owned(),
                })
        }
    }
}

/// Layers an `/Encoding` dictionary's `/Differences` array over a table of glyph names.
///
/// # A name this crate does not recognise still names the code
///
/// An earlier version kept only names with a `'static` spelling and left the base
/// encoding's name in place for the rest, which meant a code the document had explicitly
/// reassigned silently kept its old meaning. `issue20504.pdf` writes
/// `/Differences [33 /gid2436 …]` — a subsetter's convention for naming a glyph by its
/// index, which §9.6.5 does not define but does not forbid either — and every one of its
/// four codes fell back to `StandardEncoding`, so a page of Chinese was drawn as `!"#$`
/// with nothing reported. §9.6.5.4's "any *undefined* entries in the table shall be filled
/// using `StandardEncoding`" is about codes the encoding never assigned, not about codes it
/// assigned to a name we happen not to know.
///
/// So every name is kept. A recognised one keeps its static spelling and costs no
/// allocation, which is the case for nearly every name a real document writes; a novel one
/// is owned, and reaches the font's own `post` table or CFF charset where it may well be
/// found.
fn apply_differences(document: &Document, dict: &Dictionary, names: &mut GlyphNames) {
    let encoding = document.get_key(dict, "Encoding");
    let Some(encoding) = encoding.as_dict() else {
        return;
    };
    let differences = document.get_key(encoding, "Differences");
    let Some(items) = differences.as_array() else {
        return;
    };

    let mut code: Option<usize> = None;
    for item in items {
        match document.resolve(item) {
            Object::Integer(value) => code = usize::try_from(value).ok(),
            Object::Name(glyph_name) => {
                let Some(at) = code else { continue };
                if let Some(slot) = names.get_mut(at) {
                    *slot = glyph_name_of(glyph_name.as_bytes());
                }
                code = at.checked_add(1);
            }
            _ => {}
        }
    }
}

/// Returns a `/Differences` name, borrowing the specifications' spelling where there is one.
///
/// Glyph names are ASCII by specification; a font that breaks that is malformed, not a
/// reason to lose the name, so the owned case is lossy rather than fallible.
fn glyph_name_of(name: &[u8]) -> Cow<'static, str> {
    match interned(name) {
        Some(known) => Cow::Borrowed(known),
        None => Cow::Owned(String::from_utf8_lossy(name).into_owned()),
    }
}

/// Returns the `'static` spelling of a glyph name, if it is one the specifications list.
///
/// Matching one avoids an allocation for the overwhelmingly common case of a name PDF or
/// CFF already defines.
fn interned(name: &[u8]) -> Option<&'static str> {
    let name = std::str::from_utf8(name).ok()?;
    skrifa::raw::ps::string::STANDARD_STRINGS
        .iter()
        .copied()
        .find(|known| *known == name)
        .or_else(|| {
            (0..=u8::MAX).find_map(|code| {
                [
                    BaseEncoding::WinAnsi.glyph_name(code),
                    BaseEncoding::MacRoman.glyph_name(code),
                    encoding::SymbolicEncoding::Symbol.glyph_name(code),
                    encoding::SymbolicEncoding::ZapfDingbats.glyph_name(code),
                ]
                .into_iter()
                .find(|known| *known == name)
            })
        })
}

/// What [`simple_widths`] needs about the font beyond the document's own statements.
#[derive(Clone, Copy)]
struct SimpleMetrics<'a> {
    /// The substitution request, when the font is a stand-in.
    substituted: Option<substitute::Request>,
    /// The glyph name each code selects, when the font has names.
    names: Option<&'a GlyphNames>,
    /// The font program.
    data: &'a [u8],
    mapping: &'a CodeMapping,
    units_per_em: f32,
}

/// Builds a simple font's advance table, in thousandths of an em.
///
/// Three sources, in descending order of authority. `/Widths` is the document's own
/// statement and always wins. Failing that — which only the standard 14 are allowed to do
/// — the specification's published metrics for the face apply, which keeps the layout a
/// property of the document rather than of this machine. Anything still unanswered falls
/// to the substitute's own advances, which in practice means glyphs outside the standard
/// character set.
///
/// Resolved once here rather than at each lookup, so [`LoadedFont::advance`] stays free of
/// work and of allocation on the per-character path.
/// The width of every code the font dictionary's `/Widths` does not cover.
///
/// §9.6.2's Table 109 names `/MissingWidth` for exactly that, and Table 120 gives it a
/// default; see [`DEFAULT_WIDTH`], which is the whole subject.
fn missing_width(document: &Document, descriptor: Option<&Dictionary>) -> f32 {
    descriptor
        .map(|descriptor| document.get_key(descriptor, "MissingWidth"))
        .and_then(|value| value.as_number())
        .map_or(DEFAULT_WIDTH, narrow)
}

fn simple_widths(
    document: &Document,
    dict: &Dictionary,
    font: SimpleMetrics<'_>,
) -> BTreeMap<u32, f32> {
    // `/Widths` is indexed from `/FirstChar`.
    let first = document
        .get_key(dict, "FirstChar")
        .as_integer()
        .unwrap_or(0);
    let mut widths = BTreeMap::new();
    if let Some(items) = document.get_key(dict, "Widths").as_array() {
        for (offset, item) in items.iter().enumerate() {
            let Some(width) = document.resolve(item).as_number() else {
                continue;
            };
            let Ok(offset) = i64::try_from(offset) else {
                continue;
            };
            let Ok(code) = u32::try_from(first.saturating_add(offset)) else {
                continue;
            };
            widths.insert(code, narrow(width));
        }
    }

    let Some(request) = font.substituted.filter(|_| widths.is_empty()) else {
        return widths;
    };

    let standard = standard_metrics::StandardFont::for_request(request);
    if let Some(names) = font.names {
        for (code, glyph_name) in names.iter().enumerate() {
            let Ok(code) = u32::try_from(code) else {
                continue;
            };
            if let Some(width) = standard.width(glyph_name.as_ref()) {
                widths.insert(code, width);
            }
        }
    }
    for (code, width) in program_widths(font.data, font.mapping, font.units_per_em) {
        widths.entry(code).or_insert(width);
    }
    widths
}

/// Fills a width table from the font program's own advances.
///
/// Used only when the document states no widths, which the standard 14 are allowed to do.
fn program_widths(data: &[u8], mapping: &CodeMapping, units_per_em: f32) -> BTreeMap<u32, f32> {
    let mut widths = BTreeMap::new();
    let CodeMapping::Named(table) = mapping else {
        return widths;
    };
    let Ok(font) = FontRef::new(data) else {
        return widths;
    };
    let metrics = font.glyph_metrics(Size::unscaled(), LocationRef::default());

    for (code, glyph) in table.iter().enumerate() {
        let Some(glyph) = *glyph else { continue };
        let Ok(code) = u32::try_from(code) else {
            continue;
        };
        let Some(advance) = metrics.advance_width(GlyphId::from(glyph)) else {
            continue;
        };
        widths.insert(code, advance / units_per_em * 1000.0);
    }
    widths
}

/// Resolves a simple font's character codes to glyphs in a bare CFF program.
///
/// This is the specification's encoding algorithm for a font whose glyphs are keyed by
/// name (ISO 32000-2 §9.6.5.2): choose a base encoding, layer `/Differences` over it, and
/// resolve the resulting glyph names through the font's charset.
///
/// # The base encoding of an embedded program is the program's own
///
/// §9.6.5.1's Table 112 states the default in three cases, and only the last two turn on
/// the Symbolic flag:
///
/// > For a font program that is embedded in the PDF file, the default base encoding shall
/// > be the font program's built-in encoding, as described in 9.6.5, "Character encoding"
/// > and further elaborated in the subclauses on specific font types.
///
/// A bare CFF only reaches this function by having been embedded, so the first sentence
/// decides every font that gets here and the flag decides none of them. An earlier version
/// asked the flag anyway and gave a nonsymbolic font `StandardEncoding`, which is the rule
/// for a font this crate would be *substituting* — so a code the document left to the
/// program drew whatever `StandardEncoding` puts there instead of what the program does.
/// It is invisible on almost every document, because a CFF's built-in encoding usually
/// *is* `StandardEncoding`; over the whole corpus it moves one code of one font.
///
/// # Why an unresolved name is not retried against the font's own encoding
///
/// When the encoding names a glyph the font does not have, this leaves the code with no
/// glyph rather than falling back to whatever the font's built-in encoding puts there.
/// The fallback is tempting because it fills the page, and wrong because a subset font's
/// built-in encoding is arbitrary: it would draw *a* glyph, confidently, and not the one
/// the document asked for. A blank is a visible defect; a wrong letter is an invisible
/// one. That is not in tension with the rule above: the built-in encoding is the base
/// *before* `/Differences` renames a code, never a second chance after a name failed.
fn simple_code_table(
    document: &Document,
    dict: &Dictionary,
    program: &NameKeyed,
    name: &str,
) -> Result<(CodeTable, GlyphNames), FontError> {
    let NameKeyed {
        by_name,
        builtin,
        builtin_names,
    } = program;

    // No fall-back to StandardEncoding: an unnamed code belongs to the program's own
    // encoding, which is what Table 112 makes the base here.
    let mut names = encoding_names(document, dict, name, None, false)?;

    let mut table: CodeTable = [None; 256];
    for (code, slot) in table.iter_mut().enumerate() {
        match names.get(code).map(Cow::as_ref).filter(|n| !n.is_empty()) {
            // The encoding named a glyph. If the font does not have it, the code has no
            // glyph, and that is final — see the note above this function.
            Some(glyph_name) => *slot = by_name.get(glyph_name).copied(),
            // The encoding said nothing, so the font's own encoding applies.
            None => *slot = builtin.get(code).copied().flatten(),
        }
    }

    // Every code the base encoding answered is now drawn by the right glyph and named by
    // nothing, and a font with no `/ToUnicode` has only the name to say what a code means.
    // Taken from the charset rather than from a Latin table, so the name is the program's
    // own statement about the glyph the code just selected.
    for (code, slot) in names.iter_mut().enumerate() {
        if let Some(builtin_name) = builtin_names
            .get(code)
            .and_then(Option::as_deref)
            .filter(|_| slot.is_empty())
        {
            *slot = Cow::Owned(builtin_name.to_owned());
        }
    }

    if table.iter().all(Option::is_none) {
        // The font loaded and the encoding resolved, and between them they addressed not
        // one glyph. Reporting beats rendering an entirely blank page in silence.
        return Err(FontError::Malformed {
            name: name.to_owned(),
            detail: "no character code maps to a glyph".to_owned(),
        });
    }

    Ok((table, names))
}

/// The three `cmap` subtables ISO 32000-2 §9.6.5.4 distinguishes.
///
/// It names them by their platform and encoding IDs, and the whole of its algorithm turns
/// on which of them a font carries — so they are found once, by ID, rather than through a
/// "best subtable" heuristic. `skrifa`'s own `Charmap` picks the most comprehensive
/// *Unicode* subtable, which is the right choice for laying out text and the wrong one
/// here: a (1, 0) Macintosh subtable is not a Unicode mapping, so `Charmap` does not
/// consider it at all, and a font whose only subtable is that one maps nothing.
///
/// That is not a corner case. `issue20504.pdf` embeds four `TrueType` subsets and every one
/// of them carries a single (1, 0) format 6 subtable — which is exactly what §9.6.5.4's
/// third guideline tells a producer to emit.
struct Subtables<'a> {
    /// (3, 0), Microsoft Symbol: codes are looked up as they are written.
    symbol: Option<CmapSubtable<'a>>,
    /// (3, 1), Microsoft Unicode: codes reach it as characters, through glyph names.
    unicode: Option<CmapSubtable<'a>>,
    /// (1, 0), Macintosh Roman: codes reach it as Mac OS Roman codes.
    macintosh: Option<CmapSubtable<'a>>,
    /// The whole table, for the last-resort mapping that asks every subtable in turn.
    ///
    /// A font may carry a subtable §9.6.5.4 names none of — `issue5501.pdf`'s only one is
    /// (0, 0), Unicode 1.0 — and there is nowhere else left to ask by the time that
    /// matters.
    all: Option<Cmap<'a>>,
}

impl<'a> Subtables<'a> {
    fn read(font: &FontRef<'a>) -> Self {
        let mut found = Self {
            symbol: None,
            unicode: None,
            macintosh: None,
            all: None,
        };
        let Ok(cmap) = font.cmap() else {
            return found;
        };
        found.all = Some(cmap.clone());
        for record in cmap.encoding_records() {
            let Ok(subtable) = record.subtable(cmap.offset_data()) else {
                continue;
            };
            // The first subtable of each kind wins; a font listing two is malformed, and
            // taking the earlier one at least makes the choice reproducible.
            let slot = match (record.platform_id(), record.encoding_id()) {
                (PlatformId::Windows, 0) => &mut found.symbol,
                (PlatformId::Windows, 1) => &mut found.unicode,
                (PlatformId::Macintosh, 0) => &mut found.macintosh,
                _ => continue,
            };
            if slot.is_none() {
                *slot = Some(subtable);
            }
        }
        found
    }
}

/// Resolves a simple font's character codes to glyphs in a `TrueType` or `OpenType` program.
///
/// This is ISO 32000-2 §9.6.5.4, whose shape is easy to lose: a `cmap` is *not* indexed by
/// character code. Each of its subtables is indexed by something else — a Unicode
/// character, a Mac OS Roman code, a two-byte symbol code — and the subclause is a set of
/// rules for turning a PDF character code into whichever of those the font happens to
/// carry. Handing the code straight to the font's Unicode subtable is right only by
/// coincidence, for ASCII, in a font that has one.
///
/// The rules, in the order the subclause gives them:
///
/// - **The font's own codes.** When the font descriptor sets the symbolic flag, or the
///   dictionary has no `/Encoding` at all, the PDF encoding says nothing: a (3, 0) subtable
///   is addressed by the code with the high byte of its range prepended, and failing that a
///   (1, 0) subtable is addressed by the single byte.
/// - **Through a glyph name.** Otherwise the code selects a glyph *name* — from the base
///   encoding, updated by `/Differences`, with anything still undefined filled from
///   `StandardEncoding` — and the name is carried to a (3, 1) subtable through the Adobe
///   Glyph List, or to a (1, 0) subtable through Mac OS Roman.
/// - **The `post` table.** "In any of these cases, if the glyph name cannot be mapped as
///   specified, the glyph name shall be looked up in the font program's `post` table."
///   This is what reaches a subsetter's `/gid2436`, which no encoding and no character set
///   knows but the font itself may name.
///
/// # The two tiers below the specification's own, and why they are last
///
/// §9.6.5.4 closes with "if a character cannot be mapped in any of the ways described
/// previously, a PDF processor may supply a mapping of its choosing". Two are supplied, and
/// each is narrower than the code it replaced.
///
/// The first offers the code to every subtable the font has, in the font's own order,
/// which is what this crate did for every font before the algorithm above existed. It still
/// earns its place twice over: a symbolic font carrying only a (3, 1) subtable — common,
/// and contrary to the guidelines — reaches no rule above and its codes really are ASCII;
/// and `issue5501.pdf`'s subset carries its byte-to-glyph map in a (0, 0) subtable, which
/// §9.6.5.4 does not mention at all and which is nonetheless the only correct answer for
/// that font.
///
/// The second treats the code as a glyph index, and applies **only to a font with no
/// readable `cmap` at all**. That restriction is the point: the old code fell through to it
/// per *code*, so a font with a perfectly good `cmap` that simply did not cover a code drew
/// glyph number `code` instead — a wrong glyph, confidently, in place of nothing. A
/// document using a simple font is required to embed a `cmap` (§9.9.1), so a program
/// without one is malformed, and a subset really is often ordered by code.
fn truetype_code_table(
    document: &Document,
    dict: &Dictionary,
    descriptor: Option<&Dictionary>,
    data: &[u8],
    name: &str,
) -> Result<(CodeTable, GlyphNames), FontError> {
    let font = FontRef::new(data).map_err(|e| FontError::Malformed {
        name: name.to_owned(),
        detail: e.to_string(),
    })?;
    let subtables = Subtables::read(&font);
    // §9.6.5.4's last resort is "the font program's `post` table (if one is present)" — and a
    // CFF-based OpenType keeps its glyph names somewhere else. `issue215.pdf` embeds an `OTTO`
    // whose `post` is version 3.0, which by definition holds no names at all, while its `CFF `
    // charset names every one of them; §9.6.2.1's NOTE 1 is the sentence that makes those the
    // same structure, so the charset is read here as what the clause asks for. Built once per
    // font rather than per code, because it inverts the whole charset.
    let charset = font
        .table_data(skrifa::Tag::new(b"CFF "))
        .and_then(|table| CodeToGlyph::read(table.as_bytes()).ok())
        .and_then(|read| match read {
            CodeToGlyph::Named(named) => Some(named),
            CodeToGlyph::Keyed { .. } => None,
        });
    let symbolic = descriptor.is_some_and(|d| is_symbolic(document, d));
    // §9.6.5.4 fills undefined entries from StandardEncoding, which `encoding_names` does
    // by starting there — but only for a font whose names are Latin at all. A symbolic
    // font's are not, and it takes the first route below rather than this one.
    let names = match encoding_names(document, dict, name, None, !symbolic) {
        Ok(names) => names,
        // §9.6.5.4 again: when the symbolic flag is set the `/Encoding` entry "is ignored".
        // So an entry naming an encoding this crate has no table for — `issue5701.pdf`
        // writes `/Encoding /Identity-H` on a simple `TrueType` font, which is not a base
        // encoding at all — is not a font this crate cannot read. It is an entry the
        // specification tells us not to read.
        Err(FontError::UnsupportedEncoding { .. }) if symbolic => no_names(),
        Err(other) => return Err(other),
    };

    let mut table: CodeTable = [None; 256];

    // "When the font has no Encoding entry, or the font descriptor's Symbolic flag is set
    // (in which case the Encoding entry is ignored)".
    let unencoded = matches!(document.get_key(dict, "Encoding"), Object::Null);
    if symbolic || unencoded {
        for (code, slot) in table.iter_mut().enumerate() {
            let Ok(code) = u32::try_from(code) else {
                continue;
            };
            *slot = symbol_glyph(&subtables, code);
        }
    }

    for (code, slot) in table.iter_mut().enumerate() {
        if slot.is_some() {
            continue;
        }
        let glyph_name = names.get(code).map(Cow::as_ref).filter(|n| !n.is_empty());
        *slot = glyph_name
            .and_then(|glyph_name| named_glyph(&font, &subtables, charset.as_ref(), glyph_name));
    }

    // The two tiers the specification leaves to the processor; see the note above.
    for (code, slot) in table.iter_mut().enumerate() {
        if slot.is_some() {
            continue;
        }
        let Ok(code) = u32::try_from(code) else {
            continue;
        };
        *slot = as_character(&subtables, code).or_else(|| {
            subtables
                .all
                .is_none()
                .then(|| u16::try_from(code).ok())
                .flatten()
        });
    }

    if table.iter().all(Option::is_none) {
        // The font loaded and the encoding resolved, and between them they addressed not
        // one glyph. Reporting beats rendering an entirely blank page in silence.
        return Err(FontError::Malformed {
            name: name.to_owned(),
            detail: "no character code maps to a glyph".to_owned(),
        });
    }

    Ok((table, names))
}

/// Every glyph a font's Unicode subtable names, keyed by glyph.
///
/// The first character wins where several map to one glyph, which happens in every subset that
/// unifies a letter with its presentation form. Deterministic because `Charmap::mappings`
/// walks the subtable in code order.
fn invert_charmap(font: &FontRef<'_>) -> BTreeMap<u16, char> {
    let mut out = BTreeMap::new();
    for (code, glyph) in font.charmap().mappings() {
        let (Some(character), Ok(glyph)) = (char::from_u32(code), u16::try_from(glyph.to_u32()))
        else {
            continue;
        };
        out.entry(glyph).or_insert(character);
    }
    out
}

/// A code's glyph through the font's own codes: the (3, 0) subtable, then the (1, 0) one.
///
/// §9.6.5.4 says a (3, 0) subtable's codes lie in one of four ranges — `0x0000`, `0xF000`,
/// `0xF100` or `0xF200` — and that each byte from the string is prepended with the high
/// byte of *the* range the font uses. Which range that is is not recorded anywhere in the
/// font, so the four are tried in the order the subclause lists them. A subtable holding
/// two of them at once would be malformed; one holding none maps nothing, which is the
/// answer either way.
fn symbol_glyph(subtables: &Subtables<'_>, code: u32) -> Option<u16> {
    if let Some(symbol) = subtables.symbol.as_ref() {
        return [0x0000, 0xF000, 0xF100, 0xF200]
            .into_iter()
            .find_map(|high: u32| symbol.map_codepoint(high.saturating_add(code)))
            .and_then(narrow_glyph);
    }
    subtables
        .macintosh
        .as_ref()?
        .map_codepoint(code)
        .and_then(narrow_glyph)
}

/// A glyph name's glyph: through the (3, 1) subtable, the (1, 0) subtable, or `post`.
fn named_glyph(
    font: &FontRef<'_>,
    subtables: &Subtables<'_>,
    charset: Option<&NameKeyed>,
    glyph_name: &str,
) -> Option<u16> {
    let through_character = |name: &str| {
        if let Some(unicode) = subtables.unicode.as_ref() {
            read_fonts::ps::agl::name_to_char(name)
                .and_then(|character| unicode.map_codepoint(character))
        } else if let Some(macintosh) = subtables.macintosh.as_ref() {
            encoding::mac_os_roman_code(name)
                .and_then(|code| macintosh.map_codepoint(u32::from(code)))
        } else {
            None
        }
    };

    // §9.6.5.4 sends the glyph name "to a Unicode value by consulting the Adobe Glyph List
    // and Adobe Glyph List for New Fonts", and then:
    //
    // > In any of these cases, if the glyph name cannot be mapped as specified, the glyph
    // > name shall be looked up in the font program's "post" table (if one is present) and
    // > the associated glyph description shall be used.
    //
    // Those are two *lists*, and **no entry in either contains a FULL STOP**. A name like
    // `o.sc` is therefore one the lists do not map, and the sentence above is what applies
    // to it. What `read_fonts::ps::agl::name_to_char` implements is the wider *Adobe Glyph
    // List Specification*, whose algorithm for an unlisted name strips everything after the
    // first period — so it answers `o` for `o.sc`, which is a real letter and hides the
    // clause's own next step behind it.
    //
    // `issue215.pdf` is the witness and it settles the reading three times over: its
    // `/Differences` name eleven small-capital variants, `o.sc` through `n.sc`; its `post`
    // table names all eleven; its `/ToUnicode` maps them to U+F76F and neighbours, the
    // private-use block Adobe assigns to small capitals — so the *producer* says small
    // capitals. We drew `openmagazin` in lower case where four references draw small caps.
    let unlisted = glyph_name.contains('.');
    let listed_route = (!unlisted).then(|| through_character(glyph_name)).flatten();

    listed_route
        .and_then(narrow_glyph)
        .or_else(|| post_glyph(font, glyph_name))
        .or_else(|| charset.and_then(|charset| charset.by_name.get(glyph_name).copied()))
        // Last, and only for a name the lists do not hold: the specification's algorithmic
        // form. A font with no `post` entry for `o.sc` states nothing better than "an o",
        // and drawing one beats drawing nothing — but it is a recovery rather than the
        // clause's route, which is why it sits below `post` rather than above it.
        .or_else(|| {
            unlisted
                .then(|| through_character(glyph_name))
                .flatten()
                .and_then(narrow_glyph)
        })
}

/// A glyph name's glyph, from the font program's own `post` table.
///
/// Searched rather than indexed: `post` maps a glyph to its name, and this needs the
/// inverse. A simple font has at most 256 codes and this runs once per font at load time,
/// so the linear scan costs less than the map it would otherwise build — and the names it
/// is asked for are usually the ones no other route knew, so the scan usually runs to the
/// end and finds nothing.
fn post_glyph(font: &FontRef<'_>, glyph_name: &str) -> Option<u16> {
    let post = font.post().ok()?;
    (0..u16::try_from(post.num_names()).unwrap_or(u16::MAX)).find(|glyph| {
        post.glyph_name(skrifa::raw::types::GlyphId16::new(*glyph)) == Some(glyph_name)
    })
}

/// A code's glyph by treating the code itself as a character, in any subtable the font has.
///
/// The mapping of this processor's choosing, for a font that reaches none of §9.6.5.4's
/// rules. `Cmap::map_codepoint` asks every subtable in the order the font lists them, which
/// is what reaches the ones the subclause does not name. The private-use variant is where
/// symbolic `TrueType` fonts conventionally put their glyphs.
fn as_character(subtables: &Subtables<'_>, code: u32) -> Option<u16> {
    let cmap = subtables.all.as_ref()?;
    cmap.map_codepoint(code)
        .or_else(|| cmap.map_codepoint(0xF000_u32.saturating_add(code)))
        .and_then(narrow_glyph)
}

/// Narrows a glyph identifier to the `u16` a simple font's tables hold.
///
/// A glyph index beyond `u16` cannot appear in a `TrueType` font — `maxp` states the count
/// as a `u16` — so this discards nothing a well-formed font can produce.
fn narrow_glyph(glyph: GlyphId) -> Option<u16> {
    u16::try_from(glyph.to_u32()).ok()
}

/// Returns whether a font descriptor sets the symbolic flag.
///
/// Bit 3 of `/Flags`, counting from one. A symbolic font's character set is outside the
/// standard Latin set, so the encoding built into the font program describes it and a
/// Latin base encoding does not.
fn is_symbolic(document: &Document, descriptor: &Dictionary) -> bool {
    /// Bit 3, counting from one as the specification does.
    const SYMBOLIC: i64 = 1 << 2;

    document
        .get_key(descriptor, "Flags")
        .as_integer()
        .is_some_and(|flags| flags & SYMBOLIC != 0)
}

/// An embedded font program and the reader that understands it.
struct Embedded {
    data: Arc<[u8]>,
    program: Program,
}

/// Extracts the embedded font program from a font descriptor.
fn embedded_program(
    document: &Document,
    descriptor: &Dictionary,
    name: &str,
) -> Result<Embedded, FontError> {
    // `/FontFile2` is TrueType and `/FontFile3` is CFF or OpenType, both of which skrifa
    // reads. `/FontFile` is bare Type1, which it does not.
    for key in ["FontFile2", "FontFile3"] {
        let object = document.get_key(descriptor, key);
        let Some(stream) = object.as_stream() else {
            continue;
        };
        let Some(data) = document.decoded_stream_data(stream) else {
            return Err(FontError::Malformed {
                name: name.to_owned(),
                detail: format!("/{key} did not decode"),
            });
        };

        // `/FontFile3` holds either a full OpenType file or a *bare* CFF font program.
        // Its `/Subtype` says which — `Type1C` and `CIDFontType0C` for a bare CFF — but
        // the leading bytes say the same thing and cannot be mislabelled by a producer,
        // so the signature decides and the `/Subtype` is not consulted at all.
        let program = if is_bare_cff(&data) {
            Program::BareCff
        } else {
            Program::Sfnt
        };

        let data = if program == Program::Sfnt {
            repaired_loca_format(&data).map_or(data, Arc::from)
        } else {
            data
        };

        return Ok(Embedded { data, program });
    }

    // `/FontFile` is a bare Type 1 program. Read last, because Table 120 says "At most,
    // only one of the FontFile , FontFile2 , and FontFile3 entries shall be present" and a
    // file writing two has said nothing about which it means — preferring the formats with
    // a self-identifying signature keeps that choice from turning on the key's spelling.
    if let Some(stream) = document.get_key(descriptor, "FontFile").as_stream() {
        let data = document
            .decoded_stream_data(stream)
            .ok_or_else(|| FontError::Malformed {
                name: name.to_owned(),
                detail: "/FontFile did not decode".to_owned(),
            })?;
        return Ok(Embedded {
            data,
            program: Program::Type1,
        });
    }
    Err(FontError::NotEmbedded {
        name: name.to_owned(),
    })
}

/// Repairs a byte-swapped `indexToLocFormat`, returning the corrected bytes.
///
/// # Why this is a derivation and not a heuristic
///
/// `issue2537r.pdf` embeds a 60-glyph Helvetica-Bold subset whose `head` table states
/// `indexToLocFormat` as **0x0100**. The field is defined by ISO/IEC 14496-22 to be 0 (short
/// offsets) or 1 (long); 0x0100 is 1 written in the wrong byte order and is neither. `skrifa`
/// reads it strictly and reaches the wrong offsets, so the page drew `.notdef` boxes where
/// three references draw `LINE UP` — and reported nothing, because the font loaded and
/// produced *some* glyphs.
///
/// The file says which format it is, twice, in its own table directory, and that is what this
/// function reads rather than guessing:
///
/// - the last `loca` entry is the length of `glyf` — 2056 here under the long reading and 0
///   under the short one, against a `glyf` table of 2056 bytes;
/// - `loca` holds `numGlyphs + 1` entries, so its length is `2 × (n + 1)` or `4 × (n + 1)` —
///   244 here, which is the long form for 60 glyphs and twice the short form's 122.
///
/// Both agree, and only one format satisfies either. So this is the same shape as the
/// twenty-seventh session's LZW finding: **a file that states one fact twice can check
/// itself**, and no other implementation's behaviour is involved.
///
/// Returns `None` when the field is already 0 or 1, when the tables it needs are absent or
/// short, or when *neither* reading satisfies both tests — in which case the font is broken in
/// a way this cannot name, and `skrifa`'s own answer stands.
fn repaired_loca_format(data: &[u8]) -> Option<Vec<u8>> {
    /// Offset of `indexToLocFormat` within the `head` table.
    const INDEX_TO_LOC: usize = 50;

    let be16 = |at: usize| -> Option<u16> {
        let bytes = data.get(at..at.checked_add(2)?)?;
        Some(u16::from_be_bytes([*bytes.first()?, *bytes.get(1)?]))
    };
    let be32 = |at: usize| -> Option<u32> {
        let bytes = data.get(at..at.checked_add(4)?)?;
        Some(u32::from_be_bytes([
            *bytes.first()?,
            *bytes.get(1)?,
            *bytes.get(2)?,
            *bytes.get(3)?,
        ]))
    };

    let count = be16(4)?;
    let mut tables = BTreeMap::new();
    for index in 0..usize::from(count) {
        let entry = 12usize.checked_add(index.checked_mul(16)?)?;
        let tag = data.get(entry..entry.checked_add(4)?)?.to_vec();
        let offset = usize::try_from(be32(entry.checked_add(8)?)?).ok()?;
        let length = usize::try_from(be32(entry.checked_add(12)?)?).ok()?;
        tables.insert(tag, (offset, length));
    }

    let (head, _) = *tables.get(b"head".as_slice())?;
    let stated = be16(head.checked_add(INDEX_TO_LOC)?)?;
    if stated <= 1 {
        return None;
    }

    let (loca, loca_length) = *tables.get(b"loca".as_slice())?;
    let (_, glyf_length) = *tables.get(b"glyf".as_slice())?;
    let (maxp, _) = *tables.get(b"maxp".as_slice())?;
    let glyphs = usize::from(be16(maxp.checked_add(4)?)?);
    let entries = glyphs.checked_add(1)?;

    // Short offsets are stored halved, which is why the last one is doubled to compare.
    let short = be16(loca.checked_add(entries.checked_sub(1)?.checked_mul(2)?)?)
        .map(|value| usize::from(value).checked_mul(2));
    let long = be32(loca.checked_add(entries.checked_sub(1)?.checked_mul(4)?)?)
        .and_then(|value| usize::try_from(value).ok());
    let fits = |width: usize, last: Option<usize>| {
        last == Some(glyf_length) && entries.checked_mul(width) == Some(loca_length)
    };

    let corrected: u16 = if fits(2, short.flatten()) {
        0
    } else if fits(4, long) {
        1
    } else {
        return None;
    };

    let mut repaired = data.to_vec();
    let slot = repaired.get_mut(
        head.checked_add(INDEX_TO_LOC)?..head.checked_add(INDEX_TO_LOC)?.checked_add(2)?,
    )?;
    slot.copy_from_slice(&corrected.to_be_bytes());
    Some(repaired)
}

/// Returns `true` for a bare CFF font program.
///
/// A CFF file starts with a header whose first two bytes are its major and minor version,
/// conventionally 1 and 0. An sfnt file starts with a recognisable tag instead — `0x00010000`,
/// `OTTO`, `true` or `ttcf` — so a leading `01 00` that is none of those is CFF.
fn is_bare_cff(data: &[u8]) -> bool {
    match data.get(..4) {
        // The four sfnt container signatures.
        Some([0x00, 0x01, 0x00, 0x00] | b"OTTO" | b"true" | b"ttcf") => false,
        // A CFF header begins with its major and minor version.
        Some([1, 0, ..]) => true,
        _ => false,
    }
}

/// Parses a bare Type 1 program, for the one kind of program that is kept parsed.
///
/// Done once at load rather than in `build_outline`, because the units per em, the code
/// mapping and every outline come out of the same parse and that parse is the expensive
/// one; see [`type1::Program`].
fn parsed_type1(
    program: Program,
    data: &[u8],
    name: &str,
) -> Result<Option<type1::Program>, FontError> {
    match program {
        Program::Type1 => type1::Program::parse(data)
            .map(Some)
            .map_err(|e| FontError::Malformed {
                name: name.to_owned(),
                detail: e.to_string(),
            }),
        Program::Sfnt | Program::BareCff => Ok(None),
    }
}

/// Reads a font's units per em, which every outline is scaled by.
fn units_per_em(data: &[u8], program: Program, name: &str) -> Result<f32, FontError> {
    if program == Program::BareCff {
        return cff::units_per_em(data).map_err(|e| FontError::Malformed {
            name: name.to_owned(),
            detail: e.to_string(),
        });
    }

    let font = FontRef::new(data).map_err(|e| FontError::Malformed {
        name: name.to_owned(),
        detail: e.to_string(),
    })?;
    let units = font
        .metrics(Size::unscaled(), LocationRef::default())
        .units_per_em;
    if units == 0 {
        return Err(FontError::Malformed {
            name: name.to_owned(),
            detail: "units per em is zero".to_owned(),
        });
    }
    Ok(f32::from(units))
}

/// Collects glyph outlines into a [`Path`], scaling to em-normalised coordinates.
struct PathPen {
    path: Path,
    scale: f32,
    /// The current point, needed to elevate quadratic curves to cubics.
    last: Option<Point>,
}

impl PathPen {
    fn at(&self, x: f32, y: f32) -> Point {
        Point::new(x * self.scale, y * self.scale)
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
    use super::{CidToGlyph, Code, CodeMapping, LoadedFont, Program};
    use pdf_syntax::{Dictionary, Document};

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
    /// so the round trip has to start and end at the *character*. Run over every simple font
    /// on a first page of the specification corpus, so it is a statement about real encodings
    /// rather than about one constructed table.
    #[test]
    fn a_code_for_a_character_means_that_character() {
        let mut checked = 0usize;
        for path in corpus() {
            let bytes = std::fs::read(&path).expect("corpus file is readable");
            let Ok(document) = Document::open(bytes) else {
                continue;
            };
            for (name, dict) in first_page_fonts(&document) {
                let Ok(font) = LoadedFont::load(&document, &dict, &name) else {
                    continue;
                };
                if !matches!(font.mapping, CodeMapping::Named(_)) {
                    assert!(
                        font.code_for('A').is_none(),
                        "{name} is composite and must refuse rather than guess a code length"
                    );
                    continue;
                }
                let mut meaning = String::new();
                for byte in 0..=u8::MAX {
                    meaning.clear();
                    if !font.text(Code::single_byte(byte), &mut meaning) {
                        continue;
                    }
                    let mut characters = meaning.chars();
                    let (Some(character), None) = (characters.next(), characters.next()) else {
                        continue;
                    };
                    let Some(code) = font.code_for(character) else {
                        continue;
                    };
                    let mut back = String::new();
                    assert!(
                        font.text(code, &mut back),
                        "{name}: code {code:?} means nothing"
                    );
                    assert_eq!(
                        back, meaning,
                        "{name}: code {code:?} does not draw {character:?}"
                    );
                    checked = checked.saturating_add(1);
                }
            }
        }
        assert!(
            checked > 1000,
            "only {checked} codes checked; the corpus is not present"
        );
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

    /// A code the encoding does not cover must have no glyph at all.
    ///
    /// This is the regression test for the defect that motivated the work: a CFF font
    /// whose lookup falls through to treating the character code as a glyph index loads
    /// cleanly, reports nothing unsupported, and draws whatever glyph happens to sit at
    /// that index. Every subset font in the corpus has far fewer glyphs than codes, so a
    /// fall-through would show up here as a glyph where there should be none.
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

            for (code, slot) in table.iter().enumerate() {
                let Ok(byte) = u8::try_from(code) else {
                    continue;
                };
                assert!(
                    slot.is_some() || font.outline(Code::single_byte(byte)).is_none(),
                    "{file} /{name}: code {code} has no glyph in the encoding but still \
                     produced an outline"
                );
            }
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

/// ISO 32000-2 §9.6.5.4, one rule at a time, on fonts built to isolate it.
///
/// The corpus cannot do this. It can show that a real document draws, and it did — but
/// every real font carries several `cmap` subtables, so a page drawing correctly proves
/// only that *some* route worked, and a page drawing wrongly does not say which route was
/// missing. Each font here carries exactly one subtable, so exactly one rule of the
/// subclause can possibly apply to it, and a rule that stops working fails one test by
/// name. This is trap 8's argument in the handover, from the other direction.
#[cfg(test)]
mod truetype_encoding_tests {
    use super::{
        Subtables, as_character, named_glyph, post_glyph, repaired_loca_format, symbol_glyph,
    };
    use skrifa::{FontRef, MetadataProvider};

    /// The glyph index every fixture below maps its one covered code to.
    ///
    /// Deliberately not equal to any code used here: a route that quietly fell back to
    /// treating the code as a glyph index would otherwise pass.
    const GLYPH: u16 = 7;

    /// Assembles an sfnt file from tables, which is all `FontRef` needs to read one.
    fn sfnt(tables: &[([u8; 4], Vec<u8>)]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&0x0001_0000_u32.to_be_bytes());
        let count = u16::try_from(tables.len()).expect("a handful of tables");
        out.extend_from_slice(&count.to_be_bytes());
        // Binary-search hints, which nothing here reads and a well-formed file still has.
        out.extend_from_slice(&0_u16.to_be_bytes());
        out.extend_from_slice(&0_u16.to_be_bytes());
        out.extend_from_slice(&0_u16.to_be_bytes());

        let directory = 12_usize.saturating_add(16_usize.saturating_mul(tables.len()));
        let mut offset = directory;
        let mut body = Vec::new();
        for (tag, data) in tables {
            out.extend_from_slice(tag);
            out.extend_from_slice(&0_u32.to_be_bytes());
            out.extend_from_slice(&u32::try_from(offset).expect("small file").to_be_bytes());
            out.extend_from_slice(
                &u32::try_from(data.len())
                    .expect("small table")
                    .to_be_bytes(),
            );
            body.extend_from_slice(data);
            // Every table starts on a four-byte boundary.
            while body.len() % 4 != 0 {
                body.push(0);
            }
            offset = directory.saturating_add(body.len());
        }
        out.extend_from_slice(&body);
        out
    }

    /// A `head`, a `maxp`, a `loca` and a `glyf`, enough for `repaired_loca_format`.
    ///
    /// `stated` is what the `head` table claims `indexToLocFormat` is; the tables themselves
    /// are always built in the *long* form, so a fixture is well-formed exactly when it
    /// states 1.
    fn loca_fixture(stated: u16, glyphs: u16) -> Vec<([u8; 4], Vec<u8>)> {
        let entries = usize::from(glyphs).saturating_add(1);
        // Two bytes of glyph data apiece, so the last offset is a number nothing else is.
        let glyf = vec![0u8; entries.saturating_sub(1).saturating_mul(2)];
        let mut loca = Vec::new();
        for index in 0..entries {
            let at = u32::try_from(index.saturating_mul(2)).expect("small");
            loca.extend_from_slice(&at.to_be_bytes());
        }
        let mut head = vec![0u8; 54];
        head.splice(50..52, stated.to_be_bytes());
        let mut maxp = vec![0u8; 6];
        maxp.splice(4..6, glyphs.to_be_bytes());
        vec![
            (*b"head", head),
            (*b"maxp", maxp),
            (*b"loca", loca),
            (*b"glyf", glyf),
        ]
    }

    /// A byte-swapped `indexToLocFormat` is repaired from the file's own two statements.
    ///
    /// `issue2537r.pdf` states 0x0100, which is 1 in the wrong byte order and is neither of
    /// the two values ISO/IEC 14496-22 defines. What decides the repair is not another
    /// reader's behaviour but the file: `loca`'s last entry is `glyf`'s length under one
    /// reading and not the other, and `loca`'s own length is `4 × (n + 1)` rather than
    /// `2 × (n + 1)`. Both agree, so the answer is derived twice over.
    #[test]
    fn a_byte_swapped_loca_format_is_repaired_from_the_fonts_own_lengths() {
        let broken = sfnt(&loca_fixture(0x0100, 8));
        let repaired = repaired_loca_format(&broken).expect("the file says which format it is");

        let head = 12 + 16 * 4;
        assert_eq!(&repaired[head + 50..head + 52], &1_u16.to_be_bytes());
        assert_eq!(
            repaired.len(),
            broken.len(),
            "two bytes change, nothing else"
        );
    }

    /// A font that states a legal format is left alone, whichever of the two it states.
    #[test]
    fn a_font_stating_a_legal_loca_format_is_not_touched() {
        for stated in [0, 1] {
            assert_eq!(repaired_loca_format(&sfnt(&loca_fixture(stated, 8))), None);
        }
    }

    /// A font whose lengths agree with *neither* reading keeps its own answer.
    ///
    /// The point of the two tests is that this repair can fail: a `loca` that is neither
    /// `2 × (n + 1)` nor `4 × (n + 1)` bytes long is broken in a way this cannot name, and
    /// inventing a format for it would be exactly the guess the derivation exists to avoid.
    #[test]
    fn a_font_neither_reading_explains_is_left_to_skrifa() {
        let mut tables = loca_fixture(0x0100, 8);
        // One byte short of either form.
        tables[2].1.pop();
        assert_eq!(repaired_loca_format(&sfnt(&tables)), None);
    }

    /// A `cmap` with one subtable, in format 6, under the platform and encoding IDs given.
    ///
    /// Format 6 throughout, for every platform, so that what differs between the fixtures
    /// is only the identity §9.6.5.4 selects on. A subtable format is a storage detail and
    /// `read-fonts` reads all the common ones; the platform and encoding IDs are the
    /// subclause's whole subject.
    fn cmap(platform: u16, encoding: u16, first_code: u16, glyphs: &[u16]) -> Vec<u8> {
        let mut subtable = Vec::new();
        subtable.extend_from_slice(&6_u16.to_be_bytes());
        let length = 10_usize.saturating_add(2_usize.saturating_mul(glyphs.len()));
        subtable.extend_from_slice(&u16::try_from(length).expect("short").to_be_bytes());
        subtable.extend_from_slice(&0_u16.to_be_bytes());
        subtable.extend_from_slice(&first_code.to_be_bytes());
        subtable.extend_from_slice(&u16::try_from(glyphs.len()).expect("few").to_be_bytes());
        for glyph in glyphs {
            subtable.extend_from_slice(&glyph.to_be_bytes());
        }

        let mut table = Vec::new();
        table.extend_from_slice(&0_u16.to_be_bytes());
        table.extend_from_slice(&1_u16.to_be_bytes());
        table.extend_from_slice(&platform.to_be_bytes());
        table.extend_from_slice(&encoding.to_be_bytes());
        table.extend_from_slice(&12_u32.to_be_bytes());
        table.extend_from_slice(&subtable);
        table
    }

    /// A version 2.0 `post` table naming one glyph, and leaving every other `.notdef`.
    fn post(glyph: u16, name: &str) -> Vec<u8> {
        /// How many names the format reserves before a font's own begin.
        const MACINTOSH_NAMES: u16 = 258;

        let count = glyph.saturating_add(1);
        let mut table = vec![0_u8; 32];
        table.splice(0..4, 0x0002_0000_u32.to_be_bytes());
        table.extend_from_slice(&count.to_be_bytes());
        for index in 0..count {
            let entry = if index == glyph { MACINTOSH_NAMES } else { 0 };
            table.extend_from_slice(&entry.to_be_bytes());
        }
        table.push(u8::try_from(name.len()).expect("a short name"));
        table.extend_from_slice(name.as_bytes());
        table
    }

    /// The premise of the whole algorithm, asserted rather than assumed.
    ///
    /// `skrifa`'s `Charmap` selects the best *Unicode* subtable, and a (1, 0) Macintosh one
    /// is not a Unicode mapping, so it selects nothing. Handing it a character code —
    /// which is what this crate used to do for every `TrueType` font — therefore reaches no
    /// glyph at all in a font shaped the way §9.6.5.4's own guidelines ask for. If this
    /// test ever fails, `skrifa` has changed and the reasoning in `Subtables` needs
    /// re-reading, not the code.
    #[test]
    fn a_unicode_charmap_cannot_see_a_macintosh_subtable() {
        let data = sfnt(&[(*b"cmap", cmap(1, 0, 33, &[GLYPH]))]);
        let font = FontRef::new(&data).expect("the fixture is a readable sfnt");

        assert_eq!(font.charmap().map(33_u32), None);
        assert_eq!(symbol_glyph(&Subtables::read(&font), 33), Some(GLYPH));
    }

    /// "Otherwise, if the font contains a (1, 0) subtable, single bytes from the string
    /// shall be used to look up the associated glyph descriptions from the subtable."
    #[test]
    fn a_macintosh_subtable_is_addressed_by_the_byte_itself() {
        let data = sfnt(&[(*b"cmap", cmap(1, 0, 33, &[GLYPH]))]);
        let font = FontRef::new(&data).expect("readable");
        let subtables = Subtables::read(&font);

        assert_eq!(symbol_glyph(&subtables, 33), Some(GLYPH));
        assert_eq!(symbol_glyph(&subtables, 34), None);
    }

    /// "If the font contains a (3, 0) subtable, the range of character codes shall be one
    /// of these: 0x0000 - 0x00FF, 0xF000 - 0xF0FF, 0xF100 - 0xF1FF, or 0xF200 - 0xF2FF.
    /// Depending on the range of codes, each byte from the string shall be prepended with
    /// the high byte of the range."
    #[test]
    fn a_symbol_subtable_is_addressed_through_the_high_byte_of_its_range() {
        for high in [0x0000_u16, 0xF000, 0xF100, 0xF200] {
            let data = sfnt(&[(*b"cmap", cmap(3, 0, high | 0x41, &[GLYPH]))]);
            let font = FontRef::new(&data).expect("readable");

            assert_eq!(
                symbol_glyph(&Subtables::read(&font), 0x41),
                Some(GLYPH),
                "a (3, 0) subtable in the {high:#06x} range was not found"
            );
        }
    }

    /// "A character code shall be first mapped to a glyph name … the glyph name shall then
    /// be mapped to a Unicode value by consulting the Adobe Glyph List … finally, the
    /// Unicode value shall be mapped to a glyph description according to the (3, 1)
    /// subtable."
    #[test]
    fn a_unicode_subtable_is_reached_through_the_adobe_glyph_list() {
        // U+00E9, which the Adobe Glyph List spells `eacute`.
        let data = sfnt(&[(*b"cmap", cmap(3, 1, 0x00E9, &[GLYPH]))]);
        let font = FontRef::new(&data).expect("readable");
        let subtables = Subtables::read(&font);

        assert_eq!(named_glyph(&font, &subtables, None, "eacute"), Some(GLYPH));
        assert_eq!(named_glyph(&font, &subtables, None, "egrave"), None);
    }

    /// "The glyph name shall then be mapped back to a character code according to the
    /// standard Roman encoding used on Mac OS."
    ///
    /// The name and the code are chosen where Mac OS Roman and every other encoding
    /// disagree: `eacute` is code 142 there and 233 in `WinAnsiEncoding`. A route reaching
    /// this subtable with the wrong encoding's code finds nothing, rather than finding a
    /// plausible wrong glyph — which is why the fixture covers only the one code.
    #[test]
    fn a_macintosh_subtable_is_reached_through_mac_os_roman() {
        let data = sfnt(&[(*b"cmap", cmap(1, 0, 142, &[GLYPH]))]);
        let font = FontRef::new(&data).expect("readable");
        let subtables = Subtables::read(&font);

        assert_eq!(named_glyph(&font, &subtables, None, "eacute"), Some(GLYPH));
        assert_eq!(named_glyph(&font, &subtables, None, "adieresis"), None);
    }

    /// "In any of these cases, if the glyph name cannot be mapped as specified, the glyph
    /// name shall be looked up in the font program's `post` table."
    ///
    /// `gid2436` is the shape that motivated keeping unrecognised `/Differences` names at
    /// all: a subsetter's convention for naming a glyph by index, which no encoding and no
    /// character set knows, and which the font itself may nonetheless carry.
    #[test]
    fn a_name_no_encoding_knows_is_found_in_the_post_table() {
        let data = sfnt(&[
            (*b"cmap", cmap(3, 1, 0x00E9, &[1])),
            (*b"post", post(GLYPH, "gid2436")),
        ]);
        let font = FontRef::new(&data).expect("readable");
        let subtables = Subtables::read(&font);

        assert_eq!(post_glyph(&font, "gid2436"), Some(GLYPH));
        assert_eq!(named_glyph(&font, &subtables, None, "gid2436"), Some(GLYPH));
        assert_eq!(named_glyph(&font, &subtables, None, "gid9999"), None);
    }

    /// A suffixed name is not one the Adobe Glyph List holds, so the `post` table decides.
    ///
    /// §9.6.5.4 sends a glyph name "to a Unicode value by consulting the Adobe Glyph List and
    /// Adobe Glyph List for New Fonts", and then to the `post` table "if the glyph name cannot
    /// be mapped as specified". Those are two *lists*, and neither holds an entry containing a
    /// FULL STOP — but the wider Adobe Glyph List *Specification* carries an algorithm for
    /// unlisted names that strips everything after the first period, which `read_fonts`
    /// implements and which answers `o` for `o.sc`. That answer is a real letter, so it hid
    /// the clause's own next step behind it and `issue215.pdf` drew `openmagazin` in lower
    /// case where four references draw small capitals — which the file itself confirms, by
    /// mapping those codes to U+F76F and its neighbours, the private-use block Adobe assigns
    /// to small capitals.
    ///
    /// The fixture puts both glyphs in reach so that a wrong order is a wrong answer rather
    /// than a missing one: the (3, 1) subtable holds `o` at glyph 1 and the `post` table names
    /// glyph 7 `o.sc`.
    #[test]
    fn a_suffixed_glyph_name_reaches_the_program_rather_than_its_base_letter() {
        let data = sfnt(&[
            (*b"cmap", cmap(3, 1, 0x006F, &[1])),
            (*b"post", post(GLYPH, "o.sc")),
        ]);
        let font = FontRef::new(&data).expect("readable");
        let subtables = Subtables::read(&font);

        assert_eq!(named_glyph(&font, &subtables, None, "o.sc"), Some(GLYPH));
        // And the unsuffixed name still takes the clause's first route.
        assert_eq!(named_glyph(&font, &subtables, None, "o"), Some(1));
        // A suffixed name the program does not carry falls back to the base letter, which
        // is a recovery rather than the clause's route — better than drawing nothing, and
        // last precisely because it is not what the subclause says.
        assert_eq!(named_glyph(&font, &subtables, None, "o.alt"), Some(1));
    }

    /// The mapping of this processor's choosing reaches a subtable §9.6.5.4 never names.
    ///
    /// `issue5501.pdf` carries its whole byte-to-glyph mapping in a (0, 0) subtable, which
    /// the subclause does not mention and which is the only correct answer for that font.
    #[test]
    fn the_last_resort_asks_every_subtable_including_unnamed_ones() {
        let data = sfnt(&[(*b"cmap", cmap(0, 0, 4, &[GLYPH]))]);
        let font = FontRef::new(&data).expect("readable");
        let subtables = Subtables::read(&font);

        assert_eq!(symbol_glyph(&subtables, 4), None, "not a (3, 0) or (1, 0)");
        assert_eq!(as_character(&subtables, 4), Some(GLYPH));
    }

    /// A symbolic font's private-use convention, which predates this algorithm here.
    #[test]
    fn the_last_resort_also_tries_the_private_use_area() {
        let data = sfnt(&[(*b"cmap", cmap(3, 1, 0xF041, &[GLYPH]))]);
        let font = FontRef::new(&data).expect("readable");

        assert_eq!(as_character(&Subtables::read(&font), 0x41), Some(GLYPH));
    }

    /// A font with no `cmap` at all is the only one whose codes may be glyph indices.
    #[test]
    fn only_a_font_without_a_cmap_has_no_subtables_to_ask() {
        let with = sfnt(&[(*b"cmap", cmap(1, 0, 33, &[GLYPH]))]);
        let without = sfnt(&[(*b"post", post(GLYPH, "gid2436"))]);

        let font = FontRef::new(&with).expect("readable");
        assert!(Subtables::read(&font).all.is_some());
        let font = FontRef::new(&without).expect("readable");
        assert!(Subtables::read(&font).all.is_none());
    }
}

/// ISO 32000-2 §9.6.5.1's Table 112, on a name-keyed program: which encoding is the *base*.
///
/// The rule is a sentence rather than an algorithm, and it is the sentence a corpus cannot
/// test. Table 112 makes an **embedded** font program's own built-in encoding the default
/// base, and only a font whose program is *not* embedded falls back to `StandardEncoding`
/// or to the Symbolic flag. Nearly every real CFF's built-in encoding *is*
/// `StandardEncoding`, so reading the wrong sentence is invisible on all 974 corpus
/// documents but one, and on that one it moves a single code.
///
/// So the fixtures state the two encodings *differently* and ask which one answered. The
/// CFF is a value rather than a font program: what is under test is the choice of base,
/// which happens entirely in [`simple_code_table`], and building a byte-level CFF here
/// would test `read-fonts` instead.
#[cfg(test)]
mod cff_encoding_tests {
    use std::borrow::Cow;
    use std::collections::BTreeMap;

    use super::fixture::font_dictionary;
    use super::{NameKeyed, simple_code_table};

    /// The glyphs the fixture font holds, by the names its charset gives them.
    const ALPHA: u16 = 11;
    const BETA: u16 = 12;
    /// The charset also names a glyph `A`, which is what `StandardEncoding` asks for at 65.
    const LATIN_A: u16 = 13;

    /// A name-keyed CFF whose built-in encoding disagrees with `StandardEncoding`.
    ///
    /// Code 65 is `A` in `StandardEncoding` and `alpha` in this font; code 66 is `B`, which
    /// the charset does not have at all, and `beta` in this font. So each code distinguishes
    /// the two bases, and in opposite ways: one would draw the wrong glyph and the other
    /// would draw nothing.
    fn fixture() -> NameKeyed {
        let by_name: BTreeMap<Box<str>, u16> = [
            ("alpha".into(), ALPHA),
            ("beta".into(), BETA),
            ("A".into(), LATIN_A),
        ]
        .into_iter()
        .collect();
        let mut builtin = Box::new([None; 256]);
        builtin[65] = Some(ALPHA);
        builtin[66] = Some(BETA);
        let builtin_names = Box::new(std::array::from_fn(|code| match code {
            65 => Some("alpha".into()),
            66 => Some("beta".into()),
            _ => None,
        }));
        NameKeyed {
            by_name,
            builtin,
            builtin_names,
        }
    }

    /// Resolves the fixture font's codes under the `/Encoding` entries given.
    fn resolve(entries: &str) -> (super::CodeTable, super::GlyphNames) {
        let (document, dict) = font_dictionary(entries);
        simple_code_table(&document, &dict, &fixture(), "F1").expect("the fixture resolves")
    }

    /// With no `/Encoding`, every code is the program's own — which is the uncontested half.
    #[test]
    fn a_font_with_no_encoding_entry_is_its_own() {
        let (table, names) = resolve("");

        assert_eq!(table[65], Some(ALPHA));
        assert_eq!(table[66], Some(BETA));
        assert_eq!(names[65], Cow::Borrowed("alpha"));
    }

    /// `/Differences` alone describes differences from the *program's* encoding.
    ///
    /// The rule that was wrong: a code the array does not mention keeps the built-in
    /// glyph. Reading `StandardEncoding` as the base instead would draw `LATIN_A` at 65 —
    /// a plausible wrong letter — and nothing at all at 66.
    #[test]
    fn differences_without_a_base_encoding_layer_over_the_programs_own() {
        let (table, names) = resolve("/Encoding << /Type /Encoding /Differences [66 /A] >>");

        assert_eq!(table[66], Some(LATIN_A), "the array names this code");
        assert_eq!(table[65], Some(ALPHA), "and says nothing about this one");
        assert_eq!(names[65], Cow::Borrowed("alpha"));
        assert_eq!(names[66], Cow::Borrowed("A"));
    }

    /// A named `/BaseEncoding` is still the base, and the program's encoding is not consulted.
    ///
    /// Which is what keeps the rule above from being a licence: the document may state the
    /// base, and when it does, a code it leaves undefined is undefined rather than the
    /// font's own. Code 66 is `B`, which this charset lacks, so it reaches no glyph.
    #[test]
    fn a_named_base_encoding_still_wins() {
        let (table, _) = resolve("/Encoding << /BaseEncoding /WinAnsiEncoding >>");

        assert_eq!(table[65], Some(LATIN_A));
        assert_eq!(table[66], None);
    }

    /// The font descriptor's flags decide nothing here, which is the whole finding.
    ///
    /// Table 112 asks the Symbolic flag only for a font whose program is *not* embedded,
    /// and a bare CFF reaches this crate by having been embedded. Stated as a test because
    /// the previous reading was defensible from the same table's last sentence.
    #[test]
    fn the_symbolic_flag_changes_nothing_for_an_embedded_program() {
        let symbolic = resolve("/FontDescriptor << /Flags 4 >>").0;
        let nonsymbolic = resolve("/FontDescriptor << /Flags 32 >>").0;

        assert_eq!(symbolic[65], Some(ALPHA));
        assert_eq!(symbolic, nonsymbolic);
    }
}

/// A PDF whose one object is a font dictionary, for stating a rule about a dictionary.
///
/// Several rules in clause 9 are about what a font *dictionary* says, and reach a glyph
/// only afterwards. Building a font program to test one would test the program's reader
/// instead, so the fixtures that need only a dictionary build only a dictionary.
#[cfg(test)]
mod fixture {
    use std::fmt::Write as _;

    use pdf_syntax::{Dictionary, Document, ObjectId};

    /// A one-object document whose object 1 is a font dictionary with the given entries.
    pub(super) fn font_dictionary(entries: &str) -> (Document, Dictionary) {
        let body = format!("1 0 obj\n<< /Type /Font /Subtype /Type1 {entries} >>\nendobj\n");
        let mut out = String::from("%PDF-1.7\n");
        let offset = out.len();
        out.push_str(&body);
        let xref_at = out.len();
        out.push_str("xref\n0 2\n0000000000 65535 f \n");
        let _ = writeln!(out, "{offset:010} 00000 n ");
        let _ = write!(
            out,
            "trailer\n<< /Size 2 /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
        );
        let document = Document::open(out.into_bytes()).expect("the fixture is a valid PDF");
        let dict = document
            .get(ObjectId {
                number: 1,
                generation: 0,
            })
            .as_dict()
            .expect("object 1 is the font dictionary")
            .clone();
        (document, dict)
    }

    /// The same, with object 2 a stream the descriptor points at as `/FontFile2`.
    ///
    /// The bytes are deliberately not a font: what the callers need is a *deterministic*
    /// failure that every simple `/Subtype` reaches identically, and a program `skrifa`
    /// refuses gives one, where an absent program would send the load into substitution
    /// and make the test depend on which faces this machine has installed.
    pub(super) fn font_with_program(subtype: &str, program: &[u8]) -> (Document, Dictionary) {
        let mut body = format!(
            "1 0 obj\n<< /Type /Font /Subtype /{subtype} \
             /FontDescriptor << /Flags 32 /FontFile2 2 0 R >> >>\nendobj\n"
        );
        let _ = write!(
            body,
            "2 0 obj\n<< /Length {} >>\nstream\n{}\nendstream\nendobj\n",
            program.len(),
            String::from_utf8_lossy(program)
        );

        let mut out = String::from("%PDF-1.7\n");
        let mut offsets = Vec::new();
        for object in body.split_inclusive("endobj\n") {
            offsets.push(out.len());
            out.push_str(object);
        }
        let xref_at = out.len();
        let size = offsets.len().saturating_add(1);
        let _ = writeln!(out, "xref\n0 {size}");
        out.push_str("0000000000 65535 f \n");
        for offset in &offsets {
            let _ = writeln!(out, "{offset:010} 00000 n ");
        }
        let _ = write!(
            out,
            "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
        );
        let document = Document::open(out.into_bytes()).expect("the fixture is a valid PDF");
        let dict = document
            .get(ObjectId {
                number: 1,
                generation: 0,
            })
            .as_dict()
            .expect("object 1 is the font dictionary")
            .clone();
        (document, dict)
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
    use super::fixture::font_with_program;
    use super::{FontError, LoadedFont};

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
}

/// ISO 32000-2 §9.8.1's Table 120, on the one entry of it that moves a glyph.
///
/// A width is not a picture, so no rendering test can be pointed at this: what it changes
/// is where the *next* glyph goes, and only on a code the document's own `/Widths` does
/// not cover. `issue7439.pdf` is the corpus's one witness and the oracle holds it by name;
/// these state the rule itself, which is what survives that file being deleted.
#[cfg(test)]
#[expect(
    clippy::float_cmp,
    reason = "both sides are an integer written in the fixture or a constant, so the \
              comparison is exact by construction"
)]
mod missing_width_tests {
    use super::fixture::font_dictionary;
    use super::missing_width;

    /// `/MissingWidth` is read from the descriptor when it is there.
    #[test]
    fn a_stated_missing_width_is_the_width() {
        let (document, dict) = font_dictionary("/FontDescriptor << /MissingWidth 321 >>");
        let descriptor = document.get_key(&dict, "FontDescriptor");

        assert_eq!(missing_width(&document, descriptor.as_dict()), 321.0);
    }

    /// Table 120's default value is 0, not a guess at an average glyph.
    #[test]
    fn an_absent_missing_width_is_zero() {
        let (document, dict) = font_dictionary("/FontDescriptor << /Flags 32 >>");
        let descriptor = document.get_key(&dict, "FontDescriptor");

        assert_eq!(missing_width(&document, descriptor.as_dict()), 0.0);
        assert_eq!(missing_width(&document, None), 0.0);
    }
}

/// ISO 32000-2 §9.7.4.3's vertical metrics: `/DW2` and `/W2`, in glyph space.
///
/// > Glyphs from a CIDFont may be shown in vertical writing mode. This is selected by the
/// > WMode entry in the associated CMap dictionary … To be used in this way, the CIDFont
/// > shall define the vertical displacement for each glyph and the position vector that
/// > relates the horizontal and vertical writing origins.
///
/// Two vectors per glyph, and only three of their four components are ever stated: the
/// displacement `w1` has a horizontal component the clause fixes at 0, and the position
/// vector `v`'s horizontal component defaults to "half the glyph width" — which is why this
/// carries the *horizontal* width in as an argument rather than duplicating it.
#[derive(Debug)]
struct Vertical {
    /// `/DW2`'s two numbers: the vertical component of `v`, then that of `w1`.
    ///
    /// Table 122's default is `[880 -1000]`, and the sign is the clause's own NOTE: "a
    /// negative value for the vertical component places the origin of the next glyph below
    /// the current glyph because vertical coordinates in a standard coordinate system
    /// increase from bottom to top".
    default: [f32; 2],
    /// `/W2`, by CID: the vertical displacement `w1y`, then `v`'s two components.
    metrics: BTreeMap<u32, [f32; 3]>,
}

impl Vertical {
    /// Reads `/DW2` and `/W2` from a `CIDFont` dictionary.
    fn read(document: &Document, descendant: &Dictionary) -> Self {
        let default = document
            .get_key(descendant, "DW2")
            .as_array()
            .and_then(|items| {
                let read = |at: usize| {
                    items
                        .get(at)
                        .map(|item| document.resolve(item))
                        .and_then(|item| item.as_number())
                        .map(narrow)
                };
                Some([read(0)?, read(1)?])
            })
            .unwrap_or([880.0, -1000.0]);

        Self {
            default,
            metrics: composite_vertical_metrics(document, descendant),
        }
    }

    /// The displacement `w1` and position vector `v` for one CID, in glyph space.
    ///
    /// `width` is the same glyph's horizontal displacement `w0`, because the clause defines
    /// `v`'s horizontal component as half of it whenever `/W2` does not state one.
    fn metrics(&self, cid: u32, width: f32) -> ([f32; 2], [f32; 2]) {
        match self.metrics.get(&cid) {
            Some([w1y, vx, vy]) => ([0.0, *w1y], [*vx, *vy]),
            None => ([0.0, self.default[1]], [width / 2.0, self.default[0]]),
        }
    }
}

/// Reads §9.7.4.3's `/W2` array, whose two formats mirror `/W`'s with three numbers per CID.
///
/// > The elements of the array shall be organised in groups of two or five … In the first
/// > format, c is a starting CID and shall be followed by an array containing numbers
/// > interpreted in groups of three.
fn composite_vertical_metrics(
    document: &Document,
    descendant: &Dictionary,
) -> BTreeMap<u32, [f32; 3]> {
    /// The same bound `/W` takes, and for the same reason.
    const MAX_RANGE: u32 = 1 << 16;

    let mut metrics = BTreeMap::new();
    let array = document.get_key(descendant, "W2");
    let Some(items) = array.as_array() else {
        return metrics;
    };
    let resolved: Vec<Object> = items.iter().map(|item| document.resolve(item)).collect();
    let number = |at: Option<&Object>| at.and_then(Object::as_number).map(narrow);

    let mut index = 0usize;
    while index < resolved.len() {
        let Some(first) = resolved
            .get(index)
            .and_then(Object::as_integer)
            .and_then(|value| u32::try_from(value).ok())
        else {
            break;
        };
        match resolved.get(index.saturating_add(1)) {
            Some(Object::Array(list)) => {
                let values: Vec<f32> = list
                    .iter()
                    .map(|item| document.resolve(item))
                    .map_while(|item| item.as_number().map(narrow))
                    .collect();
                for (offset, group) in values.chunks_exact(3).enumerate() {
                    let (Ok(offset), [w1y, vx, vy]) = (u32::try_from(offset), group) else {
                        continue;
                    };
                    // "Specifying a given CID value more than once should not be done. In the
                    // case where it is done, the first specification is the one that shall be
                    // used" — §9.7.4.3, of `/W`, and `/W2` is the same array one field wider.
                    metrics
                        .entry(first.saturating_add(offset))
                        .or_insert([*w1y, *vx, *vy]);
                }
                index = index.saturating_add(2);
            }
            Some(second) => {
                let Some(last) = second
                    .as_integer()
                    .and_then(|value| u32::try_from(value).ok())
                else {
                    break;
                };
                let group = [
                    number(resolved.get(index.saturating_add(2))),
                    number(resolved.get(index.saturating_add(3))),
                    number(resolved.get(index.saturating_add(4))),
                ];
                let (Some(w1y), Some(vx), Some(vy)) = (group[0], group[1], group[2]) else {
                    break;
                };
                let end = last.min(first.saturating_add(MAX_RANGE));
                for cid in first..=end {
                    metrics.entry(cid).or_insert([w1y, vx, vy]);
                }
                index = index.saturating_add(5);
            }
            None => break,
        }
    }

    metrics
}
