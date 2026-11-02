//! The middle stage: what the converter decides about a requirement the document failed.
//!
//! **The spine of the verb.** [`REMEDIES`] is the decision table, [`Decision`] is what one row
//! becomes once the caller's authorisations and the document's own preparations are known, and
//! everything in the other four files hangs off that answer: [`super::prepare`] builds what a
//! decision needs, [`super::rewrite`] carries it out, [`super::report`] says what it was.
//!
//! Nothing here reads the standard and nothing here touches the document. `pdf_archive::check` is
//! the reading and [`super::prepare`] is the document; this file is a pure function from a failed
//! requirement's identifier to one of five answers, which is what makes the table reviewable by
//! somebody who is not reading Rust.
use super::prepare::Prepared;
use super::rewrite::Rewrite;
use pdf_archive::Judgement;
use pdf_archive::survey::DeviceFamily;

// One module a requirement family, each holding that family's rows of the four tables below and
// the sentences those rows quote; this file keeps the shared types and the dispatch, and
// `doc/adr/2634` is the split.
mod annotations_and_attachments;
mod colour_and_graphics;
mod content_stream_fences;
mod files_and_structure;
mod fonts;
mod metadata;

use colour_and_graphics::{cmyk_answer_under_part_four, cmyk_answer_under_part_two, rgb_answer};
pub(super) use content_stream_fences::odd_hexadecimal_digits_on_a_page;
use content_stream_fences::{ODD_HEXADECIMAL_DIGITS, SYNTAX_INSIDE_CONTENT, in_a_content_stream};
pub(super) use fonts::THE_GLYPH_IS_NOT_IN_THE_PROGRAM;
use fonts::embedded_faces;
use metadata::{CONTAINER_DISCARDED, discards_a_container};

