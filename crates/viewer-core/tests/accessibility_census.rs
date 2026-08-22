//! What a screen reader is told, counted over every document this project holds (ADR 0342).
//!
//! ADR 0323's third instrument, and the one whose shape is a **ratchet** rather than a verdict:
//! no other implementation puts a comparable tree on AT-SPI, so there is nobody to disagree with
//! us and this design does not pretend otherwise. A count that cannot fall is weaker than a judge;
//! saying so plainly is the design, and what makes the counts worth having is that each names a
//! defect *class* rather than decorating a total.
//!
//! # The first count, and why it is first
//!
//! **Pages that answer at all.** Until the four-hundred-and-ninetieth session every page but the
//! first hundred of a large tagged document answered [`Query::AccessibilityTree`] with an empty
//! list, which is exactly the answer an *untagged* page gives — so a screen reader was told a
//! thousand-page tagged document says nothing about itself, and no count, report or gate could
//! see it (ADR 0325). That defect is fixed and two residues of it are recorded in `doc/todo/31`
//! without a number beside either. This is the number.
//!
//! An empty answer is therefore classified rather than counted, and §14.7.5.4's structural parent
//! tree is what classifies it:
//!
//! - **the file names elements for the page and the answer is empty** — a page that should answer
//!   and does not, which is the defect class this census exists to make visible;
//! - **the page states no `/StructParents`**, so §14.7.5.4 has not been told and the walk falls
//!   back to the whole document's tree — ADR 0325's first recorded residue, where an empty answer
//!   on a *large* document is the bound running out and on a small one is the tree naming nothing;
//! - **the file names no elements for the page**, which is the honest case and needs no fix.
//!
//! **The predicate is not independent of the answer, and that is stated rather than hidden**
//! (trap 8). Both sides read the same file with the same crate. What makes the comparison worth
//! making is that they read *two different statements the document makes about itself*: the
//! expectation comes from §14.7.5.4's parent tree, keyed by the page's own `/StructParents`, and
//! the answer comes from walking §14.7.2's `/K` down from the root. A file whose two chains
//! disagree is precisely what ADR 0325 was about, and a census that asked one of them twice would
//! have seen nothing.
//!
//! # The other counts
//!
//! Documents with structure, by two predicates the standard keeps apart: a `/StructTreeRoot`
//! (§14.7.2), and §14.8.1's "[a] tagged PDF document shall contain a mark information dictionary
//! … with a value of true for the Marked entry". A document may have the first without claiming
//! the second, so the two counts differ and the difference is a list rather than an error.
//! ADR 0323 measured poppler's reading of the second at 78 of the pdf.js corpus with `pdfinfo`;
//! this census computes it from the file, so the run needs no subprocess and the reference reading
//! stays one command away (`pdfinfo | grep Tagged`) rather than a dependency of the ratchet.
//!
//! Then what the tree actually carries, each a census `doc/todo/31` already names as an example
//! and this promotes to a printed count: elements reached, §14.9.3's `/Alt` (and §14.9.5's `/E`)
//! carried, elements placed by Table 379's `/BBox` or §12.5.2's `/Rect`, §14.8.4.8.3's resolved
//! header cells, §12.7.5's controls behind §14.7.5.3's object references, and the elements a
//! **caret** can move through rather than only listen to (ADR 0394). And last, the count that
//! guards a *decision* rather than a capability: an untagged page answers with the honest empty
//! tree and is never given an invented reading order (ADR 0214).
//!
//! # The denominator
//!
//! Page one of every document in `doc/pdf.js/test/pdfs` and in `doc/`, and **every** page of every
//! document that states a structure tree. The specifications in `doc/` are in the population
//! deliberately: the pdf.js corpus's tagged documents are 17 pages at their largest, and the
//! defect this instrument's first count is about only appears on a document big enough for a
//! bound to run out.
//!
//! # What is asserted
//!
//! ADR 0323's rule is that an instrument's numbers enter a gate only after they have held across
//! rounds. **They have, and this is a gate since the five-hundred-and-ninetieth session**: every
//! count was unchanged from the five-hundred-and-seventh to the five-hundred-and-fifty-ninth, which
//! added a caret to every one of them without moving a single count (ADR 0394), so [`ratchet`] now
//! puts a floor under each capability and a ceiling over each defect class. Three things were
//! asserted before it and are decisions rather than counts: no input panics (principle 1), no
//! untagged page is answered with structure it does not state (ADR 0214), and no line's characters
//! disagree with its own text.
//!
//! ```text
//! cargo test --profile gates -p viewer-core --test accessibility_census -- --ignored --nocapture
//! ```

