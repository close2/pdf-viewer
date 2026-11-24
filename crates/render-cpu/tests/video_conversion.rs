//! The processor's video conversion against hand-built planes whose RGB is computed here from ITU-T
//! H.264 Annex E's equations, written out in their own form rather than through
//! `pdf_render::video::Conversion::affine` (ADR 3529).
//!
//! The equations are cited from Annex E of Rec. ITU-T H.264 (V16) (06/2026), held as
//! `doc/ITU-T-H.264-202606.pdf` (E.2.1's Equations E-17 to E-19 and E-23 to E-25 for the range,
//! Table E-5's `K_R`/`K_B` with E-35 to E-37 for the matrix, Figure E-1 for the chroma places), not
//! quoted.

#![expect(
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "test code: a fixture built here fails loudly, and its values are levels"
)]

use pdf_render::video::{Conversion, ConversionRefused, Planes, Signal};
use render_cpu::video::convert_picture;

/// Annex E's equations, from the codes up: `(Y, Cb, Cr)` to `[R, G, B]` levels, unrounded.
fn annex_e(luma: f64, cb: f64, cr: f64, kr: f64, kb: f64, full_range: bool) -> [f64; 3] {
    let (ey, epb, epr) = if full_range {
        (luma / 255.0, (cb - 128.0) / 255.0, (cr - 128.0) / 255.0)
    } else {
        (
            (luma - 16.0) / 219.0,
            (cb - 128.0) / 224.0,
            (cr - 128.0) / 224.0,
        )
    };
    let red = ey + 2.0 * (1.0 - kr) * epr;
    let blue = ey + 2.0 * (1.0 - kb) * epb;
    let green = (ey - kr * red - kb * blue) / (1.0 - kr - kb);
    [red, green, blue].map(|value| 255.0 * value)
}

fn rounded(value: f64) -> u8 {
    value.round().clamp(0.0, 255.0) as u8
}

/// A `width` by `height` picture of one colour.
fn flat(width: u32, height: u32, [y, cb, cr]: [u8; 3]) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let chroma = (width.div_ceil(2) * height.div_ceil(2)) as usize;
    (
        vec![y; (width * height) as usize],
        vec![cb; chroma],
        vec![cr; chroma],
    )
}

fn signal(matrix: u8, full_range: bool, siting: u8) -> Signal {
    Signal {
        full_range,
        matrix_coefficients: matrix,
        chroma_sample_loc_type: siting,
        ..Signal::default()
    }
}

/// Every matrix Table E-5 gives a pair for, at both ranges, over colours spread across the cube:
/// every pixel of a flat picture is Annex E's value rounded to the nearest level.
#[test]
fn a_flat_picture_is_annex_e_s_equations_rounded() {
    let pairs = [
        (1, 0.2126, 0.0722),
        (4, 0.30, 0.11),
        (5, 0.299, 0.114),
        (6, 0.299, 0.114),
        (7, 0.212, 0.087),
        (9, 0.2627, 0.0593),
    ];
    let colours = [
        [16, 128, 128],
        [235, 128, 128],
        [81, 90, 240],
        [145, 54, 34],
        [41, 240, 110],
        [0, 0, 255],
        [255, 255, 0],
        [128, 100, 200],
    ];
    for (matrix, kr, kb) in pairs {
        for full_range in [false, true] {
            let conversion = Conversion::for_signal(signal(matrix, full_range, 0), 1080).unwrap();
            assert!(conversion.matrix_stated);
            for colour in colours {
                let (y, cb, cr) = flat(5, 3, colour);
                let planes = Planes {
                    width: 5,
                    height: 3,
                    y: &y,
                    cb: &cb,
                    cr: &cr,
                };
                let raster = convert_picture(&planes, &conversion).unwrap();
                let expected = annex_e(
                    f64::from(colour[0]),
                    f64::from(colour[1]),
                    f64::from(colour[2]),
                    kr,
                    kb,
                    full_range,
                )
                .map(rounded);
                for pixel in raster.data.as_chunks::<4>().0 {
                    assert_eq!(
                        *pixel,
                        [expected[0], expected[1], expected[2], 255],
                        "matrix {matrix}, full range {full_range}, {colour:?}"
                    );
                }
            }
        }
    }
}

