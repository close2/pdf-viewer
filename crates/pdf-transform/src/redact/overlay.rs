//! Table 195's `/RO`, drawn over the emptied region after the removal, ISO 32000-2 §12.5.6.23.
//!
//! > After this redaction is applied and the affected content has been removed, the overlay
//! > appearance should be drawn such that its origin lines up with the lower-left corner of the
//! > annotation rectangle.
//!
//! **After** is the whole construction, and it is kept by the order of the code rather than by a
//! promise: the removal has planned and written the page's edited content before anything here
//! runs, and the overlay is appended to that edited stream behind a `q`\u{2026}`Q` pair that wraps it,
//! so the removal never walks the producer's form and the form never stands in for a removal
//! (`doc/questions/A834`, `A842`). Nothing is drawn under a page whose removal was refused, because
//! that page is written as the file had it.
//!
//! **The marks are the producer's.** The form `/RO` names is drawn as written, at the place the
//! clause fixes, which is `CLAUDE.md`'s clause-driven relocation of a producer-written appearance
//! stream; `/OverlayText` and `/IC` would be marks this program composes, and they stay on the far
//! side of the authoring line (`doc/scope-ratifications.md`, ADR 2868). Where `/RO` is present the
//! table says both are ignored, so they depart from nothing; where it is absent they are reported.
//!
//! **"Its origin" is the form's own** — the origin of form space, which §8.10.1's `/Matrix` maps
//! to `(e, f)` at the `Do`. The translation placed in front of the `Do` is the rectangle's corner
//! less that `(e, f)`, so form space's origin lands on the corner whatever matrix the producer
//! gave, and a form with the default identity matrix is translated by the corner alone. The
//! annotation rectangle is `/Rect` normalised to its lower-left corner (§7.9.5), whether or not
//! `/QuadPoints` chose the region.

use std::fmt::Write as _;

use pdf_syntax::Document;
use pdf_syntax::lexer::{Lexer, Token};
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId};

/// The deepest a reference chain is followed when proving the form reaches no removed object.
const MAX_DEPTH: usize = 64;

/// The most objects that proof visits before it refuses to draw rather than guess.
const MAX_VISITS: usize = 4096;

/// A redaction's `/RO` form, ready to be drawn: the object, and where form space's origin goes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Overlay {
    /// The form `XObject` the producer wrote.
    pub(super) form: ObjectId,
    /// The translation in default user space that puts form space's origin on the corner.
    pub(super) at: [f64; 2],
}

/// What one redaction states about the mark drawn over its region after the removal.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Stated {
    /// No `/RO`, `/IC` or `/OverlayText`: the region is left transparent, as Table 195's `/IC`
    /// row says of an absent fill.
    Nothing,
    /// A `/RO` form that is drawn after the removal.
    Drawn(Overlay),
    /// An overlay this program does not draw, and the reason, which the departure carries.
    NotDrawn(&'static str),
}

/// Reads Table 195's overlay entries of one `/Redact` annotation.
///
/// `removed` holds the objects the written file must not carry: the page's original content
/// streams and its redaction annotations. A form whose references reach one of them would carry
/// it into the output, so it is not drawn.
pub(super) fn stated(document: &Document, annotation: &Dictionary, removed: &[ObjectId]) -> Stated {
    // §7.3.9: an entry whose value is null is treated as absent.
    if let Some(value) = annotation
        .get("RO")
        .filter(|value| !matches!(value, Object::Null))
    {
        return match form(document, annotation, value, removed) {
            Ok(overlay) => Stated::Drawn(overlay),
            Err(reason) => Stated::NotDrawn(reason),
        };
    }
    let composed = ["OverlayText", "IC"]
        .iter()
        .any(|key| !matches!(document.get_key(annotation, key), Object::Null));
    if composed {
        Stated::NotDrawn(
            "Table 195's /OverlayText and /IC were not composed, because they are marks this \
             program would author (CLAUDE.md's authoring line, doc/questions/A842)",
        )
    } else {
        Stated::Nothing
    }
}

