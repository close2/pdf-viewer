//! Every page the corpus draws, through raster and through the CPU oracle.
//!
//! # Why this gate exists
//!
//! `real_pages.rs` renders four pages of the specification's own PDFs, and trap 12b is the
//! standing warning about what a small suite proves: the Vello backend passed fourteen
//! cross-backend scenes and then drew a *blank page* the first time it was handed a real one
//! at a real window's size. Four real pages is better than fourteen scenes and it is still
//! four. This is the same comparison over **974 documents' first pages** — every generator
//! anyone has pointed at pdf.js in fifteen years, including the malformed and the hostile.
//!
//! Both backends are handed the **same display list**, so nothing here is about PDF
//! semantics: a difference is a difference between two rasterisers, and a refusal is a
//! command the new backend cannot draw. That is the whole point — `CLAUDE.md` keeps the CPU
//! backend as the correctness oracle, and this is the instrument that holds a second backend
//! to it at the corpus's scale rather than at a fixture's.
//!
//! # What it measures
//!
//! - **Fidelity**, at **the glyph-phase quantum this product ships** — because a gate that turns
//!   a setting off cannot see what the setting does. It ran with the quantum off for the whole of
//!   its life, on the reasoning that doing so isolated the backend from a separately-gated trade,
//!   and that is exactly where raster's ADR 0073 hid: a rounding that reached the bucket count
//!   itself and wrapped, seating 3.1% of sub-pixel phases per axis a whole device pixel out of
//!   place, on the lane that draws text. `PDFVIEWER_RASTER_GLYPH_QUANTUM=off` still runs the
//!   isolation column (ADR 0498).
//! - **Refusals**, by name: a page raster cannot draw is a hole in the backend, and it must
//!   be one somebody wrote down rather than one nobody counted.
//! - **Speed**, both totals and the per-page median ratio, which answer different questions.
//!   A GPU frame here includes the readback to system memory, which a windowed host does not
//!   pay — `RENDER_LIBRARY.md` section 6.1 measures that at 55% to 92% of a frame — so the
//!   ratio below is the *offscreen* one and says nothing directly about the window.
//!
//! # Running it
//!
//! ``text
//! cargo test --release -p render-raster --test corpus -- --ignored --nocapture
//! ``
//!
//! `PDFVIEWER_RASTER_ONLY=a,b` restricts it to matching file names and **refuses to check the
//! ratchets**, saying so — a list held to equality over a subset would report every document
//! the filter excluded as fixed. `PDFVIEWER_RASTER_SCALE`, `PDFVIEWER_RASTER_COVERAGE` and
//! `PDFVIEWER_RASTER_GLYPH_QUANTUM` are the other three knobs, each documented at the function
//! that reads it, and each turning the ratchets off for the same reason; `PDFVIEWER_RASTER_TIMES`
//! writes each page's two clocks to a file and turns nothing off. Debug builds are ~15×
//! slower here; the numbers below are release numbers and the run says which it took.

#![expect(
    clippy::print_stdout,
    reason = "test code: an explanatory panic is the intended failure, and the survey \
              output is the point of the run"
)]

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use pdf_render::{DisplayList, Rasterizer, TargetSpec};
use pdf_syntax::{Document, Limits, SyntaxError};
use render_cpu::CpuRasterizer;
use render_raster::QuorraRasterizer;

#[path = "../../pdf-model/tests/support/corpus_passwords.rs"]
#[expect(
    dead_code,
    reason = "the references' spelling of a password is `pdf-model`'s oracle's; this gate \
              compares two of this tree's own backends and hands a password to neither"
)]
mod corpus_passwords;

/// Pixel budget per page: the number of pixels the **program** will hand a host in one raster.
///
/// It is `viewer_core::MAX_PIXELS` written out, because that is the population this gate is for
/// — every page this viewer draws whole, held to the oracle — and a budget below it makes the
/// instrument refuse a page the product does not.
///
/// **This constant read `64 << 20` with the comment "generous enough that no real page reaches
/// it", and a real page reached it.** `issue19517.pdf` is 12608x16806 at one device pixel per
/// point, 211 890 048 pixels, three times the old bound; the oracle gate next door had already
/// read that page — its `NO_RENDER_LARGER_THAN_THIS_GATES_BUDGET` measures `examples/render_at`
/// drawing it to within 0.005 of 255 of all three references — and declines to move its own copy
/// of the constant, because three *reference* renders of 212 megapixels are 2.5 GiB to hold and
/// cache. This gate spawns no reference: it draws the page twice, in its own process, at 1.03 GiB
/// peak and 22 s. So the reason that keeps the oracle's budget where it is does not reach here,
/// and what the old value bought was a page silently uncompared — which is where
/// [`NotComparable`] found it.
///
/// **What the raise changed, and it is one page at each scale.** At [`SCALE`] the page now reaches
/// the device, which refuses it by a capability — see [`REFUSED_BY_THE_DEVICE`]. At [`MAGNIFIED`]
/// the pages whose verdict can move are exactly those between the old bound and this one at that
/// scale, which is 4 194 304 to 16 777 216 pixels at [`SCALE`]; the corpus holds three pages above
/// the first of those figures, and they are `issue19517.pdf` (past this budget at 4x too, 3 390 240
/// 768 pixels), `issue14497.pdf` (agrees) and `issue12810.pdf` (refused by the device, and named in
/// [`REFUSED_BY_THE_DEVICE_AT_FOUR`]).
///
/// The standard states no ceiling this has to clear. ISO 32000-2's Annex C is informative, its
/// Table C.1 states no page extent at all, and Table 31's `/UserUnit` (§7.7.3.3) scales the unit
/// itself, so a page's size on paper is not bounded by its extent in coordinates either. This is
/// a resource budget with a reason, not a number read off a clause (trap 38).
const PIXEL_BUDGET: u64 = 1 << 28;

/// The scale everything is rendered at by default: the page's own resolution.
///
/// The same scale the reference oracle compares at, so a page named here can be looked at
/// beside the artefacts that gate already writes.
const SCALE: f32 = 1.0;

/// The one *other* scale this gate has a ratchet for: [`REFUSED_BY_THE_DEVICE_AT_FOUR`]'s.
///
/// Four times a page's own scale is where `viewer-ui` has switched coverage lanes and where a
/// frame's allocations are sixteen times a page's, so it is the population a resource refusal
/// belongs to. Any other value is a survey, exactly as before.
const MAGNIFIED: f32 = 4.0;

/// The scale to render at, which `PDFVIEWER_RASTER_SCALE` may override.
///
/// It exists for the speed half of this gate rather than the fidelity half. A GPU frame's
/// cost is dominated by a per-pixel floor and a readback (`RENDER_LIBRARY.md` section 6.1) while
/// this tree's CPU rasterisation grows with the pixels, so the ratio between them is a
/// *function of the scale* and one scale cannot say which way it runs — the same trap ADR
/// 0136 met comparing `rasterrocket` at 72 and 150 dpi. Overriding it **skips the ratchets**,
/// which are measured at the default.
fn scale() -> f32 {
    std::env::var("PDFVIEWER_RASTER_SCALE")
        .ok()
        .and_then(|value| value.trim().parse().ok())
        .filter(|scale: &f32| scale.is_finite() && *scale > 0.0)
        .unwrap_or(SCALE)
}

