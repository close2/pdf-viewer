//! `preserve` — the document's own content, kept on a page this conversion composes.
//!
//! # What this is, and the permission it stands on
//!
//! `doc/rfc/0007` section 2 sorts a refusal's answers by what happens to the document's content,
//! and `preserve` is the kind where the content neither stays where it was nor is lost: it
//! **moves** somewhere the target admits. Section 4.6.1's finding is that what the six targets
//! differ about is what may be *attached*, not what may be a page — every one of them admits as
//! many pages as a document likes — so appending is available at all six and is an operator's
//! choice rather than a fallback.
//!
//! A page carrying content the document already holds is the one content stream this program
//! writes that no clause specifies, which is why it needed its own amendment to `CLAUDE.md`'s
//! authoring exclusion. `doc/adr/1014` is that amendment, on the owner's `A58`, and its bound is
//! exact: **nothing may be put on such a page whose content did not come from the file.** So the
//! text laid out here is the document's own bytes, and — the strongest reading of the same rule —
//! the *typeface* it is set in is one the document itself carries. Nothing about the page comes
//! from outside the file except where its marks sit on it, and section 4 of `doc/adr/1025` is
//! where each of those placements is written down as a choice.
//!
//! # What is composed, and what is refused
//!
//! One remedy reaches a page today: the XMP packet of a document whose properties ISO 19005-2
//! section 6.6.2.3.1 rejects. `doc/pdf-a-conversion-limits.md` section 3.9 removes those
//! properties, which is a loss the caller has to authorise; with `remedy = "preserve"` the packet
//! **as the producer wrote it** is laid out on pages appended to the document, so what the removal
//! takes out of the metadata is still in the archive for a person to read.
//!
//! Everything this cannot do losslessly refuses by name rather than half-doing it — the round's
//! rule, and `CLAUDE.md` principle 1's: *a promise nothing will keep is worse than a refusal with
//! a sentence*. The refusals are [`NO_STRUCTURE_ENTRIES`], [`NO_FACE`],
//! [`NO_OUTPUT_INTENT`], [`LABELS_NOT_EXTENDABLE`], [`NO_PAGE_TO_MEASURE`], [`NO_MEASURE`],
//! [`NOT_TEXT`], [`TOO_MANY_PAGES`] and [`NO_SPARE_OBJECT`], and each says what it waits on.

use std::collections::{BTreeMap, BTreeSet};

use pdf_font::LoadedFont;
use pdf_font::tounicode::ToUnicode;
use pdf_model::Pages;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId, Stream};
use pdf_syntax::serialize::flate_encode;
use pdf_syntax::{Document, Lexer, Token};

use super::COMPRESSION_LEVEL;
use super::decision::Because;
use super::prepare::Spare;
use super::report::{Preserved, SetIn};

/// The type size the preserved text is set at, in text space units.
///
/// A documented choice, because ISO 32000-2 defines none: nine units of the twelve a line is
/// given, which puts roughly eighty characters of a text face across the measure an A4 or Letter
/// page leaves inside [`MARGIN`]. Smaller would fit more of a long packet on a page and read
/// worse; the content is meant to be read.
const SIZE: f32 = 9.0;

/// The distance between one line's baseline and the next, in text space units.
///
/// A third more than [`SIZE`], which is the conventional setting for a text face and leaves the
/// ascenders and descenders of adjacent lines clear of each other.
const LEADING: f32 = 12.0;

/// The blank border left on every side of an appended page, in text space units.
///
/// Seventy-two of them, which ISO 32000-2 section 8.3.2.3 makes one inch: the default user space
/// unit is 1/72 inch, so this is a margin stated in the only length the format itself defines.
/// **Except on a page too small for it to be a margin rather than the page** — see
/// [`the_margin_for`], which is the second half of the same choice.
const MARGIN: f32 = 72.0;

/// The largest share of a page's own width or height the margin may take.
///
/// An inch on a page of the sizes people print is a margin; an inch on each side of a page 200
/// units square is seven eighths of the page, and the preserved text would run to twenty times
/// the pages it needs. So the margin is the lesser of the inch and an eighth of each side, which
/// keeps the setting proportionate on a page of any size and is the same margin on every page
/// anybody actually prints.
const MOST_OF_A_SIDE: f32 = 8.0;

/// The margin an appended page of this size takes.
fn the_margin_for(width: f32, height: f32) -> f32 {
    MARGIN
        .min(width / MOST_OF_A_SIDE)
        .min(height / MOST_OF_A_SIDE)
}

/// How many pages one conversion may append before it refuses instead.
///
/// A budget rather than a limit anybody asked for: `CLAUDE.md` principle 3 is that memory safety
/// is not enough and pathological input needs an explicit bound, and an XMP packet is
/// attacker-supplied. Sixty-four pages is far past any packet a producer writes — the corpus's
/// largest is under two — and a document that needs more is refused by name rather than turned
/// into a thousand-page archive nobody wanted.
const MOST_PAGES: usize = 64;

/// The most lines one appended page is set with, whatever its own height allows.
///
/// A companion to [`MOST_PAGES`] and the same argument: the page size comes from the document, so
/// the line count does too, and a bound that is a property of this code is worth more than one
/// that is a property of the corpus. Three hundred lines at [`LEADING`] is a page ten feet tall.
const MOST_LINES_TO_A_PAGE: usize = 300;

/// How many font objects are examined before the search gives up.
///
/// The same argument as [`MOST_PAGES`]: the walk is over a document's own resources, and a file
/// naming ten thousand fonts should cost a refusal rather than ten thousand font loads.
const MOST_FONTS: usize = 64;

/// Why a document whose logical structure describes its content gets no appended page.
///
/// ISO 19005-2 section 6.7 requires a Level A file's structure tree to describe its content, and
/// `doc/adr/1014` section 5 makes the entries an appended page owes part of the same permission
/// rather than a separate one. They are not built, so a document that carries a structure tree —
/// or claims tagged conventions with `/MarkInfo` — is refused rather than given a page its own
/// tree does not describe. `doc/adr/1025` section 6 says what building them needs.
pub(super) const NO_STRUCTURE_ENTRIES: &str = "this document carries a logical structure tree, \
     and a page appended to it would be content that tree does not describe — which ISO 19005-2 \
     clause 6.7 forbids at Level A and makes a /MarkInfo claim false at any level. The structure \
     entries an appended page owes are inside the same permission as the page (doc/adr/1014 \
     section 5) and are not built, so the page is not written either. Converting a document with \
     no structure tree, or authorising the loss with --authorise metadata-property, are the two \
     answers today";

/// Why a document that embeds no usable face gets no appended page.
///
/// The page is set in a typeface **the document itself carries**, which is `doc/adr/1014`'s rule
/// read strictly: nothing on the page came from outside the file. A file embedding no font that
/// can be addressed by character, or none whose `/ToUnicode` says what its codes mean, cannot
/// carry the text, and embedding one of this program's own faces would put something on the page
/// that the document did not hold.
pub(super) const NO_FACE: &str = "the page this remedy composes is set in a typeface the \
     document itself embeds where it has one — so that nothing on the page comes from outside the \
     file (doc/adr/1014) — and in a face this program ships where it has not (doc/questions/A47). \
     Neither answered here: no font of this document states a ToUnicode CMap, can be addressed by \
     character and has a glyph for every character to be preserved, and no shipped face has a \
     WinAnsiEncoding code for every one of them either";

/// Why a conversion with no output intent gets no appended page.
///
/// ISO 32000-2 §8.4.1's Table 51 makes `DeviceGray` the initial value of the colour space
/// parameter:
///
/// > The current colour space in which colour values shall be interpreted (see 8.6, "Colour
/// > spaces"). There are two separate colour space parameters: one for stroking and one for all
/// > other painting operations. Initial value: DeviceGray .
///
/// So a page that shows text paints in a device colour space whether or not it names one, and
/// both parts of ISO 19005 admit a device colour space only where the file gives it a meaning.
/// The output intent this conversion adds is that meaning, and a conversion that will not carry
/// one cannot take the page.
pub(super) const NO_OUTPUT_INTENT: &str = "a page showing text paints in the colour space \
     ISO 32000-2 Table 51 makes the initial one, which is DeviceGray, and ISO 19005 admits a \
     device colour space only in a file that states an output intent. This conversion neither \
     finds one in the source nor adds one, so the appended page would make the output fail a \
     requirement its source met";

/// Why a page-label tree of an unexpected shape stops the remedy.
///
/// `doc/adr/0954`: a page count that changes leaves ISO 32000-2 section 12.4.2's labels no longer
/// describing the document, and leaving them stale is wrong even where it conforms. The label the
/// appended pages take is a documented choice (`doc/adr/1025` section 4); a number tree this
/// cannot extend leaves no honest way to make it, so the remedy is refused rather than the labels
/// left to say something false.
pub(super) const LABELS_NOT_EXTENDABLE: &str = "this document states page labels in a number tree \
     with /Kids rather than one flat /Nums array, and extending it is not built. Appending pages \
     without extending it would leave the producer's last label range describing pages the \
     producer never numbered, which doc/adr/0954 rules out";

/// Why a document whose first page states no usable box gets no appended page.
pub(super) const NO_PAGE_TO_MEASURE: &str = "an appended page is given the size of the page a \
     reader has been looking at, and this document's first page states no usable /MediaBox for it \
     to take (ISO 32000-2 Table 31 makes the entry required). Inventing a paper size would be \
     this converter choosing something the file does not state";

/// Why content that is not text cannot be laid out.
pub(super) const NOT_TEXT: &str = "the content to be preserved is not UTF-8 text, and the page \
     this remedy composes lays out text. ISO 16684-1 requires an XMP packet to be Unicode, so a \
     packet this cannot read is one that fails metadata/xmp-packets-well-formed as well";

