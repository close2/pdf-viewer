//! The errata the Markdown conversion of the specifications dropped.
//!
//! # Why this exists
//!
//! `CLAUDE.md` principle 5 makes ISO 32000-2 the only source of truth, and this project reads it
//! through `doc/md/` — a Markdown conversion of the PDFs under `doc/`, which the conformance gate
//! verifies every rustdoc quotation and citation against. **The conversion ignored annotations.**
//!
//! The four-hundred-and-sixteenth session counted them, and what they are decides everything: in
//! `ISO_32000-2_sponsored_EC3.pdf` the sponsored copy's Errata Collection 3 is *recorded as review
//! markup and applied to nothing*. Each corrected passage carries a `StrikeOut` over the retired
//! words, a `Caret` whose `/Contents` is the replacement, and a `/Subj` naming the issue — 360
//! distinct `Issue #NNN` subjects in that one file. The body text underneath is the unamended
//! 2020 text, so the conversion carries retired sentences with nothing at all marking them as
//! retired, and a rustdoc blockquote can quote a sentence an erratum has struck out and pass the
//! gate. That is the exact failure principle 5 exists to prevent, and it is not hypothetical:
//! [`landings`] finds the quotations it has already happened to.
//!
//! # What this is not
//!
//! **It is not a second copy of the standard and it must never become one.** The gate checks
//! quotations against a conversion this project did not make, and that independence is the whole
//! value of the check: if we generated what we check ourselves against, a defect in our extractor
//! would become a defect in the standard, and a wrong quotation would verify against our own
//! error. So nothing here is read by `tools/conformance`, nothing here is a test, and the
//! dependency runs one way only — this crate uses [`conformance::quote`] so that "does this doc
//! comment quote retired text" is asked with the gate's own comparison, and `conformance` knows
//! nothing about this crate. ADR 0252.
//!
//! It is also not run by any gate. `doc/todo/48` is explicit that a gate must not parse a
//! 1023-page PDF on every round, and this parses fourteen of them.
//!
//! # What it emits
//!
//! One note per annotation that says something about the text, keyed to the page and to the
//! section §12.3.3's outline puts that page in — which is a clause number in every one of these
//! documents, because they are standards and their bookmarks are their clause headings.
//!
//! **Its output is derived from documents this project may not redistribute** (ADR 0187), so it
//! goes beside them under the same `.gitignore` and the same encrypted-zip discipline. The
//! *counts* are facts about the documents and may be written down; the text may not.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use pdf_render::{Point, Transform};
use pdf_syntax::{Dictionary, Document, Object};

/// What went wrong reading a document, or the tree beside it.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The file could not be read.
    #[error("{0}: {1}")]
    Unreadable(PathBuf, std::io::Error),
    /// The bytes are not a PDF this tree opens.
    #[error("{0}: {1}")]
    Unopenable(PathBuf, pdf_syntax::SyntaxError),
    /// The source tree could not be walked for quotations.
    #[error("walking {0}: {1}")]
    Sources(PathBuf, std::io::Error),
}

/// What §12.5.6.2 makes one annotation *to* the others around it.
///
/// Table 172's `/RT` and the paragraph after it:
///
/// > The group consists of a primary annotation, which shall not have an IRT entry, and one or
/// > more subordinate annotations, which shall have an IRT entry that refers to the primary
/// > annotation and an RT entry whose value is Group .
///
/// The distinction earns its place because the two are counted differently. A group's members
/// are one erratum stated in several marks — a strikeout and the caret that replaces it — and a
/// *reply* is somebody talking about an erratum rather than stating one. §12.5.6.4's states are
/// replies: "[t]he state is not specified in the annotation itself but in a separate text
/// annotation that refers to the original annotation by means of its IRT ("in reply to") entry",
/// which is what the 1 876 `Text` annotations in ISO 32000-2 mostly are.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// No `/IRT`: this annotation states the change.
    Primary,
    /// `/IRT` with `/RT /Group`: part of the same change as the annotation it names.
    Group,
    /// `/IRT` with `/RT /R` or no `/RT` at all, since Table 172's default value is `R`.
    Reply,
}

