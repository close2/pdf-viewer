//! The artwork ISO 32000-2 §12.5.6.4 requires a processor to have and does not state.
//!
//! Table 175's `/Name` "is the name of an icon that shall be used in displaying the
//! annotation", and the clause places one obligation on the reader in as many words:
//!
//! > Interactive PDF processors shall provide predefined icon appearances for at least the
//! > following standard names: Comment, Key, Note, Help, NewParagraph, Paragraph, Insert
//!
//! It is a *shall*, and it is a shall about a picture the standard never draws. That pairing is
//! why this module exists and why it holds no PDF at all: the seven shapes below are this
//! processor's invention, chosen once and written down as a choice, exactly as `CLAUDE.md`
//! principle 5 requires of a place where the specification genuinely defines nothing. Nothing
//! here is derived from anything, and no reference renderer was consulted for any of it —
//! agreeing with one would be a coincidence and disagreeing with one is not a defect.
//!
//! # What *is* derived, and lives in [`crate::appearance`]
//!
//! Three things about the icon are stated and are read there rather than invented here:
//!
//! - that a closed text annotation "shall appear as an icon" at all (§12.5.6.4);
//! - which icon, from Table 175's `/Name`, whose default is `Note`;
//! - the colour behind it, from Table 166's `/C` — "The background of the annotation's icon
//!   when closed" — including the case of no colour at all, which that table calls
//!   "transparent".
//!
//! The symbol's *own* colour is stated nowhere, so it is black. That is the smallest invention
//! available and it has a cost worth naming rather than hiding: a document whose `/C` is dark
//! gets a dark symbol on a dark field. Choosing black-or-white by the background's luminance
//! would read better and would be a *second* invention layered on the first, with no clause
//! behind either; this tree takes the one it cannot avoid and stops.
//!
//! # The unit square
//!
//! Every coordinate below lies in [0, 1] on both axes, with y running up as PDF's does.
//! [`crate::appearance`] maps that square onto the largest square that fits inside the
//! annotation's `/Rect`, centred — a choice too, and the reason for it is that these shapes
//! carry their meaning in their proportions: a pilcrow stretched to a 400×20 rectangle is not a
//! pilcrow. **§12.5.6.4's seven no longer reach that arithmetic**, since the
//! six-hundred-and-fortieth session: a text annotation is attached to a point and holds a fixed
//! size on the screen, so `crate::annotation`'s `anchored_icon` states its square outright and
//! `/Rect` supplies only the corner it hangs from. The other three clauses' icons are still
//! inscribed in the rectangle their annotation states.
//!
//! **And §12.5.6.4's own answer to the same question is applied since the two-hundred-and-
//! twentieth session:**
//!
//! > Text annotations shall not scale and rotate with the page; they shall behave as if the
//! > NoZoom and NoRotate annotation flags (see "Table 167 -Annotation flags") were always set.
//!
//! This comment said that needed "the `NoZoom` flag §12.5.3 does not apply" — true when written
//! and false from the two-hundred-and-seventeenth (ADR 0168), which is `doc/todo/01`'s first
//! sweep finding a stale blocker in *code* rather than in the ledger. The square below is still
//! what the icon is drawn in; what changed is that it is now the same size on the screen at every
//! magnification.
//!
//! A figure is filled or stroked, never both. A filled figure may lay overlapping subpaths on
//! top of each other and rely on §8.5.3.3.2's nonzero winding rule to union them, which is what
//! makes a pilcrow four rectangles and a disc instead of one traced outline; a stroked figure
//! may not, because a seam inside a union would be drawn.

use crate::appearance::ARC;

/// One step of a path, in the unit square.
///
/// The same four operators §8.5.2 gives a content stream, which is what these become.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Mark {
    /// `m`: begin a new subpath.
    Move([f32; 2]),
    /// `l`: a straight segment.
    Line([f32; 2]),
    /// `c`: a cubic Bézier, with both control points and the end.
    Curve([f32; 2], [f32; 2], [f32; 2]),
    /// `h`: close the subpath.
    Close,
}

