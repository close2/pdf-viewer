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

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::sync::Arc;

use pdf_render::{Path, PathCommand, Point};
use pdf_syntax::{Dictionary, Document, Object};
use skrifa::outline::{DrawSettings, OutlinePen};
use skrifa::prelude::{LocationRef, Size};
use skrifa::{FontRef, GlyphId, MetadataProvider};

use crate::cff::CodeToGlyph;
use crate::encoding::BaseEncoding;

/// A character code's glyph, for each of the 256 codes a simple font can use.
type CodeTable = [Option<u16>; 256];

/// Width assumed when a font gives none, in thousandths of an em.
///
/// Half an em is close to average for Latin text, so spacing degrades gracefully rather
/// than collapsing to zero.
const DEFAULT_WIDTH: f32 = 500.0;

/// How a font maps character codes to glyphs.
#[derive(Debug)]
enum CodeMapping {
    /// One byte per code, mapped through the font program's `cmap` table.
    ///
    /// `TrueType` and `OpenType` programs, where the character map is part of the file.
    Charmap,
    /// One byte per code, mapped through a table resolved when the font was loaded.
    ///
    /// A bare CFF has no `cmap`; a code reaches a glyph by name instead, and that
    /// resolution needs the PDF `/Encoding` as well as the font, so it happens once at
    /// load time rather than per glyph.
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
    /// The font is not embedded and this machine offers nothing to stand in for it.
    ///
    /// Distinct from [`FontError::NotEmbedded`], which no longer reaches a caller: a
    /// missing program is now substituted. This is the case where substitution itself
    /// failed, which is a property of the machine rather than of the document.
    #[error("font /{name} is not embedded and no {family} substitute is installed")]
    NoSubstitute {
        /// The resource name.
        name: String,
        /// The generic family that was looked for.
        family: String,
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
    glyph_names: Option<Box<[&'static str; 256]>>,
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
                    family: format!("{:?}", request.family),
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
                // The glyphs come from the program's own character map, but the encoding
                // still says what each code *means*, and an embedded `TrueType` font
                // frequently carries no `/ToUnicode`. Resolving the names here is what
                // lets such a font's text be read back at all.
                names = encoding_names(
                    document,
                    dict,
                    name,
                    None,
                    !descriptor.is_some_and(|d| is_symbolic(document, d)),
                )
                .ok();
                CodeMapping::Charmap
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
                names: names.as_deref(),
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
        // Only the Identity encodings are handled. A named CMap needs the CMap machinery,
        // and guessing would map codes to the wrong glyphs — plausible but wrong text,
        // which is the worst kind of rendering error.
        let encoding = document.get_key(dict, "Encoding");
        let encoding_name = encoding.as_name().map_or_else(
            || "<stream CMap>".to_owned(),
            |value| String::from_utf8_lossy(value.as_bytes()).into_owned(),
        );
        if !matches!(encoding_name.as_str(), "Identity-H" | "Identity-V") {
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
                    family: format!("{:?}", request.family),
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
            .copied()
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
            CodeMapping::Charmap | CodeMapping::Named(_) => {
                bytes.iter().map(|&byte| u32::from(byte)).collect()
            }
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
            // how a CFF font draws plausible, wrong text.
            CodeMapping::Named(table) => *table.get(usize::try_from(code).ok()?)?,
            CodeMapping::Charmap => {
                let font = FontRef::new(&self.data).ok()?;
                let charmap = font.charmap();
                charmap
                    // The code as written, which is right for symbolic fonts whose
                    // character map is keyed by byte.
                    .map(code)
                    // The same code in the private-use area, where symbolic TrueType fonts
                    // conventionally place their glyphs.
                    .or_else(|| charmap.map(0xF000_u32.saturating_add(code)))
                    .and_then(|id| u16::try_from(id.to_u32()).ok())
                    // A font with no usable character map is often a subset whose glyph
                    // order matches the codes. This is a guess, and it is confined to
                    // sfnt programs because for those it is usually right — a subset
                    // `TrueType` font really is ordered that way. A CFF font never
                    // reaches here.
                    .or_else(|| u16::try_from(code).ok())
            }
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
                    widths.insert(first.saturating_add(offset), narrow(width));
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
                    widths.insert(first.saturating_add(offset), narrow(width));
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
) -> Result<Box<[&'static str; 256]>, FontError> {
    let mut names = Box::new([""; 256]);

    if let Some(symbolic) = symbolic_font {
        // The two symbolic standard-14 fonts have their own encoding and no Latin base.
        for (code, slot) in names.iter_mut().enumerate() {
            if let Ok(code) = u8::try_from(code) {
                *slot = symbolic.glyph_name(code);
            }
        }
    } else {
        let base = base_encoding(document, dict, name)?
            .or(fall_back_to_standard.then_some(BaseEncoding::Standard));
        if let Some(base) = base {
            for (code, slot) in names.iter_mut().enumerate() {
                if let Ok(code) = u8::try_from(code) {
                    *slot = base.glyph_name(code);
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
) -> Result<(CodeTable, Box<[&'static str; 256]>), FontError> {
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
    let mut mapped = 0usize;
    for (code, slot) in table.iter_mut().enumerate() {
        let Some(glyph_name) = names.get(code).copied().filter(|n| !n.is_empty()) else {
            continue;
        };
        let Some(character) = read_fonts::ps::agl::name_to_char(glyph_name) else {
            continue;
        };
        *slot = charmap
            .map(character)
            .and_then(|id| u16::try_from(id.to_u32()).ok());
        if slot.is_some() {
            mapped = mapped.saturating_add(1);
        }
    }

    if mapped == 0 {
        return Err(FontError::NoSubstitute {
            name: name.to_owned(),
            family: format!(
                "{:?} (the substitute draws none of its characters)",
                request.family
            ),
        });
    }

    Ok((table, names))
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
fn apply_differences(document: &Document, dict: &Dictionary, names: &mut [&'static str; 256]) {
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
                // The name has to outlive this function, and a `/Differences` name is
                // almost always a standard one, so the static table is consulted first and
                // only a genuinely novel name is dropped. A novel name cannot be drawn by a
                // substitute anyway: it names a glyph only the original font had.
                if let Some(slot) = names.get_mut(at)
                    && let Some(known) = interned(glyph_name.as_bytes())
                {
                    *slot = known;
                }
                code = at.checked_add(1);
            }
            _ => {}
        }
    }
}

/// Returns the `'static` spelling of a glyph name, if it is one the specifications list.
///
/// Only names with a static spelling can enter the table, which is what lets it hold
/// `&'static str` and avoid an allocation per code.
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
    names: Option<&'a [&'static str; 256]>,
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
            if let Some(width) = standard.width(glyph_name) {
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
) -> Result<(CodeTable, Box<[&'static str; 256]>), FontError> {
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
        match names.get(code).copied().filter(|n| !n.is_empty()) {
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
