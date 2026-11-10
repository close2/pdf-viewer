//! A Compact Font Format program with every charstring stated again, each x coordinate scaled:
//! the shape this reader draws a fitted face in, written into a CFF face (`doc/adr/2789`).
//!
//! [`crate::stretch::sfnt_outlines`] does this for a `glyf` program. A CFF program states its
//! outlines as Type 2 charstrings (Adobe Technical Note #5177), whose operands are relative
//! moves, and a subroutine may be shared by glyphs that push the operands it reads — so an
//! operand inside a subroutine cannot be said to be an x or a y on its own. **Each charstring is
//! therefore run as the interpreter runs it** (section 4.7's calls included, as ADR 2779's
//! `BodyWalk` runs them), every point it reaches is taken absolutely, and the charstring is
//! stated afresh with no subroutine calls: each point's x multiplied by the scale and rounded to
//! a whole unit, each y exactly as it was. The deltas the new charstring states are between the
//! rounded points, so the rounding never accumulates along a contour.
//!
//! **Why whole units, though a charstring could state 16.16** (section 3.2): this tree's outline
//! reader, `skrifa` at no pixel size, draws a CFF coordinate as its whole part, as a font-unit
//! outline does, so a fraction would be read as its floor — up to a unit off — where a whole
//! number is read as written, within half a unit of the window's. **A line whose two ends round
//! to one point is not stated**: two points a unit apart, 158 and 159 at a scale of 0.82, both
//! become 130, and a reader drops a line of no length, so the program states none. Its read-back
//! skips the same line and nothing else.

//! What else is an x is scaled the same way: `vstem` and `vstemhm` edges (section 4.3's hints;
//! an edge hint's width, −20 or −21, is a flag section 2 reserves rather than a distance, and is
//! kept), and `endchar`'s accent offset `adx` (section 4.2's four optional arguments). The width operand is copied: ISO
//! 32000-2 §9.2.4 positions by the dictionary's widths and `crate::restate` makes the program's
//! agree afterwards, exactly as for a face embedded unscaled. Every mask byte is copied, since
//! the stems it selects are the same stems in the same order.
//!
//! The four flex operators are stated as `flex` with the depth they carried (section 4.1's
//! `hflex`, `hflex1` and `flex1` are used only where the depth is 50), and not as two curves: the
//! depth decides whether a renderer draws the pair as curves or as a line, so it is kept. One
//! form is the clear construction, and the rounding can break the equal y values a short form
//! encodes.
//!
//! **What a run cannot keep exactly is refused, by name**: an escaped operator other than the
//! four flex operators (an arithmetic or storage operator changes the stack by a rule this does
//! not follow), a reserved operator, a call naming no subroutine, nesting past Appendix B's ten
//! levels, or a run past [`super::CHARSTRING_OPERATIONS`].

use skrifa::GlyphId;

use super::stack_clearing::{
    CNTRMASK, ENDCHAR, HINTMASK, HMOVETO, HSTEM, HSTEMHM, RMOVETO, VMOVETO, VSTEM, VSTEMHM,
};
use super::{
    CALL_GLOBAL, CALL_LOCAL, CHARSTRING_OPERATIONS, CffError, ESCAPE_BYTE, Layout, OP_CHARSTRINGS,
    RETURN, Subroutines, bias, dict_int, global_subroutines, index_item, index_items,
    local_subroutines, open,
};

