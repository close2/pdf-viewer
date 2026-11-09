//! The Ghent PDF Output Suite's patches that a pixel comparison decides, in both views.
//!
//! The suite is the Ghent Workgroup's, its licence keeps it out of this repository, and
//! `tools/fetch-gwg.sh` puts it where `tools/places.sh` names `PDF_VIEWER_GWG`
//! (`doc/gwg-output-suite-provenance.md`). Where it is absent — CI's case — the test says so and
//! passes; where it is present it draws each patch below on the CPU, in the screen view and in
//! §10.8.3's press view, and compares the regions the patch's README says must agree.
//!
//! **Each check states the outcome this reader has today, failures included**, so the test is a
//! ratchet over `doc/gwg-output-suite.md`'s table rather than a list of hopes: a fix flips a check
//! and fails the test until the table and the expectation move together, and a regression fails it
//! the same way. A patch is self-revealing — an `X` or a half `X` appears where the processing is
//! wrong — so most checks compare a point on an arm of the `X` with a point beside it; a faint `X`
//! is not a failure by the suite's own rule, which is what the tolerance is for. The suite's
//! expected result is evidence about the standard and not the standard (CLAUDE.md principle 5):
//! the clause each check rests on is in the table, and where the two disagree the clause wins.
//!
//! Coordinates are pixels of a patch drawn at [`SCALE`], read off renders of the suite, which
//! no file of this tree carries. ADR 2728.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code: a suite file that does not open or draw should fail loudly, and every \
              window is a few pixels inside a patch of a few hundred"
)]

use std::path::{Path, PathBuf};

/// The environment variable, and the `tools/places.sh` name, of the suite's directory.
const DIRECTORY_VARIABLE: &str = "PDF_VIEWER_GWG";

/// `tools/places.sh`, which answers the default where the environment names none.
const PLACES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tools/places.sh");

/// 180 dots per inch: a patch's `X` arms are some fifty pixels wide, and a sample window of five
/// sits well inside one.
const SCALE: f32 = 2.5;

/// The largest channel difference, of 255, two regions may show and still count as equal: the
/// suite's `ReadMeFirst` calls a faint `X` "not a failure", from rounding and differing colour
/// engines, and a clear one is tens of levels.
const FAINT: u8 = 10;

/// The largest mean difference between horizontally adjacent pixels, of 255, inside a
/// photograph. The five sixteen-bit photographs drawn right measure 7.6 to 11.2; with their TIFF
/// prediction left undone they measure 21.5 (the ICC grey one, whose profile flattens the noise)
/// to 57.8. Sixteen sits between the two populations.
const SMOOTH: f64 = 16.0;

/// Which of the two views a check draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum View {
    /// The default view: §10.8.3's simulation not asked for.
    Screen,
    /// The press view: `ViewState::set_separation_simulation(true)`.
    Press,
}

/// What a check compares.
#[derive(Debug, Clone, Copy)]
enum Probe {
    /// Two 5×5 windows, by their top-left pixels, that the README says must agree.
    Equal((u32, u32), (u32, u32)),
    /// A window, as `(x, y, width, height)`, that must be one continuous-tone photograph.
    Smooth(u32, u32, u32, u32),
}

/// One region comparison on one patch in one view, and the outcome recorded for it.
struct Check {
    /// The patch's file under the suite's `Categories/` directory.
    patch: &'static str,
    /// What is compared, in the README's words.
    what: &'static str,
    view: View,
    probe: Probe,
    /// Whether this reader passes the comparison today (`doc/gwg-output-suite.md`).
    passes: bool,
}

