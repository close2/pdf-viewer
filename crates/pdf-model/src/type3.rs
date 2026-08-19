//! Fonts whose glyphs are content streams: Type 3, ISO 32000-2 §9.6.4.
//!
//! # Why this is in `pdf-model` and not in `pdf-font`
//!
//! Every other font in PDF names a *program* — `TrueType`, CFF, Type 1 — and `pdf-font`
//! turns a character code into an outline by reading it. A Type 3 font has no program at
//! all. §9.6.4: "In Type 3 fonts, glyphs shall be defined by streams of PDF graphics
//! operators", each associated with a glyph name and stored in the font's `/CharProcs`
//! dictionary. Drawing one therefore means running the content interpreter, which lives
//! here, so `pdf_font::LoadedFont::load` refuses a Type 3 font and this module picks it up.
//!
//! # What is different about a Type 3 glyph
//!
//! Three things, and each of them is a defect if it is missed:
//!
//! - **Glyph space is whatever the font says it is.** §9.2.4 gives every other font a glyph
//!   space of one thousandth of a text-space unit; a Type 3 font states its own through
//!   `/FontMatrix`. A font drawing its glyphs on a 1-unit grid — which real documents do —
//!   is a thousand times larger than the usual convention, so assuming the convention draws
//!   one glyph across the whole page.
//! - **The widths are in that space too.** Table 110 says so explicitly, calling out the
//!   contrast with a Type 1 font's thousandths of text space.
//! - **The encoding is the only mapping there is.** §9.6.5.3: a Type 3 font's mapping from
//!   codes to glyph names "shall be entirely defined by its Encoding entry", and its NOTE
//!   adds that "Type 3 fonts do not support the concept of a default glyph name". A code the
//!   encoding does not name reaches no glyph, and a name absent from `/CharProcs` paints
//!   nothing — neither is an error, and both still advance the text position.
//!
//! The glyph names are *procedure* names and mean nothing outside the font. That is why a
//! Type 3 font can never be substituted, and why doing so was a defect the eighth session
//! removed: `french_diacritics.pdf` names its procedures `/a192`, `/a199`, `/a224`, which
//! are also `ZapfDingbats` glyph names, so a substitute drew dingbats and reported nothing.

use std::collections::BTreeMap;
use std::sync::Arc;

use pdf_render::Transform;
use pdf_syntax::{Dictionary, Document, Name, Object, Stream};

/// Why a Type 3 font could not be drawn.
///
/// Each variant is an entry Table 110 marks *required* and the file does not have. There is
/// deliberately no variant for a code that reaches no glyph: §9.6.4 defines that case as
/// painting nothing, so it is correct behaviour rather than a failure.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Type3Error {
    /// No `/CharProcs`, so the font carries no glyph descriptions at all.
    #[error("font /{name} is a Type 3 font with no /CharProcs dictionary")]
    NoCharProcs {
        /// The resource name, for diagnosis.
        name: String,
    },
    /// No `/FontMatrix`, which is the only statement of how large a glyph is.
    ///
    /// Guessing the common `[0.001 0 0 0.001 0 0]` is available and is not taken: a font
    /// drawing on a 1-unit grid would then be drawn a thousand times too small, silently.
    #[error("font /{name} is a Type 3 font with no /FontMatrix")]
    NoFontMatrix {
        /// The resource name.
        name: String,
    },
    /// No `/Encoding` that names any glyph, so no code reaches a glyph description.
    #[error("font /{name} is a Type 3 font whose /Encoding names no glyph")]
    NoEncoding {
        /// The resource name.
        name: String,
    },
}