/// Which of raster's three coverage lanes to draw with, which `PDFVIEWER_RASTER_COVERAGE` may
/// override with `cpu`, `gpu` or `compute`.
///
/// **The default is raster's own — [`raster_gpu::Coverage::Cpu`], the scanline rasteriser with
/// the glyph atlas in front of it — and that is the lane this tree draws a *still* page with.**
/// The other two exist for magnification and for motion: nothing in the sampled `gpu` lane
/// depends on the scale, so a frame at 100× re-uses what a frame at 1× built, and `viewer-ui`
/// switches to it past `GPU_COVERAGE_MAGNIFICATION`; and `surface::lane_for` takes `compute` —
/// the device-side port of the CPU scanline — for **any moved view on a real adapter** (ADR
/// 0700), so a drag or zoom step is drawn by a lane this gate has to be able to name: otherwise
/// the lane a person sees the most motion through is judged by fixtures alone — trap 12b's exact
/// shape, one lane over — while this gate, the only instrument in the tree that puts a backend
/// beside the oracle at the corpus's scale, cannot run it.
///
/// Overriding it **skips the ratchets**, for the scale knob's reason and one more: the lanes
/// deliberately do not draw identical pixels (raster's ADR 0016 states the sampled lane's bound
/// against the exact one, and their ADR 0096 the compute lane's band), so a list of differing
/// pages is a property of the lane that produced it.
/// A value that is none of the three is a **panic** rather than a fallback: this variable's
/// whole purpose is to say which lane the numbers below came from, and a typo that quietly
/// measured the default would be a run reported as the lane it did not use.
#[expect(
    clippy::panic,
    reason = "a mistyped lane must stop the run: the alternative is a survey headed `gpu` \
              that measured the default"
)]
fn coverage() -> raster_gpu::Coverage {
    match std::env::var("PDFVIEWER_RASTER_COVERAGE")
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "" | "cpu" => raster_gpu::Coverage::Cpu,
        "gpu" => raster_gpu::Coverage::Gpu,
        "compute" => raster_gpu::Coverage::Compute,
        other => panic!("PDFVIEWER_RASTER_COVERAGE={other}: expected `cpu`, `gpu` or `compute`"),
    }
}

/// The glyph-phase quantum this run draws with, which `PDFVIEWER_RASTER_GLYPH_QUANTUM` may
/// override with `off` or a bucket count.
///
/// **The default is [`render_raster::options`]'s, so the gate draws what the viewer draws.** It
/// was `None` for the whole of this gate's life, on the reasoning that a quantum-off run isolates
/// the backend's fidelity from a sub-1/32-pixel trade `real_pages.rs` gates separately — and that
/// reasoning is what let a whole-pixel misplacement live in the shipped configuration where the
/// 974-page instrument could not reach it. raster's ADR 0073: `(fx · q).round()` reaches `q`
/// itself on 3.1% of sub-pixel phases per axis, and the `% q` that followed seated the mark in
/// the *previous* pixel. The only gate that could see it was an envelope over a mean, a worst
/// tile and an SSIM, which a 3% population of whole-pixel errors moves without breaking. So the
/// default is the shipped setting and the isolation run is the override, not the other way round
/// (ADR 0498).
///
/// Overriding it **skips the ratchets**, for the same reason the scale and coverage knobs do: the
/// lists below are measured at one setting, and a list held to equality under another would report
/// that setting's own difference as a change in this backend. That the two settings currently
/// *agree* page for page is a measurement rather than a property — it is what ADR 0498 ran — and
/// a gate that assumed it would be asserting the very thing it exists to watch.
///
/// A value that is neither `off` nor a positive bucket count is a **panic**, for
/// [`coverage`]'s reason: a typo that quietly measured the default would be a run reported as the
/// setting it did not use.
#[expect(
    clippy::panic,
    reason = "a mistyped quantum must stop the run: the alternative is a survey headed \
              `quantum off` that measured the shipped one"
)]
fn glyph_quantum() -> Option<u16> {
    let Ok(value) = std::env::var("PDFVIEWER_RASTER_GLYPH_QUANTUM") else {
        return render_raster::options().glyph_quantum;
    };
    match value.trim() {
        "" => render_raster::options().glyph_quantum,
        "off" | "none" => None,
        other => Some(
            other
                .parse::<u16>()
                .ok()
                .filter(|q| *q > 0)
                .unwrap_or_else(|| {
                    panic!(
                        "PDFVIEWER_RASTER_GLYPH_QUANTUM={other}: expected `off` or a positive \
                         bucket count"
                    )
                }),
        ),
    }
}

/// How a quantum reads in a line a person reads.
fn quantum_name(quantum: Option<u16>) -> String {
    quantum.map_or_else(|| "off".to_owned(), |q| format!("1/{q}"))
}

/// How many threads raster's geometry phase may use for this run, which
/// `PDFVIEWER_RASTER_ENCODE_THREADS` may override.
///
/// **The default is [`render_raster::options`]'s, so the gate draws what the viewer draws** —
/// and the override exists because the property that matters about this number is that it
/// changes *nothing*. raster states a frame's result byte-identical at any thread count (their
/// ADR 0054); this gate over 956 real pages carrying clip chains, groups, masks and atlas
/// pressure is the evidence for that claim on *our* side of the boundary, and it is only evidence
/// if the same corpus can be run at 1 and compared line by line against the same corpus at the
/// host's number. ADR 0377 is that comparison. Overriding it does **not** skip the ratchets: a
/// thread count that moved a verdict is precisely what this gate should fail for.
fn encode_threads() -> usize {
    std::env::var("PDFVIEWER_RASTER_ENCODE_THREADS")
        .ok()
        .and_then(|value| value.trim().parse().ok())
        .filter(|threads: &usize| *threads > 0)
        .unwrap_or_else(|| render_raster::options().encode_threads)
}

/// How far a page may sit from the oracle before it is counted as differing.
///
/// Three numbers because they catch different failures, and they are `real_pages.rs`'s own
/// gates: the mean catches a page-wide shift, the worst tile catches a mark that is missing
/// or in the wrong place, and the structural similarity catches a page that has the right ink
/// in the wrong shapes. A page failing any of them is listed.
const MAX_MEAN_ERROR: f64 = 1.5;
/// See [`MAX_MEAN_ERROR`].
const MAX_WORST_TILE_ERROR: f64 = 7.0;
/// See [`MAX_MEAN_ERROR`].
const MIN_STRUCTURAL_SIMILARITY: f64 = 0.99;

/// Documents whose first page is refused **before the scene is built**, by name.
///
/// This is the first of the two stages a refusal can happen at, and the two are split because a
/// name leaving one array meant two unrelated things while they shared it: these refusals are
/// `render-raster`'s own, raised while translating a [`DisplayList`] into a
/// `raster_scene::Scene`, so they hold **at every scale** and no upstream release can move one.
/// [`REFUSED_BY_THE_DEVICE`] and [`REFUSED_BY_THE_DEVICE_AT_FOUR`] are the other stage.
///
/// Held to equality in both directions: a page arriving here is a new hole in the backend,
/// and a page leaving it is a hole closed. The reason each one gives is printed by the run.
///
/// **`bug1721218_reduced.pdf` is a group compositing in four components.** Its whole artwork is
/// one isolated `/CS /DeviceCMYK` group, which the oracle composites in ink as a pair of element
/// lists resolved per pixel at its `Do` (§11.6.6, §11.7.2, ADR 0327). The page-level pair is two
/// whole `Target::Readback` renders put together by `pdf_render::blending`; a scene under
/// composition cannot be read back, and a `raster_scene::GroupSpec` carries no conversion to run
/// over a group's composited result, so the group-scoped pair has no lane. The page also meets the
/// adapter's 16384 × 16384 coverage sheet, a ceiling this refusal preempts rather than fixes.
///
/// **`issue16742.pdf` and `issue5044.pdf` are a group compositing in three CIE-based
/// components** (ADR 0797): an isolated group whose `/CS` is a `CalRGB` or an RGB profile, drawn by
/// the oracle in the space's own components and resolved through a cube — curves, a grid and the
/// device's transfer function — per pixel at its `Do`. The page-level cube is one pass over a
/// whole readback and has no group-scoped analogue here, for the reason the pair has none.
///
/// **`issue21346.pdf` is a luminosity mask whose group composites in an sRGB profile's
/// components**, and §11.5.3's `Y` of that is three curves summed per pixel
/// (`pdf_render::Luminance`), where `raster_scene::MaskKind::Luminosity` weighs the channels in its
/// own shader — a different formula, refused rather than drawn to the wrong mask.
///
/// All four are `doc/QUORRA_FEEDBACK.md` section 43's: a `GroupSpec` carrying a conversion to run
/// over the group's composited result (curves on either side of an N-axis grid), a second body for
/// the four-component shape, and a curves-or-grid field beside the luminosity mask's backdrop.
/// raster's vocabulary has grown none of the three, so none of the four can leave. Each frame goes
/// to the CPU backend, which draws them; `headless_quorra.rs` holds the group refusal against the
/// cross-backend scene.
///
/// **What a departure from *this* list means, which is why it is no longer mixed with the
/// device's.** A name arriving is a construction the CPU oracle states and this translation
/// cannot, and the round that adds one owes `doc/QUORRA_FEEDBACK.md` the ask; a name leaving is
/// a construction raster's vocabulary grew a way to express, which is the only thing that has
/// ever taken one off. What a departure here is **not** is a statement about the adapter: this
/// stage never reaches a device, so a name here is unmoved by a texture ceiling, a byte budget
/// or a driver. And because the stage is scale-free, this one array is what both scales are held
/// to — so a second copy of these names living in the 4× list and going stale while nobody runs
/// that lane cannot happen.
const REFUSED_BEFORE_THE_SCENE: [&str; 4] = [
    "bug1721218_reduced.pdf",
    "issue16742.pdf",
    "issue21346.pdf",
    "issue5044.pdf",
];