impl Role {
    /// Reads Table 172's `/IRT` and `/RT`.
    fn of(document: &Document, annotation: &Dictionary) -> Self {
        if document.get_key(annotation, "IRT").as_dict().is_none() {
            return Self::Primary;
        }
        match document
            .get_key(annotation, "RT")
            .as_name()
            .map(pdf_syntax::Name::as_bytes)
        {
            Some(b"Group") => Self::Group,
            // "Default value: R", so an `/IRT` with no `/RT` is a reply and not a group member.
            _ => Self::Reply,
        }
    }

    /// The word this role prints as.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Primary => "primary",
            Self::Group => "group",
            Self::Reply => "reply",
        }
    }
}

/// One annotation that says something about the document's text.
#[derive(Debug, Clone)]
pub struct Note {
    /// The file it came from, without its directory.
    pub document: String,
    /// The page it is on, counting from one as a reader would.
    pub page: usize,
    /// The section §12.3.3's outline puts that page in — a clause heading in these documents.
    pub section: Option<String>,
    /// Table 172's `/Subj`, "[t]ext representing a short description of the subject being
    /// addressed by the annotation" — `Issue #404` and the like in the sponsored copies.
    pub subject: Option<String>,
    /// Table 172's `/T`, the popup window's title bar, which these files use for the author.
    pub title: Option<String>,
    /// `/Subtype`, as the file states it.
    pub subtype: String,
    /// What §12.5.6.2 makes this annotation to its neighbours.
    pub role: Role,
    /// Table 166's `/Contents`.
    pub contents: Option<String>,
    /// The page's own text under `/QuadPoints`, for a markup that covers text.
    ///
    /// This is the sentence the annotation is *about*: for a `StrikeOut` it is the text the
    /// erratum retires, and it is the only part of an erratum that cannot be read out of the
    /// annotation dictionary alone.
    pub covered: Option<String>,
    /// Table 172's `/CreationDate`, which says when the erratum was raised.
    pub created: Option<String>,
    /// Table 174's states, as the replies to this annotation assert them, each with its model.
    ///
    /// **The difference between a proposed change and an agreed one**, and the instrument is
    /// worthless without it: §12.5.6.3 puts the state in a *separate* annotation — "[t]he state
    /// is not specified in the annotation itself but in a separate text annotation that refers to
    /// the original annotation by means of its IRT ("in reply to") entry" — so an erratum read on
    /// its own says nothing about whether anybody accepted it. Table 174's Review model spells
    /// the answers out: `Accepted` is "[t]he user agrees with the change", `Rejected` is "[t]he
    /// user disagrees with the change", `Cancelled` is "[t]he change has been cancelled" and
    /// `Completed` is "[t]he change has been completed".
    ///
    /// **The model is carried beside the state, because Table 174 states two of them and only one
    /// of them is a verdict.** `Marked`'s two values say whether a reviewer ticked the note off;
    /// `Review`'s five say what they decided. The four-hundred-and-seventeenth session read five
    /// errata reported as "Accepted, Unmarked" and had to open the file to find that the second
    /// word was the *Marked* model's default rather than a second opinion — so each entry is
    /// `StateModel/State`, which Table 175 makes always available: `/StateModel` is "[r]equired
    /// if State is present".
    ///
    /// Empty where nothing replied, which Table 174 makes mean `None` — "[t]he user has indicated
    /// nothing about the change (the default)" — and not `Accepted`.
    pub states: BTreeSet<String>,
}

impl Note {
    /// Whether this note retires text: a strikeout that covers words.
    ///
    /// §12.5.6.10 gives `StrikeOut` no meaning beyond the mark — "[t]ext markup annotations shall
    /// appear as highlights, underlines, strikeouts … in the text of a document" — so what makes
    /// this an erratum rather than a reader's scribble is the document it is in and the
    /// `Issue #NNN` in its `/Subj`. Both are the caller's to judge; this is only the shape.
    #[must_use]
    pub fn retires_text(&self) -> bool {
        self.subtype == "StrikeOut" && self.covered.is_some()
    }
}

