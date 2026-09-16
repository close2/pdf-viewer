//! `doc/QUORRA_FEEDBACK.md` section 46's residual reduction drawn without the page — and the
//! measurement that **excludes** it as the cause section 46 named. Section 46 reads
//! `22060_A1_01_Plans.pdf` 15–19% heavy on raster and attributes it to "the residual filter under
//! two-to-one": `pdf_render::Image::area_averaged` reduces by the integer floor of the ratio and
//! each backend's own filter resolves what is left, the fractional factor in `1.0 ..= 2.0`. That
//! page is the only witness section 46 carries, so its ask was the one of the `raster`
//! converter/reduce family without a reproduction away from a document. This example builds that
//! reproduction, and on it the two backends **agree** — which is the finding, not a failure of it.
//!
//! # The construction
//!
//! An image is drawn onto fewer device rows than it has samples. Where the ratio's floor is `1`
//! (a residual in `1.0 ..= 2.0`) `pdf_render::Image::reduction` returns `None` and the whole
//! downsample is the residual the backend filter does alone; where the floor is `2` or more the
//! integer stage runs first (`area_averaged` on the oracle, `raster-gpu/reduce.rs` on raster) and
//! the residual follows it — the two stages `22060` runs, reproduced here at `22060`'s own
//! geometry (a 2480-sample scan reduced by 6 onto ~360 device pixels). Ink is `255 - value` summed
//! over the placement's interior and divided by 255, the window and measure of `examples/image_phase`.
//!
//! The content is walked over three shapes across the runs of this example — full stripes, thin
//! rules, and a continuous-tone grating that aliases under the reduction — because a box average
//! and a two-tap resample part company only on high frequencies, and a scan is high frequency. On
//! every one, at every reduction and every phase, cpu and raster hold to **≤ 0.1%**; the widest
//! gap in the whole table is the magnifying control's edge anti-aliasing, at 0.27%.
//!
//! # What that excludes, and where section 46 then points
//!
//! An ordinary image crosses to raster as its own samples plus §8.9.5.3's flag, and raster reduces
//! and filters them with its own copy of the rule (`render_raster::scene::image`, the non-deferred
//! arm; ADR 0702). This example drives exactly that arm, so the agreement rules out the residual
//! filter as section 46's cause. `22060`'s four heavy images are not ordinary: each is a
//! `DCTDecode` `DeviceGray` scan carrying an `/SMask`, so it takes the *deferred* `AtDeviceScale`
//! arm instead (§11.6.5.2's mask on a device grid), where quorra reduces on this side and uploads.
//! The reproduction and the page are therefore not the same code path, which is why the page is
//! heavy and the reproduction is not — and section 46's ask is re-pointed there.
//!
//! ```sh
//! cargo run --release -p render-raster --example image_residual
//! ```

#![expect(
    clippy::expect_used,
    clippy::print_stdout,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "an example: its whole output is stdout, an absent device must fail loudly rather \
              than be reported as a backend's answer, every index and sum is inside a raster \
              this file sized itself, and the casts are between that raster's sides and the page \
              coordinates this file states — a two-figure constant either way, positive by \
              construction and exact in both representations"
)]

use pdf_render::{
    BlendMode, Command, DisplayList, Image, Raster, Rasterizer, SampleAlpha, Size, TargetSpec,
    Transform,
};

/// The page, large enough to hold the widest placement with a margin of paper around it.
const PAGE: f32 = 512.0;
/// Where the placement starts before the sub-pixel offset is added.
const ORIGIN: f32 = 32.0;

/// The row pattern an image holds. A box average and a two-tap resample agree on smooth content
/// and part on high frequencies, so the three that matter are the two hard-edged extremes and the
/// continuous tone a scan actually is.
#[derive(Clone, Copy)]
enum Content {
    /// Every row black or white in turn — the full-duty hard edge.
    Stripes,
    /// A black rule one sample thick every fourth row on white — the low-duty line work a scan
    /// reduces to grey.
    ThinRules,
    /// `value = 128 + 127·sin` at a frequency that aliases under the reduction — high-frequency
    /// continuous tone.
    Grating,
}

impl Content {
    fn name(self) -> &'static str {
        match self {
            Content::Stripes => "stripes",
            Content::ThinRules => "thin rules",
            Content::Grating => "grating",
        }
    }

    /// The 8-bit grey the row holds.
    fn level(self, row: u32) -> u8 {
        match self {
            Content::Stripes => {
                if row.is_multiple_of(2) {
                    0
                } else {
                    255
                }
            }
            Content::ThinRules => {
                if row.is_multiple_of(4) {
                    0
                } else {
                    255
                }
            }
            Content::Grating => {
                let phase = f64::from(row) * std::f64::consts::PI * 2.0 * 0.37;
                (128.0 + 127.0 * phase.sin()).round().clamp(0.0, 255.0) as u8
            }
        }
    }
}

