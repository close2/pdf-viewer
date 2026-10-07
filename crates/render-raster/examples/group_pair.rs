//! What a four-component group's black list holds that its chromatic list does not, counted.
//!
//! # What it is for
//!
//! A transparency group whose blending space has four components is one content stream
//! interpreted twice (ISO 32000-2 §11.3.4, §11.4.7; `pdf_render::GroupBlending::FourComponents`),
//! and the second interpretation is most of a turn on `bug1721218_reduced.pdf` (ADR 1632
//! section 1). Drawing the black list *from* the chromatic one instead of interpreting again is
//! exact only where every field of every black command is a function of the chromatic list. This
//! counts, field by field, where the two lists differ, and then asks the one question the design
//! turns on: **is each black paint a function of the chromatic paint beside it?** — a chromatic
//! colour paired with two different black colours says the black half needs something the
//! chromatic list does not carry (ADR 1645).
//!
//! # Running it
//!
//! ```sh
//! cargo run --profile gates -p render-raster --example group_pair -- \
//!     doc/pdf.js/test/pdfs/bug1721218_reduced.pdf 1
//! ```
//!
//! The arguments are a path and a 1-based page number. Every four-component group's pair on the
//! page is walked, at every depth of the list's groups; a soft mask's own pair
//! (`pdf_render::BlackHalf`) is not, and a mask a pair names is compared by value.

#![expect(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "a measurement example: a missing document must stop the run rather than print a \
              figure about nothing, and the lines on stdout are the point"
)]

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use pdf_render::{Command, DisplayList, Paint};
use pdf_syntax::Document;

/// One more, where a count cannot reach `usize::MAX` on any page that exists.
fn bump(count: &mut usize) {
    *count = count.saturating_add(1);
}

/// The tallies, by what was compared.
#[derive(Default)]
struct Count {
    /// Pairs of commands walked, by variant.
    pairs: BTreeMap<&'static str, usize>,
    /// Fields that differed, by `variant.field`.
    differing: BTreeMap<String, usize>,
    /// Each chromatic paint, as its bits, and every black paint it was paired with.
    paints: BTreeMap<String, BTreeSet<String>>,
    /// Shadings whose kind is one `Arc` in both lists.
    shared_kinds: usize,
    /// Mask pairs whose two masks are equal by value.
    masks_equal: usize,
    /// Mask pairs whose two masks differ.
    masks_differing: usize,
    /// Four-component pairs found.
    groups: usize,
}

impl Count {
    fn differ(&mut self, variant: &str, field: &str) {
        bump(
            self.differing
                .entry(format!("{variant}.{field}"))
                .or_default(),
        );
    }
}

fn variant(command: &Command) -> &'static str {
    match command {
        Command::Fill { .. } => "Fill",
        Command::Stroke { .. } => "Stroke",
        Command::Image { .. } => "Image",
        Command::Group { .. } => "Group",
        Command::Shaped { .. } => "Shaped",
        _ => "other",
    }
}

/// A paint as a key that compares by bits, shadings by the colours they carry.
fn paint_key(paint: &Paint) -> String {
    match paint {
        Paint::Solid(colour) => format!(
            "solid {:08x} {:08x} {:08x} {:08x}",
            colour.r.to_bits(),
            colour.g.to_bits(),
            colour.b.to_bits(),
            colour.a.to_bits()
        ),
        Paint::Shading(shading) => format!("shading {:?}", shading.kind),
        _ => "other".to_owned(),
    }
}

fn compare_paints(count: &mut Count, name: &'static str, ours: &Paint, theirs: &Paint) {
    if ours != theirs {
        count.differ(name, "paint");
    }
    if let (Paint::Shading(left), Paint::Shading(right)) = (ours, theirs) {
        if Arc::ptr_eq(&left.kind, &right.kind) {
            bump(&mut count.shared_kinds);
        }
        if left.transform != right.transform {
            count.differ(name, "paint.transform");
        }
    }
    count
        .paints
        .entry(paint_key(ours))
        .or_default()
        .insert(paint_key(theirs));
}

fn compare_masks(count: &mut Count, list: &DisplayList, ours: &Command, theirs: &Command) {
    let (Some(left), Some(right)) = (ours.mask(), theirs.mask()) else {
        return;
    };
    if left == right || list.soft_mask(left) == list.soft_mask(right) {
        bump(&mut count.masks_equal);
    } else {
        bump(&mut count.masks_differing);
    }
}

/// Compares the chromatic list `ours` with the black list `theirs`, pair by pair.
fn compare(count: &mut Count, list: &DisplayList, ours: &[Command], theirs: &[Command]) {
    if ours.len() != theirs.len() {
        count.differ("list", "length");
        return;
    }
    for (left, right) in ours.iter().zip(theirs) {
        let name = variant(left);
        bump(count.pairs.entry(name).or_default());
        if variant(right) != name {
            count.differ(name, "variant");
            continue;
        }
        if left.clip() != right.clip() {
            count.differ(name, "clip");
        }
        if left.mask() != right.mask() {
            count.differ(name, "mask id");
        }
        compare_masks(count, list, left, right);
        if left.blend() != right.blend() {
            count.differ(name, "blend");
        }
        compare_pair(count, list, name, (left, right));
    }
}

