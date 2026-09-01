//! Every backend that composites a transparency group refuses a page whose groups would
//! blit past [`pdf_render::MAX_GROUP_BLIT_PIXELS`], and each says so by the same name.
//!
//! **Here rather than in one backend's own tests because the claim is about all three.**
//! `render-quorra` is the only crate in this workspace that dev-depends on the other two, so
//! this is where a sentence counting this tree's rasterisers can be checked against them
//! (`doc/todo/01`'s `--bin parts` sweep is about exactly that class of claim). A bound one
//! backend holds and another does not is worse than no bound: the same document then draws
//! in one window and hangs in the next, and no gate that rasterises with the CPU oracle can
//! see it — `doc/traps/pixels-and-rasterisers.md` trap 12b's own shape.
//!
//! The GPU arm needs **no device at all**: `render_gpu::build_scene` is the tier-2 entry a
//! window comes through, it takes a display list, and the bound is asked there — which is
//! also what this file pins, since a check placed in `Rasterizer::rasterize` alone would
//! leave the path a person uses unguarded.

#![expect(
    clippy::expect_used,
    reason = "test code, and these live in helpers the allow-expect-in-tests config cannot see"
)]

use pdf_render::{
    BackendError, BlendMode, Command, DisplayList, MAX_GROUP_BLIT_PIXELS, Rasterizer as _, Size,
    TargetSpec,
};

/// Pixel budget for a target; far above anything this file requests.
const GENEROUS: u64 = 1 << 30;

/// A4 at 72 dpi, and enough unclipped groups over it to pass the bound by one blit.
///
/// The shape `poppler-978-0.pdf` states, in miniature and with no file to read: groups side
/// by side, each spanning the sheet, each blitting the whole target once.
fn a_page_past_the_bound() -> (DisplayList, TargetSpec) {
    let mut list = DisplayList::new(Size::new(595.0, 842.0));
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("A4 at 1:1 fits the budget");
    let area = u64::from(target.width).saturating_mul(u64::from(target.height));
    let count = MAX_GROUP_BLIT_PIXELS
        .checked_div(area)
        .and_then(|whole| whole.checked_add(1))
        .expect("an A4 page at 1:1 has an area");
    for _ in 0..count {
        list.push(a_group_spanning_the_page());
    }
    (list, target)
}

/// One isolated, unclipped group: a blit of the whole target, whatever the target is.
fn a_group_spanning_the_page() -> Command {
    Command::Group {
        commands: Vec::new(),
        alpha: 1.0,
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
        isolated: true,
        knockout: false,
        alpha_is_shape: false,
        blending: None,
    }
}

/// An ordinary page with a handful of groups, which every backend must still draw.
///
/// The other half of the claim, and the half a bound that fired on everything would pass
/// without: a refusal is only as good as what it admits.
///
/// **Four groups rather than the bound itself, deliberately.** A page sized exactly at the
/// bound is 34.4 G blitted pixels and the CPU backend spends four minutes on it — which is
/// the bound working, not a defect, and is precisely why it is not drawn here. Where the
/// refusal turns over, at `>` and not `>=`, is pinned by `pdf_render::group_cost`'s own unit
/// tests, which draw nothing at all.
fn an_ordinary_page() -> (DisplayList, TargetSpec) {
    let mut list = DisplayList::new(Size::new(595.0, 842.0));
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("A4 at 1:1 fits the budget");
    for _ in 0..4 {
        list.push(a_group_spanning_the_page());
    }
    (list, target)
}

/// The words every backend's refusal has to carry, whichever error type wraps it.
///
/// Asserted on the sentence rather than on the variant because two of the three backends
/// reach it through a wrapper of their own, and because what a host prints is the sentence.
/// It names the *measure* and the demand, which no other refusal in this tree states — trap
/// 27: an assertion is only as good as what it excludes, and `GroupsTooDeep` says "over the
/// limit of" too.
fn names_the_bound(said: &str) {
    assert!(
        said.contains("transparency groups would blit"),
        "the refusal does not name the group-blit bound: {said}"
    );
}

#[test]
fn the_cpu_backend_refuses_a_page_past_the_bound() {
    let (list, target) = a_page_past_the_bound();
    let Err(error) = render_cpu::CpuRasterizer::new().rasterize(&list, target) else {
        panic!("the CPU backend drew a page past the group-blit bound");
    };
    names_the_bound(&error.to_string());
    assert!(
        matches!(
            error,
            render_cpu::CpuRasterError::Target(BackendError::GroupsTooCostly { .. })
        ),
        "the CPU backend refused for another reason: {error}"
    );
}

#[test]
fn the_cpu_backend_still_draws_an_ordinary_page() {
    let (list, target) = an_ordinary_page();
    render_cpu::CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("four groups are far inside the bound");
}

/// The Vello backend, through the entry a *window* uses — no device, no adapter, no queue.
#[test]
fn the_vello_backend_refuses_a_page_past_the_bound_with_no_device() {
    let (list, target) = a_page_past_the_bound();
    // `vello::Scene` is not `Debug`, so the `Ok` arm is named rather than unwrapped.
    let Err(error) = render_gpu::build_scene(&list, target, &render_gpu::SoftMaskRasters::none())
    else {
        panic!("the Vello backend built a scene for a page past the group-blit bound");
    };
    names_the_bound(&error.to_string());
}

#[test]
fn the_vello_backend_still_builds_a_scene_for_an_ordinary_page() {
    let (list, target) = an_ordinary_page();
    render_gpu::build_scene(&list, target, &render_gpu::SoftMaskRasters::none())
        .expect("four groups are far inside the bound");
}

/// quorra, which needs a device — the software one, for the reason `headless_quorra.rs`
/// gives: a suite that skips reports success while verifying nothing.
#[test]
fn the_quorra_backend_refuses_a_page_past_the_bound() {
    let mut quorra = match render_quorra::QuorraRasterizer::new_headless_software() {
        Ok(rasterizer) => rasterizer,
        Err(error) => panic!(
            "no software adapter for quorra: {error}\n\
             Install mesa's lavapipe (mesa-vulkan-drivers). This test does not skip."
        ),
    };
    let (list, target) = a_page_past_the_bound();
    // Named rather than unwrapped: an `expect_err` here prints a whole page of pixels.
    let Err(error) = quorra.rasterize(&list, target) else {
        panic!("quorra drew a page past the group-blit bound");
    };
    names_the_bound(&error.to_string());
    assert!(
        matches!(
            error,
            render_quorra::QuorraRasterError::Target(BackendError::GroupsTooCostly { .. })
        ),
        "quorra refused for another reason: {error}"
    );
}
