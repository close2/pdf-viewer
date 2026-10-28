//! §10.4.2.4 and the inverse of a press: the inks a screen colour needs, searched on a grid and polished.

use pdf_render::Color;
use rayon::iter::{IntoParallelIterator as _, ParallelIterator as _};

use super::cie::{srgb_to_xyz_d50, xyz_d50_to_srgb};
use super::device_cmyk::channel;
use super::press::{Press, PressIdentity};

/// ISO 32000-2 §10.4.2.4's conversion from `DeviceRGB` to `DeviceCMYK`.
///
/// The clause's own two steps, with the nominal black-generation and undercolour-removal
/// functions — `BG(k) = k` and `UCR(k) = k` — which is what [`super::ColourSpace::to_cmyk`]'s doc
/// comment records as this device's defaults. The formula, set out here rather than quoted
/// because the standard prints it in mathematical italics no transcription survives:
/// `c = 1 − red`, `m = 1 − green`, `y = 1 − blue`, `k = min(c, m, y)`, and then
/// `cyan = min(1, max(0, c − UCR(k)))` with magenta, yellow and `black = min(1, max(0, BG(k)))`
/// alongside. The clamps are the clause's: "\[i]f a value falls outside this range, the nearest
/// valid value shall be substituted automatically without error indication."
///
/// With the nominal functions the result is a *right inverse* of §10.4.2.5's classic
/// conversion back — `1 − min(1, cyan + black)` is `1 − min(1, c)`, which is `red` — so the
/// standard's own pair round-trips exactly. It is **not** a right inverse of this tree's ink
/// cube (ADR 0009), which is why §10.4.2's branch cannot be composed with §10.3's; ADR 0262
/// has the picture of what that costs. What it is used for here is [`rgb_to_ink`]'s starting
/// point and its black generation, because the nominal `k` this states is the one §10.4.2.4
/// defines and the one a `/BG` function would be called with.
pub(super) fn rgb_to_cmyk(colour: Color) -> [f32; 4] {
    let cyan = 1.0 - channel(colour.r);
    let magenta = 1.0 - channel(colour.g);
    let yellow = 1.0 - channel(colour.b);
    let black = cyan.min(magenta).min(yellow);
    [
        channel(cyan - black),
        channel(magenta - black),
        channel(yellow - black),
        channel(black),
    ]
}

/// How close a separation has to come before it counts as reproducing its colour.
///
/// Half a level of an eight-bit channel, which is below what the raster the result is written
/// into can hold: a residual under this cannot reach a pixel, so continuing to iterate would
/// buy a number nobody can see.
const INK_EXACT: f32 = 0.5 / 255.0;

/// How many black generations [`rgb_to_ink`] tries after §10.4.2.4's nominal one.
///
/// Twelve, from all the black there is down to none. The nominal generation is tried *first*
/// and answers most colours on its own: a neutral, and every colour muted enough that the
/// nominal black leaves the other three inks somewhere to go, is reproduced at that one slice.
/// Measured over 2000 document-like colours — 45% neutral, 40% muted, 15% saturated — the
/// whole construction costs **4.67** slices and **9.8** Gauss–Newton steps per distinct
/// colour, and reproduces 90.3% of them to under half a level with a median of 0.074.
///
/// The ladder reaches *above* the nominal black as well as below it, and that is not
/// symmetry for its own sake: this press's black ink is `#231F20` rather than `#000000`, so a
/// very dark saturated colour needs **more** black than §10.4.2.4's nominal `k` — which the
/// clause allows in as many words, a black-generation function being free to "return a larger
/// value for extra black".
const INK_LADDER: usize = 12;

/// How many Gauss–Newton steps one slice is allowed.
const INK_STEPS: usize = 12;

/// How many times a Gauss–Newton step is halved before a slice gives up.
///
/// The step is exact for an affine map and the ink cube is not one, so a full step can
/// overshoot; halving until the squared distance falls is what makes the iteration monotone,
/// and a slice that cannot improve at all has converged or is against the edge of the unit
/// cube.
const INK_BACKTRACKS: usize = 6;

