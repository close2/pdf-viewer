//! Every corpus document the suite can open, edited through the core's five write verbs.
//!
//! `doc/todo/58` §5 has owed this one since the write side landed: "[e]very read generator is
//! measured against every corpus document by nothing, and so is every write" — `tests/a_write.rs`
//! drives all five verbs over four committed documents and one corpus document, and the transform
//! suite's own six walks do not reach `Plan::Update` at all. This is the walk RFC 0002 section 9's
//! shape asks for, over RFC 0003 section 5.2's verbs and through the **core** rather than through
//! the transform library, because what a face does to a document is `Vfs::write` and
//! `Vfs::remove`, not `pdf_transform::apply`.
//!
//! For every corpus document the core opens, on a fresh in-memory backing per verb:
//!
//! 1. **insert** — a one-page document copied to `pages/0001.pdf`, which RFC 0003 section 5.2
//!    says "inserts before the current fourth page (the incumbent 0004 and everything after
//!    shift up on the next listing)";
//! 2. **delete** — `rm pages/0001.pdf`, the incumbent second page becoming the first;
//! 3. **attach** — a file copied into `attachments/`, read back byte for byte;
//! 4. **detach** — the same file removed again, the document's own embedded files still listed;
//! 5. **set information** — `meta/info.json` overwritten, read back, and written back once more.
//!
//! # What each of them is held to
//!
//! - **ISO 32000-2 §7.5.6's prefix property**, on every commit: "changes shall be appended to the
//!   end of the file, leaving its original contents intact", so the file after a write begins
//!   with the file before it, byte for byte. The core checks this itself before it writes
//!   (`Vfs::check_appended`); the walk checks it *after*, against the backing, which is a
//!   different claim — that what reached the file is what the check passed.
//! - **The document re-opens**, and holds the page count the edit stated.
//! - **Every surviving page draws bit-identically to the page it was.** RFC 0002 section 9 calls
//!   this the load-bearing layer, and it is derivable from the specification rather than from any
//!   other tool: the same content stream, the same resources and the same boxes shall mark the
//!   same pixels. An insertion moves every page down by the inserted count and a deletion of the
//!   first moves every page up by one, so the comparison is *between different ordinals* — which
//!   is what makes it a check of RFC 0003 section 5.2's "[o]rdinal names are positions, not
//!   identities" and not only of the writer.
//! - **§14.7.5.4's `/StructParents`**, as `pdf_transform::update` states its fate: stripped from
//!   every carried page, because "the key is an index into *this* document's parent tree and a
//!   carried page's key would name somebody else's elements" — and *kept*, unchanged, on every
//!   page of the document being edited.
//! - **Idempotence where it applies.** `meta/info.json` read and written straight back changes
//!   nothing it states, which is ADR 0855 section 5's argument for answering RFC 0003 section 9's fourth
//!   open question *yes*; attach followed by detach returns the listing to the set the document
//!   itself files. And RFC 0002 section 9's first layer beside them: the same insertion computed
//!   twice writes the same bytes — **except where ISO 32000-2 §7.6.3.1 forbids it**, since a
//!   fresh random initialisation vector in front of every AES string makes an encrypted
//!   document's update differ from one save to the next by construction. What still binds there
//!   is the length, which a vector does not change.
//!
//! # What is a failure and what is held
//!
//! The assertions bind from the first run. **A refusal is not a failure**: a document
//! `pdf_transform::update` declines by name — a cross-reference table rebuilt by scanning has no
//! offset to chain to, a page carries a §12.7 widget whose field is not coming with it — is *the
//! document's*, counted by reason and printed (trap 11). What the walk cannot explain goes in
//! [`HELD`] with a diagnosis, in the oracle's style, and an undiagnosed difference fails the run.
//!
//! Everything is in memory: the corpus is never written to, and [`MemoryBacking`] is the file. The
//! reader and the rasteriser that judge the writer are this tree's own, which is trap 8 and is
//! stated as such; `pdf-transform`'s `foreign_corpus` walk is where another program reads what
//! these writers produce.
//!
//! # The cost floor
//!
//! Each verb's mount carries [`Cost`], which holds ADR 0894's inequality on the write path:
//! `Vfs::questions().repeated <= Vfs::forgotten()`, per mount. It is a count rather than a clock,
//! for the reason `tests/read_corpus.rs` states at length, and it is on this walk as well because
//! this is the one that exercises the reads a *write* performs before it commits — a validation
//! that re-runs a generator is invisible to every counter of what was produced (trap 33, ADRs
//! 0886 and 0894).
//!
//! # Running it
//!
//! ```text
//! tools/bounded.sh --data 12 --tree 12 -- cargo test --profile gates -p pdf-vfs --test write_corpus -- --ignored --nocapture
//! ```

#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::print_stdout,
    reason = "test code: an explanatory panic is the intended failure, and the census output \
              is the point of the run"
)]

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use pdf_syntax::{Document, FileBytes, Limits};
use pdf_transform::range::Selection;
use pdf_transform::render::{ImageFormat, RenderPlan, Sizing};
use pdf_transform::split::{Pieces, SplitPlan};
use pdf_transform::{Budget, MemorySinks, Plan, Policy, Secret, Source, apply};
use pdf_vfs::generation::{Backing, Generation};
use pdf_vfs::worker::{InProcess, Worker, WorkerError, Workers};
use pdf_vfs::{Config, MemoryBacking, Vfs};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};

/// The dots per inch every raster is drawn at.
///
/// `split_corpus.rs`'s figure and its reason: the question is whether two rasters are the same
/// rather than how they look, and a corpus-wide pass at 150 dpi would spend its wall clock on
/// pixels nobody compares.
const DPI: f32 = 48.0;

/// How many pages of one document the walk draws.
///
/// `pages_corpus.rs`'s bound, and this walk pays it five times over — a document is edited five
/// ways and every one of them is drawn beside the source. The first pages exercise the same
/// interpreter as the last.
const DRAWN: usize = 3;

