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

use std::collections::BTreeMap;

use pdf_syntax::{Dictionary, Document, FilterRefusal, Object, ObjectId, StreamRefusal};

/// Deepest page-tree nesting that will be followed.
///
/// Real trees are shallow — a balanced tree over a hundred thousand pages is about six
/// levels — so anything approaching this is malformed or hostile.
const MAX_TREE_DEPTH: usize = 64;

/// Most nodes visited while looking for one page.
///
/// Bounds the work a single lookup can cost regardless of what the tree claims.
const MAX_NODES_VISITED: usize = 1 << 20;

/// A node of the page tree, as the walk holds it: by name where the file named it.
///
/// §7.7.3.2 makes `/Kids` "an array of indirect references to the immediate children of this
/// node", so nearly every node a walk meets arrives as an object number — and copying the
/// object out of the document to ask it two questions is the whole cost of the walk. A
/// thousand-page document has a `/Kids` a thousand entries wide, and finding page *n* in it
/// used to hand the walk *n* page dictionaries, each with its `/Annots`, its `/Resources` and
/// its `/Contents`, so that Table 30's `/Count` could be read off it and the copy dropped: 17.1%
/// of a document-wide search (ADR 0330).
///
/// So a node is a *name* until something needs the node itself, and the only thing that does is
/// [`build_page`] — once per lookup, on the page that was asked for.
#[derive(Debug, Clone, Copy)]
enum Node<'a> {
    /// A child of a `/Kids` array, read one entry at a time through its object number.
    Indirect(ObjectId),
    /// A node this walk was handed whole: the root, which the catalogue resolved, a page found
    /// by [`Pages::new`]'s scan, or a `/Kids` entry a producer wrote out directly instead of by
    /// reference. The identity is carried beside it because a dictionary does not know its own
    /// object number.
    Direct(&'a Dictionary, Option<ObjectId>),
}

impl<'a> Node<'a> {
    /// The node a `/Kids` entry names, or `None` where the entry names no node at all.
    ///
    /// A reference is what §7.7.3.2 asks for and is the whole point of this type. The other two
    /// are what producers write anyway and what resolving the entry used to accept without
    /// anybody deciding to: a dictionary written straight into the array, and a stream, which
    /// has a dictionary and is not a page but was read as one before this and still is.
    fn of(entry: &'a Object) -> Option<Self> {
        match entry {
            Object::Reference(id) => Some(Self::Indirect(*id)),
            Object::Dictionary(dict) => Some(Self::Direct(dict, None)),
            Object::Stream(stream) => Some(Self::Direct(&stream.dict, None)),
            _ => None,
        }
    }

    /// One entry of this node, resolved, or `None` where this is not a dictionary at all.
    ///
    /// The distinction is [`Document::get_key_of`]'s and the reason it exists: a `/Kids` entry
    /// naming something that is not a dictionary is neither a node nor a page, and a walk that
    /// read it as a page with no entries would count it and shift every later page's index.
    fn key(self, document: &'a Document, key: &str) -> Option<Object> {
        match self {
            Self::Indirect(id) => document.get_key_of(id, key),
            Self::Direct(dict, _) => Some(document.get_key(dict, key)),
        }
    }

    /// Which object this node is, where it was reached through one. See [`Page::id`].
    fn id(self) -> Option<ObjectId> {
        match self {
            Self::Indirect(id) => Some(id),
            Self::Direct(_, id) => id,
        }
    }

    /// The node's own dictionary, copied out of the document where it took one.
    ///
    /// The one copy the walk makes, and it makes it for the page it was asked for rather than
    /// for every page it stepped over.
    fn dictionary(self, document: &Document) -> Option<Dictionary> {
        match self {
            Self::Indirect(id) => document.get(id).as_dict().cloned(),
            Self::Direct(dict, _) => Some(dict.clone()),
        }
    }
}

/// Whether a node states that it is a page tree *node* rather than a page object.
///
/// §7.7.3.3 Table 31 makes `/Type` required of a page object, and says which name it takes:
///
/// > ( Required ) The type of PDF object that this dictionary describes; shall be Page for a
/// > page object or Template for an invisible Template page (see 12.7.7, "Named pages").
///
/// So a dictionary saying `/Type /Pages` has said in its own words that it is neither. §7.7.3.2
/// Table 30 then makes `/Kids` required of the node it says it is, and a node whose required
/// children are absent has *no* children rather than being one:
///
/// > ( Required ) An array of indirect references to the immediate children of this node. The
/// > children shall only be page objects or other page tree nodes.
///
/// The question is only ever asked where `/Kids` is missing, and the asymmetry is the point.
/// A dictionary with no `/Kids` and no `/Type` is a leaf, because a page object that omits the
/// entry is common and trusting `/Type` in that direction would drop those pages from their
/// documents. A dictionary with no `/Kids` and `/Type /Pages` is the file contradicting itself,
/// and this reads the entry the file *did* write rather than drawing a node as a page.
///
/// Where that empties the tree, [`Pages::new`]'s recovery scan asks every object what it says
/// it is — which is how the witness in `doc/corpora/format-corpus` finds its page — and where
/// the scan finds nothing the document has no first page and says so. ADR 0305.
fn declares_a_node(document: &Document, node: Node<'_>) -> bool {
    node.key(document, "Type")
        .as_ref()
        .and_then(Object::as_name)
        .is_some_and(|kind| kind.as_bytes() == b"Pages")
}

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
    fn overlay(&self, document: &Document, node: Node<'_>) -> Self {
        let entry = |key| node.key(document, key).unwrap_or(Object::Null);
        Self {
            resources: entry("Resources")
                .as_dict()
                .cloned()
                .or_else(|| self.resources.clone()),
            media_box: rectangle(document, &entry("MediaBox")).or(self.media_box),
            crop_box: rectangle(document, &entry("CropBox")).or(self.crop_box),
            rotate: entry("Rotate").as_integer().or(self.rotate),
        }
    }
}

