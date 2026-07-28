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
//! Embedded `TrueType` and CFF outlines, for both simple fonts and composite (Type0) fonts
//! with the Identity encoding — which between them cover the overwhelming majority of
//! modern documents. A font this crate cannot load returns an error naming why, so the
//! caller reports the text as undrawn rather than silently omitting it.

#![forbid(unsafe_code)]

pub mod cff;
pub mod encoding;
pub mod standard_metrics;
pub mod substitute;
pub mod tounicode;

use std::borrow::Cow;
use std::cell::RefCell;
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
use crate::encoding::BaseEncoding;

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

/// Width assumed when a font gives none, in thousandths of an em.
///
/// Half an em is close to average for Latin text, so spacing degrades gracefully rather
/// than collapsing to zero.
const DEFAULT_WIDTH: f32 = 500.0;

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
    /// Two bytes per code, and the code *is* the glyph index.
    ///
    /// `Identity-H` with an identity `CIDToGIDMap`, which is what almost every modern
    /// producer emits for subset fonts. It is also correct for a composite font whose
    /// embedded CFF is *not* CID-keyed, where the specification says to use CIDs as
    /// glyph indices directly.
    IdentityTwoByte,
    /// Two bytes per code, each a CID resolved through a CID-keyed CFF's charset.
    CidKeyed(BTreeMap<u16, u16>),
    /// Two bytes per code, resolved through what the code *means* rather than what it
    /// indexes.
    ///
    /// The only route to a substitute for a composite font: a CID indexes the glyphs of
    /// the font that defined it, so it says nothing about any other font, and only
    /// `/ToUnicode` records what the producer meant by it.
    Substituted(Box<tounicode::ToUnicode>),
}