/// `rlineto`.
const RLINETO: u8 = 5;
/// `hlineto`.
const HLINETO: u8 = 6;
/// `vlineto`.
const VLINETO: u8 = 7;
/// `rrcurveto`.
const RRCURVETO: u8 = 8;
/// `rcurveline`.
const RCURVELINE: u8 = 24;
/// `rlinecurve`.
const RLINECURVE: u8 = 25;
/// `vvcurveto`.
const VVCURVETO: u8 = 26;
/// `hhcurveto`.
const HHCURVETO: u8 = 27;
/// `vhcurveto`.
const VHCURVETO: u8 = 30;
/// `hvcurveto`.
const HVCURVETO: u8 = 31;
/// The second byte of `dotsection`, which Appendix C deprecates: it draws nothing and is dropped.
const DOTSECTION: u8 = 0;
/// The second bytes of `hflex`, `flex`, `hflex1` and `flex1`.
const HFLEX: u8 = 34;
/// See [`HFLEX`].
const FLEX: u8 = 35;
/// See [`HFLEX`].
const HFLEX1: u8 = 36;
/// See [`HFLEX`].
const FLEX1: u8 = 37;
/// The flex depth section 4.1 gives the three short flex forms, in hundredths of a device pixel.
const FLEX_DEPTH: f64 = 50.0;
/// One unit in 16.16 fixed, the finest number a charstring states (section 3.2).
const FIXED_ONE: f64 = 65536.0;
/// Appendix B's argument stack: the most operands one operator may be given.
const MOST_OPERANDS: usize = 48;
/// Appendix B's subroutine nesting limit.
const MOST_NESTING: usize = 10;

/// The program, a bare CFF font, with every glyph's x coordinates multiplied by `scale` about the
/// glyph's origin, its charstrings stated without subroutine calls.
///
/// # Errors
///
/// [`CffError::Malformed`] where the program does not open or its `CharStrings` INDEX does not
/// read, and [`CffError::AdvanceNotRestatable`] naming the first glyph whose charstring a run
/// cannot state again (the module comment lists why).
pub fn with_outlines_scaled(data: &[u8], scale: f64) -> Result<Vec<u8>, CffError> {
    let font = open(data)?;
    let layout = Layout::read(data).ok_or_else(|| CffError::Malformed {
        detail: "the Top DICT INDEX could not be located".to_owned(),
    })?;
    if let Some(misstated) = layout.misstated {
        return Err(misstated.refusal());
    }
    let charstrings_at = layout
        .entries
        .iter()
        .find(|(_, op)| *op == OP_CHARSTRINGS)
        .and_then(|(operands, _)| dict_int(operands))
        .and_then(|at| usize::try_from(at).ok())
        .ok_or_else(|| CffError::Malformed {
            detail: "the Top DICT states no CharStrings offset".to_owned(),
        })?;
    let charstrings = index_items(data, charstrings_at).ok_or_else(|| CffError::Malformed {
        detail: "the CharStrings INDEX could not be read".to_owned(),
    })?;
    let cid_keyed = font.is_cid();
    let global = global_subroutines(data);
    // One Local Subr INDEX per Font DICT: a charstring's `callsubr` reaches the subroutines of
    // the Private DICT it is interpreted against and no other.
    let dicts: Vec<Vec<u8>> = if cid_keyed {
        index_items(data, layout.fd_array_at.unwrap_or(0))
            .unwrap_or_default()
            .into_iter()
            .map(<[u8]>::to_vec)
            .collect()
    } else {
        vec![
            index_item(data, layout.top_index.start, 0)
                .unwrap_or_default()
                .to_vec(),
        ]
    };
    let locals: Vec<Vec<Vec<u8>>> = dicts
        .iter()
        .map(|dict| local_subroutines(data, dict))
        .collect();
    let mut scaled = Vec::with_capacity(charstrings.len());
    for (index, charstring) in charstrings.iter().enumerate() {
        let glyph = u16::try_from(index).map_err(|_| CffError::Malformed {
            detail: "the CharStrings INDEX holds more glyphs than a glyph index names".to_owned(),
        })?;
        let subfont = if cid_keyed {
            font.subfont_index(GlyphId::from(glyph)).unwrap_or(0)
        } else {
            0
        };
        let subrs = Subroutines {
            global: &global,
            local: locals
                .get(usize::from(subfont))
                .map_or(&[][..], Vec::as_slice),
        };
        let restated = Restatement::new(&subrs, scale)
            .run_glyph(charstring)
            .map_err(|refusal| CffError::AdvanceNotRestatable {
                glyph,
                detail: format!(
                    "its outline could not be stated again at the horizontal scale {scale}: \
                     {refusal}"
                ),
            })?;
        scaled.push(restated);
    }
    Ok(layout.with_charstrings(data, &scaled))
}