/// Something a conversion can throw away, which a user has to authorise first.
///
/// section 3 of `doc/pdf-a-conversion-limits.md`: "Each of these can be done. Each throws something
/// away. None of them may happen silently." A batch tool has nobody to ask — the same wall
/// `crate::Refusal::Unanswered` meets for Table 22's restrictions — so the question is asked
/// *before* the run, on the command line, and a loss nobody authorised stops the conversion
/// instead of happening quietly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Loss {
    /// section 4.7: an image's `/Interpolate` turned off.
    ///
    /// The one row of that table marked *not quite mechanical*, and the reason is in the row:
    /// interpolation is image smoothing, so a low-resolution image that had it on will look
    /// blockier afterwards. Nothing is deleted and no mark moves; what changes is how a
    /// conforming reader is told to sample the image it already has.
    ImageSmoothing,
    /// section 3.9: a metadata property its own predefined schema does not define, removed.
    ///
    /// ISO 19005-2 section 6.6.2.3.1 requires every property to *use* the schema whose namespace
    /// it names, and a packet can name a predefined namespace without using it — a value of one
    /// type where the schema defines another. Three routes exist and two are closed: correcting
    /// the value invents content, because a schema says what shape a value has and not what this
    /// document meant, and describing a predefined schema in section 6.6.2.3.2's extension
    /// container would misrepresent it in the file itself. What is left is removing the property,
    /// which throws away what its producer wrote — so the report names every one, its namespace
    /// and the value that was there, and a user can restore it by hand or supply a corrected
    /// source.
    MetadataProperty,
    /// ISO 19005-2 section 6.6.2.1, ISO 19005-4 section 6.7.2.1: a metadata packet this tree
    /// cannot read, replaced by one this conversion composes.
    ///
    /// The three editors beside this one write into a producer's packet *by span* — a property
    /// cut, a prefix moved, the identification schema restated — and a packet that will not
    /// parse, states more than one `rdf:RDF` element or breaks the data model has no spans to
    /// write into. Two answers exist and `doc/questions/A48` closes one of them: repairing the
    /// packet is this converter deciding what a malformed one meant. What is left is replacing
    /// it, and everything the producer recorded in it stops being metadata.
    ///
    /// **What the replacement states is decided by where the stream is.** The catalog's carries
    /// the identification schema and the events this conversion recorded, which is the packet
    /// `pdf_model::xmp::packet` already writes for a document that had none; every other
    /// metadata stream carries a packet stating no property, because nothing in the file says
    /// what the unreadable one meant.
    ///
    /// **The producer's own packet is kept where the configuration asks for it.** `remedy =
    /// "preserve"` with `original = "page"` lays its bytes out on a page appended to the
    /// document (`doc/adr/1014`), so what stops being metadata is still in the archive for a
    /// person to read and a machine to re-parse. Authorising the loss without that keeps
    /// nothing, and the report names every stream whose packet went either way.
    MetadataPacket,
    /// ISO 19005-2 section 6.6.4: an amendment or corrigendum identifier of the wrong form,
    /// removed.
    ///
    /// The subclause makes `pdfaid:amd` and `pdfaid:corr` optional and fixes the value as the
    /// number and the year separated by a colon. A value that is not of that form identifies no
    /// amendment the subclause recognises, and neither half of one can be recovered from it -
    /// correcting it would be this converter deciding what the producer meant, which is
    /// `doc/questions/A48`'s forbidden half. So the entry is removed, and what goes is the
    /// producer's claim about which amendment of the part their file was made to.
    ///
    /// **Small, and a loss all the same.** The claim was already wrong in the standard's terms,
    /// which is why the catalogue calls a departure here hard to want; but a reader of the
    /// output cannot see that the file ever made it, so the report names the value that was
    /// there.
    AmendmentIdentifier,
    /// section 3.7: an annotation that stated no flags is made printable.
    ///
    /// ISO 19005-2 section 6.3.2 and ISO 19005-4 section 6.3.2 require every annotation but a
    /// `Popup` to state an `/F`, and require its `Print` bit to be set. §12.5.2's Table 166 gives
    /// `/F` a default of 0, so an annotation stating none has every flag clear — and §12.5.3's
    /// Table 167 says what a clear `Print` bit means:
    ///
    /// > If clear, never print the annotation, regardless of whether it is rendered on the screen.
    ///
    /// So the file, read as the standard defines it, says this annotation is never printed, and
    /// the only `/F` that satisfies the requirement says the opposite. Nothing is deleted and no
    /// mark moves on screen; what is lost is the producer's statement about the printed page.
    ///
    /// **How much it costs depends on whether the annotation has an appearance**, and the same
    /// table says so: "If the annotation does not contain any appearance streams this flag shall
    /// be ignored." An annotation with no `/AP` therefore prints no differently for this; one
    /// with an `/AP` now appears on paper where it did not.
    AnnotationPrinting,
    /// ISO 19005-2 section 6.2.8.3, ISO 19005-4 section 6.2.7.3: the colour space specifications
    /// a JPEG 2000 image states beside the one it uses, removed.
    ///
    /// Both parts require exactly one specification to be marked as the best available where the
    /// data states several, and require the method a colour box states to be one of three. The
    /// route that restates nothing is **removal**, because the sentence after the method's says a
    /// conforming processor shall use only the selected colour space and shall ignore all the
    /// others — so what goes is what the target itself directs a processor to ignore.
    ///
    /// What it costs is one sentence of ISO 32000-2 §7.4.9:
    ///
    /// > If the colour space is given by an unsupported ICC profile, the next lower colour space,
    /// > in terms of precedence and approximation value, shall be used.
    ///
    /// The removed specifications are that fallback chain. A processor that can use the kept
    /// specification sees no difference at all; one that cannot now falls back to a device space
    /// rather than to the producer's second choice. Not one image sample is touched either way.
    Jpeg2000ColourFallback,
    /// section 3.6: the cryptographic assertion of every signature the source carries.
    ///
    /// A signature covers a byte range of one file — §12.8.1 — and a conversion rewrites every
    /// offset, so no output of this verb can carry the source's signatures as signatures; what
    /// it can do is carry the dictionary into bytes it no longer covers, which is a signature
    /// that lies and is what this verb wrote until ADR 1006. So the assertion is *removed*: each
    /// signature field loses its `/V` and keeps its widget and appearance (§12.7.5.5), the
    /// permissions dictionary loses the `DocMDP` and `UR3` entries that rested on the signature
    /// (Table 263, §12.8.2.3), and the form's `AppendOnly` flag is cleared (Table 225).
    /// `super::signatures` has the sentence deciding each.
    ///
    /// What is lost is exactly what a signature told a reader: who signed, when, and that the
    /// bytes had not changed since. **All three are written into the report before they go**,
    /// each signature named with its `/Name`, its `/M`, and what verifying it over the *source*
    /// found — so a reader of the output can still learn what the source asserted, from the
    /// report if not from the file. No mark on any page moves.
    ///
    /// Not a default and never mechanical, however invisible on the page: the assertion is the
    /// point of a signed document, and section 3.6 classes its loss *Ask, loudly*.
    SignatureAssertion,
    /// section 3.2: an annotation of a subtype ISO 19005 does not admit, taken off the page.
    ///
    /// ISO 19005-2 section 6.3.1 admits no annotation subtype ISO 32000-1 does not define, and
    /// strikes `3D`, `Sound`, `Screen` and `Movie` by name; ISO 19005-4 section 6.3.1 strikes the
    /// last three. Neither clause offers any way to keep such an annotation, so the only rewrite
    /// that meets the requirement removes it — section 3.2 of
    /// `doc/pdf-a-conversion-limits.md` classes that *Ask*, and re-badging it as a subtype the
    /// part admits is `doc/adr/0816`'s fence rather than a fix, because that invents an
    /// annotation the producer did not write.
    ///
    /// **What goes with it is everything the annotation carried except its marks.** A `Sound`
    /// annotation's sound, a `Movie`'s movie, a `Screen`'s rendition, a `3D`'s artwork, a
    /// `RichMedia`'s content, and each one's `/Contents` description: none of them is content any
    /// of the six targets holds, which is why the clause strikes the subtype at all.
    ///
    /// **Its marks are a separate question, and the configuration is where it is answered.** The
    /// normal appearance is a form `XObject` the producer wrote and §12.5.5 makes it what a
    /// reader draws for the annotation, so `remedy = "preserve"` keeps those marks on a page
    /// appended to the document rather than losing them with the rest (`doc/adr/1099`). Without
    /// it the page loses the mark, and the report names the page it was on.
    ForbiddenAnnotation,
    /// section 3.3: the behaviour an action carried, removed with the action.
    ///
    /// ISO 19005-2 section 6.5.1 and ISO 19005-4 section 6.6.1 forbid whole action *types*,
    /// sections 6.5.2 and 6.6.3 the additional-actions dictionary that triggers them, and both
    /// parts' section 6.4.1 a widget's or field's `/A`. None of the three offers anywhere to put
    /// what goes: a `Launch` action names a path on somebody's disk, and a `ResetForm`, an
    /// `ImportData` or a `Hide` has no representation at all outside a running reader.
    ///
    /// **What it costs is stated in one sentence and it is a large one: buttons stop doing
    /// things.** A form that computed its own fields stops computing them, and at a PDF/A-2
    /// target there is nowhere in the file to put what it computed — ISO 19005-4 section 6.4.1's
    /// XFDF route needs an embedded file, which ISO 19005-2 section 6.8 excludes outright.
    ///
    /// **What it does not cost is a mark.** Every content stream crosses this conversion byte for
    /// byte; what changes is what a reader does when a user clicks. And **the actions behind a
    /// removed one survive it**: §12.6.2's `/Next` chain is spliced rather than dropped, because
    /// a `GoTo` behind a `Launch` is failing no requirement (ADR 1175).
    ///
    /// Not mechanical, for `doc/pdf-a-conversion-limits.md` section 3.3's reason: it classes this
    /// *Ask*, and every removal is named in the report before it goes.
    InteractiveBehaviour,
    /// section 3.5: the document's encryption, and with it Table 22's permission flags.
    ///
    /// ISO 19005-2 section 6.1.3 and ISO 19005-4 section 6.1.3 forbid an `/Encrypt` key in the
    /// trailer outright, and sections 6.1.7.2 and 6.1.6.2 forbid a `Crypt` filter naming anything
    /// but `Identity`. Neither offers a conforming file any way to stay encrypted, so the one
    /// rewrite that meets them writes the document out with no encryption at all —
    /// [`Rewrite::EncryptionRemoved`].
    ///
    /// **No mark moves and no value changes.** §7.6.2 makes encryption a property of the file:
    /// every string and every stream this conversion carries was decrypted when the source was
    /// opened, so the output holds exactly what the source held. What is lost is the *file's*
    /// requirement of a key.
    ///
    /// **What goes with it is a statement, and the statement is kept.** §7.6.4.2's Table 22 flags
    /// say what the producer asserted a reader may do — print it, copy from it, change it — and an
    /// unencrypted file asserts none of them. Enforcement cannot survive; the assertion can, so
    /// every flag the source stated is written into the report and into the output's own
    /// `xmpMM:History` before it goes. `doc/adr/1187` is the argument, and it is
    /// `doc/pdf-a-mitigations.md` section 2's `preserve` beside the `discard`.
    ///
    /// Not mechanical, however invisible on the page: `doc/pdf-a-conversion-limits.md` section 3.5
    /// classes it *Ask* and never a default, because the archived copy becomes readable by anybody
    /// holding it.
    Encryption,
    /// ISO 19005-2 section 6.1.13: an out-of-range optional page boundary entry, removed, where
    /// its removal changes the region a reader computes.
    ///
    /// §7.7.3.3's Table 31 makes `/MediaBox` required and the crop, bleed, trim and art boxes
    /// optional, and §14.11.2.1 gives each optional one a default that is another box in the
    /// same file — the crop box's "default value is the page's media box", the other three's
    /// "the page's crop box". So removing an out-of-range entry is the page saying itself the
    /// way Table 31 admits rather than an edit to what it says, and `doc/adr/1210`'s predicate
    /// decides whether it is free: §14.11.2.1 requires that
    ///
    /// > If the bounds of the crop, trim, bleed or art box extends outside of the bounds of the
    /// > media box, a processor shall treat the box as its intersection with the media box.
    ///
    /// Where that intersection is what the default would give, no reader computes anything
    /// different and the removal is mechanical rather than this loss. **This is the other
    /// case**: a box too *small* to meet the limit, or an over-sized one a narrower crop box
    /// stands behind, where a reader would clip, trim or place the page's content differently
    /// afterwards.
    ///
    /// **No mark moves and no content is deleted.** What changes is the region a production
    /// process is told to clip or trim to, which is what the boxes are for; the report names
    /// every entry that went, the page it was on, and the rectangle a reader computed before
    /// and after. A failing **media box** is not this loss and never reaches it: it is the one
    /// boundary Table 31 requires, so there is nothing to fall back to and the requirement
    /// stays refused by name.
    PageBoundary,
    /// section 3.7: an annotation whose producer hid it, removed from the page it was on.
    ///
    /// ISO 19005-2 section 6.3.2 and ISO 19005-4 section 6.3.2 require an annotation's `/F` to
    /// set `Print` and to leave `Hidden`, `Invisible`, `NoView` and `ToggleNoView` clear, and
    /// they offer nothing to put in the place of one that does otherwise. So the document has two
    /// futures and no third — the annotation becomes visible and printable, or it goes — and
    /// `doc/pdf-a-conversion-limits.md` section 3.7 makes removal the default of the two, because
    /// an annotation somebody hid was hidden on purpose and showing it puts a mark on a page its
    /// producer kept off.
    ///
    /// **This is the half that states flags.** An annotation stating no `/F` at all is
    /// [`Self::AnnotationPrinting`] and is written rather than removed, because §12.5.2's Table
    /// 166 default of 0 is an absence rather than a decision.
    ///
    /// What is lost is the annotation and everything only it reached; the report names each one,
    /// its page, its subtype, the flags its producer wrote, and whether it drew a mark.
    HiddenAnnotation,
    /// section 3 of `doc/pdf-a-mitigations.md`'s annotation entry: a rollover or down appearance,
    /// dropped.
    ///
    /// ISO 19005-2 section 6.3.3 and ISO 19005-4 section 6.3.3 admit `/N` in an appearance
    /// dictionary and no other key. §12.5.5's Table 170 makes `/R` and `/D` optional and gives
    /// each the same default — the value of the `/N` entry — so a reader of the output draws the
    /// normal appearance where it drew the producer's rollover or down one, and the states the
    /// clause describes are the states it still has.
    ///
    /// What is lost is the artwork itself: the marks §12.5.5 has a reader draw while the pointer
    /// is over the annotation or the mouse button is held down are no longer in the file.
    AppearanceStates,
    /// section 3.8: an optional content configuration's automatic states, removed.
    ///
    /// ISO 19005-2 section 6.9 forbids the `/AS` entry. §8.11.4.3 makes it the array that has a
    /// processor set group states from external factors — the viewing magnification, the system
    /// language, whether the page is being printed — so removing it leaves the document in
    /// whatever state the configuration's own `/BaseState`, `/ON` and `/OFF` put it in, and
    /// nothing switches thereafter.
    ///
    /// **A PDF/A-4 target loses nothing here and the report says so**: ISO 19005-4 section 6.10
    /// keeps the key and requires a conforming processor to ignore it, which is the same document
    /// for a reader and the producer's array still in the file.
    AutomaticStates,
    /// ISO 19005-2 section 6.4.1, ISO 19005-4 section 6.4.1: a form whose appearances a reader
    /// was asked to build, left as the producer wrote it with the request removed.
    ///
    /// §12.7.3's Table 224 makes `/NeedAppearances` "\[a] flag specifying whether to construct
    /// appearance streams and appearance dictionaries for all widget annotations in the
    /// document", and both parts forbid the file to ask. Two answers follow and the operator
    /// chooses between them: the appearances are constructed first, which loses nothing and is
    /// the `Decision::Stated` this row takes by default, or the flag goes on its own — and then
    /// **a field whose appearance its producer never wrote is blank in the archive**, because
    /// nothing is left to tell a reader to draw it.
    ///
    /// That is the loss, and it is the second answer only. A printed copy of the same form showed
    /// the same empty boxes, which is what makes it an operator's choice rather than a defect;
    /// `construct = true` is what a caller who says nothing gets.
    FieldAppearances,
    /// ISO 19005-2 section 6.4.2, ISO 19005-4 section 6.4.2: the XFA resource, removed with the
    /// key that named it.
    ///
    /// Neither part offers anywhere to put it, and what goes is what the template carried beyond
    /// the fields themselves — the calculations, the validations and the formatting. **What does
    /// not go is the form**: ISO 32000-2 Annex K requires a file stating the key to keep its
    /// interactive form dictionary consistent with the resource, field for field and value for
    /// value, so the `AcroForm` this conversion writes is the form the XFA described.
    ///
    /// **Not a mark, and not a page.** Every content stream crosses this conversion byte for
    /// byte; what stops is a form engine recomputing fields as somebody fills them in. A document
    /// whose own `/NeedsRendering` says its pages are regenerated is the case where the
    /// `AcroForm`
    /// is *not* the document, and `doc/adr/1257` refuses that one by name rather than pricing it
    /// here.
    XfaForm,
    /// ISO 19005-2 section 6.8, ISO 19005-4 section 6.9: an embedded file the target does not
    /// admit, taken out of the document.
    ///
    /// Both clauses require an embedded file to conform to a part of ISO 19005 — part 2 to
    /// ISO 19005-1 or -2, plain PDF/A-4 to a part of its own — and neither offers anywhere to
    /// put one that does not. So the file specification and the bytes under it go, and what is
    /// lost is the attachment itself: a spreadsheet, an invoice's XML, a scanned original.
    ///
    /// **The other three answers are the configuration's and are better where they are open.**
    /// `doc/rfc/0007` section 4.6 ranks them: attach unchanged, which PDF/A-4f and -4e admit and
    /// this row does not bind at; derive a conforming PDF from the bytes with a declared tool,
    /// which `remedy = "derive"` reaches; and the owner's own XML departure, which keeps an
    /// attachment a part of ISO 19005 will not. This is the last of the four, and the report
    /// names every file that went.
    ///
    /// **No mark on any page moves.** The attachment was never drawn; what changes is that the
    /// archive no longer carries it.
    EmbeddedFile,
    /// ISO 19005-2 section 6.2.5, ISO 19005-4 section 6.2.5: a graphics state's transfer
    /// function other than the identity, removed.
    ///
    /// ISO 32000-2 §10.5's function is applied by this reader, so the colours drawn under the
    /// state move once it is gone. §8.4.1's Table 52 makes it a function "to compensate for
    /// nonlinear response in a particular output device", and §10.5 says a page description meant
    /// to be device-independent should not define one — but a file that does may depend on it for
    /// what its page shows on paper as on a screen: `issue6931_reduced.pdf`'s logo is a near-black
    /// negative without its function. So no shipped profile authorises it; an operator does.
    ///
    /// **What is not lost is the function itself**: each one is written into the output's own
    /// `xmpMM:History`, in clause 7's syntax and its data decoded, before it goes, and the report
    /// names every graphics state it went from (`doc/adr/1964`). A file whose every `/TR` is the
    /// name `Identity` costs nothing here and is not asked.
    TransferFunction,
    /// ISO 19005-2 section 6.2.5, ISO 19005-4 section 6.2.5: a halftone's `HalftoneName`,
    /// removed.
    ///
    /// ISO 32000-2 §10.6.5.1 makes the name a pointer into a marking device's own halftones, which
    /// such a device "may" use in place of the dictionary's parameters. **Nothing a screen shows
    /// moves** — §10.6.1 says halftoning is not required on a device that reproduces continuous
    /// tone, and this reader is one — so what is lost is the instruction to a press: one holding
    /// a halftone of that name screens with the dictionary's own parameters instead. A halftone
    /// that is only a name is refused rather than removed (`doc/adr/1965`).
    HalftoneName,
    /// ISO 19005-2 section 6.2.5, ISO 19005-4 section 6.2.5: a halftone of a type other than 1 or
    /// 5, replaced by the device's own, and part 4's halftone origin, removed.
    ///
    /// A threshold halftone is a whole screen, so the graphics states naming it set Table 57's
    /// `Default` — the halftone in effect at the start of the page — and the halftone leaves the
    /// file; `HTO` places a screen and is simply removed. **Nothing a screen shows moves**, for
    /// §10.6.1's reason as [`Self::HalftoneName`]; what is lost is the producer's screen on a press,
    /// which screens those marks with its own halftone, at its own origin (`doc/adr/2034`).
    Halftone,
    /// ISO 19005-2 section 6.2.4.2, ISO 19005-4 section 6.2.4.2: overprint mode 1, set to 0 in the
    /// graphics states that set it, where one of them also governs a `DeviceCMYK` mark.
    ///
    /// ISO 32000-2 §8.6.7 makes the mode act on `DeviceCMYK` marks painted with overprinting on: a
    /// zero component "shall leave the corresponding component of the previously painted colour
    /// unchanged". With the mode at 0 it paints that component out, so a press — or a reader
    /// simulating one — composites such a mark differently: black text stops overprinting the
    /// colour under it. **Nothing this reader's window draws moves**, because the mode acts here
    /// only where a group blends in four components, and a mark there is refused rather than
    /// offered (`doc/adr/1978`). Where the states govern only `ICCBased` marks, changing the mode
    /// moves nothing the standard defines and is not asked.
    OverprintMode,
    /// ISO 19005-2 section 6.7.4: a `/Lang` that is not a language identifier states that the
    /// language is unknown.
    ///
    /// §14.9.2.2 admits two values and no third: the empty text string, "to indicate that the
    /// language is unknown", or a BCP 47 tag. Correcting a malformed tag is this converter
    /// deciding what language the producer meant, which `doc/questions/A48` forbids, and removing
    /// the entry would let §14.9.2.3's inheritance assert a parent's language over the text. So
    /// the entry is kept and states the one thing that is known — that nothing is — and what goes
    /// is the producer's own spelling, which a lenient reader might have understood. Nothing on
    /// any page moves; the report names every entry with the value it held, and `remedy =
    /// "supply"` with `language` writes the operator's tag instead (`doc/adr/2066`).
    LanguageIdentifier,
    /// ISO 19005-2 section 6.2.11.4.1, ISO 19005-4 section 6.2.10.4.1: a font this viewer draws
    /// from a face of this machine's is embedded with the face this program ships instead.
    ///
    /// ADR 0270 lets the viewer take a machine's face of the same family where the document's
    /// encoding names characters the shipped face lacks, and that face is not this program's to
    /// embed: ISO 32000-2 §9.9.1 makes embeddability a fact about a licence. Where every code the
    /// page shows has a glyph in the shipped face, embedding it draws every mark the page makes, at
    /// the same widths — **what changes is the shapes**, from the machine's face to the one a
    /// reader without it already draws. §9.5's NOTE 5 leaves the face of a font the file does not
    /// carry to the processor, so nothing the file states is lost; what this viewer's window
    /// showed on this machine is (`doc/adr/2162`).
    MachineFace,
    /// ISO 19005-2 section 6.2.11.4.1, ISO 19005-4 section 6.2.10.4.1: a font this viewer draws
    /// from the face this program ships, narrowed to the file's widths, is embedded with that face
    /// as it is.
    ///
    /// ADR 0358 has the viewer draw a substitute's outlines at the horizontal scale that fits its
    /// advances to the file's `/Widths` (`pdf_font::LoadedFont::stretch`); an embedded program is
    /// drawn as its producer shipped it, at none. Embedding the unmodified face keeps every
    /// position the file states and every glyph the page selects, and draws each glyph at the
    /// face's own width instead of the fitted one. §9.5's NOTE 5 leaves the face of a font the
    /// file does not carry to the processor, so nothing the file states is lost; what this
    /// viewer's window showed is (`doc/questions/A566`, `doc/adr/2262`).
    SubstituteStretch,
    /// ISO 19005-2 section 6.2.11.4.1, ISO 19005-4 section 6.2.10.4.1: an `Adobe-Identity`
    /// composite font the file does not embed is given a shipped face's glyphs, each chosen by
    /// the character the font's `/ToUnicode` states for its code.
    ///
    /// ISO 32000-2 §9.7.5.2 forbids the identity `CMap`s with a non-embedded font, because such a
    /// CID indexes a program nobody supplied; the viewer draws these pages through the font's
    /// `/ToUnicode` all the same (ADR 0270), and the conversion writes that selection down. What
    /// is lost is the certainty that each mark is the glyph its producer drew: a `/ToUnicode` is
    /// §9.10's extraction aid, and a producer may have written it for searching rather than for
    /// drawing (`doc/questions/A570`, `doc/adr/2378`).
    GlyphsFromToUnicode,
    /// ISO 32000-2 §9.6.2.1's Table 109, ISO 19005-2 section 6.2.11.4.1, ISO 19005-4 section
    /// 6.2.10.4.1: a simple font outside the standard 14 that states neither a descriptor nor
    /// `/Widths` is embedded with a shipped face and the widths this viewer lays it out by.
    ///
    /// Table 109 makes both required of every simple font but the fourteen, and Annex D publishes
    /// those fourteen's metrics; for any other name the standard states no layout. The viewer
    /// draws such a font from the face whose metrics its name resembles
    /// (`pdf_font::standard_metrics`), and the conversion writes that layout down as the font's
    /// `/Widths`, so the output states one reader's choice as the producer's. Another reader may
    /// have laid the same name out by the face it names (`doc/questions/A687`,
    /// `doc/adr/2504`).
    LayoutFromThisViewer,
    /// ISO 19005-4 section 6.2.4.3: a `DeviceCMYK` beside an RGB destination profile is licensed by
    /// a `DefaultCMYK` over the CMYK profile this program ships, a press nothing in the file names.
    ///
    /// Section 6.2.3 lets a file state one destination profile, so the licence left is a device
    /// independent `DefaultCMYK`, and an ICC CMYK profile states a press. ISO 32000-2 §8.6.4.1
    /// gives the device spaces no colorimetric meaning, so which press the four numbers were
    /// separated for is not in the file; the conversion states one, and what is lost is the
    /// certainty that it is the producer's. `as-if-printed` is "as if printed and scanned": a
    /// lower bound on what may be lost, never an instruction to imitate a press
    /// (`doc/questions/A694`, `doc/adr/2518`). An operator's own press profile, supplied with
    /// `--output-intent-profile`, outranks it and costs nothing (`doc/adr/2130`).
    PressAssumed,
    /// ISO 32000-2 §7.5.5 Table 15, §7.7.2 Table 29, §7.7.3.2 Table 30: a document whose trailer,
    /// catalog or page tree node does not state the page tree is written out as the structure this
    /// viewer recovered.
    ///
    /// The standard makes `/Root` and `/Pages` required and defines no recovery when either fails;
    /// the viewer finds the pages by their own `/Type /Page` declarations all the same, and the
    /// conversion writes a catalog and a single page tree node listing them, each page the
    /// producer's with only the entries it owes its tree restated. What is lost is the
    /// producer's tree, which is not there to keep: another reader may find fewer pages, more
    /// pages or the same pages in another order (`doc/adr/2502`).
    RecoveredStructure,
    /// ISO 19005-2 section 6.2.11.4.1, ISO 19005-4 section 6.2.10.4.1: a page showing an
    /// `Adobe-Identity` composite font the file does not embed, whose codes no `/ToUnicode` names,
    /// is replaced by a photograph of itself as this viewer's window draws it.
    ///
    /// No face can be keyed to such a font, because the per-CID correspondence §9.7.4.2 makes the
    /// missing program's is what is missing; the window draws those marks blank (ADR 0270), and so
    /// does the photograph. What is lost is the page as content — its vector marks, its text in
    /// every other font, its resolution independence — and **nothing a reader could extract**: the
    /// owner's gate admits only a page whose readback names no character (`doc/questions/A570`,
    /// `doc/adr/2394`). A print never had a text layer either, so both shipped profiles carry it.
    TextAsImage,
    /// ISO 19005-2 section 6.2.11.4.1, ISO 19005-4 section 6.2.10.4.1 and the content-stream rows
    /// the fence stops at: a page is replaced by a photograph of itself as this viewer's window
    /// draws it **whatever text it also shows**, where it shows an unembedded font no route of this
    /// program can carry or holds a fault of its content stream the target refuses.
    ///
    /// The wider gate of [`Self::TextAsImage`], and a loss of its own because what it takes is
    /// different in kind: the narrow gate admits only a page with nothing a reader could extract,
    /// and this one turns a page's extractable text into pixels. It is what a company does when it
    /// prints a letter and scans the print; `as-if-printed` means exactly that lower bound on what
    /// may be lost, and never an instruction to imitate a press (`doc/questions/A694`,
    /// `doc/adr/2516`). The text each such page held is named in the report and in the file's
    /// history.
    PageAsImage,
    /// ISO 32000-2 section 14.7.5's content items: where a photographed page belongs to a document
    /// that states a structure tree, the tree's leaves that name the page — the marked-content
    /// sequences of its old content (section 14.7.5.2) and the objects only that content drew
    /// (section 14.7.5.3) — are taken out of the tree, and the rest of the tree is kept.
    ///
    /// A loss of its own, because what it takes is the tagging rather than the page: a reader
    /// that reads the document by its structure no longer finds the photographed page's text in
    /// it, while every other page reads as it did. The leaves go only where the photograph has
    /// already turned what they named into pixels, and Level A, which exists to keep the tagging,
    /// is offered no photograph at all (`doc/questions/A707`, `doc/adr/2516`). The number of
    /// leaves each page lost is named in the report and in the file's history.
    TaggingOfPhotographedPages,
    /// ISO 32000-2 section 8.11's optional content: where a photographed page belongs to a
    /// document that states optional content, the photograph is drawn in the configuration this
    /// viewer opens the document in — §8.11.4.3's default configuration `/D` as §8.11.4.5 applies
    /// it, its `View` usage included and §8.11.4.4's `Print` usage never — and each optional content group
    /// the page's content named is fixed in the state it was drawn in, its other state discarded.
    ///
    /// A loss of its own, because what it takes is the layers rather than the text: a reader that
    /// switches a group no longer changes what the photographed page shows, while every other page
    /// keeps its optional content and the document's `/OCProperties` stays. Level A and Level U are
    /// offered no photograph at all (`doc/questions/A717`). The groups each photographed page
    /// fixed, by `/Name` and by the state drawn, are named in the report and in the file's history.
    OptionalContentOfPhotographedPages,
    /// ISO 32000-2 §7.7.3.2 Table 30, §F.3.3 Table F.1: a page the page tree or the linearization
    /// parameter dictionary counts, and the file names a place for and does not hold there, is not
    /// in the archive; the count stated and the count carried are written into the report.
    ///
    /// A rebuilt structure ([`Self::RecoveredStructure`]) lists the pages this reader drew. Where
    /// the root's `/Count` or `/N` states more, and the difference is not a `/Kids` entry naming
    /// no object (§7.3.10's null, which is no page), the cross-reference information placed an
    /// object the reader could not read there: a page of the producer's whose bytes are not where
    /// the file says. Nothing is invented for it; what is lost is that page, and the report says
    /// how many the source stated and how many the archive holds (`doc/questions/A727`,
    /// `doc/adr/2596`).
    PageNotHeld,
    /// ISO 19005-2 section 6.2.2 and ISO 19005-4 section 6.2.2: a run of a content stream that
    /// ISO 32000-2 §7.3.3 spells as no number, and that this reader draws as the number it reads
    /// off the run's front (`33600.272.58` drawn as `33600.272`, ADR 1004), is written as that
    /// number.
    ///
    /// The page draws as the window drew it, and its text stays text: what is lost is the
    /// characters of each run after the number, which no reader of the source drew, and the
    /// run's own spelling, which the report and the statement page name with the number written
    /// (`doc/adr/2638`). It is answered ahead of the photograph, which the same profiles take for a
    /// page whose fault this does not reach.
    ContentAsDrawn,
}