/// Reads a four-number rectangle, normalising the corner order.
///
/// PDF rectangle arrays are two opposite corners in any order, so `[0 842 595 0]` is the
/// same rectangle as `[0 0 595 842]`. Producers emit both.
fn rectangle(document: &Document, array: &Object) -> Option<[f32; 4]> {
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

/// One of §14.11.2's five page boundaries.
///
/// The vocabulary is shared by two clauses that name a boundary rather than stating one:
/// §12.2's `/ViewArea`, `/ViewClip`, `/PrintArea` and `/PrintClip` each hold "the key
/// designating the relevant page boundary in the page object", and §14.11.2.2's box colour
/// information dictionary has an entry per boundary. So this is the key, and [`Page::boundary`]
/// is the rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Boundary {
    /// `/MediaBox`: "the boundaries of the physical medium on which the page is to be
    /// printed".
    Media,
    /// `/CropBox`: "the region to which the contents of the page shall be clipped (cropped)
    /// when displayed or printed". The default everywhere a boundary is named.
    #[default]
    Crop,
    /// `/BleedBox`: "the region to which the contents of the page shall be clipped when
    /// output in a production environment".
    Bleed,
    /// `/TrimBox`: "the intended dimensions of the finished page after trimming".
    Trim,
    /// `/ArtBox`: "the extent of the page's meaningful content (including potential
    /// white-space) as intended by the page's creator".
    Art,
}

impl Boundary {
    /// One of Table 31's five box keys, or `None` for a name that is not one.
    #[must_use]
    pub fn read(name: &pdf_syntax::Name) -> Option<Self> {
        match name.as_bytes() {
            b"MediaBox" => Some(Self::Media),
            b"CropBox" => Some(Self::Crop),
            b"BleedBox" => Some(Self::Bleed),
            b"TrimBox" => Some(Self::Trim),
            b"ArtBox" => Some(Self::Art),
            _ => None,
        }
    }
}

/// One page of a document.
#[derive(Debug, Clone)]
pub struct Page {
    /// Which object the page *is*, where it was reached through one.
    ///
    /// A page is always an indirect object in a well-formed file — §7.7.3.2's `/Kids` is "an
    /// array of indirect references" — so this is `Some` for every page reached through the tree
    /// and for every page found by [`Pages`]' scan. It is `None` for [`Pages::detached`], which
    /// builds a page from a dictionary a caller already holds (§12.7.7's templates).
    ///
    /// **It exists because an *edit* has to be filed against a page.** A markup annotation a
    /// person adds belongs to the page they added it on, and `ViewState` — which is a log beside
    /// an immutable document — has no other key to file it under. `Pages::indices` inverts the
    /// tree for the same question in two other places, and doing that a third time inside the
    /// interpreter would put a page-tree walk on the render path.
    pub id: Option<ObjectId>,
    /// The page dictionary.
    pub dict: Dictionary,
    /// The resource dictionary in effect, after inheritance.
    pub resources: Dictionary,
    /// The media box, after inheritance, as `[x0, y0, x1, y1]`.
    pub media_box: [f32; 4],
    /// The crop box, after inheritance, defaulting to the media box.
    pub crop_box: [f32; 4],
    /// §14.11.2.1's bleed box, defaulting to the crop box. Not inheritable (§7.7.3.4).
    pub bleed_box: [f32; 4],
    /// §14.11.2.1's trim box, defaulting to the crop box. Not inheritable (§7.7.3.4).
    pub trim_box: [f32; 4],
    /// §14.11.2.1's art box, defaulting to the crop box. Not inheritable (§7.7.3.4).
    pub art_box: [f32; 4],
    /// The region actually displayed: §12.2's `/ViewArea` boundary, the crop box by default.
    ///
    /// This is what the rasteriser draws and what [`Self::width`] and [`Self::height`]
    /// measure, and for every document that states no `/ViewArea` it *is* `crop_box` —
    /// Table 147's default for the entry is `CropBox`, and 0 of the 974 corpus documents
    /// state anything else. Kept as its own field rather than overwriting `crop_box`, because
    /// the crop box is what the file says and this is what a preference did with it.
    pub display_box: [f32; 4],
    /// The region the contents are clipped to: §12.2's `/ViewClip` boundary.
    ///
    /// Distinct from [`Self::display_box`] because the clause makes them distinct entries:
    /// one is "the area of a page that shall be displayed", the other "the page boundary to
    /// which the contents of a page shall be clipped when viewing the document on the
    /// screen". A document may display the media box while clipping to the trim box, and the
    /// margin between them is then blank rather than absent.
    pub clip_box: [f32; 4],
    /// Clockwise rotation in degrees, normalised to 0, 90, 180 or 270.
    pub rotate: u16,
    /// §7.7.3.3 Table 31's `/UserUnit`: how big a default user space unit is.
    ///
    /// > A positive number that shall give the size of default user space units, in
    /// > multiples of 1 ⁄ 72 inch. The range of supported values shall be
    /// > implementation-dependent. Default value: 1.0 (user space unit is 1 ⁄ 72 inch).
    ///
    /// So a page is `width() × user_unit` seventy-seconds of an inch across, and a device
    /// asked to draw at a resolution has to scale by it. Table 31 does **not** mark the
    /// entry inheritable, and §7.7.3.4 says "attributes that are not explicitly identified
    /// in the table as inheritable shall not be inherited", so it is read from the page
    /// object alone — unlike `/MediaBox`, `/CropBox`, `/Resources` and `/Rotate`, which are
    /// the four entries beside it that are.
    pub user_unit: f32,
}