/// Documents whose first page **the device** refuses at [`SCALE`], by name.
///
/// The second of the two stages. A refusal here is raised inside raster at render time, against
/// a capability or a budget the adapter states: a page that translated into a scene, went to
/// the device, and could not be drawn there. `issue1905.pdf` is not here: its 29 large fills
/// wind more than two values, so ADR 1389's `Encoder::compute_takes` sends them to the scratch
/// lane, which charges a tile's area and not the compute lane's accumulator of four bytes a pixel
/// besides — 252594693 scene-derived bytes against the budget of 268435456, where the compute lane
/// asked 527497181 (`doc/QUORRA_FEEDBACK.md` section 40). At 4× it is refused on the scene-byte
/// budget, in [`REFUSED_BY_THE_DEVICE_AT_FOUR`].
///
/// **What a departure from this list means.** A name arriving is a page a person could open at
/// 100% and not see: it is raster's to move, or this adapter's, and the round that finds one
/// reports it upstream with the message the run prints rather than adding a construction here.
/// A name leaving is an allocation, a tiling or a ceiling that changed upstream, which is
/// exactly what [`REFUSED_BY_THE_DEVICE_AT_FOUR`]'s own note records happening twice.
///
/// **`ContentStreamCycleType3insideType3.pdf` is the exception to "raster's to move", and it is
/// this tree's by design.** It is a cycle — a `d1` glyph whose pattern's cell shows the glyph —
/// and until ADR 0793 it was refused at a nesting depth of sixteen the moment its cell ran,
/// drawing nothing. The bound is sixty-four now and counts the cell, so the cycle is entered
/// sixty-four deep, every level is nine copies of the one below it, and the page spends the whole
/// of `MAX_OPERATIONS` on it: 3 995 603 commands, which raster prices at *383577888
/// scene-derived bytes, over the stated budget of 268435456*. The CPU backend draws the page and
/// says so. Nothing here is an upstream ask — a budget that refuses four million commands of a
/// cycle is doing what a budget is for — and what would take the name off is `doc/todo/49`'s
/// standing item, a bound on the interpreter's *work* rather than on its count.
///
/// **`issue19517.pdf` is here because of [`PIXEL_BUDGET`], not because of the page or the
/// adapter.** The page is 12608x16806 and this adapter
/// states 16384 pixels per side, so the frame is refused with *target 12608x16806 exceeds this
/// adapter's limit of 16384 pixels per side* — a capability, like `issue1905.pdf`'s sheet ceiling
/// and unlike the cycle's budget. It is this list's own sentence exactly: a page a person could
/// open at 100% and not see. The CPU backend draws it and says so, which is `CLAUDE.md`
/// principle 2's rule for a refusal, and `doc/QUORRA_FEEDBACK.md` section 44 carries the message
/// upstream. What had been hiding it is that this gate's budget refused the page three stages
/// earlier and counted the result as one of seventeen anonymous *not comparable*.
///
/// **And the cycle's message is a function of what the run drew before it, which is worth knowing
/// before quoting one.** Run alone, `ContentStreamCycleType3insideType3.pdf` is refused for
/// *frame needs 377221152 scene-derived bytes, over the stated budget of 268435456*; run in its
/// place in the corpus it is refused for *uploading would hold 536871684 resource bytes
/// (536870452 already resident), over the stated budget of 536870912* — one `QuorraRasterizer`
/// serves all 974 documents, so by the time the walk reaches this page the resource cache is at
/// its ceiling and the ceiling it meets first is that one. Both are budgets, both are reported
/// out loud, and the name is on this list either way; what a round may not do is read the printed
/// figure as this page's own cost.
const REFUSED_BY_THE_DEVICE: [&str; 2] =
    ["ContentStreamCycleType3insideType3.pdf", "issue19517.pdf"];

/// The same at [`MAGNIFIED`], which is the population the zoom path actually draws.
///
/// **This ratchet exists because the number it holds finally stopped moving.** At four times a
/// page's own scale this run has never been a gate: `doc/todo/02-every-round.md` section 2 makes
/// it a run every round that takes a raster release owes, and until now what came back was a survey
/// nobody could fail — twelve refusals at one revision, seven at the next, each release taking
/// some off and each count living in a document rather than in a test. What made them uncheckable
/// was that most of them were *arithmetic against a byte budget* that upstream kept improving:
/// four pages over the 256 MiB frame budget by 4 % to 20 % is a list that moves the moment
/// anything is allocated more tightly, and raster's ADRs 0036 to 0039 allocated every plan, mask
/// and root to what it marks. **What that allocation left refused at this scale is four pages, and
/// two of them are over the frame's scene-byte budget by far more than any allocation reaches**:
/// `issue1905.pdf` at *365144861* and `issue12810.pdf` at *609086160 scene-derived bytes*, each
/// over 268435456, beside `issue9418.pdf`'s coverage sheet and the cycle's resource bytes — a run
/// of this lane over the four names and the two that left, taken after ADR 0702 rebuilt the scene
/// in page space and [`PIXEL_BUDGET`]'s raise admitted the second page. The two bullets below are
/// what left and what stayed:
///
/// - **`22060_A1_01_Plans.pdf` left this list, and what took it off was neither a larger budget
///   nor a tighter allocation (ADR 0374).** It held 522 014 748
///   resident *resource* bytes with the next upload taking it to 548 104 348 against the
///   536 870 912 `max_resource_bytes` default — because its page drew **72 sampled images that
///   were 8 distinct rasters**, one allocation per `Do`, `image::decode_parts` having run at every
///   one of them. `image::RasterCache` (ADR 0374) made the repeated `Do`s share the raster they
///   always meant, and the same page now uploads what it actually holds. The lesson is the one
///   this comment already argued from the other end: a refusal that is arithmetic against a byte
///   budget is a question about who is spending the bytes, and this time the answer was upstream
///   of the backend entirely. Nobody raised `max_resource_bytes`, and a budget raised to admit one
///   page would still be a budget chosen by that page.
/// - `issue1905.pdf` exceeded the **16 384 × 16 384 texture** this adapter allows for the
///   rasterised-coverage sheet, and what it prints at this scale is the scene-byte budget —
///   *frame needs 365144861*, over 268435456 — so the ceiling below is a reading the byte budget
///   preempts rather than one a run still shows; at [`SCALE`] it is drawn (ADR 1389).
///   (`issue9418.pdf`, added above, is the page that prints it today.) That ceiling is a device
///   capability, not a policy. Raster measured a
///   multi-sheet fix at its `5483996` and **declined it with the numbers written down**: a second
///   sheet takes this page to 287 MB — refused again, on bytes — and its neighbours to a
///   quarter-gigabyte of per-frame upload, a page drawn at a cost its own brief calls a failure.
///   The ceiling that bites is that a clipped shape becomes one coverage tile of its own device
///   bounds; the open work upstream is on the tiling side. Its *message* changed with the release
///   this list last moved on: the refusal now names the sheet it met rather than only the
///   adapter's wall (raster's ADR 0057 decision 2), which is that ADR's other half.
///
///   **`bug1703683_page2_reduced.pdf` was here for the same reason and left by a run (ADR
///   0411).** raster's ADR 0057 sizes a clipped mark's
///   coverage tile by its chain's own bounding box instead of by the open clip rectangle, and its
///   section 1 measures this page asking 1 008 561 911 texels where its 141 chains admit 2 297 897. That
///   landed on their `cafadeb`, which `cad50156` carries; at 4× on this lane the page now **agrees
///   with the CPU oracle**. A name told it may come off stays on until a run shows it (ADR 0402
///   decision 3), and that is `CLAUDE.md` principle 5 one boundary over: a report from another
///   implementation is evidence about *that* implementation, and a ratchet held by name exists so
///   that a name comes off by a run rather than by a message.
///
/// **[`REFUSED_BEFORE_THE_SCENE`]'s names are not here, and that is what the split is for.**
/// Both stages in one list would mean a name leaving it could be a device that grew a capability
/// or a translation that grew a construction, and the ratchet could not say which; it would also
/// write this tree's own refusals down **twice**, once per scale, and a second copy goes stale
/// while no round runs the 4× lane. Those names are one scale-free array and this one holds only
/// what the device refuses. (`bug1721218_reduced.pdf` meets the sheet ceiling too; it is refused
/// before it can reach it, which is why it is not also named here.)
///
/// **What a departure from this list means.** A name arriving is a hole that only opens under
/// magnification — a page a person can open and not zoom into — and it is raster's or this
/// adapter's rather than ours, so the round that finds one carries the message upstream. A name
/// leaving is that hole closed upstream, which is what both departures recorded above were. It
/// is checked on the default lane only, because the two lanes put *different tiles* in the
/// coverage sheet this refusal is against — the encoder chooses per command (raster's ADR 0029)
/// — so the sheet a frame commits is a property of the lane, and a lane's refusals are its own.
///
/// `ContentStreamCycleType3insideType3.pdf` is here for [`REFUSED_BY_THE_DEVICE`]'s reason and
/// with the same message, measured on this lane (ADR 0793): the scene-byte budget is scale-free,
/// and four million commands are over it at any scale.
///
/// **Two are here because of [`PIXEL_BUDGET`]'s raise and not for anything the device did**, and
/// they are the two pages this scale's *budget* would otherwise refuse before the device could
/// answer. `issue12810.pdf` is 6912x10368 here and prices at *609086160
/// scene-derived bytes, over the stated budget of 268435456*. `issue9418.pdf` is 111 476 736
/// pixels here and meets the sheet: *a 4541x2842 tile would not fit a sheet at 13841x13561
/// holding 122 tiles and 143672336 texels* — a capability, and the same ceiling `issue1905.pdf`
/// used to print. `issue19517.pdf` is **not** here: it is past this budget at 4x as well, 3 390
/// 240 768 pixels, so this scale is where [`NotComparable::PastThePixelBudget`] is not empty.
/// `issue14497.pdf`, the third page the raise admits at this scale, agrees with the oracle.
///
/// **And `issue9418.pdf` is why a population is derived from the instrument that will run it.**
/// The page was found by re-running this whole lane, not by the sweep that was supposed to have
/// predicted it: `pdfinfo` over the corpus answered for 953 of 974 documents, the 21 silences
/// were read as the documents that do not open, and this was not one of them (trap 25). The
/// bound, the page and the adapter were all unchanged; only the list was wrong.
const REFUSED_BY_THE_DEVICE_AT_FOUR: [&str; 4] = [
    "ContentStreamCycleType3insideType3.pdf",
    "issue12810.pdf",
    "issue1905.pdf",
    "issue9418.pdf",
];

