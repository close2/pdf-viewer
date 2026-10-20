//! A font whose glyphs stand in for a face the document did not embed: whether it does, what
//! substitutes (§9.6.5.4, §9.8.3.3's classes) and how a code is answered by the stand-in.

use super::{ClassFace, CodeMapping, Face, LoadedFont, Placed};
use crate::cmap::{CMap, Code};
use crate::vertical::{Form, VerticalForms};
use skrifa::{FontRef, MetadataProvider};

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
    pub(super) fn substitutes_notdef(&self, code: Code) -> bool {
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
        let CodeMapping::Substituted { cmap, downward } = &self.mapping else {
            return None;
        };
        let character = self.substituted_character(cmap, code)?;
        // §9.8.3.3: "[t]he entry's value shall be a font descriptor whose contents shall override
        // the font-wide attributes for that class only", and the attribute this tree acts on is
        // which face stands in. So where Table 123 shows this character to be in one of the
        // classes `/FD` names, that class's face draws it; every other character keeps the
        // font's own. [`crate::glyph_class`] holds what "shows" means and what it refuses.
        let (face, data, downward) = match self.class_face(character) {
            Some((index, class)) => (Face::Class(index), &class.data, &class.downward),
            None => (Face::Own, &self.data, downward),
        };
        let font = FontRef::new(data).ok()?;
        let glyph = u16::try_from(font.charmap().map(character)?.to_u32()).ok()?;
        let form = downward.as_ref().map_or(Form::Upright, |downward| {
            cmap.cid(code)
                .map_or(Form::Upright, |cid| downward.form_of(character, cid, glyph))
        });
        Some((character, Placed { face, glyph }, form))
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
    ///
    /// [`FontError`]: crate::FontError
    #[must_use]
    pub fn uncovered_character(&self, code: Code) -> Option<char> {
        let CodeMapping::Substituted { cmap, .. } = &self.mapping else {
            return None;
        };
        let character = self.substituted_character(cmap, code)?;
        // The face that would draw it, which for a character in one of §9.8.3.3's classes is
        // that class's rather than the font's own.
        let data = match self.class_face(character) {
            Some((_, class)) => &class.data,
            None => &self.data,
        };
        let font = FontRef::new(data).ok()?;
        font.charmap().map(character).is_none().then_some(character)
    }
}