#![expect(
    clippy::print_stdout,
    reason = "a census whose whole output is what it counted"
)]

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Instant;

use pdf_model::Pages;
use pdf_model::structure::{Child, MarkInfo, Tree};
use pdf_syntax::{Document, Limits, SyntaxError};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use viewer_core::{AccessibilityNode, Answer, Command, DocumentId, PageTarget, Query, Viewer};

/// The document handle every open uses; one viewer per document, so one identity suffices.
const DOCUMENT: DocumentId = DocumentId(1);

/// The viewport every answer is mapped into, in logical pixels.
///
/// It decides only where an element's quadrilaterals land, never whether there are any — but a
/// census that changed it between rounds would move the placed count with it, so it is fixed here
/// beside the counts it affects.
const VIEWPORT: (u32, u32) = (800, 1000);

/// `viewer_core::accessibility::MAX_NODES`, which is private to the crate.
///
/// Restated rather than exposed: what the census does with it is print how many answers reach it,
/// and a value that drifted below the real bound would only *over*-report, never hide one. A
/// non-zero count here is the other half of `doc/todo/31`'s truncation entry — an answer cut at
/// the bound, with nothing in [`Answer::Accessibility`] able to say it was.
const ANSWER_BOUND: usize = 8192;

/// The corpus documents that refuse §7.6.4.1's default user password, with the password each
/// one's own pdf.js issue records — the same eight `pdf-syntax`'s `encryption.rs` verifies, and
/// ADR 0323's rule that the denominator is what opens "without a password or with the corpus's
/// known ones".
const KNOWN_PASSWORDS: &[(&str, &str)] = &[
    ("issue15893_reduced.pdf", "test"),
    ("issue3371.pdf", "ELXRTQWS"),
    ("bug1782186.pdf", "Hello"),
    ("issue6010_1.pdf", "abc"),
    ("issue6010_2.pdf", "\u{E6}\u{F8}\u{E5}"),
    ("saslprep-r6.pdf", "S\u{AA}SL\u{AD}prep"),
    ("pr6531_1.pdf", "asdfasdf"),
    ("print_protection.pdf", "1234"),
];

/// How many witnesses of one class are printed before the rest are summarised.
const WITNESSES: usize = 30;

