//! Every page the corpus draws, through quorra and through the CPU oracle.
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
//!   and that is exactly where quorra's ADR 0073 hid: a rounding that reached the bucket count
//!   itself and wrapped, seating 3.1% of sub-pixel phases per axis a whole device pixel out of
//!   place, on the lane that draws text. `PDFVIEWER_QUORRA_GLYPH_QUANTUM=off` still runs the
//!   isolation column (ADR 0498).
//! - **Refusals**, by name: a page quorra cannot draw is a hole in the backend, and it must
//!   be one somebody wrote down rather than one nobody counted.
//! - **Speed**, both totals and the per-page median ratio, which answer different questions.
//!   A GPU frame here includes the readback to system memory, which a windowed host does not
//!   pay — `RENDER_LIBRARY.md` section 6.1 measures that at 55% to 92% of a frame — so the
//!   ratio below is the *offscreen* one and says nothing directly about the window.
//!
//! # Running it
//!
//! ``text
//! cargo test --release -p render-quorra --test corpus -- --ignored --nocapture
//! ``
//!
//! `PDFVIEWER_QUORRA_ONLY=a,b` restricts it to matching file names and **refuses to check the
//! ratchets**, saying so — a list held to equality over a subset would report every document
//! the filter excluded as fixed. `PDFVIEWER_QUORRA_SCALE`, `PDFVIEWER_QUORRA_COVERAGE` and
//! `PDFVIEWER_QUORRA_GLYPH_QUANTUM` are the other three knobs, each documented at the function
//! that reads it, and each turning the ratchets off for the same reason. Debug builds are ~15×
//! slower here; the numbers below are release numbers and the run says which it took.

#![expect(
    clippy::print_stdout,
    reason = "test code: an explanatory panic is the intended failure, and the survey \
              output is the point of the run"
)]

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use pdf_render::{DisplayList, Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;
use render_quorra::QuorraRasterizer;

/// Pixel budget per page, generous enough that no real page reaches it.
const PIXEL_BUDGET: u64 = 64 << 20;

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

/// The scale to render at, which `PDFVIEWER_QUORRA_SCALE` may override.
///
/// It exists for the speed half of this gate rather than the fidelity half. A GPU frame's
/// cost is dominated by a per-pixel floor and a readback (`RENDER_LIBRARY.md` section 6.1) while
/// this tree's CPU rasterisation grows with the pixels, so the ratio between them is a
/// *function of the scale* and one scale cannot say which way it runs — the same trap ADR
/// 0136 met comparing `rasterrocket` at 72 and 150 dpi. Overriding it **skips the ratchets**,
/// which are measured at the default.
fn scale() -> f32 {
    std::env::var("PDFVIEWER_QUORRA_SCALE")
        .ok()
        .and_then(|value| value.trim().parse().ok())
        .filter(|scale: &f32| scale.is_finite() && *scale > 0.0)
        .unwrap_or(SCALE)
}

/// Which of quorra's two coverage lanes to draw with, which `PDFVIEWER_QUORRA_COVERAGE` may
/// override with `cpu` or `gpu`.
///
/// **The default is quorra's own — [`quorra_gpu::Coverage::Cpu`], the scanline rasteriser with
/// the glyph atlas in front of it — and that is the lane this tree draws every page with.** The
/// other one exists for magnification: nothing in it depends on the scale, so a frame at 100×
/// re-uses what a frame at 1× built, and `viewer-ui` switches to it past
/// `GPU_COVERAGE_MAGNIFICATION`. Which means the lane a person sees when they zoom in has been
/// judged by fixtures alone — trap 12b's exact shape, one lane over — while this gate, the only
/// instrument in the tree that puts a backend beside the oracle at the corpus's scale, has never
/// run it.
///
/// Overriding it **skips the ratchets**, for the scale knob's reason and one more: the two lanes
/// deliberately do not draw identical pixels (quorra's ADR 0016 states the sampled lane's bound
/// against the exact one), so a list of differing pages is a property of the lane that produced
/// it.
/// A value that is neither is a **panic** rather than a fallback: this variable's whole purpose
/// is to say which lane the numbers below came from, and a typo that quietly measured the
/// default would be a run reported as the lane it did not use.
#[expect(
    clippy::panic,
    reason = "a mistyped lane must stop the run: the alternative is a survey headed `gpu` \
              that measured the default"
)]
fn coverage() -> quorra_gpu::Coverage {
    match std::env::var("PDFVIEWER_QUORRA_COVERAGE")
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "" | "cpu" => quorra_gpu::Coverage::Cpu,
        "gpu" => quorra_gpu::Coverage::Gpu,
        other => panic!("PDFVIEWER_QUORRA_COVERAGE={other}: expected `cpu` or `gpu`"),
    }
}