/// The stage-free refusals and the device's, as one list sorted the way the run produces them.
///
/// Both halves are held to equality together, because what a run has is one list of names: the
/// split is about what a *departure* means, not about running two comparisons.
fn refused_pages(by_the_device: &[&'static str]) -> Vec<&'static str> {
    let mut all: Vec<&'static str> = REFUSED_BEFORE_THE_SCENE
        .iter()
        .chain(by_the_device)
        .copied()
        .collect();
    all.sort_unstable();
    all
}

/// Documents this gate could not compare at all, by name and by which of the causes it was.
///
/// Held to equality in both directions, like the two refusal lists above and for the same
/// reason: this population is the one place a page can leave the comparison without any of the
/// three figures being computed about it, so a page arriving here unannounced is a page nobody
/// is holding either backend to.
///
/// **Every name here is already read against the standard, one gate over.** `pdf-model`'s
/// `tests/oracle.rs` walks the same corpus and reached exactly these documents by exactly these
/// routes, and ADR 0410 names its buckets; the
/// reading lives there and is not copied here, because two documents stating one fact is how the
/// two drift. (`pdf-model`'s own `tests/corpus.rs` bounds the same two populations, above
/// `MAX_UNREADABLE_ENCRYPTION` and `MAX_PAGELESS`, and reads each document one at a time.) The
/// mapping is one-to-one:
///
/// - the two [`NotComparable::WouldNotOpen`] are its `NO_RENDER_NEEDS_A_PASSWORD`
///   (`encrypted-attachment.pdf`, whose user password is not the empty one §7.6.4.3 defines and
///   is published nowhere) and its `NO_RENDER_ENCRYPTION_THE_STANDARD_DOES_NOT_STATE`
///   (`PDFBOX-4352-0.pdf`, whose fuzzed cross-reference table leaves the `/Encrypt` §7.6.2
///   names resolving to nothing). The nine encrypted documents whose passwords are published
///   are opened with them — `pdf-model`'s `tests/support/corpus_passwords.rs` is the one table
///   every corpus gate reads, [`page_one`] among them — and each compares like any other page
///   (ADR 1377);
/// - the five [`NotComparable::NoFirstPage`] are its `NO_RENDER_NO_PAGE_IN_THE_TREE` less the one
///   entry that names a *second* page, which this gate never asks for.
///
/// So what is owed here is not a second diagnosis but the names, and the case each falls in is
/// `CLAUDE.md` principle 5's second: the file broke it, and a rasteriser comparison is not what
/// any of them is waiting on.
///
/// **Four of the six causes are empty at this scale and all four are kept.**
/// [`NotComparable::TheOracleRefused`] being empty is a *measurement* rather than an omission:
/// `CLAUDE.md` principle 2 makes the CPU backend the one that draws what the device refuses, and
/// this run is where that is checked over a corpus — every page of these 974 whose display list
/// exists and fits [`PIXEL_BUDGET`] was drawn by it, including all three the device refused.
/// [`NotComparable::PastThePixelBudget`] is empty here and not at [`MAGNIFIED`], where
/// `issue19517.pdf` is past it; [`NotComparable::Unreadable`] and
/// [`NotComparable::RastersCannotBeCompared`] are the other two silences. An empty cause is a claim
/// about this tree made by an instrument nobody has watched work, so each of the four was planted
/// in turn and the run named each one it was given (trap 13).
const NOT_COMPARABLE: [(&str, NotComparable); 7] = [
    ("Brotli-Prototype-FileA.pdf", NotComparable::NoFirstPage),
    ("PDFBOX-4352-0.pdf", NotComparable::WouldNotOpen),
    ("REDHAT-1531897-0.pdf", NotComparable::NoFirstPage),
    ("bug1020226.pdf", NotComparable::NoFirstPage),
    ("encrypted-attachment.pdf", NotComparable::WouldNotOpen),
    ("poppler-85140-0.pdf", NotComparable::NoFirstPage),
    ("poppler-937-0-fuzzed.pdf", NotComparable::NoFirstPage),
];

/// [`NOT_COMPARABLE`] as the run produces it: sorted by name, owned.
fn not_comparable_pages() -> Vec<(String, NotComparable)> {
    let mut all: Vec<(String, NotComparable)> = NOT_COMPARABLE
        .iter()
        .map(|(name, why)| ((*name).to_owned(), *why))
        .collect();
    all.sort_unstable();
    all
}

/// Pages where the two rasterisers differ only at the **edges** of what they draw.
///
/// Structural similarity above 0.99 — `raster_compare`'s own vector threshold — is the
/// statement that the same shapes are in the same places, so what is left is coverage at a
/// boundary. **This group is the floor, not a defect list** — what would make it one is a page
/// arriving in it whose similarity is high because the difference is uniform. The pages that have
/// left it, each by one backend moving onto the geometry the other already drew, are recorded in
/// `doc/adr/` and `doc/history/`.
///
/// **`issue2177.pdf` is two analytic answers rather than a quantum** (ADR 1082 on the oracle,
/// raster's ADR 0049 on the device). Its page is three clipped circles filled with a tiling
/// pattern of small coloured ellipses, so almost every inked pixel is somebody's curve boundary;
/// `examples/ink_ladder` puts
/// the two backends 0.74% apart at 1× and **0.17% apart at 8×**, and the excess shrinks at every
/// rung, which is that instrument's signature for a per-boundary cost rather than a shape. Both
/// backends read heavier at the page's own scale than at eight times it — ours by 0.68% and
/// raster's by 0.11% — which is the side §10.7.4's "[t]he area covered by painted pixels shall
/// always be at least as large as the area of the original shape" asks for. Flattening is not the
/// difference and that is measured rather than assumed: at tolerances of 1/16, 1/64, 1/256 and
/// 1/1024 of a device pixel the page's ink reads 13002.05, 13022.88, 13030.50 and 13031.90, so the
/// converter's own tolerance is within 0.011% of its limit. The worst tile the gate prints is at
/// (32, 224), which is the raster's own bottom row and one pixel tall — trap 26, and the verdict
/// here rests on the differing fraction.
const DIFFERS_AT_THE_EDGES: [&str; 1] = ["issue2177.pdf"];

/// Pages where the difference is **structural**: similarity at or below 0.99.
///
/// The name is the *classifier's*, and every page here has been examined; none is an open
/// question. Each is below with the clause that says which backend is right and the ask that
/// would take it off. The pages that left, and why, are the ADRs this list cites, ADR 1361 for
/// the strokes, and `doc/history/`; what is written here is only what holds for the one that stays.
///
/// **`issue19083.pdf` is a clip taken as a product at a coincident boundary.** It is one widget
/// appearance whose `/BBox` is `[0 0 125.25 20]` and whose whole content is `0.5 0.5 124.2502 19
/// re s` at the default `1 w` — a border rule whose outer edge sits 0.0002 *outside* the `/BBox`
/// §8.10.1 step c) clips it by, so the clip genuinely cuts and ADR 1088's `cuts_nothing` rule
/// declines it by name. §10.7.4 asks for "the intersection of the set of pixels defined by the
/// clipping region with the set of pixels for the region to be painted"; `render-cpu` composes the
/// two with `min` (ADR 0355, ADR 0535) and raster multiplies them inside the graphics library,
/// which squares a mark's coverage where its own edge and its clip's share a pixel.
/// `examples/ink_ladder` reads raster 396.58 at 1× against the oracle's 448.33 and level with it by
/// 4×, the signature of a per-boundary cost. The oracle is the side the clause states, and
/// `doc/QUORRA_FEEDBACK.md` section 24c is the ask.
const DIFFERS_IN_SHAPE: [&str; 1] = ["issue19083.pdf"];

/// The two groups as one list, sorted as the run produces them.
///
/// # The split that matters is not the one the two arrays make
///
/// Both arrays above are cut by *structural similarity*, which is a classifier's line rather than
/// a cause's: every figure this gate prints compares pixel against pixel at one scale, and two
/// unrelated faults read the same there — the same ink in different places, and different amounts
/// of ink. Only the second is something a clause arbitrates, and `examples/ink_ladder.rs` is what
/// separates them: each backend's total ink at 1×, 2×, 4× and 8×, scale-normalised, so that the
/// geometry the page states is the reference rather than either backend. Run it over this list
/// before reading a page's three numbers as a defect.
///
/// **A page whose two totals agree at every rung has nothing missing and nothing mis-sized**, and
/// what is left is where each rasteriser puts a boundary. ISO 32000-2 §10.7.4 says which grid a
/// shape's coordinates are *not* snapped to:
///
/// > Its coordinates are mapped into device space but not rounded to device pixel boundaries.
///
/// **`issue2177` is what this and the next two paragraphs read**; the pages that left the list when
/// the two backends took one substitution are ADR 1102's. The rest part on the ladder and are read
/// from `issue19083` on.
///
/// **Neither backend snaps a path's edges to a lattice, and that is measured rather than assumed.**
/// `render_cpu::area` computes the coverage this subclause's own definition of a pixel implies
/// (ADR 1082), and `examples/coverage_lattice` puts both backends at the chance its run prints,
/// 25.0% for a band of ±1.5 levels: `endchar` 14.2% against 14.2%, `pr12564` 17.0% against 17.5%,
/// `standard_fonts` 15.5% against 14.2%, `issue11473` 15.5% against 20.0%, `issue12295` 21.3%
/// against 10.8%.
///
/// **raster's own quantum is a glyph *phase* rather than a coverage, and it holds nothing here.**
/// This gate draws at [`render_raster::options`]'s `glyph_quantum: Some(16)`, so a glyph's origin
/// is bucketed to a sixteenth of a pixel (ADR 0498). `PDFVIEWER_RASTER_GLYPH_QUANTUM=off` over the
/// whole corpus prints the same sixteen names: the bucket is worth up to 0.32 of 255 of mean error
/// — `pr12564` 0.6792 → 0.3632, `issue18030` 1.7311 → 1.4812, `standard_fonts` 1.5342 → 1.4888 —
/// and takes no page off the list. The control is in the same run: five of the sixteen do not move
/// by a digit (`issue15150`, `issue16038`, `issue21068`, `issue2177`, `issue269_2`), so the knob
/// reached what it names rather than everything.
///
/// **A page whose totals part is the other shape**, and the way the gap moves along the ladder
/// says what it is. A gap that halves at every rung is a cost paid per boundary pixel, and one
/// page here is still made of that: `issue19083.pdf` reads raster 396.58 at 1× against the
/// oracle's 448.33 and is level with it by 4×, which is ADR 0355's clip-against-the-mark product
/// measured in ink instead of in pixels — §10.7.4 asks for "the intersection of the set of pixels
/// defined by the clipping region with the set of pixels for the region to be painted", and a
/// product at a coincident boundary takes ink an intersection does not. It is the page that is
/// left because its stroke's outer edge sits 0.0002 *outside* the `/BBox` §8.10.1 step c) clips
/// it by, so the clip genuinely cuts and ADR 1088's rule declines it by name;
/// `doc/QUORRA_FEEDBACK.md` section 24c is the standing ask, and it is where this measurement
/// is written down.
fn differing_pages() -> Vec<&'static str> {
    let mut all: Vec<&'static str> = DIFFERS_AT_THE_EDGES
        .iter()
        .chain(&DIFFERS_IN_SHAPE)
        .copied()
        .collect();
    all.sort_unstable();
    all
}

/// One document's outcome.
enum Outcome {
    /// Rendered by both, within tolerance.
    Agrees,
    /// Rendered by both, outside it.
    Differs(String),
    /// raster refused the display list.
    Refused(String),
    /// Nothing to compare, and which of the six things stopped the comparison.
    NotComparable(NotComparable),
}

/// Why a document produced no comparison at all.
///
/// This gate printed one number for all of these — *"17 not comparable"* — for the whole of its
/// life, and the number is the sum of six unrelated facts: three about the file, one about this
/// gate's own budget, one about the **CPU oracle**, and one about the two rasters. A bare count
/// over six causes cannot move without a round guessing which of them moved, and one of the six
/// is a hole in the correctness oracle itself, which `CLAUDE.md` principle 2 makes the backend
/// that draws what the device refuses. The oracle gate next door named its own `not comparable`
/// bucket for this reason (ADR 0414); this is the
/// same reading one gate over, and [`NOT_COMPARABLE`] holds the result by name.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum NotComparable {
    /// The bytes would not read off the disk.
    Unreadable,
    /// [`Document::open`] refused the bytes: there is no document to draw a page of.
    WouldNotOpen,
    /// The document opened and its page tree yielded no first page.
    NoFirstPage,
    /// The page's own raster at this scale is past [`PIXEL_BUDGET`].
    PastThePixelBudget,
    /// The **CPU oracle** refused the display list, so there is nothing to hold raster to.
    TheOracleRefused,
    /// Two rasters were produced and `raster_compare` refused the pair.
    RastersCannotBeCompared,
}