/// Why one charstring could not be stated again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Refusal {
    /// An operator Appendix A reserves, or one this run does not follow.
    Operator(u8, Option<u8>),
    /// A number runs past the end of the code holding it, or a mask past its data.
    Truncated,
    /// An operator found fewer operands than it takes, or more than Appendix B allows.
    Operands,
    /// A call names no subroutine, or nests past Appendix B's ten levels.
    Call,
    /// The run passed [`CHARSTRING_OPERATIONS`].
    Budget,
    /// A scaled number does not fit the 16.16 form a charstring states numbers in.
    Range,
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Operator(b0, None) => {
                write!(f, "it reaches the operator {b0}, which is reserved")
            }
            Self::Operator(b0, Some(b1)) => write!(
                f,
                "it reaches the escaped operator {b0} {b1}, which changes the stack by a rule \
                 this run does not follow"
            ),
            Self::Truncated => f.write_str("a number or a mask runs past the end of its code"),
            Self::Operands => {
                f.write_str("an operator is given a number of operands it does not take")
            }
            Self::Call => f.write_str("a call names no subroutine or nests past ten levels"),
            Self::Budget => f.write_str("its run passes the bound on charstring operations"),
            Self::Range => f.write_str("a scaled number does not fit a charstring's 16.16 form"),
        }
    }
}

/// How a run of one code ended.
enum Ended {
    /// It reached a `return`, or the end of its bytes.
    Returned,
    /// It reached `endchar`, which ends the glyph wherever it is run.
    Glyph,
}

/// One glyph's charstring being run and stated again.
struct Restatement<'s, 'a> {
    /// The subroutines its calls reach.
    subrs: &'s Subroutines<'a>,
    /// The horizontal scale.
    scale: f64,
    /// The argument stack, every number as the value it states.
    stack: Vec<f64>,
    /// Whether the first stack-clearing operator, the one a width goes with, has been met.
    cleared: bool,
    /// Stem hints declared so far, which decide how many bytes a mask carries.
    stems: usize,
    /// The current point, in the program's units, as the source charstring reaches it.
    at: (f64, f64),
    /// The current point as the new charstring states it: x scaled and rounded, y as it was.
    stated: (f64, f64),
    /// The new charstring.
    out: Vec<u8>,
    /// Operators left before [`Refusal::Budget`].
    operations: u32,
}

impl<'s, 'a> Restatement<'s, 'a> {
    const fn new(subrs: &'s Subroutines<'a>, scale: f64) -> Self {
        Self {
            subrs,
            scale,
            stack: Vec::new(),
            cleared: false,
            stems: 0,
            at: (0.0, 0.0),
            stated: (0.0, 0.0),
            out: Vec::new(),
            operations: CHARSTRING_OPERATIONS,
        }
    }

    /// The glyph's new charstring, ending in the `endchar` section 3.1 requires of every one.
    fn run_glyph(mut self, charstring: &[u8]) -> Result<Vec<u8>, Refusal> {
        match self.run(charstring, 0)? {
            Ended::Glyph => {}
            Ended::Returned => {
                if !self.stack.is_empty() {
                    return Err(Refusal::Operands);
                }
                self.out.push(ENDCHAR);
            }
        }
        Ok(self.out)
    }

    /// An x as the new charstring states it: scaled about the origin and rounded to a whole
    /// unit.
    fn x(&self, x: f64) -> f64 {
        (x * self.scale).round()
    }

