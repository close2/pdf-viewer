//! How much of a tagged page's readback no structure element reaches — §14.8.2.2.2's artifact
//! by absence, measured before it is computed.
//!
//! ISO 32000-2 §14.8.2.2.2, two paragraphs under Table 363:
//!
//! > Any content that is not included in the structure tree is an artifact
//!
//! — and the sentence goes on to say it even of content no marked-content sequence tagged
//! `/Artifact` encloses. NOTE 2 widens the subject: "[t]he phrase 'any content' above refers
//! to all page content as well as annotations."
//!
//! A reader that acts on that sentence reclassifies content **no file marked**, so the first
//! question is how much there is. On a tagged page the readback divides four ways, and this counts
//! each in characters:
//!
//! - inside a marked-content sequence whose `/MCID` §14.7.5.4's parent tree resolves to an element
//!   — *reached*, and the only class §14.8.2.2.1 calls real content;
//! - inside a §14.8.2.2.2 `/Artifact` sequence — an artifact the producer declared;
//! - inside a sequence carrying an `/MCID` the parent tree resolves to **nothing** — a producer's
//!   omission or a broken entry, and an artifact by this clause either way;
//! - inside no marked-content sequence at all — the unmarked run, the class the clause's "even
//!   when not enclosed" is written for.
//!
//! The last two together are the population a computation would move. A tagged document with a
//! great deal of it is either a producer that tagged half its page or this reader failing to find
//! the tree, and the two have different consequences — which is why this runs before the code that
//! acts on the answer (`doc/traps/parsers-and-streams.md` trap 8).
//!
//! ```sh
//! cargo run --release -p pdf-model --example unreached_content_census -- \
//!   $(find doc/pdf.js/test/pdfs -maxdepth 1 -name '*.pdf') \
//!   $(find -L doc/corpora -name '*.pdf') doc/*.pdf
//! ```
//!
//! **Only a document with a `/StructTreeRoot` is interpreted.** §14.8's rules are addressed to
//! tagged PDF files; on an untagged one every character is outside a structure tree that does not
//! exist, and calling the whole page an artifact would be a statement about the standard's scope
//! rather than about the page.
#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is the measurement"
)]
#![expect(
    clippy::arithmetic_side_effects,
    reason = "counters over a corpus four orders of magnitude below what a usize counts, and \
              this is a measurement rather than a shipped path"
)]

use std::collections::{BTreeMap, BTreeSet};

use rayon::prelude::*;

use pdf_model::content::{ArtifactSource, ContentStream, Interpretation};
use pdf_model::structure::{Child, MarkInfo, ParentTree, Tree};
use pdf_model::{Pages, interpret};
use pdf_syntax::{Document, ObjectId};

/// What one document turned out to be.
struct Finding {
    /// The file's name, for the list of what matched.
    name: String,
    /// Whether the catalog's `/MarkInfo` states `/Marked true` — §14.8.1's claim to be tagged.
    marked: bool,
    /// Pages interpreted.
    pages: usize,
    /// Pages on which at least one character is reached by no element and no `/Artifact` tag.
    pages_with_unreached: usize,
    /// Characters of readback, over every interpreted page.
    characters: usize,
    /// …inside a sequence whose `/MCID` the parent tree resolves.
    reached: usize,
    /// …inside a §14.8.2.2.2 `/Artifact` marked-content sequence.
    tagged_artifact: usize,
    /// …inside a sequence the parent tree misses but some element's `/K` names.
    reached_by_k: usize,
    /// …inside a sequence whose `/MCID` resolves to nothing.
    unresolved_identifier: usize,
    /// …inside no marked-content sequence at all.
    unmarked: usize,
    /// The worst page's share of unreached characters, and which page it was.
    worst_page: Option<(usize, f64)>,
    /// Unreached characters on a page whose object states no `/StructParents` at all.
    ///
    /// Table 359 makes that entry "[r]equired for all content streams containing marked-content
    /// sequences that are structural content items", so a page without one has said nothing about
    /// its own sequences — which is a different fact from a page that said something and left a
    /// run out of it, and the reason this is counted apart (ADR 0325).
    unreached_without_struct_parents: usize,
    /// Characters this tree's own `interpret` classified [`ArtifactSource::Absence`].
    ///
    /// The second instrument, beside the first: everything above is computed here from
    /// `Interpretation`'s public spans and the document's own dictionaries, and this is what the
    /// shipped computation produced. The two answer the same question by different routes and the
    /// difference between them is the population the shipped one reaches and this one cannot —
    /// §14.7.5.3's annotations, whose appearance streams carry no `/MCID` and whose text no public
    /// span attributes to them.
    shipped: usize,
    /// The first few unreached runs, so that what the count matched can be read (trap 11).
    samples: Vec<String>,
}