impl NotComparable {
    /// What to call it in a line a person reads and in the list held by name.
    const fn as_str(self) -> &'static str {
        match self {
            Self::Unreadable => "unreadable",
            Self::WouldNotOpen => "would not open",
            Self::NoFirstPage => "no first page",
            Self::PastThePixelBudget => "past the pixel budget",
            Self::TheOracleRefused => "the oracle refused",
            Self::RastersCannotBeCompared => "rasters not comparable",
        }
    }
}

/// Fails the gate if this build cannot reach the sandboxed image decoder.
///
/// `CCITTFaxDecode`, `JBIG2Decode` and `JPXDecode` are decoded by a separate program, and Cargo
/// does not build another package's binaries when it tests this one (trap 10). A build without
/// it draws every other image and none of those three, so what follows would be a measurement of
/// the build rather than of the tree — which is exactly what moved the accessibility census's
/// ratchet by nine elements while four rounds read the difference as something else
/// (ADR 0557, trap 16).
#[expect(
    clippy::panic,
    reason = "a gate that cannot decode the images it is measuring must stop rather than \
              print a number about a different program"
)]
fn require_the_sandbox() {
    if let Err(error) = pdf_model::image::sandboxed_decoder() {
        panic!(
            "the sandboxed image decoder is not available, so the counts below would be \
             wrong: {error}"
        );
    }
}

