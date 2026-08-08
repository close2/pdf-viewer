//! Counts what a page's clip chains and shading fills cost a rasteriser to build.
//!
//! `cargo run --release -p pdf-model --example clip_chain_census -- <file.pdf> [page]`
//!
//! Written for `doc/todo/40`, whose proposal — build a chain from its parent's cached mask
//! instead of from its root — is only worth taking if the *intermediate* clips are shared
//! between chains. A profile cannot say that: `MaskCache::get` is one line whether the 3554
//! chains of `bug1721218_reduced.pdf` pass through 3554 intermediate nodes or through four.
//! So this walks the display list and reports the three numbers the decision needs:
//!
//! - `chain steps`, the fill and intersect operations the present code performs — one per
//!   (leaf, ancestor) pair;
//! - `distinct nodes`, what the same page would cost if every node were built once from its
//!   parent, which is the proposal's floor;
//! - `mask bytes`, both ways, against `render-cpu`'s 32 MB budget.
//!
//! It also reports what the shading fills cover, because a gradient is evaluated over the
//! spans of the *path* and masked afterwards: the pixels a clip rejects are paid for at full
//! price, and the ratio between a fill's own bounds and its clip's says how many there are.

#![expect(
    clippy::print_stdout,
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    reason = "a diagnostic binary whose output is its purpose, bounded by one page's commands"
)]

use std::collections::{BTreeMap, HashMap, HashSet};

use pdf_render::{ClipId, Command, DisplayList, Paint, Rect, TargetSpec, Transform};

/// One clip node's contribution, measured in device space.
struct Node {
    /// Rows of the target the chain ending here can mark.
    rows: u32,
    /// How many steps from the root, counting the root as one.
    depth: usize,
}

/// The device bounds of the chain ending at `id`, and its depth, memoised.
fn chain(
    list: &DisplayList,
    id: ClipId,
    to_device: Transform,
    known: &mut HashMap<ClipId, Option<(Rect, usize)>>,
) -> Option<(Rect, usize)> {
    if let Some(hit) = known.get(&id) {
        return *hit;
    }
    let clip = list.clip(id).expect("the list holds every clip it names");
    let own = clip.path.bounds(clip.transform.then(to_device));
    let answer = match clip.parent {
        None => own.map(|rect| (rect, 1)),
        Some(parent) => match chain(list, parent, to_device, known) {
            None => None,
            Some((outer, depth)) => match own {
                // A clip nobody could measure widens nothing, which is what `MaskCache::build`
                // does with it too.
                None => Some((outer, depth + 1)),
                Some(own) => intersect(outer, own).map(|both| (both, depth + 1)),
            },
        },
    };
    known.insert(id, answer);
    answer
}

/// The overlap of two rectangles, or `None` where they do not meet.
fn intersect(a: Rect, b: Rect) -> Option<Rect> {
    let left = a.min.x.max(b.min.x);
    let right = a.max.x.min(b.max.x);
    let top = a.min.y.max(b.min.y);
    let bottom = a.max.y.min(b.max.y);
    (left < right && top < bottom).then(|| {
        Rect::from_corners(
            pdf_render::Point::new(left, top),
            pdf_render::Point::new(right, bottom),
        )
    })
}

/// The rows of a `height`-row target that `bounds` reaches, the way `Band::covering` counts
/// them: outset by a row, rounded outward, clamped.
fn rows(bounds: Rect, height: u32) -> u32 {
    let top = (bounds.min.y - 1.0).floor().max(0.0);
    let bottom = (bounds.max.y + 1.0).ceil().min(height as f32);
    if bottom <= top {
        return 0;
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to 0..=height, which is a u32"
    )]
    {
        (bottom - top) as u32
    }
}

/// Walks every command, nested groups included, calling `visit` on each.
fn walk(commands: &[Command], visit: &mut impl FnMut(&Command)) {
    for command in commands {
        visit(command);
        match command {
            Command::Group { commands, .. } => walk(commands, visit),
            Command::Shaped { object, shape } => {
                walk(std::slice::from_ref(&**object), visit);
                walk(std::slice::from_ref(&**shape), visit);
            }
            _ => {}
        }
    }
}

