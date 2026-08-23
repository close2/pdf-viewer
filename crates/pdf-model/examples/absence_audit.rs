//! Re-asks, with this tree's own readers, the "no corpus document does X" claims it repeats.
//!
//! The companion to `witness_census`, and the reason there are two. A name census answers
//! "does the token appear", which over-reports: a document may state `/IDTree` and have the tree
//! resolve to nothing, and a `/Threads` array may be empty. What decides a claim like "no corpus
//! document states an article" is the *structure* the claim is about, read by the code that would
//! act on it — which is the instrument ADR 0403 says must be run beside the grep rather than
//! instead of it.
//!
//! Each block below is one written claim, cited to where this tree states it, and prints the
//! population that would falsify it. **Nothing here asserts**: it is a measurement, and the
//! claims it settles are corrected in prose where they are written rather than pinned here.
//!
//! ```sh
//! cargo run --release -p pdf-model --example absence_audit
//! cargo run --release -p pdf-model --example absence_audit -- --pdfjs
//! cargo run --release -p pdf-model --example absence_audit -- --crawl   # CC-MAIN-2021-31
//! ```
//!
//! **`--crawl` is the six-hundred-and-seventy-first session's**, and it is here for the reason
//! ADR 0493 put it on `witness_census`: an instrument has a population too, and a claim can only
//! decay as far as its instrument can reach. This one's was `doc/pdf.js`, `doc/corpora` and this
//! project's fixtures, hard-coded, while the crawl sat on the same disk. Run it *with* the
//! control run rather than instead of one — a negative measured before the crawl arrived is
//! usually right about its own population, which is exactly why nothing in the tree could see it
//! (ADR 0490).

#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "an example whose entire output is a measurement"
)]
#![expect(
    clippy::arithmetic_side_effects,
    reason = "counters over a corpus of a few thousand files; a measurement rather than a \
              shipped path"
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use pdf_syntax::{Dictionary, Document, Object, ObjectId};
use rayon::prelude::*;

/// How many witnessing documents are named per claim before the list is truncated.
///
/// The curated population is small enough to print whole and the crawl is not: a claim the crawl
/// answers in the thousands would otherwise bury the claims it answers with none, which are the
/// ones this example exists to show.
const MAX_NAMED: usize = 12;

/// Which population a run is over.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Scope {
    /// The pdf.js corpus alone — "the 974", which most of this project's claims are about.
    PdfJs,
    /// That, the four `doc/corpora/` submodules, and this project's own fixtures.
    Curated,
    /// The `SafeDocs` `CC-MAIN-2021-31` crawl under `corpus-cache/`, and nothing else.
    Crawl,
}

/// Every PDF this project can measure over, in the scope asked for.
///
/// [`Scope::Crawl`] is separate rather than added, for ADR 0490's reason: a re-derivation states
/// the control and the growth apart, because one number over both would hide which of the two
/// moved.
fn corpus(scope: Scope) -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = Vec::new();
    let scope: &[&str] = match scope {
        Scope::PdfJs => &["doc/pdf.js/test/pdfs"],
        Scope::Curated => &["doc/pdf.js/test/pdfs", "doc/corpora", "doc/corpora-own"],
        Scope::Crawl => &["corpus-cache/safedocs/cc-main-2021-31"],
    };
    for relative in scope {
        collect(&root.join(relative), &mut files);
    }
    files.sort();
    files.dedup();
    files
}

/// Every `.pdf` under one directory, recursively.
fn collect(dir: &Path, into: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, into);
        } else if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("pdf"))
        {
            into.push(path);
        }
    }
}