/// Why a content long enough to need more than [`MOST_PAGES`] pages is refused.
pub(super) const TOO_MANY_PAGES: &str = "preserving this content would append more pages than \
     this conversion's budget allows, and a budget crossed is a refusal rather than a document \
     nobody asked for";

/// Why an object number could not be found for an appended page.
pub(super) const NO_SPARE_OBJECT: &str = "no unused object number could be found for the pages \
     this remedy would append";

/// Why a line of the content is too long to set even one character of.
pub(super) const NO_MEASURE: &str = "the page this remedy composes leaves no room for even one \
     character of the content at the size it is set in, which means the document's own page size \
     is smaller than its margins";

/// The pages a `preserve` remedy appends, and everything the page tree needs to hold them.
#[derive(Debug)]
pub(super) struct Composed {
    /// The objects this conversion adds, in the *source's* numbering.
    ///
    /// The page dictionaries and their content streams, ready for [`super::prepare::Prepared::added`]
    /// to hand to the walk exactly as it hands it an output intent's profile.
    pub(super) written: Vec<(ObjectId, Object)>,
    /// The page objects, in the order they are appended to the page tree's root node.
    pub(super) pages: Vec<ObjectId>,
    /// The `/PageLabels` number tree extended for the appended pages, where the document states one.
    ///
    /// `None` where the document labels no pages: the appended pages then carry no label because
    /// none of its pages does, which is `doc/adr/0954`'s obligation discharged by the document's
    /// own silence. The `ObjectId` is the object holding the tree where the catalog names it
    /// indirectly, and `None` where the catalog states the tree inline.
    pub(super) labels: Option<(Option<ObjectId>, Dictionary)>,
    /// What each appended page carries, for the report's sentence.
    pub(super) carried: Vec<Preserved>,
    /// The producer's own pages an appearance was relocated onto, keyed by the page object.
    ///
    /// **The construction `doc/adr/1123` builds, and the amendment `doc/adr/1120` put in scope.**
    /// Where a forbidden annotation's marks can be put back where §12.5.5 had them — on the
    /// producer's own page rather than on a page this conversion appends — this holds the two
    /// edits that page takes: the `/Contents` array with a `q`-prepend and a closing stream around
    /// the producer's own, and the `/Resources` naming the appearance the closing stream invokes.
    /// The prepend and closing streams themselves are in [`Self::written`]; a page reached by
    /// relocation appends no page at all, so it is absent from [`Self::pages`].
    pub(super) relocations: BTreeMap<ObjectId, RelocatedPage>,
}

/// The two entries a relocation writes onto the producer's own page.
///
/// `doc/adr/1123`: neither is a mark. `/Contents` gains a `q` before the producer's operators and
/// a closing stream after them — §8.4.2's balance kept across the sequence — and `/Resources`
/// gains the appearance as a form `XObject`. What draws is §12.5.5's own placement of the
/// producer's own stream, so the page shows the marks where the producer had them.
#[derive(Debug, Clone)]
pub(super) struct RelocatedPage {
    /// The `/Contents` array: the `q`-prepend stream, the producer's own streams, the closing one.
    pub(super) contents: Object,
    /// The `/Resources` dictionary the effective one becomes, with the appearance(s) named.
    pub(super) resources: Object,
}

/// One piece of content a `preserve` remedy is to keep, with what the report calls it.
pub(super) struct Keep<'a> {
    /// The requirement whose refusal this answers.
    pub(super) site: &'static str,
    /// What the content is, in the words the report uses — the document's own object, named.
    pub(super) subject: String,
    /// What is kept, and therefore what kind of page carries it.
    pub(super) content: Kept<'a>,
    /// Where marks reach an appended page because relocation was refused, the refusal's sentence.
    ///
    /// `None` for a text packet and for marks a caller did not try to relocate; `Some` for marks
    /// whose relocation onto the producer's own page `doc/adr/1123` declined by name.
    pub(super) declined: Option<&'static str>,
}

/// The two kinds of content this composes a page for.
///
/// **They are two kinds and not two sources**, and the difference is what a page has to do with
/// each. Text has no place of its own — an XMP packet is not *anywhere* on any page — so a page
/// for it is one this program lays out, and `doc/adr/1025` section 4 argues every choice in the
/// layout. Marks already have a place: §12.5.5 fixes where an annotation's normal appearance is
/// drawn, down to the matrix, so a page for them makes no layout choice at all and the only thing
/// composed is the page they sit on.
pub(super) enum Kept<'a> {
    /// Text laid out at [`SIZE`] on [`LEADING`] inside [`MARGIN`], in a face the document carries.
    Text(&'a [u8]),
    /// A form `XObject` the producer wrote, invoked where the producer's own entries put it.
    Marks(Marks),
}

/// One appearance stream, and everything §12.5.5 and §7.7.3.3 need to put it back where it was.
#[derive(Debug, Clone)]
pub(super) struct Marks {
    /// The form `XObject`, in the *source's* numbering.
    ///
    /// The producer's stream, referenced rather than rewritten: the whole point of this remedy is
    /// that not a byte of the marks changes.
    pub(super) appearance: ObjectId,
    /// §12.5.5's matrix `AA`, which maps the appearance's own space onto the annotation's `/Rect`.
    pub(super) placement: [f32; 6],
    /// The `/MediaBox` of the page the annotation was on, as `[x0, y0, x1, y1]`.
    pub(super) media_box: [f32; 4],
    /// The `/CropBox` of that page.
    pub(super) crop_box: [f32; 4],
    /// That page's `/Rotate`, after §7.7.3.3's inheritance.
    pub(super) rotate: i64,
}