impl Finding {
    /// Characters §14.8.2.2.2's sentence would reclassify: neither reached nor already tagged.
    const fn unreached(&self) -> usize {
        self.unresolved_identifier + self.unmarked
    }

    /// That as a share of the readback.
    fn share(&self) -> f64 {
        if self.characters == 0 {
            return 0.0;
        }
        #[expect(
            clippy::cast_precision_loss,
            reason = "a percentage printed to one decimal place"
        )]
        {
            self.unreached() as f64 * 100.0 / self.characters as f64
        }
    }
}

fn main() {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    let mut findings: Vec<Finding> = paths.par_iter().filter_map(|path| examine(path)).collect();
    findings.sort_by(|one, other| {
        other
            .unreached()
            .cmp(&one.unreached())
            .then_with(|| one.name.cmp(&other.name))
    });

    // What it matched, before the count that summarises it (trap 11).
    for found in findings.iter().filter(|found| found.unreached() > 0) {
        let worst = found.worst_page.map_or_else(
            || "no page".to_owned(),
            |(page, share)| format!("page {page} at {share:.1}%"),
        );
        println!(
            "{}: {} of {} character(s) unreached ({:.1}%) over {} page(s), {} on {} page(s); \
             worst {worst}; {} in an unresolved /MCID, {} unmarked, {} tagged /Artifact, \
             {} by /K alone; `interpret` classified {}{}",
            found.name,
            found.unreached(),
            found.characters,
            found.share(),
            found.pages,
            found.reached,
            found.pages_with_unreached,
            found.unresolved_identifier,
            found.unmarked,
            found.tagged_artifact,
            found.reached_by_k,
            found.shipped,
            if found.marked { "" } else { "; /Marked absent" },
        );
        for sample in &found.samples {
            println!("    {sample}");
        }
    }

    let tagged = findings.len();
    let characters: usize = findings.iter().map(|found| found.characters).sum();
    let reached: usize = findings.iter().map(|found| found.reached).sum();
    let artifact: usize = findings.iter().map(|found| found.tagged_artifact).sum();
    let by_k: usize = findings.iter().map(|found| found.reached_by_k).sum();
    let no_struct_parents: usize = findings
        .iter()
        .map(|found| found.unreached_without_struct_parents)
        .sum();
    let unresolved: usize = findings
        .iter()
        .map(|found| found.unresolved_identifier)
        .sum();
    let unmarked: usize = findings.iter().map(|found| found.unmarked).sum();
    let pages: usize = findings.iter().map(|found| found.pages).sum();
    let dirty: usize = findings
        .iter()
        .map(|found| found.pages_with_unreached)
        .sum();
    let documents = findings
        .iter()
        .filter(|found| found.unreached() > 0)
        .count();

    println!(
        "{} path(s) given, {tagged} with a /StructTreeRoot interpreted, {pages} page(s)",
        paths.len(),
    );
    println!(
        "characters: {characters}; reached by an element {reached}; tagged /Artifact {artifact}; \
         unresolved /MCID {unresolved}; unmarked {unmarked}"
    );
    println!(
        "reached by an element's /K where the parent tree missed it: {by_k}; \
         unreached on a page stating no /StructParents: {no_struct_parents}"
    );
    println!(
        "unreached (unresolved + unmarked): {} over {documents} document(s) and {dirty} page(s)",
        unresolved + unmarked,
    );

    let claimed: Vec<&Finding> = findings.iter().filter(|found| found.marked).collect();
    let unclaimed: Vec<&Finding> = findings.iter().filter(|found| !found.marked).collect();
    population_line("/Marked true", &claimed);
    population_line("/Marked absent", &unclaimed);
}