/// How many words a struck passage needs before it is compared against a quotation.
///
/// **Measured rather than chosen.** Run over this tree at four, six and eight words, the check
/// answered ten landings, seven and one. Three of the ten quote a passage struck out of the very
/// clause they cite and all three are real; the other seven match a phrase struck out somewhere
/// else — six of them "; shall be an indirect reference", which Errata Collection 3 struck out
/// of seven other tables and not out of the one `appearance.rs` quotes. So the length is not
/// what separates a finding from a coincidence, and raising it to eight would have hidden two of the three real
/// ones to hide seven that [`Landing::in_clause`] already separates out.
pub const MIN_WORDS: usize = 4;

/// The words of a passage with every space taken out, which is how two extractions are compared.
///
/// **Not a coarsening for its own sake — the gate's own [`conformance::quote::normalise`] misses
/// two thirds of the hazard without it.** Both sides of every comparison in this crate are
/// *extractions of the same glyphs by different programs*: the struck passage comes out of the
/// PDF through `pdf-model`'s text layer, and `doc/md/` came out of it through a converter nobody
/// here wrote. Neither can recover a space the file does not state, because PDF positions glyphs
/// rather than words — §9.4.3's `Tj` is "shown" text and the space between two of its glyphs is
/// whatever `Tz`, `Tc` and the next `Td` make it. So one extraction writes "in the" where the
/// other writes "inthe", and a comparison that keeps whitespace calls a passage absent that is
/// there in full.
///
/// Measured in the four-hundred-and-seventeenth session over all fourteen documents:
/// **79 struck passages found with whitespace kept, 151 with it removed.** One of the 72 the
/// stricter comparison missed is §12.5.3's, which `ledger.toml` was quoting as live text.
///
/// It cannot make a false positive worth reading: a passage of [`MIN_WORDS`] words is twenty-odd
/// characters, and two different sentences do not agree on twenty characters by having their
/// spaces in different places. [`Landing::in_clause`] does the separating either way.
fn squeezed(text: &str) -> String {
    conformance::quote::normalise(text)
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect()
}

/// Reads every note out of one document.
///
/// # Errors
///
/// [`Error::Unreadable`] where the file cannot be read, [`Error::Unopenable`] where it is not a
/// PDF this tree opens.
pub fn read(path: &Path) -> Result<Vec<Note>, Error> {
    let bytes = std::fs::read(path).map_err(|error| Error::Unreadable(path.to_owned(), error))?;
    let document =
        Document::open(bytes).map_err(|error| Error::Unopenable(path.to_owned(), error))?;
    let name = path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    );
    let pages = pdf_model::Pages::new(&document);
    let outline = pdf_model::outline::Outline::read(&document, &pages);
    let mut notes = Vec::new();
    for index in 0..pages.len() {
        let Some(page) = pages.get(index) else {
            continue;
        };
        let Some(list) = document
            .get_key(&page.dict, "Annots")
            .as_array()
            .map(<[Object]>::to_vec)
        else {
            continue;
        };
        let states = states_on_this_page(&document, &list);
        // Interpreting a page costs milliseconds and most pages need it, but a page whose only
        // annotations are links needs nothing: the covered text is the one field that cannot be
        // read out of the dictionary, and a link's region is not text this is about.
        let mut interpretation = None;
        for entry in &list {
            let resolved = document.resolve(entry);
            let Some(annotation) = resolved.as_dict() else {
                continue;
            };
            let subtype = name_of(&document, annotation, "Subtype");
            // Links are the navigation the conversion also dropped and nobody lost anything by;
            // a popup is §12.5.6.14's window onto its parent, which is already here.
            if matches!(subtype.as_str(), "Link" | "Popup" | "Widget") {
                continue;
            }
            let covered = if document
                .get_key(annotation, "QuadPoints")
                .as_array()
                .is_some()
            {
                let read = interpretation
                    .get_or_insert_with(|| pdf_model::content::interpret(&document, &page));
                covered_text(&document, annotation, &page, read)
            } else {
                None
            };
            notes.push(Note {
                document: name.clone(),
                page: index.saturating_add(1),
                section: outline
                    .section_at(&document, &pages, index)
                    .map(str::to_owned),
                subject: text_of(&document, annotation, "Subj"),
                title: text_of(&document, annotation, "T"),
                subtype,
                role: Role::of(&document, annotation),
                contents: text_of(&document, annotation, "Contents"),
                covered,
                created: text_of(&document, annotation, "CreationDate"),
                states: entry
                    .as_reference()
                    .and_then(|id| states.get(&id).cloned())
                    .unwrap_or_default(),
            });
        }
    }
    Ok(notes)
}