/// Composes the pages one conversion appends, or the reason it will not.
///
/// **Nothing is composed for a document that asks for none**: the caller reaches here only where
/// the operator's configuration named a `preserve` remedy whose requirement this document failed,
/// which is `doc/adr/0947`'s first rule — nothing is changed that no failed requirement asked for
/// — applied to a remedy rather than to a decision.
pub(super) fn compose(
    document: &Document,
    keeps: &[Keep<'_>],
    has_output_intent: bool,
    spare: &mut Spare,
) -> Result<Composed, Because> {
    let sets_text = keeps
        .iter()
        .any(|keep| matches!(keep.content, Kept::Text(_)));
    // **The output-intent condition belongs to the text page and not to the marks one.** A page
    // that sets text paints in the colour space Table 51 makes the initial one, which is a device
    // space the file has to give a meaning. A page that invokes an appearance stream selects no
    // colour of its own, and the stream's own colours were already in this document's rendered
    // content — ISO 19005-2 section 6.2.2's NOTE 2 puts a page description and an annotation
    // appearance under the same restrictions — so moving it onto a page asks nothing of the file
    // that the file was not already asked.
    if sets_text && !has_output_intent {
        return Err(Because::NotBuiltYet(NO_OUTPUT_INTENT));
    }
    refuse_a_document_whose_structure_would_not_describe_the_page(document)?;
    let tree = Pages::new(document);
    // Every keep is laid out on pages of its own, so that the report can say which pages carry
    // which of the document's objects rather than "somewhere in the six pages appended".
    let mut composed = Composed {
        written: Vec::new(),
        pages: Vec::new(),
        labels: None,
        carried: Vec::new(),
        relocations: BTreeMap::new(),
    };
    let first = tree.len();
    let face = sets_text
        .then(|| a_face_for(document, keeps, spare))
        .transpose()?;
    if let Some(face) = face.as_ref() {
        composed.written.extend(face.written.iter().cloned());
    }
    for keep in keeps {
        let began = first.saturating_add(composed.pages.len());
        match &keep.content {
            Kept::Text(bytes) => {
                let face = face.as_ref().ok_or(Because::NotBuiltYet(NO_FACE))?;
                set_the_text(document, bytes, face, &tree, spare, &mut composed)?;
            }
            Kept::Marks(marks) => {
                place_the_marks(document, marks, spare, &mut composed)?;
            }
        }
        composed.carried.push(Preserved {
            site: keep.site,
            subject: keep.subject.clone(),
            pages: (began..first.saturating_add(composed.pages.len())).collect(),
            placement: match keep.content {
                Kept::Text(_) => PLACEMENT,
                Kept::Marks(_) => PLACEMENT_OF_MARKS,
            },
            face: match keep.content {
                Kept::Text(_) => match face.as_ref().and_then(|face| face.embedded) {
                    Some(shipped) => SetIn::AFaceThisProgramShips(shipped.to_owned()),
                    None => SetIn::TheDocumentsOwnFace,
                },
                // Nothing is set on a page of marks, so no face was chosen for one.
                Kept::Marks(_) => SetIn::NoText,
            },
            // These are the pages this conversion appended; a mark that reached one did so because
            // relocating it onto the producer's page was refused, and the caller named that reason
            // on the keep. A text packet was never on a page to relocate onto and declines nothing.
            declined: keep.declined,
        });
    }
    // **Only where pages were appended.** A relocation-only preservation appends no page, so a
    // label range at [`first`] would name a page that does not exist; the producer's numbering is
    // then untouched because nothing changed the page count.
    if !composed.pages.is_empty() {
        composed.labels = labels_extended(document, first)?;
    }
    Ok(composed)
}

/// The face every text keep is set in, chosen once over all of them.
///
/// A face the document itself embeds first, because a page set in one puts nothing on the
/// document that did not come from it; the face this program ships second, which `doc/adr/1014`
/// section 5 and `doc/questions/A47` permit and whose condition — that what was embedded is
/// reported — the report's row carries.
fn a_face_for(document: &Document, keeps: &[Keep<'_>], spare: &mut Spare) -> Result<Face, Because> {
    let mut text = String::new();
    for keep in keeps {
        let Kept::Text(bytes) = keep.content else {
            continue;
        };
        text.push_str(std::str::from_utf8(bytes).map_err(|_| Because::NotBuiltYet(NOT_TEXT))?);
        text.push('\n');
    }
    a_face_the_document_carries(document, &text)
        .or_else(|| a_face_this_program_ships(document, &text, spare))
        .ok_or(Because::NotBuiltYet(NO_FACE))
}

/// Lays one keep's text out on pages the size of the page a reader has been looking at.
fn set_the_text(
    document: &Document,
    bytes: &[u8],
    face: &Face,
    tree: &Pages<'_>,
    spare: &mut Spare,
    composed: &mut Composed,
) -> Result<(), Because> {
    let (width, height) = the_size_of_the_page_a_reader_has_been_looking_at(tree)?;
    let margin = the_margin_for(width, height);
    let measure = width - 2.0 * margin;
    let lines_to_a_page = ((height - 2.0 * margin) / LEADING).floor();
    if measure <= 0.0 || lines_to_a_page < 1.0 {
        return Err(Because::NotBuiltYet(NO_MEASURE));
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the floor of a positive finite number that a page's own height bounds, checked \
                  above to be at least one"
    )]
    // Bounded as well as floored: a page may be 14 400 units tall, and a line count taken from a
    // page's own height should not become a number this program allocates against.
    let lines_to_a_page = (lines_to_a_page as usize).min(MOST_LINES_TO_A_PAGE);
    let readable = std::str::from_utf8(bytes).map_err(|_| Because::NotBuiltYet(NOT_TEXT))?;
    let lines = wrapped(readable, face, measure)?;
    for page in lines.chunks(lines_to_a_page) {
        if composed.pages.len() >= MOST_PAGES {
            return Err(Because::NotBuiltYet(TOO_MANY_PAGES));
        }
        let content = spare
            .take(document)
            .ok_or(Because::NotBuiltYet(NO_SPARE_OBJECT))?;
        let at = spare
            .take(document)
            .ok_or(Because::NotBuiltYet(NO_SPARE_OBJECT))?;
        composed.written.push((
            content,
            stream_of(page, width, height).ok_or(Because::NotBuiltYet(NO_SPARE_OBJECT))?,
        ));
        composed
            .written
            .push((at, page_dictionary(document, face, content, width, height)?));
        composed.pages.push(at);
    }
    Ok(())
}

/// Puts one appearance stream on a page of its own, exactly where its annotation had it.
///
/// **Nothing about this page is a layout choice.** ISO 32000-2 §12.5.5 fixes the map from the
/// appearance's own coordinate system onto the annotation's rectangle, and the rectangle is in
/// the page's default user space — so a page stating that page's own boxes, invoking the same
/// stream under the same matrix, shows the producer's marks at the producer's coordinates and at
/// the producer's size. The one sentence of the algorithm that decides it:
///
/// > A matrix A shall be computed that scales and translates the transformed appearance box to
/// > align with the edges of the annotation's rectangle (specified by the Rect entry).
///
/// `/Rotate` is the source page's rather than zero, which is where this departs from
/// `doc/adr/1025` section 4's second choice and for that choice's own reason: there the page was
/// composed upright in a box of its own and an inherited turn would have laid the text on its
/// side; here the turn is part of where the producer's marks are.
fn place_the_marks(
    document: &Document,
    marks: &Marks,
    spare: &mut Spare,
    composed: &mut Composed,
) -> Result<(), Because> {
    if composed.pages.len() >= MOST_PAGES {
        return Err(Because::NotBuiltYet(TOO_MANY_PAGES));
    }
    let content = spare
        .take(document)
        .ok_or(Because::NotBuiltYet(NO_SPARE_OBJECT))?;
    let at = spare
        .take(document)
        .ok_or(Because::NotBuiltYet(NO_SPARE_OBJECT))?;
    composed.written.push((
        content,
        marks_stream(marks).ok_or(Because::NotBuiltYet(NO_SPARE_OBJECT))?,
    ));
    composed
        .written
        .push((at, marks_page(document, marks, content)?));
    composed.pages.push(at);
    Ok(())
}

/// One appearance being relocated onto its producer's page, with §12.5.5's matrix and its name.
///
/// The name is the one the page's `/Resources` `/XObject` binds the appearance to, chosen not to
/// collide with anything the effective resources already name.
pub(super) struct OnPage {
    /// The form `XObject` the producer wrote, referenced rather than rewritten.
    pub(super) appearance: ObjectId,
    /// §12.5.5's matrix `AA`, mapping the appearance's own space onto the annotation's `/Rect`.
    pub(super) placement: [f32; 6],
    /// The resource name the closing stream's `Do` invokes.
    pub(super) name: Vec<u8>,
}

/// The graphics-state depth a page's own content leaves open, or why it cannot be relocated onto.
///
/// **The count §8.4.2 is about**, taken over the sequence of the page's content streams as one.
/// `q` pushes and `Q` pops; the running balance never going below zero is the balance the clause
/// requires, and a page that breaks it is [`UNBALANCED_PRODUCER`]. Inline-image data is not
/// operators, so a `BI` hands the scan to [`pdf_model::inline_image::scan`] and resumes past the
/// `EI` — otherwise a `q` byte inside a JPEG would be counted as a save (§8.9.7, and trap in
/// `pdf-model`'s `inline_image`). The depth left open is what the closing stream must close, and a
/// depth past [`NESTING_LIMIT`] is [`PRODUCER_TOO_DEEP`].
pub(super) fn producer_open_depth(
    document: &Document,
    content: &[u8],
    resources: &Dictionary,
) -> Result<usize, Because> {
    let mut lexer = Lexer::new(content);
    let mut depth: i64 = 0;
    while let Some(token) = lexer.next_token() {
        match token {
            Token::Keyword(word) if word == b"q" => depth = depth.saturating_add(1),
            Token::Keyword(word) if word == b"Q" => {
                depth = depth.saturating_sub(1);
                if depth < 0 {
                    return Err(Because::NotBuiltYet(UNBALANCED_PRODUCER));
                }
            }
            Token::Keyword(word) if word == b"BI" => {
                let scan = pdf_model::inline_image::scan(
                    document,
                    content,
                    lexer.position(),
                    resources,
                    true,
                );
                lexer.seek(scan.resume);
            }
            _ => {}
        }
    }
    let depth = usize::try_from(depth).unwrap_or(0);
    if depth.saturating_add(1) > NESTING_LIMIT {
        return Err(Because::NotBuiltYet(PRODUCER_TOO_DEEP));
    }
    Ok(depth)
}

/// A rectangle with its corners ordered `[x_min, y_min, x_max, y_max]`.
///
/// §7.9.5: "the ordering of the four coordinate values is not significant", so a normalisation is
/// what makes two rectangles comparable — one may be stated corner-to-corner in either direction.
#[must_use]
pub(super) fn normalise_rect(rect: [f32; 4]) -> [f32; 4] {
    [
        rect[0].min(rect[2]),
        rect[1].min(rect[3]),
        rect[0].max(rect[2]),
        rect[1].max(rect[3]),
    ]
}

/// Whether two normalised rectangles share any area, edges excluded.
///
/// **Half-open**, so rectangles that only touch along an edge do not overlap: an appearance drawn
/// exactly up to another's edge does not draw over it. `doc/adr/1123`.
#[must_use]
pub(super) fn overlaps(a: [f32; 4], b: [f32; 4]) -> bool {
    a[0] < b[2] && b[0] < a[2] && a[1] < b[3] && b[1] < a[3]
}

/// The two new content streams a relocation adds to a page, and the `/Contents` array naming them.
///
/// `depth` is [`producer_open_depth`]'s answer. The prepend is one `q`; the closing stream issues
/// `depth + 1` `Q` operators — closing every state the producer left open and the prepended one,
/// which restores the page's default user space — then draws each appearance under §12.5.5's own
/// matrix. Every `q`/`Q` written is balanced against another, so §8.4.2 holds across the array.
fn relocation_streams(
    document: &Document,
    depth: usize,
    original: &Object,
    on_page: &[OnPage],
    spare: &mut Spare,
) -> Result<(Vec<(ObjectId, Object)>, Object), Because> {
    let prepend_at = spare
        .take(document)
        .ok_or(Because::NotBuiltYet(NO_SPARE_OBJECT))?;
    let closing_at = spare
        .take(document)
        .ok_or(Because::NotBuiltYet(NO_SPARE_OBJECT))?;
    let prepend = content_stream(b"q\n").ok_or(Because::NotBuiltYet(NO_SPARE_OBJECT))?;
    let mut closing = String::new();
    for _ in 0..depth.saturating_add(1) {
        closing.push_str("Q\n");
    }
    for appearance in on_page {
        closing.push_str("q\n");
        for value in appearance.placement {
            closing.push_str(&number(value));
            closing.push(' ');
        }
        closing.push_str("cm\n/");
        closing.push_str(std::str::from_utf8(&appearance.name).map_err(|_| NAME_NOT_UTF8)?);
        closing.push_str(" Do\nQ\n");
    }
    let closing =
        content_stream(closing.as_bytes()).ok_or(Because::NotBuiltYet(NO_SPARE_OBJECT))?;
    // §7.7.3.3's Table 31 admits a single stream or an array of them; a single one is flattened to
    // the array by reference so the two new streams sit around it without a producer's byte moving.
    let mut contents = vec![Object::Reference(prepend_at)];
    match original {
        Object::Reference(_) => contents.push(original.clone()),
        Object::Array(parts) => {
            for part in parts {
                if part.as_reference().is_none() {
                    return Err(Because::NotBuiltYet(CONTENTS_NOT_REFERENCED));
                }
                contents.push(part.clone());
            }
        }
        // A page stating no `/Contents` drew nothing of its own, so the prepend and closing streams
        // are the whole of it — the marks are the only content the relocated page carries.
        Object::Null => {}
        _ => return Err(Because::NotBuiltYet(CONTENTS_NOT_REFERENCED)),
    }
    contents.push(Object::Reference(closing_at));
    Ok((
        vec![(prepend_at, prepend), (closing_at, closing)],
        Object::Array(contents),
    ))
}

/// Why a chosen resource name is not valid UTF-8, which cannot happen for a name this program picks.
const NAME_NOT_UTF8: Because = Because::NotBuiltYet(
    "a resource name for a relocated appearance was not valid UTF-8, which a name this program \
     composes from ASCII cannot be",
);

/// A flate-encoded content stream holding exactly `data`.
fn content_stream(data: &[u8]) -> Option<Object> {
    let encoded = flate_encode(data, COMPRESSION_LEVEL)?;
    let mut dict = Dictionary::new();
    dict.insert(
        Name::new(&b"Filter"[..]),
        Object::Name(Name::new(&b"FlateDecode"[..])),
    );
    dict.insert(
        Name::new(&b"Length"[..]),
        Object::Integer(i64::try_from(encoded.len()).ok()?),
    );
    Some(Object::Stream(std::sync::Arc::new(Stream {
        dict,
        data: encoded.into(),
        decryption_failed: false,
    })))
}

/// The page's effective resources with each relocated appearance named under `/XObject`.
///
/// The effective (post-inheritance) resources are cloned and only `/XObject` is replaced, with a
/// fresh inline dictionary copying the effective one's entries and adding the appearances — so no
/// object a producer shares between pages is mutated, and §7.7.3.4's inheritance is written down
/// identically for this page. `doc/adr/1123`.
fn relocation_resources(document: &Document, effective: &Dictionary, on_page: &[OnPage]) -> Object {
    let mut xobjects = document
        .get_key(effective, "XObject")
        .as_dict()
        .cloned()
        .unwrap_or_default();
    for appearance in on_page {
        xobjects.insert(
            Name::new(appearance.name.as_slice()),
            Object::Reference(appearance.appearance),
        );
    }
    let mut resources = effective.clone();
    resources.insert(Name::new(&b"XObject"[..]), Object::Dictionary(xobjects));
    Object::Dictionary(resources)
}

/// A resource name binding the appearance that no name in the effective `/XObject` collides with.
///
/// [`PRESERVED_MARKS`] first, then `PreservedMarks1`, `PreservedMarks2` and so on — so two
/// appearances relocated onto one page get names of their own. `taken` grows as each is chosen.
pub(super) fn free_xobject_name(
    document: &Document,
    effective: &Dictionary,
    taken: &mut BTreeSet<Vec<u8>>,
) -> Vec<u8> {
    let existing = document.get_key(effective, "XObject");
    let existing = existing.as_dict();
    let is_free = |name: &[u8], taken: &BTreeSet<Vec<u8>>| {
        !taken.contains(name)
            && existing
                .is_none_or(|dict| dict.get(std::str::from_utf8(name).unwrap_or("")).is_none())
    };
    if is_free(PRESERVED_MARKS, taken) {
        taken.insert(PRESERVED_MARKS.to_vec());
        return PRESERVED_MARKS.to_vec();
    }
    for suffix in 1..=u32::MAX {
        let candidate = format!("PreservedMarks{suffix}").into_bytes();
        if is_free(&candidate, taken) {
            taken.insert(candidate.clone());
            return candidate;
        }
    }
    // Unreachable in practice: a page cannot name four billion XObjects. Fall back to the base name
    // rather than panicking, and a duplicate key the serializer would resolve to the last written.
    PRESERVED_MARKS.to_vec()
}

/// Builds one page's relocation from the appearances that land on it and its own resources.
///
/// The page's `/Contents` and its effective `/Resources` are read by the caller off the model's
/// [`pdf_model::Page`]; this assembles the two streams, the `/Contents` array and the new
/// `/Resources`, and hands back the [`RelocatedPage`] the rewrite applies plus the streams the walk
/// adds. `doc/adr/1123`.
pub(super) fn relocate_a_page(
    document: &Document,
    depth: usize,
    original_contents: &Object,
    effective_resources: &Dictionary,
    on_page: &[OnPage],
    spare: &mut Spare,
) -> Result<(RelocatedPage, Vec<(ObjectId, Object)>), Because> {
    let (written, contents) =
        relocation_streams(document, depth, original_contents, on_page, spare)?;
    let resources = relocation_resources(document, effective_resources, on_page);
    Ok((
        RelocatedPage {
            contents,
            resources,
        },
        written,
    ))
}

/// How the appended pages are laid out, in the one sentence the report carries.
///
/// `doc/adr/1014` section 5's fourth bullet: *a page was appended, carrying this, from there,
/// placed so*. The first two are [`Preserved::subject`] and [`Preserved::pages`]; this is the
/// third, and `doc/adr/1025` section 4 is where each choice in it is argued.
pub(super) const PLACEMENT: &str = "set verbatim at 9 units on 12, inside a margin of one inch \
     (or an eighth of the page where the page is smaller than that makes sense) on a page the size \
     of this document's first page, in the order the source states it; long lines are broken at \
     the measure, and a byte-order mark and the whitespace-only lines at the end of the content \
     are not set";

/// How a page of preserved marks is laid out, which is to say: it is not laid out at all.
///
/// The other half of `doc/adr/1014` section 5's fourth bullet, for the content that already had a
/// place. Every number on such a page is the producer's or ISO 32000-2 §12.5.5's, which is what
/// makes this remedy the smallest one that keeps the marks.
pub const PLACEMENT_OF_MARKS: &str = "invoked as the form XObject it is, under the matrix \
     ISO 32000-2 §12.5.5 computes from the annotation's own Rect, BBox and Matrix, on a page \
     stating the MediaBox, CropBox and Rotate of the page the annotation was on — so the \
     marks are the producer's bytes at the producer's coordinates, and nothing about the page is \
     this program's choice";

/// What an operator agrees to when a `preserve` remedy keeps an annotation's marks.
///
/// **A second sentence and not a variant of the first**, because this site's `preserve` does not
/// keep everything: the annotation itself has to go, ISO 19005 having no place for its subtype,
/// and the sound, movie, rendition or 3D artwork it named goes with it. What is kept is the
/// *marks*, which is the part of it a reader was looking at. An operator told only the sentence
/// below would think nothing had been lost. `doc/adr/1099`, and `doc/adr/1123` for the two places
/// the marks may end up.
pub const PRESERVED_MARKS_AS_A_PAGE: &str = "the annotation itself is gone — ISO 19005 admits no \
     annotation of its subtype — and so is the sound, movie, rendition or 3D artwork it named. \
     What is kept is what it drew: the producer's own appearance stream, put back where \
     ISO 32000-2 §12.5.5 had it — onto the producer's own page where nothing forbids it and onto \
     a page this conversion appended otherwise — at the coordinates and the size the producer \
     gave it";

/// How a page of the producer's own marks is placed, in the one sentence the report carries.
///
/// The relocation construction `doc/adr/1123` builds: no page is composed and no layout choice is
/// made. The producer's own operators keep their place; a `q` before them and a closing stream
/// after, §12.5.5's own matrix, and the appearance the closing stream invokes, are all that is
/// added — none of it a mark.
pub const PLACEMENT_ON_PAGE: &str = "invoked as the form XObject it is, under the matrix \
     ISO 32000-2 §12.5.5 computes from the annotation's own Rect, BBox and Matrix, on the \
     producer's own page — the producer's operators kept where they were, a q before them and a \
     closing stream after, so ISO 32000-2 §8.4.2's balance holds across the page's Contents and \
     the marks draw where they drew before";

/// Why a producer's content cannot take a `q` before it and a closing stream after.
///
/// **§8.4.2's balance, read as a refusal.** "Occurrences of the q and Q operators shall be
/// balanced within a given content stream (or within the sequence of streams specified in a page
/// dictionary's Contents array)." A producer that pops further than it pushes has a `Q` with no
/// matching `q`, and the prepended `q` would be what that `Q` restores — so every mark after it
/// would draw under a state this conversion introduced. The marks go onto an appended page instead.
pub(super) const UNBALANCED_PRODUCER: &str = "the producer's own content pops the graphics state \
     further than it pushes it (a Q with no matching q, which ISO 32000-2 §8.4.2 forbids), so a q \
     prepended to it would be what that Q restores and the marks after it would draw under a \
     state this conversion introduced";

/// Why a producer's content is left too deeply nested to close.
///
/// The closing stream issues one `Q` for every state the producer left open and one for the
/// prepended `q`. ISO 32000-1:2008 Annex C Table C.1 gives an implementation limit of 28 on `q`/`Q`
/// nesting (ISO 32000-2 prints no such table); a producer leaving more than that open is past the
/// limit already, and the marks go onto an appended page rather than into a stream this bound would
/// refuse.
pub(super) const PRODUCER_TOO_DEEP: &str = "the producer's own content leaves the graphics state \
     stack nested past ISO 32000-1:2008 Annex C Table C.1's implementation limit of 28, so the \
     stream that would close it back to the page's default state is not written";

/// Why a page whose `/Contents` is not referenced indirectly cannot be relocated onto.
///
/// §7.7.3.3's Table 31 requires `/Contents` to be an indirect reference or an array of them, so a
/// relocation flattens the one shape into the other and adds its two streams by reference without
/// touching a producer's byte. A `/Contents` written as a direct stream is one this construction
/// cannot wrap, so the marks go onto an appended page.
pub(super) const CONTENTS_NOT_REFERENCED: &str = "the producer's page states its own /Contents as \
     a direct stream rather than the indirect reference or array of references ISO 32000-2 \
     §7.7.3.3's Table 31 requires, so this construction cannot wrap it without rewriting a \
     producer's bytes";

/// Why a page a remaining annotation covers cannot take the marks in its content.
///
/// §12.5.5 composites an appearance "with a backdrop consisting of the page content along with any
/// previously painted annotations", so marks moved into the content go under every annotation that
/// stays. The standard states no painting order among annotations, so which of two would be on top
/// is not a fact this converter can read; where a remaining, unhidden annotation's rectangle meets
/// where the marks would land, the marks go onto an appended page rather than under it.
pub(super) const REMAINING_ANNOTATION_OVER_THE_MARKS: &str = "a remaining, unhidden annotation's \
     rectangle overlaps where these marks would land, and ISO 32000-2 states no painting order \
     among annotations — so relocating them into the page content might put them under a mark the \
     producer drew on top, which this conversion cannot rule out";

/// ISO 32000-1:2008 Annex C Table C.1's implementation limit on `q`/`Q` nesting.
///
/// ISO 32000-2 prints no such table, so the bound is the 2008 edition's and is cited as its. The
/// closing stream issues `depth + 1` `Q` operators, and a `depth` past this is
/// [`PRODUCER_TOO_DEEP`].
const NESTING_LIMIT: usize = 28;

/// What an operator agrees to when a `preserve` remedy appends a page.
///
/// **The sentence a person is owed**, and it is deliberately not an apology: nothing is lost and
/// nothing is invented, and what changes is *where* the content is and what a reader has to do to
/// get at it (`doc/rfc/0007` section 2.1). A reader of the archive meets the packet as pages at
/// the end of the document rather than as metadata a tool reads.
pub const PRESERVED_AS_A_PAGE: &str = "this content is the document's own, kept on a page this \
     conversion composed because the target would not hold it where it was";

/// The `xmpMM:History` parameters recording every appended page, where any were appended.
///
/// `doc/adr/1014` section 5: the fact that part of an archive is a page this converter composed
/// belongs in the *file*, not only in a report somebody may not have kept. The sentence is
/// [`PRESERVED_AS_A_PAGE`] verbatim, followed by what was preserved and where it now is.
pub(super) fn preserved_history(rows: &[Preserved]) -> Option<String> {
    use std::fmt::Write as _;
    if rows.is_empty() {
        return None;
    }
    let mut out = format!("{PRESERVED_AS_A_PAGE}: ");
    for (index, row) in rows.iter().enumerate() {
        if index > 0 {
            out.push_str("; ");
        }
        let _ = write!(
            out,
            "{} is on page(s) {} of this document, answering {}, {}{}",
            row.subject,
            row.pages
                .iter()
                .map(|page| page.saturating_add(1).to_string())
                .collect::<Vec<_>>()
                .join(", "),
            row.site,
            row.placement,
            match &row.face {
                SetIn::NoText => String::new(),
                SetIn::TheDocumentsOwnFace => ", in a face this document itself embeds".to_owned(),
                SetIn::AFaceThisProgramShips(face) =>
                    format!(", in {face}, a face this program embedded for it"),
            }
        );
    }
    Some(out)
}

/// The size an appended page takes, as a reader sees the document's first page.
///
/// **A documented choice** (`doc/adr/1025` section 4): the page a reader reaches by turning past
/// the last one should be the size of the page they were looking at, so the crop box of the
/// document's first page is taken, turned by that page's own `/Rotate` where it turns, and
/// reduced to a width and a height at the origin. The appended page states its own box rather
/// than inheriting one, which is why the origin can be dropped.
fn the_size_of_the_page_a_reader_has_been_looking_at(
    tree: &Pages<'_>,
) -> Result<(f32, f32), Because> {
    let first = tree
        .get(0)
        .ok_or(Because::NotBuiltYet(NO_PAGE_TO_MEASURE))?;
    if first.substituted_media_box.is_some() {
        return Err(Because::NotBuiltYet(NO_PAGE_TO_MEASURE));
    }
    let [x0, y0, x1, y1] = first.crop_box;
    let (width, height) = ((x1 - x0).abs(), (y1 - y0).abs());
    if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
        return Err(Because::NotBuiltYet(NO_PAGE_TO_MEASURE));
    }
    // §7.7.3.3's `/Rotate` is "the number of degrees by which the page shall be rotated clockwise
    // when displayed", so a quarter turn exchanges what a reader sees as width and height.
    if first.rotate % 180 == 90 {
        return Ok((height, width));
    }
    Ok((width, height))
}