/// One summary line per population, which is the measurement this census exists for.
///
/// §14.8.1 makes `/MarkInfo` with `/Marked true` the claim to be a tagged PDF, and §14.8.2.2.2's
/// sentence is addressed to "tagged PDF files". A structure tree without that claim is a document
/// that never said §14.8's rules apply to it, so the two are never added together.
fn population_line(label: &str, population: &[&Finding]) {
    let characters: usize = population.iter().map(|found| found.characters).sum();
    let unreached: usize = population.iter().map(|found| found.unreached()).sum();
    let dirty = population
        .iter()
        .filter(|found| found.unreached() > 0)
        .count();
    let loose: usize = population
        .iter()
        .map(|found| found.unreached_without_struct_parents)
        .sum();
    let by_k: usize = population.iter().map(|found| found.reached_by_k).sum();
    let unresolved: usize = population
        .iter()
        .map(|found| found.unresolved_identifier)
        .sum();
    let unmarked: usize = population.iter().map(|found| found.unmarked).sum();
    let shipped: usize = population.iter().map(|found| found.shipped).sum();
    println!(
        "{label}: {} document(s), {characters} character(s), {unreached} unreached \
         ({unresolved} in an unresolved /MCID, {unmarked} carrying no /MCID at all), \
         {dirty} document(s) with any, {loose} on a page stating no /StructParents, \
         {by_k} reached by /K alone, {shipped} classified by `interpret` itself",
        population.len(),
    );
}

/// Reads one document, interpreting its pages only where it has a structure tree.
fn examine(path: &str) -> Option<Finding> {
    let name = std::path::Path::new(path).file_name().map_or_else(
        || path.to_owned(),
        |name| name.to_string_lossy().into_owned(),
    );
    let bytes = std::fs::read(path).ok()?;
    let document = Document::open(bytes).ok()?;
    Tree::of(&document)?;

    let mut found = Finding {
        name,
        marked: MarkInfo::read(&document).marked,
        pages: 0,
        pages_with_unreached: 0,
        characters: 0,
        reached: 0,
        tagged_artifact: 0,
        reached_by_k: 0,
        unresolved_identifier: 0,
        unmarked: 0,
        worst_page: None,
        unreached_without_struct_parents: 0,
        shipped: 0,
        samples: Vec::new(),
    };

    let claims = Claims::of(&document);
    let pages = Pages::new(&document);
    // One parent tree per content stream, read once: `ParentTree::for_page` is a number-tree walk
    // and a form drawn on forty pages would otherwise pay it forty times.
    let mut trees: BTreeMap<Option<ObjectId>, ParentTree> = BTreeMap::new();
    for index in 0..pages.len() {
        let Some(page) = pages.get(index) else {
            continue;
        };
        let interpretation = interpret(&document, &page);
        trees
            .entry(None)
            .or_insert_with(|| ParentTree::for_page(&document, &page.dict));
        let classes = classify(&document, &interpretation, &mut trees, &claims, page.id);
        let states_struct_parents = document
            .get_key(&page.dict, "StructParents")
            .as_integer()
            .is_some();
        found.pages += 1;
        found.characters += classes.total;
        found.reached += classes.reached;
        found.tagged_artifact += classes.tagged_artifact;
        found.reached_by_k += classes.reached_by_k;
        found.shipped += interpretation
            .artifacts
            .iter()
            .filter(|span| span.found == ArtifactSource::Absence)
            .map(|span| {
                interpretation
                    .text
                    .get(span.range.clone())
                    .map_or(0, |run| run.chars().count())
            })
            .sum::<usize>();
        found.unresolved_identifier += classes.unresolved;
        found.unmarked += classes.unmarked;
        let unreached = classes.unresolved + classes.unmarked;
        if !states_struct_parents {
            found.unreached_without_struct_parents += unreached;
        }
        if unreached > 0 {
            found.pages_with_unreached += 1;
            #[expect(
                clippy::cast_precision_loss,
                reason = "a percentage printed to one decimal place"
            )]
            let share = if classes.total == 0 {
                0.0
            } else {
                unreached as f64 * 100.0 / classes.total as f64
            };
            if found.worst_page.is_none_or(|(_, worst)| share > worst) {
                found.worst_page = Some((index + 1, share));
            }
            for run in classes.samples {
                if found.samples.len() >= SAMPLES {
                    break;
                }
                found.samples.push(format!("p{}: {run}", index + 1));
            }
        }
        // The page's own tree is the only per-page entry; a stream's is keyed by the object it is.
        trees.remove(&None);
    }
    Some(found)
}