impl Loss {
    /// Every loss this converter knows how to ask about.
    pub const ALL: [Self; 34] = [
        Self::ImageSmoothing,
        Self::MetadataProperty,
        Self::MetadataPacket,
        Self::AmendmentIdentifier,
        Self::AnnotationPrinting,
        Self::Jpeg2000ColourFallback,
        Self::SignatureAssertion,
        Self::ForbiddenAnnotation,
        Self::InteractiveBehaviour,
        Self::Encryption,
        Self::PageBoundary,
        Self::HiddenAnnotation,
        Self::AppearanceStates,
        Self::AutomaticStates,
        Self::FieldAppearances,
        Self::XfaForm,
        Self::EmbeddedFile,
        Self::TransferFunction,
        Self::HalftoneName,
        Self::Halftone,
        Self::OverprintMode,
        Self::LanguageIdentifier,
        Self::MachineFace,
        Self::SubstituteStretch,
        Self::GlyphsFromToUnicode,
        Self::LayoutFromThisViewer,
        Self::PressAssumed,
        Self::RecoveredStructure,
        Self::TextAsImage,
        Self::PageAsImage,
        Self::TaggingOfPhotographedPages,
        Self::OptionalContentOfPhotographedPages,
        Self::PageNotHeld,
        Self::ContentAsDrawn,
    ];

    /// The word a caller authorises it by.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::ImageSmoothing => "image-smoothing",
            Self::MetadataProperty => "metadata-property",
            Self::MetadataPacket => "metadata-packet",
            Self::AmendmentIdentifier => "amendment-identifier",
            Self::AnnotationPrinting => "annotation-printing",
            Self::Jpeg2000ColourFallback => "jpeg2000-colour-fallback",
            Self::SignatureAssertion => "signature-assertion",
            Self::ForbiddenAnnotation => "forbidden-annotation",
            Self::InteractiveBehaviour => "interactive-behaviour",
            Self::Encryption => "encryption",
            Self::PageBoundary => "page-boundary",
            Self::HiddenAnnotation => "hidden-annotation",
            Self::AppearanceStates => "appearance-states",
            Self::AutomaticStates => "automatic-states",
            Self::FieldAppearances => "field-appearances",
            Self::XfaForm => "xfa-form",
            Self::EmbeddedFile => "embedded-file",
            Self::TransferFunction => "transfer-function",
            Self::HalftoneName => "halftone-name",
            Self::Halftone => "halftone",
            Self::OverprintMode => "overprint-mode",
            Self::LanguageIdentifier => "language-identifier",
            Self::MachineFace => "machine-face",
            Self::SubstituteStretch => "substitute-stretch",
            Self::GlyphsFromToUnicode => "glyphs-from-to-unicode",
            Self::LayoutFromThisViewer => "layout-from-this-viewer",
            Self::PressAssumed => "press-assumed",
            Self::RecoveredStructure => "recovered-structure",
            Self::TextAsImage => "text-as-image",
            Self::PageAsImage => "page-as-image",
            Self::TaggingOfPhotographedPages => "tagging-of-photographed-pages",
            Self::OptionalContentOfPhotographedPages => "optional-content-of-photographed-pages",
            Self::PageNotHeld => "page-not-held",
            Self::ContentAsDrawn => "content-as-drawn",
        }
    }

    /// What is lost, in one sentence for a person.
    #[expect(
        clippy::too_many_lines,
        reason = "one arm per loss, and the arms are prose for a person rather than code. \
                  Splitting the match would need either a catch-all, which stops a new loss from \
                  failing to compile until somebody has written its sentence, or an unreachable arm"
    )]
    #[must_use]
    pub const fn describe(self) -> &'static str {
        match self {
            Self::ImageSmoothing => {
                "image smoothing is turned off, so a low-resolution image will look blockier"
            }
            Self::MetadataProperty => {
                "a metadata property whose predefined schema does not define the value it holds \
                 is removed from the packet, and what its producer wrote there is gone"
            }
            Self::MetadataPacket => {
                "a metadata stream whose packet this tree cannot read — it does not parse, it \
                 states more than one rdf:RDF element, or it breaks the XMP data model - carries \
                 a packet this conversion composed instead, and everything the producer recorded \
                 in it stops being metadata. The catalog's states the identification schema and \
                 what this conversion recorded; every other one states no property. Answer the \
                 site with remedy = \"preserve\" and original = \"page\" and the producer's own \
                 packet is laid out on a page appended to the document instead of being lost"
            }
            Self::AmendmentIdentifier => {
                "an amendment or corrigendum identifier that is not the number and the year \
                 separated by a colon is removed from the identification schema, so the file \
                 stops claiming an amendment it identified incorrectly. Nothing else in the \
                 packet changes, and the report names the value that was there"
            }
            Self::AnnotationPrinting => {
                "an annotation that stated no flags is given the Print flag ISO 19005 requires, \
                 so one whose appearance never printed now prints"
            }
            Self::Jpeg2000ColourFallback => {
                "a JPEG 2000 image keeps only the colour space specification it uses, so a \
                 processor that cannot use that one falls back to a device space rather than to \
                 the producer's next specification. No image sample is touched"
            }
            Self::SignatureAssertion => {
                "every signature the source carries loses its cryptographic assertion — the \
                 conversion rewrites every byte the signature covered, so its value is removed \
                 rather than carried as a signature that no longer covers the bytes it sits in. \
                 Each field keeps its widget and appearance; the report names each signature, \
                 its signer, its time, and what verifying it over the source found"
            }
            Self::ForbiddenAnnotation => {
                "an annotation whose subtype ISO 19005 does not admit is removed from the page it \
                 was on, and with it everything that annotation carried \u{2014} a sound, a \
                 movie, a rendition, 3D artwork, and its own Contents description. A 3D or \
                 RichMedia annotation is admitted by PDF/A-4e, the target that keeps it whole. \
                 A FileAttachment annotation's file is kept: it is filed in the document's \
                 EmbeddedFiles tree, so only the marker on the page goes. \
                 Its normal appearance is the one part of it a target will hold: answer this \
                 site with remedy = \"preserve\" and those marks are kept on a page appended to \
                 the document"
            }
            Self::InteractiveBehaviour => {
                "an action of a type ISO 19005 does not admit is taken out of the file, an /AA \
                 the part does not admit goes with it, and a widget's or field's /A goes in both \
                 parts \u{2014} so buttons stop doing things and a form stops computing its own \
                 fields. The actions ISO 32000-2 \u{a7}12.6.2's Next entry performed after a \
                 removed one are kept and take its place; no mark on any page moves, and the \
                 report names every action that went"
            }
            Self::PageBoundary => {
                "an out-of-range crop, bleed, trim or art box entry is removed from the page, \
                 and ISO 32000-2 \u{a7}14.11.2.1's default for it \u{2014} the media box for a \
                 crop box, the crop box for the other three \u{2014} names a different region \
                 from the one that entry named. No mark moves and nothing is deleted; what \
                 changes is the region a reader clips to or a production process trims to. The \
                 report names every entry that went, the page it was on, and the rectangle a \
                 conforming reader computed before and after"
            }
            Self::Encryption => {
                "the document's encryption is not carried into the output, so the archived copy \
                 is readable by anybody holding it and no password opens or withholds it. \
                 Nothing in the document changes \u{2014} every string and every stream was \
                 decrypted when the source was opened \u{2014} but ISO 32000-2 \u{a7}7.6.4.2's \
                 Table 22 permission flags stop being asserted with it, so a producer's \"no \
                 printing\" or \"no extraction\" is no longer in the file. The report names every \
                 flag the source stated, and the output's own xmpMM:History records them"
            }
            Self::HiddenAnnotation => {
                "an annotation whose F entry hides it, keeps it off the printed page or off the \
                 screen is removed from the page it was on, and everything only that annotation \
                 reached goes with it. The report names each one, the page it was on, its \
                 subtype, the flags its producer wrote and whether it drew a mark; the \
                 alternative is showing it, which puts a mark on a page its producer kept off"
            }
            Self::AppearanceStates => {
                "an annotation loses the rollover and down appearances beside its normal one, so \
                 the marks ISO 32000-2 \u{a7}12.5.5 has a reader draw while the pointer is over \
                 it or the mouse button is held down are no longer in the file. Table 170 makes \
                 the normal appearance what a reader draws in their place, and no page changes \
                 until somebody points at the annotation"
            }
            Self::AutomaticStates => {
                "an optional content configuration loses the AS array that had a reader switch \
                 layers by viewing magnification, by system language or by printing, so the \
                 archive shows the state the configuration itself sets and nothing switches \
                 afterwards. A PDF/A-4 target costs nothing here: ISO 19005-4 section 6.10 keeps \
                 the key and has a conforming processor ignore it"
            }
            Self::FieldAppearances => {
                "the interactive form dictionary's NeedAppearances is removed and no appearance \
                 is constructed in its place, so a field whose appearance its producer never \
                 wrote is blank in the archive \u{2014} nothing in the file then tells a reader \
                 to draw it. Answer forms/need-appearances-absent-or-false with construct = true, \
                 which is the default, and every visible widget the file provides none for is \
                 given one first and nothing is lost"
            }
            Self::XfaForm => {
                "the interactive form dictionary's XFA is removed, and with it the calculations, \
                 validations and formatting the XFA template carried; a form that computed its \
                 own fields stops computing them. The fields and their values stay, because ISO \
                 32000-2 Annex K requires a file stating the key to keep the interactive form \
                 dictionary consistent with the resource \u{2014} so for a static form the \
                 AcroForm is the form. No mark on any page moves"
            }
            Self::EmbeddedFile => {
                "an embedded file that is not itself a conforming PDF/A is taken out of the \
                 document, and the bytes go with it \u{2014} so the archive no longer carries \
                 the attachment. No mark on any page moves, because an attachment is not drawn. \
                 The report names every file that went; remedy = \"derive\" with a declared \
                 tool makes a conforming PDF of it instead, and a PDF/A-4f or -4e target keeps \
                 it unchanged because the rule does not bind there"
            }
            Self::TransferFunction => {
                "a graphics state's or a halftone's transfer function other than the identity is \
                 removed, so the colours ISO 32000-2 \u{a7}10.5 had it adjust for one output \
                 device are drawn without that adjustment. The function itself is written into \
                 the output's own xmpMM:History first, in the file's own syntax with its data \
                 decoded, and the report names every graphics state or halftone it went from"
            }
            Self::HalftoneName => {
                "a halftone's HalftoneName is removed and the screen its own entries define is \
                 kept, so a press holding a halftone of that name screens the page with the \
                 file's parameters instead of its own. Nothing on a screen moves: ISO 32000-2 \
                 \u{a7}10.6.1 needs no halftoning on a device that shows continuous tone"
            }
            Self::Halftone => {
                "a halftone of a type other than 1 or 5 is replaced by the device's own - the \
                 graphics states naming it set Default, the halftone in effect at the start of \
                 the page - and a graphics state's halftone origin is removed, so a press screens \
                 those marks its own way rather than with the producer's threshold screen or at \
                 the producer's origin. Nothing on a screen moves: ISO 32000-2 \u{a7}10.6.1 needs \
                 no halftoning on a device that shows continuous tone"
            }
            Self::OverprintMode => {
                "a graphics state's overprint mode 1 is set to 0, so a DeviceCMYK mark painted \
                 with overprinting on paints its zero components instead of leaving what was \
                 under them: on a press, black text knocks out the colour beneath it where it \
                 overprinted. Nothing in this viewer's window moves, and the report names every \
                 graphics state changed and the pages whose DeviceCMYK marks it governed"
            }
            Self::LanguageIdentifier => {
                "a Lang entry that is not a language identifier - on the catalog, a structure \
                 element or a marked-content property list - states the empty identifier ISO \
                 32000-2 \u{a7}14.9.2.2 gives an unknown language, so a reader that understood \
                 the producer's malformed tag anyway now falls back on its own default. No mark \
                 on any page moves, and the report names every entry with the value it held; \
                 remedy = \"supply\" with language = \"<tag>\" writes the operator's tag instead"
            }
            Self::MachineFace => {
                "a font this viewer draws from a face of this machine's, because the document's \
                 encoding names characters the face this program ships lacks, is embedded with \
                 the shipped face: every character the pages show is drawn, at the same widths, \
                 in the shipped face's shapes rather than the machine's — the page a reader \
                 without that face already shows. Name the machine's face with --font, a \
                 statement that it may lawfully be embedded, to keep its shapes instead where the \
                 font states a descriptor"
            }
            Self::SubstituteStretch => {
                "a font this viewer draws from the face this program ships, narrowed to the \
                 widths the file states, is embedded with that face unmodified: every character \
                 sits where the file puts it and selects the glyph it selected, and each glyph is \
                 drawn at the face's own width rather than the fitted one. The report names every \
                 such font with the horizontal scale it was drawn at"
            }
            Self::GlyphsFromToUnicode => {
                "an Adobe-Identity composite font the file does not embed, whose codes index a \
                 program nobody supplied, is embedded with the glyphs a shipped face draws for the \
                 characters its ToUnicode CMap states - the selection this viewer's window already \
                 makes. Every code keeps its position, and shows the character the map states for \
                 it; where the producer wrote the map for searching rather than to describe the \
                 glyphs, the page shows what the map says rather than what was drawn. Name the \
                 producer's program with --font to keep its own glyphs instead"
            }
            Self::LayoutFromThisViewer => {
                "a simple font the file names, outside the fourteen standard fonts, that states \
                 neither a FontDescriptor nor Widths is embedded with the shipped face this \
                 viewer draws it from, and the widths this viewer lays it out by are written as \
                 the font's own: the standard states no layout for such a name, so another \
                 reader may have set the same text at the advances of the face the name \
                 denotes. Every mark sits where this viewer's window put it. A font that \
                 names no BaseFont is given one equal to the descriptor's FontName"
            }
            Self::PressAssumed => {
                "a file that draws in DeviceCMYK beside an RGB destination profile is given a \
                 DefaultCMYK over the CMYK profile this program ships (named in the report and \
                 in the file's own xmpMM:History), because ISO 19005-4 licenses DeviceCMYK there \
                 only through a device independent DefaultCMYK and nothing in the file names the \
                 press its four numbers were made for. Every DeviceCMYK colour is now read as \
                 that press prints it, which may not be the press the producer used. This is the \
                 lower bound of a print scanned back in, not an attempt to imitate printing; \
                 supply the producer's own press profile with --output-intent-profile to state \
                 it instead"
            }
            Self::RecoveredStructure => {
                "a document whose trailer, catalog or page tree node does not state the page tree \
                 is written out as the structure this viewer recovered: a catalog (the producer's dictionary \
                 where it has one) and a single page tree node listing the pages this viewer \
                 found, in the order it draws them. Every page, content stream and resource is the \
                 producer's; a page's /Parent is restated and the inheritable entries it stated \
                 no value for are copied from its old ancestors. The standard defines no \
                 recovery of a missing page tree, so another reader may find other pages or the \
                 same pages in another order"
            }
            Self::TextAsImage => {
                "a page showing an Adobe-Identity composite font the file does not embed, whose \
                 codes no ToUnicode CMap names, is replaced by a photograph of itself as this \
                 viewer's window draws it, the marks of that font blank as the window leaves them: \
                 the graphics, the images and every other mark survive as pixels rather than as \
                 content. Only a page whose text no reader can extract is photographed, so no \
                 extractable text is lost. Name the producer's program with --font to keep the \
                 page as content instead"
            }
            Self::PageAsImage => {
                "a page showing a font the file does not embed and no route of this program can \
                 carry, or holding a fault of its content stream that the target refuses, is \
                 replaced by a photograph of itself as this viewer's window draws it, whatever \
                 text it also shows: the graphics, the images and every mark the window draws \
                 survive as pixels rather than as content, and the text the page held can no \
                 longer be selected or searched. The text of each such page is named in the \
                 report, and the other pages keep theirs"
            }
            Self::TaggingOfPhotographedPages => {
                "the structure tree's leaves that name a photographed page are taken out of the \
                 tree, so a reader that reads the document by its tags no longer finds that \
                 page's text in them; every other page keeps its tags, and the number of leaves \
                 each photographed page lost is named in the report"
            }
            Self::OptionalContentOfPhotographedPages => {
                "a photographed page of a document that states optional content is drawn in the \
                 configuration this viewer shows, so each optional content group its content \
                 named is fixed in the state it was drawn in and switching the group no longer \
                 changes that page; every other page keeps its optional content, and the groups \
                 each photographed page fixed are named in the report with the state drawn"
            }
            Self::PageNotHeld => {
                "a page the page tree counts and the file does not hold where its cross-reference \
                 information places it is not in the archive: nothing is drawn in its place, and \
                 the report states how many pages the source counted and how many the archive \
                 holds"
            }
            Self::ContentAsDrawn => {
                "a run of a content stream that spells no number and that this viewer draws as \
                 the number it reads off the run's front is written as that number, so the page \
                 draws as the window drew it and keeps its text; the characters after the number \
                 are gone, and the report names each run with the number written"
            }
        }
    }

    /// The loss a caller's word names.
    #[must_use]
    pub fn parse(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|loss| loss.word() == word)
    }
}