/// One census, whether of one document or of the whole population.
#[derive(Default)]
struct Census {
    /// Documents examined, and those no password on record opens.
    documents: usize,
    refused_open: Vec<(String, String)>,
    /// §14.7.2's `/StructTreeRoot`, and §14.8.1's `/MarkInfo` `/Marked`.
    with_structure: usize,
    marked: usize,
    /// Documents stating one predicate and not the other, with which way round.
    predicates_differ: Vec<(String, String)>,
    /// Pages of documents that state a structure tree, and how many answer with at least one node.
    structured_pages: usize,
    answered_pages: usize,
    /// An empty answer where §14.7.5.4's parent tree names elements for the page: the defect class.
    named_but_silent: Vec<(String, String)>,
    /// An empty answer where the page states no `/StructParents` — ADR 0325's first residue.
    no_parent_key_silent: Vec<(String, String)>,
    /// An empty answer where the file names no elements for the page: the honest case.
    nothing_named: usize,
    /// Answers that reach [`ANSWER_BOUND`], which may have been cut with nothing saying so.
    at_bound: Vec<(String, String)>,
    /// What the answers carry.
    nodes: usize,
    substituted: usize,
    placed: usize,
    /// Elements placed by §14.8.3.3's content rectangle — what their own marks drew.
    ///
    /// A different route from [`Self::placed`] and counted apart from it, because the two are
    /// different kinds of fact: that one is what a *producer* wrote about an element and this is
    /// what the page turned out to draw. Their sum is not a total — an element may have both, and
    /// most of the ones that have this have neither `/BBox` nor an annotation (ADR 0486).
    derived: usize,
    /// Elements with no place by any of the three routes, which is what a client cannot point at.
    placeless: usize,
    header_cells: usize,
    header_associations: usize,
    controls: usize,
    /// Elements whose content *is* an annotation, which is what a client may ask to click.
    ///
    /// §12.5.1 makes activation something a person does to an annotation, and Table 368 gives
    /// three structure types — `Link`, `Annot` and `Form` — whose content is one. This counts the
    /// elements that name one through §14.7.5.3, which is the population `viewer-accessibility`
    /// declares [`accesskit::Action::Click`] on. It is a superset of [`Self::controls`]: every
    /// widget is an annotation and most annotations are not widgets.
    annotations: usize,
    /// Elements a caret can move through, the lines they cross, and the characters on them.
    ///
    /// The class this is about is an element that is *read* and cannot be *moved through*: until
    /// a text run existed, a paragraph crossed as one string and an assistive technology could
    /// speak it whole or not at all. So the count worth watching is the third one against
    /// [`Self::nodes`] — how much of what a page says has a place a caret can stand in.
    with_lines: usize,
    lines: usize,
    characters: usize,
    /// An element whose own text a caret reaches and whose own text is empty, which cannot happen.
    ///
    /// Not a count but an assertion in the shape of one: [`viewer_core::TextLine`] states that its
    /// text is exactly the readback of its characters, and every consumer of the type indexes one
    /// by the other. A line breaking it would put a platform's caret arithmetic out of step with
    /// its own strings, which is the kind of defect that shows up as a screen reader reading the
    /// wrong word rather than as a crash.
    inconsistent_lines: Vec<(String, String)>,
    /// Page one of every document with no structure tree, and how many answer the honest nothing.
    untagged_pages: usize,
    untagged_honest: usize,
    /// An untagged page answered with structure — ADR 0214's decision, broken.
    invented: Vec<(String, String)>,
    /// A document whose examination panicked, which principle 1 forbids.
    panicked: Vec<(String, String)>,
}

impl Census {
    /// Folds one document's census into the population's.
    fn absorb(&mut self, from: Self) {
        self.documents = self.documents.saturating_add(from.documents);
        self.refused_open.extend(from.refused_open);
        self.with_structure = self.with_structure.saturating_add(from.with_structure);
        self.marked = self.marked.saturating_add(from.marked);
        self.predicates_differ.extend(from.predicates_differ);
        self.structured_pages = self.structured_pages.saturating_add(from.structured_pages);
        self.answered_pages = self.answered_pages.saturating_add(from.answered_pages);
        self.named_but_silent.extend(from.named_but_silent);
        self.no_parent_key_silent.extend(from.no_parent_key_silent);
        self.nothing_named = self.nothing_named.saturating_add(from.nothing_named);
        self.at_bound.extend(from.at_bound);
        self.nodes = self.nodes.saturating_add(from.nodes);
        self.substituted = self.substituted.saturating_add(from.substituted);
        self.placed = self.placed.saturating_add(from.placed);
        self.derived = self.derived.saturating_add(from.derived);
        self.placeless = self.placeless.saturating_add(from.placeless);
        self.header_cells = self.header_cells.saturating_add(from.header_cells);
        self.header_associations = self
            .header_associations
            .saturating_add(from.header_associations);
        self.controls = self.controls.saturating_add(from.controls);
        self.annotations = self.annotations.saturating_add(from.annotations);
        self.with_lines = self.with_lines.saturating_add(from.with_lines);
        self.lines = self.lines.saturating_add(from.lines);
        self.characters = self.characters.saturating_add(from.characters);
        self.inconsistent_lines.extend(from.inconsistent_lines);
        self.untagged_pages = self.untagged_pages.saturating_add(from.untagged_pages);
        self.untagged_honest = self.untagged_honest.saturating_add(from.untagged_honest);
        self.invented.extend(from.invented);
        self.panicked.extend(from.panicked);
    }