    /// Runs `code` at nesting `depth`.
    fn run(&mut self, code: &[u8], depth: usize) -> Result<Ended, Refusal> {
        let mut at = 0usize;
        while let Some(&b0) = code.get(at) {
            self.operations = self.operations.checked_sub(1).ok_or(Refusal::Budget)?;
            let step = match b0 {
                28 | 32..=255 => {
                    let (value, width) = operand(code, at)?;
                    self.push(value)?;
                    width
                }
                CALL_LOCAL | CALL_GLOBAL => {
                    if let Ended::Glyph = self.call(b0 == CALL_LOCAL, depth)? {
                        return Ok(Ended::Glyph);
                    }
                    1
                }
                RETURN => return Ok(Ended::Returned),
                ENDCHAR => {
                    self.endchar()?;
                    return Ok(Ended::Glyph);
                }
                HSTEM | HSTEMHM => {
                    self.width_if(self.stack.len() % 2 == 1)?;
                    self.horizontal_stems(b0)?;
                    1
                }
                VSTEM | VSTEMHM => {
                    self.width_if(self.stack.len() % 2 == 1)?;
                    self.vertical_stems(b0)?;
                    1
                }
                HINTMASK | CNTRMASK => {
                    self.width_if(self.stack.len() % 2 == 1)?;
                    // Operands before a mask are an implicit `vstem` (section 4.3).
                    if !self.stack.is_empty() {
                        self.vertical_stems(VSTEMHM)?;
                    }
                    let bytes = self.stems.div_ceil(8);
                    let mask = code
                        .get(at.saturating_add(1)..at.saturating_add(1).saturating_add(bytes))
                        .ok_or(Refusal::Truncated)?;
                    self.out.push(b0);
                    self.out.extend_from_slice(mask);
                    1usize.saturating_add(bytes)
                }
                RMOVETO => {
                    self.width_if(self.stack.len() > 2)?;
                    let [dx, dy] = self.exactly()?;
                    self.move_by(dx, dy)?;
                    1
                }
                HMOVETO => {
                    self.width_if(self.stack.len() > 1)?;
                    let [dx] = self.exactly()?;
                    self.move_by(dx, 0.0)?;
                    1
                }
                VMOVETO => {
                    self.width_if(self.stack.len() > 1)?;
                    let [dy] = self.exactly()?;
                    self.move_by(0.0, dy)?;
                    1
                }
                RLINETO | HLINETO | VLINETO | RRCURVETO | RCURVELINE | RLINECURVE | VVCURVETO
                | HHCURVETO | VHCURVETO | HVCURVETO => {
                    self.cleared = true;
                    self.path(b0)?;
                    1
                }
                ESCAPE_BYTE => {
                    let b1 = code
                        .get(at.saturating_add(1))
                        .copied()
                        .ok_or(Refusal::Truncated)?;
                    self.cleared = true;
                    match b1 {
                        DOTSECTION => self.stack.clear(),
                        HFLEX | FLEX | HFLEX1 | FLEX1 => self.flex(b1)?,
                        _ => return Err(Refusal::Operator(b0, Some(b1))),
                    }
                    2
                }
                _ => return Err(Refusal::Operator(b0, None)),
            };
            at = at.checked_add(step).ok_or(Refusal::Truncated)?;
            if at > code.len() {
                return Err(Refusal::Truncated);
            }
        }
        Ok(Ended::Returned)
    }

    /// A subroutine call, its number popped and biased (section 4.7), run one level down.
    fn call(&mut self, local: bool, depth: usize) -> Result<Ended, Refusal> {
        let named = self.stack.pop().ok_or(Refusal::Operands)?;
        let table = if local {
            self.subrs.local
        } else {
            self.subrs.global
        };
        let subroutine = whole(named)
            .and_then(|named| named.checked_add(bias(table.len())))
            .and_then(|index| usize::try_from(index).ok())
            .and_then(|index| table.get(index))
            .ok_or(Refusal::Call)?;
        if depth >= MOST_NESTING {
            return Err(Refusal::Call);
        }
        self.run(subroutine, depth.saturating_add(1))
    }