impl Page {
    /// A4, used when a page declares no media box anywhere in its ancestry.
    ///
    /// The specification makes `/MediaBox` required, but files omit it, and every viewer
    /// substitutes a default rather than refusing the page. A4 is chosen over US Letter
    /// because this project's corpus and locale are metric; the choice is visible here
    /// rather than buried at a call site.
    pub const DEFAULT_MEDIA_BOX: [f32; 4] = [0.0, 0.0, 595.276, 841.89];

    /// Returns the visible width in PDF units: the width of [`Self::display_box`].
    #[must_use]
    pub fn width(&self) -> f32 {
        self.display_box[2] - self.display_box[0]
    }

    /// Returns the visible height in PDF units: the height of [`Self::display_box`].
    #[must_use]
    pub fn height(&self) -> f32 {
        self.display_box[3] - self.display_box[1]
    }

    /// The rectangle one of §14.11.2's five boundaries names on this page.
    ///
    /// Every one of them has been defaulted and intersected with the media box already, which
    /// is §14.11.2.1's own rule for all four of the others: "[i]f the bounds of the crop,
    /// trim, bleed or art box extends outside of the bounds of the media box, a processor
    /// shall treat the box as its intersection with the media box."
    #[must_use]
    pub fn boundary(&self, boundary: Boundary) -> [f32; 4] {
        match boundary {
            Boundary::Media => self.media_box,
            Boundary::Crop => self.crop_box,
            Boundary::Bleed => self.bleed_box,
            Boundary::Trim => self.trim_box,
            Boundary::Art => self.art_box,
        }
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
    ///
    /// Equivalent to [`Self::content_with_report`], discarding the report. Prefer that one
    /// anywhere the result reaches a user: a part that does not decode contributes nothing,
    /// and a page silently short of a content stream is indistinguishable from a page that
    /// was meant to be empty.
    #[must_use]
    pub fn content(&self, document: &Document) -> Vec<u8> {
        self.content_with_report(document).0
    }

    /// The same, also naming the content streams that could not be decoded.
    ///
    /// A `/Contents` part fails to decode when its filter chain is one this reader does not
    /// implement — `LZWDecode` is the one that occurs — or when the stream is damaged. Both
    /// produce a page missing some or all of its drawing, and neither is visible in the
    /// result: the page renders, it is simply not the page the document describes.
    ///
    /// Returning the failures rather than swallowing them is what lets `interpret` report
    /// them, which principle 3 of `CLAUDE.md` requires of every layer.
    #[must_use]
    pub fn content_with_report(&self, document: &Document) -> (Vec<u8>, Vec<ContentIssue>) {
        // Each part is kept beside the object *as written*, because a `/Contents` the file
        // does not state and a `/Contents` naming an object this reader could not reach both
        // resolve to null and are not the same statement — see [`ContentIssue::Unreachable`].
        let stated = self.dict.get("Contents").cloned().unwrap_or(Object::Null);
        let parts: Vec<(Object, Object)> = match document.resolve(&stated) {
            Object::Array(items) => items
                .iter()
                .map(|item| (item.clone(), document.resolve(item)))
                .collect(),
            other => vec![(stated, other)],
        };

        let mut out = Vec::new();
        let mut issues = Vec::new();
        let limit = document.limits().max_stream_len;
        for (index, (named, part)) in parts.iter().enumerate() {
            // A `/Contents` that is missing entirely is an empty page, not a defect; one
            // whose entries are not streams is a malformed page and worth saying so.
            let Some(stream) = part.as_stream() else {
                match (named, part) {
                    (Object::Reference(object), Object::Null) => {
                        issues.push(ContentIssue::Unreachable {
                            index,
                            object: *object,
                        });
                    }
                    (_, Object::Null) => {}
                    _ => issues.push(ContentIssue::NotAStream { index }),
                }
                continue;
            };
            let data = match document.decoded_stream_data_reported(stream) {
                Ok(data) => data,
                // A bound refused this part; the filter chain did not fail to work. Saying
                // "undecodable" of a stream this reader can decode perfectly well would put a
                // limit of ours into a sentence about the file.
                Err(StreamRefusal::Filter {
                    why: FilterRefusal::TooLarge { limit },
                    ..
                }) => {
                    issues.push(ContentIssue::TooLarge {
                        part: Some(index),
                        limit,
                    });
                    continue;
                }
                Err(_) => {
                    issues.push(ContentIssue::Undecodable {
                        index,
                        filters: filter_names(document, &stream.dict),
                    });
                    continue;
                }
            };
            // **Table 31 says what the parts are, and therefore what bounds them.** Its
            // `/Contents` row:
            //
            // > If the value is an array, the effect shall be as if all of the streams in the
            // > array were concatenated with at least one white-space character added between
            // > the streams' data, in order, to form a single stream.
            //
            // So the array *is* one stream, and the bound one stream gets is the bound the
            // array gets — no second number, and none invented here. Without it a page could
            // state `max_array_len` = 2²⁰ parts of `max_stream_len` each and none of them would
            // be refused; the concatenation had no total at all until ADR 0306.
            if out.len().saturating_add(data.len()) > limit {
                issues.push(ContentIssue::TooLarge { part: None, limit });
                break;
            }
            out.extend_from_slice(&data);
            out.push(b'\n');
        }
        (out, issues)
    }
}

/// The pages of a document, resolved on demand.
#[derive(Debug)]
pub struct Pages<'a> {
    document: &'a Document,
    root: Option<Dictionary>,
    /// Which object the root node is, so that a one-node tree's leaf knows its own identity.
    root_id: Option<ObjectId>,
    count: usize,
    /// §12.2's `/ViewArea` and `/ViewClip`, read once with the catalog.
    ///
    /// Here rather than per page because they are a property of the *document*, and read at
    /// all because a page cannot say which boundary is displayed without them. The cost is
    /// one dictionary lookup in a catalog this function already holds — 58 of the 974 corpus
    /// documents state a `/ViewerPreferences` at all and none of them states either entry.
    view: (Boundary, Boundary),
    /// Pages found by scanning, where the page *tree* yielded none.
    ///
    /// Empty for every document whose tree works, which is 963 of the 974 corpus documents and
    /// every well-formed file — see [`Pages::new`] for why this exists and what it costs.
    scanned: Vec<ObjectId>,
}

