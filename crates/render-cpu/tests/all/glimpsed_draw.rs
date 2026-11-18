//! A draw somebody looks at while it is being built, and what they are shown.
//!
//! `render_cpu::Glance` lets a host ask a draw in progress for its picture so far (ADR 1825).
//! Three properties, in the order a mistake in each would cost:
//!
//! 1. **Looking changes nothing about the page.** This backend is the oracle, so a page drawn
//!    while it is looked at at every command must be the page drawn unobserved, byte for byte.
//! 2. **What is shown is a prefix of the page's own drawing**: the glimpse taken before command
//!    *k* is the page drawn through its first *k* commands and finished the same way — so a person
//!    is shown a state the page passes through, never a picture it does not.
//! 3. **A strip that ends while somebody is looking shows its final rows**, and those rows tile
//!    the page exactly: on a page that grants cuts, the strips land one by one.
//!
//! Over the scenes `strip_parallelism.rs` and `interrupted_draw.rs` use, for their reasons: a
//! group, a soft mask, a knockout and a curve each render through a different surface.

#![expect(
    clippy::expect_used,
    reason = "test code: a rasteriser that refuses one of these scenes should fail loudly, and the \
              helpers that draw them are outside the `#[test]`s `clippy.toml` already permits"
)]

use std::sync::{Arc, Mutex};

use pdf_render::{DisplayList, Rasterizer as _, TargetSpec};
use render_cpu::{CpuRasterizer, Glance, Glimpse};

/// Pixel budget for a target; far above anything these tests request.
const GENEROUS: u64 = 1 << 30;

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

/// Draws `list` at `strips`, asked for a look before it starts and again at every glimpse — so
/// every top-level command is followed by one.
fn looked_at(list: &DisplayList, target: TargetSpec, strips: u32) -> (Vec<u8>, Vec<Glimpse>) {
    let glance = Glance::new();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let asking = glance.clone();
    let into = Arc::clone(&seen);
    glance.ask();
    let raster = CpuRasterizer::new()
        .with_strips(strips)
        .showing(glance, move |glimpse| {
            asking.ask();
            into.lock().expect("no panic holds it").push(glimpse);
        })
        .rasterize(list, target)
        .expect("the scene is supported");
    let glimpses = std::mem::take(&mut *seen.lock().expect("no panic holds it"));
    (raster.data, glimpses)
}

fn plain(list: &DisplayList, target: TargetSpec) -> Vec<u8> {
    CpuRasterizer::new()
        .with_strips(1)
        .rasterize(list, target)
        .expect("the scene is supported")
        .data
}

#[test]
fn a_page_looked_at_every_command_is_the_page_drawn_unobserved() {
    for (name, list) in scenes() {
        let target = TargetSpec::for_page(&list, 4.0, GENEROUS).expect("a valid target");
        let unobserved = plain(&list, target);
        for strips in [1, 3] {
            let (observed, glimpses) = looked_at(&list, target, strips);
            assert!(
                observed == unobserved,
                "{name} at {strips} strip(s): looking changed the page"
            );
            assert!(!glimpses.is_empty(), "{name}: nobody answered the look");
        }
    }
}

#[test]
fn a_glimpse_is_the_page_drawn_through_the_commands_before_it() {
    for (name, list) in scenes() {
        let target = TargetSpec::for_page(&list, 4.0, GENEROUS).expect("a valid target");
        let (_, glimpses) = looked_at(&list, target, 1);
        let commands = list.commands();
        assert_eq!(
            glimpses.len(),
            commands.len() + 1,
            "{name}: one glimpse before each command and one at the end"
        );
        for glimpse in &glimpses {
            assert_eq!((glimpse.top, glimpse.rows.height), (0, target.height));
            let prefix = list.shape_list(commands.iter().take(glimpse.drawn).cloned().collect());
            assert!(
                glimpse.rows.data == plain(&prefix, target),
                "{name}: the glimpse after {} of {} commands is not the page drawn that far",
                glimpse.drawn,
                glimpse.of
            );
        }
    }
}

#[test]
fn the_strips_final_glimpses_tile_the_page() {
    for (name, list) in scenes() {
        let target = TargetSpec::for_page(&list, 4.0, GENEROUS).expect("a valid target");
        let (observed, glimpses) = looked_at(&list, target, 3);
        let mut finals: Vec<&Glimpse> = glimpses
            .iter()
            .filter(|glimpse| glimpse.drawn == glimpse.of)
            .collect();
        finals.sort_by_key(|glimpse| glimpse.top);
        let mut tiled = Vec::new();
        let mut next = 0;
        for glimpse in finals {
            assert_eq!(
                glimpse.top, next,
                "{name}: the strips leave a gap or overlap"
            );
            next += glimpse.rows.height;
            tiled.extend_from_slice(&glimpse.rows.data);
        }
        assert_eq!(
            next, target.height,
            "{name}: the strips do not reach the foot"
        );
        assert!(
            tiled == observed,
            "{name}: the final strips are not the page"
        );
    }
}

#[test]
fn a_draw_nobody_asks_to_see_sends_nothing() {
    let list = test_scenes::basic();
    let target = TargetSpec::for_page(&list, 4.0, GENEROUS).expect("a valid target");
    let sent = Arc::new(Mutex::new(0_usize));
    let count = Arc::clone(&sent);
    let raster = CpuRasterizer::new()
        .showing(Glance::new(), move |_| {
            *count.lock().expect("no panic holds it") += 1;
        })
        .rasterize(&list, target)
        .expect("the scene is supported");
    assert_eq!(*sent.lock().expect("no panic holds it"), 0);
    assert!(
        raster.data == plain(&list, target),
        "a draw nobody looked at changed"
    );
}