/// The `/RO` entry as a drawable form, or why it is not one.
fn form(
    document: &Document,
    annotation: &Dictionary,
    value: &Object,
    removed: &[ObjectId],
) -> Result<Overlay, &'static str> {
    // §7.3.8.1: a stream is always an indirect object, so a direct value names no form.
    let Object::Reference(id) = value else {
        return Err("Table 195's /RO is not a reference to a form XObject, so it was not drawn");
    };
    let object = document.get(*id);
    let Some(stream) = object.as_stream() else {
        return Err("Table 195's /RO is not a stream, so it was not drawn");
    };
    let subtype = document.get_key(&stream.dict, "Subtype");
    let is_form = subtype
        .as_name()
        .is_some_and(|name| name.as_bytes() == b"Form");
    let has_box = numbers(document, &stream.dict, "BBox").is_some_and(|bbox| bbox.len() == 4);
    if !is_form || !has_box {
        // Table 93: a form XObject's `/Subtype` is `Form` and its `/BBox` is required.
        return Err(
            "Table 195's /RO is not a form XObject with a /BBox (Table 93), so it was \
                    not drawn",
        );
    }
    if reaches(document, *id, removed) {
        return Err(
            "Table 195's /RO refers to the removed content or annotation, and drawing it \
                    would carry that content into the file, so it was not drawn",
        );
    }
    let rect = numbers(document, annotation, "Rect").filter(|rect| rect.len() == 4);
    let Some(rect) = rect else {
        return Err(
            "the redaction states no /Rect for Table 195's /RO to line up with, so it was \
                    not drawn",
        );
    };
    // §8.10.1: `/Matrix` maps form space to user space at the `Do`; Table 93's default is the
    // identity, whose `(e, f)` is the origin itself.
    let matrix = numbers(document, &stream.dict, "Matrix").filter(|matrix| matrix.len() == 6);
    let (e, f) = matrix.map_or((0.0, 0.0), |matrix| (matrix[4], matrix[5]));
    let corner = [rect[0].min(rect[2]), rect[1].min(rect[3])];
    Ok(Overlay {
        form: *id,
        at: [corner[0] - e, corner[1] - f],
    })
}

/// An array of finite numbers, each element resolved.
fn numbers(document: &Document, dict: &Dictionary, key: &str) -> Option<Vec<f64>> {
    let items = document.get_key(dict, key);
    let items = items.as_array()?;
    let values: Vec<f64> = items
        .iter()
        .filter_map(|item| document.resolve(item).as_number())
        .filter(|value| value.is_finite())
        .collect();
    (values.len() == items.len()).then_some(values)
}

/// Whether any reference reachable from `from` names one of `targets`, refusing — answering
/// `true` — where the walk is deeper or wider than it is bounded to follow.
fn reaches(document: &Document, from: ObjectId, targets: &[ObjectId]) -> bool {
    let mut seen: Vec<ObjectId> = Vec::new();
    let mut pending: Vec<(ObjectId, usize)> = vec![(from, 0)];
    while let Some((id, depth)) = pending.pop() {
        if targets.contains(&id) {
            return true;
        }
        if seen.contains(&id) {
            continue;
        }
        if depth >= MAX_DEPTH || seen.len() >= MAX_VISITS {
            return true;
        }
        seen.push(id);
        let object = document.get(id);
        let mut found = Vec::new();
        references(&object, &mut found, 0);
        pending.extend(
            found
                .into_iter()
                .map(|next| (next, depth.saturating_add(1))),
        );
    }
    false
}