impl<'a> Pages<'a> {
    /// Builds a page index for a document.
    ///
    /// Reads only the catalogue and the root of the page tree; individual pages are found
    /// when asked for.
    #[must_use]
    pub fn new(document: &'a Document) -> Self {
        let catalog = document.catalog().ok();
        let root = catalog
            .as_ref()
            .map(|catalog| document.get_key(catalog, "Pages"))
            .and_then(|pages| pages.as_dict().cloned());
        // The catalog's `/Pages` is a reference in every well-formed file, and a tree whose root
        // is also its only leaf is the case that needs it: `find_leaf` learns a node's identity
        // from the `/Kids` entry that named it, and the root was named by nobody.
        let root_id = catalog
            .as_ref()
            .and_then(|catalog| catalog.get("Pages"))
            .and_then(Object::as_reference);

        // `/Count` is authoritative when present and plausible, and the tree is counted
        // only when it is not — which keeps opening a large document cheap.
        //
        // It is not authoritative over a root with no `/Kids`, which is the one shape where
        // the entry describes a subtree the file did not write: §7.7.3.2 Table 30 requires
        // both entries of a node, so a root stating `/Count 1` and no children has contradicted
        // itself and the walk below is what settles it. One dictionary lookup, and only where
        // the root has no children — a tree with children never reaches the second condition.
        let root_has_kids = root
            .as_ref()
            .is_some_and(|node| document.get_key(node, "Kids").as_array().is_some());
        let declared = root
            .as_ref()
            .filter(|_| root_has_kids)
            .and_then(|node| document.get_key(node, "Count").as_integer())
            .and_then(|value| usize::try_from(value).ok());

        let count = match declared {
            Some(count) if count > 0 && count <= MAX_NODES_VISITED => count,
            _ => root.as_ref().map_or(0, |node| {
                count_leaves(document, Node::Direct(node, root_id))
            }),
        };

        let preferences = catalog.as_ref().map_or_else(
            crate::viewer_preferences::ViewerPreferences::default,
            |catalog| crate::viewer_preferences::ViewerPreferences::in_catalog(document, catalog),
        );

        // §7.7.3.2 makes a page tree a tree, and a file whose tree is broken has no page one —
        // which for eleven corpus documents means no render at all. But Table 31 makes `/Type`
        // **required** of a page object and says it "shall be Page", so a document that cannot be
        // walked can still be *asked*: every object declaring itself a page is one, whatever the
        // tree says. That is a recovery from the file's own declarations rather than from any
        // other reader's behaviour, and it is the same shape as `xref::rebuild`'s.
        //
        // It runs only where the tree produced nothing, so no document that opens normally pays
        // for it — `CLAUDE.md`'s "nothing eager" is intact, and 963 of the 974 corpus documents
        // never reach this line.
        let scanned = if count == 0 {
            scan_for_pages(document)
        } else {
            Vec::new()
        };

        Self {
            document,
            root,
            root_id,
            count: if count == 0 { scanned.len() } else { count },
            view: (preferences.view_area, preferences.view_clip),
            scanned,
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
        // The recovered list, in ascending object number — the only order a scan has, and a
        // documented choice rather than a claim about what the producer meant. §7.7.3.2's tree is
        // where page order lives, and a file whose tree is gone has not stated one.
        if !self.scanned.is_empty() {
            let id = *self.scanned.get(index)?;
            let object = self.document.get(id);
            let dict = object.as_dict()?;
            // §7.7.3.4's inheritance runs up `/Parent`, and a page found by scanning may still
            // have a usable chain — the *tree* is what failed, which is a walk downwards from the
            // catalog. So the ancestry is collected upwards here and applied from the top, which
            // is the same order `find_leaf` applies it in and gives a recovered page its
            // `/MediaBox` where its parents state one.
            let mut ancestry = vec![dict.clone()];
            let mut current = dict.clone();
            for _ in 0..MAX_TREE_DEPTH {
                let Some(parent) = self.document.get_key(&current, "Parent").as_dict().cloned()
                else {
                    break;
                };
                ancestry.push(parent.clone());
                current = parent;
            }
            let inherited = ancestry
                .iter()
                .rev()
                .fold(Inherited::default(), |so_far, node| {
                    so_far.overlay(self.document, Node::Direct(node, None))
                });
            return Some(build_page(
                self.document,
                dict,
                &inherited,
                self.view,
                Some(id),
            ));
        }
        let root = self.root.as_ref()?;
        let mut remaining = index;
        let mut visited = 0usize;
        find_leaf(
            self.document,
            Node::Direct(root, self.root_id),
            &Inherited::default(),
            &mut remaining,
            &mut visited,
            0,
            self.view,
        )
    }

    /// Builds a page from a dictionary that is **not** in the page tree.
    ///
    /// §12.7.7's template pages are exactly that: a page named by the document's `/Templates`
    /// tree "shall have an object type of Template rather than Page and shall have no Parent or
    /// B entry", so §7.7.3.4's inheritance — which runs up `/Parent` — has nowhere to run and
    /// such a page states everything it needs itself. This is [`Self::get`]'s own assembly with
    /// an empty ancestry, so a template page gets the same defaults, the same intersections and
    /// the same `/ViewArea` and `/ViewClip` treatment as every other page.
    ///
    /// It is not a way to draw an arbitrary dictionary: nothing here checks that the argument is
    /// a page, because the caller reached it through a tree that says it is one.
    #[must_use]
    pub fn detached(&self, dict: &Dictionary) -> Page {
        build_page(
            self.document,
            dict,
            &Inherited::default().overlay(self.document, Node::Direct(dict, None)),
            self.view,
            None,
        )
    }

    /// Returns the index of the page a reference names, or `None` when the tree has no such
    /// page.
    ///
    /// The inverse of [`Self::get`], and it is the operation §12.3.2's destinations need: a
    /// destination names its page by reference and a reader shows a page by index.
    ///
    /// Unlike [`Self::get`], this cannot use `/Count` to skip a subtree — the whole point is
    /// that the target's position is unknown, so a subtree cannot be dismissed without being
    /// searched. It therefore costs a walk of the tree in the worst case, which is why it is
    /// on the path of a person following a link and not on the path of opening a document.
    ///
    /// A reference to an intermediate node rather than to a leaf answers with the first page
    /// beneath it, which is the only page such a reference could sensibly mean.
    ///
    /// One caller is on the startup path — the catalog's `/OpenAction`, which decides which
    /// page opens — and it is the exception `CLAUDE.md`'s "no full page-tree walk" rule cannot
    /// avoid: a viewer cannot show the page a document asks for without finding it. It costs
    /// nothing for the 919 corpus documents that state no open action, and for the 55 that do
    /// it costs the walk as far as their page, which is page one for most of them.
    #[must_use]
    pub fn index_of(&self, id: ObjectId) -> Option<usize> {
        let root = self.root.as_ref()?;
        let mut counted = 0usize;
        let mut visited = 0usize;
        locate(
            self.document,
            Node::Direct(root, self.root_id),
            id,
            &mut counted,
            &mut visited,
            0,
        )
    }

    /// Every page's object and index, in **one** walk of the tree.
    ///
    /// [`Self::index_of`] answers one reference and cannot skip a subtree, because the target's
    /// position is exactly what is unknown — so a caller resolving *many* references pays a walk
    /// apiece, and that multiplies. §12.3.3's outline of ISO 32000-2 is 988 items over a
    /// 1023-page tree, and asking `index_of` for each cost **344 ms of every page turn**
    /// (session 141). This is the same information gathered once: `O(pages + references)` where
    /// the loop was `O(pages × references)`.
    ///
    /// Built on demand rather than held, because most documents never need it: a page turn
    /// wants one page and a link wants one destination, and both are what `index_of` is for.
    /// The map is the caller's to keep for as long as the answers stay useful, which is as long
    /// as the document is open — `pdf_syntax::Document` is immutable.
    ///
    /// A recovered document — one whose tree was unusable and whose pages were found by
    /// scanning — answers from that list, in the same ascending-object-number order
    /// [`Self::get`] uses, because the tree that would have stated an order is gone.
    #[must_use]
    pub fn indices(&self) -> BTreeMap<ObjectId, usize> {
        let mut out = BTreeMap::new();
        if !self.scanned.is_empty() {
            for (index, id) in self.scanned.iter().enumerate() {
                out.insert(*id, index);
            }
            return out;
        }
        let Some(root) = self.root.as_ref() else {
            return out;
        };
        let mut counted = 0usize;
        let mut visited = 0usize;
        collect(
            self.document,
            // With no identity, deliberately: see the entry `collect` records.
            Node::Direct(root, None),
            &mut out,
            &mut counted,
            &mut visited,
            0,
        );
        out
    }
}

/// Every object declaring itself a page, in ascending object number.
///
/// Table 31: `/Type` is "(Required) The type of PDF object that this dictionary describes; shall
/// be Page for a page object." So this asks each object what it says it is. A node of the page
/// *tree* says `Pages` and is not collected; an object stream's contents are reached because
/// `Document::get` resolves them like any other.
///
/// Bounded by the cross-reference table's own size, which the parser already bounds.
fn scan_for_pages(document: &Document) -> Vec<ObjectId> {
    let mut found = Vec::new();
    for number in document.xref().object_numbers() {
        if found.len() >= MAX_NODES_VISITED {
            break;
        }
        let id = ObjectId::new(number, 0);
        let object = document.get(id);
        let Some(dict) = object.as_dict() else {
            continue;
        };
        if document
            .get_key(dict, "Type")
            .as_name()
            .is_some_and(|kind| kind.as_bytes() == b"Page")
        {
            found.push(id);
        }
    }
    found
}

/// Walks the tree in page order looking for `id`; see [`Pages::index_of`].
fn locate(
    document: &Document,
    node: Node<'_>,
    id: ObjectId,
    counted: &mut usize,
    visited: &mut usize,
    depth: usize,
) -> Option<usize> {
    if depth > MAX_TREE_DEPTH || *visited > MAX_NODES_VISITED {
        return None;
    }

    let kids = node.key(document, "Kids")?;
    *visited = visited.saturating_add(1);

    let Some(kids) = kids.as_array() else {
        // A leaf, and not the one asked for, since a match is recognised at the reference in
        // the parent's `/Kids` — the only place a page's object number is written down. A node
        // that states `/Type /Pages` is not a leaf and counts as no page, which is the same
        // arithmetic `find_leaf` does.
        if !declares_a_node(document, node) {
            *counted = counted.saturating_add(1);
        }
        return None;
    };

    for entry in kids {
        if entry.as_reference() == Some(id) {
            return Some(*counted);
        }
        let Some(kid) = Node::of(entry) else { continue };
        if let Some(found) = locate(document, kid, id, counted, visited, depth.saturating_add(1)) {
            return Some(found);
        }
    }
    None
}

/// Records every page's object and index, in the order the tree states them.
///
/// [`locate`]'s walk without the early exit, and with the same two bounds: a `/Kids` cycle in a
/// hostile file is what `MAX_NODES_VISITED` is for, and a page beyond it is left out of the map
/// rather than found later by a search that would also stop.
fn collect(
    document: &Document,
    node: Node<'_>,
    out: &mut BTreeMap<ObjectId, usize>,
    counted: &mut usize,
    visited: &mut usize,
    depth: usize,
) {
    if depth > MAX_TREE_DEPTH || *visited > MAX_NODES_VISITED {
        return;
    }

    let Some(kids) = node.key(document, "Kids") else {
        return;
    };
    *visited = visited.saturating_add(1);

    // A reference to a *leaf* is the entry; a reference to an intermediate node is recorded too,
    // answering with the first page beneath it — which is what `index_of` answers for such a
    // reference and the only page it could sensibly mean. Recorded here rather than in the loop
    // below so that a `/Kids` entry naming something that is not a node is not recorded at all,
    // and so that the *root* is not: nothing in the tree names it, and `index_of` — which starts
    // at it and matches only what a `/Kids` states — cannot answer for it either.
    if let Some(id) = node.id() {
        out.entry(id).or_insert(*counted);
    }

    let Some(kids) = kids.as_array() else {
        // A leaf. Its own object number is written down in the parent's `/Kids` and not here,
        // which is why the entry is inserted by the parent below and this only counts — and a
        // node that states `/Type /Pages` is not one, on the same reading as `find_leaf`'s.
        if !declares_a_node(document, node) {
            *counted = counted.saturating_add(1);
        }
        return;
    };

    for entry in kids {
        let Some(kid) = Node::of(entry) else { continue };
        collect(
            document,
            kid,
            out,
            counted,
            visited,
            depth.saturating_add(1),
        );
    }
}

/// Counts leaf nodes, for a tree whose `/Count` is missing or implausible.
fn count_leaves(document: &Document, node: Node<'_>) -> usize {
    fn walk(document: &Document, node: Node<'_>, depth: usize, visited: &mut usize) -> usize {
        if depth > MAX_TREE_DEPTH || *visited > MAX_NODES_VISITED {
            return 0;
        }
        // A `/Kids` entry naming something that is not a dictionary is no page and no node.
        let Some(kids) = node.key(document, "Kids") else {
            return 0;
        };
        *visited = visited.saturating_add(1);

        let Some(kids) = kids.as_array() else {
            // No `/Kids` means this is a leaf, whatever its `/Type` *omits*: trusting `/Type`
            // in that direction would drop pages from files that leave it out. A node that
            // states `/Type /Pages` is the one case the file itself has answered, and Table 30
            // makes its missing `/Kids` an absence of children rather than a page.
            return usize::from(!declares_a_node(document, node));
        };

        kids.iter()
            .filter_map(Node::of)
            .map(|kid| walk(document, kid, depth.saturating_add(1), visited))
            .sum()
    }

    let mut visited = 0usize;
    walk(document, node, 0, &mut visited)
}

/// Descends to the leaf at `remaining` pages from here, accumulating inherited attributes.
///
/// **Nothing is copied out of the document until the page asked for has been found.** §7.7.3.4's
/// inheritance is applied to the node the walk descends *into* and to the leaf it stops on, and
/// not to the leaves it steps over — their overlay was always discarded, and computing it copied
/// each one's whole `/Resources` dictionary. ADR 0330.
fn find_leaf(
    document: &Document,
    node: Node<'_>,
    inherited: &Inherited,
    remaining: &mut usize,
    visited: &mut usize,
    depth: usize,
    view: (Boundary, Boundary),
) -> Option<Page> {
    if depth > MAX_TREE_DEPTH || *visited > MAX_NODES_VISITED {
        return None;
    }

    // A `/Kids` entry that is not a dictionary is neither a node nor a page. Stepped over
    // before the visit is counted, which is where resolving it and finding no dictionary left
    // it: it consumes none of `remaining` and none of the budget.
    let kids = node.key(document, "Kids")?;
    *visited = visited.saturating_add(1);

    let Some(kids) = kids.as_array() else {
        // A node whose own `/Type` says it is not a page has no page here to take, and none
        // beneath it either — so it is stepped over without consuming one of `remaining`.
        if declares_a_node(document, node) {
            return None;
        }
        // A leaf. Take it if this is the one asked for.
        if *remaining == 0 {
            let dict = node.dictionary(document)?;
            let inherited = inherited.overlay(document, node);
            return Some(build_page(document, &dict, &inherited, view, node.id()));
        }
        *remaining = remaining.saturating_sub(1);
        return None;
    };

    let inherited = inherited.overlay(document, node);
    for entry in kids {
        // The entry is read before it is resolved, because that is where the child's identity
        // is: §7.7.3.2 makes `/Kids` "an array of indirect references to the immediate
        // children", and a child held by name is a child nothing has copied yet.
        let Some(kid) = Node::of(entry) else { continue };

        // Skip whole subtrees using `/Count` where it is trustworthy: for a hundred-thousand
        // page document this is the difference between a lookup costing six node reads and
        // costing fifty thousand.
        let subtree = kid
            .key(document, "Count")
            .as_ref()
            .and_then(Object::as_integer);
        let has_kids = kid
            .key(document, "Kids")
            .as_ref()
            .and_then(Object::as_array)
            .is_some();
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
            view,
        ) {
            return Some(page);
        }
    }