/// The bytes embedded: a sentence no corpus document contains, so that reading them back is a
/// statement about the update and not about the file.
const PAYLOAD: &[u8] = b"pdf-vfs write corpus witness 909\n";

/// The name the embedded file is filed under. ASCII, no colon, no solidus — a name §7.7.4's tree
/// and a directory listing can both hold, which `Vfs::write` refuses anything else for.
const ATTACHMENT: &str = "pdf-vfs-witness-909.txt";

/// What `meta/info.json` is overwritten with.
///
/// Two of §14.3.3's Table 349 entries stated and the other seven omitted, because "[t]he file is
/// the whole of Table 349 and nothing else": a key the file omits is an entry the document shall
/// no longer state. So the readback is checkable in both directions from one write.
const INFORMATION: &[u8] =
    br#"{"title": "round 909 wrote this", "author": "the write-side corpus walk"}"#;

/// The corpus documents that refuse §7.6.4.1's default user password, with the password each
/// one's own pdf.js issue records — `split_corpus.rs`'s list, so that the population is every
/// document the suite can open rather than every document that opens for free.
///
/// `Vfs` itself has no way to be given one (`doc/todo/58` §5 records the shortfall and the
/// `SecretSource` a face would need), so the walk supplies it through [`KeyedWorkers`] — which is
/// what that design would do, at the one seam that already carries a `Secret`.
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

/// Documents whose edit the walk cannot explain, each with its diagnosis.
///
/// Empty is the state to keep. An entry here is a *reading* of why the difference is the
/// document's rather than the core's, and the walk fails on any difference it does not name.
const HELD: &[(&str, &str)] = &[];

/// What the walk found.
#[derive(Default)]
struct Tally {
    /// Documents the core could not open, by reason.
    refused_open: Vec<(String, String)>,
    /// Documents with no page at all, which four of the five verbs still apply to.
    pageless: usize,
    /// Per verb: the documents that verb refused by name, by reason.
    refused: [Vec<(String, String)>; 5],
    /// Per verb: the documents that verb committed.
    committed: [usize; 5],
    /// §7.5.6's prefix property did not hold of the file after a commit.
    prefix_failed: Vec<(String, String)>,
    /// The document did not re-open after a commit, or did not hold the pages the edit stated.
    reread_failed: Vec<(String, String)>,
    /// The page count or the renumbered listing is not what the layout says.
    renumbering_failed: Vec<(String, String)>,
    /// Pages that drew bit-identically to the page they had been.
    identical: usize,
    /// Pages that drew differently.
    differ: Vec<(String, String)>,
    /// Pages neither side would draw, so nothing was compared.
    undrawn: usize,
    /// §14.7.5.4's `/StructParents` is not what `update` says it is.
    structure_failed: Vec<(String, String)>,
    /// Insertions whose carried page had a `/StructParents` for the update to strip.
    structure_stripped: usize,
    /// The embedded file did not read back byte for byte, or the listing was not restored.
    attachment_failed: Vec<(String, String)>,
    /// `meta/info.json` did not read back what was written.
    information_failed: Vec<(String, String)>,
    /// Reading `meta/info.json` and writing it straight back changed what it states.
    not_idempotent: Vec<(String, String)>,
    /// The same insertion computed twice wrote different bytes, in a document §7.6.3.1 says
    /// cannot be written twice the same way.
    encrypted_updates_differ: usize,
    /// The same insertion computed twice wrote the same bytes into an *encrypted* document,
    /// which is what a crypt filter without an initialisation vector does.
    encrypted_updates_agree: usize,
    /// The same insertion computed twice wrote different bytes.
    nondeterministic: Vec<(String, String)>,
    /// A document whose examination panicked, which principle 1 forbids.
    panicked: Vec<(String, String)>,
    /// Questions the verb mounts put to their workers.
    asked: u64,
    /// Questions put about a subject the generation's worker had already answered.
    repeated: u64,
    /// Generated outputs those mounts' caches stopped holding, which is what a repeat may be
    /// explained by.
    forgotten: u64,
    /// A verb mount whose repeats outnumber what its cache forgot.
    unexplained: Vec<(String, String)>,
}

/// The five verbs, in the order [`Tally::refused`] and [`Tally::committed`] count them.
const VERBS: [&str; 5] = ["insert", "delete", "attach", "detach", "set information"];

/// Adds to the shared tally, ignoring a poisoned lock (another document's panic is already being
/// reported; losing one entry to it changes nothing).
fn record(tally: &Mutex<Tally>, update: impl FnOnce(&mut Tally)) {
    if let Ok(mut tally) = tally.lock() {
        update(&mut tally);
    }
}

/// Fails the gate if this build cannot reach the sandboxed image decoder.
///
/// `CCITTFaxDecode`, `JBIG2Decode` and `JPXDecode` are decoded by a separate program, and Cargo
/// does not build another package's binaries when it tests this one (trap 10). Both sides of
/// every comparison here are drawn by the same build, so a missing worker would not make them
/// *disagree* — it would make them agree on pages with the images missing, which is a weaker gate
/// wearing the same number.
fn require_the_sandbox() {
    if let Err(error) = pdf_model::image::sandboxed_decoder() {
        panic!(
            "the sandboxed image decoder is not available, so both sides of every comparison \
             would be drawn without CCITT, JBIG2 or JPEG 2000 images: {error}"
        );
    }
}

/// The corpus files, or `None` when the submodule is not checked out.
fn corpus() -> Option<Vec<PathBuf>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/pdf.js/test/pdfs");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&root)
        .ok()?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|extension| extension == "pdf"))
        .collect();
    if files.is_empty() {
        return None;
    }
    files.sort();
    Some(files)
}

/// The password the corpus records for this document, or the empty one.
fn password_for(name: &str) -> &'static str {
    KNOWN_PASSWORDS
        .iter()
        .find(|(known, _)| *known == name)
        .map_or("", |(_, password)| password)
}