/// What one document answered to each claim, as a sentence naming what it holds.
#[derive(Default)]
struct Answers {
    /// §14.7.2's `/IDTree`, and how many identifiers it names.
    id_tree: Option<String>,
    /// §12.4.3's threads, and how many beads hang off them.
    articles: Option<String>,
    /// §12.3.5's `/Collection`.
    collection: Option<String>,
    /// §12.9's `/VP` viewports on any page.
    viewports: Option<String>,
    /// §12.9.2's own population: a viewport whose `/Measure` is Table 267's rectilinear one.
    ///
    /// Separate from [`Answers::viewports`] because it is a separate claim, and because a count
    /// is what a report line gives: a document with a `GEO` measure exercises §12.10 and leaves
    /// §12.9.2's number format algorithm untouched.
    rectilinear: Option<String>,
    /// §14.10.2's `/SpiderInfo`, and on what.
    spider: Option<String>,
    /// §12.7.5.5's `/Lock` on a signature field.
    field_lock: Option<String>,
    /// §12.7.5.5's Table 236 `/P`, which that row says is deliberately not read.
    lock_permission: Option<String>,
    /// §12.8.2.4's `FieldMDP` transform.
    field_mdp: Option<String>,
    /// §12.2's four deprecated page-boundary entries of Table 147.
    boundaries: Option<String>,
    /// §12.11.1's `/Requirements`, and which types the document states.
    requirements: Option<String>,
    /// §12.5.6.21 and §14.11.6.2's `/Subtype /TrapNet` annotation.
    trap_net: Option<String>,
    /// §10.7.2's `/FL` in a graphics state parameter dictionary.
    flatness: Option<String>,
    /// §7.11.4.2's `/RF` on a file specification.
    related_files: Option<String>,
    /// §12.6.3's `/PV` and `/PI`, Table 197's two page-visibility triggers.
    page_visibility: Option<String>,
    /// §12.6.4.7's thread action — an action dictionary whose `/S` is `Thread`.
    thread_action: Option<String>,
    /// §12.8.2.2.1's `/DocMDP`, by the level §12.8.6's `/Perms` makes binding.
    ///
    /// Two facts rather than one, because the clause separates them: a `DocMDP` transform states
    /// what invalidates the author's signature, and only §12.8.2.2.1's parenthesis — a signature
    /// dictionary the permissions dictionary refers to — turns that into something a reader has
    /// to prevent. A count of certification signatures is therefore not a count of restrictions.
    certification: Option<String>,
}

fn main() {
    let scope = if std::env::args().any(|a| a == "--crawl") {
        Scope::Crawl
    } else if std::env::args().any(|a| a == "--pdfjs") {
        Scope::PdfJs
    } else {
        Scope::Curated
    };
    let files = corpus(scope);
    eprintln!("{} PDF(s) in the population", files.len());

    let results: Vec<(String, Answers)> = files
        .par_iter()
        .map(|path| {
            let label = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            (label, measure(path))
        })
        .collect();

    report_every_claim(&results);
}