    None
}

/// Assembles a page from its dictionary and the attributes it inherited.
fn build_page(
    document: &Document,
    dict: &Dictionary,
    inherited: &Inherited,
    view: (Boundary, Boundary),
    id: Option<ObjectId>,
) -> Page {
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

    // §14.11.2.1's other three boxes, each defaulting to the crop box and each intersected
    // with the medium by the clause's own rule. None of the three is inheritable — Table 31
    // marks only `/Resources`, `/MediaBox`, `/CropBox` and `/Rotate`, and §7.7.3.4 says
    // "attributes that are not explicitly identified in the table as inheritable shall not
    // be inherited" — so they are read from the page object alone.
    let production_box = |key: &str| {
        rectangle(document, &document.get_key(dict, key)).map_or(crop_box, |stated| {
            let clipped = [
                stated[0].max(media_box[0]),
                stated[1].max(media_box[1]),
                stated[2].min(media_box[2]),
                stated[3].min(media_box[3]),
            ];
            if clipped[2] > clipped[0] && clipped[3] > clipped[1] {
                clipped
            } else {
                crop_box
            }
        })
    };
    let bleed_box = production_box("BleedBox");
    let trim_box = production_box("TrimBox");
    let art_box = production_box("ArtBox");

    // §12.2's `/ViewArea` and `/ViewClip`, which name boundaries rather than stating them.
    // Both are `CropBox` for every document that says nothing, so this is the crop box in
    // every corpus document and the rest of this crate reads `display_box` regardless.
    let pick = |boundary| match boundary {
        Boundary::Media => media_box,
        Boundary::Crop => crop_box,
        Boundary::Bleed => bleed_box,
        Boundary::Trim => trim_box,
        Boundary::Art => art_box,
    };
    let (display_box, clip_box) = (pick(view.0), pick(view.1));

    // Rotation must be a multiple of 90. Negative and out-of-range values occur, so the
    // value is normalised rather than trusted.
    let rotate = inherited.rotate.unwrap_or(0).rem_euclid(360);
    let rotate = u16::try_from(rotate).unwrap_or(0);
    let rotate = (rotate / 90).saturating_mul(90) % 360;

    // "A positive number": zero, a negative and a non-finite one are all refused in favour
    // of the default rather than scaling the page by nonsense. The upper bound is this
    // implementation's answer to "the range of supported values shall be
    // implementation-dependent" — a page is at most 14 400 units on a side, so a unit of
    // 1000 is a 200-metre page, and the rasteriser's own pixel budget refuses what is left.
    let user_unit = document
        .get_key(dict, "UserUnit")
        .as_number()
        .map(narrow)
        .filter(|unit| unit.is_finite() && *unit > 0.0 && *unit <= 1000.0)
        .unwrap_or(1.0);

    Page {
        id,
        dict: dict.clone(),
        resources: inherited.resources.clone().unwrap_or_default(),
        media_box,
        crop_box,
        bleed_box,
        trim_box,
        art_box,
        display_box,
        clip_box,
        rotate,
        user_unit,
    }
}