#[test]
#[ignore = "renders 974 pages twice; run explicitly"]
fn every_corpus_page_agrees_with_the_cpu_oracle() {
    require_the_sandbox();
    let Some(files) = corpus() else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };
    let settings = Settings::from_environment();
    let scale = settings.scale;
    let only = std::env::var("PDFVIEWER_RASTER_ONLY").ok();
    let files = selected(files, only.as_deref());

    let mut raster = settings.rasterizer();
    announce(&raster, files.len(), settings);
    let mut one_thread = OneThread::beside(settings);

    let started = Instant::now();
    let mut agreed = 0usize;
    let mut incomparable: Vec<(String, NotComparable)> = Vec::new();
    let mut differing = Vec::new();
    let mut refused = Vec::new();
    let mut worst: Vec<(f64, String)> = Vec::new();
    let (mut cpu_total, mut gpu_total) = (Duration::ZERO, Duration::ZERO);
    let mut ratios: Vec<f64> = Vec::new();
    let mut times = page_times();

    for path in &files {
        let name = path.file_name().map_or_else(
            || path.display().to_string(),
            |n| n.to_string_lossy().into(),
        );
        let (list, target, cpu, cpu_took) = match oracles_render(path, scale) {
            Ok(ready) => ready,
            Err((why, detail)) => {
                println!("  not comparable: {name}: {}{detail}", why.as_str());
                incomparable.push((name, why));
                continue;
            }
        };
        let at = Instant::now();
        let ours = raster.rasterize(&list, target);
        let gpu_took = at.elapsed();
        one_thread.compare(&name, &list, target, &ours);
        let verdict = outcome(&cpu, &ours);
        // **A refused frame is a fast frame**, and counting one as a time would report a
        // backend that draws nothing as the quickest there is: at four times the page's own
        // scale, 533 of these documents are refused and the median ratio came back as 0.00×
        // before this line existed. Only a frame that was produced is timed.
        if !matches!(verdict, Outcome::Refused(_)) {
            cpu_total = cpu_total.saturating_add(cpu_took);
            gpu_total = gpu_total.saturating_add(gpu_took);
            if let Some(times) = times.as_mut() {
                // A file that cannot be written loses the column, not the run: the survey above
                // it is the gate, and this is a side output for comparing two builds page by page.
                let _ = writeln!(
                    times,
                    "{name}\t{:.3}\t{:.3}",
                    cpu_took.as_secs_f64() * 1e3,
                    gpu_took.as_secs_f64() * 1e3
                );
            }
            if cpu_took > Duration::from_millis(1) {
                // Below a millisecond the clock is measuring itself; a ratio taken there is
                // noise with a decimal point.
                ratios.push(gpu_took.as_secs_f64() / cpu_took.as_secs_f64());
            }
        }

        match verdict {
            Outcome::Agrees => agreed = agreed.saturating_add(1),
            Outcome::Differs(how) => {
                // The frame judged, not a second one: a redraw is another frame on the retained
                // atlas, which the one-threaded backend beside this one would not have drawn.
                write_artefacts(&name, &cpu, &ours);
                let mean = how
                    .split_once("mean ")
                    .and_then(|(_, rest)| rest.split_whitespace().next())
                    .and_then(|value| value.parse::<f64>().ok())
                    .unwrap_or(0.0);
                worst.push((mean, name.clone()));
                differing.push(name.clone());
                println!("  differs: {name}: {how}");
            }
            Outcome::Refused(why) => {
                refused.push(name.clone());
                println!("  refused: {name}: {why}");
            }
            Outcome::NotComparable(why) => {
                println!("  not comparable: {name}: {}", why.as_str());
                incomparable.push((name, why));
            }
        }
    }

    incomparable.sort_unstable();
    report(
        &mut Tally {
            agreed,
            incomparable: &incomparable,
            differing: &differing,
            refused: &refused,
            worst: &mut worst,
        },
        (cpu_total, gpu_total, &mut ratios),
        started.elapsed(),
    );

    hold(
        ratchets(files.len(), settings, only.is_some()),
        &refused,
        &differing,
        &incomparable,
    );
    one_thread.hold();
}

/// The same backend at one encode thread, beside a run that fans out, and every page whose bytes
/// the thread count moved.
///
/// **A frame is a function of the display list and the view**, never of how many threads encoded
/// it: raster states it (their ADR 0054), `CLAUDE.md`'s oracle comparison rests on it, and trap 66
/// is how it was lost — thirteen pages of this corpus drew differently at one thread and at
/// twenty-four while every gate passed, because no gate ran both. So this one does, inside the
/// walk rather than as a second walk: the page is already parsed and the list already built, and a
/// second raster at one thread is all the comparison costs. Held to **empty** at every scale, lane
/// and quantum, and under a filter too — equality to nothing is the one list a subset cannot
/// misreport. Each backend keeps its atlas from page to page, so the two must draw the same frames
/// in the same order, a refused frame included ([`OneThread::compare`], ADR 1419); ADR 1407
/// section 1 is the encoder's half of the same property.
struct OneThread {
    /// `None` when the run under test is itself one-threaded, so there is nothing to compare.
    raster: Option<QuorraRasterizer>,
    /// The pages whose bytes differ, or which one thread refused and many drew.
    differing: Vec<String>,
}

impl OneThread {
    /// A one-threaded backend configured as `settings` otherwise is, when `settings` fans out.
    fn beside(settings: Settings) -> Self {
        let raster = (settings.threads > 1).then(|| {
            Settings {
                threads: 1,
                ..settings
            }
            .rasterizer()
        });
        if raster.is_none() {
            println!("one encode thread: the thread-count comparison has nothing to compare");
        }
        Self {
            raster,
            differing: Vec::new(),
        }
    }

    /// Draws the page again at one thread and records it when the bytes are not the same, or when
    /// one of the two refused what the other drew.
    ///
    /// **Drawn whatever the fanned-out backend answered, a refusal included.** Each backend keeps
    /// its atlas from page to page, and a frame refused part way through its encode has already
    /// committed the atlas inserts before the refusing charge — so a page drawn by one backend and
    /// skipped by the other leaves the next page on two different atlases, and every page after it
    /// differs by the retained state rather than by the thread count (ADR 1419). The two histories
    /// are the same frames in the same order, or the comparison compares histories.
    fn compare(
        &mut self,
        name: &str,
        list: &DisplayList,
        target: TargetSpec,
        ours: &Result<pdf_render::Raster, impl ToString>,
    ) {
        let Some(raster) = self.raster.as_mut() else {
            return;
        };
        match (raster.rasterize(list, target), ours) {
            (Ok(serial), Ok(ours)) if serial.data == ours.data => {}
            (Err(serial), Err(ours)) if serial.to_string() == ours.to_string() => {}
            (Ok(serial), Ok(ours)) => {
                let row = usize::try_from(serial.width).map_or(1, |w| w.saturating_mul(4).max(1));
                let (mut bytes, mut most) = (0usize, 0u8);
                let (mut left, mut top, mut right, mut bottom) = (usize::MAX, usize::MAX, 0, 0);
                for (at, (a, b)) in serial.data.iter().zip(&ours.data).enumerate() {
                    if a != b {
                        bytes = bytes.saturating_add(1);
                        most = most.max(a.abs_diff(*b));
                        let (x, y) = (
                            at.checked_rem(row).unwrap_or(0) / 4,
                            at.checked_div(row).unwrap_or(0),
                        );
                        (left, right) = (left.min(x), right.max(x));
                        (top, bottom) = (top.min(y), bottom.max(y));
                    }
                }
                println!(
                    "  thread count moves: {name}: {bytes} bytes differ, by up to {most} levels, \
                     in x {left}–{right}, y {top}–{bottom}"
                );
                self.differing.push(name.to_owned());
            }
            (Err(why), Ok(_)) => {
                println!("  thread count moves: {name}: one thread refused what many drew: {why}");
                self.differing.push(name.to_owned());
            }
            (Ok(_), Err(why)) => {
                println!(
                    "  thread count moves: {name}: many threads refused what one drew: {}",
                    why.to_string()
                );
                self.differing.push(name.to_owned());
            }
            (Err(serial), Err(many)) => {
                println!(
                    "  thread count moves: {name}: the two refusals differ: one thread {serial}, many {}",
                    many.to_string()
                );
                self.differing.push(name.to_owned());
            }
        }
    }