/// Refuses a document whose own logical structure would not describe the page.
fn refuse_a_document_whose_structure_would_not_describe_the_page(
    document: &Document,
) -> Result<(), Because> {
    let Ok(catalog) = document.catalog() else {
        return Ok(());
    };
    let tagged = !document.get_key(&catalog, "StructTreeRoot").is_null()
        || document
            .get_key(&catalog, "MarkInfo")
            .as_dict()
            .is_some_and(|info| document.get_key(info, "Marked") == Object::Boolean(true));
    if tagged {
        return Err(Because::NotBuiltYet(NO_STRUCTURE_ENTRIES));
    }
    Ok(())
}

/// A face the document itself embeds that can set every character of the text.
///
/// The population is every font object the document's own pages name in their resources, in
/// object order so that the answer is the document's rather than the walk's. Four conditions, and
/// each is a property a preserved page needs rather than a preference:
///
/// - the font **states a `/ToUnicode` `CMap`** that says what each code it is asked for means,
///   because text nobody can extract is not preserved text;
/// - the document **embeds its program** ([`LoadedFont::is_substituted`] is false), so that the
///   glyphs on the page are the file's own rather than this machine's;
/// - it **can be addressed by character** ([`LoadedFont::code_for`]), which is the inverse of the
///   mapping a page's own text already goes through;
/// - it has **a glyph and a positive advance for every character** the content holds — a font
///   without one would draw `.notdef`, which ISO 19005-2 section 6.2.11.8 forbids outright.
fn a_face_the_document_carries(document: &Document, text: &str) -> Option<Face> {
    let wanted: BTreeSet<char> = text
        .chars()
        .filter(|character| is_set(*character))
        .collect();
    for at in font_objects(document) {
        let Some(dict) = document.get(at).as_dict().cloned() else {
            continue;
        };
        if document
            .get_key(&dict, "Subtype")
            .as_name()
            .and_then(|name| name.as_str())
            == Some("Type3")
        {
            // A Type 3 font's glyphs are content streams with resources of their own, and the
            // page would have to carry those too. Out of scope rather than impossible.
            continue;
        }
        let Some(stream) = document.get_key(&dict, "ToUnicode").as_stream().cloned() else {
            continue;
        };
        let Some(bytes) = document.decoded_stream_data(&stream) else {
            continue;
        };
        let unicode = ToUnicode::parse(&bytes);
        let Ok(font) = LoadedFont::load(document, &dict, "preserved") else {
            continue;
        };
        if font.is_substituted() || !font.addresses_characters() {
            continue;
        }
        let mut codes = BTreeMap::new();
        let complete = wanted.iter().all(|character| {
            let Some(code) = font.code_for(*character) else {
                return false;
            };
            if unicode.char_for(code.value()) != Some(*character) {
                return false;
            }
            let advance = font.advance(code);
            if !advance.is_finite() || advance <= 0.0 {
                return false;
            }
            codes.insert(*character, (code_bytes(code), advance));
            true
        });
        if complete {
            return Some(Face {
                at,
                written: Vec::new(),
                embedded: None,
                codes,
            });
        }
    }
    None
}