/// The losses one decision incurs: one for most rows, and every one a rewrite costs where it
/// costs more than one.
///
/// **One decision carries every loss it incurs** (`doc/questions/A566`, `doc/adr/2262`). A
/// document's fonts are embedded by one rewrite, and one font may be drawn from a machine's face
/// while another is drawn from the shipped face narrowed to its widths — two losses with two
/// names, each of which a caller may withhold. Reporting the first alone would let a caller
/// authorise half of what the conversion does, so the decision holds the set and proceeds only
/// where every member was authorised.
///
/// Indexed by a loss's position in [`Loss::ALL`], which is its declaration order; the set
/// iterates in that order, so the report names the losses the same way every time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Losses(u64);

// Room for every loss in the bits of one word, checked where the word is chosen.
const _: () = assert!(Loss::ALL.len() <= u64::BITS as usize);

impl Losses {
    /// The set holding one loss.
    #[must_use]
    pub const fn one(loss: Loss) -> Self {
        Self(1 << (loss as u64))
    }

    /// This set with one more loss in it.
    #[must_use]
    pub const fn with(self, loss: Loss) -> Self {
        Self(self.0 | (1 << (loss as u64)))
    }

    /// Whether the set holds this loss.
    #[must_use]
    pub const fn contains(self, loss: Loss) -> bool {
        self.0 & (1 << (loss as u64)) != 0
    }

    /// Whether the set holds no loss.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// The losses in the set, in [`Loss::ALL`]'s order.
    pub fn iter(self) -> impl Iterator<Item = Loss> {
        Loss::ALL
            .into_iter()
            .filter(move |loss| self.contains(*loss))
    }

    /// The words a caller authorises the set by, separated by commas.
    #[must_use]
    pub fn words(self) -> String {
        self.iter().map(Loss::word).collect::<Vec<_>>().join(", ")
    }

    /// What the set loses, one sentence per loss, separated by semicolons.
    #[must_use]
    pub fn describe(self) -> String {
        self.iter()
            .map(Loss::describe)
            .collect::<Vec<_>>()
            .join("; ")
    }
}

/// What the caller has authorised this conversion to lose.
///
/// One field per [`Loss`] rather than a set, so that adding a loss to the table is a compile
/// error everywhere it has to be answered rather than a word nobody matched.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the field-per-loss shape is the decision the comment above states, and the lint's \
              remedy — a set — is exactly what it declines: a caller could then hold a word no \
              arm of `grants` answers. Each flag is a separate question a person answered, not a \
              state machine's state"
)]
pub struct Authorisations {
    /// Whether [`Loss::ImageSmoothing`] was authorised.
    pub image_smoothing: bool,
    /// Whether [`Loss::MetadataProperty`] was authorised.
    pub metadata_property: bool,
    /// Whether [`Loss::MetadataPacket`] was authorised.
    pub metadata_packet: bool,
    /// Whether [`Loss::AmendmentIdentifier`] was authorised.
    pub amendment_identifier: bool,
    /// Whether [`Loss::AnnotationPrinting`] was authorised.
    pub annotation_printing: bool,
    /// Whether [`Loss::Jpeg2000ColourFallback`] was authorised.
    pub jpeg2000_colour_fallback: bool,
    /// Whether [`Loss::SignatureAssertion`] was authorised.
    pub signature_assertion: bool,
    /// Whether [`Loss::ForbiddenAnnotation`] was authorised.
    pub forbidden_annotation: bool,
    /// Whether [`Loss::InteractiveBehaviour`] was authorised.
    pub interactive_behaviour: bool,
    /// Whether [`Loss::Encryption`] was authorised.
    pub encryption: bool,
    /// Whether [`Loss::PageBoundary`] was authorised.
    pub page_boundary: bool,
    /// Whether [`Loss::HiddenAnnotation`] was authorised.
    pub hidden_annotation: bool,
    /// Whether [`Loss::AppearanceStates`] was authorised.
    pub appearance_states: bool,
    /// Whether [`Loss::AutomaticStates`] was authorised.
    pub automatic_states: bool,
    /// Whether [`Loss::FieldAppearances`] was authorised.
    pub field_appearances: bool,
    /// Whether [`Loss::XfaForm`] was authorised.
    pub xfa_form: bool,
    /// Whether [`Loss::EmbeddedFile`] was authorised.
    pub embedded_file: bool,
    /// Whether [`Loss::TransferFunction`] was authorised.
    pub transfer_function: bool,
    /// Whether [`Loss::HalftoneName`] was authorised.
    pub halftone_name: bool,
    /// Whether [`Loss::Halftone`] was authorised.
    pub halftone: bool,
    /// Whether [`Loss::OverprintMode`] was authorised.
    pub overprint_mode: bool,
    /// Whether [`Loss::LanguageIdentifier`] was authorised.
    pub language_identifier: bool,
    /// Whether [`Loss::MachineFace`] was authorised.
    pub machine_face: bool,
    /// Whether [`Loss::SubstituteStretch`] was authorised.
    pub substitute_stretch: bool,
    /// Whether [`Loss::GlyphsFromToUnicode`] was authorised.
    pub glyphs_from_to_unicode: bool,
    /// Whether [`Loss::LayoutFromThisViewer`] was authorised.
    pub layout_from_this_viewer: bool,
    /// Whether [`Loss::PressAssumed`] was authorised.
    pub press_assumed: bool,
    /// Whether [`Loss::RecoveredStructure`] was authorised.
    pub recovered_structure: bool,
    /// Whether [`Loss::TextAsImage`] was authorised.
    pub text_as_image: bool,
    /// Whether [`Loss::PageAsImage`] was authorised.
    pub page_as_image: bool,
    /// Whether [`Loss::TaggingOfPhotographedPages`] was authorised.
    pub tagging_of_photographed_pages: bool,
    /// Whether [`Loss::OptionalContentOfPhotographedPages`] was authorised.
    pub optional_content_of_photographed_pages: bool,
    /// Whether [`Loss::PageNotHeld`] was authorised.
    pub page_not_held: bool,
    /// Whether [`Loss::PageNotHeld`], where it is taken, is also stated on the appended page
    /// `append-statement` composes (`doc/adr/2596`). No loss of its own: it answers *where* a
    /// taken loss is said, and `grants` reads [`Self::page_not_held`] alone.
    pub page_not_held_stated: bool,
    /// Whether [`Loss::ContentAsDrawn`] was authorised.
    pub content_as_drawn: bool,
}