/// The fields of a fill or a stroke beside the ones every command has; `false` for any other pair.
fn compare_marks(
    count: &mut Count,
    name: &'static str,
    (left, right): (&Command, &Command),
) -> bool {
    match (left, right) {
        (
            Command::Fill {
                path,
                transform,
                fill_rule,
                paint,
                ..
            },
            Command::Fill {
                path: other_path,
                transform: other_transform,
                fill_rule: other_rule,
                paint: other_paint,
                ..
            },
        ) => {
            if !Arc::ptr_eq(path, other_path) {
                count.differ(name, "path (not one Arc)");
                if path != other_path {
                    count.differ(name, "path (by value)");
                }
            }
            if transform != other_transform {
                count.differ(name, "transform");
            }
            if fill_rule != other_rule {
                count.differ(name, "fill_rule");
            }
            compare_paints(count, name, paint, other_paint);
        }
        (
            Command::Stroke {
                path,
                transform,
                stroke,
                paint,
                ..
            },
            Command::Stroke {
                path: other_path,
                transform: other_transform,
                stroke: other_stroke,
                paint: other_paint,
                ..
            },
        ) => {
            if path != other_path {
                count.differ(name, "path (by value)");
            }
            if transform != other_transform {
                count.differ(name, "transform");
            }
            if stroke != other_stroke {
                count.differ(name, "stroke");
            }
            compare_paints(count, name, paint, other_paint);
        }
        _ => return false,
    }
    true
}

/// The fields of one pair beside the ones every command has.
fn compare_pair(
    count: &mut Count,
    list: &DisplayList,
    name: &'static str,
    (left, right): (&Command, &Command),
) {
    if compare_marks(count, name, (left, right)) {
        return;
    }
    match (left, right) {
        (
            Command::Image {
                image,
                transform,
                alpha,
                ..
            },
            Command::Image {
                image: other_image,
                transform: other_transform,
                alpha: other_alpha,
                ..
            },
        ) => {
            if image != other_image {
                count.differ(name, "image");
            }
            if transform != other_transform {
                count.differ(name, "transform");
            }
            if alpha.to_bits() != other_alpha.to_bits() {
                count.differ(name, "alpha");
            }
        }
        (
            Command::Group {
                commands,
                alpha,
                isolated,
                knockout,
                alpha_is_shape,
                blending,
                ..
            },
            Command::Group {
                commands: other_commands,
                alpha: other_alpha,
                isolated: other_isolated,
                knockout: other_knockout,
                alpha_is_shape: other_shape,
                blending: other_blending,
                ..
            },
        ) => {
            if alpha.to_bits() != other_alpha.to_bits() {
                count.differ(name, "alpha");
            }
            if (isolated, knockout, alpha_is_shape) != (other_isolated, other_knockout, other_shape)
            {
                count.differ(name, "flags");
            }
            if blending != other_blending {
                count.differ(name, "blending");
            }
            compare(count, list, commands, other_commands);
        }
        (
            Command::Shaped { object, shape },
            Command::Shaped {
                object: other_object,
                shape: other_shape,
            },
        ) => {
            compare(
                count,
                list,
                std::slice::from_ref(object),
                std::slice::from_ref(other_object),
            );
            compare(
                count,
                list,
                std::slice::from_ref(shape),
                std::slice::from_ref(other_shape),
            );
        }
        _ => count.differ(name, "unknown variant"),
    }
}

/// Finds every four-component pair under `commands` and compares it.
fn walk(count: &mut Count, list: &DisplayList, commands: &[Command]) {
    for command in commands {
        match command {
            Command::Group {
                commands: inner,
                blending,
                ..
            } => {
                if let Some(black) = blending
                    .as_deref()
                    .and_then(pdf_render::GroupBlending::black)
                {
                    bump(&mut count.groups);
                    compare(count, list, inner, black);
                }
                walk(count, list, inner);
            }
            Command::Shaped { object, .. } => walk(count, list, std::slice::from_ref(object)),
            _ => {}
        }
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("a document path");
    let index = args
        .next()
        .map_or(1, |n| n.parse::<usize>().expect("a 1-based page number"));
    let document = Document::open(std::fs::read(&path).expect("the document is readable"))
        .expect("the document opens");
    let pages = pdf_model::Pages::new(&document);
    let state = pdf_model::view::ViewState::of(&document);
    let fonts = pdf_model::content::FontCache::new();
    let page = pages
        .get(index.saturating_sub(1))
        .expect("the document has that page");
    let interpretation = pdf_model::content::interpret_with_fonts(&document, &page, &state, &fonts);
    let list = &interpretation.display_list;
    let mut count = Count::default();
    walk(&mut count, list, list.commands());
    println!("four-component pairs: {}", count.groups);
    println!("soft masks in the list: {}", list.soft_mask_count());
    for (name, pairs) in &count.pairs {
        println!("pairs {name}: {pairs}");
    }
    for (field, n) in &count.differing {
        println!("differing {field}: {n}");
    }
    println!("shading kinds shared by one Arc: {}", count.shared_kinds);
    println!(
        "mask pairs equal by value: {}, differing: {}",
        count.masks_equal, count.masks_differing
    );
    let distinct = count.paints.len();
    let ambiguous = count
        .paints
        .values()
        .filter(|black| black.len() > 1)
        .count();
    println!(
        "distinct chromatic paints: {distinct}, of them paired with more than one black paint: \
         {ambiguous}"
    );
    for (chromatic, black) in count.paints.iter().filter(|(_, b)| b.len() > 1).take(5) {
        let chromatic: String = chromatic.chars().take(60).collect();
        println!("  {chromatic} -> {} black paints", black.len());
    }
}