/// Limited range's black and white are 16 and 235 with neutral chroma, and BT.601's red
/// `(81, 90, 240)` is red to within a level, by the equations.
#[test]
fn limited_range_black_white_and_red() {
    let conversion = Conversion::for_signal(signal(6, false, 0), 480).unwrap();
    let convert = |colour| {
        let (y, cb, cr) = flat(2, 2, colour);
        let planes = Planes {
            width: 2,
            height: 2,
            y: &y,
            cb: &cb,
            cr: &cr,
        };
        convert_picture(&planes, &conversion).unwrap().data[..3].to_vec()
    };
    assert_eq!(convert([16, 128, 128]), [0, 0, 0]);
    assert_eq!(convert([235, 128, 128]), [255, 255, 255]);
    let red = convert([81, 90, 240]);
    assert!(red[0] >= 254 && red[1] <= 1 && red[2] <= 1, "{red:?}");
}

/// Figure E-1's six places, read through a chroma plane of two columns and two rows whose red
/// difference steps: each pixel's chroma is the linear interpolation at its own position, which
/// is `(2x − site_x) / 4` chroma columns and `(2y − site_y) / 4` rows, the edges held.
#[test]
fn each_siting_interpolates_at_its_own_place() {
    // Cr 0 and 64 across, 128 and 192 down: a value in sixteenths is 16 × the interpolation.
    let cr_plane = [0u8, 64, 128, 192];
    let (width, height) = (4u32, 4u32);
    let at = |column: f64, row: f64| {
        let column = column.clamp(0.0, 1.0);
        let row = row.clamp(0.0, 1.0);
        let top = 64.0 * column;
        let bottom = 128.0 + 64.0 * column;
        top + (bottom - top) * row
    };
    for siting in 0..=5u8 {
        let conversion = Conversion::for_signal(signal(1, true, siting), 4).unwrap();
        let (site_x, site_y) = (
            [0.0, 0.5, 0.0, 0.5, 0.0, 0.5][usize::from(siting)],
            [0.5, 0.5, 0.0, 0.0, 1.0, 1.0][usize::from(siting)],
        );
        for y in 0..height {
            for x in 0..width {
                let expected =
                    16.0 * at((f64::from(x) - site_x) / 2.0, (f64::from(y) - site_y) / 2.0);
                let got = pdf_render::video::chroma_sixteenths(&cr_plane, 2, 2, &conversion, x, y);
                assert_eq!(f64::from(got), expected, "siting {siting} at ({x}, {y})");
            }
        }
        let y_plane = vec![128u8; 16];
        let neutral = [128u8; 4];
        let planes = Planes {
            width,
            height,
            y: &y_plane,
            cb: &neutral,
            cr: &cr_plane,
        };
        let raster = convert_picture(&planes, &conversion).unwrap();
        assert_eq!(raster.data.len(), 64);
    }
}

/// An unspecified matrix is the stated choice — BT.601's pair up to 576 lines, BT.709's above —
/// and says it was chosen; the matrices whose formula is not the pair, and a siting past Figure
/// E-1's six, are refused by name.
#[test]
fn the_choice_and_the_refusals_are_said() {
    for (code, height, kr) in [
        (2, 576, 0.299),
        (2, 720, 0.2126),
        (3, 480, 0.299),
        (18, 1080, 0.2126),
        (200, 1080, 0.2126),
    ] {
        let conversion = Conversion::for_signal(signal(code, false, 0), height).unwrap();
        assert!(!conversion.matrix_stated);
        assert!((conversion.kr - kr).abs() < 1e-12, "{code} at {height}");
    }
    // Table E-5 of the held V16 edition defines 15 (IPT-C2), 16 (YCgCo-Re) and 17 (YCgCo-Ro); the
    // first reserved code is 18.
    for code in [0, 8, 10, 11, 12, 13, 14, 15, 16, 17] {
        assert!(matches!(
            Conversion::for_signal(signal(code, false, 0), 1080),
            Err(ConversionRefused::Matrix { code: refused, .. }) if refused == code
        ));
    }
    assert_eq!(
        Conversion::for_signal(signal(1, false, 6), 1080),
        Err(ConversionRefused::Siting(6))
    );
    let conversion = Conversion::for_signal(Signal::default(), 1080).unwrap();
    let planes = Planes {
        width: 3,
        height: 3,
        y: &[0; 9],
        cb: &[0; 3],
        cr: &[0; 4],
    };
    assert_eq!(
        convert_picture(&planes, &conversion),
        Err(ConversionRefused::Planes {
            width: 3,
            height: 3
        })
    );
}
