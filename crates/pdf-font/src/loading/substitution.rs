//! A font whose glyphs stand in for a face the document did not embed: whether it does, what
//! substitutes (§9.6.5.4, §9.8.3.3's classes) and how a code is answered by the stand-in.

use super::{ClassFace, CodeMapping, Face, LoadedFont, Placed};
use crate::cmap::{CMap, Code};
use crate::vertical::{Downward, Form, VerticalForms};
use skrifa::{FontRef, MetadataProvider};
use std::sync::Arc;

impl LoadedFont {
    /// The glyph classes ISO 32000-2 §9.8.3.3's `/FD` named and this reader declined to apply,
    /// each with the sentence that says why.
    ///
    /// Empty for every font that states no `/FD` and for one whose every class was applied. A
    /// name here is a *refusal* and not a failure: the class's glyphs keep the main descriptor,
    /// which is the font the page had before the entry was read, and "never guess a class" is
    /// what the list records having obeyed. `crate::glyph_class` states the five kinds of
    /// refusal and the clause behind each.
    ///
    /// **A list rather than a report.** What it says is about this machine's catalogue and this
    /// reader's reading of Table 123 rather than about the file, which is ADR 0152's distinction
    /// and the same one `unsupplied_vertical_form` is a count for: a report here would take a
    /// page off the oracle's judged set for a statement about a face.
    #[must_use]
    pub fn refused_glyph_classes(&self) -> Vec<(&str, &'static str)> {
        self.class_of
            .refused()
            .iter()
            .map(|(name, reason)| (name.as_str(), *reason))
            .collect()
    }

    /// Whether these glyphs stand in for a font the document did not embed.
    ///
    /// The shapes are then not the ones the producer chose, though the metrics still are
    /// wherever the document stated them.
    #[must_use]
    pub fn is_substituted(&self) -> bool {
        self.substituted
    }

    /// The program of the face that stands in, where the glyphs are a substitute; `None` for a
    /// font whose program the document embedded.
    ///
    /// The face's own tables — its `OS/2` weight class and width class, its italic bit, its `post`
    /// fixed-pitch flag — are what the stand-in *is*, and this is how a test or a census asks
    /// whether [`crate::substitute`] chose it by what Table 120 describes (ADR 1441). The bytes
    /// are an `sfnt` from the machine or one of §9.6.2.2's compiled-in faces
    /// ([`crate::standard::face`]).
    #[must_use]
    pub fn substitute_program(&self) -> Option<&[u8]> {
        self.substituted.then_some(&*self.data)
    }

    /// Whether the face this font draws from states vertical forms of its own glyphs.
    ///
    /// A question about the **face** and about nothing else: it reads the program's `GSUB` for
    /// the OpenType `vert` and `vrt2` features, whatever the font's writing mode is and whether
    /// or not the glyphs are a substitute. See `crate::vertical` for what those features are
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