const GREYS: &str = "3-ICC-CMS/Patches/GWG230_Four_different Grays_x1a.pdf";
const OPM: &str = "1-CMYK/Patches/GWG011_Overprint-Mode_x3.pdf";
const JPX_RGB: &str = "3-ICC-CMS/Patches/GWG172_JPEG2000_compression_ICCBasedRGB_x4.pdf";
const MASKS_1: &str = "1-CMYK/Patches/GWG168_Softmasks_Vector_part1_X4.pdf";
const MASKS_2: &str = "1-CMYK/Patches/GWG169_Softmasks_Vector_part2_X4.pdf";
const TEXT_2: &str = "1-CMYK/Patches/GWG1611_Softmasks_Text_part2_X4.pdf";
const OVERPRINT: &str = "1-CMYK/Patches/GWG010_CMYK_OP_x3.pdf";
const SPOT_OVERPRINT: &str = "2-SPOT/Patches/GWG020_CMYKSpot_OP_x1a.pdf";
const DEVICEN_BLACK: &str = "1-CMYK/Patches/GWG190_DeviceN_Overprint_Black_X1a.pdf";
const DEVICEN_YELLOW: &str = "1-CMYK/Patches/GWG191_DeviceN_Overprint_Yellow_X1a.pdf";
const DEVICEN_WHITE: &str = "1-CMYK/Patches/GWG192_DeviceN_Overprint_White_X1a.pdf";
const SOURCE_PROFILE: &str = "3-ICC-CMS/Patches/GWG130_ICC_Source_Profile_x4.pdf";
const BLEND_RGB: &str = "3-ICC-CMS/Patches/GWG161_Transp_Basic_BM_ICCBasedRGB_x4.pdf";
const INTENT_INDICATOR: &str = "3-ICC-CMS/Patches/GWG221_OutputIntentChangeIndicator_x4.pdf";
const GREY_OVERPRINT: &str = "2-SPOT/Patches/GWG030_Gray_K_black_OP_X1.pdf";