/// A Type 3 font: an encoding, a dictionary of glyph procedures, and the matrix that sizes
/// them.
///
/// Holds the `/CharProcs` dictionary rather than the decoded streams because a page uses a
/// handful of the codes a font defines, and decoding the rest would be work for glyphs that
/// are never shown.
#[derive(Debug, Clone)]
pub struct Type3Font {
    /// `/FontMatrix`, which Table 110 defines as "mapping glyph space to text space".
    font_matrix: Transform,
    /// What each code is named, from `/Encoding` (§9.6.5.3).
    ///
    /// A [`Name`] rather than a `String`, because §9.6.4's step b) looks this up in
    /// `/CharProcs` and §7.3.5 makes that lookup a comparison of bytes.
    encoding: BTreeMap<u8, Name>,
    /// The glyph descriptions, keyed by the names the encoding produces.
    char_procs: Dictionary,
    /// `/Widths`, in glyph space, starting at `/FirstChar`.
    widths: Vec<f32>,
    /// `/FirstChar`: the code `widths[0]` describes.
    first_char: i64,
    /// The font's own `/Resources`, if it has any.
    resources: Option<Dictionary>,
    /// `/ToUnicode`, the first of §9.10.2's methods and the producer's own statement.
    ///
    /// **This comment used to say a Type 3 glyph name "names a procedure, so — unlike every
    /// other simple font — the name is no evidence at all about the character".** It names a
    /// procedure *and* a character: §9.6.4's step b) is "[g]et the glyph name from the Encoding
    /// entry", which is the glyph selection algorithm §9.10.2's second method asks about, and a
    /// producer that calls its procedure `/colon` has said which character it draws. See
    /// [`Type3Font::text`] for the two methods that follow this one.
    to_unicode: pdf_font::tounicode::ToUnicode,
}