/// The side of the grid [`Press::table`] holds, over sRGB.
///
/// 17 × 17 × 17 separations, 78 KB, and the sRGB cube's own corners are grid points — so
/// paper and `#000000` are looked up rather than interpolated. Chosen by measurement: with
/// [`INK_POLISH`] after it, the worst gap over 800 random colours of the cube's own image is
/// **0.50 of 255**, which is `INK_EXACT` itself — the table's answer *is* the search's.
pub(super) const INK_TABLE_SIDE: usize = 17;

/// How many Gauss–Newton steps may land the table's answer on the exact separation.
///
/// The loop stops as soon as the colour is reproduced, so away from the gamut's boundary this
/// costs one step and often none at all. The cases that use the rest are the boundary itself,
/// where the step is against the edge of the unit cube and has to be taken in pieces.
const INK_POLISH: usize = 6;

/// `search_ink` over a grid of sRGB, built once and only where a page asks for it.
///
/// The map is a pure function of `super::device_cmyk::CMYK_CORNERS`, which is a compile-time constant, so this
/// could in principle be generated data. It is a `OnceLock` instead because it is wanted by
/// **0.6% of the pdf.js corpus and 3.5% of the documents `SafeDocs` samples from the web** — the
/// pages §11.4.7 composites in ink — and `CLAUDE.md`'s launch rule is that anything not needed
/// to show page one is deferred until first use. Building it costs 4913 searches, measured at
/// **7.5–10.0 ms** across this machine's 24 threads. What it buys is the difference between
/// 791 ns and 12.5 µs per distinct colour on a page made of them: over the 61 web witnesses of
/// `doc/todo/23`, searching every colour added **37.3 s** to their page-one renders where the
/// table adds **1.8 s**.
///
/// # Why the table is built *outside* the initialiser
///
/// `OnceLock::get_or_init` blocks every other caller until the closure returns, and rayon's
/// `collect` **runs other jobs while it waits** — so a closure holding the lock and calling
/// rayon can be handed a job that calls this function again, on the initialising thread, and
/// then waits for itself. That is a deadlock rather than a slow path: the whole process stops
/// with every thread parked. ADR 0269 found it in three of 145 `SafeDocs` archives, and a
/// reduction of it — a rayon `collect` inside a
/// `OnceLock` initialiser, called from a parallel iterator — hung 10 runs out of 10.
///
/// So the grid is computed first and the lock is held only across a move. A second caller
/// that arrives while the first is still computing builds its own copy and throws it away,
/// which costs one grid and cannot wait on anything.
impl Press {
    /// [`search_ink`] over a grid of sRGB, built once and only where a page asks for it.
    fn table(&self) -> &[[f32; 4]] {
        if let Some(table) = self.table.get() {
            return table;
        }
        let built = build_ink_table(&self.space);
        self.table.get_or_init(|| built)
    }
}

/// [`search_ink`] over every point of the [`INK_TABLE_SIDE`] grid.
///
/// **Parallel off a rayon worker and serial on one**, which is the second half of
/// [`Press::table`]'s deadlock argument rather than a second opinion about speed. The parallelism
/// is there for the launch path — 61.7 ms serially against 7.5–10.0 ms across this machine's
/// 24 threads, on the way to page one — and the launch path is a host's own thread, never a
/// worker of the pool. Inside a worker, `collect` would run other jobs while it waits and this
/// function would nest once per job that wants the grid; the depth of that is a property of
/// the caller's work queue rather than of anything here, so the branch declines it. What it
/// costs is 61.7 ms on a thread that is already one of many, where the machine has no idle
/// core to spend anyway.
fn build_ink_table(space: &pdf_render::BlendingSpace) -> Vec<[f32; 4]> {
    let side = INK_TABLE_SIDE;
    #[expect(
        clippy::cast_precision_loss,
        reason = "a grid index below 17 and the side itself, both exact in f32"
    )]
    let at = |index: usize| index as f32 / side.saturating_sub(1) as f32;
    let entry = |index: usize| {
        let blue = index % side;
        let green = index / side % side;
        let red = index / side / side % side;
        search_ink(space, [at(red), at(green), at(blue)])
    };
    let points = 0..side.saturating_mul(side).saturating_mul(side);
    if rayon::current_thread_index().is_some() {
        points.map(entry).collect()
    } else {
        points.into_par_iter().map(entry).collect()
    }
}