fn main() {
    let path = std::env::args().nth(1).expect("a path to a PDF");
    let index: usize = std::env::args()
        .nth(2)
        .map_or(1, |n| n.parse().expect("a page number"));
    let bytes = std::fs::read(&path).expect("readable");
    let document = pdf_syntax::Document::open(bytes).expect("valid PDF");
    let page = pdf_model::Pages::new(&document)
        .get(index - 1)
        .expect("page exists");
    let list = pdf_model::interpret(&document, &page).display_list;
    let target = TargetSpec::for_page(&list, 1.0, 1 << 30).expect("a valid target");

    let mut variants: BTreeMap<&str, usize> = BTreeMap::new();
    let mut leaves: Vec<ClipId> = Vec::new();
    let mut shadings = 0_usize;
    let mut path_pixels = 0_u64;
    let mut clipped_pixels = 0_u64;
    walk(list.commands(), &mut |command| {
        let name = match command {
            Command::Fill { .. } => "fill",
            Command::Stroke { .. } => "stroke",
            Command::Image { .. } => "image",
            Command::Group { .. } => "group",
            Command::Shaped { .. } => "shaped",
            _ => "other",
        };
        *variants.entry(name).or_default() += 1;
        if let Some(id) = command.clip() {
            leaves.push(id);
        }
        if let Command::Fill {
            path,
            transform,
            paint: Paint::Shading(_),
            clip,
            ..
        } = command
        {
            shadings += 1;
            let Some(own) = path.bounds(transform.then(target.transform)) else {
                return;
            };
            let area = |rect: Rect| {
                u64::from(rows(rect, target.height))
                    * u64::from(rows(
                        Rect::from_corners(
                            pdf_render::Point::new(rect.min.y, rect.min.x),
                            pdf_render::Point::new(rect.max.y, rect.max.x),
                        ),
                        target.width,
                    ))
            };
            path_pixels += area(own);
            let mut known = HashMap::new();
            let narrowed = clip
                .and_then(|id| chain(&list, id, target.transform, &mut known))
                .and_then(|(outer, _)| intersect(outer, own))
                .unwrap_or(own);
            clipped_pixels += area(narrowed);
        }
    });

    let distinct_leaves: HashSet<ClipId> = leaves.iter().copied().collect();
    let mut known = HashMap::new();
    let mut nodes: HashMap<ClipId, Node> = HashMap::new();
    let mut steps = 0_usize;
    let mut depths: BTreeMap<usize, usize> = BTreeMap::new();
    for &leaf in &distinct_leaves {
        let mut id = Some(leaf);
        let mut depth = 0;
        while let Some(this) = id {
            depth += 1;
            if let Some((bounds, from_root)) = chain(&list, this, target.transform, &mut known) {
                nodes.entry(this).or_insert(Node {
                    rows: rows(bounds, target.height),
                    depth: from_root,
                });
            }
            id = list
                .clip(this)
                .expect("the list holds every clip it names")
                .parent;
        }
        steps += depth;
        *depths.entry(depth).or_default() += 1;
    }

    let width = u64::from(target.width);
    let leaf_bytes: u64 = distinct_leaves
        .iter()
        .filter_map(|id| nodes.get(id))
        .map(|node| u64::from(node.rows) * width)
        .sum();
    let all_bytes: u64 = nodes.values().map(|n| u64::from(n.rows) * width).sum();
    let mut by_depth: BTreeMap<usize, (usize, u64)> = BTreeMap::new();
    for node in nodes.values() {
        let entry = by_depth.entry(node.depth).or_default();
        entry.0 += 1;
        entry.1 += u64::from(node.rows) * width;
    }

    println!(
        "{path} page {index}, target {}x{}",
        target.width, target.height
    );
    println!("  commands {variants:?}");
    println!("  soft masks {}", list.soft_mask_count());
    println!(
        "  clip references {}, distinct leaves {}",
        leaves.len(),
        distinct_leaves.len()
    );
    println!("  chain steps (fills + intersects today) {steps}");
    println!("  distinct clip nodes {}", nodes.len());
    println!("  chain depth histogram {depths:?}");
    println!("  nodes and bytes by depth {by_depth:?}");
    println!(
        "  mask bytes: leaves only {leaf_bytes} ({:.1} MB), every node {all_bytes} ({:.1} MB)",
        leaf_bytes as f64 / 1e6,
        all_bytes as f64 / 1e6
    );
    println!(
        "  shading fills {shadings}: path pixels {path_pixels}, within their clips \
         {clipped_pixels} ({:.1}%)",
        100.0 * clipped_pixels as f64 / path_pixels.max(1) as f64
    );
}
