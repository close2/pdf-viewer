//! Table 74's `/TilingType`: the lattice a tiling pattern's cells are laid on.
//!
//! ISO 32000-2 §8.7.3.1 replicates a pattern cell "at fixed horizontal and vertical intervals",
//! and Table 74's `/TilingType` is "[a] code that controls adjustments to the spacing of tiles
//! relative to the device pixel grid". Three codes, and the difference between them is which of
//! two quantities a processor is allowed to move — the cell, or the gaps between cells:
//!
//! - **1, constant spacing.** "Pattern cells shall be spaced consistently - that is, by a multiple
//!   of a device pixel", achieved where it has to be by distorting the cell slightly — the clause
//!   names small adjustments to `/XStep`, `/YStep` and the transformation matrix as the way — and
//!   "[t]he amount of distortion shall not exceed 1 device pixel."
//! - **2, no distortion.** "The pattern cell shall not be distorted, but the spacing between
//!   pattern cells may vary by as much as 1 device pixel, both horizontally and vertically, when
//!   the pattern is painted."
//! - **3, constant spacing and faster tiling.** Code 1's lattice "but with additional distortion
//!   permitted to enable a more efficient" tiling. A processor may take that permission and this
//!   one does not: code 3 is given code 1's arithmetic, which distorts no more than code 1 may.
//!
//! # The arithmetic, in device pixels
//!
//! §8.7.3.1 puts site (*i*, *j*) at *i* × `/XStep` and *j* × `/YStep` in pattern space, so the two
//! *step vectors* are what the lattice is made of. Carried through the pattern matrix's linear
//! part and the device scale, they are
//!
//! ```text
//! u = s · (a·XStep, b·XStep)        v = s · (c·YStep, d·YStep)
//! ```
//!
//! in device pixels, where `s` is pixels per page unit. "[S]paced consistently … by a multiple of
//! a device pixel" is then one sentence of arithmetic: round each component of `u` and of `v` to
//! the nearest whole pixel. Every site is a whole number of `u` and `v` away from the key cell, so
//! rounding those two makes every site land at a whole number of device pixels from it — which is
//! what makes every cell of the tiling carry the *same* sub-pixel phase, however the pattern
//! matrix rotates or shears.
//!
//! What the rounding costs is a changed pattern matrix, which is the clause's own remedy: the
//! adjusted matrix is the one that sends `(/XStep, 0)` and `(0, /YStep)` to the rounded vectors,
//! with `/XStep`, `/YStep` and the matrix's translation left exactly as the file states them. The
//! translation is untouched on purpose — §8.7.3.1 makes it the tiling's *phase* ("[t]he phase of
//! the tiling can be controlled by the translation components of the Matrix entry"), and the
//! clause asks for consistent spacing rather than for a particular place to start.
//!
//! # Why the bound is a condition and not a hope
//!
//! Each component moves by at most half a pixel, so the *step* is always within Table 74's
//! tolerance. The **cell** need not be: `/BBox` may be many steps across (Table 74's NOTE 2
//! permits it), and a cell *k* steps wide is distorted by up to *k* half-pixels. So the distortion
//! is measured — [`Lattice::distortion`], the furthest any corner of the cell moves relative to
//! the cell's own anchor — and a snap that would exceed [`MAX_DISTORTION`] is not taken at all.
//! The pattern then keeps the geometry the file states, which is code 2's tolerance rather than
//! code 1's; a cell that cannot be spaced consistently *without* breaking the clause's own bound
//! is a cell the clause does not ask to be spaced consistently.

use crate::geom::Transform;

/// Table 74's limit on what a constant-spacing lattice may cost the cell, in device pixels.
///
/// ISO 32000-2 §8.7.3.1, Table 74, `/TilingType` 1: "The amount of distortion shall not exceed 1
/// device pixel."
pub const MAX_DISTORTION: f32 = 1.0;

/// Table 74's `/TilingType` — what a Type 1 pattern asks of the device pixel grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TilingType {
    /// 1: constant spacing, the cell distorted by up to one device pixel to achieve it.
    ConstantSpacing,
    /// 2: no distortion, the spacing free to vary by up to one device pixel.
    NoDistortion,
    /// 3: constant spacing as in 1, with further distortion permitted for speed.
    FasterTiling,
}