/// Narrows a PDF number to the precision the geometry is carried in.
fn narrow(value: f64) -> f32 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "bounded to 0.0..=1000.0 by the caller"
    )]
    {
        value as f32
    }
}

/// The `/Filter` names on a stream, for reporting one that could not be applied.
fn filter_names(document: &Document, dict: &Dictionary) -> Vec<String> {
    let filter = document.get_key(dict, "Filter");
    let named = |object: &Object| {
        object
            .as_name()
            .map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned())
    };
    match &filter {
        Object::Array(items) => items
            .iter()
            .map(|item| document.resolve(item))
            .filter_map(|item| named(&item))
            .collect(),
        other => named(other).into_iter().collect(),
    }
}

/// A `/Contents` part that did not become drawable bytes.
///
/// Reported rather than skipped: a page short of one of its content streams still renders,
/// and looks like a page that was meant to be sparser than it is.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum ContentIssue {
    /// A part, or the parts together, passed [`pdf_syntax::Limits::max_stream_len`].
    ///
    /// **Not [`Self::Undecodable`], and the difference is which of the two is being described.**
    /// An undecodable part is one this reader could not read; this one it could read and
    /// declined to, which is a statement about a bound of ours. A page that reports this has
    /// content the document states and this program refused, and saying so is trap 5.
    TooLarge {
        /// Which part of `/Contents`, counting from zero, or `None` where it was the
        /// concatenation of all of them that passed the bound rather than any single part.
        part: Option<usize>,
        /// The bound, in bytes.
        limit: usize,
    },
    /// A part's filter chain could not be applied, or the stream is damaged.
    ///
    /// **Not a part this reader declined on a bound** — that is [`Self::TooLarge`], and the two
    /// were one report until ADR 0306.
    Undecodable {
        /// Which part of `/Contents`, counting from zero.
        index: usize,
        /// The `/Filter` names it declared, which is normally why.
        filters: Vec<String>,
    },
    /// A `/Contents` entry that is neither a stream nor null.
    NotAStream {
        /// Which part of `/Contents`, counting from zero.
        index: usize,
    },
    /// A `/Contents` entry naming an object this reader could not reach.
    ///
    /// ISO 32000-2 §7.3.10 makes this *conforming* rather than an error — "[a]n indirect
    /// reference to an undefined object shall not be considered an error by a PDF processor;
    /// it shall be treated as a reference to the null object" — and §7.3.9 then makes a null
    /// value "equivalent to omitting the entry". So the standard permits drawing nothing.
    /// What it does not permit is drawing nothing *in silence*: a page whose producer named a
    /// content stream and got a blank page is not the same thing as a page whose producer
    /// stated no contents, and Table 31's `/Contents` is the entry that tells them apart.
    ///
    /// This is ADR 0255's shape one clause over — a name the file never defines, said out
    /// loud — and its first witness came from outside the pdf.js corpus:
    /// `pdf-differences`' `UnknownFilter-PageContentStream.pdf`, whose content stream's own
    /// dictionary ends with one `>` where §7.3.7 requires two, so the object does not parse
    /// and the page drew nothing and said nothing. `poppler` prints *Illegal character '>'*.
    Unreachable {
        /// Which part of `/Contents`, counting from zero.
        index: usize,
        /// The object the entry named.
        object: ObjectId,
    },
}