/// A square image of `side` rows, opaque, holding `content`.
fn image(content: Content, side: u32) -> Image {
    let mut data = Vec::with_capacity((side * side * 4) as usize);
    for row in 0..side {
        let level = content.level(row);
        for _ in 0..side {
            data.extend_from_slice(&[level, level, level, 255]);
        }
    }
    Image {
        width: side,
        height: side,
        data: data.into(),
        // §8.9.5.3's hint, taken as one that turns filtering on, so the *ratio* is the only thing
        // that varies between the rungs.
        interpolate: true,
        sample_alpha: SampleAlpha::Shape,
    }
}

/// One page holding `picture` over `device` device pixels each way, offset by `offset`.
fn page(picture: Image, device: f32, offset: f32) -> DisplayList {
    let mut list = DisplayList::new(Size::new(PAGE, PAGE));
    list.push(Command::Image {
        image: picture.into(),
        transform: Transform::scale(device, device)
            .then(Transform::translate(ORIGIN + offset, ORIGIN + offset)),
        alpha: 1.0,
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });
    list
}

/// The ink the placement's interior holds, less its boundary pixels (paper as well as image).
/// `TargetSpec::for_page` flips about the page's height (trap 12a).
fn ink(raster: &Raster, device: f32, offset: f32) -> f64 {
    let left = (ORIGIN + offset).ceil() as u32;
    let right = (ORIGIN + offset + device).floor() as u32;
    let bottom = (PAGE - (ORIGIN + offset)).floor() as u32;
    let top = (PAGE - (ORIGIN + offset + device)).ceil() as u32;
    let mut sum = 0.0_f64;
    for y in top..bottom {
        for x in left..right {
            let at = ((y * raster.width + x) * 4) as usize;
            sum += f64::from(255 - raster.data[at]);
        }
    }
    sum / 255.0
}

fn main() {
    let mut cpu = render_cpu::CpuRasterizer::new();
    let mut quorra =
        render_raster::QuorraRasterizer::new_headless().expect("a headless raster device");

    // (label, image side in samples, device side in pixels). The ratio side/device is the
    // reduction. Above two and non-integer is section 46's case: `area_averaged` reduces by the
    // integer floor and the backend filter resolves what is left — the two stages 22060 runs.
    // The exact-integer rung reduces onto a native grid with no residual (section 48), and the
    // magnifying rung is the control where no reduction happens at all (section 46's 8× column).
    // (label, image side in samples, device side in pixels). The ratio side/device is the
    // reduction. Above two and non-integer is section 46's two-stage case: `area_averaged`
    // reduces by the integer floor and the backend filter resolves the residual left over. The
    // last two are 22060's own geometry — a 2480-sample scan reduced by 6 onto ~360 device
    // pixels — and the first is the control where the placement magnifies and nothing reduces.
    let rungs: [(&str, u32, f32); 5] = [
        ("magnify   0.67:1 (control)", 32, 48.0),
        ("2-stage   2.38:1 -> 1.19", 76, 32.0),
        ("2-stage   6.94:1 -> 1.18", 222, 32.0),
        ("22060     6.89:1 -> 1.15", 2480, 360.0),
        ("22060     3.44:1 -> 1.72", 2480, 720.0),
    ];

    // The offsets that separate a filter from a point sample: a filter's ink walks with the
    // phase, a point sample's is a staircase (`examples/image_phase`). The worst of them per rung
    // is what a divergence would show in, so the table prints that offset's figures.
    let offsets = [0.0_f32, 0.3, 0.5, 0.7];

    for content in [Content::Stripes, Content::ThinRules, Content::Grating] {
        println!(
            "\n{} — cpu vs raster, worst of offsets {:?}",
            content.name(),
            offsets
        );
        println!("  rung                       cpu ink     raster ink   worst raster−cpu");
        for (what, side, device) in rungs {
            let (mut ci_at, mut ri_at, mut worst) = (0.0, 0.0, 0.0_f64);
            for &offset in &offsets {
                let list = page(image(content, side), device, offset);
                let target = TargetSpec::for_page(&list, 1.0, 1 << 24).expect("a target");
                let ours = cpu.rasterize(&list, target).expect("the oracle draws it");
                let theirs = quorra.rasterize(&list, target).expect("raster draws it");
                let ci = ink(&ours, device, offset);
                let ri = ink(&theirs, device, offset);
                let rel = if ci > 0.0 {
                    (ri - ci) / ci * 100.0
                } else {
                    0.0
                };
                if rel.abs() >= worst.abs() {
                    (ci_at, ri_at, worst) = (ci, ri, rel);
                }
            }
            println!("  {what:<22}  {ci_at:>10.3}   {ri_at:>10.3}   {worst:>+13.2}%");
        }
    }
}