/// ISO 32000-2 §11.7.2's conversion from sRGB *into* `DeviceCMYK`, on §10.3's branch.
///
/// # Why this is not §10.4.2.4
///
/// §10.4.2.1 states two branches and ranks them, and [`super::device_cmyk::CMYK_CORNERS`] puts this tree's
/// conversion *out* of `DeviceCMYK` on the higher one. §11.7.5.3 says the conversion *into* a
/// group's colour space is the same conversion as the one onto the device with a different
/// target, so it belongs on the same branch — and composing one branch with the other is not
/// the identity. Taken through §10.4.2.4 and back through [`super::device_cmyk::cmyk`], `1 0 0 rg` comes back
/// `#ED1C24` and `0 g` comes back `#231F20`: two marks moved by a conversion the clause asked
/// for only so that they could be composited, on pages where nothing composites them.
///
/// So this is a **right inverse of [`super::device_cmyk::cmyk`]**, which is what §10.3's branch means here — the
/// ink cube stands in for a press's profile (ADRs 0009, 0042), and converting into the space
/// it defines is asking which ink that press would lay down to make this colour. [`search_ink`]
/// is that question answered from nothing; this is it answered from [`Press::table`] and landed
/// by [`polish_four_inks`], which is the same answer 998 times in 1000 and **16 times faster**
/// on the colours a document is made of.
///
/// # Where the press has a profile with a `B2A`, the profile answers instead
///
/// A press sampled from a bi-directional profile converts in through [`xyz_to_ink`]: the
/// colour's D50 XYZ — for a device colour, the sRGB this processor takes device colours to be
/// (§10.3.2, ADR 0009), inverted by [`srgb_to_xyz_d50`] — through the profile's own "from
/// CIE" table. That is §10.3's branch as the standard states it rather than as this tree
/// stood in for it, and [`super::press::press_for_profile`] says why the search is the fallback and not the
/// rule.
pub(super) fn rgb_to_ink(press: &Press, colour: Color) -> [f32; 4] {
    if press.identity == PressIdentity::DeviceInk {
        return rgb_to_cmyk(colour);
    }
    if press.profile.is_some() {
        return xyz_to_ink(press, srgb_to_xyz_d50(colour));
    }
    let target = [channel(colour.r), channel(colour.g), channel(colour.b)];
    polish_four_inks(&press.space, target, ink_lookup(press, target))
}

/// The separation of a D50 XYZ, through the press's profile where it has a `B2A` and through
/// [`rgb_to_ink`]'s search on the sRGB it becomes where it has not.
///
/// The direct route is what a colour already stated in CIE terms wants — a `Lab`, a `CalRGB`,
/// another profile's colour — because §10.3.1 makes the conversion between two CIE-based
/// spaces the ICC specification's, from connection space to connection space, and sRGB's
/// gamut has no business standing between them. Black point compensation is undone by the
/// profile's own reading of it: `super::press::sample_press` takes the conversion out under
/// [`Press::rendering`], so the conversion in is asked under the same black point, and the two
/// stay inverses whichever way it is set.
pub(super) fn xyz_to_ink(press: &Press, xyz: [f32; 3]) -> [f32; 4] {
    if let Some(inks) = press
        .profile
        .as_deref()
        .and_then(|profile| profile.to_device(xyz, press.rendering))
    {
        return [
            channel(inks[0]),
            channel(inks[1]),
            channel(inks[2]),
            channel(inks[3]),
        ];
    }
    let colour = xyz_d50_to_srgb(xyz);
    let target = [channel(colour.r), channel(colour.g), channel(colour.b)];
    polish_four_inks(&press.space, target, ink_lookup(press, target))
}