/// Prints one section per written claim, in the order the claims were added.
///
/// Separate from [`main`] only because the list grows by one entry per round that re-derives a
/// negative, and a claim is a line here rather than a branch anywhere: the ordering carries no
/// meaning and nothing reads the sections back.
fn report_every_claim(results: &[(String, Answers)]) {
    report(
        "§14.7.2's /IDTree — the claim was \"none at all\" and is FALSE (ADR 0405)",
        results,
        |a| a.id_tree.as_deref(),
    );
    report(
        "§12.4.3's articles — the claim was \"none\"; true of pdf.js, FALSE wider (ADR 0405)",
        results,
        |a| a.articles.as_deref(),
    );
    report(
        "§12.3.5's /Collection — the claim was \"none\"; true of pdf.js, FALSE wider (ADR 0405)",
        results,
        |a| a.collection.as_deref(),
    );
    report(
        "§12.9's /VP, with each viewport's /Measure subtype named",
        results,
        |a| a.viewports.as_deref(),
    );
    report(
        "§12.9.2's algorithm — its population is Table 267's RL measure, not §12.10's GEO",
        results,
        |a| a.rectilinear.as_deref(),
    );
    report(
        "§14.10.2's /SpiderInfo — the claim was \"none\" and is FALSE (ADR 0405)",
        results,
        |a| a.spider.as_deref(),
    );
    report(
        "§12.7.5.5's /Lock on a signed field — the claim was \"none\" (curated) and is FALSE wider",
        results,
        |a| a.field_lock.as_deref(),
    );
    report(
        "§12.7.5.5's Table 236 /P — the claim is that it is deliberately not read here",
        results,
        |a| a.lock_permission.as_deref(),
    );
    report(
        "§12.8.2.4's FieldMDP — the transform, over the whole population",
        results,
        |a| a.field_mdp.as_deref(),
    );
    report(
        "§12.2's Table 147 boundary entries — the claim was \"none of the four\" (curated)",
        results,
        |a| a.boundaries.as_deref(),
    );
    report(
        "§12.11.1's /Requirements — the claim was \"no corpus document states a requirement\"",
        results,
        |a| a.requirements.as_deref(),
    );
    report(
        "§12.5.6.21 and §14.11.6.2's /TrapNet — the claim was \"none at all\"",
        results,
        |a| a.trap_net.as_deref(),
    );
    report(
        "§10.7.2's /FL in an /ExtGState — the claim was \"none writes it in one at all\"",
        results,
        |a| a.flatness.as_deref(),
    );
    report(
        "§7.11.4.2's /RF — the claim was \"no corpus file specification carries one\"",
        results,
        |a| a.related_files.as_deref(),
    );
    report(
        "§12.6.3's /PV and /PI — the claim was \"no document states\" either",
        results,
        |a| a.page_visibility.as_deref(),
    );
    report(
        "§12.6.4.7's thread action — the claim was \"no corpus document states\" one",
        results,
        |a| a.thread_action.as_deref(),
    );
    report(
        "§12.8.2.2.1's /DocMDP — the claim was \"one corpus document, and it states /P 2\"",
        results,
        |a| a.certification.as_deref(),
    );
}

/// How deep an object's own structure is walked before [`visit`] gives up.
///
/// The walk never follows a reference, so this bounds one object's own nesting rather than the
/// document's graph — the same bound and the same reason as `witness_census`'s.
const MAX_DEPTH: usize = 64;

/// What one document's objects hold, for the claims whose subject is a dictionary somewhere.
#[derive(Default)]
struct Sightings {
    /// §14.10.2: the `/Type` of each dictionary carrying a `/SpiderInfo`.
    spider: Vec<String>,
    /// §12.5.6.21, §14.11.6.2: trap network annotations.
    trap_nets: usize,
    /// §10.7.2: how each `/FL` was reached — a typed dictionary, or a resource entry.
    flatness: Vec<String>,
    /// §7.11.4.2: what `attachment::related` returns for each file specification with an `/RF`.
    related: Vec<String>,
    /// §12.6.3: which of `/PV` and `/PI` an annotation's `/AA` states.
    visibility: Vec<String>,
    /// §12.6.4.7: action dictionaries whose `/S` is `Thread`.
    threads: usize,
    /// §12.7.5.5: Table 236's `/P`, as each signature field lock dictionary states it.
    lock_permissions: Vec<String>,
}