/// One path of an icon, and how it is painted.
pub(crate) struct Figure {
    /// Filled with the nonzero winding rule, or stroked at [`STROKE_WIDTH`].
    pub filled: bool,
    /// The path, which may hold several subpaths when it is filled.
    pub marks: &'static [Mark],
}

/// The line width every stroked figure is drawn at, as a fraction of the icon's side.
///
/// A sixteenth of the icon reads as a drawn line rather than a hairline at the sizes a sticky
/// note is written at, and leaves the interior of a 0.8-wide sheet or balloon open.
pub(crate) const STROKE_WIDTH: f32 = 0.06;

/// The corner radius of the field Table 166's `/C` fills, as a fraction of the icon's side.
pub(crate) const BACKGROUND_RADIUS: f32 = 0.15;

/// The seven names §12.5.6.4 makes mandatory, and the artwork this processor supplies for each.
///
/// Returns `None` for anything else. The clause's next sentence — "Additional names may be
/// supported as well" — is a permission rather than a requirement, so a `/Name` outside these
/// seven is reported by name instead of being drawn as the default: Table 175's default of
/// `Note` is what an *absent* entry means, not what an unrecognised one means.
pub(crate) fn text_annotation(name: &[u8]) -> Option<&'static [Figure]> {
    match name {
        b"Note" => Some(&NOTE),
        b"Comment" => Some(&COMMENT),
        b"Key" => Some(&KEY),
        b"Help" => Some(&HELP),
        b"NewParagraph" => Some(&NEW_PARAGRAPH),
        b"Paragraph" => Some(&PARAGRAPH),
        b"Insert" => Some(&INSERT),
        _ => None,
    }
}

/// Table 175: "Default value: Note ."
pub(crate) const DEFAULT_TEXT_NAME: &[u8] = b"Note";

/// §12.5.6.15's four names, and the artwork this processor supplies for each.
///
/// Table 187 asks for them with a **should** rather than a shall:
///
/// > PDF writers should include this entry and PDF readers should provide predefined icon
/// > appearances for at least the following standard names: Graph , PushPin , Paperclip , Tag
///
/// `doc/todo/26` weighed that verb for a hundred and nineteen sessions and its condition was
/// "worth doing for the one corpus document only if the artwork can be argued from the clause's
/// own descriptions". **It can, and that is the whole difference from `Stamp`**: a push pin, a
/// paperclip, a tag and a graph are *objects*, and the clause names them. A name that names a
/// thing is a great deal more than the standard gives §12.5.6.4's seven, where `NewParagraph`
/// and `Insert` had to be invented out of a typographer's convention.
pub(crate) fn file_attachment(name: &[u8]) -> Option<&'static [Figure]> {
    match name {
        b"PushPin" => Some(&PUSH_PIN),
        b"Paperclip" => Some(&PAPERCLIP),
        b"Graph" => Some(&GRAPH),
        b"Tag" => Some(&TAG),
        _ => None,
    }
}

/// Table 187: "Default value: `PushPin` ."
pub(crate) const DEFAULT_FILE_ATTACHMENT_NAME: &[u8] = b"PushPin";

/// §12.5.6.16's two names, on the same reading as [`file_attachment`]'s four.
///
/// Table 188: "PDF writers should include this entry and PDF readers should provide predefined
/// icon appearances for at least the standard names Speaker and Mic ."
pub(crate) fn sound(name: &[u8]) -> Option<&'static [Figure]> {
    match name {
        b"Speaker" => Some(&SPEAKER),
        b"Mic" => Some(&MIC),
        _ => None,
    }
}

/// Table 188: "Default value: Speaker ."
pub(crate) const DEFAULT_SOUND_NAME: &[u8] = b"Speaker";