/// [`Press::table`] read at `target`, trilinearly between the eight grid separations around it.
fn ink_lookup(press: &Press, target: [f32; 3]) -> [f32; 4] {
    let table = press.table();
    let side = INK_TABLE_SIDE;
    let last = side.saturating_sub(1);
    let mut base = [0usize; 3];
    let mut fraction = [0.0f32; 3];
    for (axis, (index, offset)) in base.iter_mut().zip(fraction.iter_mut()).enumerate() {
        #[expect(
            clippy::cast_precision_loss,
            reason = "the side is 17 and a cell index below it, both exact in f32"
        )]
        let scaled = target.get(axis).copied().unwrap_or(0.0) * last as f32;
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "`scaled` is in 0..=16 because `target` is a clamped channel"
        )]
        let cell = (scaled as usize).min(last.saturating_sub(1));
        *index = cell;
        #[expect(
            clippy::cast_precision_loss,
            reason = "a cell index below 17, exact in f32"
        )]
        let low = cell as f32;
        *offset = (scaled - low).clamp(0.0, 1.0);
    }

    let mut inks = [0.0f32; 4];
    for corner in 0..8usize {
        let mut weight = 1.0f32;
        let mut index = 0usize;
        for axis in 0..3usize {
            let high = corner >> axis & 1 == 1;
            let offset = fraction.get(axis).copied().unwrap_or(0.0);
            weight *= if high { offset } else { 1.0 - offset };
            let step = base
                .get(axis)
                .copied()
                .unwrap_or(0)
                .saturating_add(usize::from(high));
            index = index.saturating_mul(side).saturating_add(step.min(last));
        }
        let Some(entry) = table.get(index) else {
            continue;
        };
        for (component, held) in inks.iter_mut().zip(entry) {
            *component = weight.mul_add(*held, *component);
        }
    }
    inks
}

/// The separation of one sRGB colour, searched for rather than looked up.
///
/// # The construction, and where each part comes from
///
/// 1. **[`rgb_to_cmyk`] gives the nominal separation**, whose `k` is §10.4.2.4's own: "the
///    minimum of the intermediate c , m , and y values that have been computed by subtracting
///    the original red , green , and blue components from 1.0".
/// 2. **The three chromatic inks are solved** so that [`super::device_cmyk::cmyk`] reproduces the colour at that
///    black. This is undercolour removal computed rather than stated — §10.4.2.4 asks a `UCR`
///    function for "the amount to subtract from each of the intermediate c , m , and y values",
///    and at a known press the amount is not a guess.
/// 3. **Where no such three exist, another black generation is tried**, down [`INK_LADDER`]
///    from all the black there is to none, and the first that reproduces the colour is the
///    most black that does. The clause leaves this to the device — "\[t]he correct choice of
///    black-generation and undercolour-removal functions depends on the characteristics of the
///    output device. Each device shall be configured with default values that are appropriate
///    for that device" — and names the freedom exactly: a black-generation function "may
///    simply return its k operand unchanged, or it may return a larger value for extra black,
///    a smaller value for less black, or 0.0 for no black at all."
/// 4. **Where no black generation on the ladder reproduces it**, all four inks are moved
///    together from the closest rung — [`polish_four_inks`] — which closes a colour whose
///    feasible band of black falls *between* two rungs. Measured over the cube's own image on
///    a 6⁴ grid the worst colour this whole search still misses by is **3.12 of 255**, and
///    every one of them is a dark saturated ink at the gamut's own edge.
/// 5. **What is left is outside the press's gamut**, and the nearest ink the search found is
///    used. That is a *choice*, and the clause that makes it a choice rather than an error is
///    §11.7.5.3: the rendering intent governs the conversion "taking into account the target
///    space's colour gamut (the range of colours it can reproduce)". Which mapping an intent
///    selects is ISO 15076-1's, a standard this project does not hold, so the mapping here is
///    the nearest reachable colour by squared distance in the device's own three components —
///    recorded in ADR 0263 as a decision, not derived. sRGB's own primaries are the standing
///    example: no mixture of these inks makes `#FF0000`, and it lands on `#ED1C24`.
///
/// A page whose colours are all inside that gamut is therefore drawn exactly as it is drawn
/// today, with only its *composites* moving, which is the whole of what §11.7.2 asks for.
pub(super) fn search_ink(space: &pdf_render::BlendingSpace, target: [f32; 3]) -> [f32; 4] {
    let nominal = rgb_to_cmyk(Color::rgb(target[0], target[1], target[2]));
    let mut warm = [nominal[0], nominal[1], nominal[2]];

    let first = ink_at_black(space, target, nominal[3], warm);
    if first.worst < INK_EXACT {
        return [first.inks[0], first.inks[1], first.inks[2], nominal[3]];
    }
    let mut nearest = (first, nominal[3]);
    warm = first.inks;

    for rung in 0..INK_LADDER {
        let span = INK_LADDER.saturating_sub(1);
        #[expect(
            clippy::cast_precision_loss,
            reason = "the ladder is twelve rungs; both counts are exact in f32"
        )]
        let black = span.saturating_sub(rung) as f32 / span as f32;
        let solved = ink_at_black(space, target, black, warm);
        if solved.worst < INK_EXACT {
            return [solved.inks[0], solved.inks[1], solved.inks[2], black];
        }
        if solved.squared < nearest.0.squared {
            nearest = (solved, black);
        }
        warm = solved.inks;
    }

    let (closest, black) = nearest;
    polish_four_inks(
        space,
        target,
        [closest.inks[0], closest.inks[1], closest.inks[2], black],
    )
}