impl Authorisations {
    /// Whether this loss was authorised.
    #[must_use]
    pub const fn grants(self, loss: Loss) -> bool {
        match loss {
            Loss::ImageSmoothing => self.image_smoothing,
            Loss::MetadataProperty => self.metadata_property,
            Loss::MetadataPacket => self.metadata_packet,
            Loss::AmendmentIdentifier => self.amendment_identifier,
            Loss::AnnotationPrinting => self.annotation_printing,
            Loss::Jpeg2000ColourFallback => self.jpeg2000_colour_fallback,
            Loss::SignatureAssertion => self.signature_assertion,
            Loss::ForbiddenAnnotation => self.forbidden_annotation,
            Loss::InteractiveBehaviour => self.interactive_behaviour,
            Loss::Encryption => self.encryption,
            Loss::PageBoundary => self.page_boundary,
            Loss::HiddenAnnotation => self.hidden_annotation,
            Loss::AppearanceStates => self.appearance_states,
            Loss::AutomaticStates => self.automatic_states,
            Loss::FieldAppearances => self.field_appearances,
            Loss::XfaForm => self.xfa_form,
            Loss::EmbeddedFile => self.embedded_file,
            Loss::TransferFunction => self.transfer_function,
            Loss::HalftoneName => self.halftone_name,
            Loss::Halftone => self.halftone,
            Loss::OverprintMode => self.overprint_mode,
            Loss::LanguageIdentifier => self.language_identifier,
            Loss::MachineFace => self.machine_face,
            Loss::SubstituteStretch => self.substitute_stretch,
            Loss::GlyphsFromToUnicode => self.glyphs_from_to_unicode,
            Loss::LayoutFromThisViewer => self.layout_from_this_viewer,
            Loss::PressAssumed => self.press_assumed,
            Loss::RecoveredStructure => self.recovered_structure,
            Loss::TextAsImage => self.text_as_image,
            Loss::PageAsImage => self.page_as_image,
            Loss::TaggingOfPhotographedPages => self.tagging_of_photographed_pages,
            Loss::OptionalContentOfPhotographedPages => self.optional_content_of_photographed_pages,
            Loss::PageNotHeld => self.page_not_held,
            Loss::ContentAsDrawn => self.content_as_drawn,
        }
    }

    /// Whether every loss of the set was authorised.
    #[must_use]
    pub fn grants_all(self, losses: Losses) -> bool {
        losses.iter().all(|loss| self.grants(loss))
    }

    /// Authorises one loss.
    pub const fn authorise(&mut self, loss: Loss) {
        match loss {
            Loss::ImageSmoothing => self.image_smoothing = true,
            Loss::MetadataProperty => self.metadata_property = true,
            Loss::MetadataPacket => self.metadata_packet = true,
            Loss::AmendmentIdentifier => self.amendment_identifier = true,
            Loss::AnnotationPrinting => self.annotation_printing = true,
            Loss::Jpeg2000ColourFallback => self.jpeg2000_colour_fallback = true,
            Loss::SignatureAssertion => self.signature_assertion = true,
            Loss::ForbiddenAnnotation => self.forbidden_annotation = true,
            Loss::InteractiveBehaviour => self.interactive_behaviour = true,
            Loss::Encryption => self.encryption = true,
            Loss::PageBoundary => self.page_boundary = true,
            Loss::HiddenAnnotation => self.hidden_annotation = true,
            Loss::AppearanceStates => self.appearance_states = true,
            Loss::AutomaticStates => self.automatic_states = true,
            Loss::FieldAppearances => self.field_appearances = true,
            Loss::XfaForm => self.xfa_form = true,
            Loss::EmbeddedFile => self.embedded_file = true,
            Loss::TransferFunction => self.transfer_function = true,
            Loss::HalftoneName => self.halftone_name = true,
            Loss::Halftone => self.halftone = true,
            Loss::OverprintMode => self.overprint_mode = true,
            Loss::LanguageIdentifier => self.language_identifier = true,
            Loss::MachineFace => self.machine_face = true,
            Loss::SubstituteStretch => self.substitute_stretch = true,
            Loss::GlyphsFromToUnicode => self.glyphs_from_to_unicode = true,
            Loss::LayoutFromThisViewer => self.layout_from_this_viewer = true,
            Loss::PressAssumed => self.press_assumed = true,
            Loss::RecoveredStructure => self.recovered_structure = true,
            Loss::TextAsImage => self.text_as_image = true,
            Loss::PageAsImage => self.page_as_image = true,
            Loss::TaggingOfPhotographedPages => self.tagging_of_photographed_pages = true,
            Loss::OptionalContentOfPhotographedPages => {
                self.optional_content_of_photographed_pages = true;
            }
            Loss::PageNotHeld => self.page_not_held = true,
            Loss::ContentAsDrawn => self.content_as_drawn = true,
        }
    }
}

/// Why a requirement the document failed cannot be answered by this conversion.
///
/// Three reasons, and they are three different facts about the world rather than three shades
/// of *no*. section 2.0 of `doc/pdf-a-conversion-limits.md` is the argument for separating them: a
/// user who is told "not this target" has somewhere to go, and one who is told "not yet" knows
/// the answer will change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Because {
    /// The fix is on the far side of ADR 0816's fence: it would edit a content stream, invent a
    /// mark, or author content the source does not carry. Section 5.2 of
    /// `doc/pdf-a-conversion-limits.md` is the first of those and section 5.1 the third — the
    /// converter does not rewrite the producer's page to reach conformance, and it does not
    /// write down semantics nobody produced.
    ///
    /// **Not a gap.** Nothing later in this project's life closes one of these, which is what
    /// separates it from [`Self::NotBuiltYet`]: a user told this has been told the final
    /// answer, and `doc/pdf-a-conversion-limits.md` section 1.1's table is what they do next.
    TheFence(&'static str),
    /// This converter does not implement the fix yet, and the sentence names which slice owes
    /// it.
    ///
    /// Honest rather than a stub: `doc/pdf-a-conversion-limits.md` names the resource or the
    /// decision each of these waits on, and a converter that wrote the file anyway would be
    /// asserting a conformance it had not reached.
    NotBuiltYet(&'static str),
    /// No conforming file can be made from this document *for the target asked for*, whatever
    /// is implemented. Section 2.5's implementation limits and section 2.3's external data are the
    /// standing cases, and section 1.1's table is what a user does about it.
    NotThisTarget(&'static str),
    /// The conversion could have been carried out and the **caller asked that it not be**.
    ///
    /// The fourth kind of *no*, and the one that is not a fact about the document or about this
    /// program at all: `doc/pdf-a-conversion-limits.md` section 4.9 offers `--no-substitute` to
    /// the user who would rather a font be refused than replaced, and a report that said "not
    /// built yet" about a flag they had just typed would be wrong in the most confusing
    /// direction available. Unlike [`Self::NotBuiltYet`] the answer changes the moment the flag
    /// comes off, and unlike [`Self::NotThisTarget`] no other target helps.
    Declined(&'static str),
    /// The caller's configuration answers this requirement with an external program, and the
    /// program has not been run yet.
    ///
    /// **The fifth kind of *no*, and the only one that is about this pass rather than about the
    /// world.** `doc/questions/A54`: `apply` returns the invocation as data and never starts a
    /// process, so a conversion whose configuration names a tool is answered in two passes — the
    /// first returns [`crate::Report::requested`], the caller executes it through the shared
    /// executor, and the second is handed the results. A caller that applies once and stops sees
    /// this; a caller that does what `quorra-transform archive` does never does.
    AwaitingTool(&'static str),
}

impl Because {
    /// The reason, in one sentence.
    #[must_use]
    pub const fn sentence(self) -> &'static str {
        match self {
            Self::TheFence(why)
            | Self::NotBuiltYet(why)
            | Self::NotThisTarget(why)
            | Self::Declined(why)
            | Self::AwaitingTool(why) => why,
        }
    }

    /// A stable word for the report's machine-readable form.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::TheFence(_) => "the-fence",
            Self::NotBuiltYet(_) => "not-built-yet",
            Self::NotThisTarget(_) => "not-this-target",
            Self::Declined(_) => "declined",
            Self::AwaitingTool(_) => "awaiting-tool",
        }
    }
}

/// What the converter decided about one requirement the document did not meet.
///
/// **The middle stage's whole product.** section 2, section 3 and section 4 of
/// `doc/pdf-a-conversion-limits.md` are three different answers to a failed requirement — a
/// refusal, a loss somebody has to authorise, and a default that is right — and a converter
/// that ran them together would be unable to say which one a given file got.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// The *Mechanical* class: the requirement is met by a rewrite that loses nothing, every time.
    Mechanical(Rewrite),
    /// section 3: it can be done, it loses something, and the caller authorised every loss.
    Authorised {
        /// What is lost.
        losses: Losses,
        /// The rewrite that loses it.
        rewrite: Rewrite,
    },
    /// section 3: the same, and at least one of its losses nobody authorised. The conversion
    /// stops.
    Unauthorised {
        /// What would have been lost.
        losses: Losses,
        /// The rewrite that would have lost it.
        rewrite: Rewrite,
    },
    /// section 4: the requirement is met by writing down an interpretation the standard defines,
    /// which changes what the file *asserts* without changing what it holds.
    ///
    /// **The class `doc/adr/0927`'s four permissions belong to**, and the reason it is not
    /// [`Self::Mechanical`]: nothing is lost, and something all the same is different afterwards.
    /// `doc/questions/A48` draws the line these sit on — *state an interpretation the standard
    /// defines; never fill in an absence* — and each of the four is conditional on the same
    /// thing, that what was written is reported. So the sentence saying what a different reader
    /// may now do differently rides in the decision itself rather than being left to a caller to
    /// remember.
    Stated {
        /// The rewrite that states it.
        rewrite: Rewrite,
        /// What is now asserted that was not before, in one sentence for a person.
        reinterprets: &'static str,
    },
    /// section 2: no file is written, and the reason says which of three kinds of *no* this is.
    Refused(Because),
    /// `doc/rfc/0007` section 2: the **operator's** configuration answered this requirement, and the
    /// answer was carried out.
    ///
    /// **Not [`Self::Stated`]**, and the difference is the whole reason for a variant of its own.
    /// A `Stated` decision writes down an interpretation the *standard* defines, which is
    /// `doc/questions/A48`'s permitted half. These two do not:
    ///
    /// - `derive` makes a representation that was not in the document before, which stands on the
    ///   far side of `A48`'s line by construction — the owner allowed it in `A55` on terms rather
    ///   than by denying that it is what it is;
    /// - `supply` writes a fact **the operator** holds and the document does not, which `A48` does
    ///   not bind at all because it binds *this program's* guessing.
    ///
    /// Either way the file afterwards says something neither the producer nor the standard said,
    /// and `warns` is the sentence a person is owed about it.
    Configured {
        /// Which of `doc/rfc/0007` section 2's kinds.
        kind: super::RemedyKind,
        /// The rewrite that carries it out.
        rewrite: Rewrite,
        /// What the operator is agreeing to, in one sentence for a person.
        warns: &'static str,
    },
}

impl Decision {
    /// Whether this decision lets the conversion proceed.
    #[must_use]
    pub const fn proceeds(self) -> bool {
        matches!(
            self,
            Self::Mechanical(_)
                | Self::Stated { .. }
                | Self::Authorised { .. }
                | Self::Configured { .. }
        )
    }

    /// The rewrite this decision performs, where it performs one.
    #[must_use]
    pub const fn rewrite(self) -> Option<Rewrite> {
        match self {
            Self::Mechanical(rewrite)
            | Self::Stated { rewrite, .. }
            | Self::Authorised { rewrite, .. }
            | Self::Configured { rewrite, .. } => Some(rewrite),
            Self::Unauthorised { .. } | Self::Refused(_) => None,
        }
    }

    /// A stable word for the report's machine-readable form.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Mechanical(_) => "mechanical",
            Self::Stated { .. } => "stated",
            Self::Authorised { .. } => "authorised",
            Self::Unauthorised { .. } => "unauthorised",
            Self::Refused(_) => "refused",
            Self::Configured { .. } => "configured",
        }
    }
}