/// One page's readback, split four ways in characters.
#[derive(Default)]
struct Classes {
    /// Characters of readback.
    total: usize,
    /// Inside a sequence whose `/MCID` the parent tree resolves.
    reached: usize,
    /// Inside a `/Artifact` marked-content sequence.
    tagged_artifact: usize,
    /// Inside a sequence the parent tree misses but some element's `/K` names.
    reached_by_k: usize,
    /// Inside a sequence whose `/MCID` resolves to nothing.
    unresolved: usize,
    /// Inside no marked-content sequence at all.
    unmarked: usize,
    /// What the unreached runs on this page say, squeezed and bounded.
    samples: Vec<String>,
}

/// How many unreached runs a document prints, and how long each is printed.
///
/// A count with no sample is a sentence about the instrument (`doc/traps/instruments-and-reports.md`
/// trap 11): the whole question this census answers is whether a page's unreached content is a
/// running head or the body of the page, and only the text says which.
const SAMPLES: usize = 4;
/// Characters of each sampled run.
const SAMPLE_WIDTH: usize = 72;

/// Splits one page's readback by what claims each character.
///
/// The order the classes are tried in is the clause's own: a character an element reaches is real
/// content whatever else encloses it (§14.8.2.2.1), a character inside an `/Artifact` sequence is
/// the artifact its producer declared (§14.8.2.2.2's two marked-content forms), and what is left is
/// the population the clause's "any content" sentence reclassifies.
fn classify(
    document: &Document,
    interpretation: &Interpretation,
    trees: &mut BTreeMap<Option<ObjectId>, ParentTree>,
    claims: &Claims,
    page: Option<ObjectId>,
) -> Classes {
    let mut classes = Classes::default();
    // One byte-indexed verdict per character start, taken from the spans that cover it.
    let text = &interpretation.text;
    let mut reached = vec![false; text.len()];
    let mut by_k = vec![false; text.len()];
    let mut marked = vec![false; text.len()];
    for span in &interpretation.marked {
        let named = match span.stream {
            ContentStream::Page => None,
            // A stream stating no `/StructParents` of its own keeps the enclosing tree, which is
            // what `Interpreter::enter_stream_structure` does and what §14.7.5.2's first method
            // means: the whole form is part of the page's sequence.
            ContentStream::Object(id) => match document.get(id).as_stream() {
                Some(stream)
                    if document
                        .get_key(&stream.dict, "StructParents")
                        .as_integer()
                        .is_some_and(|key| key >= 0) =>
                {
                    Some(id)
                }
                _ => None,
            },
            // Table 357 requires `/Stm` to be an indirect reference, so nothing can claim a
            // sequence in a stream nothing can name.
            ContentStream::Unnameable => {
                mark(&mut marked, &span.range);
                continue;
            }
        };
        let tree = trees.entry(named).or_insert_with(|| {
            named
                .and_then(|id| {
                    document
                        .get(id)
                        .as_stream()
                        .map(|stream| stream.dict.clone())
                })
                .map_or_else(ParentTree::default, |dict| {
                    ParentTree::for_page(document, &dict)
                })
        });
        mark(&mut marked, &span.range);
        if tree.element(document, span.mcid).is_some() {
            mark(&mut reached, &span.range);
        } else if claims.names(page, named, span.mcid) {
            mark(&mut by_k, &span.range);
        }
    }
    // [`ArtifactSource::Declared`] alone: the spans this program adds by absence are the *answer*
    // this census exists to check, and counting them here would make the instrument agree with
    // whatever the code under it did (`doc/traps/parsers-and-streams.md` trap 8).
    let mut artifact = vec![false; text.len()];
    for span in &interpretation.artifacts {
        if span.found == ArtifactSource::Declared {
            mark(&mut artifact, &span.range);
        }
    }

    let mut run = String::new();
    for (at, character) in text.char_indices() {
        classes.total += 1;
        let unreached = if reached.get(at).copied().unwrap_or(false) {
            classes.reached += 1;
            false
        } else if by_k.get(at).copied().unwrap_or(false) {
            classes.reached_by_k += 1;
            false
        } else if artifact.get(at).copied().unwrap_or(false) {
            classes.tagged_artifact += 1;
            false
        } else if marked.get(at).copied().unwrap_or(false) {
            classes.unresolved += 1;
            true
        } else {
            classes.unmarked += 1;
            true
        };
        if unreached {
            if run.chars().count() < SAMPLE_WIDTH {
                run.push(if character.is_control() {
                    ' '
                } else {
                    character
                });
            }
        } else {
            close_run(&mut run, &mut classes.samples);
        }
    }
    close_run(&mut run, &mut classes.samples);
    classes
}