    /// Adds what one page's answer carries.
    fn carried(&mut self, nodes: &[AccessibilityNode], where_: &str) {
        self.nodes = self.nodes.saturating_add(nodes.len());
        for node in nodes {
            if node.substituted {
                self.substituted = self.substituted.saturating_add(1);
            }
            if node.bounds.is_some() {
                self.placed = self.placed.saturating_add(1);
            }
            if node.drawn.is_some() {
                self.derived = self.derived.saturating_add(1);
            }
            // The three routes `viewer_accessibility::tree::place` asks in order. A node that has
            // none of them implements no `Component` interface on AT-SPI at all, so this is the
            // count of what a magnifier cannot be pointed at.
            if node.bounds.is_none() && node.drawn.is_none() && node.quads.is_empty() {
                self.placeless = self.placeless.saturating_add(1);
            }
            if !node.headers.is_empty() {
                self.header_cells = self.header_cells.saturating_add(1);
                self.header_associations =
                    self.header_associations.saturating_add(node.headers.len());
            }
            if node.control.is_some() {
                self.controls = self.controls.saturating_add(1);
            }
            if node.annotation.is_some() {
                self.annotations = self.annotations.saturating_add(1);
            }
            if !node.lines.is_empty() {
                self.with_lines = self.with_lines.saturating_add(1);
            }
            self.lines = self.lines.saturating_add(node.lines.len());
            for line in &node.lines {
                self.characters = self.characters.saturating_add(line.characters.len());
                let stated: usize = line
                    .characters
                    .iter()
                    .map(|character| character.bytes)
                    .sum();
                if stated != line.text.len() {
                    self.inconsistent_lines.push((
                        where_.to_owned(),
                        format!(
                            "a {} line's characters state {stated} byte(s) and its text is {}",
                            node.role,
                            line.text.len()
                        ),
                    ));
                }
            }
        }
    }
}

/// Every document this instrument counts: the pdf.js corpus, and the specifications in `doc/`.
///
/// `None` when neither is on disk, which is a fresh clone that has run neither
/// `git submodule update --init` nor `doc/environment.md`'s one `unzip`.
fn population() -> Option<Vec<PathBuf>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc");
    let mut files: Vec<PathBuf> = Vec::new();
    for directory in [root.join("pdf.js/test/pdfs"), root] {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        files.extend(
            entries
                .filter_map(|entry| entry.ok().map(|entry| entry.path()))
                .filter(|path| path.extension().is_some_and(|extension| extension == "pdf")),
        );
    }
    if files.is_empty() {
        return None;
    }
    files.sort();
    Some(files)
}

/// The password on record for one file, or the empty default §7.6.4.1 starts with.
fn password_for(name: &str) -> &'static str {
    KNOWN_PASSWORDS
        .iter()
        .find(|(known, _)| *known == name)
        .map_or("", |(_, password)| *password)
}

/// How many elements the whole document's tree holds, and whether the read was itself bounded.
///
/// The diagnosis for an empty answer on a page that states no `/StructParents`: the fallback is a
/// walk of the *document's* tree, so a tree larger than [`ANSWER_BOUND`] cannot be walked to this
/// page's part of it and the silence is ADR 0325's residue rather than a document that says
/// nothing. Computed only for a document that produced such a page, because it is a walk.
fn tree_size(document: &Document, tree: &Tree) -> (usize, bool) {
    let walk = tree.walk(document);
    let elements = walk
        .items
        .iter()
        .filter(|(_, child)| matches!(child, Child::Element(_)))
        .count();
    (elements, walk.truncated)
}