/// A backing the walk and the tree both hold, so that "what reached the file" is a thing the walk
/// can read rather than a thing it infers from the core's own answer.
#[derive(Debug)]
struct SharedBacking(Arc<MemoryBacking>);

impl Backing for SharedBacking {
    fn generation(&self) -> std::io::Result<Generation> {
        self.0.generation()
    }
    fn bytes(&self) -> std::io::Result<FileBytes> {
        self.0.bytes()
    }
    fn describe(&self) -> String {
        self.0.describe()
    }
    fn commit(&self, bytes: &[u8]) -> std::io::Result<()> {
        self.0.commit(bytes)
    }
}

/// Workers that know §7.6.4.1's password for this document.
///
/// `Vfs` passes `None` at the one place it spawns a worker, which is `doc/todo/58` §5's recorded
/// shortfall — a mount that survives a change of the file needs a `SecretSource` a face
/// implements. This is that source, at the seam where it would go, so that eight documents of the
/// corpus are in this walk's population rather than in its refusal list.
#[derive(Debug)]
struct KeyedWorkers(&'static str);

impl Workers for KeyedWorkers {
    fn spawn(
        &self,
        bytes: FileBytes,
        password: Option<Secret>,
        policy: Policy,
        budget: Budget,
    ) -> Result<Box<dyn Worker>, WorkerError> {
        let secret =
            password.or_else(|| (!self.0.is_empty()).then(|| Secret::from(self.0.to_owned())));
        let source = match secret {
            Some(secret) => Source::with_password(bytes, secret),
            None => Source::new(bytes),
        };
        // One strip: this walk runs a rayon thread per document already, and a worker that split
        // a render across the pool inside one of them would be measuring the scheduler.
        Ok(Box::new(InProcess::new(source, policy, budget, Some(1))))
    }
}

/// What one verb's mount asked its workers, recorded wherever that verb returns from.
///
/// **The read walk's cost floor, on the write side** (ADR 0894): a generator is run once per
/// subject per generation, and the only thing that may make it run again is the cache having
/// stopped holding what it produced. It is a count rather than a clock, so a neighbouring round's
/// load cannot move it.
///
/// A guard with a `Drop` rather than a line at the end of each verb, and the reason is the
/// shape of this file: a verb returns from a dozen places — every refusal, every fault, every
/// early diagnosis — and a measurement taken at the end of the function would silently be a
/// measurement of the documents that got that far. That is trap 25 in miniature, and the borrow
/// checker's drop order does the work: the guard is declared after the tree it measures, so it
/// runs first.
struct Cost<'a> {
    /// The mount being measured.
    vfs: &'a Vfs,
    /// Where the counts go.
    tally: &'a Mutex<Tally>,
    /// The document, and which verb's mount this is.
    what: String,
}

impl Drop for Cost<'_> {
    fn drop(&mut self) {
        let questions = self.vfs.questions();
        let forgotten = self.vfs.forgotten();
        let twice = self.vfs.asked_twice();
        let what = self.what.clone();
        record(self.tally, |t| {
            t.asked = t.asked.saturating_add(questions.asked);
            t.repeated = t.repeated.saturating_add(questions.repeated);
            t.forgotten = t.forgotten.saturating_add(forgotten);
            if questions.repeated > forgotten {
                t.unexplained.push((
                    what,
                    format!(
                        "{} of {} questions were about a subject already answered, the cache \
                         forgot {}, and the generators run twice were {twice:?}",
                        questions.repeated, questions.asked, forgotten
                    ),
                ));
            }
        });
    }
}

/// A tree over a document held in memory, and the backing beside it.
fn mounted(name: &str, bytes: &[u8]) -> (Arc<MemoryBacking>, Vfs) {
    let backing = Arc::new(MemoryBacking::new(name, bytes.to_vec()));
    let vfs = Vfs::new(
        Box::new(SharedBacking(Arc::clone(&backing))),
        Box::new(KeyedWorkers(password_for(name))),
        Config::default(),
    );
    (backing, vfs)
}

/// The whole document, as it is on the backing right now.
fn on_disk(backing: &MemoryBacking) -> Vec<u8> {
    let bytes = backing.bytes().expect("a memory backing reads");
    bytes.read(0..bytes.len()).into_owned()
}

/// ISO 32000-2 §7.5.6, read off the file: the update begins with what was there.
fn is_appended_to(now: &[u8], before: &[u8]) -> bool {
    now.len() >= before.len() && now.get(..before.len()) == Some(before)
}

/// The budget both sides are drawn under, so that a page refused for size is refused twice.
fn budget() -> Budget {
    Budget {
        limits: Limits::DEFAULT,
        // `split_corpus.rs`'s ceiling and its reason: a corpus-wide pass holds several rasters at
        // once across rayon, and a page past it is refused on both sides.
        max_pixels: 1 << 24,
    }
}

/// A transform source over these bytes, with the corpus's known password where the file has one.
fn source(name: &str, bytes: &[u8]) -> Source {
    let password = password_for(name);
    if password.is_empty() {
        Source::new(bytes.to_vec())
    } else {
        Source::with_password(bytes.to_vec(), Secret::from(password.to_owned()))
    }
}

/// Draws one page of these bytes as a PPM, or `None` where nothing was drawn.
fn draw(name: &str, bytes: &[u8], page: usize) -> Option<Vec<u8>> {
    let sinks = MemorySinks::new();
    apply(
        &Plan::Render(RenderPlan {
            source: 0,
            pages: page.to_string().parse::<Selection>().expect("a selection"),
            size: Sizing::Dpi(DPI),
            format: ImageFormat::Ppm,
            page_box: None,
            annotations: true,
            names: "page.ppm".parse().expect("a pattern"),
            strips: Some(1),
        }),
        &[source(name, bytes)],
        &sinks,
        &Policy::default(),
        &budget(),
    )
    .ok()?;
    let mut outputs = sinks.into_outputs();
    (!outputs.is_empty()).then(|| outputs.remove(0).1)
}

