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
    /// The first of those rows, which is what decides whether a parent's mask may be reused
    /// verbatim: see [`band`].
    top: u32,
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
    band(bounds, height).1
}

/// The first row and the row count that `bounds` reaches, the way `Band::covering` counts them.
///
/// The pair rather than the count, because `doc/todo/40`'s exactness question is about the
/// *whole* band: `Surface::to_device` composes a band's first row into the translation, so two
/// chains rasterise identically only where their bands agree in both numbers.
fn band(bounds: Rect, height: u32) -> (u32, u32) {
    let top = (bounds.min.y - 1.0).floor().max(0.0);
    let bottom = (bounds.max.y + 1.0).ceil().min(height as f32);
    if bottom <= top {
        return (0, 0);
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to 0..=height, which is a u32"
    )]
    {
        (top as u32, (bottom - top) as u32)
    }
}

/// Whether one clip node's path is a device rectangle covering every pixel of the target.
///
/// Such a node admits everything: `mask_rectangle` writes a whole pixel's coverage at every
/// pixel of the mask, so filling it produces 255 everywhere and intersecting it takes
/// `min(kept, 255)`, which is `kept`. It is therefore droppable from a chain without any
/// departure at all — the arithmetic ADR 0219 prices never enters, because no scan conversion
/// is reused, one is *declined*.
fn covers_the_target(
    list: &DisplayList,
    id: ClipId,
    to_device: Transform,
    target: TargetSpec,
) -> bool {
    let Some(clip) = list.clip(id) else {
        return false;
    };
    let Some(pdf_render::DeviceRectangles::One(rect)) =
        pdf_render::device_rectangles(&clip.path, clip.transform.then(to_device))
    else {
        return false;
    };
    rect.min.x <= 0.0
        && rect.min.y <= 0.0
        && rect.max.x >= target.width as f32
        && rect.max.y >= target.height as f32
}

/// What building a page's clip masks costs, in operations and in the rows each touches.
///
/// Rows rather than operations alone because a fill and an intersect are both a scan
/// conversion over the band, so a count on its own ranks a 792-row band with a 4-row one.
#[derive(Default)]
struct Cost {
    /// Masks opened by filling a root path.
    fills: usize,
    /// Paths intersected into a mask already open.
    intersects: usize,
    /// Masks copied verbatim from an ancestor's.
    clones: usize,
    /// Band rows summed over `fills`.
    fill_rows: u64,
    /// Band rows summed over `intersects`.
    intersect_rows: u64,
    /// Band rows summed over `clones`.
    clone_rows: u64,
}

impl Cost {
    /// The scan conversions, which are what a rasteriser spends: a clone is a copy and is
    /// counted apart.
    fn scans(&self) -> u64 {
        self.fill_rows + self.intersect_rows
    }
}

