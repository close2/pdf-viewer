//! The page tree, and the attributes pages inherit through it.
//!
//! # Inheritance is not optional
//!
//! `/Resources`, `/MediaBox`, `/CropBox` and `/Rotate` may be set on any node of the page
//! tree, and a page without them takes its nearest ancestor's value (ISO 32000-2 table 31,
//! and confirmed against the generated model in `pdf-spec`). A reader that ignores this
//! renders pages at the wrong size or with no resources at all, which looks like a
//! rendering bug and is really a model bug.
//!
//! # Lazy, and bounded
//!
//! Pages are found by walking the tree on demand rather than by building a list at open
//! time, because time-to-first-page is what a user perceives. The walk is bounded in both
//! depth and total nodes visited: `/Kids` may contain a cycle, and a tree claiming a
//! million nodes should cost a bounded amount of work rather than all available memory.

use pdf_syntax::{Dictionary, Document, Object};

/// Deepest page-tree nesting that will be followed.
///
/// Real trees are shallow — a balanced tree over a hundred thousand pages is about six
/// levels — so anything approaching this is malformed or hostile.
const MAX_TREE_DEPTH: usize = 64;

/// Most nodes visited while looking for one page.
///
/// Bounds the work a single lookup can cost regardless of what the tree claims.
const MAX_NODES_VISITED: usize = 1 << 20;

/// Attributes a page inherits from its ancestors.
#[derive(Debug, Clone, Default)]
struct Inherited {
    resources: Option<Dictionary>,
    media_box: Option<[f32; 4]>,
    crop_box: Option<[f32; 4]>,
    rotate: Option<i64>,
}

impl Inherited {
    /// Overlays the inheritable attributes present on `node`.
    ///
    /// Values found deeper in the tree win, which is what "inherited" means: the nearest
    /// ancestor's value applies, not the root's.
    fn overlay(&self, document: &Document, node: &Dictionary) -> Self {
        Self {
            resources: document
                .get_key(node, "Resources")
                .as_dict()
                .cloned()
                .or_else(|| self.resources.clone()),
            media_box: rectangle(document, node, "MediaBox").or(self.media_box),
            crop_box: rectangle(document, node, "CropBox").or(self.crop_box),
            rotate: document
                .get_key(node, "Rotate")
                .as_integer()
                .or(self.rotate),
        }
    }
}

/// Reads a four-number rectangle, normalising the corner order.
///
/// PDF rectangle arrays are two opposite corners in any order, so `[0 842 595 0]` is the
/// same rectangle as `[0 0 595 842]`. Producers emit both.
fn rectangle(document: &Document, dict: &Dictionary, key: &str) -> Option<[f32; 4]> {
    let array = document.get_key(dict, key);
    let items = array.as_array()?;
    if items.len() < 4 {
        return None;
    }

    let mut values = [0f32; 4];
    for (slot, item) in values.iter_mut().zip(items) {
        let number = document.resolve(item).as_number()?;
        if !number.is_finite() {
            return None;
        }
        #[expect(
            clippy::cast_possible_truncation,
            reason = "page coordinates are bounded by the format at 14 400 units, far \
                      inside f32's exact integer range"
        )]
        {
            *slot = number as f32;
        }
    }

    Some([
        values[0].min(values[2]),
        values[1].min(values[3]),
        values[0].max(values[2]),
        values[1].max(values[3]),
    ])
}

/// One page of a document.
#[derive(Debug, Clone)]
pub struct Page {
    /// The page dictionary.
    pub dict: Dictionary,
    /// The resource dictionary in effect, after inheritance.
    pub resources: Dictionary,
    /// The media box, after inheritance, as `[x0, y0, x1, y1]`.
    pub media_box: [f32; 4],
    /// The crop box, after inheritance, defaulting to the media box.
    pub crop_box: [f32; 4],
    /// Clockwise rotation in degrees, normalised to 0, 90, 180 or 270.
    pub rotate: u16,
}

impl Page {
    /// A4, used when a page declares no media box anywhere in its ancestry.
    ///
    /// The specification makes `/MediaBox` required, but files omit it, and every viewer
    /// substitutes a default rather than refusing the page. A4 is chosen over US Letter
    /// because this project's corpus and locale are metric; the choice is visible here
    /// rather than buried at a call site.
    pub const DEFAULT_MEDIA_BOX: [f32; 4] = [0.0, 0.0, 595.276, 841.89];

    /// Returns the visible width in PDF units, after cropping.
    #[must_use]
    pub fn width(&self) -> f32 {
        self.crop_box[2] - self.crop_box[0]
    }

    /// Returns the visible height in PDF units, after cropping.
    #[must_use]
    pub fn height(&self) -> f32 {
        self.crop_box[3] - self.crop_box[1]
    }

    /// Returns the content streams for this page, concatenated and decoded.
    ///
    /// `/Contents` may be one stream or an array of them, and the specification says an
    /// array is to be treated as a single stream with the parts joined — a token may even
    /// be split across the boundary, so joining before interpretation is required rather
    /// than merely convenient.
    ///
    /// A newline is inserted between parts, since a producer that relied on the split as a
    /// token boundary would otherwise have two operators run together.
    #[must_use]
    pub fn content(&self, document: &Document) -> Vec<u8> {
        let contents = document.get_key(&self.dict, "Contents");
        let parts: Vec<Object> = match contents {
            Object::Array(items) => items.iter().map(|item| document.resolve(item)).collect(),
            other => vec![other],
        };

        let mut out = Vec::new();
        for part in &parts {
            let Some(stream) = part.as_stream() else {
                continue;
            };
            let Some(data) = document.decoded_stream_data(stream) else {
                continue;
            };
            out.extend_from_slice(&data);
            out.push(b'\n');
        }
        out
    }
}