    /// Fails the gate on any page the thread count moved.
    fn hold(&self) {
        if self.raster.is_some() {
            println!(
                "  thread count: {} page(s) differ between one encode thread and many",
                self.differing.len()
            );
        }
        assert!(
            self.differing.is_empty(),
            "a frame's bytes depend on its encode thread count on {:?}",
            self.differing
        );
    }
}

/// Where each timed page's two clocks go, one `name, oracle ms, raster ms` line per page, when
/// `PDFVIEWER_RASTER_TIMES` names a file.
///
/// A total and a median say how much a change costs and not where: comparing two builds page by
/// page is what finds the pages a construction moved, which are the ones worth profiling (ADR
/// 1397). The ratchets are unaffected, since this only writes what the run already measured.
fn page_times() -> Option<std::io::BufWriter<std::fs::File>> {
    let path = std::env::var_os("PDFVIEWER_RASTER_TIMES")?;
    std::fs::File::create(path)
        .ok()
        .map(std::io::BufWriter::new)
}

/// Holds the run to whichever lists it is the measurement for.
fn hold(
    which: Ratchets,
    refused: &[String],
    differing: &[String],
    incomparable: &[(String, NotComparable)],
) {
    match which {
        Ratchets::None => {}
        Ratchets::RefusalsUnderMagnification => {
            assert_eq!(
                refused,
                refused_pages(&REFUSED_BY_THE_DEVICE_AT_FOUR),
                "the pages refused at {MAGNIFIED}× have changed: a name in \
                 REFUSED_BEFORE_THE_SCENE is this tree's translation, a name in \
                 REFUSED_BY_THE_DEVICE_AT_FOUR is the adapter's"
            );
        }
        Ratchets::All => {
            assert_eq!(
                refused,
                refused_pages(&REFUSED_BY_THE_DEVICE),
                "the pages refused at {SCALE}× have changed: a name in REFUSED_BEFORE_THE_SCENE \
                 is this tree's translation, a name in REFUSED_BY_THE_DEVICE is the adapter's"
            );
            assert_eq!(
                differing,
                differing_pages(),
                "the pages raster draws differently from the oracle have changed"
            );
            let named: Vec<(String, NotComparable)> = incomparable
                .iter()
                .map(|(name, why)| (name.clone(), *why))
                .collect();
            assert_eq!(
                named,
                not_comparable_pages(),
                "the pages that could not be compared at all have changed: each cause is a \
                 different statement, and NOT_COMPARABLE's note says what each one of them is"
            );
        }
    }
}

/// Which ratchets a run checks, which depends on which run it is.
#[derive(Clone, Copy)]
enum Ratchets {
    /// Both lists: the survey is the measurement they were taken from.
    All,
    /// The refusals alone, against [`REFUSED_BY_THE_DEVICE_AT_FOUR`]. A *differing* list is a
    /// property of the coverage quantum and
    /// shrinks as a page grows, so it is a different measurement at every scale and nobody has
    /// stabilised one; a *refusal* at magnification is arithmetic against a stated budget or a
    /// stated device limit, and both of those hold still.
    RefusalsUnderMagnification,
    /// Nothing: the run measured a population no list was taken over.
    None,
}

/// What a run was configured with: the four knobs, carried together so that the one function
/// that decides which lists a run may be held to sees all of them.
#[derive(Clone, Copy)]
struct Settings {
    scale: f32,
    coverage: raster_gpu::Coverage,
    threads: usize,
    quantum: Option<u16>,
}

impl Settings {
    /// The four knobs as the environment leaves them, each read by the function that documents it.
    fn from_environment() -> Self {
        Self {
            scale: scale(),
            coverage: coverage(),
            threads: encode_threads(),
            quantum: glyph_quantum(),
        }
    }

    /// A backend configured this way.
    ///
    /// **The quantum and the thread count are the only two of raster's options this overrides**,
    /// and both default to what [`render_raster::options`] ships: the gate draws what the viewer
    /// draws, because a gate that turns a shipped setting off is measuring a configuration nobody
    /// runs (ADR 0498). The coverage lane is not an option but a call, so it is set after.
    #[expect(
        clippy::panic,
        reason = "test code: a machine with no adapter cannot run this gate, and saying so is \
                  the intended failure"
    )]
    fn rasterizer(self) -> QuorraRasterizer {
        let mut raster = QuorraRasterizer::with_options(&raster_gpu::Options {
            glyph_quantum: self.quantum,
            encode_threads: self.threads,
            ..render_raster::options()
        })
        .unwrap_or_else(|e| panic!("no adapter available for raster: {e}"));
        raster.set_coverage(self.coverage);
        raster
    }
}

/// Which ratchets below the survey are checked, saying why when some are not.
///
/// The refusal lists and the differing lists are measured over the whole corpus, at [`SCALE`], on
/// raster's default coverage lane, at the shipped glyph quantum — exactly, because a measurement
/// taken anywhere else is a different measurement — so each of those knobs turns them off. A list
/// held to equality over a subset would report every document the filter excluded as fixed, and a
/// list held over the *other* lane would report the two lanes' stated difference (raster's ADR
/// 0016) as a change in this backend.
///
/// **One knob has a ratchet of its own** (ADR 0313): the same corpus at [`MAGNIFIED`] on the
/// default lane holds [`REFUSED_BY_THE_DEVICE_AT_FOUR`] beside [`REFUSED_BEFORE_THE_SCENE`]. Its
/// doc comment is the argument for why that list can be held.
fn ratchets(documents: usize, settings: Settings, filtered: bool) -> Ratchets {
    let Settings {
        scale,
        coverage,
        quantum,
        ..
    } = settings;
    // Every list below was measured over the whole corpus on the default lane at the shipped
    // quantum; only the scale decides which of them a run can still be held to.
    let as_measured = !filtered
        && coverage == raster_gpu::Coverage::Cpu
        && quantum == render_raster::options().glyph_quantum;
    if as_measured && is_exactly(scale, SCALE) {
        return Ratchets::All;
    }
    if as_measured && is_exactly(scale, MAGNIFIED) {
        println!(
            "{documents} of the corpus at scale {scale} on the {} lane. The refusals below ARE \
             checked, against the list measured at this scale on this lane; the differing list is \
             not, because it is a property of the coverage quantum and is a different measurement \
             at every scale.",
            lane_name(coverage)
        );
        return Ratchets::RefusalsUnderMagnification;
    }
    println!(
        "{documents} of the corpus at scale {scale} on the {} lane with the glyph quantum {}. The \
         ratchets below are NOT checked: they are measured over the whole corpus at scale {SCALE} \
         on the default lane at the shipped quantum, and a list held to equality over anything \
         else would report every document it excluded as fixed.",
        lane_name(coverage),
        quantum_name(quantum)
    );
    Ratchets::None
}