/// Simulates `doc/todo/40`'s proposal, restricted to the steps that cost no departure.
///
/// A chain may start from an ancestor's cached mask *byte for byte* only where the two share a
/// band, because `Surface::to_device` composes a band's first row into the translation and ADR
/// 0219 measured what shifting it does to `y·sy + ty`. Where the bands differ the node is built
/// from its root exactly as today, so this is a floor on what an exact implementation saves
/// rather than an estimate of the full proposal.
fn build_exact(
    list: &DisplayList,
    nodes: &HashMap<ClipId, Node>,
    id: ClipId,
    built: &mut HashSet<ClipId>,
    cost: &mut Cost,
) {
    if !built.insert(id) {
        return;
    }
    let Some(node) = nodes.get(&id) else {
        return;
    };
    let shared = list
        .clip(id)
        .expect("the list holds every clip it names")
        .parent
        .filter(|parent| {
            nodes
                .get(parent)
                .is_some_and(|above| above.top == node.top && above.rows == node.rows)
        });
    if let Some(parent) = shared {
        build_exact(list, nodes, parent, built, cost);
        cost.clones += 1;
        cost.clone_rows += u64::from(node.rows);
        cost.intersects += 1;
        cost.intersect_rows += u64::from(node.rows);
    } else {
        cost.fills += 1;
        cost.fill_rows += u64::from(node.rows);
        cost.intersects += node.depth - 1;
        cost.intersect_rows += u64::from(node.rows) * (node.depth - 1) as u64;
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
                let (top, height) = band(bounds, target.height);
                nodes.entry(this).or_insert(Node {
                    rows: height,
                    top,
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

    let mut today = Cost::default();
    for leaf in distinct_leaves.iter().filter_map(|id| nodes.get(id)) {
        today.fills += 1;
        today.fill_rows += u64::from(leaf.rows);
        today.intersects += leaf.depth - 1;
        today.intersect_rows += u64::from(leaf.rows) * (leaf.depth - 1) as u64;
    }
    let mut exact = Cost::default();
    let mut built = HashSet::new();
    for &leaf in &distinct_leaves {
        build_exact(&list, &nodes, leaf, &mut built, &mut exact);
    }
    // The proposal without the exactness restriction: every node built once from its parent's
    // mask, cropped to its own band. Where the two bands differ that crop is not the prefix's
    // contribution in the child's band — it is the parent's rasterisation reused — which is the
    // departure ADR 0219 prices and the `exact` arm above declines.
    let mut full = Cost::default();
    for node in nodes.values() {
        if node.depth == 1 {
            full.fills += 1;
            full.fill_rows += u64::from(node.rows);
        } else {
            full.clones += 1;
            full.clone_rows += u64::from(node.rows);
            full.intersects += 1;
            full.intersect_rows += u64::from(node.rows);
        }
    }
    // The third question, and the one that needs no departure: a chain node whose path is a
    // device rectangle covering the whole target admits everything, so the step that scan-converts
    // it is work with no result. A chain made only of those admits everything too.
    let mut covering: HashMap<ClipId, bool> = HashMap::new();
    for &id in nodes.keys() {
        let answer = covers_the_target(&list, id, target.transform, target);
        covering.insert(id, answer);
    }
    let covered_steps: usize = distinct_leaves
        .iter()
        .flat_map(|&leaf| {
            let mut chain = Vec::new();
            let mut id = Some(leaf);
            while let Some(this) = id {
                chain.push(this);
                id = list
                    .clip(this)
                    .expect("the list holds every clip it names")
                    .parent;
            }
            chain
        })
        .filter(|id| covering.get(id).copied().unwrap_or_default())
        .count();
    let unclipped_references = leaves
        .iter()
        .filter(|&&leaf| {
            let mut id = Some(leaf);
            while let Some(this) = id {
                if !covering.get(&this).copied().unwrap_or_default() {
                    return false;
                }
                id = list
                    .clip(this)
                    .expect("the list holds every clip it names")
                    .parent;
            }
            true
        })
        .count();
    let shareable = nodes
        .iter()
        .filter(|(id, node)| {
            list.clip(**id)
                .and_then(|clip| clip.parent)
                .and_then(|parent| nodes.get(&parent))
                .is_some_and(|above| above.top == node.top && above.rows == node.rows)
        })
        .count();

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
        "  nodes sharing their parent's band {shareable} of {} non-root",
        nodes.len().saturating_sub(1)
    );
    println!(
        "  today  {} fills + {} intersects, {} scanned rows",
        today.fills,
        today.intersects,
        today.scans()
    );
    println!(
        "  exact  {} fills + {} intersects + {} clones, {} scanned rows ({:+.1}%), \
         {} cloned rows",
        exact.fills,
        exact.intersects,
        exact.clones,
        exact.scans(),
        100.0 * (exact.scans() as f64 - today.scans() as f64) / today.scans().max(1) as f64,
        exact.clone_rows
    );
    println!(
        "  full   {} fills + {} intersects + {} clones, {} scanned rows ({:+.1}%), \
         {} cloned rows",
        full.fills,
        full.intersects,
        full.clones,
        full.scans(),
        100.0 * (full.scans() as f64 - today.scans() as f64) / today.scans().max(1) as f64,
        full.clone_rows
    );
    println!(
        "  covering rectangles: {covered_steps} of {steps} chain steps admit everything, \
         {unclipped_references} of {} clip references admit the whole target",
        leaves.len()
    );
    println!(
        "  shading fills {shadings}: path pixels {path_pixels}, within their clips \
         {clipped_pixels} ({:.1}%)",
        100.0 * clipped_pixels as f64 / path_pixels.max(1) as f64
    );
}