/// What the converter knows how to do about one of the validator's requirements.
///
/// The answer half of [`Remedy`], before the caller's authorisations are known. `Decision` is
/// what it becomes once they are.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Answer {
    /// A rewrite that loses nothing.
    Mechanical(Rewrite),
    /// A rewrite that states an interpretation, with the sentence [`Decision::Stated`] carries.
    ///
    /// The `Option<DeviceFamily>` is the one thing a static table cannot answer about an output
    /// intent: ISO 19005 section 6.2.4.3 licenses `DeviceRGB` through an **RGB** destination
    /// profile and `DeviceCMYK` through a **CMYK** one, so which of those rows an intent answers
    /// depends on the profile in hand rather than on the requirement alone. `None` is a row any
    /// PDF/A output intent answers whatever its profile is.
    Stated(Option<DeviceFamily>, Rewrite, &'static str),
    /// A rewrite that loses something, which the caller must authorise.
    Loses(Loss, Rewrite),
    /// ISO 19005-2 section 6.2.4.3's `DeviceCMYK` sentence, which admits two licences this verb
    /// can reach and picks between them on the profile in hand.
    ///
    /// The only row of [`REMEDIES`] whose answer is not a rewrite, because the sentence itself
    /// offers two: a PDF/A output intent holding a **CMYK** destination profile, which is the
    /// right answer and needs a profile only the document's owner can supply, and a
    /// **`DeviceN`-based** `/DefaultCMYK`, which the subclause's NOTE 2 makes device independent
    /// and which needs nothing but the standard. [`decide`] takes the first where the profile in
    /// hand is a CMYK one and the second otherwise; `doc/pdf-a-conversion-limits.md` section 10.1
    /// is the reading, and `doc/questions/A48` the permission.
    ///
    /// **Part 4 states this sentence without the `DeviceN` half**, so its own row is an ordinary
    /// [`Self::Stated`] naming the output intent and a document whose only failure is
    /// `DeviceCMYK` under PDF/A-4 is refused unless a CMYK profile is supplied. That is the
    /// standard's difference rather than this converter's.
    CmykUnderPartTwo,
    /// ISO 19005-4 section 6.2.4.3's `DeviceCMYK` sentence, which admits a CMYK output intent or
    /// a device independent `/DefaultCMYK` and states no `DeviceN` half.
    ///
    /// [`decide`] takes the output intent where the profile in hand is a CMYK one, and otherwise
    /// the `/DefaultCMYK` over the press profile the operator supplied, where they supplied one
    /// (`doc/adr/2130`). With none supplied it writes the profile this program ships as an
    /// authorised [`Loss::PressAssumed`] where the caller grants that loss, and otherwise the
    /// requirement is refused with the sentence that says what the operator can do
    /// (`doc/adr/2518`).
    CmykUnderPartFour,
    /// ISO 19005 section 6.2.4.3's `DeviceRGB` sentence, and every row that sends a space to it,
    /// which admits a device independent `/DefaultRGB` beside an RGB output intent (`doc/adr/2114`).
    ///
    /// Both parts state the two licences, so one variant serves both. [`decide`] takes the
    /// `/DefaultRGB` where the preparation built one — the profile in hand is of another family
    /// and the document draws in `DeviceRGB` — and otherwise the row's answer is the output
    /// intent's, exactly as [`Self::Stated`] with this family would give it. `Some(Rgb)` for the
    /// two rows that are `DeviceRGB`'s own; `None` for the rows — a group's `CS`, a base, an
    /// alternate — whose family is the document's, which the output's own verdict settles.
    RgbUnderPartTwoOrFour(Option<DeviceFamily>),
    /// The requirement asks other requirements' rules again of a narrower population, and its
    /// answer is theirs.
    ///
    /// **Routing rather than a rewrite**, and `doc/pdf-a-mitigations.md` section 7 named the
    /// absence of it as a refusal the conversion would in fact have fixed: ISO 19005-2 section
    /// 6.4.3 and ISO 19005-4 section 6.5.1 say that a signature field's widget meets the
    /// annotation flag and appearance rules, which are three rows of this table already — so a
    /// decision taken per requirement identifier refused a document every one of whose actual
    /// failures had an answer.
    ///
    /// The rows named here are never themselves [`Self::AsUnderlying`]: a compound of a compound
    /// would need an order this table does not have, and [`decide_as_underlying`] passes over
    /// one rather than following it.
    AsUnderlying(&'static [&'static str]),
    /// The inner answer where this document can take it, and the stated rewrite, with its
    /// sentence, where it cannot and that rewrite's own proof holds.
    ///
    /// For the three rows that hold a rendered TrueType font's encoding to ISO 32000-2 §9.6.5.4's
    /// route, the second is the restated `cmap` ([`Rewrite::TrueTypeCmapRestated`]): each row has
    /// a narrow rewrite of the dictionary's `/Encoding` that proves itself only for the codes the
    /// pages showed, and the symbolic route states every code's glyph in the program without a
    /// name, so a font the narrow rewrite refuses meets the same row through it (`doc/adr/2520`).
    /// For the `.notdef` row it is the glyph given ([`Rewrite::NotdefGlyphGiven`]), where the
    /// inner answer is a Type 3 font's rename (`doc/adr/2543`). The inner answer is tried first
    /// because it changes less: a name in the dictionary rather than a program and a descriptor.
    OrStated(&'static Answer, Rewrite, &'static str),
}

impl Answer {
    /// Every rewrite this answer might perform, whatever the caller has authorised.
    ///
    /// Three rather than one because [`Self::CmykUnderPartTwo`] can be answered three ways and
    /// [`Prepared`] has to build whichever [`decide`] turns out to choose — which it cannot know
    /// before the constructions exist.
    pub(super) const fn rewrites(self) -> [Option<Rewrite>; 3] {
        match self {
            Self::Mechanical(rewrite) | Self::Stated(_, rewrite, _) | Self::Loses(_, rewrite) => {
                [Some(rewrite), None, None]
            }
            Self::CmykUnderPartTwo => [
                Some(Rewrite::OutputIntent),
                Some(Rewrite::SuppliedDefaultCmyk),
                Some(Rewrite::DefaultCmyk),
            ],
            Self::CmykUnderPartFour => [
                Some(Rewrite::OutputIntent),
                Some(Rewrite::SuppliedDefaultCmyk),
                None,
            ],
            Self::RgbUnderPartTwoOrFour(Some(_)) => {
                [Some(Rewrite::OutputIntent), Some(Rewrite::DefaultRgb), None]
            }
            // A row whose family is the document's sends a group's `CS`, a base or an alternate
            // to section 6.2.4.3, and §11.6.6's Table 145 remaps a group's device space through
            // a `/DefaultCMYK` as it does through a `/DefaultRGB` (`doc/adr/2130`).
            Self::RgbUnderPartTwoOrFour(None) => [
                Some(Rewrite::OutputIntent),
                Some(Rewrite::DefaultRgb),
                Some(Rewrite::SuppliedDefaultCmyk),
            ],
            // None of its own: the rewrites are the underlying rows', and those rows are in this
            // same table, so a preparation asking "did any failed requirement want this rewrite"
            // has already been answered by them.
            Self::AsUnderlying(_) => [None, None, None],
            Self::OrStated(inner, rewrite, _) => {
                let [first, ..] = inner.rewrites();
                [first, Some(rewrite), None]
            }
        }
    }
}

/// One row of the converter's decision table: what to do about one of the validator's
/// requirements.
///
/// **Data for `pdf_archive::Requirement`'s reason.** The remedies are a few dozen, most are
/// one line, and every one has to be checkable against the requirement it answers by somebody
/// who is not reading Rust. A requirement absent from this table is not forgotten: it is
/// refused by name, with its own clause and its own sentence, by [`decide`].
#[derive(Debug, Clone, Copy)]
pub(super) struct Remedy {
    /// The `pdf_archive` requirement identifier this answers.
    pub(super) requirement: &'static str,
    /// What is done about it.
    pub(super) answer: Answer,
}

/// Why a colour requirement an output intent of another family answers is refused.
const WRONG_FAMILY: &str = "this requirement is licensed by a destination profile of its own \
     colour family, and the profile this conversion has is of another. Where the file states no \
     output intent of its own, supply the right one with --output-intent-profile";

/// [`REMEDIES`]'s rows, one slice a family.
const REMEDIES_BY_FAMILY: &[&[Remedy]] = &[
    fonts::REMEDIES,
    colour_and_graphics::REMEDIES,
    content_stream_fences::REMEDIES,
    files_and_structure::REMEDIES,
    annotations_and_attachments::REMEDIES,
    metadata::REMEDIES,
];

/// Every requirement this converter can answer, and how.
///
/// `doc/pdf-a-conversion-limits.md` section 4.7's table is the list this slice takes, less the rows
/// that need something shipped. Three of that table's rows are absent on purpose and each has
/// its reason written where it belongs rather than here:
///
/// - **The `Crypt` filter.** The row says its removal "follows from section 3.5" — encryption — and
///   the requirement itself permits an `Identity` `Crypt` filter to stay. So a filter chain
///   this converter re-encodes for the `LZWDecode` rewrite drops its `Crypt` stages, and a
///   conforming `Identity` one is left alone: **nothing is changed that no failed requirement
///   asked for**, which is the rule that makes "a document already conforming is not rewritten"
///   true by construction rather than by a special case.
/// - **Hexadecimal string padding, `endstream`/`endobj` whitespace, `/Length`, the indirect
///   object syntax and the cross-reference keyword's line endings.** The row's own note says
///   it: "the serializer emits this shape anyway". Every one of them is a property of
///   `pdf_syntax::write`, so the remedy is the writer and the rewrite is the whole-file
///   rewrite. They are answered by [`Answer::Mechanical`] with [`Rewrite::FileHeader`]'s
///   sibling — see [`WRITER_EMITS`], which is the list of requirements the writer satisfies by
///   construction and which therefore need no rewrite of their own.
/// - **Names that are not valid UTF-8.** The row calls it a **Refusal**, not a rewrite, and
///   says why: renaming a font or a colourant would break the references that use it.
pub(super) const REMEDIES: &[Remedy] =
    &concatenated::<Remedy, { rows(REMEDIES_BY_FAMILY) }>(REMEDIES_BY_FAMILY);

/// What a `REMEDIES` row's own answer does not settle by itself.
///
/// `doc/adr/1209`. Most rows are decided by the table: a `Mechanical` row is mechanical for every
/// document that fails the requirement, and a `Loses` row always costs the same thing. Two rows
/// are not, and both were invisible to the enumeration because of it — [`super::census::standing`]
/// read them as ordinary `Mechanical` rows and [`super::config::sites`] lists only the sites a
/// configuration has something to say about, so a site whose answer an operator still has to
/// supply something for had dropped off the listing while documents went on being refused at it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Conditional {
    /// The rewrite is mechanical, and only over bytes the **caller** hands in.
    ///
    /// `doc/adr/1199`: a stream whose `/F` puts its data on somebody else's disk can be brought
    /// inside the file without changing anything a reader computes — but only once somebody has
    /// fetched it, which this program does not do and will not acquire.
    CallerSuppliesTheBytes,
    /// The rewrite is mechanical for some documents and costs [`Loss`] for others, and which is
    /// a property of the file rather than of the requirement.
    ///
    /// `doc/adr/1210`: removing an out-of-range optional page boundary moves no mark where
    /// §14.11.2.1 has already made the entry its intersection with the media box, and changes
    /// what a reader shows where it has not.
    PerDocument(Losses),
    /// The rewrite states an interpretation under one answer the site admits and costs [`Loss`]
    /// under the other, and which is the **operator's** to say rather than the file's.
    ///
    /// `doc/adr/1257`: §12.7.3's Table 224 makes clearing `/NeedAppearances` a claim that
    /// appearance streams have been provided, so `construct = true` builds them and loses
    /// nothing, and `construct = false` clears the flag alone and leaves a field its producer
    /// gave no stream blank.
    PerConfiguration(Losses),
}

impl Conditional {
    /// What the answer waits on, in one clause for the enumeration a person reads.
    #[must_use]
    pub const fn describe(self) -> &'static str {
        match self {
            Self::CallerSuppliesTheBytes => {
                "mechanical, and only over bytes the caller resolved: the data is outside the \
                 file and nothing in the document supplies it"
            }
            Self::PerDocument(_) => {
                "mechanical for some documents and an authorised loss for others, decided per \
                 file against the clause rather than by the decision table"
            }
            Self::PerConfiguration(_) => {
                "lossless under one answer this site admits and an authorised loss under the \
                 other, decided by the configuration rather than by the file"
            }
        }
    }

    /// The losses this answer may cost where the document makes it costly: empty for a row
    /// waiting on bytes rather than on a document.
    #[must_use]
    pub const fn losses(self) -> Losses {
        match self {
            Self::CallerSuppliesTheBytes => Losses(0),
            Self::PerDocument(losses) | Self::PerConfiguration(losses) => losses,
        }
    }
}

/// [`CONDITIONAL`]'s rows, one slice a family.
const CONDITIONAL_BY_FAMILY: &[&[(&str, Conditional)]] = &[
    fonts::CONDITIONAL,
    colour_and_graphics::CONDITIONAL,
    content_stream_fences::CONDITIONAL,
    files_and_structure::CONDITIONAL,
    annotations_and_attachments::CONDITIONAL,
    metadata::CONDITIONAL,
];

/// The rows of [`REMEDIES`] whose answer the table alone does not settle.
///
/// Read by [`super::census::standing`], so that a site whose `Mechanical` answer is conditional
/// stays enumerable in `--remedy-sites` and countable in the census; and by
/// `super::config`'s loss table, so that a `discard` at a site whose condition is a per-document
/// loss authorises that loss exactly as a `discard` at an ordinary `Loses` row does.
pub(super) const CONDITIONAL: &[(&str, Conditional)] =
    &concatenated::<(&str, Conditional), { rows(CONDITIONAL_BY_FAMILY) }>(CONDITIONAL_BY_FAMILY);

/// What a row's answer waits on, for the two rows that wait on anything.
#[must_use]
pub fn conditional(requirement: &str) -> Option<Conditional> {
    CONDITIONAL
        .iter()
        .find(|(id, _)| *id == requirement)
        .map(|(_, held)| *held)
}

/// Whether this requirement's answer needs nothing authorised to be carried out.
///
/// The question `super::config::Configuration::unbuilt` asks of a `discard`. That word says *do
/// not stop the conversion here; I accept what it costs* — so at a row answered by a rewrite that
/// loses nothing, or by one that writes down an interpretation the standard defines, the
/// instruction is already carried out and the site never refuses. Reporting such an answer as one
/// this version does not carry out says the site stays refused, and it does not.
///
/// **Two rows are not these**, and each for its own reason. [`Answer::Loses`] is what a `discard`
/// authorises, and it is built exactly where the loss table names it. [`Answer::AsUnderlying`]
/// routes to rules whose answers are its own, so whether a `discard` here reaches them is a
/// question about the other rows rather than about this one, and it keeps its name in the
/// listing. A `Mechanical` answer waiting on bytes nobody in this program can fetch is not one
/// either (`doc/adr/1199`).
#[must_use]
pub(super) fn answered_without_authorisation(requirement: &str) -> bool {
    if matches!(
        conditional(requirement),
        Some(Conditional::CallerSuppliesTheBytes)
    ) {
        return false;
    }
    REMEDIES.iter().any(|remedy| {
        remedy.requirement == requirement
            && matches!(
                remedy.answer,
                Answer::Mechanical(_)
                    | Answer::Stated(..)
                    | Answer::OrStated(..)
                    | Answer::CmykUnderPartTwo
                    | Answer::CmykUnderPartFour
                    | Answer::RgbUnderPartTwoOrFour(_)
            )
    })
}