/// One document: opened at the boundary and read beside it, page by page.
fn examine(path: &Path) -> Census {
    let mut census = Census::default();
    let name = path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    );
    let Ok(bytes) = std::fs::read(path) else {
        return census;
    };
    census.documents = 1;
    let password = password_for(&name);

    // The document, read beside the boundary: this is where the *expectation* comes from —
    // §14.7.2's tree, §14.8.1's claim and §14.7.5.4's parent tree — while every answer counted
    // below comes through `Query`. See the module comment on what that does and does not buy.
    let document = match Document::open_with_password(bytes.clone(), Limits::DEFAULT, password) {
        Ok(document) => document,
        Err(SyntaxError::PasswordRequired) => {
            census
                .refused_open
                .push((name, "needs a password nobody has recorded".to_owned()));
            return census;
        }
        Err(error) => {
            census.refused_open.push((name, error.to_string()));
            return census;
        }
    };
    let tree = Tree::of(&document);
    if tree.is_some() {
        census.with_structure = 1;
    }
    let marked = MarkInfo::read(&document).marked;
    if marked {
        census.marked = 1;
    }
    match (tree.is_some(), marked) {
        (true, false) => census.predicates_differ.push((
            name.clone(),
            "a /StructTreeRoot without §14.8.1's /MarkInfo /Marked".to_owned(),
        )),
        (false, true) => census.predicates_differ.push((
            name.clone(),
            "§14.8.1's /MarkInfo /Marked without a /StructTreeRoot".to_owned(),
        )),
        _ => {}
    }

    let mut viewer = Viewer::new(VIEWPORT.0, VIEWPORT.1, 1.0);
    let opened = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes,
            password: (!password.is_empty()).then(|| password.to_owned()),
            fragment: None,
        })
        .any(|event| matches!(event, viewer_core::Event::Opened { .. }));
    if !opened {
        census.refused_open.push((
            name,
            "the boundary did not open what pdf-syntax did".to_owned(),
        ));
        return census;
    }

    let Answer::Count(count) = viewer.query(Query::PageCount) else {
        census
            .refused_open
            .push((name, "the boundary answered no page count".to_owned()));
        return census;
    };
    sweep(
        &mut census,
        &document,
        tree.as_ref(),
        &mut viewer,
        &name,
        count,
    );
    census
}

/// Every page of one document, asked at the boundary and classified beside it.
///
/// Every page where the document states a structure tree; page one otherwise — the untagged
/// population's whole question is answered by any one of its pages, and asking a thousand of them
/// would buy the same answer a thousand times.
fn sweep(
    census: &mut Census,
    document: &Document,
    tree: Option<&Tree>,
    viewer: &mut Viewer,
    name: &str,
    count: usize,
) {
    let pages = Pages::new(document);
    let mut sized: Option<(usize, bool)> = None;
    for index in 0..if tree.is_some() { count } else { count.min(1) } {
        viewer
            .handle(Command::GoTo(PageTarget::Index(index)))
            .for_each(drop);
        let Answer::Accessibility(shown) = viewer.query(Query::AccessibilityTree) else {
            continue;
        };
        // **The entry for the page asked for, by its own number.** The arrangement here is Table
        // 29's default, so there is one entry and it is this page — and taking it by *name*
        // rather than by position is what makes a page answered under another page's number read
        // as the silence it would be, instead of passing as this page's tree.
        let nodes: Vec<AccessibilityNode> = shown
            .into_iter()
            .filter(|structure| structure.page == index)
            .flat_map(|structure| structure.nodes)
            .collect();
        let where_ = format!("{name} p{}", index.saturating_add(1));
        let Some(tree) = tree else {
            census.untagged_pages = census.untagged_pages.saturating_add(1);
            if nodes.is_empty() {
                census.untagged_honest = census.untagged_honest.saturating_add(1);
            } else {
                census.invented.push((
                    where_,
                    format!(
                        "{} node(s) on a document with no /StructTreeRoot",
                        nodes.len()
                    ),
                ));
            }
            continue;
        };
        census.structured_pages = census.structured_pages.saturating_add(1);
        census.carried(&nodes, &where_);
        if nodes.len() >= ANSWER_BOUND {
            census.at_bound.push((
                where_.clone(),
                format!("{} nodes, at viewer-core's own bound", nodes.len()),
            ));
        }
        if nodes.is_empty() {
            classify_silence(
                census,
                document,
                tree,
                pages.get(index).map(|page| page.dict),
                where_,
                &mut sized,
            );
        } else {
            census.answered_pages = census.answered_pages.saturating_add(1);
        }
    }
}

