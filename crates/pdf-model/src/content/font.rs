//! Selecting and loading fonts: `Tf`, Table 57's `/Font`, and §9.6.2.2's standard fourteen.
//!
//! What a loaded [`Font`] *is* — an outline program or a Type 3 content stream — is decided
//! here once and cached by the object's identity; showing its glyphs is [`super::text`]'s.

use std::rc::Rc;

use pdf_font::Code;
use pdf_syntax::{Dictionary, Name, Object, ObjectId};

use super::report::Unsupported;
use super::run::narrow;
use super::{GraphicsState, Interpreter};

/// The current font, which is one of the two kinds PDF has.
///
/// They differ in what a glyph *is*. Every font with a program hands out an outline, and the
/// interpreter fills it. A Type 3 font hands out a content stream, and the interpreter runs
/// it — see `crate::type3` for why that puts the two kinds in different crates.
#[derive(Debug, Clone)]
pub(super) enum Font {
    /// A font with a glyph program, read by `pdf-font`.
    Program(Rc<pdf_font::LoadedFont>),
    /// A Type 3 font, whose glyphs are content streams (§9.6.4).
    Type3(Rc<crate::type3::Type3Font>),
}

impl Font {
    /// Whether this font is shown in §9.2.4's writing mode 1.
    ///
    /// A Type 3 font is a *simple* font and §9.2.4 confines a second set of metrics to
    /// composite ones, so it is never vertical.
    pub(super) fn is_vertical(&self) -> bool {
        match self {
            Self::Program(font) => font.is_vertical(),
            Self::Type3(_) => false,
        }
    }

    /// Splits a PDF string into character codes.
    ///
    /// A Type 3 font is a simple font — Table 110 gives it `/FirstChar` and `/LastChar`,
    /// which are byte codes — so one byte is one code, always.
    pub(super) fn decode(&self, bytes: &[u8]) -> Vec<Code> {
        match self {
            Self::Program(font) => font.decode(bytes),
            Self::Type3(_) => bytes.iter().copied().map(Code::single_byte).collect(),
        }
    }

    /// A code's advance in text-space units, where one em is 1.0.
    pub(super) fn advance(&self, code: Code) -> f32 {
        match self {
            Self::Program(font) => font.advance(code),
            Self::Type3(font) => font.advance(code.value()),
        }
    }

    /// Appends what a code means to the page's extracted text.
    ///
    /// §9.10.2's methods first, and where every one of them has declined, the one code the
    /// standard names a character for outside that clause. §9.3.3 states it twice, and the
    /// first sentence is the naming:
    ///
    /// > Word spacing works the same way as character spacing but shall apply only to the
    /// > ASCII SPACE character (20h).
    ///
    /// > Word spacing shall be applied to every occurrence of the single-byte character code
    /// > 32 in a string when using a simple font (including Type 3) or a composite font that
    /// > defines code 32 as a single-byte code.
    ///
    /// Read together those say that a single-byte code 32 in a show string **is** the ASCII
    /// SPACE character — the clause identifies the code with the character in order to say
    /// which glyph `Tw` applies to, and identifying them is what it does. So a font whose
    /// encoding, `/ToUnicode` and program all decline to say what such a code means has not
    /// contradicted the clause; it has said nothing, and the clause has already said it.
    ///
    /// **This is last, not first**, because §9.10.2's methods are the producer's own
    /// statements about a code and this is the standard's about the encoding. A
    /// `/Differences` naming code 32 `/bullet`, or a `/ToUnicode` mapping it to U+2019, is
    /// answered by the earlier method and never reaches here.
    ///
    /// It is what [`pdf_font::LoadedFont`]'s own last resort excludes: §9.10.2's closing
    /// permission is taken there for 0x21 to 0x7E only, because reading a code *as* its byte
    /// is a choice about a producer's convention. This one is not that choice. Two corpus
    /// documents show the difference — `issue4304.pdf` is 895 bytes named after it, a
    /// `/Times-Roman` whose `/Differences` maps 32 to `/.notdef`, drawing
    /// *Words that should have spaces between them.* since the four-hundred-and-fifth session
    /// fixed its advances and reading back `Wordsthatshouldhavespacesbetweenthem.` until this;
    /// and `Type3WordSpacing.pdf`, whose Type 3 font names no glyph at code 32 at all and
    /// whose six lines are drawn with `Tw` from 50 down to 0.
    pub(super) fn text(&self, code: Code, out: &mut String) -> bool {
        if match self {
            Self::Program(font) => font.text(code, out),
            Self::Type3(font) => font.text(code.value(), out),
        } {
            return true;
        }
        if code.takes_word_spacing() {
            out.push(' ');
            return true;
        }
        false
    }

    /// Which of §9.10.2's methods could have named a code and did not, for a code that read
    /// back as nothing.
    ///
    /// Both kinds of font answer it — a page mixes them freely — and what differs is which of the
    /// clause's methods could have applied, which is the question the answer is about. §9.3.3's
    /// code 32 is not consulted here: this is asked only where [`Self::text`] has already
    /// declined, and that rule is inside it.
    pub(super) fn naming_gap(&self, code: Code) -> Option<pdf_font::NamingGap> {
        match self {
            Self::Program(font) => font.naming_gap(code),
            Self::Type3(font) => font.naming_gap(code.value()),
        }
    }
}