/// The requirements the *writer* satisfies, because every output of this verb is a new file.
///
/// `doc/pdf-a-conversion-limits.md` section 4.7's note for each of them is "the serializer emits
/// this shape anyway", and that is exactly what makes them a separate list from [`REMEDIES`]:
/// nothing decides anything about the document, and there is no rewrite to report per place. A
/// whole-file rewrite is the remedy, and the report says so once.
///
/// Each is still *reported*, because a requirement the input failed is a fact about the input
/// whether the fix cost anything or not.
pub(super) const WRITER_EMITS: &[&str] = &[
    // ISO 19005-2 section 6.1.6, ISO 19005-4 section 6.1.5: `pdf_syntax::write` writes every
    // string in §7.3.4.3's hexadecimal form, two digits a byte.
    "file-structure/hexadecimal-string-digits",
    "file-structure/hexadecimal-string-holds-only-digits",
    // ISO 19005-2 section 6.1.7.1 and section 6.1.9, ISO 19005-4 section 6.1.6.1 and section
    // 6.1.8: the keyword line endings and the object syntax the serializer emits.
    "file-structure/stream-keyword-line-endings",
    "file-structure/indirect-object-syntax",
    "file-structure/cross-reference-keyword-line-endings",
    // Both parts: `/Length` is re-derived as a direct integer from the bytes actually written.
    "file-structure/stream-length-matches-the-data",
    // ISO 19005-2 section 6.1.3, ISO 19005-4 section 6.1.3: nothing follows the last `%%EOF`
    // this writer emits.
    "file-structure/nothing-after-the-last-end-of-file-marker",
    // ISO 19005-2 section 6.1.3, ISO 19005-4 section 5.1: §14.4's pair of identifiers, which
    // `pdf_syntax::serialize` writes into the trailer of every file it creates — the source's
    // own permanent identifier where it had one, and a digest of the bytes written for the
    // changing half — answered by reading the serializer rather than by building anything (ADR
    // 0955).
    "file-structure/file-identifier",
    // ISO 19005-2 section 6.1.13's object-count limit, and the one row of that subclause with a
    // remedy — `doc/pdf-a-mitigations.md` section 13.3. The serializer writes the objects the
    // *converted* document reaches and nothing else, so every object no reference reaches is
    // gone from the output before the count is taken. A file over the limit because it
    // accumulated orphans across twenty years of incremental updates is therefore converted with
    // nothing lost and nobody asked to authorise anything.
    //
    // **Where the objects are genuinely reachable this does not help**, and nothing would: the
    // count is then a fact about what the document holds, and `doc/adr/0947`'s third stage
    // refuses the file rather than writing one wearing a claim it has not earned. The nine other
    // rows of the subclause stay refused with `IMPLEMENTATION_LIMITS`, whose sentence says what
    // this one no longer needs to — ask for PDF/A-4, which states no implementation limits.
    "implementation-limits/indirect-object-count",
];

/// The requirements a file states the PDF/A identification schema to meet.
///
/// `doc/rfc/0007` section 4.7.2: a departed conversion omits the identification by default, so the
/// output states no `pdfaid:*` and fails exactly these requirements — deliberately, because the
/// file does not claim to be PDF/A. Stage three tolerates them for a departed, non-claiming
/// conversion (and only then), which is `super::stands_as_departed`'s second clause. It is
/// `metadata/catalog-metadata-stream` and `metadata/identification-amendment-form`'s *absence*
/// that makes this the claim rather than the packet: the packet is still written, and an amendment
/// identifier is not a claim to conform.
pub(super) const IDENTIFICATION_CLAIM: &[&str] = &[
    "metadata/identification-schema-prefix",
    "metadata/identification-part-number",
    "metadata/identification-conformance-level",
    "metadata/identification-declares-level-a",
    "metadata/identification-part-number-four",
    "metadata/identification-revision-year",
    "metadata/identification-states-no-flavour",
    "metadata/identification-declares-flavour-e",
    "metadata/identification-declares-flavour-f",
];

/// One half of a requirement that splits into two, with the answer this converter has for it.
///
/// `doc/rfc/0007` section 5b.2 and `doc/pdf-a-mitigations.md` section 14's first finding: seven
/// requirements fail for two different reasons under one identifier, and the two reasons take
/// different answers. A configuration that could only key on the identifier would take an operator
/// who answered the safe half to have answered the dangerous one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Shape {
    /// The word a `[site."…".shape."<name>"]` header states.
    pub(super) name: &'static str,
    /// Whether [`REMEDIES`] answers this half. The other half keeps the refusal its requirement
    /// carries, which is what makes a row for it inert rather than a promise.
    pub(super) answered: bool,
}

/// [`SHAPES`]'s rows, one slice a family.
const SHAPES_BY_FAMILY: &[&[(&str, &[Shape])]] = &[
    colour_and_graphics::SHAPES,
    files_and_structure::SHAPES,
    annotations_and_attachments::SHAPES,
];

/// The requirements that split into two shapes, and which half this converter answers.
///
/// **Read beside [`REMEDIES`], because that is what decides each row's `answered`.** A requirement
/// here is one whose `REMEDIES` entry — or whose absence from it — is true of one half only:
/// `graphics/content-streams-have-an-explicit-resources-dictionary` has a `Mechanical` answer for a
/// page's inherited dictionary (§7.7.3.4 resolves it already, so copying it down changes nothing a
/// reader sees) and for a form `XObject`'s, which §7.8.3 ends at the page's resources
/// (`doc/adr/2500`).
///
/// `/CIDToGIDMap` is the finding's seventh distinction and is deliberately absent: it differs by
/// *base edition*, which is a property of the target, so `doc/rfc/0007` section 4.6's target
/// qualifier already states it and a second spelling would let one row disagree with the other.
///
/// ADR 1211 is the argument, and what `answered` decides is in `config::applies_to`.
pub(super) const SHAPES: &[(&str, &[Shape])] =
    &concatenated::<(&str, &[Shape]), { rows(SHAPES_BY_FAMILY) }>(SHAPES_BY_FAMILY);

/// The shapes a requirement splits into, or nothing where it does not split.
pub(super) fn shapes(requirement: &str) -> &'static [Shape] {
    SHAPES
        .iter()
        .find(|(id, _)| *id == requirement)
        .map_or(&[], |(_, shapes)| *shapes)
}

/// Every requirement identifier this converter answers, from both tables.
///
/// Public for one test — `tests/archive.rs` compares it with the identifiers `pdf_archive`
/// actually states — because a typo in a table key does not fail to compile. It quietly becomes
/// a requirement nobody answers, which this verb then refuses for a reason no reader could see.
#[must_use]
pub fn answered() -> Vec<&'static str> {
    let mut out: Vec<&'static str> = REMEDIES.iter().map(|remedy| remedy.requirement).collect();
    out.extend_from_slice(WRITER_EMITS);
    out
}

/// Every requirement identifier this converter refuses with a sentence of its own.
///
/// Public for the same test and for the same reason as [`answered`]: a typo in one of these keys
/// does not fail to compile, it quietly demotes a refusal that had an argument behind it to
/// `NOT_BUILT_YET`'s "a later slice owes this" — which is the one sentence every row of
/// `REFUSED_BY_NAME` exists to avoid saying.
#[must_use]
pub fn refused_by_name() -> Vec<&'static str> {
    REFUSED_BY_NAME.iter().map(|(id, _)| *id).collect()
}

/// [`REFUSED_BY_NAME`]'s rows, one slice a family.
const REFUSED_BY_NAME_BY_FAMILY: &[&[(&str, Because)]] = &[
    fonts::REFUSED_BY_NAME,
    colour_and_graphics::REFUSED_BY_NAME,
    content_stream_fences::REFUSED_BY_NAME,
    files_and_structure::REFUSED_BY_NAME,
    annotations_and_attachments::REFUSED_BY_NAME,
    metadata::REFUSED_BY_NAME,
];

/// Every requirement this converter refuses with a sentence written for that requirement.
///
/// **A third table beside [`REMEDIES`] and [`WRITER_EMITS`], and the reason it exists is that
/// [`NOT_BUILT_YET`]'s sentence tells a reader almost nothing.** Each row here says which of
/// four things the refusal is: ADR 0816's fence, which no slice of this converter will ever
/// cross; a target that conforms where the one asked for cannot; a caller's own `--no-substitute`;
/// or a rewrite this converter owes, *named*, so that "not yet" comes with what it is waiting on.
/// Telling a user "not yet" about a document that will never convert sends them back tomorrow
/// for the same answer, and telling them "not yet" without saying what for tells them nothing at
/// all.
///
/// A requirement in none of the three tables is still refused — by [`decide`], with
/// [`NOT_BUILT_YET`] — so nothing here is load-bearing for safety. What it is load-bearing for
/// is the report saying something true. `super::census` is what counts the requirements that
/// have reached none of the three, and `tests/archive_unconsidered.txt` holds that count where
/// it is.
pub(super) const REFUSED_BY_NAME: &[(&str, Because)] = &concatenated::<
    (&str, Because),
    { rows(REFUSED_BY_NAME_BY_FAMILY) },
>(REFUSED_BY_NAME_BY_FAMILY);

/// Why a compound row is refused where none of the rules it names failed.
///
/// The one case [`Answer::AsUnderlying`] cannot route: the compound's own predicate found
/// something at a population the rules it names did not reach. A signature field's widget that no
/// page's `/Annots` array holds is the standing shape — `pdf_archive` walks the annotation rules
/// over the pages and the signature rule over the form's field tree, and a widget in the second
/// and not the first is a document whose annotation this conversion never sees.
const NOTHING_UNDERLYING_FAILED: &str = "this requirement asks other rules of this same table \
     again, of a narrower population, and every one of those rules passed. So the failure is at \
     something the rules themselves did not reach — for a signature field's widget, an \
     annotation the interactive form names and no page's Annots array holds. This conversion \
     rewrites annotations the pages reach, and one reachable only through the field tree is not \
     among them";

/// The sentence a requirement absent from both tables is refused with.
///
/// Deliberately not per-requirement prose. The requirement's own `asks` and its clause are
/// already in the report beside this, and a second sentence restating them would be this
/// converter's paraphrase of a paraphrase. What this adds is the only thing the reader does not
/// already have: that the gap is *this program's*, not the document's.
const NOT_BUILT_YET: &str = "this converter does not yet meet this requirement; \
     doc/pdf-a-conversion-limits.md names the resource or the decision each remaining one waits \
     on, and no file is written rather than one wearing a claim it has not earned";

/// Decides what to do about every requirement the document failed.
///
/// The middle stage, as a pure function of the validator's report and the caller's
/// authorisations. **A requirement absent from [`REMEDIES`] and [`WRITER_EMITS`] is refused by
/// name** — never passed over, and never answered by a rewrite invented here.
/// The rewrite the table answers one requirement with, where it answers it with exactly one.
///
/// **Read off [`REMEDIES`] rather than written down a second time**, so that a `preserve` remedy
/// cannot name a rewrite the table would not have taken for the site: what `preserve` changes is
/// where the content goes, never what makes the file conform. `None` for a requirement the table
/// does not answer, and for [`Answer::CmykUnderPartTwo`], [`Answer::CmykUnderPartFour`] and
/// [`Answer::RgbUnderPartTwoOrFour`], whose rewrites the profile in hand picks between.
pub(super) fn rewrite_for(id: &str) -> Option<Rewrite> {
    let remedy = REMEDIES.iter().find(|remedy| remedy.requirement == id)?;
    match remedy.answer {
        Answer::Mechanical(rewrite) | Answer::Stated(_, rewrite, _) | Answer::Loses(_, rewrite) => {
            Some(rewrite)
        }
        Answer::CmykUnderPartTwo
        | Answer::CmykUnderPartFour
        | Answer::RgbUnderPartTwoOrFour(_)
        | Answer::AsUnderlying(_) => None,
        Answer::OrStated(inner, ..) => match inner {
            Answer::Mechanical(rewrite) | Answer::Stated(_, rewrite, _) => Some(*rewrite),
            _ => None,
        },
    }
}

/// Whether the table gives a requirement no answer at all, so that [`decide`] refuses it by name.
///
/// Asked by a preparation that is costly to make and could only matter to a document nothing
/// else refuses (`doc/adr/2520`).
pub(super) fn answers_nothing(id: &str) -> bool {
    !WRITER_EMITS.contains(&id) && !REMEDIES.iter().any(|remedy| remedy.requirement == id)
}

pub(super) fn decide(
    input: &pdf_archive::Report,
    judgement: &Judgement,
    authorised: Authorisations,
    prepared: &Prepared,
) -> Decision {
    // **A page photographed answers a fault of its content stream** where the profile authorised
    // `page-as-image` and every page the row fails on was photographed (A694 item 1, ADR 2516),
    // with the tagging of those pages where the document states a structure tree (A707).
    // The row keeps its refusal below otherwise: `as-shown` and a caller who authorised nothing
    // are refused by the fence's sentence.
    //
    // The same answer for §7.3.4.3's digit where every stream that omitted one was a content
    // stream the photographs took off their pages, so that no stream is left for the repair to
    // write into (`super::hexadecimal::Completed::leaving_out`, `doc/adr/2640`).
    let only_photographed = judgement.id == ODD_HEXADECIMAL_DIGITS
        && prepared
            .hexadecimal
            .as_ref()
            .is_ok_and(super::hexadecimal::Completed::only_on_photographed_pages);
    if authorised.grants(Loss::PageAsImage)
        && let Ok(substitutes) = prepared.substitutes.as_ref()
        && (substitutes.fenced.contains(judgement.id) || only_photographed)
    {
        // A page photographed for an operator that is no run keeps the runs written on every
        // other page, and the row costs both (`doc/adr/2638`).
        let runs_written = authorised.grants(Loss::ContentAsDrawn)
            && prepared
                .as_drawn
                .as_ref()
                .is_ok_and(|drawn| !drawn.runs.is_empty());
        let losses = substitutes.photograph_losses();
        return losing(
            if runs_written && judgement.id == super::fenced::OPERATORS {
                losses.with(Loss::ContentAsDrawn)
            } else {
                losses
            },
            Rewrite::SubstituteFontProgram,
            authorised,
        );
    }
    if WRITER_EMITS.contains(&judgement.id) {
        // **Not unconditionally.** Some of these rules reach inside a content stream — ISO
        // 19005's hexadecimal string rule is stated of the file's syntax and a content stream
        // is syntax — and a content stream crosses this verb byte for byte. So a failure a
        // page names is one the whole-file rewrite does not reach, and saying otherwise would
        // be promising a fix that never arrives.
        if in_a_content_stream(judgement) {
            // **One of the two rows has a value the standard states, and the other does not.**
            // ISO 32000-2 §7.3.4.3 says the missing final digit "shall be assumed to be 0", so
            // writing it down transcribes a decision the standard already took and changes
            // nothing a reader computes; a byte that is neither a digit nor white space has no
            // value in that clause at all, so removing one would be a guess about the producer.
            // `doc/adr/1176` is the argument and `super::hexadecimal` the rewrite.
            if judgement.id == ODD_HEXADECIMAL_DIGITS {
                return answer_of(
                    Answer::Mechanical(Rewrite::HexadecimalDigitCompleted),
                    authorised,
                    prepared,
                );
            }
            return Decision::Refused(Because::TheFence(SYNTAX_INSIDE_CONTENT));
        }
        return Decision::Mechanical(Rewrite::WholeFileRewritten);
    }
    let Some(remedy) = REMEDIES
        .iter()
        .find(|remedy| remedy.requirement == judgement.id)
    else {
        return Decision::Refused(
            REFUSED_BY_NAME
                .iter()
                .find(|(id, _)| *id == judgement.id)
                .map_or(Because::NotBuiltYet(NOT_BUILT_YET), |(_, because)| *because),
        );
    };
    if let Answer::AsUnderlying(rules) = remedy.answer {
        return decide_as_underlying(input, rules, authorised, prepared);
    }
    answer_of(remedy.answer, authorised, prepared)
}

