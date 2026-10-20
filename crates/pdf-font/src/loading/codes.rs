//! A character code and the text it stands for: splitting a string into codes, ISO 32000-2
//! §9.10.2's methods for naming a code, the font's `/ToUnicode`, and the inverse (a character's code).

use super::{CodeMapping, LoadedFont, MAX_ADDRESSABLE_CODES, NamingGap};
use crate::cmap::{CMap, Code};
use crate::encoding;
use crate::post::PostNames;
use crate::truetype::invert_charmap;
use skrifa::FontRef;
use skrifa::raw::TableProvider;
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::sync::OnceLock;

impl LoadedFont {
    /// Splits a PDF string into character codes.
    ///
    /// One byte per code for a simple font (§9.7.1: "each byte of a string to be shown selects
    /// one glyph"); for a composite font, whatever its `CMap`'s codespace ranges say, which
    /// may be one to four bytes and may differ from code to code within one string. Getting
    /// this wrong does not merely shift text, it reads entirely different glyphs.
    #[must_use]
    pub fn decode(&self, bytes: &[u8]) -> Vec<Code> {
        let Some(cmap) = self.cmap() else {
            return bytes.iter().copied().map(Code::single_byte).collect();
        };
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

    /// The `CMap` this font splits its codes with, where it has one.
    ///
    /// `None` for every simple font, whose codes are single bytes (§9.6.1); `Some` for every
    /// composite one, whatever became of its program — which is why this is asked of the
    /// mapping rather than of the subtype.
    pub(super) fn cmap(&self) -> Option<&CMap> {
        match &self.mapping {
            CodeMapping::Composite { cmap, .. } | CodeMapping::Substituted { cmap, .. } => {
                Some(cmap)
            }
            CodeMapping::MetricsOnly { cmap } => cmap.as_deref(),
            CodeMapping::Named(_) => None,
        }
    }

    /// Appends the text a character code represents, reporting whether any was found.
    ///
    /// Three sources, in order of authority, and the first two are §9.10.2's own first two
    /// methods. `/ToUnicode` is the producer's own statement of what a code means and is
    /// preferred. Failing that, the glyph name the encoding selects identifies a character
    /// through the Adobe Glyph List — which describes what was actually drawn, and so stays
    /// right even when a producer's `/ToUnicode` is not. The third is not a method of the
    /// clause but the choice it explicitly permits where its methods fail; see
    /// `Self::text_from_program`.
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
            && let Some(cid) = self.cmap().and_then(|cmap| cmap.cid(code))
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
    /// answered with**, because the clause itself ranks them: "\[a] PDF processor can use these
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
            // No method of the clause's was tried, because the font draws nothing and this
            // reader states no character for a mark the page does not carry. A code here has
            // a displacement and no meaning; see [`Self::metrics_only`].
            CodeMapping::MetricsOnly { cmap } => Some(if cmap.is_some() {
                NamingGap::UnaddressableCid
            } else {
                NamingGap::UnnamedGlyph
            }),
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
    /// choose one where its three methods fail, and `Self::text_from_program` takes that
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
    /// measured over the pdf.js corpus rather than assumed (ADR 0067).
    ///
    /// **Two statements the program makes, in that order.** The `post` table names the glyph,
    /// which the Adobe Glyph List turns into a character — the same step §9.10.2's own second
    /// method takes, from a name the file supplies rather than one the encoding chose. Failing
    /// that, the program's Unicode `cmap` subtable is inverted: an entry mapping U+00E4 to
    /// glyph 74 is the font saying that glyph 74 is `ä`, whichever direction it is read in.
    /// `issue15910.pdf` needs the second, because its `post` is version 2.0 with every name an
    /// empty string — a table that satisfies the format and states nothing.
    ///
    /// **A composite font reaches this too** (ADR 0259). §9.10.2's third method does not apply
    /// to every composite font, and the clause says so in its own first line:
    ///
    /// > If the font is a composite font that uses one of the predefined CMaps listed in
    /// > "Table 116 -Predefined CJK CMap names" (except Identity -H and Identity -V ) or whose
    /// > descendant CIDFont uses the Adobe-GB1, Adobe-CNS1, Adobe-Japan1, Adobe-Korea1
    /// > (deprecated in PDF 2.0 (2020)) or Adobe-KR (added in PDF 2.0 (2020)) character
    /// > collection
    ///
    /// An `Identity-H` font whose descendant is `Adobe-Identity` is excluded by name from the
    /// third method and cannot use the second, so a `/ToUnicode` that answers nothing leaves
    /// *every* method failed — which is the precondition of the permission quoted above, and a
    /// refusal would decline it. Three documents in `doc/corpora/pdfbox` are that shape, and a
    /// refusal reads all three back short or blank while reporting nothing:
    /// `PDFBOX-4322-Empty-ToUnicode-reduced.pdf` (a `/ToUnicode` that is a copy of the `Identity-H`
    /// CID `CMap`, so it holds no `bfchar` or `bfrange` at all and §9.10.3 requires those),
    /// `PDFBOX-5838-0024320-reduced.pdf` (a `/ToUnicode` covering 8 of its 15 codes, reading
    /// `H Reeach Pec` for `Honors Research Project`) and `sample_fonts_solidconvertor.pdf` (two
    /// fonts whose `/ToUnicode` is the *name* `/Identity-H`, two whole lines of the page read back
    /// as nothing).
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
            CodeMapping::Substituted { .. } | CodeMapping::MetricsOnly { .. } => None,
        };
        match glyph.and_then(|glyph| self.program_characters().get(&glyph).copied()) {
            Some(character) => {
                out.push(character);
                true
            }
            // §9.10.2's closing permission — "a PDF processor may choose a character code of
            // their choosing" — is a *reader's* choice, and this reader declines it for a font
            // whose glyphs are not on the page: naming those codes would put selectable text
            // under blank paper. See [`Self::metrics_only`].
            None if self.is_metrics_only() => false,
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
            // Resolved once for the face: asking `read-fonts` glyph by glyph would step over
            // every earlier name for each one (ADR 1584).
            let post = PostNames::new(&font);
            let glyphs = font.maxp().map_or(0, |maxp| maxp.num_glyphs());
            for glyph in 0..glyphs {
                if let Some(character) = post
                    .name(glyph)
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

    /// The one character the font's own `/ToUnicode` states this code stands for (§9.10.3).
    ///
    /// `None` for a font carrying no `/ToUnicode`, for a code it does not map, and for one it
    /// maps to more than one character. Only the producer's statement answers here, never
    /// [`Self::text`]'s later methods: a caller asking which character the *file* says a code is
    /// must not be handed this reader's inference (ADR 1942).
    #[must_use]
    pub fn stated_character(&self, code: Code) -> Option<char> {
        self.to_unicode.char_for(code.value())
    }

    /// Whether [`Self::code_for`] can answer for this font at all.
    ///
    /// The distinction is public so a caller can report *which* of the two refusals it hit —
    /// "this font lacks that character" and "this font cannot be addressed by character" are not
    /// the same statement.
    ///
    /// **A composite font answers too** (ADR 0337): a `CMap`'s codespace ranges decide a code's
    /// length (§9.7.6.2), and [`crate::cmap::CMap::each_addressable_code`] inverts them, and the
    /// one case left is a `CMap` stating more codes than that walk will visit.
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
    pub(super) fn addressable_codes(&self) -> Option<&BTreeMap<char, Code>> {
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
        match self.cmap() {
            None => {
                for byte in 0..=u8::MAX {
                    consider(Code::single_byte(byte));
                }
            }
            Some(cmap) => {
                if !cmap.each_addressable_code(MAX_ADDRESSABLE_CODES, &mut consider) {
                    return None;
                }
            }
        }
        Some(map)
    }
}