/// How a `pages/` or `text/` listing spells one ordinal in a document of this many pages.
///
/// RFC 0003 section 4: "zero-padded ordinal; width from page count", with four the floor. The
/// walk spells the name itself rather than taking it from a listing because a document with no
/// page has no listing to take it from, and inserting into one is a position the core admits.
fn page_stem(pages: usize, ordinal: usize) -> String {
    let width = pages.to_string().len().max(4);
    format!("{ordinal:0width$}")
}

/// §14.7.5.4's key on one page, as the document states it.
fn struct_parents(document: &Document, index: usize) -> Option<i64> {
    let pages = pdf_model::Pages::new(document);
    let page = pages.get(index)?;
    document.get_key(&page.dict, "StructParents").as_integer()
}

/// Every name in a listing, sorted, or the error as a sentence.
fn names(vfs: &Vfs, path: &str) -> Result<Vec<String>, String> {
    let mut out: Vec<String> = vfs
        .list(path)
        .map_err(|error| format!("{path}: {error}"))?
        .into_iter()
        .map(|entry| entry.name)
        .collect();
    out.sort();
    Ok(out)
}

/// One virtual file's bytes, or the error as a sentence.
fn read(vfs: &Vfs, path: &str) -> Result<Vec<u8>, String> {
    Ok(vfs
        .open(path)
        .map_err(|error| format!("{path}: {error}"))?
        .bytes()
        .to_vec())
}

/// The one-page document every insertion carries in, and its raster.
///
/// Derived rather than committed: page 1 of `doc/PDF20_AN001-BPC.pdf` taken out by the transform
/// suite's own `split`, which is what `pages/0001.pdf` *is* in this tree. It is a tagged page, so
/// it carries §14.7.5.4's `/StructParents` and the insertion has something to strip.
struct Inserted {
    /// The single-page document.
    bytes: Vec<u8>,
    /// Its page 1, drawn alone at [`DPI`].
    raster: Option<Vec<u8>>,
    /// Whether its page states §14.7.5.4's key, so that stripping it is a thing to check for.
    has_struct_parents: bool,
}

impl Inserted {
    /// Builds it, or explains why it could not be built.
    fn build() -> Result<Self, String> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/PDF20_AN001-BPC.pdf");
        let whole = std::fs::read(&path).map_err(|why| format!("{}: {why}", path.display()))?;
        let sinks = MemorySinks::new();
        apply(
            &Plan::Split(SplitPlan {
                source: 0,
                pages: "1".parse::<Selection>().expect("a selection"),
                pieces: Pieces::EachPage,
                names: "piece.pdf".parse().expect("a pattern"),
            }),
            &[Source::new(whole)],
            &sinks,
            &Policy::default(),
            &budget(),
        )
        .map_err(|why| format!("page 1 could not be split out: {why}"))?;
        let mut outputs = sinks.into_outputs();
        if outputs.is_empty() {
            return Err(String::from("split wrote no piece"));
        }
        let bytes = outputs.remove(0).1;
        let document = Document::open_with_limits(bytes.clone(), Limits::DEFAULT)
            .map_err(|why| format!("the piece does not open: {why}"))?;
        if pdf_model::Pages::new(&document).len() != 1 {
            return Err(String::from("the piece is not one page"));
        }
        Ok(Self {
            raster: draw("piece.pdf", &bytes, 1),
            has_struct_parents: struct_parents(&document, 0).is_some(),
            bytes,
        })
    }
}

/// Which of the tally's three commit columns a fault belongs in.
///
/// An enum rather than the string this was first written with: a column named by text is one a
/// typo puts in the wrong list, silently, and a walk whose census lies is worth less than no
/// walk (trap 11 is about the condition a report fires on; this is the same rule about where it
/// lands).
#[derive(Debug, Clone, Copy)]
enum Column {
    /// §7.5.6's prefix property did not hold of the file.
    Prefix,
    /// The updated document did not open.
    Reread,
    /// The page count or the listing is not what the edit said.
    Renumbering,
}

/// One commit's faults, each under the column of the tally it belongs to.
type Faults = Vec<(Column, String)>;

/// What one verb's commit is held to, whatever the verb was.
///
/// The three claims every write of this core makes: §7.5.6's prefix property against the file,
/// the document re-opening, and the page count the edit stated. `Err` is the sentence to record;
/// `Ok` carries the re-read document and the bytes it was read from.
fn commit_holds(
    backing: &MemoryBacking,
    before: &[u8],
    name: &str,
    expected_pages: usize,
) -> Result<(Vec<u8>, Document), Faults> {
    let after = on_disk(backing);
    let mut faults: Faults = Vec::new();
    if !is_appended_to(&after, before) {
        faults.push((
            Column::Prefix,
            format!(
                "{} bytes became {} and the first {} are not the document's",
                before.len(),
                after.len(),
                before.len()
            ),
        ));
    }
    let read =
        match Document::open_with_password(after.clone(), Limits::DEFAULT, password_for(name)) {
            Ok(read) => read,
            Err(error) => {
                faults.push((
                    Column::Reread,
                    format!("the updated document does not open: {error}"),
                ));
                return Err(faults);
            }
        };
    let pages = pdf_model::Pages::new(&read).len();
    if pages != expected_pages {
        faults.push((
            Column::Renumbering,
            format!("the edit said {expected_pages} pages and the document holds {pages}"),
        ));
    }
    if faults.is_empty() {
        Ok((after, read))
    } else {
        Err(faults)
    }
}

/// Files one commit's faults into the tally under the columns they belong to.
fn record_faults(tally: &Mutex<Tally>, name: &str, verb: &str, faults: Faults) {
    record(tally, |t| {
        for (column, detail) in faults {
            let entry = (name.to_owned(), format!("{verb}: {detail}"));
            match column {
                Column::Prefix => t.prefix_failed.push(entry),
                Column::Reread => t.reread_failed.push(entry),
                Column::Renumbering => t.renumbering_failed.push(entry),
            }
        }
    });
}