/// What one slice of the ladder found: three inks, and how far the colour they make still is.
#[derive(Clone, Copy)]
struct Separation {
    /// Cyan, magenta and yellow, in `0.0..=1.0`.
    inks: [f32; 3],
    /// The largest of the three per-component differences, which decides *reproduced*.
    worst: f32,
    /// The squared distance, which decides *nearest* between two that are not.
    squared: f32,
}

/// The three chromatic inks that come closest to `target` at a fixed `black`.
///
/// Gauss–Newton on the trilinear slice, projected onto the unit cube and backtracked so that
/// the squared distance falls at every step. `start` is the previous rung's answer, which is
/// why the ladder costs far fewer steps than it has rungs.
fn ink_at_black(
    space: &pdf_render::BlendingSpace,
    target: [f32; 3],
    black: f32,
    start: [f32; 3],
) -> Separation {
    let mut inks = [channel(start[0]), channel(start[1]), channel(start[2])];
    let mut jacobian = [[0.0f32; 3]; 3];
    let mut value = press_at_black(space, black, inks, Some(&mut jacobian));
    let (mut worst, mut squared) = gaps(value, target);

    for _ in 0..INK_STEPS {
        if worst < INK_EXACT {
            break;
        }
        let right = [
            value[0] - target[0],
            value[1] - target[1],
            value[2] - target[2],
        ];
        let Some(step) = solve_three(&jacobian, right) else {
            break;
        };
        let mut scale = 1.0f32;
        let mut improved = false;
        for _ in 0..INK_BACKTRACKS {
            let candidate = [
                channel(scale.mul_add(-step[0], inks[0])),
                channel(scale.mul_add(-step[1], inks[1])),
                channel(scale.mul_add(-step[2], inks[2])),
            ];
            let next = press_at_black(space, black, candidate, None);
            let (next_worst, next_squared) = gaps(next, target);
            if next_squared < squared {
                inks = candidate;
                worst = next_worst;
                squared = next_squared;
                improved = true;
                break;
            }
            scale *= 0.5;
        }
        if !improved {
            break;
        }
        value = press_at_black(space, black, inks, Some(&mut jacobian));
    }
    Separation {
        inks,
        worst,
        squared,
    }
}