/// Asks one object, and everything nested inside it, the six object-scoped claims.
///
/// References are not followed — an indirect object is asked when the loop reaches it — so this
/// terminates on a document whose graph is a cycle, which is a shape a hostile file states.
fn visit(document: &Document, object: &Object, depth: usize, into: &mut Sightings) {
    if depth > MAX_DEPTH {
        return;
    }
    let dict = match object {
        Object::Dictionary(dict) => dict,
        Object::Stream(stream) => &stream.dict,
        Object::Array(items) => {
            for item in items {
                visit(document, item, depth + 1, into);
            }
            return;
        }
        _ => return,
    };

    // §14.10.2's `/SpiderInfo`, which Table 28 puts on the catalog and Table 358 on a structure
    // element's `/K`.
    if !document.get_key(dict, "SpiderInfo").is_null() {
        let owner = document.get_key(dict, "Type");
        into.spider.push(match owner.as_name() {
            Some(name) => String::from_utf8_lossy(name.as_bytes()).into_owned(),
            None => "untyped".to_owned(),
        });
    }

    // §12.5.6.21's and §14.11.6.2's trap network annotation, which is a `/Subtype` *value* rather
    // than a key — the direction `witness_census`'s own comment distinguishes.
    if named(document, dict, "Subtype", b"TrapNet") {
        into.trap_nets += 1;
    }

    // §10.7.2's flatness tolerance, asked two ways because Table 57's `/Type` is optional: a
    // dictionary that declares itself a graphics state parameter dictionary, and one a resource
    // dictionary names under `/ExtGState`. Both are printed, because a document that states `/FL`
    // only in an untyped one is still a document that states it.
    if named(document, dict, "Type", b"ExtGState") && !document.get_key(dict, "FL").is_null() {
        into.flatness.push("a typed /ExtGState".to_owned());
    }
    if let Some(states) = document.get_key(dict, "ExtGState").as_dict() {
        for (_, value) in states.iter() {
            if let Some(state) = document.resolve(value).as_dict()
                && !document.get_key(state, "FL").is_null()
            {
                into.flatness
                    .push("a /Resources /ExtGState entry".to_owned());
            }
        }
    }

    // §7.11.4.2's related files array. A file specification is what Table 43 describes, so the
    // question is asked of a dictionary that says it is one or that carries the `/EF` making it
    // an attachment — and the answer is `attachment::related`'s, which knows that `/RF` is keyed
    // like `/EF` and holds a flat array of `2 × n` elements rather than an array of pairs.
    let specification = named(document, dict, "Type", b"Filespec")
        || document.get_key(dict, "EF").as_dict().is_some();
    if specification && !document.get_key(dict, "RF").is_null() {
        let pairs = pdf_model::attachment::related(document, dict);
        into.related.push(format!("{} pair(s)", pairs.len()));
    }

    // §12.6.3's `/PV` and `/PI`. Table 197's ten events are an *annotation's* and Table 198's two
    // are a page's, told apart by the dictionary holding the `/AA` rather than by the key — which
    // is the rule `actions.rs::the_corpus_states_these_page_scoped_triggers` applies, and this is
    // that gate's question asked over a wider population.
    if !named(document, dict, "Type", b"Page")
        && let Some(additional) = document.get_key(dict, "AA").as_dict()
    {
        for key in ["PV", "PI"] {
            if additional.get(key).is_some() {
                into.visibility.push(format!("/{key}"));
            }
        }
    }

    // §12.6.4.7's thread action. `/S` is Table 206's required action type, and `Thread` is a value
    // no other table gives that key, so the name settles it.
    if named(document, dict, "S", b"Thread") {
        into.threads += 1;
    }

    // §12.7.5.5's Table 236 `/P`, which §12.7.5.5's ledger row says is "deliberately not read
    // here". That disposal rests on one of the entry's sentences — absence having "no effect on
    // signature validation rules" — and the entry states five more, of which one is addressed to
    // a processor that changes the file: "The new permission applies to any incremental changes
    // to the document following the signature of which this key is part." So the question the row
    // settles by argument has a population, and nothing was counting it.
    //
    // Asked of the lock dictionary wherever one is stated, rather than only where the field is
    // signed: the row's claim is that this tree never reads the key, and a document stating it on
    // an unsigned field still falsifies "never stated" while not falsifying "asserts nothing".
    // The signed count is `field_locks`' and is reported by the block above.
    //
    // The `/Action` is named beside the level because the two answer different halves of what a
    // reader would owe: `All` already withholds every field from `Operation::FillInForm`, so a
    // `/P` beside it changes nothing this program does, while a narrower action leaves the rest of
    // the document open to an operation the level says is not permitted.
    if let Some(lock) = document.get_key(dict, "Lock").as_dict()
        && let Some(level) = document.get_key(lock, "P").as_integer()
    {
        let action = document.get_key(lock, "Action");
        let action = action.as_name().map_or_else(
            || "(no /Action)".to_owned(),
            |name| format!("/{}", String::from_utf8_lossy(name.as_bytes())),
        );
        into.lock_permissions
            .push(format!("/P {level} on {action}"));
    }

    for (_, value) in dict.iter() {
        visit(document, value, depth + 1, into);
    }
}