/// Whether a character of the content is one a face has to draw.
///
/// Three are not, and each for a reason rather than for convenience. A line feed is the break
/// between two lines and is set as one. A byte-order mark is what ISO 16684-1 puts at the head of
/// an XMP packet to say how the packet is encoded — every packet this remedy is asked about starts
/// with one — and it marks the bytes rather than saying anything in them; no text face has a glyph
/// for it, and setting it would put a `.notdef` on the page, which ISO 19005-2 section 6.2.11.8
/// forbids. Its reversal, U+FFFE, is the same fact read the other way round.
fn is_set(character: char) -> bool {
    !matches!(character, '\n' | '\u{feff}' | '\u{fffe}')
}

/// Every font object the document's pages name, in object order.
fn font_objects(document: &Document) -> Vec<ObjectId> {
    let tree = Pages::new(document);
    let mut out: BTreeSet<ObjectId> = BTreeSet::new();
    for index in 0..tree.len() {
        let Some(page) = tree.get(index) else {
            continue;
        };
        let Some(fonts) = document.get_key(&page.resources, "Font").as_dict().cloned() else {
            continue;
        };
        for (_, value) in fonts.iter() {
            if let Some(at) = value.as_reference() {
                out.insert(at);
            }
            if out.len() >= MOST_FONTS {
                return out.into_iter().collect();
            }
        }
    }
    out.into_iter().collect()
}

/// The bytes one code occupies in a string, most significant first (§9.7.6.2).
fn code_bytes(code: pdf_font::Code) -> Vec<u8> {
    let length = usize::from(code.length()).clamp(1, 4);
    code.value().to_be_bytes()[4_usize.saturating_sub(length)..].to_vec()
}

/// The face the preserved text is set in, and the bytes and advance each character takes.
///
/// **Two sources, in this order, and the order is the whole of the argument.** A face the
/// *document* embeds puts nothing on the page that did not come from the file, which is
/// `doc/adr/1014`'s rule read as strictly as it can be read; a face *this program* ships is what
/// ADR 1014 section 5 contemplates where the document has none — "for text, a face, which must be
/// embedded under every target, and which `A47`'s permission and condition already govern" — and
/// `A47`'s condition is that what was embedded is reported, which [`Preserved::face`] is.
struct Face {
    /// The font dictionary the appended page's resources name, in the source's numbering.
    at: ObjectId,
    /// The objects this conversion adds for it, where it is a face this program ships.
    written: Vec<(ObjectId, Object)>,
    /// The face embedded, for the report, where this conversion embedded one.
    embedded: Option<&'static str>,
    /// The string bytes and the advance in ems, by character.
    codes: BTreeMap<char, (Vec<u8>, f32)>,
}