/// `PushPin`, the default: a pin seen from the side — a head, a collar and a point.
const PIN_HEAD: [Mark; 5] = bar(0.34, 0.70, 0.66, 0.86);
const PIN_COLLAR: [Mark; 5] = bar(0.42, 0.40, 0.58, 0.70);
const PIN_POINT: [Mark; 4] = triangle([0.42, 0.40], [0.58, 0.40], [0.50, 0.14]);
const PUSH_PIN: [Figure; 3] = [
    Figure {
        filled: true,
        marks: &PIN_HEAD,
    },
    Figure {
        filled: true,
        marks: &PIN_COLLAR,
    },
    Figure {
        filled: true,
        marks: &PIN_POINT,
    },
];

/// `Paperclip`: two nested hairpins, the inner one open at the other end.
const CLIP_OUTER: [Mark; 8] = [
    Mark::Move([0.34, 0.14]),
    Mark::Line([0.34, 0.72]),
    Mark::Curve([0.34, 0.86], [0.66, 0.86], [0.66, 0.72]),
    Mark::Line([0.66, 0.30]),
    Mark::Curve([0.66, 0.20], [0.46, 0.20], [0.46, 0.30]),
    Mark::Line([0.46, 0.66]),
    Mark::Curve([0.46, 0.74], [0.56, 0.74], [0.56, 0.66]),
    Mark::Line([0.56, 0.28]),
];
const PAPERCLIP: [Figure; 1] = [Figure {
    filled: false,
    marks: &CLIP_OUTER,
}];

/// `Graph`: two axes and three bars of rising height.
const GRAPH_AXES: [Mark; 3] = [
    Mark::Move([0.18, 0.86]),
    Mark::Line([0.18, 0.16]),
    Mark::Line([0.86, 0.16]),
];
const GRAPH_FIRST: [Mark; 5] = bar(0.28, 0.16, 0.42, 0.40);
const GRAPH_SECOND: [Mark; 5] = bar(0.46, 0.16, 0.60, 0.60);
const GRAPH_THIRD: [Mark; 5] = bar(0.64, 0.16, 0.78, 0.82);
const GRAPH: [Figure; 4] = [
    Figure {
        filled: false,
        marks: &GRAPH_AXES,
    },
    Figure {
        filled: true,
        marks: &GRAPH_FIRST,
    },
    Figure {
        filled: true,
        marks: &GRAPH_SECOND,
    },
    Figure {
        filled: true,
        marks: &GRAPH_THIRD,
    },
];

/// `Tag`: a luggage label — a pentagon pointing left, with an eyelet.
const TAG_BODY: [Mark; 7] = [
    Mark::Move([0.12, 0.50]),
    Mark::Line([0.34, 0.20]),
    Mark::Line([0.86, 0.20]),
    Mark::Line([0.86, 0.80]),
    Mark::Line([0.34, 0.80]),
    Mark::Close,
    Mark::Close,
];
const TAG_EYELET: [Mark; 6] = disc([0.34, 0.50], 0.07);
const TAG: [Figure; 2] = [
    Figure {
        filled: false,
        marks: &TAG_BODY,
    },
    Figure {
        filled: false,
        marks: &TAG_EYELET,
    },
];

/// `Speaker`, the default: a cone on a box, with two arcs of sound leaving it.
const SPEAKER_BODY: [Mark; 7] = [
    Mark::Move([0.14, 0.38]),
    Mark::Line([0.28, 0.38]),
    Mark::Line([0.48, 0.18]),
    Mark::Line([0.48, 0.82]),
    Mark::Line([0.28, 0.62]),
    Mark::Line([0.14, 0.62]),
    Mark::Close,
];
const SPEAKER_NEAR: [Mark; 2] = [
    Mark::Move([0.60, 0.36]),
    Mark::Curve([0.72, 0.44], [0.72, 0.56], [0.60, 0.64]),
];
const SPEAKER_FAR: [Mark; 2] = [
    Mark::Move([0.72, 0.26]),
    Mark::Curve([0.90, 0.40], [0.90, 0.60], [0.72, 0.74]),
];
const SPEAKER: [Figure; 3] = [
    Figure {
        filled: true,
        marks: &SPEAKER_BODY,
    },
    Figure {
        filled: false,
        marks: &SPEAKER_NEAR,
    },
    Figure {
        filled: false,
        marks: &SPEAKER_FAR,
    },
];