impl Type3Font {
    /// Reads a Type 3 font dictionary.
    ///
    /// # Errors
    ///
    /// See [`Type3Error`]: one variant per required entry of Table 110 whose absence leaves
    /// nothing to draw.
    pub fn read(document: &Document, dict: &Dictionary, name: &str) -> Result<Self, Type3Error> {
        let font_matrix = matrix(document, dict).ok_or_else(|| Type3Error::NoFontMatrix {
            name: name.to_owned(),
        })?;

        let char_procs = document
            .get_key(dict, "CharProcs")
            .as_dict()
            .cloned()
            .ok_or_else(|| Type3Error::NoCharProcs {
                name: name.to_owned(),
            })?;

        let encoding = encoding(document, dict);
        if encoding.is_empty() {
            return Err(Type3Error::NoEncoding {
                name: name.to_owned(),
            });
        }

        let first_char = document
            .get_key(dict, "FirstChar")
            .as_integer()
            .unwrap_or_default();
        let widths = document
            .get_key(dict, "Widths")
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .map(|item| document.resolve(item))
                    .map(|item| item.as_number().map_or(0.0, narrow))
                    .collect()
            })
            .unwrap_or_default();

        Ok(Self {
            font_matrix,
            encoding,
            char_procs,
            widths,
            first_char,
            resources: document.get_key(dict, "Resources").as_dict().cloned(),
            to_unicode: pdf_font::tounicode::ToUnicode::parse(
                &document
                    .get_key(dict, "ToUnicode")
                    .as_stream()
                    .and_then(|stream| document.decoded_stream_data(stream))
                    .unwrap_or_else(|| Arc::from([])),
            ),
        })
    }

    /// The glyph description a character code selects, if it reaches one.
    ///
    /// §9.6.4's steps a) and b): look the code up in `/Encoding` to obtain a glyph name,
    /// then look that name up in `/CharProcs`. "If the name is not present as a key in
    /// `CharProcs`, no glyph shall be painted" — which is this returning `None`.
    #[must_use]
    pub fn glyph(&self, document: &Document, code: u32) -> Option<Arc<Stream>> {
        let code = u8::try_from(code).ok()?;
        let name = self.encoding.get(&code)?;
        // §7.3.5: two names denote one object only when "the resulting sequences of bytes are
        // … an exact binary match", so the probe carries the encoding's own bytes. A trip
        // through `str` would lose a name a producer wrote with a byte no UTF-8 sequence
        // begins, and would make two such names one (ADR 0438, ADR 0439).
        document
            .get_key_by_name(&self.char_procs, name)
            .as_stream()
            .cloned()
    }

    /// The glyph name §9.6.4 step a) maps a code to, which is what step b) then looks up.
    ///
    /// Separate from [`Self::glyph`] because a report about a glyph description names the
    /// *glyph* rather than the code that reached it: `/CharProcs` is keyed by name, and two
    /// codes may reach one description.
    #[must_use]
    pub fn glyph_name(&self, code: u32) -> Option<&Name> {
        let code = u8::try_from(code).ok()?;
        self.encoding.get(&code)
    }

    /// A code's advance width, in text-space units where one em is 1.0.
    ///
    /// Table 110 on `/Widths`: the values "shall be interpreted in glyph space as specified
    /// by `FontMatrix` (unlike the widths of a Type 1 font, which are in thousandths of a unit
    /// of text space)", and "if `FontMatrix` specifies a rotation, only the horizontal
    /// component of the transformed width shall be used". A width is a displacement rather
    /// than a point, so only the matrix's linear part applies to it, and taking the
    /// horizontal component of that leaves the `a` coefficient alone.
    #[must_use]
    pub fn advance(&self, code: u32) -> f32 {
        // "For character codes outside the range FirstChar to LastChar, the width shall be
        // 0" — Table 110, which is why nothing here consults `/MissingWidth`.
        let width = i64::from(code)
            .checked_sub(self.first_char)
            .and_then(|index| usize::try_from(index).ok())
            .and_then(|index| self.widths.get(index))
            .copied()
            .unwrap_or(0.0);
        self.font_matrix.a * width
    }

    /// The matrix mapping this font's glyph space to text space.
    #[must_use]
    pub fn font_matrix(&self) -> Transform {
        self.font_matrix
    }

    /// The resources a glyph description's operators name, in the order §7.8.3 searches.
    ///
    /// **Errata Collection 3 rewrote this rule and added a step in front of it** — Issue #128,
    /// `/State` `Review` `Completed`, on §7.8.3 and on §9.6.4 together, and invisible to
    /// `doc/md/` because the sponsored copy records EC3 as review markup that the conversion
    /// dropped (ADR 0252, ADR 0253). §9.6.4's step d) used to read "shall be looked up in the
    /// Resources entry of the Type 3 font dictionary. If any glyph descriptions refer to named
    /// resources but this dictionary is absent, the names shall be looked up in the resource
    /// dictionary of the page on which the font is used", and it now points at §7.8.3, which
    /// states a four-step search: the glyph description content **stream's own dictionary**
    /// first, then the Type 3 font dictionary that held the `/CharProcs` entry, and only if
    /// neither states one, the page and what the page inherits (§7.7.3.4).
    ///
    /// So `glyph` is the first step and it is the one this tree did not have — a glyph
    /// description that carried its own `/Resources` was read against the font's or the page's,
    /// which is a different dictionary whenever the two disagree. The last two steps arrive
    /// together in `page`, because the caller's resource dictionary is already the inherited
    /// one.
    #[must_use]
    pub fn resources<'a>(
        &'a self,
        glyph: Option<&'a Dictionary>,
        page: &'a Dictionary,
    ) -> &'a Dictionary {
        glyph.or(self.resources.as_ref()).unwrap_or(page)
    }

    /// Appends what a character code means, reporting whether anything was found.
    ///
    /// Only `/ToUnicode` can answer: see the field's own comment for why the glyph name
    /// cannot.
    pub fn text(&self, code: u32, out: &mut String) -> bool {
        if self.to_unicode.append(code, out) {
            return true;
        }
        // §9.10.2's **second** method, which applies here and which this module refused for
        // three hundred sessions: "[i]f the font is a simple font and the glyph selection
        // algorithm (see 9.6.5, "Character encoding") uses a glyph name, that name can be looked
        // up in the Adobe Glyph List … to obtain the corresponding Unicode value". A Type 3 font
        // is a simple font, and §9.6.4's own step b) is a name: "[g]et the glyph name from the
        // Encoding entry"; §9.6.5.3 makes `/Differences` "the complete character encoding for
        // this font". So a name *is* used, and where it is a name the Adobe Glyph List knows,
        // the clause says what it means.
        //
        // What the field comment beside `to_unicode` used to say — that a Type 3 glyph name
        // "names a procedure, so … the name is no evidence at all about the character" — is
        // true of the procedure and not of the name: a producer that calls its procedure
        // `/colon` has said which character it draws, in the same table the clause sends every
        // other simple font to. `pr4922.pdf`'s Type 3 font names its twenty glyphs `/P`, `/a`,
        // `/colon`, `/five`, `/hyphen`, `/slash` and so on, and nothing in it is invented.
        if let Some(text) = self
            .encoding
            .get(&u8::try_from(code).unwrap_or(0))
            .and_then(|name| pdf_font::encoding::text_for(name.as_str()?))
        {
            out.push_str(&text);
            return true;
        }
        Self::text_from_the_code(code, out)
    }

    /// Which of §9.10.2's methods could have named a code and did not, or `None` where one did.
    ///
    /// The Type 3 half of [`pdf_font::LoadedFont::naming_gap`], whose documentation states the
    /// rule both follow: the answer is the highest-priority method the clause states that this
    /// font could have answered with, because the clause ranks them itself. Only two of them
    /// can apply here — a Type 3 font is a simple font, so the composite route is out by the
    /// clause's own first sentence, and §9.6.5.3 leaves its `/Encoding` as the whole of its
    /// glyph selection.
    #[must_use]
    pub fn naming_gap(&self, code: u32) -> Option<pdf_font::NamingGap> {
        let mut discarded = String::new();
        if self.text(code, &mut discarded) {
            return discarded
                .is_empty()
                .then_some(pdf_font::NamingGap::EmptyMapping);
        }
        if !self.to_unicode.is_empty() {
            return Some(pdf_font::NamingGap::IncompleteToUnicode);
        }
        match u8::try_from(code)
            .ok()
            .and_then(|code| self.encoding.get(&code))
        {
            // The gap is a *report*, which is §7.3.5's own "occasionally the need arises to
            // treat a name object as text" and the one place the lossy spelling belongs.
            Some(name) => Some(pdf_font::NamingGap::UnlistedName(
                String::from_utf8_lossy(name.as_bytes()).into_owned(),
            )),
            // §9.6.5.3 makes `/Differences` "the complete character encoding for this font" and
            // its NOTE adds that "Type 3 fonts do not support the concept of a default glyph
            // name", so a code the encoding does not name has no name anywhere — and no glyph
            // either.
            None => Some(pdf_font::NamingGap::UnnamedGlyph),
        }
    }

    /// §9.10.2's last resort, where the name is one the Adobe Glyph List does not know.
    ///
    /// **A choice and not a reading**, and the clause states the licence in the same sentence
    /// that states the outcome:
    ///
    /// > If these methods fail to produce a Unicode value, there is no way to determine what the
    /// > character code represents in which case a PDF processor may choose a character code of
    /// > their choosing.
    ///
    /// The choice is the **code itself**, and only where it is a printable ASCII byte. What
    /// makes it worth taking is a population the corpus names: `dvips` writes a Type 3 font
    /// whose `/Differences` calls every glyph `aNN` after its own code — a name the Adobe Glyph
    /// List cannot resolve — so a whole page of English read back as nothing at all.
    /// `issue918.pdf` is 193 words of it.
    ///
    /// **The bound is what keeps this from being the fallback-that-fills-the-page this project
    /// forbids.** 0x21 to 0x7E is the range in which a byte and a Unicode code point mean the
    /// same character under every encoding §9.6.5 states — `Standard`, `WinAnsi`, `MacRoman`
    /// and `PDFDoc` agree there and differ everywhere else — so a code outside it is one this declines
    /// rather than guesses at. Space is excluded because a readback of whitespace is what
    /// `Interpretation` uses to tell a missing mark from a blank one.
    fn text_from_the_code(code: u32, out: &mut String) -> bool {
        let Ok(byte) = u8::try_from(code) else {
            return false;
        };
        if !(0x21..=0x7E).contains(&byte) {
            return false;
        }
        out.push(char::from(byte));
        true
    }
}