/// Compares one page of an edited document with the page of the source it used to be.
fn compare_page(
    name: &str,
    verb: &str,
    before: Option<&Vec<u8>>,
    after: Option<&Vec<u8>>,
    at: usize,
    was: usize,
    tally: &Mutex<Tally>,
) {
    match (before, after) {
        (Some(before), Some(after)) if before == after => {
            record(tally, |t| t.identical = t.identical.saturating_add(1));
        }
        (Some(before), Some(after)) => {
            record(tally, |t| {
                t.differ.push((
                    name.to_owned(),
                    format!(
                        "{verb}: page {at} was page {was}, and {} bytes of raster became {}{}",
                        before.len(),
                        after.len(),
                        if before.len() == after.len() {
                            ", same size"
                        } else {
                            ""
                        }
                    ),
                ));
            });
        }
        _ => record(tally, |t| t.undrawn = t.undrawn.saturating_add(1)),
    }
}

/// One document through the walk.
fn examine(path: &Path, insert: &Inserted, tally: &Mutex<Tally>) {
    let name = path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    );
    let Ok(bytes) = std::fs::read(path) else {
        return;
    };

    // The core is what opens it, because the core is what a face has. A document it cannot open
    // has no tree and no write; the reason is the tree's own.
    let (_backing, probe) = mounted(&name, &bytes);
    let pages = match probe.pages() {
        Ok(pages) => pages,
        Err(error) => {
            record(tally, |t| t.refused_open.push((name, error.to_string())));
            return;
        }
    };
    drop(probe);
    if pages == 0 {
        record(tally, |t| t.pageless = t.pageless.saturating_add(1));
    }

    // Drawn once and compared against five times.
    let source_rasters: Vec<Option<Vec<u8>>> = (1..=pages.min(DRAWN))
        .map(|page| draw(&name, &bytes, page))
        .collect();
    let read = Document::open_with_password(bytes.clone(), Limits::DEFAULT, password_for(&name));
    let structure: Vec<Option<i64>> = match &read {
        Ok(document) => (0..pages.min(DRAWN))
            .map(|index| struct_parents(document, index))
            .collect(),
        Err(_) => Vec::new(),
    };
    // §7.6.3.1's random initialisation vector is why this is asked, and `pdf_syntax::write`
    // already states the consequence: "an encrypted document's update differs from one save to
    // the next by construction".
    let encrypted = read.as_ref().is_ok_and(Document::is_encrypted);
    drop(read);

    insert_verb(
        &name,
        &bytes,
        pages,
        &source_rasters,
        &structure,
        insert,
        encrypted,
        tally,
    );
    delete_verb(&name, &bytes, pages, &source_rasters, tally);
    attachment_verbs(&name, &bytes, pages, &source_rasters, tally);
    information_verb(&name, &bytes, pages, &source_rasters, tally);
}

/// RFC 0003 section 5.2's first row: another document's pages inserted at the name's position.
#[expect(
    clippy::too_many_lines,
    reason = "one verb, one document, and every claim the round makes about it in the order the \
              claims are made; splitting it would put the source's rasters and its §14.7.5.4 keys \
              behind another argument list without making either half readable on its own"
)]
#[expect(
    clippy::too_many_arguments,
    reason = "one document's whole insertion: the source's rasters, its §14.7.5.4 keys, the \
              document going in and whether §7.6.3.1's initialisation vector is in play are all \
              things this verb is judged against, and computing any of them twice would double \
              the walk's most expensive call"
)]
fn insert_verb(
    name: &str,
    bytes: &[u8],
    pages: usize,
    source_rasters: &[Option<Vec<u8>>],
    structure: &[Option<i64>],
    insert: &Inserted,
    encrypted: bool,
    tally: &Mutex<Tally>,
) {
    let (backing, vfs) = mounted(name, bytes);
    let _cost = Cost {
        vfs: &vfs,
        tally,
        what: format!("{name} (insert)"),
    };
    let path = format!("/pages/{}.pdf", page_stem(pages, 1));
    let committed = match vfs.write(&path, &insert.bytes) {
        Ok(committed) => committed,
        Err(error) => {
            record(tally, |t| {
                t.refused[0].push((name.to_owned(), error.to_string()));
            });
            return;
        }
    };
    record(tally, |t| t.committed[0] = t.committed[0].saturating_add(1));

    let expected = pages.saturating_add(1);
    let read = match commit_holds(&backing, bytes, name, expected) {
        Ok((_, read)) => read,
        Err(faults) => {
            record_faults(tally, name, "insert", faults);
            return;
        }
    };
    if committed.pages != expected {
        record(tally, |t| {
            t.renumbering_failed.push((
                name.to_owned(),
                format!("insert: the commit reported {} pages", committed.pages),
            ));
        });
    }
    match names(&vfs, "/pages") {
        Ok(listed) if listed.len() == expected => {}
        Ok(listed) => record(tally, |t| {
            t.renumbering_failed.push((
                name.to_owned(),
                format!(
                    "insert: pages/ lists {} names, not {expected}",
                    listed.len()
                ),
            ));
        }),
        Err(why) => record(tally, |t| {
            t.renumbering_failed
                .push((name.to_owned(), format!("insert: {why}")));
        }),
    }

    // §14.7.5.4, as `pdf_transform::update` states its fate: gone from the carried page, and
    // untouched on every page that was already here.
    if insert.has_struct_parents {
        record(tally, |t| {
            t.structure_stripped = t.structure_stripped.saturating_add(1);
        });
        if struct_parents(&read, 0).is_some() {
            record(tally, |t| {
                t.structure_failed.push((
                    name.to_owned(),
                    String::from("insert: the carried page kept §14.7.5.4's /StructParents"),
                ));
            });
        }
        if !committed
            .warnings
            .iter()
            .any(|detail| detail.contains("StructParents"))
        {
            record(tally, |t| {
                t.structure_failed.push((
                    name.to_owned(),
                    String::from("insert: a key was stripped and nothing said so"),
                ));
            });
        }
    }
    for (index, was) in structure.iter().enumerate() {
        let now = struct_parents(&read, index.saturating_add(1));
        if now != *was {
            record(tally, |t| {
                t.structure_failed.push((
                    name.to_owned(),
                    format!(
                        "insert: page {}'s /StructParents was {was:?} and is {now:?}",
                        index.saturating_add(1)
                    ),
                ));
            });
        }
    }

    // Layer 3. The carried page is page 1 and everything the document had is one further down.
    let after = on_disk(&backing);
    compare_page(
        name,
        "insert",
        insert.raster.as_ref(),
        draw(name, &after, 1).as_ref(),
        1,
        0,
        tally,
    );
    for (index, before) in source_rasters.iter().enumerate() {
        let at = index.saturating_add(2);
        compare_page(
            name,
            "insert",
            before.as_ref(),
            draw(name, &after, at).as_ref(),
            at,
            index.saturating_add(1),
            tally,
        );
    }

    // RFC 0002 section 9's first layer, asserted on the verb that allocates object numbers —
    // **except where the clause forbids it**. ISO 32000-2 §7.6.3.1 requires a fresh random
    // initialisation vector in front of every AES string and stream, so an encrypted document's
    // update differs from one save to the next by construction and `pdf_syntax::write::identify`
    // already says so. What is still checkable there is the *length*: the same plaintext under
    // the same crypt filter is the same number of bytes whatever the vector is, so a difference
    // in length is a difference this clause does not explain.
    let (second, twice) = mounted(name, bytes);
    let _cost = Cost {
        vfs: &twice,
        tally,
        what: format!("{name} (insert, computed twice)"),
    };
    if twice.write(&path, &insert.bytes).is_ok() {
        let again = on_disk(&second);
        if again == after {
            if encrypted {
                record(tally, |t| {
                    t.encrypted_updates_agree = t.encrypted_updates_agree.saturating_add(1);
                });
            }
        } else if encrypted && again.len() == after.len() {
            record(tally, |t| {
                t.encrypted_updates_differ = t.encrypted_updates_differ.saturating_add(1);
            });
        } else {
            record(tally, |t| {
                t.nondeterministic.push((
                    name.to_owned(),
                    format!(
                        "two insertions, two files of {} and {} bytes{}",
                        after.len(),
                        again.len(),
                        if encrypted {
                            " — encrypted, and §7.6.3.1's vector does not change a length"
                        } else {
                            ""
                        }
                    ),
                ));
            });
        }
    }
}

