//! ISO 32000-2 §7.10.5's type 4 function, in the form a *device* is handed one.
//!
//! # Why the display list carries a program at all
//!
//! §8.7.4.5.2's type 1 shading is "a function of two variables" over a domain, and ADR 0339
//! decided that the function is read at the device's own grid rather than at a resolution
//! chosen while interpreting. That decision stands and this module does not touch it: the
//! colours still come from [`crate::DeferredColours`], one evaluation per device pixel, and
//! that is what every backend draws unless it can do better.
//!
//! What ADR 0376 adds is the *option* of doing better. A backend whose graphics library can
//! evaluate the program itself — quorra's `Paint::Function`, one generated fragment shader —
//! needs the program, and it can only see this crate. So [`ShadingProgram`] rides beside the
//! producer on [`ShadingKind::Sampled`](crate::ShadingKind::Sampled), as an *alternative
//! statement of the same colours*, and a backend that has no use for it ignores the field.
//!
//! # Why this is not `pdf_model::function`'s vocabulary
//!
//! `pdf-model` compiles §7.10.5's source and evaluates the result, and its `Instruction`
//! carries a `Value` whose Annex B coercions — a boolean read as the number it stands for, a
//! real truncated where an integer is wanted — are *evaluator* semantics. A device evaluates
//! nothing here: it is handed a list and generates code from it. So the two forms differ in
//! exactly the two places that matter to a code generator and nowhere else:
//!
//! - **the literal carries its type in the instruction** ([`ProgramStep::PushInt`],
//!   [`ProgramStep::PushReal`], [`ProgramStep::PushBool`]) rather than inside a `Value`,
//!   because §7.10.5.1's `not`, `and`, `or` and `xor` are different operations on a boolean
//!   and on an integer and a generator has to choose before it writes a line;
//! - **a jump target is a `u32`**, because a program handed to a device is bounded by that
//!   device's own budget and an index that cannot be expressed is a refusal rather than a
//!   `usize`.
//!
//! Everything else is one-to-one, and `pdf_model::shading` owns the lowering — the one place
//! that knows both forms. A test there matches on every `Operator` with no wildcard arm, so a
//! Table 42 operator added on that side cannot silently fail to arrive here.
//!
//! # What a step does *not* say
//!
//! Nothing here states an arithmetic. §7.3.3 defers a number's precision to "the internal
//! representations used in the computer on which the PDF processor is running" and §7.10.5.2
//! defers Table 42's semantics to PLRM3, which defers again to the hardware, so a backend
//! evaluating this list and this crate's own [`crate::DeferredColours`] evaluating the same
//! function are two readings of one mapping rather than one reading twice. ADR 0376 section 4
//! is where that costs something.

use std::sync::Arc;

/// One step of a compiled §7.10.5 type 4 (PostScript calculator) function.
///
/// Table 42's forty-two operators are here as one variant each, except that `if` and `ifelse`
/// arrive already lowered to [`ProgramStep::JumpUnless`] and [`ProgramStep::Jump`] — the
/// `{ … }` blocks, the comments and the tokens are `pdf_model::function`'s and stay there — and
/// that `true` and `false` are [`ProgramStep::PushBool`]. The three `Push*` steps are literals,
/// which Table 42 does not list because source text needs no operator to write one down.
///
/// **Jumps are forward only**, which is what makes a program's length a bound on its own
/// execution and is why a device can run one without a loop. It is a property of what
/// `pdf_model::function::compile_postscript` produces rather than a claim this type enforces;
/// a device that depends on it validates it.
///
/// **Deliberately not `#[non_exhaustive]`, which is the opposite of this crate's habit.**
/// Table 42 is closed — the standard lists forty-two operators and states no way to extend
/// them — and a backend translating this list into its library's own vocabulary must be
/// *broken* by a step it has never seen rather than pushed into a wildcard arm that would
/// draw it as something else. The cost is that adding a variant is a breaking change for
/// every backend, which is exactly the property being bought.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProgramStep {
    /// An integer literal. §7.10.5.2 hands the operand *syntax* to PDF rather than to
    /// PostScript — "[t]he operand syntax for Type 4 functions shall follow PDF conventions
    /// rather than PostScript language conventions" — and §7.3.2's integer is digits with an
    /// optional sign, so `63` is an integer because the file said so (ADR 0371).
    PushInt(i32),
    /// A real literal: §7.3.3's number, the one carrying a PERIOD.
    PushReal(f32),
    /// §7.10.5.1's `true` and `false`.
    PushBool(bool),
    /// One of Table 42's operators, `if` and `ifelse` excepted.
    Operator(ProgramOperator),
    /// Pop a boolean; when it is false, continue at `target`. Forward only.
    JumpUnless {
        /// An index into the program, ahead of this step. The program's own length means
        /// "stop", which is how a trailing `if` lowers.
        target: u32,
    },
    /// Continue at `target`. Forward only, under [`ProgramStep::JumpUnless`]'s rule.
    Jump {
        /// See [`ProgramStep::JumpUnless::target`].
        target: u32,
    },
}