/// Which reader extracts outlines from the embedded font program.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Program {
    /// An sfnt container — `TrueType` or `OpenType` — read through `skrifa`'s `FontRef`.
    Sfnt,
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
    mapping: CodeMapping,
    /// Glyph advances by character code, in thousandths of an em.
    widths: BTreeMap<u32, f32>,
    /// Advance for a code with no entry.
    default_width: f32,
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
        let units_per_em = units_per_em(&data, program, name)?;

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
                // A descriptor is present whenever a program was embedded.
                let descriptor = descriptor.ok_or_else(|| FontError::NotEmbedded {
                    name: name.to_owned(),
                })?;
                let (table, resolved) = simple_code_table(document, dict, descriptor, &cff, name)?;
                names = Some(resolved);
                CodeMapping::Named(Box::new(table))
            }
        };

        let widths = simple_widths(
            document,
            dict,
            SimpleMetrics {
                substituted,
                names: names.as_ref(),
                data: &data,
                mapping: &mapping,
                units_per_em,
            },
        );

        let default_width = descriptor
            .map(|descriptor| document.get_key(descriptor, "MissingWidth"))
            .and_then(|value| value.as_number())
            .map_or(DEFAULT_WIDTH, narrow);

        Ok(Self {
            data,
            program,
            mapping,
            widths,
            default_width,
            units_per_em,
            substituted: substituted.is_some(),
            to_unicode: to_unicode(document, dict),
            glyph_names: names,
            outlines: RefCell::new(BTreeMap::new()),
        })
    }

    /// Loads a composite (Type0) font.
    fn load_composite(
        document: &Document,
        dict: &Dictionary,
        name: &str,
    ) -> Result<Self, FontError> {
        // Only `Identity-H` is handled. A named CMap needs the CMap machinery, and guessing
        // would map codes to the wrong glyphs — plausible but wrong text, which is the worst
        // kind of rendering error.
        //
        // `Identity-V` maps codes the same way and differs in the writing mode, which is a
        // property of the *CMap* rather than of the mapping (§9.7.5.2): mode 1 is vertical,
        // and §9.2.4 gives a glyph in vertical writing a second set of metrics — a
        // displacement vector `w1` with a zero horizontal component, and a position vector
        // `v` from the horizontal origin to the vertical one, both from the CIDFont's `/W2`
        // and `/DW2` (§9.7.4.3). None of that is implemented, so a vertical run drawn as a
        // horizontal one is not a near miss: `vertical.pdf` should set two columns down the
        // right edge of the page and came out as one overlapping line across the top,
        // reporting nothing. Refused here until the metrics exist, per the rule that
        // unsupported input stays loud.
        let encoding = document.get_key(dict, "Encoding");
        let encoding_name = encoding.as_name().map_or_else(
            || "<stream CMap>".to_owned(),
            |value| String::from_utf8_lossy(value.as_bytes()).into_owned(),
        );
        if encoding_name == "Identity-V" {
            return Err(FontError::UnsupportedEncoding {
                name: name.to_owned(),
                encoding: "Identity-V, whose vertical writing mode needs /W2 metrics".to_owned(),
            });
        }
        if encoding_name != "Identity-H" {
            return Err(FontError::UnsupportedEncoding {
                name: name.to_owned(),
                encoding: encoding_name,
            });
        }

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
            Ok(Embedded { data, program }) => (data, program, false),
            Err(FontError::NotEmbedded { .. } | FontError::UnsupportedProgram { .. }) => {
                let request = substitute::Request::derive(document, &descendant, descriptor);
                let data = substitute::find(request).ok_or_else(|| FontError::NoSubstitute {
                    name: name.to_owned(),
                    reason: format!("no {:?} face is installed", request.family),
                })?;
                (data, Program::Sfnt, true)
            }
            Err(other) => return Err(other),
        };
        let units_per_em = units_per_em(&data, program, name)?;

        let mapping = if substituted {
            // A CID is meaningless outside the font that defined it — it is an index into
            // that font's glyphs, not a character — so a substitute can only be reached
            // through what the codes *mean*. `/ToUnicode` is the only thing that says so,
            // and a composite font without one cannot be substituted at all.
            let text = to_unicode(document, dict);
            if text.is_empty() {
                return Err(FontError::UnsupportedEncoding {
                    name: name.to_owned(),
                    encoding: "no /ToUnicode, so a substitute cannot be addressed".to_owned(),
                });
            }
            CodeMapping::Substituted(Box::new(text))
        } else {
            match program {
                // A `TrueType` descendant takes the CID as a glyph index directly.
                Program::Sfnt => CodeMapping::IdentityTwoByte,
                Program::BareCff => {
                    let cff = CodeToGlyph::read(&data).map_err(|e| FontError::Malformed {
                        name: name.to_owned(),
                        detail: e.to_string(),
                    })?;
                    match cff {
                        CodeToGlyph::Keyed { by_cid } => CodeMapping::CidKeyed(by_cid),
                        // A composite font may embed a font program that is not CID-keyed.
                        // The specification is explicit that its CIDs are then glyph
                        // indices, so the charset's names play no part.
                        CodeToGlyph::Named { .. } => CodeMapping::IdentityTwoByte,
                    }
                }
            }
        };

        // A `/CIDToGIDMap` stream remaps CIDs to glyphs; without reading it the glyphs
        // would be wrong, so it is refused rather than approximated. It describes the
        // embedded program, so it has nothing to say about a substitute.
        match document.get_key(&descendant, "CIDToGIDMap") {
            _ if substituted => {}
            Object::Null => {}
            Object::Name(map) if map == "Identity" => {}
            _ => {
                return Err(FontError::UnsupportedEncoding {
                    name: name.to_owned(),
                    encoding: "non-identity CIDToGIDMap".to_owned(),
                });
            }
        }

        let default_width = document
            .get_key(&descendant, "DW")
            .as_number()
            .map_or(1000.0, narrow);

        Ok(Self {
            data,
            program,
            mapping,
            substituted,
            to_unicode: to_unicode(document, dict),
            glyph_names: None,
            widths: composite_widths(document, &descendant),
            default_width,
            units_per_em,
            outlines: RefCell::new(BTreeMap::new()),
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
    /// Two sources, in order of authority. `/ToUnicode` is the producer's own statement of
    /// what a code means and is preferred. Failing that, the glyph name the encoding
    /// selects identifies a character through the Adobe Glyph List — which describes what
    /// was actually drawn, and so stays right even when a producer's `/ToUnicode` is not.
    ///
    /// Takes the destination by reference because extraction calls this once per character
    /// on the page, and returning a `String` would allocate for every one.
    pub fn text(&self, code: u32, out: &mut String) -> bool {
        if self.to_unicode.append(code, out) {
            return true;
        }
        let Some(names) = self.glyph_names.as_ref() else {
            return false;
        };
        let Some(name) = usize::try_from(code)
            .ok()
            .and_then(|code| names.get(code))
            .map(Cow::as_ref)
            .filter(|name| !name.is_empty())
        else {
            return false;
        };
        match read_fonts::ps::agl::name_to_char(name) {
            Some(character) => {
                out.push(character);
                true
            }
            None => false,
        }
    }

    /// Splits a PDF string into character codes.
    ///
    /// One byte per code for a simple font, two for an Identity composite font. Getting
    /// this wrong does not merely shift text, it reads entirely different glyphs.
    #[must_use]
    pub fn decode(&self, bytes: &[u8]) -> Vec<u32> {
        match self.mapping {
            CodeMapping::Named(_) => bytes.iter().map(|&byte| u32::from(byte)).collect(),
            CodeMapping::IdentityTwoByte
            | CodeMapping::CidKeyed(_)
            | CodeMapping::Substituted(_) => bytes
                .chunks(2)
                .map(|pair| match pair {
                    [high, low] => (u32::from(*high) << 8) | u32::from(*low),
                    // A trailing odd byte is malformed; treating it as a high byte matches
                    // other readers and keeps the rest of the string aligned.
                    [single] => u32::from(*single) << 8,
                    _ => 0,
                })
                .collect(),
        }
    }

    /// Returns a code's advance width in text-space units, where one em is 1.0.
    #[must_use]
    pub fn advance(&self, code: u32) -> f32 {
        self.widths
            .get(&code)
            .copied()
            .unwrap_or(self.default_width)
            / 1000.0
    }

    /// Returns the outline for a character code, with one em as one unit.
    ///
    /// That is the space PDF's text matrix expects, so the caller multiplies by the font
    /// size and nothing else.
    ///
    /// Returns `None` when the code has no glyph, which includes the ordinary case of a
    /// space in a font with no space outline.
    #[must_use]
    pub fn outline(&self, code: u32) -> Option<Arc<Path>> {
        let glyph = self.glyph_for(code)?;

        if let Some(cached) = self.outlines.borrow().get(&glyph) {
            return cached.clone();
        }
        let built = self.build_outline(glyph);
        self.outlines.borrow_mut().insert(glyph, built.clone());
        built
    }

    /// Resolves a character code to a glyph index.
    ///
    /// Deliberately not memoised. Two of the mappings build a `FontRef` here, which looks
    /// like a per-character cost worth caching — but measuring it on a dense specification
    /// page (3587 lookups, 211 distinct codes, two thirds of them through the character
    /// map) moved the interpretation pass by less than the run-to-run noise. `FontRef` is
    /// a zero-copy view over the table directory, not a parse. A cache here would be
    /// unmeasured cleverness, and `CLAUDE.md` forbids that.
    fn glyph_for(&self, code: u32) -> Option<u16> {
        match &self.mapping {
            // The code is the glyph index by construction.
            CodeMapping::IdentityTwoByte => u16::try_from(code).ok(),
            CodeMapping::CidKeyed(by_cid) => by_cid.get(&u16::try_from(code).ok()?).copied(),
            // The substitute has no notion of this document's CIDs, so the code is taken
            // to the character it stands for and that character is looked up.
            CodeMapping::Substituted(text) => {
                let font = FontRef::new(&self.data).ok()?;
                let character = text.char_for(code)?;
                let id = font.charmap().map(character)?;
                u16::try_from(id.to_u32()).ok()
            }
            // Resolved when the font was loaded. A code with no entry has no glyph, and
            // that is final: falling back to the code as a glyph index here is exactly
            // how a font draws plausible, wrong text.
            CodeMapping::Named(table) => *table.get(usize::try_from(code).ok()?)?,
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
/// This is the specification's encoding algorithm (ISO 32000-2, 9.6.6.2): choose a base
/// encoding, layer `/Differences` over it, and resolve the resulting glyph names through
/// the font's charset.
///
/// # Why an unresolved name is not retried against the font's own encoding
///
/// When the encoding names a glyph the font does not have, this leaves the code with no
/// glyph rather than falling back to whatever the font's built-in encoding puts there.
/// The fallback is tempting because it fills the page, and wrong because a subset font's
/// built-in encoding is arbitrary: it would draw *a* glyph, confidently, and not the one
/// the document asked for. A blank is a visible defect; a wrong letter is an invisible
/// one.
fn simple_code_table(
    document: &Document,
    dict: &Dictionary,
    descriptor: &Dictionary,
    cff: &CodeToGlyph,
    name: &str,
) -> Result<(CodeTable, GlyphNames), FontError> {
    let CodeToGlyph::Named { by_name, builtin } = cff else {
        // A CID-keyed program has no glyph names for `/Encoding` to address.
        return Err(FontError::UnsupportedEncoding {
            name: name.to_owned(),
            encoding: "CID-keyed CFF in a simple font".to_owned(),
        });
    };

    // With no base encoding named, a symbolic font uses the encoding built into the font
    // program — its glyphs are outside the standard Latin set, so Latin glyph names would
    // not address them. A nonsymbolic font defaults to StandardEncoding.
    let names = encoding_names(
        document,
        dict,
        name,
        None,
        !is_symbolic(document, descriptor),
    )?;

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
        *slot = glyph_name.and_then(|glyph_name| named_glyph(&font, &subtables, glyph_name));
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
fn named_glyph(font: &FontRef<'_>, subtables: &Subtables<'_>, glyph_name: &str) -> Option<u16> {
    let by_subtable = if let Some(unicode) = subtables.unicode.as_ref() {
        read_fonts::ps::agl::name_to_char(glyph_name)
            .and_then(|character| unicode.map_codepoint(character))
    } else if let Some(macintosh) = subtables.macintosh.as_ref() {
        encoding::mac_os_roman_code(glyph_name)
            .and_then(|code| macintosh.map_codepoint(u32::from(code)))
    } else {
        None
    };
    by_subtable
        .and_then(narrow_glyph)
        .or_else(|| post_glyph(font, glyph_name))
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

        return Ok(Embedded { data, program });
    }

    if document
        .get_key(descriptor, "FontFile")
        .as_stream()
        .is_some()
    {
        return Err(FontError::UnsupportedProgram {
            name: name.to_owned(),
            kind: "Type1",
        });
    }
    Err(FontError::NotEmbedded {
        name: name.to_owned(),
    })
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
    use super::{CodeMapping, LoadedFont, Program};
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
            .filter(|(_, _, f)| matches!(f.mapping, CodeMapping::CidKeyed(_)))
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
                let Some(glyph) = font.glyph_for(code) else {
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
                let Ok(code) = u32::try_from(code) else {
                    continue;
                };
                assert!(
                    slot.is_some() || font.outline(code).is_none(),
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
    use super::{Subtables, as_character, named_glyph, post_glyph, symbol_glyph};
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

        assert_eq!(named_glyph(&font, &subtables, "eacute"), Some(GLYPH));
        assert_eq!(named_glyph(&font, &subtables, "egrave"), None);
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

        assert_eq!(named_glyph(&font, &subtables, "eacute"), Some(GLYPH));
        assert_eq!(named_glyph(&font, &subtables, "adieresis"), None);
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
        assert_eq!(named_glyph(&font, &subtables, "gid2436"), Some(GLYPH));
        assert_eq!(named_glyph(&font, &subtables, "gid9999"), None);
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
