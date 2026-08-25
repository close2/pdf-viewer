//! Fuzzes the display-list decoder — ADR 0607's payload, read by the host from the confined side.
//!
//! **Why this target exists.** ADR 0607 settled that a window on the confinement receives
//! *display lists* rather than pixels, because a graphics device cannot be confined at all. That
//! moves a whole page of geometry across the boundary and gives the **unconfined** host a parser
//! over bytes the confined process chose — the same shape as `confined_wire`, over a much larger
//! vocabulary: four shared tables, a clip table whose entries name each other, a nested command
//! tree, sixteen blend modes, four shading geometries and a soft mask holding commands of its
//! own. `CLAUDE.md` principle 3 says a parser is fuzzed from its first commit, and this is that
//! commit.
//!
//! Four properties are under test, and the last two are what a smaller target would miss.
//!
//! **Nothing panics**, over any byte sequence. A panic here is the confined process reaching
//! into the host, which is the one thing the boundary exists to prevent. Stack exhaustion counts:
//! the format nests, and the decoder's bound on that nesting is what this is asked to break.
//!
//! **Nothing allocates from a claim.** The fuzz profile keeps overflow checks on, and every
//! count in this format is checked against the bytes that could hold it — per table, at the
//! smallest record that table admits — before anything is reserved.
//!
//! **Every identifier a decoded list holds points at something.** A `ClipId`, a `SoftMaskId`, a
//! path index and a shading index are all indices into tables, and a host rasterises what it is
//! given: a message naming clip 4000 of a table of two must be a refusal in the decoder rather
//! than an assumption in `render-quorra`. Asserted here rather than trusted, so that deleting
//! the check fails this target.
//!
//! **Anything this decoder accepts, this encoder can write, and writing it reads back the same.**
//! The two halves of a codec are two statements of one format, and the way they drift is a case
//! the encoder never produces and the decoder happily accepts. This target closes that with
//! bytes an attacker chose rather than with a fixture.
//!
//! # Why the two equality assertions compare *bytes* rather than lists
//!
//! `DisplayList`'s `PartialEq` is ultimately `f32`'s, and `f32`'s is **not reflexive**: a message
//! may state NaN, and NaN is equal to nothing including itself. So `assert_eq!` on two decodes of
//! one message fails on a value that decoded perfectly — which is what this target reported on its
//! first run, at 750 executions. Re-encoding is a total function, so comparing the bytes says the
//! same thing about determinism and about the round trip, and says it for every input rather than
//! for the finite ones. **The decoder deliberately does not refuse a non-finite number**, and ADR
//! 0626 section 7 has the reason: the confined path must draw the page the in-process path draws,
//! so a value the interpreter can produce may not be refused at the boundary.

#![no_main]

use libfuzzer_sys::fuzz_target;
use pdf_render::{Command, DisplayList, ImageSource, Paint, ShadingKind, SoftMaskId};
use viewer_confined::wire;

/// Everything a decoded list must satisfy, whatever the bytes said.
fn check(list: &DisplayList) {
    let clips = list.clip_count();
    let masks = list.soft_mask_count();

    // A clip's parent sits strictly before it, which is what `DisplayList::add_clip` guarantees
    // of any table it built and what keeps `clip_bounds` from walking a cycle.
    for at in 0..clips {
        let Ok(index) = u32::try_from(at) else {
            continue;
        };
        let clip = list
            .clip(pdf_render::ClipId::new(index))
            .expect("a clip the table's own length names");
        if let Some(parent) = clip.parent {
            assert!(
                parent.index() < at,
                "clip {at} names {} as its parent",
                parent.index()
            );
        }
    }

    for command in list.commands() {
        walk(command, clips, masks);
    }
    for at in 0..masks {
        let Ok(index) = u32::try_from(at) else {
            continue;
        };
        let mask = list
            .soft_mask(SoftMaskId::new(index))
            .expect("a mask the table's own length names");
        for command in &mask.commands {
            walk(command, clips, masks);
        }
    }
    if let Some(black) = list.black() {
        check(black);
    }
}

/// One command and everything under it.
fn walk(command: &Command, clips: usize, masks: usize) {
    if let Some(clip) = command.clip() {
        assert!(clip.index() < clips, "a command names clip {} of {clips}", clip.index());
    }
    if let Some(mask) = command.mask() {
        assert!(
            mask.index() < masks,
            "a command names soft mask {} of {masks}",
            mask.index()
        );
    }
    match command {
        Command::Fill { paint, .. } | Command::Stroke { paint, .. } => check_paint(paint),
        Command::Image { image, .. } => match image {
            // The invariant a backend indexes by: a stated grid its samples do not fill would be
            // a read past the end of a buffer in a rasteriser that trusted the dimensions.
            ImageSource::Decoded(decoded) => assert!(
                decoded.is_consistent(),
                "an image crossed as {}x{} with {} bytes of samples",
                decoded.width,
                decoded.height,
                decoded.data.len()
            ),
            // Neither producer can be decoded at all, so one here would be a decoder inventing a
            // value the format does not carry.
            other => panic!("a deferred producer decoded from bytes: {other:?}"),
        },
        Command::Group {
            commands, blending, ..
        } => {
            if let Some(pair) = blending {
                for element in &pair.black {
                    walk(element, clips, masks);
                }
            }
            for element in commands {
                walk(element, clips, masks);
            }
        }
        Command::Shaped { object, shape } => {
            walk(object, clips, masks);
            walk(shape, clips, masks);
        }
        _ => {}
    }
}

fn check_paint(paint: &Paint) {
    let Paint::Shading(shading) = paint else {
        return;
    };
    match shading.kind.as_ref() {
        // `Ramp`'s own documentation says its stops are never empty and every reader of one
        // depends on it, so a decoded ramp with none would be a value no interpreter produces.
        ShadingKind::Axial { ramp, .. } | ShadingKind::Radial { ramp, .. } => {
            assert!(!ramp.stops.is_empty(), "a shading crossed with no stops");
        }
        ShadingKind::Mesh { ramp, .. } => {
            if let Some(ramp) = ramp {
                assert!(!ramp.stops.is_empty(), "a mesh crossed with an empty ramp");
            }
        }
        other => panic!("a deferred producer decoded from bytes: {other:?}"),
    }
}

fuzz_target!(|data: &[u8]| {
    let Ok(list) = wire::display_list(data) else {
        return;
    };
    let written = wire::encode_display_list(&list)
        .expect("whatever this decoder accepts, this encoder can write");

    // Decoding is a function of the bytes: this reader carries a cursor and four tables, and a
    // decoder that leaked state between elements is exactly the defect one pass cannot see.
    let again = wire::display_list(data).expect("the same bytes decode the same way");
    let again = wire::encode_display_list(&again).expect("a list this decoder accepted");
    assert_eq!(written, again, "one message decoded two ways");

    check(&list);

    // The encoding is a canonical form: what this encoder wrote decodes to a list that encodes
    // to the same bytes again. A field dropped, reordered or widened on one side fails here.
    let back = wire::display_list(&written).expect("what this encoder wrote");
    let back = wire::encode_display_list(&back).expect("a list this decoder accepted");
    assert_eq!(
        written, back,
        "a list changed by being written down and read back"
    );
});
