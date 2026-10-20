//! §11.6.7: the graphics state a shading pattern's definition is evaluated under, and what a pattern
//! dictionary's own `/ExtGState` adds to it (ADR 2081).

use std::rc::Rc;
use std::sync::Arc;

use pdf_syntax::{Dictionary, Object};

use crate::black_generation::BlackGeneration;
use crate::icc::Rendering;

use crate::content::colour::{BlackPoint, Intent};
use crate::content::run::narrow;
use crate::content::{GraphicsState, Interpreter};

/// The graphics state a shading pattern's *definition* is evaluated under (ISO 32000-2 §11.6.7).
///
/// A pattern is a colour (§8.7.2) and `scn` is where a colour is set, so the obvious moment to
/// resolve a shading pattern's colours is the `scn` — and the obvious repair, when that turns out
/// to be several graphics states before the mark, is to move it to the mark. **The clause names
/// neither**, twice and in two places. §11.6.7:
///
/// > The definition shall not inherit the current values of the graphics state parameters at the
/// > time it is evaluated; those parameters shall take effect only when the resulting pattern is
/// > later used to paint an object.
///
/// > Any parameters that are not so specified shall be inherited from the graphics state that was
/// > in effect at the beginning of the content stream in which the shading pattern is set to be
/// > the current colour in the graphics state or in which the sh operator is used.
///
/// and Table 75's `/ExtGState` entry says the same of the same parameters — "inherited from the
/// graphics state that was in effect at the beginning of the pattern's parent content stream, and
/// as modified by clause 11.6.7".
///
/// This tree already obeyed that rule for **one** parameter and had never noticed: §8.7.2's
/// pattern matrix maps to "the default coordinate system of the pattern's parent content stream",
/// which is `Interpreter::base`, swapped at each of the four ways of becoming a parent. The
/// transformation matrix is the first of the three §11.6.7 names; the other two are this struct.
///
/// # Which parameters, and why not the transfer function
///
/// §11.6.7's third bullet names them: "Only those parameters that affect the sh operator, such as
/// the current transformation matrix, black point compensation and rendering intent, shall be
/// used." §10.7.3's smoothness is a fourth by the same test — it is the tolerance a shading's
/// colour function is sampled to, and nothing else reads it.
///
/// §10.5's transfer function is deliberately **not** here, and the reason is §11.7.5.3's NOTE
/// about the intent it has just placed at the painting operation:
///
/// > This differs from the current halftone and transfer function, whose values are used only when
/// > all colour compositing has been completed and rasterization is being performed.
///
/// So a transfer is not part of a pattern's evaluation at all; §11.7.5.2 puts it at the topmost
/// painting object, which is the mark — and [`super::ShadingDefinition`] is what lets the mark have it
/// without any of these three moving with it.
#[derive(Debug, Clone)]
pub(in crate::content) struct PatternInitial {
    /// Table 57's `/UseBlackPtComp` as the beginning of this content stream had it.
    black_point: BlackPoint,
    /// §8.6.5.8's rendering intent, which §8.6.5.9 lets override the entry above.
    intent: Intent,
    /// Table 57's `/SM`, §10.7.3's smoothness tolerance.
    pub(super) smoothness: Option<f32>,
    /// Table 57's `/BG`, `/BG2`, `/UCR` and `/UCR2`, §10.4.2.4's pair.
    ///
    /// A fourth parameter that affects `sh`, on the third bullet's own test: it decides what a
    /// colour becomes when the shading's ramp is converted into a `DeviceCMYK` group (§11.7.5.3,
    /// ADR 1207), and the bullet's list is "such as" rather than closed. It is also why this
    /// type is no longer `Copy`.
    black_generation: Option<Arc<BlackGeneration>>,
    /// The whole graphics state the content stream began with, which is what a *tiling*
    /// pattern's cell starts from (ISO 32000-2 §8.7.3.1).
    ///
    /// The second of the clause's four steps is
    ///
    /// > Installs the graphics state that was in effect at the beginning of the pattern's
    /// > parent content stream, with the current transformation matrix altered by the pattern
    /// > matrix as described in 8.7.2, "General properties of patterns"
    ///
    /// and §8.7.2 says which stream that is where a form is concerned:
    ///
    /// > Similarly, if a pattern is used within a form XObject (see 8.10, "Form XObjects" ), the
    /// > pattern matrix maps pattern space to the form's default user space (that is, the form
    /// > coordinate space at the time the form is painted with the Do operator).
    ///
    /// So a cell named inside a form starts from the form's
    /// state at its `Do` — its `/AIS` among the rest — rather than from §8.4's initial state,
    /// and [`Interpreter::run_cell`] takes §11.6.7's resets off this. Shared rather than copied
    /// because a shading definition holds this struct too and never reads the field.
    pub(super) stream: Rc<GraphicsState>,
}