/// What a loaded font is remembered by.
///
/// Two things select a font and they do not select it the same way, which is §8.4.1 NOTE 1's
/// "either way" with a twist: `Tf` names a resource, and §8.4.5's Table 57 `/Font` is an
/// array whose first element "shall be an indirect object reference instead of a resource
/// name". Both are cached, and a document that reaches one font both ways loads it twice —
/// which costs one parse and is the price of not pretending a name and an object identity are
/// the same key.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum FontKey {
    /// A font dictionary, by the object it is.
    ///
    /// The only kind, since the hundred-and-twenty-seventh session: keying by the resource
    /// *name* conflated a page's `/F1` with a form `XObject`'s. Kept as an enum of one because
    /// the two routes to a font — `Tf`'s resource name and Table 57's `/Font`, which §8.4.1's
    /// NOTE 1 makes alternatives — arrive here differently and only this says they are the
    /// same thing when they name the same object.
    Referenced(ObjectId),
}

/// The font a `Tf` names when the resource dictionary defines nothing under that name.
///
/// **A documented choice about a malformed file, and a narrow one.** §7.8.3 requires the writer
/// to supply the resources a stream uses — "a PDF writer shall include a Resources entry in the
/// stream's dictionary specifying the resource dictionary which contains all the resources used
/// by that content stream" — and a file that names `/F1` with nothing behind it has broken that
/// `shall`; nothing here invents a font for it, and the report stands.
///
/// The exception is the fourteen names §9.6.2.2 lists, because for those the standard states what
/// the name means: Table 109 makes `/FirstChar`, `/LastChar`, `/Widths` and `/FontDescriptor`
/// "(Required; optional in PDF 1.0-1.7 for the standard 14 fonts)", so a file may name one and
/// say nothing else about it. The clause used to add that the fonts "shall be available to the
/// PDF processor", and Errata Collection 3 struck that sentence and made its neighbour a NOTE
/// (Issue #47 and #48; [`pdf_font::standard`] carries the reading and ADR 0253 the reason
/// `doc/md/` cannot show it) — which leaves the permission where the work is anyway.
///
/// So a stream whose `Tf` says `/Helvetica` with an empty resource dictionary has named something
/// the standard permits it to name and nothing else, and drawing it from the compiled-in fourteen
/// (ADR 0133) is a better reading of that stream than drawing nothing. `issue17492.pdf` is the
/// witness: a text widget's stored appearance stream carries `/Resources <<>>` and sets its text
/// in `/Helvetica 12 Tf`, `mupdf` and `ghostscript` draw the three lines, `poppler` refuses with
/// *Unknown font tag 'Helvetica'*, and this tree drew nothing and said so.
///
/// **The same argument `variable_text`'s `STANDARD_ABBREVIATIONS` makes**, one clause over and
/// with a stronger premise: there the name is a four-letter convention for one of the fourteen,
/// here it *is* one of the fourteen. `pdf_font::standard::is_standard_name` is deliberately exact
/// — no case folding, no families — so `/F1`, `/Arial` and `/helvetica` still name nothing and
/// still report. Two corpus documents naming `/F1` are unaffected, which is the narrowness
/// visible in the gate rather than argued in a comment.
fn standard_font_named(name: &str) -> Option<Object> {
    if !pdf_font::standard::is_standard_name(name) {
        return None;
    }
    let entry = |key: &str, value: &str| {
        (
            Name::new(key.as_bytes().to_vec()),
            Object::Name(Name::new(value.as_bytes().to_vec())),
        )
    };
    let mut dict = Dictionary::new();
    // The dictionary §9.6.2.2 allows for one of the fourteen: no `/FirstChar`, `/LastChar`,
    // `/Widths` or `/FontDescriptor`, which the same clause makes optional for these and only
    // these, so `pdf-font` reads the metrics from `standard_metrics` and the program from
    // `standard`.
    for (key, value) in [
        entry("Type", "Font"),
        entry("Subtype", "Type1"),
        entry("BaseFont", name),
    ] {
        dict.insert(key, value);
    }
    Some(Object::Dictionary(dict))
}