/// Reads `/FontMatrix`, which Table 110 requires and this does not invent.
fn matrix(document: &Document, dict: &Dictionary) -> Option<Transform> {
    let entry = document.get_key(dict, "FontMatrix");
    let values: Vec<f32> = entry
        .as_array()?
        .iter()
        .map(|item| document.resolve(item))
        .filter_map(|item| item.as_number())
        .map(narrow)
        .collect();
    (values.len() >= 6).then(|| {
        Transform::new(
            values[0], values[1], values[2], values[3], values[4], values[5],
        )
    })
}

/// Builds the code-to-glyph-name table from the font's `/Encoding` entry.
///
/// §9.6.5.3 makes the entry the whole mapping, and Table 110 says its `/Differences` array
/// "shall specify the complete character encoding for this font". `/BaseEncoding` is read
/// underneath it because Table 112 defines that entry for any encoding dictionary and a
/// handful of producers use it here — but there is no *default* base: §9.6.5.1 excepts Type
/// 3 fonts from having a built-in encoding at all, so a code neither `/BaseEncoding` nor
/// `/Differences` names reaches nothing.
fn encoding(document: &Document, dict: &Dictionary) -> BTreeMap<u8, Name> {
    let mut table = BTreeMap::new();
    let entry = document.get_key(dict, "Encoding");
    let Some(encoding) = entry.as_dict() else {
        return table;
    };

    if let Some(base) = document
        .get_key(encoding, "BaseEncoding")
        .as_name()
        .and_then(|name| pdf_font::encoding::BaseEncoding::by_name(name.as_bytes()))
    {
        for code in 0..=u8::MAX {
            let name = base.glyph_name(code);
            if !name.is_empty() {
                table.insert(code, Name::new(name.as_bytes()));
            }
        }
    }

    // §9.6.5.1: "Each code shall be the first index in a sequence of character codes to be
    // changed. The first character name after the code becomes the name corresponding to
    // that code. Subsequent names replace consecutive code indices until the next code
    // appears in the array or the array ends."
    let differences = document.get_key(encoding, "Differences");
    let Some(items) = differences.as_array() else {
        return table;
    };
    let mut code: Option<u8> = None;
    for item in items {
        match document.resolve(item) {
            Object::Integer(value) => code = u8::try_from(value).ok(),
            Object::Name(glyph) => {
                let Some(at) = code else { continue };
                table.insert(at, glyph.clone());
                code = at.checked_add(1);
            }
            _ => {}
        }
    }
    table
}