impl PatternInitial {
    /// The parameters as one graphics state has them.
    ///
    /// Read off the state a content stream *begins* with, which is what makes it the clause's
    /// quantity — see [`Interpreter::run_reader`], the one place this is called.
    pub(in crate::content) fn of(state: &GraphicsState) -> Self {
        Self {
            black_point: state.use_black_pt_comp,
            intent: state.intent,
            smoothness: state.smoothness,
            black_generation: state.black_generation.clone(),
            stream: Rc::new(state.clone()),
        }
    }

    /// The same parameters as Table 75's `/ExtGState` augments them, per §11.6.7's third bullet.
    ///
    /// > In the case of a shading pattern, the parameter values may be augmented by the contents
    /// > of the ExtGState entry in the pattern dictionary (see 8.7.4, "Shading patterns"). Only
    /// > those parameters that affect the sh operator, such as the current transformation matrix,
    /// > black point compensation and rendering intent, shall be used. Parameters that affect
    /// > path-painting operators shall not be used, since the execution of sh does not entail
    /// > painting a path.
    ///
    /// Three entries are read because three are what a shading's colours depend on here.
    /// Everything else Table 57 can carry is either a path-painting parameter the sentence
    /// excludes outright, one §11.6.7's first bullet has already initialised (the blend mode, the
    /// two alpha constants, the soft mask), or one this device does not perform at all —
    /// §10.6's halftone screen, §8.6.7's overprint, §10.4's black generation and undercolour
    /// removal.
    /// **A `/TR`, a `/TR2` or an `/HT` here is none of those and is deliberately not read**:
    /// §11.7.5.3's NOTE takes the transfer function out of the group evaluation, so a pattern's
    /// `/ExtGState` cannot state one for its own colours by either of §10.5's two routes. All three
    /// are silent for the same reason and none of them is reported;
    /// `Interpreter::note_black_generation` is what reports the entries that are skipped and
    /// could have marked, which is `/BG` and `/UCR`.
    pub(super) fn augmented(self, document: &pdf_syntax::Document, dict: &Dictionary) -> Self {
        let Some(state) = document.get_key(dict, "ExtGState").as_dict().cloned() else {
            return self;
        };
        let mut augmented = self;
        if let Object::Name(value) = document.get_key(&state, "UseBlackPtComp") {
            augmented.black_point = match value.as_bytes() {
                b"ON" => BlackPoint::On,
                b"OFF" => BlackPoint::Off,
                _ => BlackPoint::Default,
            };
        }
        if let Object::Name(intent) = document.get_key(&state, "RI") {
            augmented.intent = Intent::read(intent.as_bytes());
        }
        if let Some(tolerance) = document.get_key(&state, "SM").as_number() {
            augmented.smoothness = Some(narrow(tolerance));
        }
        augmented.black_generation =
            BlackGeneration::read(document, &state, augmented.black_generation.as_deref());
        augmented
    }

    /// Whether §8.6.5.9's black point compensation applies to the colours built under this.
    ///
    /// The same combination [`GraphicsState::black_point_under`] makes, for the same reason: the
    /// clause states the override over an object's intent rather than over the entry.
    fn black_point(&self) -> BlackPoint {
        if self.intent == Intent::Absolute {
            return BlackPoint::Off;
        }
        self.black_point
    }