/// A rustdoc quotation that quotes text an erratum struck out.
#[derive(Debug, Clone)]
pub struct Landing {
    /// The source file the quotation is in.
    pub file: PathBuf,
    /// Its line.
    pub line: usize,
    /// The clause it is attributed to, where the doc comment states one.
    pub clause: Option<String>,
    /// The note whose struck text it contains.
    pub note: Note,
}

impl Landing {
    /// Whether the quotation cites the clause the passage was struck out of.
    ///
    /// **What tells a finding from a coincidence**, and the reason [`MIN_WORDS`] can be as low as
    /// it is: the standard repeats short phrases, so a struck phrase can occur in a quotation of
    /// a clause three hundred pages from the one it was struck in, and that quotation is quoting
    /// its own clause correctly. Equal numbers, or one an ancestor of the other, since a doc
    /// comment cites the subclause it implements and the outline names the section a page opens.
    ///
    /// `false` where the doc comment attributes the quotation to no clause, which is a quotation
    /// the conformance gate cannot check either.
    #[must_use]
    pub fn in_clause(&self) -> bool {
        let (Some(cited), Some(section)) = (self.clause.as_deref(), self.section_clause()) else {
            return false;
        };
        cited == section
            || cited.starts_with(&format!("{section}."))
            || section.starts_with(&format!("{cited}."))
    }

    /// The clause number the outline's section title opens with, if it opens with one.
    fn section_clause(&self) -> Option<&str> {
        let title = self.note.section.as_deref()?.trim_start();
        let token = title.split_whitespace().next()?;
        let usable = token
            .split('.')
            .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()))
            || token.split_once('.').is_some_and(|(head, tail)| {
                head.len() == 1
                    && head.chars().all(|c| c.is_ascii_uppercase())
                    && tail
                        .split('.')
                        .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()))
            });
        usable.then_some(token)
    }
}

/// Every rustdoc quotation under `roots` that contains text one of `notes` struck out.
///
/// The comparison is [`conformance::quote::normalise`]'s, which is the one the gate uses, so a
/// quotation that passes the gate and lands here is quoting retired text by the gate's own
/// standard of what quoting is.
///
/// # Errors
///
/// [`Error::Sources`] where a root cannot be walked, [`Error::Unreadable`] where a source file
/// under it cannot be read.
pub fn landings(notes: &[Note], roots: &[PathBuf]) -> Result<Vec<Landing>, Error> {
    let struck: Vec<(&Note, String)> = notes
        .iter()
        .filter(|note| note.retires_text())
        .filter_map(|note| {
            let covered = note.covered.as_deref()?;
            (covered.split_whitespace().count() >= MIN_WORDS).then(|| (note, squeezed(covered)))
        })
        .collect();
    let sources = conformance::citation::rust_sources(roots)
        .map_err(|error| Error::Sources(roots.first().cloned().unwrap_or_default(), error))?;
    let mut found = Vec::new();
    for file in sources {
        let source = std::fs::read_to_string(&file)
            .map_err(|error| Error::Unreadable(file.clone(), error))?;
        for quotation in conformance::citation::scan(&source).quotations {
            let text = squeezed(&quotation.text);
            for (note, passage) in &struck {
                if text.contains(passage.as_str()) {
                    found.push(Landing {
                        file: file.clone(),
                        line: quotation.line,
                        clause: quotation.clause.as_ref().map(ToString::to_string),
                        note: (*note).clone(),
                    });
                }
            }
        }
    }
    Ok(found)
}