/// Whether one key of a dictionary is a name with this spelling.
///
/// A `Name` token compared as a token, so that `/TrapNet` does not match a `/TrapNetwork` a
/// producer invented and a name inside a string does not match at all.
fn named(document: &Document, dict: &Dictionary, key: &str, value: &[u8]) -> bool {
    document
        .get_key(dict, key)
        .as_name()
        .is_some_and(|name| name.as_bytes() == value)
}

/// Prints one claim's witnesses, or says it has none.
///
/// # The tally, where the list is truncated
///
/// A negative is retired by a count and *replaced* by a distribution — §12.2's ninety-six
/// witnesses turned out to state Table 147's own default, which is what kept the sharper half of
/// that claim true — so a truncated list of a hundred and forty-three names is a number with the
/// finding cut off it. Where more witnesses were found than [`MAX_NAMED`] prints, the answers
/// themselves are tallied: each block's answer is worded as what the document *holds*, so the
/// distinct answers are the distribution and there are few of them.
fn report(claim: &str, results: &[(String, Answers)], pick: impl Fn(&Answers) -> Option<&str>) {
    let found: Vec<(&str, &str)> = results
        .iter()
        .filter_map(|(label, answers)| pick(answers).map(|what| (label.as_str(), what)))
        .collect();
    println!();
    println!("{claim}");
    if found.is_empty() {
        println!("  no witness in this population");
        return;
    }
    println!("  {} witness(es)", found.len());
    for (label, what) in found.iter().take(MAX_NAMED) {
        println!("    {label}: {what}");
    }
    if found.len() > MAX_NAMED {
        println!("    … and {} more", found.len() - MAX_NAMED);
        let mut tally: BTreeMap<&str, usize> = BTreeMap::new();
        for (_, what) in &found {
            *tally.entry(what).or_default() += 1;
        }
        println!("    what they hold, every distinct answer:");
        for (what, count) in &tally {
            println!("      {count:>6}  {what}");
        }
    }
}