/// Every reference a direct value holds, a stream's dictionary included.
fn references(value: &Object, out: &mut Vec<ObjectId>, depth: usize) {
    if depth >= MAX_DEPTH {
        return;
    }
    let next = depth.saturating_add(1);
    match value {
        Object::Reference(id) => out.push(*id),
        Object::Array(items) => {
            for item in items {
                references(item, out, next);
            }
        }
        Object::Dictionary(dict) => {
            for (_, item) in dict.iter() {
                references(item, out, next);
            }
        }
        Object::Stream(stream) => {
            for (_, item) in stream.dict.iter() {
                references(item, out, next);
            }
        }
        _ => {}
    }
}

/// A page's edited content once every overlay its redactions state has been dealt with.
pub(super) struct AfterRemoval {
    /// The content to write: the edited content, with each drawn form after it.
    pub(super) content: Vec<u8>,
    /// Each drawn form, by the `/XObject` name the page's resources gain for it.
    pub(super) named: Vec<(Vec<u8>, ObjectId)>,
    /// What was not drawn and why, for the page's departure; `None` where nothing departs.
    pub(super) departure: Option<String>,
}

/// Draws each redaction's `/RO` after the edited content, and gathers what was not drawn.
pub(super) fn after_removal<'a>(
    document: &Document,
    resources: &Dictionary,
    edited: Vec<u8>,
    stated: impl Iterator<Item = &'a Stated>,
) -> AfterRemoval {
    let mut reasons: Vec<&'static str> = Vec::new();
    let mut overlays: Vec<Overlay> = Vec::new();
    for overlay in stated {
        match overlay {
            Stated::Drawn(drawn) => overlays.push(*drawn),
            Stated::NotDrawn(reason) if !reasons.contains(reason) => reasons.push(reason),
            Stated::NotDrawn(_) | Stated::Nothing => {}
        }
    }
    let (content, named) = match drawn_after(document, resources, edited, &overlays) {
        Drawn::Overlaid(content, named) => (content, named),
        Drawn::Untouched(content, reason) => {
            reasons.extend(reason);
            (content, Vec::new())
        }
    };
    AfterRemoval {
        content,
        named,
        departure: (!reasons.is_empty()).then(|| reasons.join("; ")),
    }
}

/// A resource name for each overlay that the page's `/XObject` table does not already hold.
///
/// The removal's own form copies are named `RedactForm`*n* and its mask states `RedactMask`*n*, so
/// `OverlayRO`*n* cannot meet them, and a name holding no `/Redact` cannot be mistaken for the
/// annotation §12.5.6.23 removes; the producer's names are the only ones to step around.
fn names(
    document: &Document,
    resources: &Dictionary,
    overlays: &[Overlay],
) -> Vec<(Vec<u8>, ObjectId)> {
    let table = document.get_key(resources, "XObject");
    let table = table.as_dict();
    let mut out: Vec<(Vec<u8>, ObjectId)> = Vec::with_capacity(overlays.len());
    let mut ordinal = 1usize;
    for overlay in overlays {
        loop {
            let candidate = format!("OverlayRO{ordinal}").into_bytes();
            ordinal = ordinal.saturating_add(1);
            let taken = table
                .is_some_and(|dict| dict.get_by_name(&Name::new(candidate.as_slice())).is_some());
            if !taken {
                out.push((candidate, overlay.form));
                break;
            }
        }
    }
    out
}