/// Which kind of silence one empty answer is, by §14.7.5.4 — see the module comment.
///
/// `sized` is the document's tree size, computed at most once and only where a page needs the
/// diagnosis: the fallback route walks the *document's* tree, so how big that tree is decides
/// whether an empty answer is the file saying nothing or the walk's bound running out.
fn classify_silence(
    census: &mut Census,
    document: &Document,
    tree: &Tree,
    page: Option<pdf_syntax::Dictionary>,
    where_: String,
    sized: &mut Option<(usize, bool)>,
) {
    match page.and_then(|page| tree.elements_on_page(document, &page)) {
        Some(elements) if !elements.is_empty() => census.named_but_silent.push((
            where_,
            format!(
                "§14.7.5.4's parent tree names {} element(s) for the page and the answer is empty",
                elements.len()
            ),
        )),
        Some(_) => census.nothing_named = census.nothing_named.saturating_add(1),
        None => {
            let (elements, truncated) = *sized.get_or_insert_with(|| tree_size(document, tree));
            census.no_parent_key_silent.push((
                where_,
                format!(
                    "no /StructParents, so the fallback walks the document's tree of {elements} \
                     element(s), {}",
                    if elements > ANSWER_BOUND || truncated {
                        "which is larger than the walk's bound — ADR 0325's residue"
                    } else {
                        "all of them, and none is on this page"
                    }
                ),
            ));
        }
    }
}

/// Prints one class of witness, capped, with its length.
fn print_witnesses(what: &str, entries: &[(String, String)]) {
    println!("  {what}: {}", entries.len());
    for (name, why) in entries.iter().take(WITNESSES) {
        println!("    {name}: {why}");
    }
    if entries.len() > WITNESSES {
        println!("    … and {} more", entries.len().saturating_sub(WITNESSES));
    }
}

/// Prints the whole census, in the order the counts were argued for.
fn report(census: &Census, files: usize, seconds: f64) {
    println!(
        "{files} documents in {seconds:.1}s: page one of each, and every page of every document \
         with structure"
    );
    print_witnesses("refused open", &census.refused_open);
    println!(
        "structure (§14.7.2's /StructTreeRoot): {} documents",
        census.with_structure
    );
    println!(
        "  tagged (§14.8.1's /MarkInfo /Marked): {} documents",
        census.marked
    );
    print_witnesses("one predicate and not the other", &census.predicates_differ);
    println!(
        "pages that answer at all: {} of {} pages of documents with structure",
        census.answered_pages, census.structured_pages
    );
    print_witnesses(
        "the file names elements for the page and the answer is empty",
        &census.named_but_silent,
    );
    print_witnesses(
        "no /StructParents, and the whole-tree fallback answered nothing",
        &census.no_parent_key_silent,
    );
    println!(
        "  the file names no elements for the page, which is the honest case: {}",
        census.nothing_named
    );
    print_witnesses(
        "answers reaching viewer-core's node bound, which nothing says out loud",
        &census.at_bound,
    );
    println!("elements reached: {}", census.nodes);
    println!(
        "  §14.9.3's /Alt or §14.9.5's /E carried: {}",
        census.substituted
    );
    println!(
        "  placed by Table 379's /BBox or §12.5.2's /Rect: {}",
        census.placed
    );
    println!(
        "  placed by §14.8.3.3's content rectangle, which is what they drew: {}",
        census.derived
    );
    println!(
        "  with no place by any of the three routes: {}",
        census.placeless
    );
    println!(
        "  cells with §14.8.4.8.3's headers resolved: {} ({} associations)",
        census.header_cells, census.header_associations
    );
    println!(
        "  §12.7.5's controls behind §14.7.5.3's object references: {}",
        census.controls
    );
    println!(
        "  elements whose content is §12.5's annotation, which a client may click: {}",
        census.annotations
    );
    println!(
        "  elements a caret can move through: {} ({} lines, {} characters)",
        census.with_lines, census.lines, census.characters
    );
    print_witnesses(
        "a line whose characters and text disagree, which no consumer could index",
        &census.inconsistent_lines,
    );
    println!(
        "untagged pages answering the honest empty tree: {} of {}",
        census.untagged_honest, census.untagged_pages
    );
    print_witnesses(
        "an untagged page answered with structure it does not state",
        &census.invented,
    );
    print_witnesses("panicked", &census.panicked);
}