/// Every struck passage that the Markdown conversion still carries, unmarked.
///
/// The measurement behind the argument: a passage in this list is one an erratum retired and
/// `doc/md/` presents as the standard's current text. Answers the file it was found in.
///
/// # Errors
///
/// [`Error::Unreadable`] where a `.md` file under `directory` cannot be read.
pub fn still_in_conversion(notes: &[Note], directory: &Path) -> Result<Vec<Note>, Error> {
    let mut conversions = Vec::new();
    let entries = std::fs::read_dir(directory)
        .map_err(|error| Error::Unreadable(directory.to_owned(), error))?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|extension| extension == "md") {
            let text = std::fs::read_to_string(&path)
                .map_err(|error| Error::Unreadable(path.clone(), error))?;
            conversions.push(squeezed(&text));
        }
    }
    Ok(notes
        .iter()
        .filter(|note| note.retires_text())
        .filter(|note| {
            let Some(covered) = note.covered.as_deref() else {
                return false;
            };
            covered.split_whitespace().count() >= MIN_WORDS && {
                let passage = squeezed(covered);
                conversions
                    .iter()
                    .any(|conversion| conversion.contains(passage.as_str()))
            }
        })
        .cloned()
        .collect())
}

/// The notes, as a Markdown document keyed to page and section.
///
/// Replies are left out: §12.5.6.4's state annotations say a reviewer finished with an erratum
/// and say nothing about the standard's text, and there are more of them than there are errata.
#[must_use]
pub fn markdown(notes: &[Note]) -> String {
    let documents: BTreeSet<&str> = notes.iter().map(|note| note.document.as_str()).collect();
    // Lines joined at the end rather than a `String` pushed into: `format!` into an accumulator
    // is the allocation `clippy::format_push_string` objects to, and the alternative it suggests
    // returns a `Result` that writing to a `String` cannot produce. A `Vec` of lines has neither
    // problem and reads as what it is.
    let mut lines: Vec<String> = Vec::new();
    for document in documents {
        lines.push(format!("# {document}\n"));
        let mut section = None;
        for note in notes
            .iter()
            .filter(|note| note.document == document && note.role != Role::Reply)
        {
            if section.as_deref() != note.section.as_deref() {
                section.clone_from(&note.section);
                lines.push(format!(
                    "\n## {}\n",
                    note.section.as_deref().unwrap_or("(no section)")
                ));
            }
            let mut head = format!(
                "- **p. {} {} ({})**",
                note.page,
                note.subtype,
                note.role.as_str()
            );
            if let Some(subject) = &note.subject {
                head.push_str(" — ");
                head.push_str(subject);
            }
            if let Some(created) = &note.created {
                head.push_str(" — ");
                head.push_str(created);
            }
            if !note.states.is_empty() {
                head.push_str(" — state ");
                head.push_str(
                    &note
                        .states
                        .iter()
                        .map(String::as_str)
                        .collect::<Vec<&str>>()
                        .join(", "),
                );
            }
            lines.push(head);
            if let Some(covered) = &note.covered {
                lines.push(format!("  - over: {}", one_line(covered)));
            }
            if let Some(contents) = &note.contents {
                lines.push(format!("  - says: {}", one_line(contents)));
            }
        }
        lines.push(String::new());
    }
    lines.push(String::new());
    lines.join("\n")
}

/// Table 174's states one page's replies assert, keyed by the annotation each one names.
///
/// §12.5.6.4 requires both to be on the same page — Table 172's `/IRT` says "[b]oth annotations
/// shall be on the same page of the document" — so one page's `/Annots` is the whole population
/// and this needs no index over the file.
fn states_on_this_page(
    document: &Document,
    list: &[Object],
) -> std::collections::BTreeMap<pdf_syntax::ObjectId, BTreeSet<String>> {
    let mut out: std::collections::BTreeMap<pdf_syntax::ObjectId, BTreeSet<String>> =
        std::collections::BTreeMap::new();
    for entry in list {
        let resolved = document.resolve(entry);
        let Some(annotation) = resolved.as_dict() else {
            continue;
        };
        let Some(target) = annotation.get("IRT").and_then(Object::as_reference) else {
            continue;
        };
        // Table 175 makes `/StateModel` "[r]equired if State is present", and Table 174 states
        // two models whose values a bare `/State` cannot be told apart by.
        if let Some(state) = text_of(document, annotation, "State") {
            let model = text_of(document, annotation, "StateModel");
            out.entry(target)
                .or_default()
                .insert(model.map_or_else(|| state.clone(), |model| format!("{model}/{state}")));
        }
    }
    out
}