/// A compound requirement's decision, taken from the rules it names.
///
/// The answer is the **most constraining** of theirs, in the order a conversion is constrained: a
/// refusal stops the file, then an unauthorised loss, then one the caller authorised, then a
/// statement, then a rewrite that loses nothing. So a widget failing two rules, one of which this
/// converter refuses, is refused — which is what the conversion is going to do anyway, said in
/// the row a reader is looking at.
///
/// A rule the document did not fail contributes nothing, and where **none** of them failed the
/// compound is refused: its predicate found something at a population the rules did not reach,
/// and [`NOTHING_UNDERLYING_FAILED`] is what that is.
fn decide_as_underlying(
    input: &pdf_archive::Report,
    rules: &[&str],
    authorised: Authorisations,
    prepared: &Prepared,
) -> Decision {
    let mut best: Option<Decision> = None;
    for judgement in input.failures() {
        if !rules.contains(&judgement.id) {
            continue;
        }
        let Some(remedy) = REMEDIES
            .iter()
            .find(|remedy| remedy.requirement == judgement.id)
        else {
            return Decision::Refused(
                REFUSED_BY_NAME
                    .iter()
                    .find(|(id, _)| *id == judgement.id)
                    .map_or(Because::NotBuiltYet(NOT_BUILT_YET), |(_, because)| *because),
            );
        };
        // A compound naming a compound would need an order this table does not have; the rows
        // `Answer::AsUnderlying` names are ordinary ones, and one that were not is passed over
        // rather than followed.
        if matches!(remedy.answer, Answer::AsUnderlying(_)) {
            continue;
        }
        let decision = answer_of(remedy.answer, authorised, prepared);
        if best.is_none_or(|held| constraint(decision) > constraint(held)) {
            best = Some(decision);
        }
    }
    best.unwrap_or(Decision::Refused(Because::NotBuiltYet(
        NOTHING_UNDERLYING_FAILED,
    )))
}

/// How much one decision constrains the conversion, for [`decide_as_underlying`]'s ordering.
const fn constraint(decision: Decision) -> u8 {
    match decision {
        Decision::Mechanical(_) => 0,
        Decision::Stated { .. } => 1,
        // Above a statement and below a loss: a configured remedy leaves the file conforming and
        // loses nothing, and all the same it is the one class where a person had to say yes.
        Decision::Configured { .. } => 2,
        Decision::Authorised { .. } => 3,
        Decision::Unauthorised { .. } => 4,
        Decision::Refused(_) => 5,
    }
}

/// One row's answer, once the caller's authorisations and the document's preparations are known.
/// A `Stated` row whose cost the preparation found to be a fact about the file: the statement
/// where the rewrite moves nothing, the row's loss where it does, and the preparation's own refusal
/// where it could not tell.
///
/// `rewrite` is one of the two rows [`answer_of`] sends here:
///
/// - ISO 19005-2 section 6.2.4.2's overprint mode: a state governing only `ICCBased` marks changes
///   nothing ISO 32000-2 §8.6.7 defines, and one governing a `DeviceCMYK` mark changes what a
///   press composites (`doc/adr/1978`).
/// - ISO 19005-2 section 6.2.5's transfer functions: removing `/TR /Identity` moves nothing on
///   this reader's device and is a statement, and removing any other function moves the colours
///   it adjusted (`doc/adr/1964`).
fn stated_where_free(
    rewrite: Rewrite,
    reinterprets: &'static str,
    authorised: Authorisations,
    prepared: &Prepared,
) -> Decision {
    let (free, loss) = if rewrite == Rewrite::OverprintModeRestated {
        (
            prepared
                .owed
                .overprint_modes
                .as_ref()
                .map(super::overprint::OverprintModes::costs_nothing),
            Answer::Loses(Loss::OverprintMode, Rewrite::OverprintModeZeroed),
        )
    } else {
        (
            prepared
                .owed
                .transfers
                .as_ref()
                .map(super::transfer::TransferFunctions::costs_nothing),
            Answer::Loses(Loss::TransferFunction, Rewrite::TransferFunctionRemoved),
        )
    };
    match free {
        Err(because) => Decision::Refused(*because),
        Ok(true) => Decision::Stated {
            rewrite,
            reinterprets,
        },
        Ok(false) => answer_of(loss, authorised, prepared),
    }
}

/// A rewrite that loses `losses`: authorised where every one of them was, and the question a
/// person has to answer otherwise.
fn losing(losses: Losses, rewrite: Rewrite, authorised: Authorisations) -> Decision {
    if authorised.grants_all(losses) {
        Decision::Authorised { losses, rewrite }
    } else {
        Decision::Unauthorised { losses, rewrite }
    }
}

fn answer_of(answer: Answer, authorised: Authorisations, prepared: &Prepared) -> Decision {
    match answer {
        // ISO 19005-2 section 6.1.13's page-boundary limit, whose answer the table cannot take:
        // §14.11.2.1 has already made an over-sized box its intersection with the media box, so
        // where that intersection is what the entry's own default would give, removing the entry
        // changes nothing a reader computes — and where it is not, a reader clips or trims a
        // different region and the caller has to authorise that. Which of the two it is is a
        // fact about the file, so the preparation decides it and this routes on the answer
        // (`doc/adr/1210`).
        Answer::Mechanical(Rewrite::PageBoundaryRemoved) => match &prepared.boundaries {
            Err(because) => Decision::Refused(*because),
            Ok(removals) if removals.costs_nothing() => {
                Decision::Mechanical(Rewrite::PageBoundaryRemoved)
            }
            Ok(_) => answer_of(
                Answer::Loses(Loss::PageBoundary, Rewrite::PageBoundaryRemoved),
                authorised,
                prepared,
            ),
        },
        // ISO 19005-2 section 6.6.2.3.3: a misspelled field is respelled, and a field the file
        // states nowhere costs the container and the properties it described, which the
        // preparation found per packet — so the row is the loss's where it did, as `CONDITIONAL`
        // says (`super::extension_container`).
        Answer::Mechanical(Rewrite::ExtensionSchemaPrefixes) if discards_a_container(prepared) => {
            answer_of(CONTAINER_DISCARDED, authorised, prepared)
        }
        // A rewrite is an answer only where the document can take it, and the reason it cannot
        // is the reason the requirement is refused with — never this converter's own paraphrase
        // of it. `Prepared::obstacle` is where that question is asked, once.
        Answer::Mechanical(Rewrite::RunWrittenAsDrawn) => {
            content_stream_fences::run_written_as_drawn(authorised, prepared)
        }
        Answer::Mechanical(rewrite) => prepared
            .obstacle(rewrite)
            .map_or(Decision::Mechanical(rewrite), Decision::Refused),
        Answer::CmykUnderPartTwo => cmyk_answer_under_part_two(prepared),
        Answer::CmykUnderPartFour => cmyk_answer_under_part_four(prepared, authorised),
        Answer::RgbUnderPartTwoOrFour(family) => rgb_answer(family, authorised, prepared),
        Answer::Stated(family, Rewrite::OutputIntent, reinterprets) => match &prepared.intent {
            Err(because) => Decision::Refused(*because),
            // ISO 19005 section 6.2.4.3 licenses a device colour space through a destination
            // profile **of its own family**, so a row naming one is answered by this intent only
            // where the profile in hand is that family's. A row naming none is answered by any
            // PDF/A output intent, and a row whose family is the *document's* rather than the
            // requirement's names none — the output's own verdict is what settles those.
            Ok(intent) if family.is_none_or(|wanted| wanted == intent.family) => Decision::Stated {
                rewrite: Rewrite::OutputIntent,
                reinterprets,
            },
            Ok(_) => Decision::Refused(Because::NotBuiltYet(WRONG_FAMILY)),
        },
        // ISO 19005-2 section 6.3.3 and ISO 19005-4 section 6.3.3 ask an appearance dictionary of
        // every annotation a *conforming file* holds, and section 6.3.1 forbids some subtypes
        // outright — so where every place this row failed at is an annotation the removal takes
        // off the page, what answers the row is the removal and not a construction. The decision
        // is therefore the removal's, with the removal's loss and the removal's authorisation:
        // nothing is constructed, nothing is `Stated`, and a caller who authorised nothing is
        // refused here exactly as they are at the section 6.3.1 row itself (ADR 1105).
        Answer::Stated(_, Rewrite::AppearanceDictionary, reinterprets) => {
            match &prepared.appearances {
                Ok(appearances) if appearances.at.is_empty() && appearances.removed > 0 => {
                    answer_of(
                        Answer::Loses(
                            Loss::ForbiddenAnnotation,
                            Rewrite::ForbiddenAnnotationRemoved,
                        ),
                        authorised,
                        prepared,
                    )
                }
                _ => prepared.obstacle(Rewrite::AppearanceDictionary).map_or(
                    Decision::Stated {
                        rewrite: Rewrite::AppearanceDictionary,
                        reinterprets,
                    },
                    Decision::Refused,
                ),
            }
        }
        // The two `Stated` rows whose cost is a fact about the file: `stated_where_free` says
        // which (`doc/adr/1964`, `doc/adr/1978`).
        Answer::Stated(
            _,
            rewrite @ (Rewrite::OverprintModeRestated | Rewrite::IdentityTransferFunctionRemoved),
            reinterprets,
        ) => stated_where_free(rewrite, reinterprets, authorised, prepared),
        // §12.7.3's Table 224 read as the two answers it admits (`doc/adr/1257`): the flag with
        // the appearances built is a statement, and the flag on its own is a loss. Which of the
        // two this conversion is taking is the operator's `construct`, which the preparation has
        // already read — an empty construction beside a flag that still has to go.
        Answer::Stated(_, Rewrite::NeedAppearancesCleared, reinterprets) => {
            if prepared.constructs_field_appearances {
                prepared.obstacle(Rewrite::NeedAppearancesCleared).map_or(
                    Decision::Stated {
                        rewrite: Rewrite::NeedAppearancesCleared,
                        reinterprets,
                    },
                    Decision::Refused,
                )
            } else {
                answer_of(
                    Answer::Loses(Loss::FieldAppearances, Rewrite::NeedAppearancesCleared),
                    authorised,
                    prepared,
                )
            }
        }
        // A face embedded for a font this viewer draws from one of the machine's: the same rewrite,
        // and the shapes it changes are a loss the operator authorises (`doc/adr/2162`).
        Answer::Stated(_, Rewrite::SubstituteFontProgram, reinterprets) => {
            embedded_faces(reinterprets, authorised, prepared)
        }
        // Every other `Stated` row is decided by the requirement and the standard, and then by
        // whether *this* document can take the rewrite — an appearance whose subtype clause
        // states no artwork is the standing case, and the reason is the preparation's own.
        Answer::Stated(_, rewrite, reinterprets) => prepared.obstacle(rewrite).map_or(
            Decision::Stated {
                rewrite,
                reinterprets,
            },
            Decision::Refused,
        ),
        // A loss the document cannot take is refused rather than offered: telling a user that
        // `--authorise` would allow something this file cannot have is worse than saying why.
        Answer::Loses(loss, rewrite) => match prepared.obstacle(rewrite) {
            Some(because) => Decision::Refused(because),
            None => losing(Losses::one(loss), rewrite, authorised),
        },
        // Resolved by `decide` before this function is reached, so that the routing is one place
        // rather than two. A row reaching here is one `decide_as_underlying` passed over.
        Answer::AsUnderlying(_) => Decision::Refused(Because::NotBuiltYet(NOT_BUILT_YET)),
        Answer::OrStated(inner, rewrite, reinterprets) => {
            narrower_or_stated(*inner, (rewrite, reinterprets), authorised, prepared)
        }
    }
}

/// [`Answer::OrStated`]: the narrower rewrite where the document can take it, the stated one
/// where it cannot and the stated one's proof holds.
///
/// Where neither can be proved, the refusal that stands is the narrower rewrite's own, unless that
/// one is the fence: a narrower rewrite fenced off has said only that it does not reach these
/// fonts — a Type 3 rename asked of a font with a program — and the stated rewrite is the one that
/// reached them, so its sentence is the reason (`doc/adr/2590`).
fn narrower_or_stated(
    inner: Answer,
    (rewrite, reinterprets): (Rewrite, &'static str),
    authorised: Authorisations,
    prepared: &Prepared,
) -> Decision {
    match answer_of(inner, authorised, prepared) {
        Decision::Refused(narrower) => match prepared.obstacle(rewrite) {
            None => Decision::Stated {
                rewrite,
                reinterprets,
            },
            Some(stated) if matches!(narrower, Because::TheFence(_)) => Decision::Refused(stated),
            Some(_) => Decision::Refused(narrower),
        },
        taken => taken,
    }
}

/// How many rows a table's families hold together, which is the length of the table they make.
///
/// Evaluated only while compiling, in the four tables' constants. A sum that saturated would fail
/// [`concatenated`]'s assertion and so the build; no table comes near one.
const fn rows<T>(families: &[&[T]]) -> usize {
    let mut total = 0_usize;
    let mut rest = families;
    while let [family, others @ ..] = rest {
        total = total.saturating_add(family.len());
        rest = others;
    }
    total
}

/// One table out of its families' slices: the families in the order given, each family's rows in
/// its own order.
///
/// The order is no reader's: every requirement has one row in a table, and the dispatch, the
/// census and the configuration find, test or map a row by its requirement (`doc/adr/2634`).
///
/// A `const fn` cannot build an array element by element without `unsafe`, so the first family's
/// first row fills it before every slot is written over, and the closing assertion fails the build
/// if a slot was not. Like [`rows`], it is evaluated only while compiling.
const fn concatenated<T: Copy, const N: usize>(families: &[&[T]]) -> [T; N] {
    let mut table = [families[0][0]; N];
    let mut at = 0_usize;
    let mut rest = families;
    while let [family, others @ ..] = rest {
        let mut rows = *family;
        while let [row, later @ ..] = rows {
            table[at] = *row;
            at = at.saturating_add(1);
            rows = later;
        }
        rest = others;
    }
    assert!(at == N, "a table is as long as its families' rows together");
    table
}