/// `Mic`: a capsule on a stand.
const MIC_CAPSULE: [Mark; 6] = [
    Mark::Move([0.38, 0.56]),
    Mark::Line([0.38, 0.74]),
    Mark::Curve([0.38, 0.88], [0.62, 0.88], [0.62, 0.74]),
    Mark::Line([0.62, 0.56]),
    Mark::Curve([0.62, 0.42], [0.38, 0.42], [0.38, 0.56]),
    Mark::Close,
];
const MIC_CRADLE: [Mark; 2] = [
    Mark::Move([0.26, 0.52]),
    Mark::Curve([0.26, 0.24], [0.74, 0.24], [0.74, 0.52]),
];
const MIC_STEM: [Mark; 2] = segment([0.50, 0.30], [0.50, 0.14]);
const MIC_FOOT: [Mark; 2] = segment([0.34, 0.14], [0.66, 0.14]);
const MIC: [Figure; 4] = [
    Figure {
        filled: true,
        marks: &MIC_CAPSULE,
    },
    Figure {
        filled: false,
        marks: &MIC_CRADLE,
    },
    Figure {
        filled: false,
        marks: &MIC_STEM,
    },
    Figure {
        filled: false,
        marks: &MIC_FOOT,
    },
];

/// An axis-aligned rectangle, as four corners and a close.
const fn bar(left: f32, bottom: f32, right: f32, top: f32) -> [Mark; 5] {
    [
        Mark::Move([left, bottom]),
        Mark::Line([right, bottom]),
        Mark::Line([right, top]),
        Mark::Line([left, top]),
        Mark::Close,
    ]
}

/// A circle, as the four Bézier quarter-arcs [`ARC`] makes accurate to a fifth of a pixel.
const fn disc(centre: [f32; 2], radius: f32) -> [Mark; 6] {
    let (x, y) = (centre[0], centre[1]);
    let grip = radius * ARC;
    [
        Mark::Move([x - radius, y]),
        Mark::Curve(
            [x - radius, y + grip],
            [x - grip, y + radius],
            [x, y + radius],
        ),
        Mark::Curve(
            [x + grip, y + radius],
            [x + radius, y + grip],
            [x + radius, y],
        ),
        Mark::Curve(
            [x + radius, y - grip],
            [x + grip, y - radius],
            [x, y - radius],
        ),
        Mark::Curve(
            [x - grip, y - radius],
            [x - radius, y - grip],
            [x - radius, y],
        ),
        Mark::Close,
    ]
}

/// A closed triangle.
const fn triangle(a: [f32; 2], b: [f32; 2], c: [f32; 2]) -> [Mark; 4] {
    [Mark::Move(a), Mark::Line(b), Mark::Line(c), Mark::Close]
}

/// A single straight stroke.
const fn segment(from: [f32; 2], to: [f32; 2]) -> [Mark; 2] {
    [Mark::Move(from), Mark::Line(to)]
}

/// `Note`, the default: a sheet of paper with a turned-down corner and three lines of writing.
const NOTE_SHEET: [Mark; 6] = [
    Mark::Move([0.20, 0.12]),
    Mark::Line([0.80, 0.12]),
    Mark::Line([0.80, 0.68]),
    Mark::Line([0.60, 0.88]),
    Mark::Line([0.20, 0.88]),
    Mark::Close,
];
const NOTE_FOLD: [Mark; 3] = [
    Mark::Move([0.60, 0.88]),
    Mark::Line([0.60, 0.68]),
    Mark::Line([0.80, 0.68]),
];
const NOTE_LINE_TOP: [Mark; 2] = segment([0.30, 0.62], [0.52, 0.62]);
const NOTE_LINE_MIDDLE: [Mark; 2] = segment([0.30, 0.44], [0.70, 0.44]);
const NOTE_LINE_BOTTOM: [Mark; 2] = segment([0.30, 0.26], [0.62, 0.26]);
const NOTE: [Figure; 5] = [
    Figure {
        filled: false,
        marks: &NOTE_SHEET,
    },
    Figure {
        filled: false,
        marks: &NOTE_FOLD,
    },
    Figure {
        filled: false,
        marks: &NOTE_LINE_TOP,
    },
    Figure {
        filled: false,
        marks: &NOTE_LINE_MIDDLE,
    },
    Figure {
        filled: false,
        marks: &NOTE_LINE_BOTTOM,
    },
];