/// RFC 0003 section 5.2's second row: `rm pages/0001.pdf`, and everything after moves up.
fn delete_verb(
    name: &str,
    bytes: &[u8],
    pages: usize,
    source_rasters: &[Option<Vec<u8>>],
    tally: &Mutex<Tally>,
) {
    if pages == 0 {
        return;
    }
    let (backing, vfs) = mounted(name, bytes);
    let _cost = Cost {
        vfs: &vfs,
        tally,
        what: format!("{name} (delete)"),
    };
    let path = format!("/pages/{}.pdf", page_stem(pages, 1));
    let committed = match vfs.remove(&path) {
        Ok(committed) => committed,
        Err(error) => {
            record(tally, |t| {
                t.refused[1].push((name.to_owned(), error.to_string()));
            });
            return;
        }
    };
    record(tally, |t| t.committed[1] = t.committed[1].saturating_add(1));

    let expected = pages.saturating_sub(1);
    if let Err(faults) = commit_holds(&backing, bytes, name, expected) {
        record_faults(tally, name, "delete", faults);
        return;
    }
    match names(&vfs, "/pages") {
        Ok(listed) if listed.len() == expected => {}
        Ok(listed) => record(tally, |t| {
            t.renumbering_failed.push((
                name.to_owned(),
                format!(
                    "delete: pages/ lists {} names, not {expected}",
                    listed.len()
                ),
            ));
        }),
        Err(why) => record(tally, |t| {
            t.renumbering_failed
                .push((name.to_owned(), format!("delete: {why}")));
        }),
    }
    // RFC 0003 section 5.3 insists §7.5.6's one consequence be said where a person deletes.
    if !committed
        .warnings
        .iter()
        .any(|detail| detail.contains("§7.5.6"))
    {
        record(tally, |t| {
            t.renumbering_failed.push((
                name.to_owned(),
                String::from("delete: nothing said the page's bytes stay in the file"),
            ));
        });
    }

    let after = on_disk(&backing);
    for (index, before) in source_rasters.iter().enumerate().skip(1) {
        let at = index;
        compare_page(
            name,
            "delete",
            before.as_ref(),
            draw(name, &after, at).as_ref(),
            at,
            index.saturating_add(1),
            tally,
        );
    }
}