    fn push(&mut self, value: f64) -> Result<(), Refusal> {
        if self.stack.len() >= MOST_OPERANDS {
            return Err(Refusal::Operands);
        }
        self.stack.push(value);
        Ok(())
    }

    /// The width, where this is the first stack-clearing operator and `present` says the stack
    /// holds one more operand than the operator takes: copied to the new charstring first, so it
    /// is the first operand of the first operator stated, as section 3.1 places it.
    fn width_if(&mut self, present: bool) -> Result<(), Refusal> {
        if self.cleared {
            return Ok(());
        }
        self.cleared = true;
        if present {
            let width = self.stack.remove(0);
            put_number(&mut self.out, width)?;
        }
        Ok(())
    }

    /// The stack's operands, which must be exactly `N`, taken.
    fn exactly<const N: usize>(&mut self) -> Result<[f64; N], Refusal> {
        let operands: [f64; N] = self
            .stack
            .as_slice()
            .try_into()
            .map_err(|_| Refusal::Operands)?;
        self.stack.clear();
        Ok(operands)
    }

    /// `hstem` or `hstemhm`: y edges, copied as they are.
    fn horizontal_stems(&mut self, op: u8) -> Result<(), Refusal> {
        let operands = std::mem::take(&mut self.stack);
        if !operands.len().is_multiple_of(2) {
            return Err(Refusal::Operands);
        }
        self.stems = self.stems.saturating_add(operands.len() / 2);
        for value in operands {
            put_number(&mut self.out, value)?;
        }
        self.out.push(op);
        Ok(())
    }

    /// `vstem` or `vstemhm`: x edges, each pair's edges scaled where they stand absolutely and
    /// stated relative again (section 4.3: the first edge from 0, each later one from the edge
    /// before it). A width of −20 or −21 is not a distance: section 2 reserves them for an
    /// edge hint's right and left edge, so the edge is scaled and the flag kept.
    fn vertical_stems(&mut self, op: u8) -> Result<(), Refusal> {
        let operands = std::mem::take(&mut self.stack);
        if !operands.len().is_multiple_of(2) {
            return Err(Refusal::Operands);
        }
        self.stems = self.stems.saturating_add(operands.len() / 2);
        let (mut edge, mut stated_edge) = (0.0_f64, 0.0_f64);
        for pair in operands.chunks_exact(2) {
            let (lower, upper) = (edge + pair[0], edge + pair[0] + pair[1]);
            let stated_lower = self.x(lower);
            let stated_upper = if matches!(whole(pair[1]), Some(-20 | -21)) {
                stated_lower + pair[1]
            } else {
                self.x(upper)
            };
            put_number(&mut self.out, stated_lower - stated_edge)?;
            put_number(&mut self.out, stated_upper - stated_lower)?;
            (edge, stated_edge) = (upper, stated_upper);
        }
        self.out.push(op);
        Ok(())
    }

    /// `endchar`, with its accent's four operands where it has them (section 4.2): `adx` is an
    /// x offset and is scaled, the rest copied.
    fn endchar(&mut self) -> Result<(), Refusal> {
        self.width_if(self.stack.len() == 1 || self.stack.len() == 5)?;
        let operands = std::mem::take(&mut self.stack);
        match operands.as_slice() {
            [] => {}
            [adx, ady, bchar, achar] => {
                let adx = self.x(*adx);
                put_number(&mut self.out, adx)?;
                for value in [*ady, *bchar, *achar] {
                    put_number(&mut self.out, value)?;
                }
            }
            _ => return Err(Refusal::Operands),
        }
        self.out.push(ENDCHAR);
        Ok(())
    }