/// The checks. The four squares of patch 23.0 are centred at x = 103, 245, 386 and 528, y = 138;
/// an arm's window sits 35 pixels up and to one side of a centre and its neighbour 45 to the
/// same side on the centre line, outside the arm. The left half of each square is line art and
/// the right half an image, so each half is its own check.
const CHECKS: &[Check] = &[
    // 23.0: DeviceGray 25 %, DeviceCMYK 0/0/0/75, Separation Black 75 and DeviceN Black 75 must
    // render alike (§10.4.2.2 and §10.4.2.3's conversions; §8.6.6.4's `Black` colourant).
    grey_check(View::Screen, "DeviceGray, line art", 103, -1, false),
    grey_check(View::Screen, "DeviceCMYK, line art", 245, -1, true),
    grey_check(View::Screen, "Separation Black, line art", 386, -1, false),
    grey_check(View::Screen, "DeviceN Black, line art", 528, -1, true),
    grey_check(View::Screen, "DeviceN Black, image", 528, 1, false),
    // On the simulated press a grey is the press's black ink alone, §10.3.2's "should follow the
    // method described in 10.4.2.3" on a device whose native colour space is CMYK (ADR 2758).
    grey_check(View::Press, "DeviceGray, line art", 103, -1, true),
    grey_check(View::Press, "DeviceGray, image", 103, 1, true),
    grey_check(View::Press, "DeviceCMYK, line art", 245, -1, true),
    // In the press view `Black` is the simulated press's own colourant, which a `Separation` or
    // `DeviceN` naming it paints directly (§8.6.6.4, ADR 2736).
    grey_check(View::Press, "Separation Black, line art", 386, -1, true),
    grey_check(View::Press, "DeviceN Black, image", 528, 1, true),
    // 1.1: overprint mode 0 knocks a zero component out, and mode 1 leaves it (§8.6.7). The
    // squares are centred at x = 155 and 474, y = 135.
    Check {
        patch: OPM,
        what: "OPM 0 square",
        view: View::Press,
        probe: Probe::Equal((118, 98), (108, 133)),
        passes: true,
    },
    Check {
        patch: OPM,
        what: "OPM 1 square",
        view: View::Press,
        probe: Probe::Equal((437, 98), (427, 133)),
        passes: true,
    },
    // 1.0: a pink `X` overprinted on a brown square must vanish into it under §8.6.7. Squares a
    // and b are centred at x = 108 and 212, y = 106; the window on the arm sits on the centre and
    // its neighbour 20 pixels left of it, between two arms (ADR 2736).
    overprint_check(OVERPRINT, "font, OPM 0", (108, 106), true),
    overprint_check(OVERPRINT, "vector, OPM 0", (212, 106), true),
    // Square e is a radial DeviceN gradient over a yellow `X`, centred at (530, 106): the arm's
    // window and its neighbour sit at the same radius, on the diagonal and on the centre line
    // (ADR 2737).
    Check {
        patch: OVERPRINT,
        what: "DeviceN shading, OPM 0",
        view: View::Press,
        probe: Probe::Equal((542, 118), (548, 104)),
        passes: true,
    },
    // 2.0: a CMYK image over a spot square must leave the spot ink (§8.6.7, §11.7.4.3); squares c
    // and h are centred at x = 328, y = 106 and 208 (ADR 2737).
    overprint_check(SPOT_OVERPRINT, "CMYK image over spot, c", (328, 106), true),
    overprint_check(SPOT_OVERPRINT, "spot image over CMYK, h", (328, 208), true),
    // 19.0–19.2: a DeviceN cross overprinted with `OP` true must leave square a (vector) and b
    // (image) one solid colour (ADRs 2736, 2737). The squares are centred at x = 119 and 247.
    overprint_check(DEVICEN_BLACK, "vector a", (119, 121), true),
    overprint_check(DEVICEN_BLACK, "image b", (247, 121), true),
    overprint_check(DEVICEN_YELLOW, "vector a", (119, 90), true),
    overprint_check(DEVICEN_YELLOW, "image b", (247, 90), true),
    overprint_check(DEVICEN_WHITE, "vector a", (119, 121), true),
    overprint_check(DEVICEN_WHITE, "image b", (247, 121), true),
    // 3.0: a 50 % grey overprinting spot inks, b under OPM 0 and h under OPM 1, must leave them
    // as a 50 % K does: the grey is the press's black ink and still `DeviceGray` to §8.6.7, so
    // neither mode keeps a process ink it states as zero (ADR 2758). Centred at y = 200.
    overprint_check(GREY_OVERPRINT, "grey over spot, OPM 0, b", (65, 200), true),
    overprint_check(GREY_OVERPRINT, "grey over spot, OPM 1, h", (495, 200), true),
    // 13.0: an `ICCBased` colour stated `Perceptual` must reach the press as the green the `X`
    // stands on, which it does through the press's own `B2A0` (§11.7.5.3, ADR 2757). The squares
    // are centred at x = 106 + 142 n, y = 142; the window beside the arm is at the top's middle.
    source_check(View::Press, "RGB vector, a", 106, true),
    source_check(View::Press, "RGB image, b", 248, true),
    source_check(View::Press, "CMYK vector, c", 390, true),
    source_check(View::Press, "CMYK image, d", 532, true),
    // On screen the RGB pair is shown by its own profile, wider than the press (as 17.2 is).
    source_check(
        View::Screen,
        "RGB vector, a, beyond the press's gamut",
        106,
        false,
    ),
    source_check(View::Screen, "CMYK vector, c", 390, true),
    // 16.1 (ICCBased RGB): `Saturation` objects blended in the page's `DeviceCMYK` must leave
    // no clear `X`, which they do through the press's `B2A2` (ADR 2757). Squares by centre.
    blend_check(View::Screen, "Multiply", (121, 70)),
    blend_check(View::Screen, "Color Dodge", (436, 70)),
    blend_check(View::Screen, "Color", (436, 156)),
    blend_check(View::Press, "Multiply", (121, 70)),
    blend_check(View::Press, "Color Dodge", (436, 70)),
    blend_check(View::Press, "Color", (436, 156)),
    // 22.1: the grey Color Burn indicator shows no `X` when the intent is the file's own; its
    // `X` is drawn `Perceptual` (ADR 2757). Centred at (319, 150), beside the arm at the top.
    Check {
        patch: INTENT_INDICATOR,
        what: "Color Burn indicator",
        view: View::Screen,
        probe: Probe::Equal((317, 148), (317, 78)),
        passes: true,
    },
    Check {
        patch: INTENT_INDICATOR,
        what: "Color Burn indicator",
        view: View::Press,
        probe: Probe::Equal((317, 148), (317, 78)),
        passes: true,
    },
    // 18.x: sixteen-bit images, TIFF-predicted, must draw as the photograph (§7.4.4.4, §8.9.5).
    sixteen(
        "3-ICC-CMS/Patches/GWG180_16Bit_Images_ICCbasedRGB_x4.pdf",
        232,
        77,
    ),
    sixteen("1-CMYK/Patches/GWG181_16Bit_DeviceCMYK_x4.pdf", 223, 77),
    sixteen(
        "3-ICC-CMS/Patches/GWG182_16Bit_Images_ICCbasedGray_x4.pdf",
        268,
        77,
    ),
    sixteen(
        "3-ICC-CMS/Patches/GWG183_16Bit_Images_DeviceGray_x4.pdf",
        208,
        77,
    ),
    sixteen(
        "3-ICC-CMS/Patches/GWG184_16Bit_Images_ICCbasedCMYK_x4.pdf",
        232,
        74,
    ),
    // 17.x: a JPEG 2000 image must cover the X drawn behind it (§7.4.9); the squares are centred
    // at x = 315, y = 141.
    Check {
        patch: "1-CMYK/Patches/GWG170_JPEG2000_compression_DeviceCMYK_X4.pdf",
        what: "JPX DeviceCMYK image",
        view: View::Screen,
        probe: Probe::Equal((278, 104), (268, 139)),
        passes: true,
    },
    // 17.2's image is drawn in the dictionary's Adobe RGB, which the screen shows beyond the
    // press's gamut: the screen's `X` is not a decoding defect (deficit 3), and the press view
    // converts it into the press (ADR 2736).
    Check {
        patch: JPX_RGB,
        what: "JPX ICCBased RGB image, beyond the press's gamut on screen",
        view: View::Screen,
        probe: Probe::Equal((278, 104), (268, 139)),
        passes: false,
    },
    Check {
        patch: JPX_RGB,
        what: "JPX ICCBased RGB image, in the press's gamut",
        view: View::Press,
        probe: Probe::Equal((278, 104), (268, 139)),
        passes: true,
    },
    // 16.8, 16.9, 16.11: a vector or text soft mask must match the reference image drawn
    // `below` pixels under it (§11.5.3); each window sits in a glow's or a feather's ramp, where
    // a grey separated colorimetrically into four inks masked darker (ADR 2734).
    mask_check(MASKS_1, "Outer Glow", View::Screen, (334, 90), 94),
    mask_check(MASKS_1, "Inner Glow", View::Screen, (457, 108), 94),
    mask_check(MASKS_1, "Inner Glow", View::Press, (457, 108), 94),
    mask_check(MASKS_2, "Basic Feather", View::Screen, (271, 81), 96),
    mask_check(MASKS_2, "Directional Feather", View::Screen, (358, 78), 96),
    mask_check(MASKS_2, "Gradient Feather", View::Screen, (514, 114), 96),
    mask_check(MASKS_2, "Gradient Feather", View::Press, (514, 114), 96),
    mask_check(TEXT_2, "Basic Feather", View::Screen, (259, 116), 110),
    mask_check(TEXT_2, "Directional Feather", View::Screen, (358, 71), 110),
    mask_check(TEXT_2, "Gradient Feather", View::Screen, (496, 119), 110),
    mask_check(TEXT_2, "Gradient Feather", View::Press, (496, 119), 110),
];