impl TilingType {
    /// The code Table 74 states, or `None` for anything that is not one of its three.
    ///
    /// Table 74 makes the entry "( Required )" and states no default, so `None` is a file that
    /// asked for nothing readable rather than a file that asked for a particular thing. It is
    /// given [`Self::NoDistortion`]'s treatment by [`Self::constant_spacing`] returning `false`,
    /// which is a deliberate choice and not a reading of the clause: of the three codes it is the
    /// only one that requires nothing of the processor, so it is the one that leaves the geometry
    /// the file *did* state untouched.
    #[must_use]
    pub const fn from_code(code: i64) -> Option<Self> {
        match code {
            1 => Some(Self::ConstantSpacing),
            2 => Some(Self::NoDistortion),
            3 => Some(Self::FasterTiling),
            _ => None,
        }
    }

    /// Whether this code asks for the lattice to be snapped to the device pixel grid.
    ///
    /// True for codes 1 and 3, which Table 74 describes with the same words — "[p]attern cells
    /// shall be spaced consistently as in tiling Type 1" — and false for code 2, whose cell
    /// "shall not be distorted".
    #[must_use]
    pub const fn constant_spacing(self) -> bool {
        matches!(self, Self::ConstantSpacing | Self::FasterTiling)
    }
}

/// A tiling's pattern matrix once Table 74's constant spacing has been applied to it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lattice {
    /// The adjusted pattern-space-to-page-space transform the cell and every site are placed by.
    ///
    /// Its translation is the file's own: only the linear part moves, and only by what rounding
    /// the two step vectors onto whole device pixels asks for.
    pub to_page: Transform,
    /// How far the cell moved, in device pixels — Table 74's "amount of distortion".
    ///
    /// Measured at the cell's corners relative to the cell's own anchor, so it is the change to
    /// the cell's *shape* and not the lattice displacement every site shares. Never greater than
    /// [`MAX_DISTORTION`]: [`snap_lattice`] returns `None` rather than a lattice that would
    /// exceed it.
    pub distortion: f32,
}

/// Table 74's constant spacing for one tiling, or `None` where it is not to be taken.
///
/// `to_page` is the pattern matrix composed with the parent content stream's default space
/// (§8.7.2), `step` is `/XStep` and `/YStep` in pattern units, `extent` is the cell's width and
/// height in the same units, and `pixels_per_unit` is the device scale the page is being drawn at.
///
/// Returns `None` — meaning *place the tiling exactly as the file states it*, which is Table 74's
/// code 2 — in four circumstances, each of which is a lattice the clause's own sentence cannot be
/// met on:
///
/// - a degenerate input: a step, an extent or a scale that is not a finite positive number;
/// - a step vector that rounds to nothing, which would put every site in one place;
/// - two step vectors that round onto one line, which would collapse a lattice that had area;
/// - a snap whose distortion would exceed [`MAX_DISTORTION`], which Table 74 forbids outright.
#[must_use]
pub fn snap_lattice(
    to_page: Transform,
    step: (f32, f32),
    extent: (f32, f32),
    pixels_per_unit: f32,
) -> Option<Lattice> {
    let (step_x, step_y) = step;
    if !pixels_per_unit.is_finite() || pixels_per_unit <= 0.0 {
        return None;
    }
    if !step_x.is_finite() || !step_y.is_finite() {
        return None;
    }
    // Table 74: `/XStep` and `/YStep` "may be either positive or negative but shall not be zero".
    if step_x.abs() <= 0.0 || step_y.abs() <= 0.0 {
        return None;
    }
    // §8.7.3.1's two step vectors, in device pixels. The pattern matrix's translation is common
    // to every site and cancels out of a displacement, so only its linear part appears here.
    let stated = [
        pixels_per_unit * to_page.a * step_x,
        pixels_per_unit * to_page.b * step_x,
        pixels_per_unit * to_page.c * step_y,
        pixels_per_unit * to_page.d * step_y,
    ];
    if !stated.iter().all(|component| component.is_finite()) {
        return None;
    }
    let snapped = [
        stated[0].round(),
        stated[1].round(),
        stated[2].round(),
        stated[3].round(),
    ];
    // A step vector that rounds to the origin would place every site of its axis on top of the
    // key cell; two that round onto one line would collapse a lattice that had area. Neither is
    // "spaced consistently" — both are a different tiling — so the file's own geometry stands.
    if snapped[0].abs() + snapped[1].abs() <= 0.0 || snapped[2].abs() + snapped[3].abs() <= 0.0 {
        return None;
    }
    let area_before = stated[0].mul_add(stated[3], -(stated[1] * stated[2]));
    let area_after = snapped[0].mul_add(snapped[3], -(snapped[1] * snapped[2]));
    if area_after.abs() <= 0.0 && area_before.abs() > 0.0 {
        return None;
    }
    let moved = [
        snapped[0] - stated[0],
        snapped[1] - stated[1],
        snapped[2] - stated[2],
        snapped[3] - stated[3],
    ];
    // Nothing needed rounding, so nothing is recomputed: a lattice already on the grid keeps the
    // file's own matrix rather than one divided back out of it.
    if moved.iter().map(|component| component.abs()).sum::<f32>() <= 0.0 {
        return Some(Lattice {
            to_page,
            distortion: 0.0,
        });
    }
    let distortion = distortion_of(extent, step, moved)?;
    if distortion > MAX_DISTORTION {
        return None;
    }
    Some(Lattice {
        to_page: Transform::new(
            snapped[0] / (pixels_per_unit * step_x),
            snapped[1] / (pixels_per_unit * step_x),
            snapped[2] / (pixels_per_unit * step_y),
            snapped[3] / (pixels_per_unit * step_y),
            to_page.e,
            to_page.f,
        ),
        distortion,
    })
}