    /// A move by `(dx, dy)` from the current point, stated as an `rmoveto`.
    fn move_by(&mut self, dx: f64, dy: f64) -> Result<(), Refusal> {
        let point = (self.at.0 + dx, self.at.1 + dy);
        let stated = self.stated_point(point);
        self.at = point;
        put_number(&mut self.out, stated.0 - self.stated.0)?;
        put_number(&mut self.out, stated.1 - self.stated.1)?;
        self.stated = stated;
        self.out.push(RMOVETO);
        Ok(())
    }

    /// A point as the new charstring states it.
    fn stated_point(&self, (x, y): (f64, f64)) -> (f64, f64) {
        (self.x(x), y)
    }

    /// A line to the point `(dx, dy)` from the current one, stated as an `rlineto`.
    fn line_by(&mut self, dx: f64, dy: f64) -> Result<(), Refusal> {
        let point = (self.at.0 + dx, self.at.1 + dy);
        let stated = self.stated_point(point);
        self.at = point;
        // A line of no length draws nothing and a reader drops it; see the module comment.
        if stated == self.stated {
            return Ok(());
        }
        put_number(&mut self.out, stated.0 - self.stated.0)?;
        put_number(&mut self.out, stated.1 - self.stated.1)?;
        self.out.push(RLINETO);
        self.stated = stated;
        Ok(())
    }

    /// A curve through three points each given relative to the one before, stated as an
    /// `rrcurveto`.
    fn curve_by(&mut self, deltas: [f64; 6]) -> Result<(), Refusal> {
        for (dx, dy) in [(0, 1), (2, 3), (4, 5)].map(|(x, y)| (deltas[x], deltas[y])) {
            let point = (self.at.0 + dx, self.at.1 + dy);
            let stated = self.stated_point(point);
            put_number(&mut self.out, stated.0 - self.stated.0)?;
            put_number(&mut self.out, stated.1 - self.stated.1)?;
            self.at = point;
            self.stated = stated;
        }
        self.out.push(RRCURVETO);
        Ok(())
    }

    /// The line and curve operators, each segment they draw stated as `rlineto` or `rrcurveto`
    /// (section 4.1's operand patterns).
    fn path(&mut self, op: u8) -> Result<(), Refusal> {
        let operands = std::mem::take(&mut self.stack);
        let ops = operands.as_slice();
        match op {
            RLINETO => {
                if ops.is_empty() || !ops.len().is_multiple_of(2) {
                    return Err(Refusal::Operands);
                }
                for pair in ops.chunks_exact(2) {
                    self.line_by(pair[0], pair[1])?;
                }
            }
            HLINETO | VLINETO => {
                if ops.is_empty() {
                    return Err(Refusal::Operands);
                }
                let mut horizontal = op == HLINETO;
                for &delta in ops {
                    if horizontal {
                        self.line_by(delta, 0.0)?;
                    } else {
                        self.line_by(0.0, delta)?;
                    }
                    horizontal = !horizontal;
                }
            }
            RRCURVETO => {
                if ops.is_empty() || !ops.len().is_multiple_of(6) {
                    return Err(Refusal::Operands);
                }
                for six in ops.chunks_exact(6) {
                    self.curve_by(sextuple(six))?;
                }
            }
            RCURVELINE => {
                if ops.len() < 8 || !ops.len().saturating_sub(2).is_multiple_of(6) {
                    return Err(Refusal::Operands);
                }
                let (curves, line) = ops.split_at(ops.len().saturating_sub(2));
                for six in curves.chunks_exact(6) {
                    self.curve_by(sextuple(six))?;
                }
                self.line_by(line[0], line[1])?;
            }
            RLINECURVE => {
                if ops.len() < 8 || !ops.len().saturating_sub(6).is_multiple_of(2) {
                    return Err(Refusal::Operands);
                }
                let (lines, curve) = ops.split_at(ops.len().saturating_sub(6));
                for pair in lines.chunks_exact(2) {
                    self.line_by(pair[0], pair[1])?;
                }
                self.curve_by(sextuple(curve))?;
            }
            VVCURVETO | HHCURVETO => {
                let (first, rest) = if ops.len() % 4 == 1 {
                    (ops[0], &ops[1..])
                } else {
                    (0.0, ops)
                };
                if rest.is_empty() || !rest.len().is_multiple_of(4) {
                    return Err(Refusal::Operands);
                }
                for (index, four) in rest.chunks_exact(4).enumerate() {
                    let lead = if index == 0 { first } else { 0.0 };
                    let deltas = if op == VVCURVETO {
                        [lead, four[0], four[1], four[2], 0.0, four[3]]
                    } else {
                        [four[0], lead, four[1], four[2], four[3], 0.0]
                    };
                    self.curve_by(deltas)?;
                }
            }
            _ => self.alternating_curves(op == HVCURVETO, ops)?,
        }
        Ok(())
    }