/// Ends the run being accumulated, keeping it if it says anything.
fn close_run(run: &mut String, samples: &mut Vec<String>) {
    let text = run.trim().to_owned();
    run.clear();
    if !text.is_empty() && samples.len() < SAMPLES {
        samples.push(text);
    }
}

/// What the structure tree's own `/K` says it holds, walked once per document.
///
/// §14.7.5.4's parent tree is an *index*: the tree's `/K` entries are the statement, and the index
/// is what a content stream can reach backwards through. A file may write one and not the other, so
/// a census that decided inclusion from the index alone would report a producer's broken index as
/// this reader's artifact — which is the whole question this instrument exists to separate.
struct Claims {
    /// Every `(page, stream, identifier)` some element's `/K` names.
    marked: BTreeSet<(Option<ObjectId>, Option<ObjectId>, i64)>,
}

impl Claims {
    /// Walks the whole tree once.
    fn of(document: &Document) -> Self {
        let mut marked = BTreeSet::new();
        if let Some(tree) = Tree::of(document) {
            for (_, child) in &tree.walk(document).items {
                if let Child::MarkedContent {
                    mcid, page, stream, ..
                } = child
                {
                    marked.insert((*page, *stream, *mcid));
                }
            }
        }
        Self { marked }
    }

    /// Whether some element's `/K` names this sequence.
    ///
    /// A `/K` entry stating no `/Pg` names a sequence on whatever page its element belongs to, so
    /// it is accepted for any page: this is the *generous* side of the comparison on purpose, and
    /// a generous claim that still leaves content unreached is the strongest form of the answer.
    fn names(&self, page: Option<ObjectId>, stream: Option<ObjectId>, mcid: i64) -> bool {
        self.marked.contains(&(page, stream, mcid)) || self.marked.contains(&(None, stream, mcid))
    }
}

/// Sets every byte of one range.
fn mark(flags: &mut [bool], range: &std::ops::Range<usize>) {
    let end = range.end.min(flags.len());
    if let Some(slice) = flags.get_mut(range.start.min(end)..end) {
        slice.fill(true);
    }
}