/// A soft-mask effect's window and the same window of its reference image, `below` pixels down.
const fn mask_check(
    patch: &'static str,
    what: &'static str,
    view: View,
    (x, y): (u32, u32),
    below: u32,
) -> Check {
    Check {
        patch,
        what,
        view,
        probe: Probe::Equal((x, y), (x, y + below)),
        passes: true,
    }
}

/// One square of patch 13.0, by its centre's `x`: the arm at the centre against the top's middle.
const fn source_check(view: View, what: &'static str, x: u32, passes: bool) -> Check {
    Check {
        patch: SOURCE_PROFILE,
        what,
        view,
        probe: Probe::Equal((x - 2, 140), (x - 2, 78)),
        passes,
    }
}

/// One square of patch 16.1 (`ICCBased` RGB), by its centre: the centre against 20 pixels left.
const fn blend_check(view: View, what: &'static str, (x, y): (u32, u32)) -> Check {
    Check {
        patch: BLEND_RGB,
        what,
        view,
        probe: Probe::Equal((x - 2, y - 2), (x - 22, y - 2)),
        passes: true,
    }
}

/// An overprinted `X` in the press view, by its centre: the window on the centre against the one
/// 20 pixels to its left, between the arms (§8.6.7, §10.8.3).
const fn overprint_check(
    patch: &'static str,
    what: &'static str,
    (x, y): (u32, u32),
    passes: bool,
) -> Check {
    Check {
        patch,
        what,
        view: View::Press,
        probe: Probe::Equal((x - 2, y - 2), (x - 22, y - 2)),
        passes,
    }
}