/// The glyph-phase quantum this run draws with, which `PDFVIEWER_QUORRA_GLYPH_QUANTUM` may
/// override with `off` or a bucket count.
///
/// **The default is [`render_quorra::options`]'s, so the gate draws what the viewer draws.** It
/// was `None` for the whole of this gate's life, on the reasoning that a quantum-off run isolates
/// the backend's fidelity from a sub-1/32-pixel trade `real_pages.rs` gates separately — and that
/// reasoning is what let a whole-pixel misplacement live in the shipped configuration where the
/// 974-page instrument could not reach it. quorra's ADR 0073: `(fx · q).round()` reaches `q`
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
    let Ok(value) = std::env::var("PDFVIEWER_QUORRA_GLYPH_QUANTUM") else {
        return render_quorra::options().glyph_quantum;
    };
    match value.trim() {
        "" => render_quorra::options().glyph_quantum,
        "off" | "none" => None,
        other => Some(
            other
                .parse::<u16>()
                .ok()
                .filter(|q| *q > 0)
                .unwrap_or_else(|| {
                    panic!(
                        "PDFVIEWER_QUORRA_GLYPH_QUANTUM={other}: expected `off` or a positive \
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

/// How many threads quorra's geometry phase may use for this run, which
/// `PDFVIEWER_QUORRA_ENCODE_THREADS` may override.
///
/// **The default is [`render_quorra::options`]'s, so the gate draws what the viewer draws** —
/// and the override exists because the property that matters about this number is that it
/// changes *nothing*. quorra states a frame's result byte-identical at any thread count (their
/// ADR 0054); this gate over 956 real pages carrying clip chains, groups, masks and atlas
/// pressure is the evidence for that claim on *our* side of the boundary, and it is only evidence
/// if the same corpus can be run at 1 and compared line by line against the same corpus at the
/// host's number. ADR 0377 is that comparison. Overriding it does **not** skip the ratchets: a
/// thread count that moved a verdict is precisely what this gate should fail for.
fn encode_threads() -> usize {
    std::env::var("PDFVIEWER_QUORRA_ENCODE_THREADS")
        .ok()
        .and_then(|value| value.trim().parse().ok())
        .filter(|threads: &usize| *threads > 0)
        .unwrap_or_else(|| render_quorra::options().encode_threads)
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
/// `render-quorra`'s own, raised while translating a [`DisplayList`] into a
/// `quorra_scene::Scene`, so they hold **at every scale** and no upstream release can move one.
/// [`REFUSED_BY_THE_DEVICE`] and [`REFUSED_BY_THE_DEVICE_AT_FOUR`] are the other stage.
///
/// Held to equality in both directions: a page arriving here is a new hole in the backend,
/// and a page leaving it is a hole closed. The reason each one gives is printed by the run.
///
/// **One page, and its refusal now says what actually ran out.** `bug1721218_reduced.pdf` is
/// this viewer's most pathological page by some margin, and quorra refuses it with "the
/// frame's rasterised coverage outgrew the 16384x16384 scratch image this adapter allows" —
/// a texture-capacity limit named as one. The six pages that used to sit here beside it were
/// casualties of the *old* scratch sheet (2048 texels wide, and a refusal message whose
/// arithmetic contradicted itself — `QUORRA_FEEDBACK.md` section 3); quorra widened the sheet
/// to the device dimension and all six draw. `issue17848.pdf` left for a different reason:
/// its mesh shading has no visible raster, pdf.js's issue #17848 traced that to a defective
/// document, and the backend now draws nothing for it exactly as this viewer's own two
/// backends always have.
///
/// **Four joined it in the three-hundred-and-ninety-seventh, and every one of them is a page
/// this backend used to draw *wrongly*.** ADR 0234 states §11.4.6's shape apart from the alpha
/// for the elements where the two differ, and `quorra_scene::Compose` has source-over and
/// coverage-modulated source — the second of which *is* the assumption the new element exists
/// to contradict. So these four pages used to be drawn by reading the shape off the alpha, which
/// agreed with a CPU backend doing the same thing; now the display list says what the clause
/// says and the backend says it cannot draw it. A refusal that replaces an agreement about a
/// wrong picture is a gain, and it is `doc/QUORRA_FEEDBACK.md` section 14's entry: two Porter-Duff
/// operators, Destination-Out and Plus. **Both arrived at `89d7dd77`** (quorra's ADR 0025).
///
/// **The four-hundred-and-thirty-ninth session wrote that translation out and found it cannot
/// be asked for**, which is why they are still here and why the reason each gives has changed
/// again. `SceneBuilder::fill` refuses a staged operator inside a knockout group, and
/// `Command::Shaped` occurs nowhere else; `SceneBuilder::group` takes no compositing operator
/// at all, and three of these four state a `Shaped` whose two halves are *groups*. Section
/// 14.2 is the ask that follows, and `headless_quorra.rs`'s
/// `quorra_will_not_take_the_pair_where_this_tree_would_hand_it_over` holds the first half of
/// it against the vocabulary directly, so this list moves when quorra moves rather than when
/// somebody re-reads a document.
///
/// **Four joined in the four-hundredth for §11.4.4's non-isolated group, and three of them left
/// again in the four-hundred-and-thirty-eighth, which is the first time a name on this list has
/// left because the request under it was answered.** ADR 0237 draws §11.4.4's non-isolated group
/// — its elements composite onto the *page's* own colour rather than onto §11.4.5's transparency
/// — and `quorra_scene::GroupSpec` opened its layer transparent, as every rasterising library's
/// does, so the backend refused and named the clause. `doc/QUORRA_FEEDBACK.md` section 16 asked
/// for one flag; quorra's ADR 0019 is that flag, Table 145's `/I`, together with the composite
/// back, and this backend now passes it straight through. `bug1755507.pdf`, `issue13520.pdf` and
/// `issue18032.pdf` **agree with the CPU oracle** — two independent transcriptions of §11.4.4
/// meeting on Illustrator and `InDesign` artwork, which is not what the agreement they had before
/// was: that one was two backends substituting the same wrong backdrop.
///
/// **The fourth stayed, and what it says now is the finding.** `issue12798_page1_reduced.pdf`
/// refuses with *"a page composited in a four-component blending colour space (§11.4.7)"* — the
/// §11.4.4 refusal had been standing in front of a second one, and the page belongs to section 17's
/// population rather than to section 16's. Which is why a refusal names what it cannot do — a
/// list that only counted would have reported three closed holes and missed that a fourth
/// changed its reason. **The ordinals below are the order names *arrived* in and no longer the
/// order they sit in**: eleven joined this list and eight are on it.
///
/// **A tenth in the four-hundred-and-twenty-sixth, and it is the third time this has happened
/// for the third reason.** ADR 0262 draws §11.4.7's page group in the `DeviceCMYK` blending
/// space it states, by interpreting the page twice — once per half of the space's four
/// components — and putting the two rasters together at the end. A `quorra_scene::Scene`
/// renders one, so the backend refuses the list by name; `personwithdog.pdf` is the corpus's
/// one such page and it used to be *reported* rather than drawn, so this is a refusal replacing
/// a report rather than an agreement. `doc/QUORRA_FEEDBACK.md` section 17 is the request, and it
/// is a small one: nothing new in the scene vocabulary, only a way to render one scene into two
/// targets or to run the pipeline twice and hand back both readbacks. **The second of those was
/// already true**, and `89d7dd77` added the test that keeps it true: two `Target::Readback` renders
/// against one device come back whole, share their uploaded resources, and the second pays no
/// geometry at all because quorra's glyph key does not carry colour. So this refusal and the one
/// below are `render-quorra`'s own work now — two `rasterize` calls and `pdf_render::blending`'s
/// recombination, which `render-cpu` already does — and `doc/todo/23` carries them.
///
/// **An eleventh in the four-hundred-and-twenty-seventh, and it is the same request reaching one
/// more page rather than a new one.** ADR 0263 gives §11.7.2 its conversion *into* the blending
/// space, so a page whose colours are not already ink is drawn in ink rather than reported — and
/// `bug1365930.pdf` is the corpus's one page that states `/CS /DeviceCMYK` and reports *nothing*,
/// because nothing on it composites. It was never a report and never a disagreement: it drew on
/// the device's three components and quorra agreed. It now takes §11.4.7's pair of rasters like
/// the others, for the clause's own reason — "[a]ll page-level compositing shall be done in the
/// default blending colour space of the page" is not conditioned on two marks overlapping — and
/// the backend refuses the list by name. Section 17 answers this page along with the tenth.
///
/// **Three left in the four-hundred-and-thirty-ninth, and they are the first names on this list
/// to leave because the work under them was done here rather than upstream.** Section 17 asked
/// whether two `Target::Readback` renders against one device were possible; the answer at
/// `89d7dd77` was that they always had been, and `quorra-gpu/tests/two_rasters.rs` holds it.
/// So `render-quorra` draws the page twice with a different three of the four components
/// loaded and `pdf_render::blending` puts the pair back together, which is what `render-cpu`
/// already did. `personwithdog.pdf`, `bug1365930.pdf` and `issue12798_page1_reduced.pdf` all
/// **agree with the CPU oracle** — 0.0288, 0.0093 and 0.0760 mean, page inks 20.3953/20.3659,
/// 0.8637/0.8634 and 16.9057/16.9117 — and the third is the one worth naming twice: ADR 0274
/// found its §11.4.4 refusal standing in front of this one, so it is the page whose *second*
/// reason has now gone too.
///
/// **And the four §11.4.6 pages left in the four-hundred-and-fifty-sixth, which is the first
/// time a whole request left this list at once.** Section 14.2 asked for two things — the
/// staged pair where the clause puts it, and a compositing operator on a group — and quorra's
/// ADRs 0032 and 0033 are those two. `render-quorra` states §11.4.6's two stages now (ADR
/// 0291), and all four pages **agree with the CPU oracle**: two independent transcriptions of
/// `P' = (1 − f) × P + S`, one per backend, meeting on artwork that no fixture invented.
/// What is left here is the one page that was here first, and its refusal is a texture
/// capacity rather than a clause.
///
/// **Two in the four-hundred-and-ninety-second, and both are the oracle gaining a
/// construction this backend cannot state** (ADR 0327). `bug1721218_reduced.pdf` stays, with
/// its refusal renamed: its whole artwork is one isolated `/CS /DeviceCMYK` group that now
/// composites in ink as a pair of element lists resolved per pixel at its `Do` (§11.6.6,
/// §11.7.2), and a scene under composition cannot be read back — the page-level pair's two
/// whole `Target::Readback` renders have no group-scoped analogue. The texture-capacity
/// refusal that used to print for it has not been fixed, only preempted: the pair is refused
/// before the scene is built. `issue18032.pdf` arrives for §11.4.6's non-isolated knockout
/// group, whose every element composites with the group's *own* initial backdrop — a backdrop
/// retained beside the accumulation with a scratch per element, which neither a
/// `quorra_scene::GroupSpec` nor the staged `DestOut`/`Plus` pair (written on §11.4.5's
/// transparent start) can express. Both frames go to the CPU backend, which draws them;
/// `headless_quorra.rs` holds both refusals against the cross-backend scenes.
///
/// **What a departure from *this* list means, which is why it is no longer mixed with the
/// device's.** A name arriving is a construction the CPU oracle states and this translation
/// cannot, and the round that adds one owes `doc/QUORRA_FEEDBACK.md` the ask; a name leaving is
/// a construction quorra's vocabulary grew a way to express, which is the only thing that has
/// ever taken one off. What a departure here is **not** is a statement about the adapter: this
/// stage never reaches a device, so a name here is unmoved by a texture ceiling, a byte budget
/// or a driver. And because the stage is scale-free, this one array is what both scales are held
/// to — the hazard that the five-hundred-and-twelfth session met, a second copy of these names
/// living in the 4× list and going stale while nobody ran that lane, cannot recur.
const REFUSED_BEFORE_THE_SCENE: [&str; 2] = ["bug1721218_reduced.pdf", "issue18032.pdf"];

/// Documents whose first page **the device** refuses at [`SCALE`], by name.
///
/// The second of the two stages. A refusal here is raised inside quorra at render time, against
/// a capability or a budget the adapter states: a page that translated into a scene, went to
/// the device, and could not be drawn there. This array held nothing — "at a page's own
/// resolution, nothing in this corpus asks the adapter for more than it has" — until the
/// owner-merged GPU arc took quorra's `5fb011a` and rebuilt the scene in page space (ADR 0702),
/// after which `issue1905.pdf` is over the frame's scene-byte budget at 1× as well as over the
/// coverage-sheet texture ceiling at 4×: *frame refused: frame needs 272158852 scene-derived
/// bytes, over the stated budget of 268435456* — 1.4% over. The merge round that pinned it
/// (773) ran this gate at the arc's own head with the four merged rounds absent and got the
/// identical refusal, byte for byte, so the name is the arc's and not the merge's; the CPU
/// backend draws the page and says so, which is `CLAUDE.md`'s rule for a budget refusal. The
/// upstream ask, with the run's message, is `doc/QUORRA_FEEDBACK.md` section 40.
///
/// **What a departure from this list means.** A name arriving is a page a person could open at
/// 100% and not see: it is quorra's to move, or this adapter's, and the round that finds one
/// reports it upstream with the message the run prints rather than adding a construction here.
/// A name leaving is an allocation, a tiling or a ceiling that changed upstream, which is
/// exactly what [`REFUSED_BY_THE_DEVICE_AT_FOUR`]'s own note records happening twice.
const REFUSED_BY_THE_DEVICE: [&str; 1] = ["issue1905.pdf"];

/// The same at [`MAGNIFIED`], which is the population the zoom path actually draws.
///
/// **This ratchet exists because the number it holds finally stopped moving.** At four times a
/// page's own scale this run has never been a gate: `doc/todo/02-every-round.md` section 2 makes
/// it a run every round that takes a quorra release owes, and until now what came back was a survey
/// nobody could fail — twelve refusals at one revision, seven at the next, each release taking
/// some off and each count living in a document rather than in a test. What made them uncheckable
/// was that most of them were *arithmetic against a byte budget* that upstream kept improving:
/// four pages over the 256 MiB frame budget by 4 % to 20 % is a list that moves the moment
/// anything is allocated more tightly, and quorra's ADRs 0036 to 0039 allocated every plan, mask
/// and root to what it marks. **Not one page of this corpus is refused for frame bytes at any
/// scale now**, and what is left at this stage is one page and one ceiling that no allocation
/// strategy reaches — the two bullets below being what left and what stayed:
///
/// - **`22060_A1_01_Plans.pdf` left this list in the five-hundred-and-thirty-ninth session, and
///   what took it off was neither a larger budget nor a tighter allocation.** It held 522 014 748
///   resident *resource* bytes with the next upload taking it to 548 104 348 against the
///   536 870 912 `max_resource_bytes` default — because its page drew **72 sampled images that
///   were 8 distinct rasters**, one allocation per `Do`, `image::decode_parts` having run at every
///   one of them. `image::RasterCache` (ADR 0374) made the repeated `Do`s share the raster they
///   always meant, and the same page now uploads what it actually holds. The lesson is the one
///   this comment already argued from the other end: a refusal that is arithmetic against a byte
///   budget is a question about who is spending the bytes, and this time the answer was upstream
///   of the backend entirely. Nobody raised `max_resource_bytes`, and a budget raised to admit one
///   page would still be a budget chosen by that page.
/// - `issue1905.pdf` exceeds the **16 384 × 16 384 texture** this adapter allows for the
///   rasterised-coverage sheet. That is a device capability, not a policy. Quorra measured a
///   multi-sheet fix at its `5483996` and **declined it with the numbers written down**: a second
///   sheet takes this page to 287 MB — refused again, on bytes — and its neighbours to a
///   quarter-gigabyte of per-frame upload, a page drawn at a cost its own brief calls a failure.
///   The ceiling that bites is that a clipped shape becomes one coverage tile of its own device
///   bounds; the open work upstream is on the tiling side. Its *message* changed with the release
///   this list last moved on: the refusal now names the sheet it met rather than only the
///   adapter's wall (quorra's ADR 0057 decision 2), which is that ADR's other half.
///
///   **`bug1703683_page2_reduced.pdf` was here for the same reason and left in the
///   five-hundred-and-seventy-sixth session, by a run.** quorra's ADR 0057 sizes a clipped mark's
///   coverage tile by its chain's own bounding box instead of by the open clip rectangle, and its
///   §1 measures this page asking 1 008 561 911 texels where its 141 chains admit 2 297 897. That
///   landed on their `cafadeb`, which `cad50156` carries; at 4× on this lane the page now **agrees
///   with the CPU oracle**. Two rounds were *told* the name could come off before one could watch
///   it come off — the five-hundred-and-sixty-seventh ran the gate against `eada81ec` and left it
///   on, which ADR 0402 decision 3 argues was right — and that is `CLAUDE.md` principle 5 one
///   boundary over: a report from another implementation is evidence about *that* implementation,
///   and a ratchet held by name exists so that a name comes off by a run rather than by a message.
///   The message was accurate this time, which is not the same thing as its having been sufficient.
///
/// **This array used to hold [`REFUSED_BEFORE_THE_SCENE`]'s two names as well, and that is the
/// flattening the split undoes.** Both stages in one list meant a name leaving it could be a
/// device that grew a capability or a translation that grew a construction, and the ratchet
/// could not say which; it also meant this tree's own refusals were written down **twice**, once
/// per scale, which went wrong exactly the way a second copy does — `issue18032.pdf` was added
/// at the scale-1 list in the four-hundred-and-ninety-second session and arrived here in the
/// five-hundred-and-twelfth, not because anything changed but because no round in between ran
/// the 4× lane. Those two names are one scale-free array now and this one holds only what the
/// device refuses. (`bug1721218_reduced.pdf` meets the sheet ceiling too; it is refused before
/// it can reach it, which is why it is not also named here.)
///
/// **What a departure from this list means.** A name arriving is a hole that only opens under
/// magnification — a page a person can open and not zoom into — and it is quorra's or this
/// adapter's rather than ours, so the round that finds one carries the message upstream. A name
/// leaving is that hole closed upstream, which is what both departures recorded above were. It
/// is checked on the default lane only, because the two lanes put *different tiles* in the
/// coverage sheet this refusal is against — the encoder chooses per command (quorra's ADR 0029)
/// — so the sheet a frame commits is a property of the lane, and a lane's refusals are its own.
const REFUSED_BY_THE_DEVICE_AT_FOUR: [&str; 1] = ["issue1905.pdf"];

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

/// Pages where the two rasterisers differ only at the **edges** of what they draw.
///
/// Structural similarity above 0.99 — `raster_compare`'s own vector threshold — is the
/// statement that the same shapes are in the same places, so what is left is coverage at a
/// boundary. This group used to be dominated by one document family (`tracemonkey.pdf` and
/// its variants and relatives, a page of dense text measured against a different glyph
/// rasteriser), and that family is gone now — see the seventeen below. **This group is the
/// floor, not a defect list** — what would make it one is a page arriving in it whose
/// similarity is high because the difference is uniform.
///
/// **Seventeen left at once in the five-hundred-and-twelfth session, on quorra's
/// `87898c69`, and every one is the two rasterisers agreeing rather than either moving
/// toward the other.** Two upstream changes, each derived from the standard's own words
/// rather than from any renderer's output. Fifteen — the six `tracemonkey*` pages,
/// `bug1885505.pdf`, `bug1992868.pdf`, `chrome-text-selection-markedContent.pdf`,
/// `issue14438.pdf`, `issue15012.pdf`, `issue18911.pdf`, `issue19239.pdf`, `issue7014.pdf`
/// and `issue7492.pdf` — are quorra's ADR 0044: a cubic's flattening bound is now the
/// tighter of the fixed quarter-pixel tolerance and 1/32 of the cubic's own device extent,
/// which floors a full turn at 16 chords. §10.7.2's own NOTE 2 is the argument — "the
/// purpose of the flatness tolerance is to control the precision of curve rendering, not to
/// draw inscribed polygons" — and the population the floor reaches is glyph outlines, whose
/// bowls at body size are cubics two to five device pixels across, which is why what moved
/// is prose. The other two, `extgstate.pdf` and `inks_basic.pdf`, are quorra's round-cap
/// fix (its `d594566`): the near cap of an open subpath was the *inward* half-disc wound
/// against the body it sat inside, punching a Table 53-sized hole that summed ink could not
/// see; the cap fan is now built from the outward direction the stroker already has.
///
/// `issue4260_reduced.pdf` — once the worst page in the run at similarity 0.49, a grid of
/// zero-height rectangles drawn blank — arrived here from the shape list when the backend
/// started asking `pdf_render::split_collapsed_fill` the §10.7.4 question, and **left
/// altogether in the three-hundred-and-sixty-eighth session, when that question started being
/// answered with whole device pixels.** What kept it here was the last thing two rasterisers
/// can disagree about: a mark laid down at the shape's own fractional position is an
/// anti-aliased band, and each backend distributes such a band across two rows in its own way.
/// A mark that is a whole pixel row has nothing left to distribute — the two backends draw the
/// same rows, from the same shared geometry, and the page agrees to the byte. ADR 0208.
///
/// **`issue21068.pdf` left in the two-hundred-and-seventh session and the reason is worth
/// keeping**: it is four rows of comb fields whose separators sit exactly on their `/BBox`, and
/// the anti-aliased clip was costing each of them a fraction of a pixel — differently in the two
/// rasterisers, which is what put it here. Both draw from the *same* display list, so once the
/// redundant clip came off (ADR 0165) there was nothing left to differ about. **A page in this
/// list can be here because of something upstream of both backends**, which is not what its name
/// suggests.
///
/// **Three more left in the three-hundred-and-eighty-ninth, and it is the same shape one clause
/// along.** `bug1308536.pdf` (mean 1.5709), `issue11913.pdf` (1.5275) and `issue13447.pdf`
/// (1.5652) agreed when §10.7.4's coverage rule reached a shape thinner than a device pixel on
/// the processor (ADR 0226). What each of them was is a rectangle under a pixel thick that
/// `tiny-skia` rounded to a quantum of its own — up to a whole row, or down to nothing — where
/// quorra drew the area. Once the processor draws the area too there is nothing left for two
/// rasterisers to distribute differently, which is the same sentence `issue4260_reduced.pdf`
/// left on.
///
/// **Two arrived and one left when §10.7.4's quantum boundary became inclusive (ADR 0285).**
/// `tiny-skia` takes its hairline for every width up to *and including* one device pixel, so a
/// `1 w` rule — most of the line work in a technical drawing at the page's own scale — was drawn
/// at `cos θ` of its own area. It is filled as its own outline now, which is what quorra always
/// did, and the two backends therefore differ on more *edges* and on less ink: `bug1743245.pdf`
/// and `issue2177.pdf` cross a threshold in the third decimal place (mean 1.5070 against a bound
/// of 1.5; worst tile 7.14 against 7.0), and `issue14415.pdf` left. **Not one page's verdict
/// moved on the reference oracle** — 905 agree, 68 contradicted, 786 ambiguous before and after
/// — which is what makes this churn at the bound rather than a change of picture, and it is
/// written down here so that a later round does not read the arrival as a defect.
///
/// **And a fourth left in the four-hundred-and-thirty-second, on the same sentence and the other
/// half of the shape.** `issue12810.pdf` is a 1728 × 2592 drawing whose page one states **34 787
/// strokes thinner than a device pixel**, 25 406 of them *not* axis-aligned, and 62.6% of their
/// 173 316 pixels of length lies within 40° to 50° of an axis — which is where `tiny-skia`'s
/// hairline was at its worst, laying one pixel down per step along the line's *longer* device axis
/// and so carrying `cos θ` of the rule's area. Weighted by length that was **19.4% of the page's
/// stroke ink**, and quorra was drawing all of it. Once the processor draws a turned sub-pixel rule
/// one device pixel wide with the width it gave up in the paint's alpha (ADR 0268), the page's own
/// ink goes 5.0782 → 5.1550 and the two backends have nothing left to differ about.
/// **`issue8187.pdf` left in the seven-hundred-and-eleventh, and it is the same sentence as the
/// three that left in the three-hundred-and-eighty-ninth one clause over** (ADR 0583). Its page is
/// fourteen fills of which **fourteen are paths stating several axis-aligned rectangles** — eight
/// whose portions fall in different device pixels and six whose do not — and the processor was
/// sending all of them to `tiny-skia`'s supersampled path converter, which rounds an axis-aligned
/// edge to a quarter, where quorra tracks the fraction to a level of 255. §11.6.2 is what admits
/// measuring each portion exactly ("[p]ortions of an object shall not be composited with one
/// another", so drawing them one at a time is only allowed while no pixel receives two), and once
/// the processor measures the eight the way quorra already did, the page agrees. **The processor
/// moving to the device, which is the direction this list is allowed to move in** — the same as
/// ADR 0476's `issue18823.pdf`.
///
/// **`issue2177.pdf` left in the five-hundred-and-thirty-second, and the defect was quorra's
/// own.** Its `fill_mask` computes a shape's coverage over a rectangle of device pixels, and
/// where an edge entered that rectangle from *outside* it clamped the edge piece's two endpoints
/// to the border and interpolated between them. That preserves the row's total winding — every
/// column past the crossing reads the right value, which is why neither side's tests saw it for
/// as long as they did — while handing the columns *at* the border the height the piece spent
/// outside. It cuts the piece at the border now (quorra's ADR 0049, and its own test states the
/// expected value from the geometry: 0.625 of a pixel of area is 159 of 255, where the tree read
/// 128). **Only a tile a clip or the page edge cuts in x can move at all**, which is why one page
/// of 956 does and why the 4× lane does not move at all — and it is a page joining the oracle
/// rather than a bound being crossed, which is the direction this list is allowed to move in.
const DIFFERS_AT_THE_EDGES: [&str; 5] = [
    "bug1743245.pdf",
    "endchar.pdf",
    "issue11473.pdf",
    "issue2884_reduced.pdf",
    "pr12564.pdf",
];

/// Pages where the difference is **structural**: similarity at or below 0.99.
///
/// The name is the *classifier's*, and every page here has now been examined; none is an
/// open question. What left, and why: `issue4260_reduced.pdf` (§10.7.4's degenerate fills)
/// moved to the edge group; `issue2177.pdf`, `issue6769.pdf`, `issue6769_no_matrix.pdf` and
/// `bug946506.pdf` agreed when anisotropically-transformed strokes started outlining in path
/// space; `issue10572.pdf` and `issue14165.pdf` agreed when quorra raised its ramp sampling
/// from 256 to 4096 texels — a banded shading's hard stop boundary snapped to the coarser
/// grid and sat ~3.5 px off on a page-spanning axis.
///
/// **Three more left in the three-hundred-and-eighty-ninth**, when §10.7.4's coverage rule
/// reached a sub-pixel shape on the processor (ADR 0226): `160F-2019.pdf` (mean 1.3399, ssim
/// 0.96854), `issue7454.pdf` (1.8493, 0.97706) and `issue840.pdf` (0.8886, 0.99000). All three
/// were named in the paragraph below as pages where nothing moved and only a uniform sub-step
/// coverage difference dragged the score down — which is exactly what a quantised coverage *is*,
/// and it stopped being one backend's when the two started answering a thin rectangle with its
/// own area.
///
/// **`issue21068.pdf` and `bug1844583.pdf` arrived, and `knockout_groups_test.pdf` left, with
/// ADR 0285's inclusive quantum boundary** — see the note on the list above, which holds the
/// argument for both. `issue21068.pdf` is worth the second mention: it is the comb-field page
/// that left this list in the two-hundred-and-seventh session when a redundant clip came off,
/// and its separators are `1 w` rules, so it is exactly the population the change moves. Against
/// the *references* it improved — mean 10.06 → 9.42, ssim 0.8183 → 0.8413 — while moving away
/// from quorra, which is the shape of a page where the two backends' coverage quanta now show on
/// more edges rather than one of them drawing the wrong thing.
///
/// What stays, measured pixel by pixel: four pages (`copy_paste_ligatures`,
/// `issue4402_reduced`, `issue18030`, `issue15150`) have **zero**
/// pixels differing by more than 64 of 255 — miniature or enormous pages where uniform
/// sub-step coverage differences drag the similarity score below the threshold without any
/// shape moving. `knockout_groups_test.pdf` differs on 2 pixels. The rest are the
/// hairline-and-texture family — sub-half-pixel rules
/// (`issue16038`, `issue12295`, `issue20232`, `22060_A1_01_Plans`; the first of those went from
/// mean 6.1643 to **6.5359** in the three-hundred-and-seventy-fourth session, when its pattern's
/// rule became one 0.53-pixel stroke instead of two clipped 0.27-pixel halves — the same ink for
/// two rasterisers to distribute differently, in one mark instead of two, and the page moved 4%
/// toward the geometry's own answer while doing it, ADR 0213 — **and back to 1.8563 in the
/// three-hundred-and-eighty-ninth**, ssim 0.90046 to 0.97723, when that one 0.53-pixel stroke
/// stopped being a hairline on the processor and became the fill of its own outline: it is the
/// largest single movement this list has recorded, and it is the two backends agreeing rather
/// than either moving toward the other, ADR 0226), 8-px text
/// (`issue16316`, `standard_fonts`), halftone photographs under the stated linear-sampler
/// variance (`issue269_2`) — where the two rasterisers put the same ink on different sides
/// of a pixel boundary. **`22060_A1_01_Plans.pdf` is unmoved to four decimals and that is a
/// finding rather than an omission**: its rules are diagonals and polylines, which
/// `pdf_render::sub_pixel` declines by name, so the corpus's largest page of sub-pixel line work
/// is the one the new rule does not reach. Matching `tiny-skia`'s sub-pixel distribution
/// byte-for-byte would be curve-fitting to another renderer, which quorra's charter forbids;
/// they stay listed so a *growth* in their numbers is still a finding.
///
/// **Four arrived when a clip stopped multiplying into the mark's own coverage (ADR 0355)**, and
/// they are one population: a widget appearance whose **border sits on its own `/BBox`**.
/// `bug1844576.pdf`, `issue16473.pdf` and `issue18823.pdf` are text fields and radio groups whose
/// box rule lands where §8.10.1 step c) clips the form, and `bug1978317.pdf` is a page of dense
/// small text inside one. §10.7.4 states a clip as a set of pixels, so a clip that *contains* a
/// mark takes nothing from it; `render-cpu` now composes the two with `min` where `tiny-skia`
/// multiplied, and quorra multiplies inside the graphics library. That is the same divergence ADR
/// 0280 recorded one step earlier, and `doc/QUORRA_FEEDBACK.md`'s twenty-fourth section is the ask.
///
/// **`issue18823.pdf` left that population in the six-hundred-and-forty-sixth session, and it left
/// by the processor moving to the device rather than the other way about.** A `/BBox` clip is an
/// axis-aligned rectangle, and `render-cpu` measured its region with `tiny-skia`'s supersampled
/// path converter, whose answer at an axis-aligned edge is a quarter of a pixel; the graphics
/// device has never had that quantum and tracks the fraction to a level of 255. ADR 0476 gives
/// both a rectangular fill and a rectangular clip region the coverage §10.7.4's own definition of
/// a pixel implies, so on a page whose only disagreement was that quantum the two backends now
/// answer alike. The other three stay: their disagreement is the clip *product* ADR 0355 narrowed
/// rather than the coverage this measures, which is why one of four moved and not four.
///
/// **This list's own note about `issue21068.pdf` is the same finding two hundred sessions early**:
/// it left in the two-hundred-and-seventh session because a *redundant* clip came off its comb
/// separators, and the sentence written then — "the anti-aliased clip was costing each of them a
/// fraction of a pixel" — is what the general rule now answers for every such page.
/// `issue16038.pdf` moved the other way, mean 1.3235 → 1.2808, because a tiling cell's clip is
/// also coincident with the rule it admits.
///
/// **A fifth of that population arrived in the six-hundred-and-ninetieth, when the same
/// composition reached a *stroke* (ADR 0535).** `issue19083.pdf` is one widget appearance whose
/// `/BBox` is `[0 0 125.25 20]` and whose whole content is `0.5 0.5 124.2502 19 re s` at the
/// default `1 w` — a border rule whose outer edge is the `/BBox` §8.10.1 step c) clips it by, on
/// three sides exactly. It was the four above with `f` instead of `s`, and it stayed out of this
/// list only because a stroke went through `tiny_skia::PixmapMut::stroke_path`, which multiplies
/// the mask into the mark. `render-cpu` now draws such a stroke as the fill of its own outline and
/// composes it by `min`; quorra multiplies at the mark inside the graphics library, on both
/// operators. Against the *references* the page improved — mean 7.45 → 7.38, worst tile 16.44 →
/// 15.32, ssim 0.8333 → 0.8543 — which is the same direction and the same reason the four took,
/// and `doc/QUORRA_FEEDBACK.md` section 24 is still the ask.
///
/// **ADR 0735 reaches neither of the two pattern-shaped names here, which had to be checked
/// rather than assumed** (ADR 0738). The eight-hundred-and-second session drew a stroke whose
/// colour is a tiling pattern by cutting the tiles to the alpha of a group holding that stroke
/// (§11.5.2), and it names `issue12295.pdf` and `issue16038.pdf` once each, in a measurement
/// line recording that neither moved. Neither can: `issue12295.pdf` states no `/Pattern` and no
/// `SCN` anywhere, and `issue16038.pdf` installs its cells with `scn` under a `B` whose stroking
/// colour is a flat `0 G`, so both take the fill arm, which that session left unchanged in every
/// particular. What this gate prints for `issue16038.pdf` today is mean **1.2328** at ssim
/// 0.98798 — below the 1.2808 recorded above, so the two rasterisers have come *closer* on it
/// since, and the figures above are the movements their own sessions measured.
const DIFFERS_IN_SHAPE: [&str; 17] = [
    "22060_A1_01_Plans.pdf",
    "bug1844576.pdf",
    "bug1844583.pdf",
    "bug1978317.pdf",
    "copy_paste_ligatures.pdf",
    "issue12295.pdf",
    "issue15150.pdf",
    "issue16038.pdf",
    "issue16316.pdf",
    "issue16473.pdf",
    "issue18030.pdf",
    "issue19083.pdf",
    "issue20232.pdf",
    "issue21068.pdf",
    "issue269_2.pdf",
    "issue4402_reduced.pdf",
    "standard_fonts.pdf",
];

/// The two groups as one list, sorted as the run produces them.
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
    /// quorra refused the display list.
    Refused(String),
    /// Nothing to compare: no page, no display list, or a target past the budget.
    Skipped,
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
    let only = std::env::var("PDFVIEWER_QUORRA_ONLY").ok();
    let files = selected(files, only.as_deref());

    let mut quorra = settings.rasterizer();
    announce(&quorra, files.len(), settings);

    let started = Instant::now();
    let mut agreed = 0usize;
    let mut skipped = 0usize;
    let mut differing = Vec::new();
    let mut refused = Vec::new();
    let mut worst: Vec<(f64, String)> = Vec::new();
    let (mut cpu_total, mut gpu_total) = (Duration::ZERO, Duration::ZERO);
    let mut ratios: Vec<f64> = Vec::new();

    for path in &files {
        let name = path.file_name().map_or_else(
            || path.display().to_string(),
            |n| n.to_string_lossy().into(),
        );
        let Some(list) = page_one(path) else {
            skipped = skipped.saturating_add(1);
            continue;
        };
        let Ok(target) = TargetSpec::for_page(&list, scale, PIXEL_BUDGET) else {
            skipped = skipped.saturating_add(1);
            continue;
        };

        let at = Instant::now();
        let Ok(cpu) = CpuRasterizer::new().rasterize(&list, target) else {
            // A page the oracle itself refuses says nothing about the backend under test.
            skipped = skipped.saturating_add(1);
            continue;
        };
        let cpu_took = at.elapsed();
        let at = Instant::now();
        let ours = quorra.rasterize(&list, target);
        let gpu_took = at.elapsed();
        let verdict = outcome(&cpu, ours);
        // **A refused frame is a fast frame**, and counting one as a time would report a
        // backend that draws nothing as the quickest there is: at four times the page's own
        // scale, 533 of these documents are refused and the median ratio came back as 0.00×
        // before this line existed. Only a frame that was produced is timed.
        if !matches!(verdict, Outcome::Refused(_)) {
            cpu_total = cpu_total.saturating_add(cpu_took);
            gpu_total = gpu_total.saturating_add(gpu_took);
            if cpu_took > Duration::from_millis(1) {
                // Below a millisecond the clock is measuring itself; a ratio taken there is
                // noise with a decimal point.
                ratios.push(gpu_took.as_secs_f64() / cpu_took.as_secs_f64());
            }
        }

        match verdict {
            Outcome::Agrees => agreed = agreed.saturating_add(1),
            Outcome::Differs(how) => {
                write_artefacts(&name, &cpu, &quorra.rasterize(&list, target));
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
            Outcome::Skipped => skipped = skipped.saturating_add(1),
        }
    }

    report(
        &mut Tally {
            agreed,
            skipped,
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
    );
}

/// Holds the run to whichever lists it is the measurement for.
fn hold(which: Ratchets, refused: &[String], differing: &[String]) {
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
                "the pages quorra draws differently from the oracle have changed"
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
    coverage: quorra_gpu::Coverage,
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
    /// **The quantum and the thread count are the only two of quorra's options this overrides**,
    /// and both default to what [`render_quorra::options`] ships: the gate draws what the viewer
    /// draws, because a gate that turns a shipped setting off is measuring a configuration nobody
    /// runs (ADR 0498). The coverage lane is not an option but a call, so it is set after.
    #[expect(
        clippy::panic,
        reason = "test code: a machine with no adapter cannot run this gate, and saying so is \
                  the intended failure"
    )]
    fn rasterizer(self) -> QuorraRasterizer {
        let mut quorra = QuorraRasterizer::with_options(&quorra_gpu::Options {
            glyph_quantum: self.quantum,
            encode_threads: self.threads,
            ..render_quorra::options()
        })
        .unwrap_or_else(|e| panic!("no adapter available for quorra: {e}"));
        quorra.set_coverage(self.coverage);
        quorra
    }
}

/// Which ratchets below the survey are checked, saying why when some are not.
///
/// The refusal lists and the differing lists are measured over the whole corpus, at [`SCALE`], on
/// quorra's default coverage lane, at the shipped glyph quantum — exactly, because a measurement
/// taken anywhere else is a different measurement — so each of those knobs turns them off. A list
/// held to equality over a subset would report every document the filter excluded as fixed, and a
/// list held over the *other* lane would report the two lanes' stated difference (quorra's ADR
/// 0016) as a change in this backend.
///
/// **One knob has a ratchet of its own since the four-hundred-and-seventy-eighth session**:
/// the same corpus at [`MAGNIFIED`] on the default lane holds [`REFUSED_BY_THE_DEVICE_AT_FOUR`]
/// beside [`REFUSED_BEFORE_THE_SCENE`]. Its doc
/// comment is the argument for why that list can be held now and could not be before.
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
        && coverage == quorra_gpu::Coverage::Cpu
        && quantum == render_quorra::options().glyph_quantum;
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
fn lane_name(coverage: quorra_gpu::Coverage) -> &'static str {
    match coverage {
        quorra_gpu::Coverage::Cpu => "cpu",
        quorra_gpu::Coverage::Gpu => "gpu",
        quorra_gpu::Coverage::Compute => "compute",
    }
}

/// Says which adapter and which build produced the numbers below them.
///
/// Both matter to every number this gate prints: a software adapter and a discrete GPU are
/// different machines, and a debug build is ~15× slower here.
fn announce(quorra: &QuorraRasterizer, documents: usize, settings: Settings) {
    let Settings {
        scale,
        coverage,
        threads,
        quantum,
    } = settings;
    println!("adapter: {}", quorra.adapter_description());
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

/// The corpus, or the part of it `PDFVIEWER_QUORRA_ONLY` names.
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
    skipped: usize,
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
        tally.skipped
    );
    println!(
        "  rasterisation: {cpu_total:.2?} on the CPU backend, {gpu_total:.2?} through quorra \
         (offscreen, readback included)"
    );
    ratios.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    if let Some(median) = ratios.get(ratios.len() / 2) {
        println!(
            "  median page: quorra takes {median:.2}× the CPU backend's time, over {} pages \
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
        .join("../../target/tmp/quorra")
        .join(name.trim_end_matches(".pdf"));
    if std::fs::create_dir_all(&root).is_err() {
        return;
    }
    for (what, raster) in [("cpu", cpu), ("quorra", ours)] {
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
fn outcome(cpu: &pdf_render::Raster, ours: Result<pdf_render::Raster, impl ToString>) -> Outcome {
    let ours = match ours {
        Ok(raster) => raster,
        Err(why) => return Outcome::Refused(why.to_string()),
    };
    let Ok(c) = raster_compare::compare(cpu, &ours) else {
        return Outcome::Skipped;
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

/// The display list of a document's first page, or `None` where there is not one.
fn page_one(path: &Path) -> Option<DisplayList> {
    let bytes = std::fs::read(path).ok()?;
    let document = Document::open(bytes).ok()?;
    let pages = pdf_model::Pages::new(&document);
    let page = pages.get(0)?;
    Some(pdf_model::content::interpret(&document, &page).display_list)
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