/// The page's own text under an annotation's `/QuadPoints`.
///
/// §12.5.6.10 states the entry "in default user space", and `Interpretation::text_layer` is in
/// the display list's own coordinates, so the quadrilaterals go through
/// `content::page_transform` — the map that exists for exactly this direction.
fn covered_text(
    document: &Document,
    annotation: &Dictionary,
    page: &pdf_model::Page,
    interpretation: &pdf_model::Interpretation,
) -> Option<String> {
    let quads = document.get_key(annotation, "QuadPoints");
    let numbers: Vec<f32> = quads
        .as_array()?
        .iter()
        .filter_map(|item| document.resolve(item).as_number())
        .map(|value| {
            // `as_number` answers an `f64` and the geometry is `f32`; a coordinate outside
            // `f32`'s range is not a page coordinate, and `as` saturates rather than wrapping.
            #[expect(
                clippy::cast_possible_truncation,
                reason = "the display list's own precision, and a page coordinate fits it"
            )]
            let narrowed = value as f32;
            narrowed
        })
        .filter(|value| value.is_finite())
        .collect();
    // "8×n numbers": anything else has stated no quadrilateral.
    if numbers.is_empty() || !numbers.len().is_multiple_of(8) {
        return None;
    }
    let transform = pdf_model::content::page_transform(page);
    let mut spans: Vec<std::ops::Range<usize>> = Vec::new();
    for placed in &interpretation.text_layer {
        if placed.span.is_empty() {
            continue;
        }
        let Some(centre) = centre(&placed.quad) else {
            continue;
        };
        if numbers
            .chunks_exact(8)
            .any(|quad| contains(quad, transform, centre))
        {
            spans.push(placed.span.clone());
        }
    }
    spans.sort_by_key(|span| span.start);
    spans.dedup();
    let mut out = String::new();
    for span in spans {
        if let Some(text) = interpretation.text.get(span) {
            out.push_str(text);
        }
    }
    (!out.trim().is_empty()).then_some(out)
}

/// The centre of a glyph's quadrilateral, in the display list's coordinates.
fn centre(quad: &[f32; 8]) -> Option<Point> {
    let mut x = 0.0f32;
    let mut y = 0.0f32;
    for pair in quad.chunks_exact(2) {
        x += pair.first()?;
        y += pair.get(1)?;
    }
    Some(Point {
        x: x / 4.0,
        y: y / 4.0,
    })
}

/// Whether a point of the display list lies in one `/QuadPoints` quadrilateral.
///
/// The quadrilateral's corners are mapped rather than the point unmapped, because
/// `content::page_transform` is the direction this crate has and inverting it would answer `None`
/// for a degenerate page rather than for a degenerate quadrilateral.
///
/// A bounding box rather than the crossing-number rule the clause's counterclockwise order would
/// want: §12.5.6.10's own NOTE says producers have written these corners "in a different order",
/// and every quadrilateral in these documents is axis-aligned, so the box is the quadrilateral.
fn contains(quad: &[f32], transform: Transform, point: Point) -> bool {
    let mut low = Point {
        x: f32::MAX,
        y: f32::MAX,
    };
    let mut high = Point {
        x: f32::MIN,
        y: f32::MIN,
    };
    for pair in quad.chunks_exact(2) {
        let (Some(x), Some(y)) = (pair.first(), pair.get(1)) else {
            return false;
        };
        let mapped = transform.apply(Point { x: *x, y: *y });
        low.x = low.x.min(mapped.x);
        low.y = low.y.min(mapped.y);
        high.x = high.x.max(mapped.x);
        high.y = high.y.max(mapped.y);
    }
    point.x >= low.x && point.x <= high.x && point.y >= low.y && point.y <= high.y
}