/// One half of one square of patch 23.0: `side` is -1 for the left half and 1 for the right.
const fn grey_check(view: View, what: &'static str, centre: u32, side: i32, passes: bool) -> Check {
    let arm = centre.saturating_add_signed(side * 35);
    let beside = centre.saturating_add_signed(side * 45);
    Check {
        patch: GREYS,
        what,
        view,
        probe: Probe::Equal((arm - 2, 101), (beside - 2, 136)),
        passes,
    }
}

/// A sixteen-bit patch's photograph, by the top-left of a 100-pixel window inside it, in the
/// screen view: the samples are the same whichever view draws them.
const fn sixteen(patch: &'static str, x: u32, y: u32) -> Check {
    Check {
        patch,
        what: "16-bit photograph",
        view: View::Screen,
        probe: Probe::Smooth(x, y, 100, 100),
        passes: true,
    }
}

/// A patch drawn: width, height and its RGBA bytes.
struct Drawn {
    width: u32,
    height: u32,
    data: Vec<u8>,
}

impl Drawn {
    /// The mean colour of the 5×5 window whose top-left pixel is `(x, y)`.
    fn window(&self, (x, y): (u32, u32)) -> [u8; 3] {
        let mut sum = [0u32; 3];
        for dy in 0..5 {
            for dx in 0..5 {
                let pixel = self.pixel(x + dx, y + dy);
                for (total, channel) in sum.iter_mut().zip(pixel) {
                    *total += u32::from(channel);
                }
            }
        }
        sum.map(|total| u8::try_from(total / 25).expect("a mean of bytes is a byte"))
    }

    /// The pixel at `(x, y)`, which the checks keep inside the patch.
    fn pixel(&self, x: u32, y: u32) -> [u8; 3] {
        assert!(
            x < self.width && y < self.height,
            "({x}, {y}) is off the patch"
        );
        let at = (y as usize * self.width as usize + x as usize) * 4;
        [self.data[at], self.data[at + 1], self.data[at + 2]]
    }

    /// The mean difference between horizontally adjacent pixels' green channel in a window.
    fn roughness(&self, x: u32, y: u32, width: u32, height: u32) -> f64 {
        let mut total = 0u64;
        let mut count = 0u64;
        for row in y..y + height {
            for column in x..x + width - 1 {
                let left = self.pixel(column, row)[1];
                let right = self.pixel(column + 1, row)[1];
                total += u64::from(left.abs_diff(right));
                count += 1;
            }
        }
        #[expect(
            clippy::cast_precision_loss,
            reason = "a window's sum and count are far below 2^52"
        )]
        let mean = total as f64 / count as f64;
        mean
    }
}

/// Draws page one of `path` on the CPU at [`SCALE`], in `view`.
fn draw(path: &Path, view: View) -> Drawn {
    let bytes = std::fs::read(path).expect("the suite's file is readable");
    let document = pdf_syntax::Document::open(bytes).expect("the patch opens");
    let pages = pdf_model::Pages::new(&document);
    let page = pages.get(0).expect("the patch has a page");
    let mut state = pdf_model::view::ViewState::of(&document);
    if view == View::Press {
        state.set_separation_simulation(true);
    }
    let list = pdf_model::content::interpret_with(&document, &page, &state).display_list;
    let target = pdf_render::TargetSpec::for_page(&list, SCALE, 1 << 34).expect("a target");
    let mut rasterizer = render_cpu::CpuRasterizer::new();
    let raster = <render_cpu::CpuRasterizer as pdf_render::Rasterizer>::rasterize(
        &mut rasterizer,
        &list,
        target,
    )
    .expect("the patch draws");
    Drawn {
        width: raster.width,
        height: raster.height,
        data: raster.data.clone(),
    }
}