/// Whether a run's scale is exactly the one a list was measured at.
///
/// Written as a difference against zero rather than as `==` because that is what the question
/// is — a list belongs to the scale it was taken at and to no neighbour of it — and because a
/// scale that is not a number must match nothing: `NaN <= 0.0` is false, which is the answer
/// wanted here.
fn is_exactly(scale: f32, measured: f32) -> bool {
    (scale - measured).abs() <= 0.0
}

/// What to call a lane in a line a person reads.
fn lane_name(coverage: raster_gpu::Coverage) -> &'static str {
    match coverage {
        raster_gpu::Coverage::Cpu => "cpu",
        raster_gpu::Coverage::Gpu => "gpu",
        raster_gpu::Coverage::Compute => "compute",
    }
}

/// Says which adapter and which build produced the numbers below them.
///
/// Both matter to every number this gate prints: a software adapter and a discrete GPU are
/// different machines, and a debug build is ~15× slower here.
fn announce(raster: &QuorraRasterizer, documents: usize, settings: Settings) {
    let Settings {
        scale,
        coverage,
        threads,
        quantum,
    } = settings;
    println!("adapter: {}", raster.adapter_description());
    println!(
        "{documents} documents, page one, at scale {scale}, {} coverage lane, \
         glyph quantum {}, {threads} encode thread(s), {} build",
        lane_name(coverage),
        quantum_name(quantum),
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        }
    );
}

/// The corpus, or the part of it `PDFVIEWER_RASTER_ONLY` names.
fn selected(files: Vec<PathBuf>, filter: Option<&str>) -> Vec<PathBuf> {
    let Some(filter) = filter else { return files };
    files
        .into_iter()
        .filter(|path| {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            filter.split(',').any(|want| name.contains(want.trim()))
        })
        .collect()
}

/// What the run found, for [`report`].
struct Tally<'a> {
    agreed: usize,
    incomparable: &'a [(String, NotComparable)],
    differing: &'a [String],
    refused: &'a [String],
    worst: &'a mut Vec<(f64, String)>,
}

/// Prints the survey: the counts, the two clocks, the median ratio and the ten worst pages.
///
/// The totals and the median answer different questions and only quoting both is honest —
/// `hayro`'s comparison learned that, and a distribution with a long tail makes a total say
/// something a per-page ratio does not.
fn report(tally: &mut Tally<'_>, timing: (Duration, Duration, &mut Vec<f64>), took: Duration) {
    let (cpu_total, gpu_total, ratios) = timing;
    let compared = tally
        .agreed
        .saturating_add(tally.differing.len())
        .saturating_add(tally.refused.len());
    println!(
        "\n{compared} pages compared in {took:.1?}: {} agree, {} differ, {} refused, {} not \
         comparable",
        tally.agreed,
        tally.differing.len(),
        tally.refused.len(),
        tally.incomparable.len()
    );
    println!(
        "  rasterisation: {cpu_total:.2?} on the CPU backend, {gpu_total:.2?} through raster \
         (offscreen, readback included)"
    );
    ratios.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    if let Some(median) = ratios.get(ratios.len() / 2) {
        println!(
            "  median page: raster takes {median:.2}× the CPU backend's time, over {} pages \
             above a millisecond",
            ratios.len()
        );
    }
    tally
        .worst
        .sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    if !tally.worst.is_empty() {
        println!("  furthest from the oracle:");
        for (mean, name) in tally.worst.iter().take(10) {
            println!("    {mean:8.3} mean  {name}");
        }
    }
}

/// Writes both renders of a differing page beside each other, for looking at.
///
/// The oracle's own artefacts are the fastest diagnostic in this tree and this gate had
/// none: a list of names and three numbers cannot tell a missing mark from a soft edge. Only
/// the pages that differ get one, so what is on disk is exactly the set worth opening.
fn write_artefacts(
    name: &str,
    cpu: &pdf_render::Raster,
    ours: &Result<pdf_render::Raster, impl ToString>,
) {
    let Ok(ours) = ours else { return };
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/tmp/raster")
        .join(name.trim_end_matches(".pdf"));
    if std::fs::create_dir_all(&root).is_err() {
        return;
    }
    for (what, raster) in [("cpu", cpu), ("raster", ours)] {
        let Ok(file) = std::fs::File::create(root.join(format!("{what}.png"))) else {
            continue;
        };
        let mut encoder =
            png::Encoder::new(std::io::BufWriter::new(file), raster.width, raster.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        if let Ok(mut writer) = encoder.write_header() {
            let _ = writer.write_image_data(&raster.data);
        }
    }
}

/// Compares one page, or says why it could not be.
fn outcome(cpu: &pdf_render::Raster, ours: &Result<pdf_render::Raster, impl ToString>) -> Outcome {
    let ours = match ours {
        Ok(raster) => raster,
        Err(why) => return Outcome::Refused(why.to_string()),
    };
    let Ok(c) = raster_compare::compare(cpu, ours) else {
        return Outcome::NotComparable(NotComparable::RastersCannotBeCompared);
    };
    if c.mean_error < MAX_MEAN_ERROR
        && c.worst_tile_error < MAX_WORST_TILE_ERROR
        && c.structural_similarity > MIN_STRUCTURAL_SIMILARITY
    {
        return Outcome::Agrees;
    }
    Outcome::Differs(format!(
        "mean {:.4} worst tile {:.2} at {:?} differing {:.4} ssim {:.5}",
        c.mean_error,
        c.worst_tile_error,
        c.worst_tile_at,
        c.differing_fraction,
        c.structural_similarity,
    ))
}

/// The three stages before the backend under test is asked anything, or which of them stopped.
///
/// The comparison needs a page, a target for it and the **oracle's** raster of that target, and
/// each of the three can decline: a document that yields no page one, a target past
/// [`PIXEL_BUDGET`], and the CPU backend refusing the list. They are together here because they
/// are one statement — *nothing was asked of raster* — and because the third of them is a fact
/// about the correctness oracle rather than about either backend, which is why it is a
/// [`NotComparable`] of its own and not a counter.
fn oracles_render(
    path: &Path,
    scale: f32,
) -> Result<(DisplayList, TargetSpec, pdf_render::Raster, Duration), (NotComparable, String)> {
    let list = page_one(path)?;
    let target = TargetSpec::for_page(&list, scale, PIXEL_BUDGET)
        .map_err(|why| (NotComparable::PastThePixelBudget, format!(": {why}")))?;
    let at = Instant::now();
    let cpu = CpuRasterizer::new()
        .rasterize(&list, target)
        .map_err(|why| (NotComparable::TheOracleRefused, format!(": {why}")))?;
    Ok((list, target, cpu, at.elapsed()))
}

/// The display list of a document's first page, or which of three things there was not one for.
///
/// The three are kept apart because they are three different statements: a file that will not
/// read is this machine's, a document that will not open is `pdf-syntax`'s, and a page tree with
/// no first page is the producer's. Collapsing them into `None` was what made [`NotComparable`]
/// necessary to write down at all.
fn page_one(path: &Path) -> Result<DisplayList, (NotComparable, String)> {
    let bytes =
        std::fs::read(path).map_err(|why| (NotComparable::Unreadable, format!(": {why}")))?;
    // §7.6.4.1's default user password first, and where it is refused the password
    // `corpus_passwords` publishes for the file — the prompt a person would have answered.
    let known = path
        .file_name()
        .and_then(|name| corpus_passwords::corpus_password(&name.to_string_lossy()));
    let opened = match (Document::open(bytes.clone()), known) {
        (Err(SyntaxError::PasswordRequired), Some(known)) => {
            Document::open_with_password(bytes, Limits::default(), known.password)
        }
        (opened, _) => opened,
    };
    let document = opened.map_err(|why| (NotComparable::WouldNotOpen, format!(": {why}")))?;
    let pages = pdf_model::Pages::new(&document);
    let page = pages
        .get(0)
        .ok_or((NotComparable::NoFirstPage, String::new()))?;
    Ok(pdf_model::content::interpret(&document, &page).display_list)
}

/// The corpus files, or `None` when the submodule is not checked out.
fn corpus() -> Option<Vec<PathBuf>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/pdf.js/test/pdfs");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&root)
        .ok()?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|extension| extension == "pdf"))
        .collect();
    if files.is_empty() {
        return None;
    }
    files.sort();
    Some(files)
}