/// One `/Subtype`-like name entry, as a string.
fn name_of(document: &Document, dict: &Dictionary, key: &str) -> String {
    document
        .get_key(dict, key)
        .as_name()
        .map_or_else(String::new, |name| {
            String::from_utf8_lossy(name.as_bytes()).into_owned()
        })
}

/// One text string entry, decoded, with an empty value treated as absent.
fn text_of(document: &Document, dict: &Dictionary, key: &str) -> Option<String> {
    let value = document.get_key(dict, key);
    let bytes: Vec<u8> = match &value {
        Object::String(bytes) => bytes.to_vec(),
        Object::Stream(stream) => document.decoded_stream_data(stream)?.to_vec(),
        _ => return None,
    };
    let decoded = pdf_syntax::text_string::text_string(&bytes);
    (!decoded.trim().is_empty()).then_some(decoded)
}

/// A value on one line, so that a list item stays a list item.
fn one_line(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reply_is_not_a_group_member() {
        // Table 172: "Default value: R", so an `/IRT` with no `/RT` is a reply — which is what
        // §12.5.6.4's state annotations are, and there are more of them than there are errata.
        let document = document_with(
            "1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n\
             2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj\n\
             3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] >> endobj\n\
             4 0 obj << /Type /Annot /Subtype /Text >> endobj\n\
             5 0 obj << /Type /Annot /Subtype /Text /IRT 4 0 R >> endobj\n\
             6 0 obj << /Type /Annot /Subtype /StrikeOut /IRT 4 0 R /RT /Group >> endobj\n",
        );
        let roles: Vec<Role> = (4..=6)
            .filter_map(|number| {
                let object = document.get(pdf_syntax::ObjectId::new(number, 0));
                Some(Role::of(&document, object.as_dict()?))
            })
            .collect();
        assert_eq!(roles, vec![Role::Primary, Role::Reply, Role::Group]);
    }

    #[test]
    fn a_strikeout_over_nothing_retires_nothing() {
        // The shape a caller filters on: a strikeout whose quadrilaterals covered no glyph has
        // struck out a rule or a table border, and reporting it as retired text would be a
        // sentence this project invented.
        let note = Note {
            document: "x.pdf".to_owned(),
            page: 1,
            section: None,
            subject: None,
            title: None,
            subtype: "StrikeOut".to_owned(),
            role: Role::Primary,
            contents: None,
            covered: None,
            created: None,
            states: BTreeSet::new(),
        };
        assert!(!note.retires_text());
        assert!(
            Note {
                covered: Some("some words".to_owned()),
                ..note
            }
            .retires_text()
        );
    }

    #[test]
    fn two_extractions_disagree_about_where_the_spaces_are() {
        // §12.5.3's erratum, as the two extractions write it: the strikeout's text layer joins
        // "in the", the Markdown conversion does not, and the passage is the same passage. With
        // the spaces kept this comparison answers `false` and the hazard goes unreported — which
        // is what happened to `ledger.toml`'s §12.5.3 note for one session.
        let struck = "without regard to any other keys and values inthe annotation dictionary";
        let conversion = "shall render the appearance dictionary without regard to any other \
                          keys and values in the annotation dictionary and shall ignore";
        assert!(
            !conformance::quote::normalise(conversion)
                .contains(&conformance::quote::normalise(struck))
        );
        assert!(squeezed(conversion).contains(&squeezed(struck)));
    }

    /// A document from a body, with the trailer and cross-reference table a reader needs.
    fn document_with(body: &str) -> Document {
        let head = format!("%PDF-2.0\n{body}");
        let start = head.len();
        let bytes = format!("{head}trailer << /Size 7 /Root 1 0 R >>\nstartxref\n{start}\n%%EOF\n");
        // The reader recovers objects by scanning where there is no usable table, which is what
        // makes a fixture this short possible at all.
        Document::open(bytes.into_bytes()).unwrap_or_else(|error| unreachable!("{error}"))
    }
}