/// The suite's `Categories/` directory, or `None` where this machine has no suite.
fn categories() -> Option<PathBuf> {
    let directory = std::env::var_os(DIRECTORY_VARIABLE)
        .filter(|set| !set.is_empty())
        .map(PathBuf::from)
        .or_else(default_directory)?;
    let categories = directory.join("Ghent_PDF_Output_Suite_V50_Patches/Categories");
    categories.join(GREYS).is_file().then_some(categories)
}

/// The place `tools/places.sh` derives where the environment set none (ADR 2656); a failure to
/// answer is a broken `places.sh` and is loud rather than read as "no suite here".
#[cfg(unix)]
#[expect(
    clippy::unnecessary_wraps,
    reason = "one signature for both platforms, and the other half answers None"
)]
fn default_directory() -> Option<PathBuf> {
    let answered = std::process::Command::new("bash")
        .arg("-c")
        .arg(format!(". \"$1\" && printf %s \"${DIRECTORY_VARIABLE}\""))
        .arg("places")
        .arg(PLACES)
        .output()
        .expect("bash reads tools/places.sh");
    assert!(
        answered.status.success(),
        "tools/places.sh did not answer: {}",
        String::from_utf8_lossy(&answered.stderr)
    );
    Some(PathBuf::from(
        String::from_utf8_lossy(&answered.stdout).into_owned(),
    ))
}

/// Off Unix only a directory the environment names is read (the reason is
/// `crates/pdf-sandbox/tests/t88_conformance.rs`'s, which this follows).
#[cfg(not(unix))]
fn default_directory() -> Option<PathBuf> {
    None
}

#[test]
fn the_decidable_patches_keep_their_recorded_verdicts() {
    let Some(categories) = categories() else {
        println!(
            "skipped: the Ghent PDF Output Suite is not on this machine. Its licence keeps it out \
             of the repository (doc/gwg-output-suite-provenance.md); tools/fetch-gwg.sh fetches \
             it, pinned by hash, to where ${DIRECTORY_VARIABLE} names"
        );
        return;
    };
    let mut drawn: Vec<((&str, View), Drawn)> = Vec::new();
    let mut moved = Vec::new();
    for check in CHECKS {
        let key = (check.patch, check.view);
        if !drawn.iter().any(|(at, _)| *at == key) {
            drawn.push((key, draw(&categories.join(check.patch), check.view)));
        }
        let (_, patch) = drawn
            .iter()
            .find(|(at, _)| *at == key)
            .expect("drawn just above");
        let (passes, measured) = match check.probe {
            Probe::Equal(first, second) => {
                let (a, b) = (patch.window(first), patch.window(second));
                let gap = a
                    .iter()
                    .zip(b)
                    .map(|(x, y)| x.abs_diff(y))
                    .max()
                    .unwrap_or(0);
                (gap <= FAINT, format!("{a:?} against {b:?}, gap {gap}"))
            }
            Probe::Smooth(x, y, width, height) => {
                let rough = patch.roughness(x, y, width, height);
                (rough <= SMOOTH, format!("roughness {rough:.1}"))
            }
        };
        println!(
            "{} — {} ({:?}): {} — {measured}",
            check.patch,
            check.what,
            check.view,
            if passes { "pass" } else { "fail" }
        );
        if passes != check.passes {
            moved.push(format!(
                "{} — {} ({:?}) was recorded as {} and now {}: {measured}",
                check.patch,
                check.what,
                check.view,
                if check.passes { "a pass" } else { "a failure" },
                if passes { "passes" } else { "fails" }
            ));
        }
    }
    assert!(
        moved.is_empty(),
        "a patch's verdict moved; update doc/gwg-output-suite.md and this check together:\n{}",
        moved.join("\n")
    );
}