    /// Whether §9.6.5.2's substitution applies to `code`.
    ///
    /// One condition, a documented departure, because every code of a name-keyed program names a
    /// glyph.
    ///
    /// **The clause's own**: the encoding "maps to a character name" the program lacks. A code
    /// whose slot no encoding assigned names `.notdef` (ADR 2956). The Adobe Type 1 Font Format, a
    /// normative reference, has a special encoding's assignments "performed as shown in the
    /// example in section 2.3", whose array is filled with `0 1 255 {1 index exch /.notdef put }
    /// for` before them, and its `BuildChar` takes a name from that array for every code. So such
    /// a code draws `.notdef` too, through the same step: there it is the slot's own name rather
    /// than a substitute, and §9.6.5.2 leaves what it looks like to the designer.
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
    pub(super) fn substitutes_notdef(&self, code: Code) -> bool {
        if self.glyph_names.is_none() {
            return false;
        }
        let mut meaning = String::new();
        let whitespace = self.text(code, &mut meaning)
            && !meaning.is_empty()
            && meaning.chars().all(char::is_whitespace);
        !whitespace
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
    pub(super) fn substituted_glyph(&self, code: Code) -> Option<(char, Placed, Form)> {
        let CodeMapping::Substituted {
            cmap,
            downward,
            borrowing,
        } = &self.mapping
        else {
            return None;
        };
        let character = self.substituted_character(cmap, code)?;
        // §9.8.3.3, as Issue #5 corrects it (published on pdf-issues.pdfa.org): the entry's value
        // is a font descriptor whose contents are a subset of Table 120's keys that override the
        // font-wide attributes for that class only, and the attribute this tree acts on is
        // which face stands in. So where Table 123 shows this character to be in one of the
        // classes `/FD` names, that class's face draws it; every other character keeps the
        // font's own. [`crate::glyph_class`] holds what "shows" means and what it refuses.
        let (face, data, downward) = match self.class_face(character) {
            Some((index, class)) => (Face::Class(index), &class.data, &class.downward),
            None => (Face::Own, &self.data, downward),
        };
        let form_in = |downward: Option<&Downward>, glyph: u16| {
            downward.map_or(Form::Upright, |downward| {
                cmap.cid(code)
                    .map_or(Form::Upright, |cid| downward.form_of(character, cid, glyph))
            })
        };
        // not a repair: the face is the stand-in this machine supplies, not a program the
        //   document embeds; one that will not read declines (None).
        let font = FontRef::new(data).ok()?;
        if let Some(glyph) = font.charmap().map(character) {
            // not a repair: a width conversion that declines where the value does not fit; nothing
            // is substituted.
            let glyph = u16::try_from(glyph.to_u32()).ok()?;
            return Some((
                character,
                Placed { face, glyph },
                form_in(downward.as_deref(), glyph),
            ));
        }
        // §9.10.2 made the code this character, and the face chosen for the script does not draw
        // it: the face this machine offers for the character itself does (ADR 3067).
        let (index, borrowed) = borrowing.face_for(character, self.own_downward())?;
        // not a repair: the face is the stand-in this machine supplies, not a program the
        //   document embeds; one that will not read declines (None).
        let font = FontRef::new(&borrowed.data).ok()?;
        // not a repair: a width conversion that declines where the value does not fit; nothing is
        // substituted.
        let glyph = u16::try_from(font.charmap().map(character)?.to_u32()).ok()?;
        Some((
            character,
            Placed {
                face: Face::Borrowed(index),
                glyph,
            },
            form_in(borrowed.downward.as_deref(), glyph),
        ))
    }

    /// The font's own vertical route, which a borrowed face reads again from its own tables.
    fn own_downward(&self) -> Option<&Downward> {
        match &self.mapping {
            CodeMapping::Substituted { downward, .. } => downward.as_deref(),
            _ => None,
        }
    }

    /// The face [`Face::Borrowed`] names, for the outline reader.
    pub(super) fn borrowed_face(&self, index: usize) -> Option<Arc<ClassFace>> {
        match &self.mapping {
            CodeMapping::Substituted { borrowing, .. } => borrowing.face(index),
            _ => None,
        }
    }

    /// §9.8.3.3's class face for a character, where one of `/FD`'s classes holds it.
    ///
    /// `None` for every font with no `/FD`, for a character Table 123 puts in none of the classes
    /// the file names, for a class [`crate::glyph_class::Decision`] refused, and for one this
    /// machine offered no face for. In all five the font's own face draws the glyph, which is
    /// what it drew before the entry was read.
    fn class_face(&self, character: char) -> Option<(usize, &ClassFace)> {
        let index = self.class_of.class_for(character)?;
        Some((index, self.classes.get(index)?.as_ref()?))
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
    /// two tables as [`Self::text`]'s first two branches, which is the point: a route that took
    /// one table for the whole font while the readback took both per code would let a page name
    /// a character it did not draw (ADR 1002).
    ///
    /// One character rather than a string because this is what *substitution* needs: a
    /// substitute face is addressed by character, so a code standing for a cluster has no glyph
    /// to look up.
    fn substituted_character(&self, cmap: &CMap, code: Code) -> Option<char> {
        if self.to_unicode.states(code) {
            return self.to_unicode.char_for(code);
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

    /// What the file states for a code of a substituted font that reaches no glyph by any route
    /// a clause gives a substitute — the code's marks are then lost on every machine, whatever
    /// face it offers.
    ///
    /// A substitute is no program the document named, so a code reaches it only through what
    /// the code *means*. For a simple font that is the glyph name the encoding gives it
    /// (§9.6.5.1), which §9.6.5.4 takes through the Adobe Glyph List to a character and, where
    /// "the glyph name cannot be mapped as specified", to the face's own `post` table; a bare CFF
    /// face keys its glyphs by that name directly (§9.6.5.2). `/ToUnicode` selects no glyph of a
    /// simple font: §9.10.2 states it for extracting text, and §9.6.5's routes run from the
    /// encoding (ADR 2880).
    ///
    /// `Some` only where that route ends before any character or name the face could answer: a
    /// code that the encoding names nothing, or names with a name that is no character under the
    /// Adobe Glyph List and no glyph of the face. A code the face lacks only *here* — a character
    /// the machine's face does not draw — is [`Self::uncovered_character`]'s, a statement about
    /// the machine rather than the file (ADR 0152). `None` for an embedded program, for a code
    /// that reached a glyph, for a code the encoding sends to `.notdef`, which names the
    /// producer's own blank, and for every code of a composite font, which reaches its substitute
    /// by the character §9.10.2 gives it (§9.7.4.2): a code with none names no mark either.
    #[must_use]
    pub fn unreached_in_substitute(&self, code: Code) -> Option<UnreachedCode> {
        if !self.substituted || self.glyph_for(code).is_some() {
            return None;
        }
        match &self.mapping {
            CodeMapping::Named(_) => {
                // not a repair: a width conversion that declines where the value does not fit;
                // nothing is substituted.
                let index = usize::try_from(code.value()).ok()?;
                let name = self
                    .glyph_names
                    .as_ref()
                    .and_then(|names| names.get(index))
                    .map(AsRef::as_ref)
                    .filter(|name| !name.is_empty());
                match name {
                    None => Some(UnreachedCode::NoGlyphName),
                    Some(".notdef") => None,
                    Some(name) => read_fonts::ps::agl::name_to_char(name)
                        .is_none()
                        .then(|| UnreachedCode::GlyphName(name.to_owned())),
                }
            }
            // A composite font's code reaches its substitute by the character §9.10.2 gives it,
            // so a code with no character is one that names no mark either, and a code with one
            // the face lacks is the machine's (ADR 0152).
            CodeMapping::Substituted { .. }
            | CodeMapping::Composite { .. }
            | CodeMapping::MetricsOnly { .. } => None,
        }
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
    ///
    /// [`FontError`]: crate::FontError
    #[must_use]
    pub fn uncovered_character(&self, code: Code) -> Option<char> {
        let CodeMapping::Substituted {
            cmap, borrowing, ..
        } = &self.mapping
        else {
            return None;
        };
        let character = self.substituted_character(cmap, code)?;
        // The face that would draw it, which for a character in one of §9.8.3.3's classes is
        // that class's rather than the font's own.
        let data = match self.class_face(character) {
            Some((_, class)) => &class.data,
            None => &self.data,
        };
        // not a repair: the face is the stand-in this machine supplies, not a program the
        //   document embeds; one that will not read declines (None).
        let font = FontRef::new(data).ok()?;
        // And where that face does not, the face borrowed for the character alone (ADR 3067):
        // the character is uncovered only where this machine offers no face that draws it.
        let drawn = font.charmap().map(character).is_some()
            || borrowing.face_for(character, self.own_downward()).is_some();
        (!drawn).then_some(character)
    }
}

/// What a substituted font's file states for a code that reaches no glyph on any machine
/// ([`LoadedFont::unreached_in_substitute`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnreachedCode {
    /// The encoding names this glyph (§9.6.5.1), and the name is neither a character under the
    /// Adobe Glyph List nor a glyph of the face (§9.6.5.4's `post` table, §9.6.5.2's names).
    GlyphName(String),
    /// The encoding gives the code no glyph name at all.
    NoGlyphName,
}