impl Face {
    /// One line's bytes, where every character of it has a code.
    fn encode(&self, line: &[char]) -> Option<Vec<u8>> {
        let mut out = Vec::new();
        for character in line {
            out.extend_from_slice(&self.codes.get(character)?.0);
        }
        Some(out)
    }

    /// What one character advances the text position at [`SIZE`].
    fn advance(&self, character: char) -> Option<f32> {
        Some(self.codes.get(&character)?.1 * SIZE)
    }
}

/// The face this program ships, embedded for a document that carries none this can set text in.
///
/// **`doc/adr/1014` section 5 and `doc/questions/A47`.** The amendment names a face as one of the
/// placement choices an appended page makes, and `A47` is the standing permission to embed one of
/// the faces this program ships — with its standing condition, that what was embedded is reported.
/// Nothing about the *content* comes from outside the file; what comes from outside is the shape
/// of the letters, which is the same thing the margin and the type size are: a choice this
/// program makes and writes down.
///
/// Four objects are written: §9.6.2's font dictionary, §9.8's descriptor, the program itself under
/// §9.9's Table 124 `/FontFile2`, and a `/ToUnicode` `CMap` so the preserved text can be extracted
/// as text — which is the point of preserving it.
fn a_face_this_program_ships(document: &Document, text: &str, spare: &mut Spare) -> Option<Face> {
    // The request is derived from a dictionary naming §9.6.2.2's `/Helvetica`, which is how this
    // program's own table answers *which shipped face* — and the dictionary written into the file
    // below names the face that answer produced, never Helvetica, because the file has to say what
    // its bytes are.
    let mut asked = Dictionary::new();
    asked.insert(
        Name::new(&b"Type"[..]),
        Object::Name(Name::new(&b"Font"[..])),
    );
    asked.insert(
        Name::new(&b"Subtype"[..]),
        Object::Name(Name::new(&b"TrueType"[..])),
    );
    asked.insert(
        Name::new(&b"BaseFont"[..]),
        Object::Name(Name::new(&b"Helvetica"[..])),
    );
    asked.insert(
        Name::new(&b"Encoding"[..]),
        Object::Name(Name::new(&b"WinAnsiEncoding"[..])),
    );
    let shipped = pdf_font::standard::shipped_face(document, &asked, "preserved").ok()?;
    if shipped.format != pdf_font::substitute::Format::Sfnt {
        // Table 124 admits a `glyf`-based sfnt under a `/TrueType` dictionary and nothing else,
        // and the descriptor written below is an sfnt's. A CFF face would need a `/Type1`
        // dictionary and its own reader, which no shipped sans-serif face needs today.
        return None;
    }
    let metrics = SfntMetrics::of(shipped.program)?;
    let by_character = win_ansi_codes();
    let mut codes = BTreeMap::new();
    let mut used = BTreeMap::new();
    for character in text.chars().filter(|character| is_set(*character)) {
        if codes.contains_key(&character) {
            continue;
        }
        let code = *by_character.get(&character)?;
        let glyph = shipped.glyph(code)?;
        if glyph == 0 {
            // §9.6.5.2 sends a code with no glyph to `.notdef`, and both parts' `.notdef` clause
            // forbids a conforming file from showing it.
            return None;
        }
        let advance = pdf_font::restate::advance(shipped.program, shipped.format, glyph)?;
        if !advance.is_finite() || advance <= 0.0 {
            return None;
        }
        codes.insert(character, (vec![code], advance));
        used.insert(code, (character, advance));
    }
    let (&first, _) = used.first_key_value()?;
    let (&last, _) = used.last_key_value()?;
    let descriptor = spare.take(document)?;
    let program = spare.take(document)?;
    let unicode = spare.take(document)?;
    let at = spare.take(document)?;
    let face = Face {
        at,
        written: vec![
            (
                at,
                font_dictionary(&metrics, descriptor, unicode, first, last, &used),
            ),
            (descriptor, descriptor_dictionary(&metrics, program)),
            (program, program_stream(shipped.program)?),
            (unicode, unicode_map(&used)?),
        ],
        embedded: Some(shipped.describe()),
        codes,
    };
    Some(face)
}

/// The character each `/WinAnsiEncoding` code selects, inverted.
///
/// Annex D.2's table, read backwards: §9.6.5's route from a code to a glyph goes through the
/// encoding's glyph *name*, and the Adobe Glyph List is what turns that name into a character. A
/// code is kept only where the round trip answers, so a character here is one this encoding can
/// actually address.
fn win_ansi_codes() -> BTreeMap<char, u8> {
    let mut out = BTreeMap::new();
    for code in 0..=u8::MAX {
        let name = pdf_font::encoding::BaseEncoding::WinAnsi.glyph_name(code);
        if name.is_empty() {
            continue;
        }
        if let Some(character) = pdf_font::encoding::character_for(name) {
            out.entry(character).or_insert(code);
        }
    }
    out
}

/// The numbers §9.8.1's Table 122 asks of a descriptor, read off the program's own tables.
struct SfntMetrics {
    /// The PostScript name the `name` table states, which `/BaseFont` and `/FontName` carry.
    name: String,
    /// The design units in one em, which every other number here is scaled by.
    units: f32,
    /// `head`'s glyph bounding box, in design units.
    bbox: [i16; 4],
    /// `hhea`'s ascender and descender, in design units.
    extent: (i16, i16),
    /// `OS/2`'s capital height, in design units.
    cap_height: i16,
    /// `post`'s italic angle, in degrees.
    italic_angle: f32,
}

impl SfntMetrics {
    /// Reads them, or `None` where the program states no table this needs.
    fn of(program: &[u8]) -> Option<Self> {
        let tables = sfnt_tables(program)?;
        let head = tables.get(&b"head"[..]).copied()?;
        let hhea = tables.get(&b"hhea"[..]).copied()?;
        let os2 = tables.get(&b"OS/2"[..]).copied()?;
        let post = tables.get(&b"post"[..]).copied()?;
        let units = f32::from(be_u16(program, head, 18)?);
        if units <= 0.0 {
            return None;
        }
        // `OS/2` states `sCapHeight` from version 2 onwards and this descriptor needs it; a face
        // whose table is older is one this cannot describe, and describing it by measuring a
        // glyph would be this program's estimate rather than the face's own statement.
        if be_u16(program, os2, 0)? < 2 {
            return None;
        }
        Some(Self {
            name: postscript_name(program, tables.get(&b"name"[..]).copied()?)?,
            units,
            bbox: [
                be_i16(program, head, 36)?,
                be_i16(program, head, 38)?,
                be_i16(program, head, 40)?,
                be_i16(program, head, 42)?,
            ],
            extent: (be_i16(program, hhea, 4)?, be_i16(program, hhea, 6)?),
            cap_height: be_i16(program, os2, 88)?,
            // `post`'s italic angle is a 16.16 fixed-point number of degrees.
            italic_angle: f32::from(be_i16(program, post, 4)?),
        })
    }

    /// One design-unit measurement in the thousandths of an em a descriptor states.
    fn thousandths(&self, units: i16) -> i64 {
        thousandths(f32::from(units) * 1000.0 / self.units)
    }
}

/// One measurement rounded to the whole number a font dictionary states it as.
///
/// Every value this rounds comes from a font program's own tables scaled by its em, so it is a
/// small number by construction; the clamp is what makes that a property of the code rather than
/// of the input, for a program whose tables say something absurd.
fn thousandths(value: f32) -> i64 {
    if !value.is_finite() {
        return 0;
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "clamped to a range i64 holds exactly, one line above"
    )]
    let rounded = value.round().clamp(-1.0e6, 1.0e6) as i64;
    rounded
}

/// The sfnt table directory: each tag, and where its table starts and how long it is.
///
/// The format this reads is stated in the base standard's own §9.9 and in the sfnt specification
/// it points to: `numTables` at offset 4, then sixteen bytes a table — tag, checksum, offset,
/// length — from offset 12.
fn sfnt_tables(program: &[u8]) -> Option<BTreeMap<Vec<u8>, usize>> {
    let count = usize::from(be_u16(program, 0, 4)?);
    let mut out = BTreeMap::new();
    for index in 0..count {
        let at = 12_usize.checked_add(index.checked_mul(16)?)?;
        let tag = program.get(at..at.checked_add(4)?)?.to_vec();
        let offset = usize::try_from(be_u32(program, at, 8)?).ok()?;
        if offset >= program.len() {
            return None;
        }
        out.insert(tag, offset);
    }
    Some(out)
}

/// The PostScript name the `name` table states, which is name ID 6.
///
/// Two of the three storage formats a `name` table uses reach a Latin name: platform 3 (Windows)
/// stores it as UTF-16BE and platform 1 (Macintosh) as one byte a character, and the PostScript
/// name is ASCII in both. A name that is not ASCII is not one, and `None` leaves the face unused
/// rather than guessed at.
fn postscript_name(program: &[u8], table: usize) -> Option<String> {
    let count = usize::from(be_u16(program, table, 2)?);
    let strings = table.checked_add(usize::from(be_u16(program, table, 4)?))?;
    for index in 0..count {
        let at = table.checked_add(6)?.checked_add(index.checked_mul(12)?)?;
        if be_u16(program, at, 6)? != 6 {
            continue;
        }
        let platform = be_u16(program, at, 0)?;
        let length = usize::from(be_u16(program, at, 8)?);
        let offset = strings.checked_add(usize::from(be_u16(program, at, 10)?))?;
        let bytes = program.get(offset..offset.checked_add(length)?)?;
        let name: String = match platform {
            3 => bytes
                .chunks_exact(2)
                .map(|pair| char::from(pair.get(1).copied().unwrap_or(0)))
                .collect(),
            1 => bytes.iter().map(|byte| char::from(*byte)).collect(),
            _ => continue,
        };
        if !name.is_empty() && name.is_ascii() && name.bytes().all(|byte| byte.is_ascii_graphic()) {
            return Some(name);
        }
    }
    None
}