/// The instrument. Ignored: minutes over every document this project holds, every page of the
/// tagged ones — run it deliberately, under the gates profile or in release.
#[test]
#[ignore = "corpus-scale; every page of every tagged document — run explicitly, in release"]
fn what_a_screen_reader_is_told_about_every_document() {
    let Some(files) = population() else {
        println!("skipped: neither doc/pdf.js nor doc/'s specifications are on disk");
        return;
    };
    let census = Mutex::new(Census::default());
    let started = Instant::now();
    files.par_iter().for_each(|path| {
        let name = path.display().to_string();
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| examine(path)));
        let mut one = match outcome {
            Ok(one) => one,
            Err(payload) => {
                let what = payload
                    .downcast_ref::<&str>()
                    .map(ToString::to_string)
                    .or_else(|| payload.downcast_ref::<String>().cloned())
                    .unwrap_or_else(|| "non-string panic payload".to_owned());
                let mut one = Census::default();
                one.panicked.push((name, what));
                one
            }
        };
        if let Ok(mut census) = census.lock() {
            census.absorb(std::mem::take(&mut one));
        }
    });
    let elapsed = started.elapsed();
    let census = census.into_inner().expect("reported through the census");
    report(&census, files.len(), elapsed.as_secs_f64());

    assert!(
        census.panicked.is_empty(),
        "principle 1: no panic on any input"
    );
    // ADR 0214's decision, asserted rather than counted: reading order is what §14.7 exists to
    // state, and a page whose document states none must say so instead of being given a guess.
    assert!(
        census.invented.is_empty(),
        "an untagged page was answered with structure: {:?}",
        census.invented
    );
    // `TextLine`'s own invariant, over the whole population rather than over an example: the
    // characters' byte counts sum to the line's text. Every platform text interface indexes one
    // by the other, so a line that broke it would misplace a caret rather than fail.
    assert!(
        census.inconsistent_lines.is_empty(),
        "a line's characters and its text disagree: {:?}",
        census.inconsistent_lines
    );
    ratchet(&census, files.len());
}