/// Asks one document every claim.
///
/// One block per written claim, each cited to the clause it is about: splitting them into
/// helpers would separate a claim from the reader that settles it, which is the whole point of
/// this example existing beside `witness_census`.
#[expect(
    clippy::too_many_lines,
    reason = "one block per written claim is this example's design, stated above: a helper per \
              claim would put the claim in one place and the reader that settles it in another, \
              and a helper for several would put unrelated clauses in one function"
)]
fn measure(path: &Path) -> Answers {
    let mut answers = Answers::default();
    let Ok(bytes) = std::fs::read(path) else {
        return answers;
    };
    let Ok(document) = Document::open(bytes) else {
        return answers;
    };

    // §14.7.2's Table 354 `/IDTree`, resolved as §7.9.6's name tree rather than counted as a key:
    // an entry present and empty would not falsify the claim.
    if let Ok(catalog) = document.catalog() {
        let root = document.get_key(&catalog, "StructTreeRoot");
        if let Some(root) = root.as_dict() {
            let entry = document.get_key(root, "IDTree");
            if let Some(dict) = entry.as_dict() {
                let pairs = pdf_syntax::tree::name_pairs(dict, &|object| document.resolve(object));
                answers.id_tree = Some(format!("{} identifier(s)", pairs.len()));
            }
        }
    }

    // §12.4.3's threads and beads, through the reader §12.4.3's panel uses.
    let articles = pdf_model::article::Articles::read(&document);
    if !articles.is_empty() {
        let beads: usize = articles.threads.iter().map(|t| t.beads.len()).sum();
        answers.articles = Some(format!(
            "{} thread(s), {beads} bead(s)",
            articles.threads.len()
        ));
    }

    // §12.3.5's `/Collection`, through the reader the sidebar's folder tree uses.
    if let Some(collection) = pdf_model::collection::Collection::read(&document) {
        answers.collection = Some(format!(
            "{} field(s), {} folder(s)",
            collection.schema.len(),
            collection.all_folders().len()
        ));
    }

    // §12.9's `/VP`, asked of every page rather than of page one: a viewport is a region of a
    // page and a document that states one need not state it first.
    //
    // **The `/Measure` subtype is named rather than counted**, and that is §12.9.2's claim rather
    // than §12.9's: the number format algorithm is Table 267's, so a page full of `GEO` viewports
    // leaves "[n]o corpus document exercises it" standing while one `RL` with a number format
    // array retires it. Table 266 makes `RL` the *default*, so a measure dictionary with no
    // `/Subtype` is one — which is why this asks the reader rather than the key.
    let pages = pdf_model::Pages::new(&document);
    let mut viewports = 0usize;
    let mut measures: Vec<&'static str> = Vec::new();
    for index in 0..pages.len() {
        let Some(page) = pages.get(index) else {
            continue;
        };
        for viewport in pdf_model::measurement::Viewports::read(&document, &page.dict).viewports {
            viewports += 1;
            match viewport.measure {
                Some(pdf_model::measurement::Measure::Rectilinear(_)) => measures.push("RL"),
                Some(pdf_model::measurement::Measure::Geospatial(_)) => measures.push("GEO"),
                Some(pdf_model::measurement::Measure::Other(_)) => measures.push("another subtype"),
                None => measures.push("no /Measure"),
            }
        }
    }
    if viewports > 0 {
        measures.sort_unstable();
        measures.dedup();
        if measures.contains(&"RL") {
            answers.rectilinear = Some("a rectilinear measure".to_owned());
        }
        answers.viewports = Some(format!("{viewports} viewport(s), {}", measures.join(", ")));
    }

    // §14.10.2's `/SpiderInfo`, which Table 28 puts on the catalog and Table 358 on a structure
    // element's `/K`; asked of every object, because the claim is about the document.
    //
    // **Five later claims share this walk rather than each opening one of their own.** Each is a
    // question about some dictionary *somewhere* in the file — a trap network annotation, a
    // graphics state parameter dictionary, a file specification, an annotation's `/AA`, an action
    // — and the object walk is where all five of those live. `document.xref().object_numbers()`
    // reaches the objects inside §7.5.7 object streams as well as the ones the table lists
    // directly, which is the scope a byte search does not have (ADR 0405).
    //
    // **The walk goes into each object's own nested structure**, which the `/SpiderInfo` block
    // never needed and the five do. A hand-built witness stating all seven constructs was run
    // through this example before the numbers were believed (`doc/habits.md`'s planted-witness
    // rule), and a first version that asked only the top-level dictionaries scored it **zero for
    // a thread action** — the action was written inline inside the annotation's `/AA`, which is
    // the six-hundred-and-forty-eighth session's finding exactly. A resource dictionary's
    // `/ExtGState` was invisible for the same reason, being one level under `/Resources`.
    let mut sightings = Sightings::default();
    for number in document.xref().object_numbers() {
        let object = document.get(ObjectId {
            number,
            generation: 0,
        });
        visit(&document, &object, 0, &mut sightings);
    }
    if !sightings.spider.is_empty() {
        sightings.spider.sort();
        sightings.spider.dedup();
        answers.spider = Some(format!("on /Type {}", sightings.spider.join(", ")));
    }
    if sightings.trap_nets > 0 {
        answers.trap_net = Some(format!("{} annotation(s)", sightings.trap_nets));
    }
    if !sightings.flatness.is_empty() {
        let total = sightings.flatness.len();
        sightings.flatness.sort();
        sightings.flatness.dedup();
        answers.flatness = Some(format!("{total} — {}", sightings.flatness.join(", ")));
    }
    if !sightings.related.is_empty() {
        answers.related_files = Some(sightings.related.join(", "));
    }
    if !sightings.visibility.is_empty() {
        sightings.visibility.sort();
        sightings.visibility.dedup();
        answers.page_visibility = Some(sightings.visibility.join(" "));
    }
    if sightings.threads > 0 {
        answers.thread_action = Some(format!("{} action(s)", sightings.threads));
    }
    if !sightings.lock_permissions.is_empty() {
        sightings.lock_permissions.sort();
        sightings.lock_permissions.dedup();
        answers.lock_permission = Some(sightings.lock_permissions.join(", "));
    }

    // §12.2's four deprecated page-boundary entries, read as *statements*: `ViewerPreferences`
    // resolves an absent entry to Table 147's `CropBox` default, so the reader alone cannot tell
    // a document that states the default from one that states nothing, and the claim is about
    // what a document states. Both are printed — the key, and what this tree makes of it.
    if let Ok(catalog) = document.catalog()
        && let Some(preferences) = document.get_key(&catalog, "ViewerPreferences").as_dict()
    {
        let stated: Vec<String> = ["ViewArea", "ViewClip", "PrintArea", "PrintClip"]
            .into_iter()
            .filter(|key| preferences.get(key).is_some())
            .map(|key| {
                let value = document.get_key(preferences, key);
                match value.as_name() {
                    Some(name) => {
                        format!("/{key} /{}", String::from_utf8_lossy(name.as_bytes()))
                    }
                    None => format!("/{key} (not a name)"),
                }
            })
            .collect();
        if !stated.is_empty() {
            let read =
                pdf_model::viewer_preferences::ViewerPreferences::in_catalog(&document, &catalog);
            answers.boundaries = Some(format!(
                "{} — this tree reads view {:?}/{:?}, print {:?}/{:?}",
                stated.join(", "),
                read.view_area,
                read.view_clip,
                read.print_area,
                read.print_clip
            ));
        }
    }

    // §12.11.1's requirements, through the reader that answers whether each is met.
    let requirements = pdf_model::requirements::read(&document);
    if !requirements.is_empty() {
        let kinds: Vec<&str> = requirements
            .iter()
            .map(|requirement| requirement.kind.as_str())
            .collect();
        answers.requirements = Some(kinds.join(" "));
    }

    // §12.7.5.5 and §12.8.2.4, the pair ADR 0403 corrected, re-asked over the wider population.
    let locks = pdf_model::signature::field_locks(&document);
    if !locks.is_empty() {
        answers.field_lock = Some(format!("{locks:?}"));
    }
    let covered = pdf_model::signature::field_mdp(&document);
    if !covered.is_empty() {
        answers.field_mdp = Some(format!("{covered:?}"));
    }

    // §12.8.2.2.1's `/P`, asked along the route that makes it binding rather than by the name.
    //
    // The clause states the two apart, and a claim about what a *reader* owes is about the second
    // only: "(These changes to the document shall also be prevented if the signature dictionary is
    // referred from the DocMDP entry in the permissions dictionary.)" So the key is asked of the
    // permissions dictionary, the level is `permissions`' — the same reader `restriction::asserted`
    // consults — and the number of certification signature fields is printed beside it, because a
    // `DocMDP` transform on a signature nothing points at asserts nothing and would otherwise be
    // counted as a restriction that is not there.
    //
    // **The third figure is the one a name census cannot produce**: a `/Perms /DocMDP` this tree
    // reads no level out of. Table 263 says the entry's dictionary "shall contain a Reference entry
    // that shall be a signature reference dictionary", singular, while Table 255 makes `/Reference`
    // an array — so a producer following the wrong one of the standard's two sentences writes a
    // bare dictionary, and a reader that only accepts the array finds no level and permits
    // everything without saying so.
    let states_perms_doc_mdp = document.catalog().is_ok_and(|catalog| {
        document
            .get_key(&catalog, "Perms")
            .as_dict()
            .is_some_and(|perms| !document.get_key(perms, "DocMDP").is_null())
    });
    let certifications = pdf_model::signature::signatures(&document)
        .iter()
        .filter(|signature| signature.certification)
        .count();
    if states_perms_doc_mdp || certifications > 0 {
        let bound = match pdf_model::signature::permissions(&document).doc_mdp {
            Some(level) => format!("/Perms binds {level:?}"),
            None if states_perms_doc_mdp => {
                "a /Perms /DocMDP this tree reads no level out of".to_owned()
            }
            None => "no /Perms /DocMDP".to_owned(),
        };
        answers.certification = Some(format!("{bound}; {certifications} certification field(s)"));
    }

    answers
}