/// Moves all four inks together, from a starting separation near the answer.
///
/// The ladder fixes the black at one of thirteen values and [`Press::table`] at one of its grid
/// neighbours', so a colour whose feasible band of black falls *between* those is missed by
/// both. Here the black moves with the other three: the Jacobian is three rows by four
/// columns, so the step taken is the smallest one that answers — `Jᵀ(JJᵀ)⁻¹r`, the
/// minimum-norm solution — which keeps the answer near where it started rather than wandering
/// to some other preimage.
fn polish_four_inks(
    space: &pdf_render::BlendingSpace,
    target: [f32; 3],
    start: [f32; 4],
) -> [f32; 4] {
    let mut inks = [
        channel(start[0]),
        channel(start[1]),
        channel(start[2]),
        channel(start[3]),
    ];
    let mut jacobian = [[0.0f32; 3]; 4];
    // The value alone first: a separation that already reproduces its colour — which is what
    // a lookup away from the gamut's boundary is — costs one evaluation and no derivative.
    let mut value = press_at(space, inks, None);
    let (mut worst, mut squared) = gaps(value, target);
    if worst < INK_EXACT {
        return inks;
    }
    value = press_at(space, inks, Some(&mut jacobian));

    for _ in 0..INK_POLISH {
        if worst < INK_EXACT {
            break;
        }
        let right = [
            value[0] - target[0],
            value[1] - target[1],
            value[2] - target[2],
        ];
        // `J Jᵀ` is three by three, and its columns are what `solve_three` takes.
        let mut normal = [[0.0f32; 3]; 3];
        for (column, out) in normal.iter_mut().enumerate() {
            for (row, entry) in out.iter_mut().enumerate() {
                for ink in &jacobian {
                    *entry = ink
                        .get(row)
                        .copied()
                        .unwrap_or(0.0)
                        .mul_add(ink.get(column).copied().unwrap_or(0.0), *entry);
                }
            }
        }
        let Some(multipliers) = solve_three(&normal, right) else {
            break;
        };
        let mut step = [0.0f32; 4];
        for (ink, component) in jacobian.iter().zip(step.iter_mut()) {
            for (row, factor) in multipliers.iter().enumerate() {
                *component = ink
                    .get(row)
                    .copied()
                    .unwrap_or(0.0)
                    .mul_add(*factor, *component);
            }
        }
        let mut scale = 1.0f32;
        let mut improved = false;
        for _ in 0..INK_BACKTRACKS {
            let mut candidate = [0.0f32; 4];
            for ((component, was), taken) in candidate.iter_mut().zip(inks).zip(step) {
                *component = channel(scale.mul_add(-taken, was));
            }
            let next = press_at(space, candidate, None);
            let (next_worst, next_squared) = gaps(next, target);
            if next_squared < squared {
                inks = candidate;
                worst = next_worst;
                squared = next_squared;
                improved = true;
                break;
            }
            scale *= 0.5;
        }
        if !improved {
            break;
        }
        value = press_at(space, inks, Some(&mut jacobian));
    }
    inks
}

/// The largest per-component difference between two colours, and the squared distance.
fn gaps(value: [f32; 3], target: [f32; 3]) -> (f32, f32) {
    let mut worst = 0.0f32;
    let mut squared = 0.0f32;
    for (component, wanted) in value.iter().zip(target) {
        let gap = component - wanted;
        worst = worst.max(gap.abs());
        squared = gap.mul_add(gap, squared);
    }
    (worst, squared)
}

/// Which grid cell `value` falls in on one axis, and how far across that cell it is.
///
/// At a side of two the cell is the whole axis and the fraction is the value itself, which is
/// what makes every evaluator below reduce *exactly* to the arithmetic they had when
/// [`super::device_cmyk::CMYK_CORNERS`] was the only press.
fn press_cell(side: usize, value: f32) -> (usize, f32) {
    let last = side.saturating_sub(1);
    #[expect(
        clippy::cast_precision_loss,
        reason = "a grid side below 18, exact in f32"
    )]
    let scaled = channel(value) * last as f32;
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "`scaled` is in 0..=last because `channel` clamps its argument"
    )]
    let cell = (scaled as usize).min(last.saturating_sub(1));
    #[expect(
        clippy::cast_precision_loss,
        reason = "a cell index below the grid side"
    )]
    let fraction = (scaled - cell as f32).clamp(0.0, 1.0);
    (cell, fraction)
}