/// A big-endian `uint16` at a table's offset.
fn be_u16(bytes: &[u8], table: usize, at: usize) -> Option<u16> {
    let at = table.checked_add(at)?;
    let pair = bytes.get(at..at.checked_add(2)?)?;
    Some(u16::from_be_bytes([
        pair.first().copied()?,
        pair.get(1).copied()?,
    ]))
}

/// A big-endian `int16` at a table's offset.
fn be_i16(bytes: &[u8], table: usize, at: usize) -> Option<i16> {
    #[expect(
        clippy::cast_possible_wrap,
        reason = "the two readings are the same bytes; the sfnt format states which tables hold                   which, and this is the signed one"
    )]
    Some(be_u16(bytes, table, at)? as i16)
}

/// A big-endian `uint32` at a table's offset.
fn be_u32(bytes: &[u8], table: usize, at: usize) -> Option<u32> {
    let at = table.checked_add(at)?;
    let four = bytes.get(at..at.checked_add(4)?)?;
    Some(u32::from_be_bytes([
        four.first().copied()?,
        four.get(1).copied()?,
        four.get(2).copied()?,
        four.get(3).copied()?,
    ]))
}

/// §9.6.2's font dictionary for the face this conversion embeds.
fn font_dictionary(
    metrics: &SfntMetrics,
    descriptor: ObjectId,
    unicode: ObjectId,
    first: u8,
    last: u8,
    used: &BTreeMap<u8, (char, f32)>,
) -> Object {
    let mut widths = Vec::new();
    for code in first..=last {
        // §9.6.2.1 indexes `/Widths` by code from `/FirstChar`, so a code between the two that
        // this page never shows still needs an entry. Its width is zero, which is what
        // `/MissingWidth`'s own default says about a code the font does not measure — and no mark
        // depends on it, because nothing on the page uses it.
        let advance = used
            .get(&code)
            .map_or(0.0, |(_, advance)| *advance * 1000.0);
        widths.push(Object::Integer(thousandths(advance)));
    }
    let mut dict = Dictionary::new();
    dict.insert(
        Name::new(&b"Type"[..]),
        Object::Name(Name::new(&b"Font"[..])),
    );
    dict.insert(
        Name::new(&b"Subtype"[..]),
        Object::Name(Name::new(&b"TrueType"[..])),
    );
    dict.insert(
        Name::new(&b"BaseFont"[..]),
        Object::Name(Name::new(metrics.name.as_bytes())),
    );
    dict.insert(Name::new(&b"FirstChar"[..]), Object::Integer(first.into()));
    dict.insert(Name::new(&b"LastChar"[..]), Object::Integer(last.into()));
    dict.insert(Name::new(&b"Widths"[..]), Object::Array(widths));
    dict.insert(
        Name::new(&b"FontDescriptor"[..]),
        Object::Reference(descriptor),
    );
    dict.insert(
        Name::new(&b"Encoding"[..]),
        Object::Name(Name::new(&b"WinAnsiEncoding"[..])),
    );
    dict.insert(Name::new(&b"ToUnicode"[..]), Object::Reference(unicode));
    Object::Dictionary(dict)
}

/// §9.8.1's Table 122 descriptor, every number of it read off the program.
///
/// **`/StemV` is stated as zero, and that is the standard's own word rather than a gap.** Table
/// 122 defines the entry as "[t]he thickness measured horizontally, of the dominant vertical
/// stems of glyphs in the font", and then: "A value of 0 indicates an unknown stem thickness."
/// This program has not measured the face's stems, so nought is what is true of its knowledge;
/// any other number would be an invention, which is exactly why
/// `super::fonts::NO_DESCRIPTOR_TO_EMBED_INTO` refuses to write a descriptor for a *producer's*
/// font.
///
/// `/Flags` is 32, which §9.8.2's Table 123 makes bit position 6, *Nonsymbolic*: the face is set
/// with `/WinAnsiEncoding`, whose characters are the standard Latin set that bit is about.
fn descriptor_dictionary(metrics: &SfntMetrics, program: ObjectId) -> Object {
    let mut dict = Dictionary::new();
    dict.insert(
        Name::new(&b"Type"[..]),
        Object::Name(Name::new(&b"FontDescriptor"[..])),
    );
    dict.insert(
        Name::new(&b"FontName"[..]),
        Object::Name(Name::new(metrics.name.as_bytes())),
    );
    dict.insert(Name::new(&b"Flags"[..]), Object::Integer(32));
    dict.insert(
        Name::new(&b"FontBBox"[..]),
        Object::Array(
            metrics
                .bbox
                .iter()
                .map(|side| Object::Integer(metrics.thousandths(*side)))
                .collect(),
        ),
    );
    dict.insert(
        Name::new(&b"ItalicAngle"[..]),
        Object::Real(f64::from(metrics.italic_angle)),
    );
    dict.insert(
        Name::new(&b"Ascent"[..]),
        Object::Integer(metrics.thousandths(metrics.extent.0)),
    );
    dict.insert(
        Name::new(&b"Descent"[..]),
        Object::Integer(metrics.thousandths(metrics.extent.1)),
    );
    dict.insert(
        Name::new(&b"CapHeight"[..]),
        Object::Integer(metrics.thousandths(metrics.cap_height)),
    );
    dict.insert(Name::new(&b"StemV"[..]), Object::Integer(0));
    dict.insert(Name::new(&b"FontFile2"[..]), Object::Reference(program));
    Object::Dictionary(dict)
}

/// The font program stream, with the `/Length1` §9.9's Table 125 makes an sfnt's uncompressed size.
fn program_stream(program: &[u8]) -> Option<Object> {
    let data = flate_encode(program, COMPRESSION_LEVEL)?;
    let mut dict = Dictionary::new();
    dict.insert(
        Name::new(&b"Filter"[..]),
        Object::Name(Name::new(&b"FlateDecode"[..])),
    );
    dict.insert(
        Name::new(&b"Length"[..]),
        Object::Integer(i64::try_from(data.len()).ok()?),
    );
    dict.insert(
        Name::new(&b"Length1"[..]),
        Object::Integer(i64::try_from(program.len()).ok()?),
    );
    Some(Object::Stream(std::sync::Arc::new(Stream {
        dict,
        data: data.into(),
        decryption_failed: false,
    })))
}

/// The `/ToUnicode` `CMap` for the codes the appended pages show.
///
/// §9.10.3's construction, written for the codes this page uses and no others: one `bfchar` each,
/// inside the single-byte codespace a simple font's codes live in (§9.7.1: "each byte of a string
/// to be shown selects one glyph").
fn unicode_map(used: &BTreeMap<u8, (char, f32)>) -> Option<Object> {
    use std::fmt::Write as _;
    let mut text = String::from(
        "/CIDInit /ProcSet findresource begin
12 dict begin
begincmap
         /CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def
         /CMapName /Adobe-Identity-UCS def
/CMapType 2 def
         1 begincodespacerange
<00> <FF>
endcodespacerange
",
    );
    // §9.10.3 caps one `bfchar` section at 100 entries.
    for chunk in used.iter().collect::<Vec<_>>().chunks(100) {
        let _ = writeln!(text, "{} beginbfchar", chunk.len());
        for (code, (character, _)) in chunk {
            let mut encoded = [0_u16; 2];
            let units = character.encode_utf16(&mut encoded);
            let mut hex = String::new();
            for unit in units.iter() {
                let _ = write!(hex, "{unit:04X}");
            }
            let _ = writeln!(text, "<{code:02X}> <{hex}>");
        }
        text.push_str(
            "endbfchar
",
        );
    }
    text.push_str(
        "endcmap
CMapName currentdict /CMap defineresource pop
end
end
",
    );
    let data = flate_encode(text.as_bytes(), COMPRESSION_LEVEL)?;
    let mut dict = Dictionary::new();
    dict.insert(
        Name::new(&b"Filter"[..]),
        Object::Name(Name::new(&b"FlateDecode"[..])),
    );
    dict.insert(
        Name::new(&b"Length"[..]),
        Object::Integer(i64::try_from(data.len()).ok()?),
    );
    Some(Object::Stream(std::sync::Arc::new(Stream {
        dict,
        data: data.into(),
        decryption_failed: false,
    })))
}

/// The content, broken into lines that fit the measure.
///
/// **Three documented choices** (`doc/adr/1025` section 4): the source's own line breaks are
/// kept; a line longer than the measure is broken at the character that would overrun it, with
/// nothing added to mark the break; and a horizontal tab is set as a space, because it is
/// whitespace with no glyph of its own in any text face. Whitespace-only lines at the end of the
/// content are dropped — an XMP packet carries kilobytes of them as the padding ISO 16684-1
/// recommends, and setting them would cost pages that preserve nothing.
fn wrapped(text: &str, face: &Face, measure: f32) -> Result<Vec<Vec<u8>>, Because> {
    let mut out: Vec<Vec<char>> = Vec::new();
    for source in text.split('\n') {
        let source = source.trim_end();
        let mut line: Vec<char> = Vec::new();
        let mut width = 0.0;
        for character in source.chars().filter(|character| is_set(*character)) {
            let character = if character == '\t' { ' ' } else { character };
            let advance = face
                .advance(character)
                .ok_or(Because::NotBuiltYet(NO_FACE))?;
            if width + advance > measure {
                if line.is_empty() {
                    return Err(Because::NotBuiltYet(NO_MEASURE));
                }
                out.push(std::mem::take(&mut line));
                width = 0.0;
            }
            width += advance;
            line.push(character);
        }
        out.push(line);
    }
    while out.last().is_some_and(Vec::is_empty) {
        out.pop();
    }
    out.iter()
        .map(|line| face.encode(line).ok_or(Because::NotBuiltYet(NO_FACE)))
        .collect()
}