/// Narrows a PDF number to the precision the display list is in.
fn narrow(value: f64) -> f32 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a font matrix or width outside f32's range is not usable as one"
    )]
    {
        value as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `/Widths` are in glyph space, so `/FontMatrix` decides what they mean.
    ///
    /// The case this pins is the one that draws a glyph a thousand times too wide: a font
    /// stating `/FontMatrix [1 0 0 1 0 0]` and `/Widths [1]` advances by one em, and the
    /// same width under the conventional `[0.001 …]` matrix advances by a thousandth.
    #[test]
    fn a_width_is_read_through_the_font_matrix() {
        let unit = Type3Font {
            font_matrix: Transform::new(1.0, 0.0, 0.0, 1.0, 0.0, 0.0),
            encoding: BTreeMap::new(),
            char_procs: Dictionary::default(),
            widths: vec![1.0],
            first_char: 97,
            resources: None,
            to_unicode: pdf_font::tounicode::ToUnicode::parse(&[]),
        };
        let thousandths = Type3Font {
            font_matrix: Transform::new(0.001, 0.0, 0.0, 0.001, 0.0, 0.0),
            ..unit.clone()
        };

        assert!((unit.advance(97) - 1.0).abs() < f32::EPSILON);
        assert!((thousandths.advance(97) - 0.001).abs() < f32::EPSILON);
        // Table 110: outside `/FirstChar`..`/LastChar` the width is zero, not the
        // descriptor's `/MissingWidth`.
        assert!((unit.advance(98) - 0.0).abs() < f32::EPSILON);
        assert!((unit.advance(96) - 0.0).abs() < f32::EPSILON);
    }
}