impl Interpreter<'_> {
    /// Table 57's `/Font`, which is §8.4.5's other route to the two parameters `Tf` sets.
    ///
    /// > An array of the form [ font size ], where font shall be an indirect reference to a
    /// > font dictionary and size shall be a number expressed in text space units. These two
    /// > objects correspond to the operands of the Tf operator (see 9.3, "Text state
    /// > parameters and operators"); however, the first operand shall be an indirect object
    /// > reference instead of a resource name.
    ///
    /// So both text state parameters are set, exactly as `Tf` sets them, and the font is
    /// cached by the object it *is* rather than by a name it has none of. That last point is
    /// the whole reason this took twenty-four sessions: the font cache was keyed by resource
    /// name, so there was nowhere to put a font that has none, and `extgstate.pdf` — whose
    /// page says "I should be courier!" — was reported rather than drawn.
    pub(super) fn apply_ext_gstate_font(&mut self, dict: &Dictionary, state: &mut GraphicsState) {
        let entry = self.document.get_key(dict, "Font");
        let Some(entry) = entry.as_array() else {
            return;
        };
        let reference = entry.first().cloned();
        let size = entry
            .get(1)
            .map(|item| self.document.resolve(item))
            .and_then(|item| item.as_number());
        if let (Some(Object::Reference(id)), Some(size)) = (reference, size) {
            let font_dict = self.document.get(id).as_dict().cloned();
            let name = format!("object {} {}", id.number, id.generation);
            state.text.font =
                self.load_font(Some(FontKey::Referenced(id)), font_dict.as_ref(), &name);
            state.text.size = narrow(size);
        } else {
            // A `/Font` this crate cannot read as the clause states it is reported rather
            // than half-applied: a size without a font would move every glyph the page
            // draws afterwards.
            self.note(Unsupported::Font {
                detail: "Table 57's /Font is not [indirect-reference size]".to_owned(),
            });
        }
    }

    /// Loads a font by resource name, caching the result including failures.
    ///
    /// A failure is cached too: a page that names an unloadable font on every `Tf` should
    /// pay for the attempt once, and should report it once.
    pub(super) fn font(&mut self, resources: &Dictionary, name: &Name) -> Option<Font> {
        // **Keyed by the font's identity, never by the name the stream used.** A resource name
        // is scoped to the resource dictionary that defines it, and §8.10.1 gives a form
        // `XObject` a `/Resources` of its own — so a page's `/F1` and a form's `/F1` are two
        // fonts as often as they are one, and a cache keyed by `F1` hands the second the
        // first's glyphs with nothing reported. That is trap 1's archetype, and it is what this
        // cache did for thirty-one sessions. `shading::Cache` had the same question and the
        // same answer (see `resource_entry`, whose whole reason for existing is this one).
        // §9.6.2.2's fourteen are ASCII names, so a resource name that is not text cannot be one
        // of them and `as_str` returning `None` is that answer rather than a lost lookup.
        let label = String::from_utf8_lossy(name.as_bytes());
        let entry = self
            .resource_entry(resources, "Font", name)
            .or_else(|| name.as_str().and_then(standard_font_named));
        let key = entry
            .as_ref()
            .and_then(Object::as_reference)
            .map(FontKey::Referenced);
        let dict = entry
            .map(|object| self.document.resolve(&object))
            .and_then(|object| object.as_dict().cloned());
        self.load_font(key, dict.as_ref(), &label)
    }

    /// Loads a font, caching it under `key`, which is what `Tf` and Table 57's `/Font` share.
    ///
    /// §8.4.1's NOTE 1 gives most graphics state parameters two routes, and this is the one
    /// where the two do not name the same thing: `Tf` names a *resource*, and Table 57's
    /// `/Font` is "an indirect reference to a font dictionary" instead. A cache keyed only by
    /// the resource name therefore had nowhere to put the second, which is why one corpus
    /// document's `/ExtGState` font was reported rather than loaded for twenty-four sessions.
    /// `key` is `None` for a resource dictionary that states its font *directly* rather than
    /// by reference. Such a font has no identity to key on and is therefore loaded afresh each
    /// time — correctness before speed, and the case is rare enough that no corpus document
    /// reaches it: every one of the 974 states its fonts indirectly, counted.
    fn load_font(
        &mut self,
        key: Option<FontKey>,
        dict: Option<&Dictionary>,
        name: &str,
    ) -> Option<Font> {
        if let Some(key) = key.as_ref()
            && let Some(cached) = self.fonts.get(key)
        {
            return cached.clone();
        }

        let loaded = dict.map(|dict| pdf_font::LoadedFont::load(self.document, dict, name));

        let result = match loaded {
            Some(Ok(font)) => Some(Font::Program(Rc::new(font))),
            // A Type 3 font has no program for `pdf-font` to read: its glyphs are content
            // streams, so it is this crate that draws them (§9.6.4). The refusal there is
            // the hand-off rather than a failure, which is why this is not a report.
            Some(Err(pdf_font::FontError::Type3 { .. })) => {
                match dict.map(|dict| crate::type3::Type3Font::read(self.document, dict, name)) {
                    Some(Ok(font)) => Some(Font::Type3(Rc::new(font))),
                    Some(Err(error)) => {
                        self.note(Unsupported::Font {
                            detail: error.to_string(),
                        });
                        None
                    }
                    None => None,
                }
            }
            Some(Err(error)) => {
                self.note(Unsupported::Font {
                    detail: error.to_string(),
                });
                None
            }
            None => {
                self.note(Unsupported::Font {
                    detail: format!("no /Font resource named /{name}"),
                });
                None
            }
        };

        if let Some(key) = key {
            self.fonts.insert(key, result.clone());
        }
        result
    }
}