/// The corner radius of `Comment`'s balloon, and the control-point offset that rounds it.
const BALLOON_RADIUS: f32 = 0.14;
const BALLOON_GRIP: f32 = BALLOON_RADIUS * (1.0 - ARC);

/// `Comment`: a speech balloon with a tail, and two lines of writing in it.
const COMMENT_BALLOON: [Mark; 13] = [
    Mark::Move([0.26, 0.34]),
    Mark::Line([0.30, 0.34]),
    Mark::Line([0.22, 0.10]),
    Mark::Line([0.50, 0.34]),
    Mark::Line([0.74, 0.34]),
    Mark::Curve(
        [0.88 - BALLOON_GRIP, 0.34],
        [0.88, 0.34 + BALLOON_GRIP],
        [0.88, 0.48],
    ),
    Mark::Line([0.88, 0.76]),
    Mark::Curve(
        [0.88, 0.90 - BALLOON_GRIP],
        [0.88 - BALLOON_GRIP, 0.90],
        [0.74, 0.90],
    ),
    Mark::Line([0.26, 0.90]),
    Mark::Curve(
        [0.12 + BALLOON_GRIP, 0.90],
        [0.12, 0.90 - BALLOON_GRIP],
        [0.12, 0.76],
    ),
    Mark::Line([0.12, 0.48]),
    Mark::Curve(
        [0.12, 0.34 + BALLOON_GRIP],
        [0.12 + BALLOON_GRIP, 0.34],
        [0.26, 0.34],
    ),
    Mark::Close,
];
const COMMENT_LINE_TOP: [Mark; 2] = segment([0.26, 0.72], [0.74, 0.72]);
const COMMENT_LINE_BOTTOM: [Mark; 2] = segment([0.26, 0.56], [0.60, 0.56]);
const COMMENT: [Figure; 3] = [
    Figure {
        filled: false,
        marks: &COMMENT_BALLOON,
    },
    Figure {
        filled: false,
        marks: &COMMENT_LINE_TOP,
    },
    Figure {
        filled: false,
        marks: &COMMENT_LINE_BOTTOM,
    },
];

/// The corner radius of `Key`'s bow, and the control-point offset that rounds it.
const BOW_RADIUS: f32 = 0.12;
const BOW_GRIP: f32 = BOW_RADIUS * (1.0 - ARC);

/// `Key`: one traced outline — a rounded bow, a shaft, and two teeth hanging off it.
///
/// Traced rather than assembled from a bow and a shaft because it is stroked, and two
/// overlapping stroked subpaths would draw the seam where they meet.
const KEY_OUTLINE: [Mark; 21] = [
    Mark::Move([0.20, 0.45]),
    Mark::Line([0.38, 0.45]),
    Mark::Curve(
        [0.50 - BOW_GRIP, 0.45],
        [0.50, 0.45 + BOW_GRIP],
        [0.50, 0.57],
    ),
    Mark::Line([0.50, 0.61]),
    Mark::Line([0.66, 0.61]),
    Mark::Line([0.66, 0.50]),
    Mark::Line([0.72, 0.50]),
    Mark::Line([0.72, 0.61]),
    Mark::Line([0.78, 0.61]),
    Mark::Line([0.78, 0.50]),
    Mark::Line([0.84, 0.50]),
    Mark::Line([0.84, 0.61]),
    Mark::Line([0.92, 0.61]),
    Mark::Line([0.92, 0.71]),
    Mark::Line([0.50, 0.71]),
    Mark::Line([0.50, 0.75]),
    Mark::Curve(
        [0.50, 0.87 - BOW_GRIP],
        [0.50 - BOW_GRIP, 0.87],
        [0.38, 0.87],
    ),
    Mark::Line([0.20, 0.87]),
    Mark::Curve(
        [0.08 + BOW_GRIP, 0.87],
        [0.08, 0.87 - BOW_GRIP],
        [0.08, 0.75],
    ),
    Mark::Line([0.08, 0.57]),
    Mark::Close,
];
const KEY_HOLE: [Mark; 6] = disc([0.29, 0.66], 0.09);
const KEY: [Figure; 2] = [
    Figure {
        filled: false,
        marks: &KEY_OUTLINE,
    },
    Figure {
        filled: false,
        marks: &KEY_HOLE,
    },
];