    /// `hvcurveto` and `vhcurveto`: curves whose tangents alternate between horizontal and
    /// vertical, four operands each, the last curve taking a fifth for its final other-axis
    /// delta (section 4.1).
    fn alternating_curves(&mut self, starts_horizontal: bool, ops: &[f64]) -> Result<(), Refusal> {
        let (curves, last) = match ops.len() % 4 {
            0 => (ops, None),
            1 => (&ops[..ops.len().saturating_sub(1)], ops.last().copied()),
            _ => return Err(Refusal::Operands),
        };
        if curves.is_empty() {
            return Err(Refusal::Operands);
        }
        let count = curves.len() / 4;
        let mut horizontal = starts_horizontal;
        for (index, four) in curves.chunks_exact(4).enumerate() {
            let tail = if index.saturating_add(1) == count {
                last.unwrap_or(0.0)
            } else {
                0.0
            };
            let deltas = if horizontal {
                [four[0], 0.0, four[1], four[2], tail, four[3]]
            } else {
                [0.0, four[0], four[1], four[2], four[3], tail]
            };
            self.curve_by(deltas)?;
            horizontal = !horizontal;
        }
        Ok(())
    }

    /// The four flex operators, each stated as `flex` (12 35) with its six points' deltas and its
    /// depth (section 4.1).
    fn flex(&mut self, op: u8) -> Result<(), Refusal> {
        let operands = std::mem::take(&mut self.stack);
        let ops = operands.as_slice();
        let (deltas, depth): ([f64; 12], f64) = match (op, ops) {
            (FLEX, [d @ .., fd]) if d.len() == 12 => {
                (d.try_into().map_err(|_| Refusal::Operands)?, *fd)
            }
            (HFLEX, &[dx1, dx2, dy2, dx3, dx4, dx5, dx6]) => (
                [dx1, 0.0, dx2, dy2, dx3, 0.0, dx4, 0.0, dx5, -dy2, dx6, 0.0],
                FLEX_DEPTH,
            ),
            (HFLEX1, &[dx1, dy1, dx2, dy2, dx3, dx4, dx5, dy5, dx6]) => (
                [
                    dx1,
                    dy1,
                    dx2,
                    dy2,
                    dx3,
                    0.0,
                    dx4,
                    0.0,
                    dx5,
                    dy5,
                    dx6,
                    -(dy1 + dy2 + dy5),
                ],
                FLEX_DEPTH,
            ),
            (FLEX1, &[dx1, dy1, dx2, dy2, dx3, dy3, dx4, dy4, dx5, dy5, d6]) => {
                let dx = dx1 + dx2 + dx3 + dx4 + dx5;
                let dy = dy1 + dy2 + dy3 + dy4 + dy5;
                let last = if dx.abs() > dy.abs() {
                    (d6, -dy)
                } else {
                    (-dx, d6)
                };
                (
                    [
                        dx1, dy1, dx2, dy2, dx3, dy3, dx4, dy4, dx5, dy5, last.0, last.1,
                    ],
                    FLEX_DEPTH,
                )
            }
            _ => return Err(Refusal::Operands),
        };
        for (dx, dy) in deltas.chunks_exact(2).map(|pair| (pair[0], pair[1])) {
            let point = (self.at.0 + dx, self.at.1 + dy);
            let stated = self.stated_point(point);
            put_number(&mut self.out, stated.0 - self.stated.0)?;
            put_number(&mut self.out, stated.1 - self.stated.1)?;
            self.at = point;
            self.stated = stated;
        }
        put_number(&mut self.out, depth)?;
        self.out.extend_from_slice(&[ESCAPE_BYTE, FLEX]);
        Ok(())
    }
}