/// RFC 0003 section 5.2's third and fourth rows, as one transaction of two commits.
#[expect(
    clippy::too_many_lines,
    reason = "one verb, one document, and every claim the round makes about it in the order the \
              claims are made; splitting it would put the source's rasters and its §14.7.5.4 keys \
              behind another argument list without making either half readable on its own"
)]
fn attachment_verbs(
    name: &str,
    bytes: &[u8],
    pages: usize,
    source_rasters: &[Option<Vec<u8>>],
    tally: &Mutex<Tally>,
) {
    let (backing, vfs) = mounted(name, bytes);
    let _cost = Cost {
        vfs: &vfs,
        tally,
        what: format!("{name} (attach and detach)"),
    };
    let before = match names(&vfs, "/attachments") {
        Ok(before) => before,
        Err(why) => {
            record(tally, |t| {
                t.attachment_failed.push((
                    name.to_owned(),
                    format!("attachments/ does not list: {why}"),
                ));
            });
            return;
        }
    };
    let path = format!("/attachments/{ATTACHMENT}");
    if let Err(error) = vfs.write(&path, PAYLOAD) {
        record(tally, |t| {
            t.refused[2].push((name.to_owned(), error.to_string()));
        });
        return;
    }
    record(tally, |t| t.committed[2] = t.committed[2].saturating_add(1));

    let attached = on_disk(&backing);
    match commit_holds(&backing, bytes, name, pages) {
        Ok(_) => {}
        Err(faults) => {
            record_faults(tally, name, "attach", faults);
            return;
        }
    }
    match (read(&vfs, &path), names(&vfs, "/attachments")) {
        (Ok(back), Ok(listed)) => {
            if back != PAYLOAD {
                record(tally, |t| {
                    t.attachment_failed.push((
                        name.to_owned(),
                        format!(
                            "{} bytes went in and {} came back",
                            PAYLOAD.len(),
                            back.len()
                        ),
                    ));
                });
            }
            if !listed.contains(&ATTACHMENT.to_owned())
                || !before.iter().all(|had| listed.contains(had))
            {
                record(tally, |t| {
                    t.attachment_failed.push((
                        name.to_owned(),
                        format!("attach: {before:?} became {listed:?}"),
                    ));
                });
            }
        }
        (back, listed) => record(tally, |t| {
            t.attachment_failed.push((
                name.to_owned(),
                format!(
                    "attach: read {}, list {}",
                    back.err().unwrap_or_else(|| String::from("ok")),
                    listed.err().unwrap_or_else(|| String::from("ok"))
                ),
            ));
        }),
    }
    // The document's pages are not this verb's business, so page 1 draws as it did.
    if let Some(first) = source_rasters.first() {
        compare_page(
            name,
            "attach",
            first.as_ref(),
            draw(name, &attached, 1).as_ref(),
            1,
            1,
            tally,
        );
    }

    // And out again — a second commit on the same tree, so §7.5.6's property is checked against
    // the document as it was *before either*, which is the claim a chain of updates makes.
    match vfs.remove(&path) {
        Ok(_) => {
            record(tally, |t| t.committed[3] = t.committed[3].saturating_add(1));
        }
        Err(error) => {
            record(tally, |t| {
                t.refused[3].push((name.to_owned(), error.to_string()));
            });
            return;
        }
    }
    let detached = on_disk(&backing);
    if !is_appended_to(&detached, &attached) || !is_appended_to(&detached, bytes) {
        record(tally, |t| {
            t.prefix_failed.push((
                name.to_owned(),
                String::from("detach: the second update is not appended to the first"),
            ));
        });
    }
    match names(&vfs, "/attachments") {
        Ok(listed) if listed == before => {}
        Ok(listed) => record(tally, |t| {
            t.attachment_failed.push((
                name.to_owned(),
                format!("detach: {before:?} did not come back, {listed:?} did"),
            ));
        }),
        Err(why) => record(tally, |t| {
            t.attachment_failed
                .push((name.to_owned(), format!("detach: {why}")));
        }),
    }
    if let Some(first) = source_rasters.first() {
        compare_page(
            name,
            "detach",
            first.as_ref(),
            draw(name, &detached, 1).as_ref(),
            1,
            1,
            tally,
        );
    }
}

/// RFC 0003 section 5.2's fifth row, and ADR 0855 section 5's argument checked over the corpus.
fn information_verb(
    name: &str,
    bytes: &[u8],
    pages: usize,
    source_rasters: &[Option<Vec<u8>>],
    tally: &Mutex<Tally>,
) {
    let (backing, vfs) = mounted(name, bytes);
    let _cost = Cost {
        vfs: &vfs,
        tally,
        what: format!("{name} (set information)"),
    };
    if let Err(error) = vfs.write("/meta/info.json", INFORMATION) {
        record(tally, |t| {
            t.refused[4].push((name.to_owned(), error.to_string()));
        });
        return;
    }
    record(tally, |t| t.committed[4] = t.committed[4].saturating_add(1));

    let after = on_disk(&backing);
    if let Err(faults) = commit_holds(&backing, bytes, name, pages) {
        record_faults(tally, name, "set information", faults);
        return;
    }
    let stated = match read(&vfs, "/meta/info.json") {
        Ok(stated) => String::from_utf8_lossy(&stated).into_owned(),
        Err(why) => {
            record(tally, |t| {
                t.information_failed.push((name.to_owned(), why));
            });
            return;
        }
    };
    // What was written is what the document states, and what was *not* written is what it no
    // longer states — the two halves of "the file is the whole of Table 349".
    for expected in [
        "\"title\": \"round 909 wrote this\"",
        "\"author\": \"the write-side corpus walk\"",
        "\"subject\": null",
        "\"producer\": null",
    ] {
        if !stated.contains(expected) {
            record(tally, |t| {
                t.information_failed.push((
                    name.to_owned(),
                    format!("info.json does not state {expected}: {stated}"),
                ));
            });
            return;
        }
    }
    // ADR 0855 section 5: reading the file and writing it straight back changes nothing.
    match vfs.write("/meta/info.json", stated.as_bytes()) {
        Ok(_) => match read(&vfs, "/meta/info.json") {
            Ok(again) if String::from_utf8_lossy(&again) == stated => {}
            Ok(again) => record(tally, |t| {
                t.not_idempotent.push((
                    name.to_owned(),
                    format!("{stated} became {}", String::from_utf8_lossy(&again)),
                ));
            }),
            Err(why) => record(tally, |t| t.not_idempotent.push((name.to_owned(), why))),
        },
        Err(error) => record(tally, |t| {
            t.not_idempotent.push((
                name.to_owned(),
                format!("the second write refused: {error}"),
            ));
        }),
    }
    if let Some(first) = source_rasters.first() {
        compare_page(
            name,
            "set information",
            first.as_ref(),
            draw(name, &after, 1).as_ref(),
            1,
            1,
            tally,
        );
    }
}