/// One sample of a press's grid, by its index on each of the four axes.
fn press_sample(space: &pdf_render::BlendingSpace, at: [usize; 4]) -> [f32; 3] {
    let side = space.side();
    let index = ((at[3].saturating_mul(side).saturating_add(at[2]))
        .saturating_mul(side)
        .saturating_add(at[1]))
    .saturating_mul(side)
    .saturating_add(at[0]);
    space.grid().get(index).copied().unwrap_or([0.0; 3])
}

/// A press's value at three inks and a fixed black, with the slope in each of the three.
///
/// A press's grid is multilinear *inside a cell*, so fixing the black leaves a trilinear map
/// over the other three whose eight corners are the cell's, each interpolated along the black
/// axis. The Jacobian [`multilinear`] returns is with respect to the fraction across a cell,
/// so it is scaled back to the ink itself — which is the identity at a side of two, where the
/// cell *is* the axis.
fn press_at_black(
    space: &pdf_render::BlendingSpace,
    black: f32,
    inks: [f32; 3],
    mut jacobian: Option<&mut [[f32; 3]; 3]>,
) -> [f32; 3] {
    let side = space.side();
    let (black_cell, black_fraction) = press_cell(side, black);
    let cells = [
        press_cell(side, inks[0]),
        press_cell(side, inks[1]),
        press_cell(side, inks[2]),
    ];
    let mut slice = [[0.0f32; 3]; 8];
    for (index, corner) in slice.iter_mut().enumerate() {
        let at = |axis: usize| {
            cells
                .get(axis)
                .map_or(0, |cell| cell.0)
                .saturating_add(index >> axis & 1)
        };
        let without = press_sample(space, [at(0), at(1), at(2), black_cell]);
        let with = press_sample(space, [at(0), at(1), at(2), black_cell.saturating_add(1)]);
        for (component, (low, high)) in corner.iter_mut().zip(without.iter().zip(with)) {
            *component = black_fraction.mul_add(high - low, *low);
        }
    }
    let fractions = [cells[0].1, cells[1].1, cells[2].1];
    let value = multilinear(&slice, &fractions, jacobian.as_deref_mut());
    if let Some(jacobian) = jacobian {
        scale_jacobian(jacobian, side);
    }
    value
}

/// A press's value at four inks, with the slope in each of the four.
///
/// The same construction as [`press_at_black`] one axis higher: the sixteen corners of the
/// cell the inks fall in, and the Jacobian scaled from the cell back to the inks.
fn press_at(
    space: &pdf_render::BlendingSpace,
    inks: [f32; 4],
    mut jacobian: Option<&mut [[f32; 3]; 4]>,
) -> [f32; 3] {
    let side = space.side();
    let cells = [
        press_cell(side, inks[0]),
        press_cell(side, inks[1]),
        press_cell(side, inks[2]),
        press_cell(side, inks[3]),
    ];
    let mut corners = [[0.0f32; 3]; 16];
    for (index, corner) in corners.iter_mut().enumerate() {
        let at = |axis: usize| {
            cells
                .get(axis)
                .map_or(0, |cell| cell.0)
                .saturating_add(index >> axis & 1)
        };
        *corner = press_sample(space, [at(0), at(1), at(2), at(3)]);
    }
    let fractions = [cells[0].1, cells[1].1, cells[2].1, cells[3].1];
    let value = multilinear(&corners, &fractions, jacobian.as_deref_mut());
    if let Some(jacobian) = jacobian {
        scale_jacobian(jacobian, side);
    }
    value
}