/// One page's content stream: the lines, set from the top margin down.
///
/// **No colour operator is written**, and that is deliberate rather than an omission: ISO 32000-2
/// Table 51 makes `DeviceGray` the initial colour space and section 8.6.4.2 makes 0.0 black in it,
/// so the text is black without this program stating a colour the document did not choose.
fn stream_of(lines: &[Vec<u8>], width: f32, height: f32) -> Option<Object> {
    use std::fmt::Write as _;
    let margin = the_margin_for(width, height);
    let mut content = String::new();
    content.push_str("BT\n/PreservedText ");
    content.push_str(&number(SIZE));
    content.push_str(" Tf\n");
    content.push_str(&number(LEADING));
    content.push_str(" TL\n");
    content.push_str(&number(margin));
    content.push(' ');
    // The first baseline sits one line below the top margin, so that the tallest ascender of the
    // first line clears it.
    content.push_str(&number(height - margin - SIZE));
    content.push_str(" Td\n");
    for (index, line) in lines.iter().enumerate() {
        if index > 0 {
            content.push_str("T*\n");
        }
        content.push('<');
        for byte in line {
            let _ = write!(content, "{byte:02X}");
        }
        content.push_str("> Tj\n");
    }
    content.push_str("ET\n");
    let data = flate_encode(content.as_bytes(), COMPRESSION_LEVEL)?;
    let mut dict = Dictionary::new();
    dict.insert(
        Name::new(&b"Filter"[..]),
        Object::Name(Name::new(&b"FlateDecode"[..])),
    );
    dict.insert(
        Name::new(&b"Length"[..]),
        Object::Integer(i64::try_from(data.len()).ok()?),
    );
    Some(Object::Stream(std::sync::Arc::new(Stream {
        dict,
        data: data.into(),
        decryption_failed: false,
    })))
}

/// The name the appended page's resource dictionary binds the preserved appearance to.
///
/// One name on a page this program composes, whose `/XObject` dictionary holds nothing else, so
/// there is nothing for it to collide with.
const PRESERVED_MARKS: &[u8] = b"PreservedMarks";

/// One page of marks: the producer's stream invoked under §12.5.5's own matrix.
///
/// **No colour, no clip and no state of this program's.** The three operators written are `q`,
/// `cm` and `Do` with the matching `Q` — the concatenation the algorithm computes and the
/// invocation §8.10.1 defines — and §8.10.2 makes the stream's own `/BBox` clip it, so the
/// appearance draws exactly what it drew inside the annotation.
fn marks_stream(marks: &Marks) -> Option<Object> {
    let mut content = String::from("q\n");
    for value in marks.placement {
        content.push_str(&number(value));
        content.push(' ');
    }
    content.push_str("cm\n/");
    content.push_str(std::str::from_utf8(PRESERVED_MARKS).ok()?);
    content.push_str(" Do\nQ\n");
    let data = flate_encode(content.as_bytes(), COMPRESSION_LEVEL)?;
    let mut dict = Dictionary::new();
    dict.insert(
        Name::new(&b"Filter"[..]),
        Object::Name(Name::new(&b"FlateDecode"[..])),
    );
    dict.insert(
        Name::new(&b"Length"[..]),
        Object::Integer(i64::try_from(data.len()).ok()?),
    );
    Some(Object::Stream(std::sync::Arc::new(Stream {
        dict,
        data: data.into(),
        decryption_failed: false,
    })))
}

/// One page of marks' dictionary, stating the boxes and the turn its source page stated.
///
/// `/MediaBox` and `/CropBox` are both written, as they are on a page of text and for the same
/// reason: §7.7.3.3 makes both inheritable, and a page tree root stating either would otherwise
/// decide where these marks land.
fn marks_page(document: &Document, marks: &Marks, content: ObjectId) -> Result<Object, Because> {
    let root = document
        .catalog()
        .ok()
        .and_then(|catalog| catalog.get("Pages").and_then(Object::as_reference))
        .ok_or(Because::NotBuiltYet(NO_PAGE_TO_MEASURE))?;
    let box_of = |corners: [f32; 4]| {
        Object::Array(
            corners
                .into_iter()
                .map(|value| Object::Real(f64::from(value)))
                .collect(),
        )
    };
    let mut xobjects = Dictionary::new();
    xobjects.insert(
        Name::new(PRESERVED_MARKS),
        Object::Reference(marks.appearance),
    );
    let mut resources = Dictionary::new();
    resources.insert(Name::new(&b"XObject"[..]), Object::Dictionary(xobjects));
    let mut dict = Dictionary::new();
    dict.insert(
        Name::new(&b"Type"[..]),
        Object::Name(Name::new(&b"Page"[..])),
    );
    dict.insert(Name::new(&b"Parent"[..]), Object::Reference(root));
    dict.insert(Name::new(&b"MediaBox"[..]), box_of(marks.media_box));
    dict.insert(Name::new(&b"CropBox"[..]), box_of(marks.crop_box));
    dict.insert(Name::new(&b"Rotate"[..]), Object::Integer(marks.rotate));
    dict.insert(Name::new(&b"Resources"[..]), Object::Dictionary(resources));
    dict.insert(Name::new(&b"Contents"[..]), Object::Reference(content));
    Ok(Object::Dictionary(dict))
}

/// A number as a content stream writes one, without an exponent.
fn number(value: f32) -> String {
    format!("{value:.2}")
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_owned()
}

/// One appended page's dictionary.
///
/// `/MediaBox` and `/CropBox` are both stated so that nothing the page tree's root node says about
/// either reaches this page, and `/Rotate 0` for the same reason: the page is composed upright in
/// its own box, and an inherited quarter turn would lay the text on its side.
fn page_dictionary(
    document: &Document,
    face: &Face,
    content: ObjectId,
    width: f32,
    height: f32,
) -> Result<Object, Because> {
    let root = document
        .catalog()
        .ok()
        .and_then(|catalog| catalog.get("Pages").and_then(Object::as_reference))
        .ok_or(Because::NotBuiltYet(NO_PAGE_TO_MEASURE))?;
    let box_of = || {
        Object::Array(vec![
            Object::Integer(0),
            Object::Integer(0),
            Object::Real(f64::from(width)),
            Object::Real(f64::from(height)),
        ])
    };
    let mut fonts = Dictionary::new();
    fonts.insert(Name::new(&b"PreservedText"[..]), Object::Reference(face.at));
    let mut resources = Dictionary::new();
    resources.insert(Name::new(&b"Font"[..]), Object::Dictionary(fonts));
    let mut dict = Dictionary::new();
    dict.insert(
        Name::new(&b"Type"[..]),
        Object::Name(Name::new(&b"Page"[..])),
    );
    dict.insert(Name::new(&b"Parent"[..]), Object::Reference(root));
    dict.insert(Name::new(&b"MediaBox"[..]), box_of());
    dict.insert(Name::new(&b"CropBox"[..]), box_of());
    dict.insert(Name::new(&b"Rotate"[..]), Object::Integer(0));
    dict.insert(Name::new(&b"Resources"[..]), Object::Dictionary(resources));
    dict.insert(Name::new(&b"Contents"[..]), Object::Reference(content));
    Ok(Object::Dictionary(dict))
}

/// The page-label tree with a range for the appended pages, where the document states one.
///
/// **A documented choice** (`doc/adr/1025` section 4). The appended pages are not part of the
/// producer's numbering, and ISO 32000-2 Table 159 makes every entry of a page label dictionary
/// optional — so a range whose dictionary states none gives them no label at all, which is the
/// only thing this converter can say about them that is true. Letting the producer's last range
/// run on would number them as though the producer had, and a prefix would be this program
/// writing words into a document.
fn labels_extended(
    document: &Document,
    first: usize,
) -> Result<Option<(Option<ObjectId>, Dictionary)>, Because> {
    let Ok(catalog) = document.catalog() else {
        return Ok(None);
    };
    let Some(stated) = catalog.get("PageLabels") else {
        return Ok(None);
    };
    let at = stated.as_reference();
    let Some(tree) = document.resolve(stated).as_dict().cloned() else {
        return Err(Because::NotBuiltYet(LABELS_NOT_EXTENDABLE));
    };
    if tree.get("Kids").is_some() {
        return Err(Because::NotBuiltYet(LABELS_NOT_EXTENDABLE));
    }
    let Some(nums) = document
        .get_key(&tree, "Nums")
        .as_array()
        .map(<[Object]>::to_vec)
    else {
        return Err(Because::NotBuiltYet(LABELS_NOT_EXTENDABLE));
    };
    let appended = i64::try_from(first).map_err(|_| Because::NotBuiltYet(LABELS_NOT_EXTENDABLE))?;
    // §7.9.7's number tree keeps its keys in increasing order, so a key at or past the first
    // appended page would leave the array unsorted — and a tree this converter cannot read is one
    // it does not edit.
    if nums.iter().step_by(2).any(|key| {
        document
            .resolve(key)
            .as_integer()
            .is_none_or(|at| at >= appended)
    }) {
        return Err(Because::NotBuiltYet(LABELS_NOT_EXTENDABLE));
    }
    let mut extended = nums;
    extended.push(Object::Integer(appended));
    extended.push(Object::Dictionary(Dictionary::new()));
    let mut out = tree;
    out.insert(Name::new(&b"Nums"[..]), Object::Array(extended));
    Ok(Some((at, out)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_number_is_written_without_a_trailing_point() {
        assert_eq!(number(9.0), "9");
        assert_eq!(number(841.89), "841.89");
        assert_eq!(number(12.5), "12.5");
    }

    #[test]
    fn a_code_is_written_most_significant_byte_first() {
        assert_eq!(code_bytes(pdf_font::Code::single_byte(0x41)), vec![0x41]);
    }
}