/// Table 42's operators, less the two this crate's caller has already lowered to jumps.
///
/// One variant per operator and nothing else: the semantics are §7.10.5.2's, which that clause
/// delegates to PLRM3, and they are stated once — beside the evaluator in
/// `pdf_model::function`, where the choices this project had to make are argued (ADRs 0369 and
/// 0371). Restating them here would be two statements of one rule.
///
/// Closed for [`ProgramStep`]'s reason, and it is the same reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(
    missing_docs,
    reason = "each variant is the Table 42 operator it is named for; PLRM3 states the semantics \
              and `pdf_model::function` states this project's readings of them"
)]
pub enum ProgramOperator {
    Abs,
    Add,
    Atan,
    Ceiling,
    Cos,
    Cvi,
    Cvr,
    Div,
    Exp,
    Floor,
    Idiv,
    Ln,
    Log,
    Mod,
    Mul,
    Neg,
    Round,
    Sin,
    Sqrt,
    Sub,
    Truncate,
    And,
    Bitshift,
    Eq,
    Ge,
    Gt,
    Le,
    Lt,
    Ne,
    Not,
    Or,
    Xor,
    Copy,
    Dup,
    Exch,
    Index,
    Pop,
    Roll,
}

/// §7.10.1's `Range` for a program a device evaluates: the bounds of each output component,
/// and the component count with it.
///
/// One type rather than a slice plus a count, so the two cannot disagree — and only the two
/// shapes a device can turn into a colour without a colour-management step. The producer
/// ([`crate::DeferredColours`]) has no such restriction and draws every other space; this is
/// the door the *device* path fits through, and a shading that does not fit it takes the
/// producer's answer instead. `pdf_model::shading` is where that door is, with the conditions.
///
/// **The bounds are already intersected with `[0, 1]`.** §7.10.1 clips a function's outputs to
/// its `Range`, and this tree then converts the clipped components to a device colour, where
/// `DeviceGray` and `DeviceRGB` clamp each component to the unit interval. Clamping into
/// `[lo, hi]` and then into `[0, 1]` is clamping into the intersection, so handing a device the
/// intersection is the *same* arithmetic rather than an approximation of it — and it means a
/// document that declares a wider range, which Table 78 explicitly allows for, does not need a
/// second adjustment on the device side that nothing would state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProgramRange {
    /// `DeviceGray`: one component, replicated into three by whoever draws it.
    Gray([f32; 2]),
    /// `DeviceRGB`: three components, in order.
    Rgb([[f32; 2]; 3]),
}

impl ProgramRange {
    /// The `[min, max]` bounds in component order — one for [`Gray`](ProgramRange::Gray),
    /// three for [`Rgb`](ProgramRange::Rgb).
    #[must_use]
    pub fn bounds(&self) -> &[[f32; 2]] {
        match self {
            Self::Gray(pair) => std::slice::from_ref(pair),
            Self::Rgb(pairs) => pairs,
        }
    }

    /// How many components the program leaves on the operand stack.
    #[must_use]
    pub fn components(&self) -> usize {
        self.bounds().len()
    }
}

/// A §8.7.4.5.2 type 1 shading's colours, stated as the program that computes them.
///
/// Carried beside — never instead of — the [`crate::DeferredColours`] that answers the same
/// question by evaluating on the processor. A backend that can hand this to its graphics
/// library does; every other backend, and this one when the device declines the program,
/// draws the producer's grid, and the two are the same picture within the difference of
/// colour ADR 0376 bounds.
///
/// The `Arc` is the identity a backend's resource cache keys on, exactly as
/// [`crate::DeferredImage`]'s is: a program uploaded once and re-used across frames keeps the
/// device's compiled shader, and a program uploaded afresh every frame would throw it away.
#[derive(Debug, Clone)]
pub struct ShadingProgram {
    steps: Arc<[ProgramStep]>,
    range: ProgramRange,
}

impl ShadingProgram {
    /// A program with its output range.
    #[must_use]
    pub fn new(steps: Arc<[ProgramStep]>, range: ProgramRange) -> Self {
        Self { steps, range }
    }

    /// The flat instruction list.
    #[must_use]
    pub fn steps(&self) -> &Arc<[ProgramStep]> {
        &self.steps
    }

    /// §7.10.1's `Range`, and the component count with it.
    #[must_use]
    pub fn range(&self) -> ProgramRange {
        self.range
    }
}

impl PartialEq for ShadingProgram {
    /// Two programs are the same one when they are the same object and state the same range.
    ///
    /// [`crate::DeferredColours`]'s argument, unchanged: the display list's own `PartialEq`
    /// exists so a test can say a list was rebuilt unchanged, and comparing two thousand
    /// instructions to answer that would cost more than the question is worth. The range is
    /// compared by value because it is four floats and because a `with_alpha` copy shares the
    /// steps.
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.steps, &other.steps) && self.range == other.range
    }
}