/// The ratchet, which is what ADR 0323 called this instrument's verdict shape.
///
/// # Why the numbers may be written down here and nowhere else
///
/// `CLAUDE.md`'s rule is that a fact a command can print is not written down — and a ratchet is
/// the exception it names, because a floor is not a *report* of a count but the previous run's
/// count used as a *bound*. The same shape `pdf-model`'s text-extraction gate has carried since
/// ADR 0333. What must not appear is these numbers in an instruction file, and they do not.
///
/// # Why now, and not in the round that built the census
///
/// ADR 0323's rule: an instrument's counts enter a gate only once they have held across rounds.
/// They have — every one of them was unchanged from the five-hundred-and-seventh session to the
/// five-hundred-and-fifty-ninth, which added the caret without moving anything else — and this is
/// `doc/todo/05`'s third instrument being closed rather than a new promise.
///
/// # Which way each number moves
///
/// A **capability** count may only rise: elements reached, elements placed, cells given headers,
/// somewhere a caret can stand. A **defect** count may only fall: a page whose file names elements
/// for it and that answers nothing, an answer cut at the bound, a document that will not open.
/// Trap 5's amendment applies to the first kind and not to the second — a rise in a capability
/// count is always welcome, and a *fall* is what this catches.
///
/// # The population is checked before the counts are
///
/// Every floor here is a count over 988 documents, and a tree with the `doc/pdf.js` submodule
/// unchecked has 14. Comparing a smaller population against these would fail for the one reason
/// that is not a regression, so the floors are skipped and the skip says so — which is the same
/// guard ADR 0421 put under the selection verdict's judged set.
fn ratchet(census: &Census, files: usize) {
    /// How many documents the floors below were measured over.
    const POPULATION: usize = 988;

    if files < POPULATION {
        println!(
            "not ratcheted: {files} documents against the {POPULATION} the floors were taken \
             over — `git submodule update --init doc/pdf.js`, and `doc/environment.md`'s one unzip"
        );
        return;
    }
    // A capability may only rise. Each is one line of `report` above.
    let floor = |what: &str, is: usize, was: usize| {
        assert!(
            is >= was,
            "{what}: {is}, and it was {was} — a screen reader is being told less than it was"
        );
    };
    floor("documents with structure", census.with_structure, 104);
    floor("pages that answer at all", census.answered_pages, 1502);
    floor("elements reached", census.nodes, 102_853);
    floor("§14.9.3's /Alt carried", census.substituted, 664);
    floor("elements placed", census.placed, 7538);
    floor("elements placed by their own marks", census.derived, 93_267);
    floor("cells with headers", census.header_cells, 16_617);
    floor("header associations", census.header_associations, 27_273);
    floor("§12.7.5's controls", census.controls, 272);
    floor("elements that are annotations", census.annotations, 7413);
    floor("elements a caret reaches", census.with_lines, 57_116);
    floor("lines", census.lines, 114_011);
    floor("characters", census.characters, 2_974_185);
    floor(
        "untagged pages answering honestly",
        census.untagged_honest,
        876,
    );

    // A defect class may only fall. The first two are already empty and stay so; the other two are
    // populations with a number, and each has its own entry in `doc/todo/31`.
    let ceiling = |what: &str, entries: &[(String, String)], was: usize| {
        assert!(
            entries.len() <= was,
            "{what}: {} of them, and there were {was}: {entries:?}",
            entries.len()
        );
    };
    ceiling(
        "pages whose file names elements and whose answer is empty",
        &census.named_but_silent,
        0,
    );
    ceiling("answers cut at the node bound", &census.at_bound, 0);
    ceiling(
        "pages with no /StructParents whose fallback answered nothing",
        &census.no_parent_key_silent,
        56,
    );
    ceiling("documents that would not open", &census.refused_open, 2);
}

/// One tagged document through the whole census, un-ignored, so the classification cannot rot
/// between explicit runs.
///
/// `structure_simple.pdf` is the smallest corpus document that states a structure tree, and what
/// it pins is the shape of the answer rather than a count: the document is counted as structured,
/// its page answers, and no class of silence is entered.
#[test]
fn the_census_reads_a_tagged_document_as_structured_and_answering() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc/pdf.js/test/pdfs/structure_simple.pdf");
    if !path.exists() {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    }
    let census = examine(&path);
    assert_eq!(census.with_structure, 1, "the document states a tree");
    assert_eq!(census.structured_pages, 1, "one page, and it was visited");
    assert_eq!(census.answered_pages, 1, "and it answered");
    assert!(census.nodes > 0, "with elements");
    assert!(census.named_but_silent.is_empty());
    assert!(census.no_parent_key_silent.is_empty());
    assert_eq!(census.untagged_pages, 0, "it is not in that population");
    // And a caret reaches its words. The count is not pinned — that is the ratchet's business —
    // but "some element of a document that draws text has a line" is the shape of the answer,
    // and a page that lost every line would still pass every assertion above it.
    assert!(
        census.with_lines > 0 && census.characters > 0,
        "a tagged page that draws text answers with somewhere for a caret to stand"
    );
}

/// And an untagged one, which is 885 of the corpus's 974: the honest empty answer, counted as
/// honest rather than as a defect.
#[test]
fn the_census_reads_an_untagged_document_as_honestly_silent() {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/pdf.js/test/pdfs/basicapi.pdf");
    if !path.exists() {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    }
    let census = examine(&path);
    assert_eq!(census.with_structure, 0);
    assert_eq!(census.untagged_pages, 1, "page one alone answers for it");
    assert_eq!(census.untagged_honest, 1);
    assert!(census.invented.is_empty());
}