/// `Help`: a question mark in a ring.
///
/// The hook is stroked rather than filled for the same reason the key is traced: a glyph's
/// filled outline is a font's work, and this module has no font.
const HELP_RING: [Mark; 6] = disc([0.50, 0.50], 0.40);
const HELP_HOOK: [Mark; 5] = [
    Mark::Move([0.34, 0.62]),
    Mark::Curve([0.34, 0.76], [0.44, 0.82], [0.52, 0.82]),
    Mark::Curve([0.62, 0.82], [0.70, 0.75], [0.70, 0.66]),
    Mark::Curve([0.70, 0.56], [0.60, 0.54], [0.55, 0.48]),
    Mark::Curve([0.53, 0.45], [0.52, 0.43], [0.52, 0.40]),
];
const HELP_DOT: [Mark; 6] = disc([0.52, 0.28], 0.055);
const HELP: [Figure; 3] = [
    Figure {
        filled: false,
        marks: &HELP_RING,
    },
    Figure {
        filled: false,
        marks: &HELP_HOOK,
    },
    Figure {
        filled: true,
        marks: &HELP_DOT,
    },
];

/// `Paragraph`: a pilcrow, as two stems, a head bar and a bowl unioned by the winding rule.
const PILCROW_RIGHT_STEM: [Mark; 5] = bar(0.68, 0.10, 0.80, 0.90);
const PILCROW_LEFT_STEM: [Mark; 5] = bar(0.48, 0.10, 0.60, 0.90);
const PILCROW_HEAD: [Mark; 5] = bar(0.24, 0.78, 0.80, 0.90);
const PILCROW_BOWL: [Mark; 6] = disc([0.42, 0.66], 0.18);
const PARAGRAPH: [Figure; 4] = [
    Figure {
        filled: true,
        marks: &PILCROW_RIGHT_STEM,
    },
    Figure {
        filled: true,
        marks: &PILCROW_LEFT_STEM,
    },
    Figure {
        filled: true,
        marks: &PILCROW_HEAD,
    },
    Figure {
        filled: true,
        marks: &PILCROW_BOWL,
    },
];

/// `NewParagraph`: two blocks of writing with a caret pointing into the break between them.
///
/// Deliberately not a pilcrow with something added: `Paragraph` is a pilcrow, and two icons a
/// reader has to look twice at are worse than two that share nothing.
const NEW_PARAGRAPH_FIRST: [Mark; 5] = bar(0.14, 0.80, 0.86, 0.88);
const NEW_PARAGRAPH_SECOND: [Mark; 5] = bar(0.14, 0.66, 0.60, 0.74);
const NEW_PARAGRAPH_THIRD: [Mark; 5] = bar(0.14, 0.26, 0.86, 0.34);
const NEW_PARAGRAPH_FOURTH: [Mark; 5] = bar(0.14, 0.12, 0.72, 0.20);
const NEW_PARAGRAPH_CARET: [Mark; 4] = triangle([0.34, 0.42], [0.50, 0.58], [0.66, 0.42]);
const NEW_PARAGRAPH: [Figure; 5] = [
    Figure {
        filled: true,
        marks: &NEW_PARAGRAPH_FIRST,
    },
    Figure {
        filled: true,
        marks: &NEW_PARAGRAPH_SECOND,
    },
    Figure {
        filled: true,
        marks: &NEW_PARAGRAPH_THIRD,
    },
    Figure {
        filled: true,
        marks: &NEW_PARAGRAPH_FOURTH,
    },
    Figure {
        filled: true,
        marks: &NEW_PARAGRAPH_CARET,
    },
];