/// The edited content with each overlay drawn after it, and the `/XObject` name each form is
/// drawn by — or the edited content untouched and why nothing could be drawn after it.
///
/// Each `Do` has to run in default user space under the initial graphics state, so the edited
/// content is opened with a `q` and closed with a `Q` for it and one for every `q` it leaves open;
/// a stream whose `Q` pops further than its `q`s pushed breaks §8.4.2's balance, would pop that
/// opening `q`, and leaves no state the overlay could be placed in, so it is drawn under no
/// overlay. Each overlay is drawn in a pair of its own so one form's state never reaches the next.
fn drawn_after(
    document: &Document,
    resources: &Dictionary,
    edited: Vec<u8>,
    overlays: &[Overlay],
) -> Drawn {
    if overlays.is_empty() {
        return Drawn::Untouched(edited, None);
    }
    // Each `/RO`'s origin, in §7.3.3's syntax through `pdf_syntax::write`'s formatter: the
    // `/Rect` and `/Matrix` it comes from are finite, and their difference can still leave `f64`'s
    // range, which no number can state (ADR 3686).
    let mut origins = Vec::with_capacity(overlays.len());
    for overlay in overlays {
        let written = pdf_syntax::write::real_number(overlay.at[0]).and_then(|x| {
            Ok(format!(
                "{x} {}",
                pdf_syntax::write::real_number(overlay.at[1])?
            ))
        });
        let Ok(origin) = written else {
            return Drawn::Untouched(
                edited,
                Some(
                    "Table 195's /RO was not drawn, because its /Rect less its /Matrix's \
                     translation is past what a §7.3.3 number can state",
                ),
            );
        };
        origins.push(origin);
    }
    let Some(open) = open_depth(document, &edited, resources) else {
        return Drawn::Untouched(
            edited,
            Some(
                "Table 195's /RO was not drawn, because the page's content pops more graphics \
                 states than it pushes (§8.4.2)",
            ),
        );
    };
    let named = names(document, resources, overlays);
    let mut out = Vec::with_capacity(
        edited
            .len()
            .saturating_add(64usize.saturating_mul(overlays.len())),
    );
    out.extend_from_slice(b"q\n");
    out.extend_from_slice(&edited);
    out.push(b'\n');
    for _ in 0..=open {
        out.extend_from_slice(b"Q\n");
    }
    for (origin, (name, _)) in origins.iter().zip(&named) {
        let mut line = String::new();
        let _ = write!(line, "q 1 0 0 1 {origin} cm /");
        out.extend_from_slice(line.as_bytes());
        out.extend_from_slice(name);
        out.extend_from_slice(b" Do Q\n");
    }
    Drawn::Overlaid(out, named)
}

/// What [`drawn_after`] made of a page's edited content.
enum Drawn {
    /// The content with the overlays drawn after it, and the name each form is drawn by.
    Overlaid(Vec<u8>, Vec<(Vec<u8>, ObjectId)>),
    /// The content as the removal left it, and why no overlay was drawn, where one was asked for.
    Untouched(Vec<u8>, Option<&'static str>),
}

/// How many graphics states `content` leaves saved, or `None` where a `Q` pops one it never
/// pushed (§8.4.2: "Occurrences of the q and Q operators shall be balanced within a given content
/// stream"). Inline image data is not operators, so a `BI` hands the scan to
/// [`pdf_model::inline_image::scan`] and resumes past its `EI` (§8.9.7).
fn open_depth(document: &Document, content: &[u8], resources: &Dictionary) -> Option<usize> {
    let mut lexer = Lexer::new(content);
    let mut depth = 0usize;
    while let Some(token) = lexer.next_token() {
        match token {
            Token::Keyword(word) if word == b"q" => depth = depth.saturating_add(1),
            Token::Keyword(word) if word == b"Q" => depth = depth.checked_sub(1)?,
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
    Some(depth)
}

/// The page's resources with each overlay's form named in `/XObject`, in the source document's
/// terms, so the carrying that follows copies the form as it copies any other resource.
pub(super) fn with_overlays(
    document: &Document,
    resources: Dictionary,
    named: &[(Vec<u8>, ObjectId)],
) -> Dictionary {
    if named.is_empty() {
        return resources;
    }
    let mut table = document
        .get_key(&resources, "XObject")
        .as_dict()
        .cloned()
        .unwrap_or_default();
    for (name, form) in named {
        table.insert(Name::new(name.as_slice()), Object::Reference(*form));
    }
    let mut out = resources;
    out.insert(Name::new(&b"XObject"[..]), Object::Dictionary(table));
    out
}
