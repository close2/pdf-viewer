//! A draw that can be abandoned, and a draw that is not.
//!
//! `pdf_render::Interrupt` exists because since ADR 0633 a page usually crosses the confinement
//! as *marks* and the **host** draws them — in the unconfined process, on the host's own thread,
//! where `doc/todo/34` §3's cancel is a kill of something that has already finished. ADR 0650.
//!
//! Two properties, and the first is the one that costs something if it is wrong. This backend is
//! the correctness oracle every corpus and oracle verdict is taken from, so a check added to its
//! command loop must not change a byte of what it draws; and an interrupt that has not been
//! raised must therefore be *exactly* no interrupt at all. That is asserted here over the scenes
//! `strip_parallelism.rs` uses, which is where a group, a soft mask, a knockout and a curve each
//! render through a different surface.
//!
//! The second is that a raised interrupt is honoured **before the target is allocated**, which is
//! a deterministic test of a path that otherwise only shows up under a race: the target below is
//! a gibibyte and a half of pixels, and a run that allocated it would be measured in the time it
//! takes rather than in microseconds.
//!
//! **What is not here is the mid-draw case**, deliberately. Interrupting a draw already in
//! progress needs a draw long enough to still be going, which is a hostile document rather than a
//! scene: `viewer-confined`'s `a_host_drawing_marks_that_will_not_finish_interrupts_its_own_draw`
//! is that test, on marks that came out of a real confined worker.

// No lint exception: every `expect` below is inside a `#[test]`, which `clippy.toml`'s
// `allow-expect-in-tests` already permits, and an `#[expect]` that is never needed is itself an
// error (`doc/traps/instruments-and-reports.md` trap 7).

use pdf_render::{DisplayList, Interrupt, Rasterizer as _, TargetSpec, Transform};
use render_cpu::CpuRasterizer;

/// Pixel budget for a target; far above anything these tests request.
const GENEROUS: u64 = 1 << 30;

/// The scenes, chosen for `strip_parallelism.rs`'s reasons and shared with it by name.
fn scenes() -> [(&'static str, DisplayList); 7] {
    [
        ("basic", test_scenes::basic()),
        ("curves", test_scenes::curves()),
        ("diagonal_stroke", test_scenes::diagonal_stroke()),
        ("transparency_group", test_scenes::transparency_group()),
        ("knockout_group", test_scenes::knockout_group()),
        (
            "knockout_stated_shape",
            test_scenes::knockout_stated_shape(),
        ),
        ("soft_mask", test_scenes::soft_mask()),
    ]
}

/// An interrupt nobody raised changes nothing about the page.
#[test]
fn a_draw_with_an_unraised_interrupt_is_the_draw_without_one() {
    for (name, list) in scenes() {
        let target = TargetSpec::for_page(&list, 4.0, GENEROUS).expect("a valid target");
        let plain = CpuRasterizer::new()
            .rasterize(&list, target)
            .expect("the scene is supported")
            .data;
        let watched = CpuRasterizer::new()
            .interruptible(Interrupt::new())
            .rasterize(&list, target)
            .expect("the scene is supported")
            .data;
        let differing = plain
            .iter()
            .zip(&watched)
            .filter(|(one, other)| one != other)
            .count();
        assert_eq!(
            differing,
            0,
            "{name}: {differing} of {} bytes differ under an interrupt nobody raised",
            plain.len()
        );
    }
}

/// A raised interrupt refuses, and does so before the pixmap exists.
///
/// The target is 20 000 × 20 000 — 1.6 GB of RGBA — so a refusal that came *after* the allocation
/// would be visible as the time it takes to ask the kernel for it and to fill it, or as a failure
/// to get it at all on a loaded machine. Neither is what this asserts directly, because a timing
/// assertion in a unit test is a test of the scheduler; what it asserts is that the refusal is
/// [`pdf_render::BackendError::Interrupted`] rather than the allocation failure a machine without
/// that much memory would otherwise report.
#[test]
fn an_interrupt_raised_before_a_draw_refuses_before_the_target_is_allocated() {
    let list = test_scenes::basic();
    let target = TargetSpec {
        width: 20_000,
        height: 20_000,
        transform: Transform::scale(1.0, 1.0),
    };
    let interrupt = Interrupt::new();
    interrupt.raise();
    assert!(interrupt.raised());

    let refusal = CpuRasterizer::new()
        .interruptible(interrupt)
        .rasterize(&list, target)
        .expect_err("a raised interrupt refuses");
    assert!(
        refusal.to_string().contains("interrupted"),
        "and it says which refusal it is rather than reporting the allocation: {refusal}"
    );
}