/// Prints one census list, capped, with its length.
fn print_list(what: &str, entries: &[(String, String)]) {
    println!("vfs-write:   {what}: {}", entries.len());
    for (name, why) in entries.iter().take(40) {
        println!("    {name}: {why}");
    }
    if entries.len() > 40 {
        println!("    … and {} more", entries.len().saturating_sub(40));
    }
}

/// The walk.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "the census: every column of the tally printed and then asserted on, in one place, \
              so that what the run says and what fails it cannot drift apart"
)]
#[ignore = "corpus-scale: every document edited five ways through the core, re-read and drawn; run explicitly under the gates profile"]
fn every_corpus_document_survives_the_five_write_verbs() {
    require_the_sandbox();
    let Some(files) = corpus() else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };
    let insert = Inserted::build().unwrap_or_else(|why| {
        panic!("the one-page document every insertion carries could not be built: {why}")
    });
    assert!(
        insert.raster.is_some(),
        "the inserted page must draw, or every insertion's first comparison is vacuous"
    );

    let tally = Mutex::new(Tally::default());
    let started = Instant::now();
    files.par_iter().for_each(|path| {
        let name = path.display().to_string();
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            examine(path, &insert, &tally);
        }));
        if let Err(payload) = outcome {
            let what = payload
                .downcast_ref::<&str>()
                .map(ToString::to_string)
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "non-string panic payload".to_owned());
            record(&tally, |t| t.panicked.push((name, what)));
        }
    });
    let elapsed = started.elapsed();
    let tally = tally.into_inner().expect("reported through the tally");

    println!(
        "vfs-write: {} documents in {:.1}s, {} threads, {DPI} dpi, {DRAWN} pages a document",
        files.len(),
        elapsed.as_secs_f64(),
        rayon::current_num_threads()
    );
    print_list("refused open", &tally.refused_open);
    println!("vfs-write:   documents with no page: {}", tally.pageless);
    for (index, verb) in VERBS.iter().enumerate() {
        println!("vfs-write:   {verb} committed: {}", tally.committed[index]);
        print_list(&format!("{verb} refused by name"), &tally.refused[index]);
    }
    println!(
        "vfs-write:   pages drawn bit-identically to the page they were: {}, neither side drew: {}",
        tally.identical, tally.undrawn
    );
    println!(
        "vfs-write:   insertions whose carried page had §14.7.5.4's /StructParents to strip: {}",
        tally.structure_stripped
    );
    print_list("§7.5.6's prefix property failed", &tally.prefix_failed);
    print_list("the updated document did not re-open", &tally.reread_failed);
    print_list(
        "the renumbering is not the layout's",
        &tally.renumbering_failed,
    );
    print_list("a page drew differently", &tally.differ);
    print_list("§14.7.5.4's /StructParents", &tally.structure_failed);
    print_list("the embedded file", &tally.attachment_failed);
    print_list("§14.3.3's entries", &tally.information_failed);
    print_list("info.json written back changed it", &tally.not_idempotent);
    println!(
        "vfs-write:   encrypted documents whose two insertions differ by §7.6.3.1's \
         initialisation vector: {}, and agree because their crypt filter has none: {}",
        tally.encrypted_updates_differ, tally.encrypted_updates_agree
    );
    print_list("two insertions, two files", &tally.nondeterministic);
    println!(
        "vfs-write:   questions put to the workers: {}, about a subject already answered: {}, \
         outputs the caches forgot: {}",
        tally.asked, tally.repeated, tally.forgotten
    );
    print_list("a repeat the cache cannot explain", &tally.unexplained);
    print_list("panicked", &tally.panicked);

    assert!(
        tally.panicked.is_empty(),
        "principle 1: no panic on any input"
    );
    // The write side of ADR 0894's cost floor, per verb mount: a generator is run once per
    // subject per generation, and only the cache forgetting what it produced may make it run
    // again. It is here as well as in `read_corpus.rs` because this is the walk that exercises
    // the *write* path's own reads — the validations a verb performs before it commits — and
    // those are exactly where a cost paid in answering a question hides (trap 33).
    assert!(
        tally.unexplained.is_empty(),
        "a verb's mount ran a generator again for a subject it had already answered, more often \
         than its cache forgot anything:\n{}",
        tally
            .unexplained
            .iter()
            .take(40)
            .map(|(what, why)| format!("    {what}: {why}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
    assert!(
        tally.prefix_failed.is_empty(),
        "§7.5.6: changes shall be appended to the end of the file, leaving its original contents \
         intact"
    );
    assert!(
        tally.reread_failed.is_empty(),
        "this tree must read back every document it wrote"
    );
    assert!(
        tally.renumbering_failed.is_empty(),
        "RFC 0003 section 5.2: an ordinal is a position, and the listing after a write is a \
         listing of the document as it now is"
    );
    assert!(
        tally.structure_failed.is_empty(),
        "§14.7.5.4: a carried page's key would name somebody else's elements, and an edited \
         document's own pages keep theirs"
    );
    assert!(
        tally.attachment_failed.is_empty(),
        "§7.11.4: a file embedded is a file that reads back, and removing it leaves the \
         document's own files listed"
    );
    assert!(
        tally.information_failed.is_empty(),
        "§14.3.3: meta/info.json is the whole of Table 349, so what it states is what the \
         document states"
    );
    assert!(
        tally.not_idempotent.is_empty(),
        "ADR 0855 section 5: reading meta/info.json and writing it straight back changes nothing"
    );
    assert!(
        tally.nondeterministic.is_empty(),
        "RFC 0002 section 9: same sources, same plan, same bytes"
    );

    for (name, why) in &tally.differ {
        assert!(
            HELD.iter().any(|(held, _)| held == name),
            "a page that draws differently after an edit and nobody has read: {name}: {why}"
        );
    }
    assert!(
        tally.identical > 0,
        "a corpus in which no edited page draws as it did is not this corpus"
    );
    for (index, verb) in VERBS.iter().enumerate() {
        assert!(
            tally.committed[index] > 0,
            "no corpus document was edited by `{verb}`, so this walk measured nothing of it"
        );
    }
}