    /// The rendering parameters the colours built under this convert with.
    ///
    /// The same pair [`GraphicsState::rendering`] makes, off the state §11.6.7 puts a shading
    /// pattern's definition under: §8.6.5.8's intent selects the profile transform and decides
    /// the black point above.
    pub(super) fn rendering(&self) -> Rendering {
        Rendering::new(self.intent.a2b(), self.black_point().applies())
    }

    /// §10.4.2.4's pair the colours built under this convert with, if the state states one.
    pub(super) fn black_generation(&self) -> Option<Arc<BlackGeneration>> {
        self.black_generation.clone()
    }
}

impl Interpreter<'_> {
    // Which of Table 57's entries §11.6.7 lets a shading pattern's `/ExtGState` state, and what
    // becomes of each here. The clause admits "those parameters that affect the sh operator" and
    // excludes those that "affect path-painting operators", and every entry falls into one of five
    // buckets. Written down because a reader of `PatternInitial::augmented` will otherwise ask why
    // three entries are read and twenty are not, and because the answer is the clause's for most of
    // them rather than this tree's:
    //
    // - **Read there**: `/UseBlackPtComp`, `/RI`, `/SM` — the three §11.6.7's own sentence names, less
    //   the transformation matrix, which Table 57 cannot state and which `Interpreter::base` already
    //   carries — and `/BG`, `/BG2`, `/UCR`, `/UCR2`, which the sentence does not name and which its
    //   "such as" does not exclude: §10.4.2.4's pair decides what a shading's ramp becomes when it is
    //   converted into a `DeviceCMYK` group, so it affects `sh` on the bullet's own test (ADR 1207).
    // - **Excluded by §11.6.7's first bullet**, which initialises them: `/BM`, `/CA`, `/ca`,
    //   `/SMask`, `/AIS`. A pattern's `/ExtGState` may not put a blend mode or an alpha constant
    //   back, because "as always for transparency groups" they are the group's own, applied once
    //   where the pattern is used.
    // - **Excluded because they affect path painting** and `sh` "does not entail painting a path":
    //   `/LW`, `/LC`, `/LJ`, `/ML`, `/D`, `/SA`, `/FL`, and the whole of Table 102's text state.
    // - **Left to a standing decision recorded elsewhere**, because this device does not perform it
    //   at all: `/OP`, `/op` and `/OPM`, on §8.6.7's own permission.
    // - **`/TR`, `/TR2` and `/HT`**, which are none of those: §11.7.5.3's NOTE takes the transfer
    //   function out of the group evaluation entirely, so one stated here says nothing about the
    //   pattern's own colours. `/HT` is here rather than in the bullet above because §10.6's
    //   condition covers a halftone *screen*, not the `TransferFunction` §10.5's second bullet reads
    //   out of a halftone dictionary (ADR 0505): it is skipped for the transfer function's reason
    //   rather than the screen's.
    //   See `PatternInitial`.
    /// Records Table 57's black generation and undercolour removal where a *pattern dictionary*
    /// states them (ISO 32000-2 §11.6.7's third bullet, Table 75's `/ExtGState`).
    ///
    /// `Interpreter::apply_ext_gstate` records the same statement for the `gs` operator; this is
    /// the second of the two routes §8.4.5's parameters have to the same page, and reading one of
    /// them is the failure mode that reports nothing. The flag is monotone for the page there and
    /// here for the same reason: the parameters apply wherever a §10.4.2.4 conversion happens,
    /// not only where they were set. What the flag decides is
    /// `Interpreter::note_black_generation_departure`, which is now a report about the two
    /// moments of §11.7.5.3 this tree does *not* carry out rather than about all three; the pair
    /// itself reaches a shading pattern's colours through `PatternInitial::augmented` above.
    pub(super) fn note_black_generation(&mut self, dict: &Dictionary) {
        let Some(state) = self.document.get_key(dict, "ExtGState").as_dict().cloned() else {
            return;
        };
        if self.states_black_generation(&state) {
            self.black_generation_stated = true;
        }
    }
}