/// The number a charstring states at `at`, and how many bytes state it (section 3.2).
fn operand(code: &[u8], at: usize) -> Result<(f64, usize), Refusal> {
    let byte = |offset: usize| -> Result<u8, Refusal> {
        code.get(at.saturating_add(offset))
            .copied()
            .ok_or(Refusal::Truncated)
    };
    match byte(0)? {
        28 => Ok((f64::from(i16::from_be_bytes([byte(1)?, byte(2)?])), 3)),
        255 => {
            let fixed = i32::from_be_bytes([byte(1)?, byte(2)?, byte(3)?, byte(4)?]);
            Ok((f64::from(fixed) / FIXED_ONE, 5))
        }
        b0 @ 32..=246 => Ok((f64::from(i16::from(b0).saturating_sub(139)), 1)),
        b0 => {
            // 247 to 254: two bytes, whose reading `small_operand` already states.
            let (value, width) = super::small_operand(b0, byte(1)?);
            let value = i32::try_from(value).map_err(|_| Refusal::Range)?;
            Ok((f64::from(value), width))
        }
    }
}

/// Six operands as an array; the callers have checked the length.
fn sextuple(six: &[f64]) -> [f64; 6] {
    let mut out = [0.0; 6];
    out.copy_from_slice(six);
    out
}

/// A number that is a whole one, as an integer.
fn whole(value: f64) -> Option<i64> {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a whole number within ±32768, which an i64 holds exactly"
    )]
    (value.fract() == 0.0 && value.abs() <= 32768.0).then_some(value as i64)
}

/// A number in the shortest form section 3.2 gives it: an integer in one, two or three bytes, any
/// other value as 16.16 fixed.
fn put_number(out: &mut Vec<u8>, value: f64) -> Result<(), Refusal> {
    if let Some(integer) = whole(value).and_then(|integer| i16::try_from(integer).ok()) {
        match integer {
            -107..=107 => {
                out.push(u8::try_from(integer.saturating_add(139)).map_err(|_| Refusal::Range)?);
            }
            108..=1131 => {
                let rest = integer.saturating_sub(108);
                out.push(
                    u8::try_from((rest / 256).saturating_add(247)).map_err(|_| Refusal::Range)?,
                );
                out.push(u8::try_from(rest % 256).map_err(|_| Refusal::Range)?);
            }
            -1131..=-108 => {
                let rest = integer.saturating_neg().saturating_sub(108);
                out.push(
                    u8::try_from((rest / 256).saturating_add(251)).map_err(|_| Refusal::Range)?,
                );
                out.push(u8::try_from(rest % 256).map_err(|_| Refusal::Range)?);
            }
            _ => {
                out.push(28);
                out.extend_from_slice(&integer.to_be_bytes());
            }
        }
        return Ok(());
    }
    let fixed = (value * FIXED_ONE).round();
    if !(f64::from(i32::MIN)..=f64::from(i32::MAX)).contains(&fixed) {
        return Err(Refusal::Range);
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "in range by the check above, and a whole number by the rounding"
    )]
    let fixed = fixed as i32;
    out.push(255);
    out.extend_from_slice(&fixed.to_be_bytes());
    Ok(())
}

#[cfg(test)]
#[path = "cff_stretch_tests.rs"]
mod tests;