/// How far the far corners of a cell move under a snap, in device pixels.
///
/// The adjustment is linear in pattern space, so a point *p* moves by `(p.x/XStep)·Δu +
/// (p.y/YStep)·Δv` — and taking *p* relative to the cell's own anchor is what makes this the
/// cell's distortion rather than the lattice displacement its site already carries. Three corners
/// rather than four: the anchor itself does not move.
///
/// `None` where the cell's extent in steps is not a finite number, which is a `/BBox` no snap can
/// be measured against.
fn distortion_of(extent: (f32, f32), step: (f32, f32), moved: [f32; 4]) -> Option<f32> {
    let across = (extent.0 / step.0).abs();
    let up = (extent.1 / step.1).abs();
    if !across.is_finite() || !up.is_finite() {
        return None;
    }
    let mut worst = 0.0f32;
    for (x, y) in [(across, 0.0), (0.0, up), (across, up)] {
        let dx = x.mul_add(moved[0], y * moved[2]);
        let dy = x.mul_add(moved[1], y * moved[3]);
        worst = worst.max(dx.hypot(dy));
    }
    Some(worst)
}

#[cfg(test)]
mod tests {
    use super::{Lattice, MAX_DISTORTION, TilingType, snap_lattice};
    use crate::geom::Transform;

    /// Table 74's three codes, and everything else being nothing readable.
    #[test]
    fn table_74s_codes_are_the_three_the_clause_states() {
        assert_eq!(TilingType::from_code(1), Some(TilingType::ConstantSpacing));
        assert_eq!(TilingType::from_code(2), Some(TilingType::NoDistortion));
        assert_eq!(TilingType::from_code(3), Some(TilingType::FasterTiling));
        assert_eq!(TilingType::from_code(0), None);
        assert_eq!(TilingType::from_code(4), None);
        assert!(TilingType::ConstantSpacing.constant_spacing());
        assert!(TilingType::FasterTiling.constant_spacing());
        assert!(!TilingType::NoDistortion.constant_spacing());
    }

    /// The calibration fixture's arithmetic: 10.4 units at one pixel per unit snaps to 10.
    #[test]
    fn a_step_of_ten_point_four_becomes_a_whole_number_of_pixels() {
        let lattice = snap_lattice(Transform::IDENTITY, (10.4, 10.4), (10.4, 10.4), 1.0)
            .expect("a step within half a pixel of a whole one snaps");
        assert!(
            (lattice.to_page.a * 10.4 - 10.0).abs() < 1e-3,
            "the step became {} pixels",
            lattice.to_page.a * 10.4
        );
        // 0.4 of a pixel on each axis, at the cell's far corner: hypot(0.4, 0.4).
        assert!(
            (lattice.distortion - 0.4f32.hypot(0.4)).abs() < 1e-4,
            "distortion {}",
            lattice.distortion
        );
        assert!(lattice.distortion <= MAX_DISTORTION);
    }

