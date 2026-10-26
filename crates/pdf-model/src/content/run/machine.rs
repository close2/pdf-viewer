//! What a run holds between one operator and the next, and what it leaves where a stream ends.

use pdf_render::{FillRule, Path, Point, Transform};

use crate::content::marked::Marked;

use crate::content::GraphicsState;
use crate::content::text::TextObject;

/// §8.4.2's current graphics state and the stack beneath it, carrying §9.4.2's two text matrices
/// beside each saved state — where a run begins, and where it ends.
#[derive(Debug, Clone)]
pub(in crate::content) struct Composed {
    /// The current graphics state.
    pub(in crate::content) state: GraphicsState,
    /// The states `q` saved, innermost last.
    pub(in crate::content) stack: Vec<(GraphicsState, Transform, Transform)>,
    /// The marked-content sections open, outermost first — only ever sections that record
    /// nothing but whether they hide (§8.11.3.2), where a piece of a divided stream begins
    /// inside them (ADR 1894).
    pub(in crate::content) marked: Vec<Marked>,
}

impl Composed {
    /// A stream's own beginning: `initial` in force and nothing saved.
    pub(in crate::content) fn fresh(initial: &GraphicsState) -> Self {
        Self {
            state: initial.clone(),
            stack: Vec::new(),
            marked: Vec::new(),
        }
    }

    /// How many of the open sections hide what they enclose — the interpreter's `hidden` count
    /// a run beginning in this state begins with.
    pub(in crate::content) fn hiding(&self) -> usize {
        self.marked.iter().filter(|section| section.hides).count()
    }
}

/// What a run holds between one operator and the next, other than the operands: the graphics
/// state and the stack beneath it, the path and the text object being built, and the sections
/// open. The operator families (`run_graphics_state_op`, `run_path_painting_op`, …) each read and
/// change the part they own, which is why this is one value passed to them rather than a dozen
/// borrows.
pub(super) struct Machine {
    /// The current graphics state.
    pub(super) state: GraphicsState,
    /// §8.4.2's stack, carrying §9.4.2's two text matrices beside the graphics state.
    ///
    /// ISO 32000-2 §9.4.2, as Errata Collection 3 adds it (issue #368, `/State` `Review`
    /// `Completed`): within a text object the graphics state stack operators q and Q "shall
    /// additionally push and pop Tm and Tlm as part of the graphics state stack". Quoted in
    /// prose rather than as a blockquote because the sentence is an addition and `doc/md/`
    /// is a conversion of the base text — see `text_state.rs`'s test for the whole reading.
    ///
    /// So the entry is a triple rather than a `GraphicsState`: the two matrices are saved
    /// and restored *with* it, which is what "as part of the graphics state stack" says, and
    /// holding them in a stack of their own would let the two go out of step on a stream
    /// whose `q` is inside a text object and whose `Q` is outside it. ADR 0421.
    pub(super) stack: Vec<(GraphicsState, Transform, Transform)>,
    /// The path being built.
    pub(super) path: Path,
    /// Where the current subpath began.
    pub(super) start: Point,
    /// The current point.
    pub(super) current: Point,
    /// The pending clip requested by `W`/`W*`.
    pub(super) pending_clip: Option<FillRule>,
    /// Whether a text object is open.
    pub(super) in_text: bool,
    /// The text object's own parameters. `BT` resets them, and `q`/`Q` save and restore
    /// them with the graphics state — §9.4.2 as Errata Collection 3 amends it, above.
    pub(super) text_object: TextObject,
    /// One entry per open marked-content section, saying whether it hid what follows.
    /// Every `BMC` and `BDC` pushes, so an `EMC` closes the section it actually belongs
    /// to rather than the last optional one — which is why this is not just a counter.
    ///
    /// It carries no bound of its own: it lives for one stream, a section costs at least the
    /// five bytes of `/ BMC` in it, and Table 31's `Limits::max_stream_len` bounds those —
    /// the same bound the token loop has. What a closed section leaves behind is a
    /// span, and the spans are charged to `MAX_LIST_BYTES` (ADR 1810).
    pub(super) marked: Vec<Marked>,
    /// §7.8.2's compatibility section, `BX` … `EX`, which is nesting depth rather than a
    /// flag because the clause says the pair "may be nested".
    ///
    /// > Ordinarily, when a PDF reader encounters an operator in a content stream that it
    /// > does not recognise, an error shall occur.
    ///
    /// — and inside the section, "unrecognised operators (along with their operands) shall
    /// be ignored without error". So this is the one place in the interpreter where an
    /// unsupported *input* is deliberately silent, and it is silent because the file said
    /// in advance that ignoring it is the appropriate thing to do.
    pub(super) compatibility: usize,
    /// Where the last `/ActualText` replacement ended in the readback, for §14.9.4's rule
    /// that two consecutive ones have no word break between them.
    pub(super) replaced_ends_at: Option<usize>,
}

impl Machine {
    /// The machine a run begins with: `from` in force and nothing built.
    pub(super) fn begin(from: Composed) -> Self {
        let Composed {
            state,
            stack,
            marked,
        } = from;
        Self {
            state,
            stack,
            path: Path::new(),
            start: Point::new(0.0, 0.0),
            current: Point::new(0.0, 0.0),
            pending_clip: None,
            in_text: false,
            text_object: TextObject::default(),
            marked,
            compatibility: 0,
            replaced_ends_at: None,
        }
    }

    /// What the run left open where its stream ran out.
    pub(super) fn into_leftover(self) -> Leftover {
        Leftover {
            composed: Composed {
                state: self.state,
                stack: self.stack,
                marked: Vec::new(),
            },
            in_text: self.in_text,
            text_object: self.text_object,
            pending_clip: self.pending_clip,
            marked: self.marked,
            compatibility: self.compatibility,
            path_open: !self.path.commands().is_empty(),
        }
    }
}

/// What a run left open where its stream ran out, for [`Interpreter::end_of_stream`](crate::content::Interpreter::end_of_stream) to report.
#[derive(Debug)]
pub(in crate::content) struct Leftover {
    /// The state the run ended in.
    pub(in crate::content) composed: Composed,
    /// Whether a text object was open.
    pub(super) in_text: bool,
    /// The text object's own parameters.
    pub(super) text_object: TextObject,
    /// A `W` or `W*` no painting operator followed.
    pub(super) pending_clip: Option<FillRule>,
    /// The marked-content sections still open.
    pub(super) marked: Vec<Marked>,
    /// How many compatibility sections were still open.
    pub(super) compatibility: usize,
    /// Whether a path was under construction.
    pub(super) path_open: bool,
}

impl Leftover {
    /// Whether nothing but the graphics state, and marked-content sections that record nothing
    /// but whether they hide, is open — the only point at which a page's stream may be divided
    /// (ADRs 1880, 1894): no text object (§9.4.1), no path or pending clip (§8.5.2.1), no
    /// compatibility section (§7.8.2), and no section a join would have to carry (§8.11.3.2's
    /// optional content is decided again for the next piece, from the sections its spine opens).
    pub(in crate::content) fn only_plain_sections_open(&self) -> bool {
        !self.in_text
            && self.pending_clip.is_none()
            && self.marked.iter().all(Marked::records_nothing)
            && self.compatibility == 0
            && !self.path_open
            && self.text_object.enclosing_reading.is_none()
    }

    /// The state the run ended in, with the sections it left open — what the next piece of a
    /// divided stream begins in.
    pub(in crate::content) fn into_composed(self) -> Composed {
        Composed {
            marked: self.marked,
            ..self.composed
        }
    }
}
