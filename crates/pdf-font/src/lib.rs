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

// Retained though not yet on the loading path: it builds the sfnt container a bare CFF
// needs, which is the first half of supporting those fonts. See the refusal in
// `embedded_program` for why the second half must land before either is used.
pub mod cff;

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::sync::Arc;

use pdf_render::{Path, PathCommand, Point};
use pdf_syntax::{Dictionary, Document, Object};
use skrifa::outline::{DrawSettings, OutlinePen};
use skrifa::prelude::{LocationRef, Size};
use skrifa::{FontRef, GlyphId, MetadataProvider};

/// Width assumed when a font gives none, in thousandths of an em.
///
/// Half an em is close to average for Latin text, so spacing degrades gracefully rather
/// than collapsing to zero.
const DEFAULT_WIDTH: f32 = 500.0;

/// How a font maps character codes to glyphs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CodeMapping {
    /// One byte per code, mapped through the font's character map.
    Simple,
    /// Two bytes per code, and the code *is* the glyph index.
    ///
    /// `Identity-H` with an identity `CIDToGIDMap`, which is what almost every modern
    /// producer emits for subset fonts.
    IdentityTwoByte,
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
}

/// A font ready to produce glyph outlines.
pub struct LoadedFont {
    /// The embedded font program, which `FontRef` borrows from on each use.
    data: Arc<[u8]>,
    mapping: CodeMapping,
    /// Glyph advances by character code, in thousandths of an em.
    widths: BTreeMap<u32, f32>,
    /// Advance for a code with no entry.
    default_width: f32,
    units_per_em: f32,
    /// Cached outlines: a page reuses the same few dozen glyphs constantly, and
    /// re-extracting each one would dominate the render.
    outlines: RefCell<BTreeMap<u16, Option<Arc<Path>>>>,
}

impl std::fmt::Debug for LoadedFont {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoadedFont")
            .field("bytes", &self.data.len())
            .field("mapping", &self.mapping)
            .field("units_per_em", &self.units_per_em)
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
        let descriptor = document.get_key(dict, "FontDescriptor");
        let descriptor = descriptor.as_dict().ok_or_else(|| FontError::NotEmbedded {
            name: name.to_owned(),
        })?;
        let data = embedded_program(document, descriptor, name)?;
        let units_per_em = units_per_em(&data, name)?;

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

        let default_width = document
            .get_key(descriptor, "MissingWidth")
            .as_number()
            .map_or(DEFAULT_WIDTH, narrow);

        Ok(Self {
            data,
            mapping: CodeMapping::Simple,
            widths,
            default_width,
            units_per_em,
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

        let descriptor = document.get_key(&descendant, "FontDescriptor");
        let descriptor = descriptor.as_dict().ok_or_else(|| FontError::NotEmbedded {
            name: name.to_owned(),
        })?;
        let data = embedded_program(document, descriptor, name)?;
        let units_per_em = units_per_em(&data, name)?;

        // A `/CIDToGIDMap` stream remaps CIDs to glyphs; without reading it the glyphs
        // would be wrong, so it is refused rather than approximated.
        match document.get_key(&descendant, "CIDToGIDMap") {
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
            mapping: CodeMapping::IdentityTwoByte,
            widths: composite_widths(document, &descendant),
            default_width,
            units_per_em,
            outlines: RefCell::new(BTreeMap::new()),
        })
    }

    /// Splits a PDF string into character codes.
    ///
    /// One byte per code for a simple font, two for an Identity composite font. Getting
    /// this wrong does not merely shift text, it reads entirely different glyphs.
    #[must_use]
    pub fn decode(&self, bytes: &[u8]) -> Vec<u32> {
        match self.mapping {
            CodeMapping::Simple => bytes.iter().map(|&byte| u32::from(byte)).collect(),
            CodeMapping::IdentityTwoByte => bytes
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
    fn glyph_for(&self, code: u32) -> Option<u16> {
        match self.mapping {
            // The code is the glyph index by construction.
            CodeMapping::IdentityTwoByte => u16::try_from(code).ok(),
            CodeMapping::Simple => {
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
                    // order matches the codes.
                    .or_else(|| u16::try_from(code).ok())
            }
        }
    }

    /// Extracts and normalises one glyph outline.
    fn build_outline(&self, glyph: u16) -> Option<Arc<Path>> {
        let font = FontRef::new(&self.data).ok()?;
        let outline = font.outline_glyphs().get(GlyphId::from(glyph))?;

        let mut pen = PathPen {
            path: Path::new(),
            scale: 1.0 / self.units_per_em,
            last: None,
        };
        // Unhinted and unscaled: hinting is a device-resolution decision, and this outline
        // is resolution-independent because the text matrix scales it later.
        outline
            .draw(
                DrawSettings::unhinted(Size::unscaled(), LocationRef::default()),
                &mut pen,
            )
            .ok()?;

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

/// Extracts the embedded font program from a font descriptor.
fn embedded_program(
    document: &Document,
    descriptor: &Dictionary,
    name: &str,
) -> Result<Arc<[u8]>, FontError> {
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

        // `/FontFile3` holds either a full OpenType file or a *bare* CFF font program,
        // distinguished by its `/Subtype`. skrifa reads fonts through an sfnt container, so
        // a bare CFF fails with "invalid sfnt version" — an error about the wrapper that
        // says nothing about the real cause. Detecting it here makes the diagnosis right.
        let subtype = document
            .get_key(&stream.dict, "Subtype")
            .as_name()
            .map(|value| value.as_bytes().to_vec())
            .unwrap_or_default();
        if matches!(subtype.as_slice(), b"Type1C" | b"CIDFontType0C") || is_bare_cff(&data) {
            // `cff::wrap_in_sfnt` builds a container skrifa accepts, and the outlines are
            // then readable — but that is only half the problem, and shipping the half
            // would be worse than shipping neither.
            //
            // A CFF font maps a character code to a glyph through its *charset* (glyph
            // names) combined with the PDF `/Encoding`, not through a `cmap` table. The
            // synthesised container has no `cmap`, so lookup silently falls through to
            // treating the code as a glyph index — which loads successfully, reports
            // nothing unsupported, and draws the wrong glyphs or none at all.
            //
            // Refusing keeps the failure visible until the charset mapping exists. See
            // `cff::wrap_in_sfnt`, which is retained and tested for that work.
            return Err(FontError::UnsupportedProgram {
                name: name.to_owned(),
                kind: "bare CFF (container synthesis works; charset-based code-to-glyph \
                       mapping is still needed)",
            });
        }

        return Ok(data);
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
fn units_per_em(data: &[u8], name: &str) -> Result<f32, FontError> {
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