    /// The distortion the clause bounds is the cell's, so scale magnifies it.
    #[test]
    fn the_same_step_snaps_differently_at_a_different_scale() {
        let at_two = snap_lattice(Transform::IDENTITY, (10.4, 10.4), (10.4, 10.4), 2.0)
            .expect("20.8 pixels snaps to 21");
        assert!((at_two.to_page.a * 10.4 * 2.0 - 21.0).abs() < 1e-3);
        assert!(at_two.distortion <= MAX_DISTORTION);
    }

    /// Table 74's bound is a condition on taking the snap at all.
    #[test]
    fn a_cell_many_steps_across_is_left_alone_rather_than_distorted_past_the_bound() {
        // Ten steps across, each rounding by 0.5 of a pixel: five pixels of distortion.
        assert_eq!(
            snap_lattice(Transform::IDENTITY, (10.5, 10.5), (105.0, 105.0), 1.0),
            None
        );
    }

    /// A lattice already on the grid keeps the file's own matrix, bit for bit.
    #[test]
    fn a_step_already_whole_changes_nothing() {
        let to_page = Transform::new(2.0, 0.0, 0.0, 3.0, 17.0, 19.0);
        assert_eq!(
            snap_lattice(to_page, (5.0, 5.0), (5.0, 5.0), 1.0),
            Some(Lattice {
                to_page,
                distortion: 0.0
            })
        );
    }

    /// A step under half a pixel has no whole number of pixels to be spaced by.
    #[test]
    fn a_step_that_rounds_to_nothing_is_not_snapped() {
        assert_eq!(
            snap_lattice(Transform::IDENTITY, (0.3, 0.3), (0.3, 0.3), 1.0),
            None
        );
    }

    /// The translation is the tiling's phase and the snap does not touch it.
    #[test]
    fn the_matrixs_translation_is_left_where_the_file_put_it() {
        let to_page = Transform::new(1.0, 0.0, 0.0, 1.0, 3.25, -7.75);
        let lattice = snap_lattice(to_page, (10.4, 10.4), (10.4, 10.4), 1.0).expect("snaps");
        assert!((lattice.to_page.e - 3.25).abs() < f32::EPSILON);
        assert!((lattice.to_page.f + 7.75).abs() < f32::EPSILON);
    }

    /// A rotated pattern matrix has both components of each step vector rounded.
    #[test]
    fn a_rotated_lattice_lands_on_whole_pixels_in_both_axes() {
        // 30 degrees, step 10: the x step is (8.660, 5.000) device pixels.
        let rotation = Transform::new(0.866_025, 0.5, -0.5, 0.866_025, 0.0, 0.0);
        let lattice = snap_lattice(rotation, (10.0, 10.0), (10.0, 10.0), 1.0).expect("snaps");
        for (component, expected) in [
            (lattice.to_page.a * 10.0, 9.0),
            (lattice.to_page.b * 10.0, 5.0),
            (lattice.to_page.c * 10.0, -5.0),
            (lattice.to_page.d * 10.0, 9.0),
        ] {
            assert!(
                (component - expected).abs() < 1e-3,
                "{component} is not {expected}"
            );
        }
        assert!(lattice.distortion <= MAX_DISTORTION);
    }

    /// Degenerate inputs are declined rather than divided by.
    #[test]
    fn a_degenerate_step_scale_or_matrix_declines_the_snap() {
        assert_eq!(
            snap_lattice(Transform::IDENTITY, (0.0, 10.0), (10.0, 10.0), 1.0),
            None
        );
        assert_eq!(
            snap_lattice(Transform::IDENTITY, (10.4, 10.4), (10.4, 10.4), 0.0),
            None
        );
        assert_eq!(
            snap_lattice(Transform::IDENTITY, (f32::NAN, 10.0), (10.0, 10.0), 1.0),
            None
        );
        // A matrix whose two steps already lie on one line keeps whatever it had; one that had
        // area and would lose it is declined.
        let collapsing = Transform::new(1.0, 0.0, 1.0, 0.4, 0.0, 0.0);
        assert_eq!(
            snap_lattice(collapsing, (1.0, 1.0), (1.0, 1.0), 1.0),
            None,
            "a lattice with area may not round onto one line"
        );
    }
}