/// Turns a derivative with respect to a cell fraction into one with respect to the ink.
///
/// A fraction crosses its whole cell while the ink crosses `1 / (side − 1)` of its axis, so
/// the chain rule multiplies by `side − 1` — which is one at a side of two.
fn scale_jacobian<const N: usize>(jacobian: &mut [[f32; 3]; N], side: usize) {
    let last = side.saturating_sub(1);
    if last <= 1 {
        return;
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "a grid side below 18, exact in f32"
    )]
    let factor = last as f32;
    for column in jacobian.iter_mut() {
        for slope in column.iter_mut() {
            *slope *= factor;
        }
    }
}

/// A multilinear map's value at `inks`, and optionally its Jacobian as `N` columns.
///
/// `corners` holds the map's `2^N` values, indexed by the bits of the ink axes with the first
/// axis least significant — the shape [`super::device_cmyk::cmyk`] already uses. `jacobian[i]` is the derivative
/// of the sRGB triple with respect to the `i`th ink, which is the same sum with that axis's
/// weights replaced by ±1.
///
/// **The Jacobian is optional and that is a measured decision**: it is three quarters of the
/// arithmetic here, and the backtracking inside a Gauss–Newton step needs only the value —
/// evaluating it there as well took the whole conversion from 6.9 µs to 26.7 µs per colour.
fn multilinear<const N: usize>(
    corners: &[[f32; 3]],
    inks: &[f32; N],
    mut jacobian: Option<&mut [[f32; 3]; N]>,
) -> [f32; 3] {
    if let Some(jacobian) = jacobian.as_deref_mut() {
        *jacobian = [[0.0f32; 3]; N];
    }
    let mut value = [0.0f32; 3];
    for (index, corner) in corners.iter().enumerate() {
        let weight_at = |axis: usize| {
            let ink = inks.get(axis).copied().unwrap_or(0.0);
            if index >> axis & 1 == 1 {
                ink
            } else {
                1.0 - ink
            }
        };
        let mut weight = 1.0f32;
        for axis in 0..N {
            weight *= weight_at(axis);
        }
        for (axis, output) in value.iter_mut().enumerate() {
            *output = weight.mul_add(corner.get(axis).copied().unwrap_or(0.0), *output);
        }
        let Some(jacobian) = jacobian.as_deref_mut() else {
            continue;
        };
        for (ink, column) in jacobian.iter_mut().enumerate() {
            let mut slope = if index >> ink & 1 == 1 {
                1.0f32
            } else {
                -1.0f32
            };
            for axis in 0..N {
                if axis != ink {
                    slope *= weight_at(axis);
                }
            }
            for (axis, output) in column.iter_mut().enumerate() {
                *output = slope.mul_add(corner.get(axis).copied().unwrap_or(0.0), *output);
            }
        }
    }
    value
}

/// Solves `columns · x = right` for `x`, or `None` where the three columns are coplanar.
///
/// Cramer's rule on three columns, which is the clearest construction at this size and needs
/// no pivoting to be read: each unknown is the determinant with its own column replaced,
/// divided by the determinant. A singular system is a point where the map is locally flat in
/// some direction, and the caller stops there rather than stepping to infinity.
pub(crate) fn solve_three(columns: &[[f32; 3]; 3], right: [f32; 3]) -> Option<[f32; 3]> {
    let (a, b, c) = (columns[0], columns[1], columns[2]);
    let determinant = triple(a, b, c);
    if determinant.abs() < 1e-9 {
        return None;
    }
    Some([
        triple(right, b, c) / determinant,
        triple(a, right, c) / determinant,
        triple(a, b, right) / determinant,
    ])
}

/// The scalar triple product `a · (b × c)`, which is the determinant of the three as columns.
fn triple(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> f32 {
    a[0].mul_add(
        b[1].mul_add(c[2], -(b[2] * c[1])),
        a[1].mul_add(
            b[2].mul_add(c[0], -(b[0] * c[2])),
            a[2] * b[0].mul_add(c[1], -(b[1] * c[0])),
        ),
    )
}