/// The pages of a document, resolved on demand.
#[derive(Debug)]
pub struct Pages<'a> {
    document: &'a Document,
    root: Option<Dictionary>,
    count: usize,
}

impl<'a> Pages<'a> {
    /// Builds a page index for a document.
    ///
    /// Reads only the catalogue and the root of the page tree; individual pages are found
    /// when asked for.
    #[must_use]
    pub fn new(document: &'a Document) -> Self {
        let root = document
            .catalog()
            .ok()
            .map(|catalog| document.get_key(&catalog, "Pages"))
            .and_then(|pages| pages.as_dict().cloned());

        // `/Count` is authoritative when present and plausible, and the tree is counted
        // only when it is not — which keeps opening a large document cheap.
        let declared = root
            .as_ref()
            .and_then(|node| document.get_key(node, "Count").as_integer())
            .and_then(|value| usize::try_from(value).ok());

        let count = match declared {
            Some(count) if count > 0 && count <= MAX_NODES_VISITED => count,
            _ => root.as_ref().map_or(0, |node| count_leaves(document, node)),
        };

        Self {
            document,
            root,
            count,
        }
    }

    /// Returns the number of pages.
    #[must_use]
    pub fn len(&self) -> usize {
        self.count
    }

    /// Returns `true` if the document has no pages.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Returns the page at a zero-based index.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<Page> {
        let root = self.root.clone()?;
        let mut remaining = index;
        let mut visited = 0usize;
        find_leaf(
            self.document,
            &root,
            &Inherited::default(),
            &mut remaining,
            &mut visited,
            0,
        )
    }
}

/// Counts leaf nodes, for a tree whose `/Count` is missing or implausible.
fn count_leaves(document: &Document, node: &Dictionary) -> usize {
    fn walk(document: &Document, node: &Dictionary, depth: usize, visited: &mut usize) -> usize {
        if depth > MAX_TREE_DEPTH || *visited > MAX_NODES_VISITED {
            return 0;
        }
        *visited = visited.saturating_add(1);

        let kids = document.get_key(node, "Kids");
        let Some(kids) = kids.as_array() else {
            // No `/Kids` means this is a leaf, whatever its `/Type` claims. Trusting
            // `/Type` instead would drop pages from files that omit it.
            return 1;
        };

        kids.iter()
            .map(|kid| document.resolve(kid))
            .filter_map(|kid| kid.as_dict().cloned())
            .map(|kid| walk(document, &kid, depth.saturating_add(1), visited))
            .sum()
    }

    let mut visited = 0usize;
    walk(document, node, 0, &mut visited)
}

/// Descends to the leaf at `remaining` pages from here, accumulating inherited attributes.
fn find_leaf(
    document: &Document,
    node: &Dictionary,
    inherited: &Inherited,
    remaining: &mut usize,
    visited: &mut usize,
    depth: usize,
) -> Option<Page> {
    if depth > MAX_TREE_DEPTH || *visited > MAX_NODES_VISITED {
        return None;
    }
    *visited = visited.saturating_add(1);

    let inherited = inherited.overlay(document, node);
    let kids = document.get_key(node, "Kids");

    let Some(kids) = kids.as_array() else {
        // A leaf. Take it if this is the one asked for.
        if *remaining == 0 {
            return Some(build_page(node, &inherited));
        }
        *remaining = remaining.saturating_sub(1);
        return None;
    };

    for kid in kids {
        let kid = document.resolve(kid);
        let Some(kid) = kid.as_dict() else { continue };

        // Skip whole subtrees using `/Count` where it is trustworthy: for a hundred-thousand
        // page document this is the difference between a lookup costing six node reads and
        // costing fifty thousand.
        let subtree = document.get_key(kid, "Count").as_integer();
        let has_kids = document.get_key(kid, "Kids").as_array().is_some();
        if has_kids
            && let Some(count) = subtree.and_then(|value| usize::try_from(value).ok())
            && count > 0
            && count <= *remaining
        {
            *remaining = remaining.saturating_sub(count);
            continue;
        }

        if let Some(page) = find_leaf(
            document,
            kid,
            &inherited,
            remaining,
            visited,
            depth.saturating_add(1),
        ) {
            return Some(page);
        }
    }

    None
}

/// Assembles a page from its dictionary and the attributes it inherited.
fn build_page(dict: &Dictionary, inherited: &Inherited) -> Page {
    let media_box = inherited.media_box.unwrap_or(Page::DEFAULT_MEDIA_BOX);

    // The crop box is intersected with the media box, as the specification requires: a
    // crop box larger than the medium does not enlarge the page.
    let crop_box = inherited.crop_box.map_or(media_box, |crop| {
        [
            crop[0].max(media_box[0]),
            crop[1].max(media_box[1]),
            crop[2].min(media_box[2]),
            crop[3].min(media_box[3]),
        ]
    });
    // An empty intersection means the crop box is nonsense; fall back rather than render
    // nothing.
    let crop_box = if crop_box[2] > crop_box[0] && crop_box[3] > crop_box[1] {
        crop_box
    } else {
        media_box
    };

    // Rotation must be a multiple of 90. Negative and out-of-range values occur, so the
    // value is normalised rather than trusted.
    let rotate = inherited.rotate.unwrap_or(0).rem_euclid(360);
    let rotate = u16::try_from(rotate).unwrap_or(0);
    let rotate = (rotate / 90).saturating_mul(90) % 360;

    Page {
        dict: dict.clone(),
        resources: inherited.resources.clone().unwrap_or_default(),
        media_box,
        crop_box,
        rotate,
    }
}