/// `Insert`: a proofreader's caret standing on the line the text goes into.
const INSERT_CARET: [Mark; 4] = triangle([0.18, 0.22], [0.50, 0.76], [0.82, 0.22]);
const INSERT_BASELINE: [Mark; 2] = segment([0.12, 0.14], [0.88, 0.14]);
const INSERT: [Figure; 2] = [
    Figure {
        filled: true,
        marks: &INSERT_CARET,
    },
    Figure {
        filled: false,
        marks: &INSERT_BASELINE,
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    /// Every one of the seven names §12.5.6.4 makes mandatory has artwork.
    ///
    /// The clause's list, in the clause's order. A name added to it in a future edition fails
    /// here rather than silently becoming a report.
    #[test]
    fn the_seven_names_the_clause_requires_all_have_an_appearance() {
        for name in [
            &b"Comment"[..],
            b"Key",
            b"Note",
            b"Help",
            b"NewParagraph",
            b"Paragraph",
            b"Insert",
        ] {
            assert!(
                text_annotation(name).is_some_and(|figures| !figures.is_empty()),
                "{} has no artwork",
                String::from_utf8_lossy(name)
            );
        }
        assert!(text_annotation(DEFAULT_TEXT_NAME).is_some());
    }

    /// Nothing outside the seven is drawn, because nothing outside them is required.
    #[test]
    fn a_name_the_clause_does_not_list_has_none() {
        assert!(text_annotation(b"Circle").is_none());
        assert!(text_annotation(b"note").is_none());
        assert!(text_annotation(b"").is_none());
    }

    /// No mark leaves the unit square, including a Bézier's control points.
    ///
    /// The whole of this module is hand-placed coordinates, and a transposed digit would put
    /// part of a symbol outside the icon's square, where it would overlap the page instead of
    /// the background. A control point outside is caught too: it cannot move the curve outside
    /// on its own, but it is always a mistake here and never a technique.
    #[test]
    fn every_coordinate_lies_in_the_unit_square() {
        let inside =
            |point: [f32; 2]| (0.0..=1.0).contains(&point[0]) && (0.0..=1.0).contains(&point[1]);
        for name in [
            &b"Comment"[..],
            b"Key",
            b"Note",
            b"Help",
            b"NewParagraph",
            b"Paragraph",
            b"Insert",
        ] {
            let figures = text_annotation(name).expect("one of the seven");
            for figure in figures {
                for mark in figure.marks {
                    match *mark {
                        Mark::Move(point) | Mark::Line(point) => assert!(
                            inside(point),
                            "{}: {point:?}",
                            String::from_utf8_lossy(name)
                        ),
                        Mark::Curve(first, second, end) => {
                            for point in [first, second, end] {
                                assert!(
                                    inside(point),
                                    "{}: {point:?}",
                                    String::from_utf8_lossy(name)
                                );
                            }
                        }
                        Mark::Close => {}
                    }
                }
            }
        }
    }

    /// Every figure starts a subpath before it draws one.
    ///
    /// A `Line` or a `Curve` with no `Move` before it is §8.5.2.1's error — "if the current
    /// point is undefined" — and on a path this module writes it would be a typo rather than a
    /// document's doing.
    #[test]
    fn every_subpath_begins_with_a_move() {
        for name in [
            &b"Comment"[..],
            b"Key",
            b"Note",
            b"Help",
            b"NewParagraph",
            b"Paragraph",
            b"Insert",
        ] {
            for figure in text_annotation(name).expect("one of the seven") {
                assert!(
                    matches!(figure.marks.first(), Some(Mark::Move(_))),
                    "{} starts a figure without a move",
                    String::from_utf8_lossy(name)
                );
            }
        }
    }
}
