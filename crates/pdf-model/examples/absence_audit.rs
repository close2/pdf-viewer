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
    /// §7.6.5's public-key security handler — a `/Filter` that is not `/Standard`.
    public_key: Option<String>,
    /// §7.9.2.2.2's U+001B language escape inside a Unicode text string.
    language_escape: Option<String>,
    /// §8.9.5.2's `/Decode` array, where it is neither Table 88's default nor its reversal.
    decode_array: Option<String>,
    /// §8.10.3's group `XObject` whose `/S` is not `/Transparency`.
    group_subtype: Option<String>,
    /// §11.6.5.2's soft-mask image behind one of §7.4's image codecs.
    codec_mask: Option<String>,
    /// The half of that claim the residue is actually about: a codec-carrying mask on a pair
    /// `image.rs::worth_combining` sends to the device-scale route, which is the only place the
    /// codec is asked about at all.
    codec_mask_deferred: Option<String>,
    /// §12.3.2.2's destination whose first element is an integer rather than a page reference.
    numbered_destination: Option<String>,
    /// §12.4.2's page label ranges, for a document stating as many as the clause's example.
    label_ranges: Option<String>,
    /// §12.5.1's rotated page carrying a widget annotation.
    rotated_widget: Option<String>,
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
#[expect(
    clippy::too_many_lines,
    reason = "one call per written claim, which is this example's design: the length is the \
              number of claims re-derived and splitting it would put half the list somewhere else"
)]
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
    report(
        "§7.6.5's public-key handler — the claim was \"no corpus document uses one\"",
        results,
        |a| a.public_key.as_deref(),
    );
    report(
        "§7.9.2.2.2's language escape — the claim was \"no corpus document writes\" the construct",
        results,
        |a| a.language_escape.as_deref(),
    );
    report(
        "§8.9.5.2's /Decode — the claim was \"all 974 write Table 88's default or its reversal\"",
        results,
        |a| a.decode_array.as_deref(),
    );
    report(
        "§8.10.3's /Group /S — the claim was \"no corpus document writes another subtype\"",
        results,
        |a| a.group_subtype.as_deref(),
    );
    report(
        "§11.6.5.2's codec-carrying /SMask — the claim was \"no corpus document states\" one",
        results,
        |a| a.codec_mask.as_deref(),
    );
    report(
        "§11.6.5.2's codec-carrying /SMask on a pair the device-scale route would have taken",
        results,
        |a| a.codec_mask_deferred.as_deref(),
    );
    report(
        "§12.3.2.2's integer first element — the claim was \"no corpus link uses it\"",
        results,
        |a| a.numbered_destination.as_deref(),
    );
    report(
        "§12.4.2's ranges — the claim was \"no corpus document exercises all three\" of the \
         example's",
        results,
        |a| a.label_ranges.as_deref(),
    );
    report(
        "§12.5.1's rotated page with a widget — the claim was \"no corpus document states\" one",
        results,
        |a| a.rotated_widget.as_deref(),
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
    /// §7.9.2.2.2: how many U+001B escapes were stated, and how many formed a sequence.
    escapes: (usize, usize),
    /// §8.9.5.2: how each `/Decode` array that is neither default nor reversal was shaped.
    decodes: Vec<String>,
    /// §8.10.3: what each group `XObject`'s `/S` says, where it does not say `/Transparency`.
    groups: Vec<String>,
    /// §11.6.5.2: which of §7.4's codecs each soft-mask image is behind.
    codec_masks: Vec<String>,
    /// §11.6.5.2: the same, for a pair `worth_combining` refuses.
    deferred_codec_masks: Vec<String>,
    /// §12.3.2.2: where each integer-first destination was stated.
    numbered_destinations: Vec<String>,
}

/// Asks one object, and everything nested inside it, the six object-scoped claims.
///
/// References are not followed — an indirect object is asked when the loop reaches it — so this
/// terminates on a document whose graph is a cycle, which is a shape a hostile file states.
#[expect(
    clippy::too_many_lines,
    reason = "one block per object-scoped claim, each cited to its clause; a helper per claim \
              would separate a claim from the reader that settles it, which is what `measure`'s \
              own expectation says of the same design"
)]
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
        // §7.9.2.2.2's language escape sequence, which lives inside a *string* rather than in
        // any dictionary — so this arm exists for it, and it is the only claim here whose
        // subject is an object with no keys.
        Object::String(bytes) => {
            let (stated, formed) = language_escapes(bytes);
            into.escapes.0 += stated;
            into.escapes.1 += formed;
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

    // §8.9.5.2's `/Decode` and §11.6.5.2's soft mask, both of them questions about an image
    // XObject and asked together because Table 87 is where both entries live.
    if named(document, dict, "Subtype", b"Image") {
        if let Some(departure) = decode_departure(document, dict) {
            into.decodes.push(departure);
        }
        // §11.6.5.2's third residue, refused by name in `image.rs`: "a mask behind an image
        // codec, whose samples have no position until the whole codestream is decoded". The
        // mask is a stream, so the question is its own `/Filter` chain rather than the parent's.
        //
        // **The codec alone is not the condition, which is trap 11's whole subject.** A
        // codec-carrying mask is refused nothing: `soft_mask_entry` consults
        // `eligible_for_the_device_scale` only where `worth_combining` has already said the
        // refinement of the two grids is too large to build, so a mask on a grid the eager route
        // will allocate is combined and drawn however it is filtered. The pair that *reaches* the
        // sentence is the one this splits out, and it is the population a report can fire on.
        let mask = document.get_key(dict, "SMask");
        if let Some(mask) = dict_of(&mask) {
            let codecs: Vec<String> = filter_names(document, mask)
                .into_iter()
                .filter(|filter| is_image_codec(filter))
                .collect();
            if !codecs.is_empty() {
                if !worth_combining(document, dict, mask) {
                    into.deferred_codec_masks.push(codecs.join(" "));
                }
                into.codec_masks.push(codecs.join(" "));
            }
        }
    }

    // §8.10.3's group XObject. Table 94 makes `/S` required and §11.6.6 says a group with any
    // other subtype "shall not be subject to any grouping behaviour", so an absent `/S` and a
    // stated one that is not `/Transparency` are two different witnesses and are named apart.
    let group = document.get_key(dict, "Group");
    if let Some(group) = group.as_dict() {
        let subtype = document.get_key(group, "S");
        match subtype.as_name() {
            Some(name) if name.as_bytes() == b"Transparency" => {}
            Some(other) => into
                .groups
                .push(format!("/S /{}", String::from_utf8_lossy(other.as_bytes()))),
            None => into.groups.push("a /Group with no /S".to_owned()),
        }
    }

    // §12.3.2.2's integer first element, asked of the two keys that hold a destination in a
    // dictionary: Table 176's `/Dest` on a link, Table 151's on an outline item, and Table 202's
    // `/D` on a go-to action. Only the explicit array and §12.3.2.4's dictionary form are asked here — a
    // *named* destination is asked once per document at its definition, in `measure`, because
    // resolving one costs a name-tree walk and a document states each name once and links to it
    // many times.
    //
    // `/D` is a key several unrelated tables use — Table 168's down appearance is the loudest —
    // and nothing filters them, because `Destination::read` is the filter: it yields `Some` only
    // for an array whose second element is one of Table 149's eight form names, and
    // `Target::Number` only where the first is an integer.
    for key in ["Dest", "D"] {
        let entry = document.get_key(dict, key);
        if matches!(entry, Object::Null) {
            continue;
        }
        if !matches!(
            document.resolve(&entry),
            Object::Array(_) | Object::Dictionary(_)
        ) {
            continue;
        }
        if let Some(destination) = pdf_model::destination::Destination::read(document, &entry)
            && matches!(
                destination.target,
                pdf_model::destination::Target::Number(_)
            )
        {
            // The NOTE makes the integer form belong to a destination in *another* document:
            // "No page object can be specified for a destination associated with a remote go-to
            // action". So a `/GoToR` or `/GoToE` stating one is the clause's own case, and
            // anything else is the shape §12.3.2.2's "shall" excludes — a distinction the count
            // alone would lose.
            let remote =
                named(document, dict, "S", b"GoToR") || named(document, dict, "S", b"GoToE");
            into.numbered_destinations.push(if remote {
                "on a /GoToR or /GoToE, which §12.3.2.2's NOTE describes".to_owned()
            } else {
                "on a destination naming no other file".to_owned()
            });
        }
    }

    for (_, value) in dict.iter() {
        visit(document, value, depth + 1, into);
    }
}

/// The dictionary of an object that is one, or of a stream that carries one.
fn dict_of(object: &Object) -> Option<&Dictionary> {
    match object {
        Object::Dictionary(dict) => Some(dict),
        Object::Stream(stream) => Some(&stream.dict),
        _ => None,
    }
}

/// `image.rs::worth_combining`, asked of an image dictionary and its mask's.
///
/// The two constants are that function's own — `PREFER_DEVICE_SCALE_ABOVE` is 2^24 and the
/// comparison is against the base image's own sample count, so a mask on the image's grid is
/// never "not worth combining". Restated here rather than exported, because exporting a private
/// threshold so that a census can agree with it is how the two stop being independent: this
/// copy is wrong the moment the original moves, and a census that silently tracked it would
/// report a condition nobody had checked.
fn worth_combining(document: &Document, dict: &Dictionary, mask: &Dictionary) -> bool {
    let dimension = |dict: &Dictionary, key| {
        document
            .get_key(dict, key)
            .as_integer()
            .and_then(|value| u64::try_from(value).ok())
            .unwrap_or(0)
    };
    let (width, height) = (dimension(dict, "Width"), dimension(dict, "Height"));
    let (mask_width, mask_height) = (dimension(mask, "Width"), dimension(mask, "Height"));
    let grid = width
        .max(mask_width)
        .saturating_mul(height.max(mask_height));
    let image = width.saturating_mul(height);
    grid <= (1u64 << 24).max(image)
}

/// Every name in a stream dictionary's `/Filter`, which §7.4 allows to be a name or an array.
fn filter_names(document: &Document, dict: &Dictionary) -> Vec<String> {
    let entry = document.get_key(dict, "Filter");
    let mut names = Vec::new();
    let mut push = |object: &Object| {
        if let Some(name) = object.as_name() {
            names.push(String::from_utf8_lossy(name.as_bytes()).into_owned());
        }
    };
    match &entry {
        Object::Array(items) => {
            for item in items {
                push(&document.resolve(item));
            }
        }
        other => push(other),
    }
    names
}

/// Whether a filter name is one of the four §7.4 image codecs, including Table 6's abbreviations.
///
/// The distinction §11.6.5.2 draws is between a filter that unpacks samples in place and one
/// whose output has "no position until the whole codestream is decoded"; these four are the
/// second kind.
fn is_image_codec(filter: &str) -> bool {
    matches!(
        filter,
        "DCTDecode" | "DCT" | "JPXDecode" | "JBIG2Decode" | "CCITTFaxDecode" | "CCF"
    )
}

/// §8.9.5.2: how an image's `/Decode` array departs from Table 88, or `None` where it does not.
///
/// The claim this settles is that every corpus image "write[s] Table 88's default or its exact
/// reversal", so the four defaults the table states have to be told from a general array. Three
/// of them are arithmetic on the image dictionary alone — every device space's components run
/// from 0.0 to 1.0, an `/ImageMask`'s single component likewise, and NOTE 2's `Indexed` default
/// is `[0 2^n − 1]` from `/BitsPerComponent`. The fourth, `Lab`, takes its two chromatic bounds
/// from the space's own `/Range`, which is read here from the space array for the same reason
/// `ColourSpace::parse` reads it there.
///
/// **The space is not resolved through a resource dictionary**, because an object walk has none
/// — a `/ColorSpace /CS0` names an entry of the page's resources. That costs nothing for the
/// three defaults above, which do not depend on the space, and it means a `Lab` image naming its
/// space by a resource key is reported as a departure. The answer names what `/ColorSpace` says,
/// so such a hit is legible as one.
fn decode_departure(document: &Document, dict: &Dictionary) -> Option<String> {
    let entry = document.get_key(dict, "Decode");
    let items = entry.as_array()?;
    let stated: Vec<f64> = items
        .iter()
        .filter_map(|item| document.resolve(item).as_number())
        .collect();
    if stated.len() != items.len() || stated.is_empty() || !stated.len().is_multiple_of(2) {
        return Some(format!(
            "{} element(s), not a whole set of numeric pairs",
            items.len()
        ));
    }
    let pairs: Vec<(f64, f64)> = stated
        .chunks_exact(2)
        .map(|pair| (pair[0], pair[1]))
        .collect();

    // NOTE 2: an `Indexed` space's default passes the index through unchanged, so its pair is
    // `[0 2^n − 1]` at the image's own bit depth.
    let bits = if matches!(document.get_key(dict, "ImageMask"), Object::Boolean(true)) {
        1
    } else {
        document
            .get_key(dict, "BitsPerComponent")
            .as_integer()
            .and_then(|value| u32::try_from(value).ok())
            .unwrap_or(8)
    };
    let top = f64::from((1u32 << bits.min(16)).saturating_sub(1));

    // Table 88's three shapes an image dictionary can be measured against without its resources:
    // every component's full range, which is 0.0 to 1.0 for every device space, for `CalRGB`,
    // `CalGray`, `ICCBased`, a separation or `DeviceN`, and for `/ImageMask true`; NOTE 2's
    // `Indexed` pair; and the `Lab` row, read from the space array.
    let mut defaults: Vec<Vec<(f64, f64)>> = vec![
        std::iter::repeat_n((0.0, 1.0), pairs.len()).collect(),
        vec![(0.0, top)],
    ];
    defaults.extend(lab_default(document, dict));

    // **And each default's exact reversal**, which the claim names beside the default itself and
    // which is *not* `[1 0]` per component in general: `issue10339_reduced.pdf` states
    // `/Decode [255.0 0.0]` on an eight-bit `Indexed` image, which is `[0 255]` reversed. A first
    // draft of this block scored it a departure and would have retired a claim that holds.
    if defaults.iter().any(|default| {
        *default == pairs
            || default
                .iter()
                .map(|(low, high)| (*high, *low))
                .eq(pairs.iter().copied())
    }) {
        return None;
    }

    Some(format!(
        "{} pair(s) on {}",
        pairs.len(),
        colour_space_label(document, dict)
    ))
}

/// Table 88's `Lab` row: `[0 100 amin amax bmin bmax]`, from the space's own `/Range`.
fn lab_default(document: &Document, dict: &Dictionary) -> Option<Vec<(f64, f64)>> {
    let space = document.get_key(dict, "ColorSpace");
    let space = document.resolve(&space);
    let items = space.as_array()?;
    if items.first()?.as_name()?.as_bytes() != b"Lab" {
        return None;
    }
    let parameters = document.resolve(items.get(1)?);
    let parameters = parameters.as_dict()?;
    // §8.6.5.4 Table 65: "Default value: [−100 100 −100 100]".
    let range = document.get_key(parameters, "Range");
    let range: Vec<f64> = range
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| document.resolve(item).as_number())
                .collect()
        })
        .filter(|values: &Vec<f64>| values.len() == 4)
        .unwrap_or_else(|| vec![-100.0, 100.0, -100.0, 100.0]);
    Some(vec![
        (0.0, 100.0),
        (range[0], range[1]),
        (range[2], range[3]),
    ])
}

/// What an image dictionary says its colour space is, as a phrase for a tally.
fn colour_space_label(document: &Document, dict: &Dictionary) -> String {
    if matches!(document.get_key(dict, "ImageMask"), Object::Boolean(true)) {
        return "an /ImageMask".to_owned();
    }
    let space = document.get_key(dict, "ColorSpace");
    let space = document.resolve(&space);
    match &space {
        Object::Name(name) => format!("/{}", String::from_utf8_lossy(name.as_bytes())),
        Object::Array(items) => items.first().and_then(Object::as_name).map_or_else(
            || "an array naming no family".to_owned(),
            |name| format!("[/{} …]", String::from_utf8_lossy(name.as_bytes())),
        ),
        _ => "no /ColorSpace".to_owned(),
    }
}

/// §7.9.2.2.2: how many U+001B escapes a string states, and how many of them form a sequence.
///
/// The escape belongs to the two Unicode forms and to no other: §7.9.2.2.2 states its byte
/// sequences for UTF-16BE and UTF-8 and not for `PDFDocEncoding`, which is what `text_string`
/// implements. So a string with neither prefix is not this construct however many `0x1B` bytes it
/// holds, and answering otherwise would count every binary string in the file. Which prefix marks
/// which form is §7.9.2.2.1's:
///
/// > For text strings encoded in UTF-16BE, the first two bytes shall be 254 followed by 255.
///
/// > For text strings encoded in UTF-8, the first three bytes shall be 239 followed by 187,
/// > followed by 191.
///
/// The second figure is the reader's own: `text_string` removes a *well-formed* sequence and
/// leaves a lone escape where it stands, so an escape that survives the decode is one the clause
/// does not define and the difference is the count of sequences the reader acted on.
fn language_escapes(bytes: &[u8]) -> (usize, usize) {
    const ESCAPE: char = '\u{1b}';
    let stated = if let Some(rest) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        rest.chunks_exact(2)
            .filter(|unit| *unit == [0x00, 0x1B])
            .count()
    } else if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        rest.split(|byte| *byte == 0x1B).count().saturating_sub(1)
    } else {
        0
    };
    if stated == 0 {
        return (0, 0);
    }
    let surviving = pdf_syntax::text_string(bytes)
        .chars()
        .filter(|character| *character == ESCAPE)
        .count();
    (stated, stated.saturating_sub(surviving))
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
    // §7.6.5's public-key security handlers, asked of the reader that would act on one. The row's
    // own sentence is that "a /Filter that is not /Standard produces `SyntaxError::
    // UnsupportedEncryption` quoting the name", so the refusal *is* the measurement and there is
    // nothing further to open — a document this arm answers for has no object graph here at all.
    //
    // The other details that error carries are §7.6.4's and §7.6.6's — a revision, a crypt filter
    // method, a key length — and they are a different clause, so only the handler sentence is
    // taken. `crypt.rs` writes it in one place.
    let document = match Document::open(bytes) {
        Ok(document) => document,
        Err(pdf_syntax::SyntaxError::UnsupportedEncryption { detail }) => {
            if detail.contains("is not the standard security handler") {
                answers.public_key = Some(detail);
            }
            return answers;
        }
        Err(_) => return answers,
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
    let mut rotations: Vec<u16> = Vec::new();
    for index in 0..pages.len() {
        let Some(page) = pages.get(index) else {
            continue;
        };

        // §12.5.1's "no corpus document states a rotated page with a widget", which is the reason
        // ADR 0211's fix to `Query::Focus` could not be checked against a page. `Page::rotate` is
        // §7.7.3.3's inherited value already reduced to a multiple of 90, which is what the ring's
        // transform is built from — so this asks the same number the defect was in.
        if page.rotate != 0 {
            let annotations = document.get_key(&page.dict, "Annots");
            let widgets = annotations.as_array().is_some_and(|items| {
                items.iter().any(|item| {
                    document.resolve(item).as_dict().is_some_and(|annotation| {
                        named(&document, annotation, "Subtype", b"Widget")
                    })
                })
            });
            if widgets {
                rotations.push(page.rotate);
            }
        }

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
    if !rotations.is_empty() {
        rotations.sort_unstable();
        rotations.dedup();
        // The answer is the *set* of rotations rather than a count of pages, so that the tally
        // this example prints past twelve witnesses is a distribution over §7.7.3.3's four
        // values rather than one line per document.
        let named: Vec<String> = rotations
            .iter()
            .map(|rotate| format!("/Rotate {rotate}"))
            .collect();
        answers.rotated_widget = Some(named.join(", "));
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
    let (stated_escapes, formed_escapes) = sightings.escapes;
    if stated_escapes > 0 {
        answers.language_escape = Some(if formed_escapes > 0 {
            "at least one well-formed sequence".to_owned()
        } else {
            "a lone U+001B, which the clause's two shapes do not cover".to_owned()
        });
    }
    if !sightings.decodes.is_empty() {
        sightings.decodes.sort();
        sightings.decodes.dedup();
        answers.decode_array = Some(sightings.decodes.join(", "));
    }
    if !sightings.groups.is_empty() {
        sightings.groups.sort();
        sightings.groups.dedup();
        answers.group_subtype = Some(sightings.groups.join(", "));
    }
    if !sightings.codec_masks.is_empty() {
        sightings.codec_masks.sort();
        sightings.codec_masks.dedup();
        answers.codec_mask = Some(sightings.codec_masks.join(", "));
    }
    if !sightings.deferred_codec_masks.is_empty() {
        sightings.deferred_codec_masks.sort();
        sightings.deferred_codec_masks.dedup();
        answers.codec_mask_deferred = Some(sightings.deferred_codec_masks.join(", "));
    }

    // §12.3.2.2's named destinations, asked once each at their definition rather than at every
    // link that names one: §12.3.2.4 puts them in the catalog's `/Dests` dictionary and in the
    // name dictionary's `/Dests` tree, and `Destination::read` walks the second per lookup.
    let mut destinations = sightings.numbered_destinations;
    if let Ok(catalog) = document.catalog() {
        let mut defined: Vec<Object> = Vec::new();
        let dests = document.get_key(&catalog, "Dests");
        if let Some(table) = dests.as_dict() {
            defined.extend(table.iter().map(|(_, value)| value.clone()));
        }
        let names = document.get_key(&catalog, "Names");
        if let Some(names) = names.as_dict() {
            let tree = document.get_key(names, "Dests");
            if let Some(tree) = tree.as_dict() {
                let pairs = pdf_syntax::tree::name_pairs(tree, &|object| document.resolve(object));
                defined.extend(pairs.into_iter().map(|(_, value)| value));
            }
        }
        for entry in &defined {
            if let Some(destination) = pdf_model::destination::Destination::read(&document, entry)
                && matches!(
                    destination.target,
                    pdf_model::destination::Target::Number(_)
                )
            {
                destinations.push("in a §12.3.2.4 destination table".to_owned());
            }
        }
    }
    if !destinations.is_empty() {
        destinations.sort();
        destinations.dedup();
        answers.numbered_destination = Some(destinations.join(", "));
    }

    // §12.4.2's ranges. The claim is about the clause's worked example, which states three:
    // lowercase Roman, decimal, and decimal again with a `/P` prefix and an `/St`. So the
    // population is a document stating at least that many ranges, and the answer says whether
    // the three shapes are all present — a count of ranges alone would not settle it.
    if let Ok(catalog) = document.catalog() {
        let entry = document.get_key(&catalog, "PageLabels");
        if let Some(root) = entry.as_dict() {
            let ranges = pdf_syntax::tree::number_pairs(root, &|object| document.resolve(object));
            if ranges.len() >= 3 {
                let mut styles: Vec<String> = Vec::new();
                let mut roman = false;
                let mut decimal = false;
                let mut prefixed_with_start = false;
                for (_, value) in &ranges {
                    let Some(range) = value.as_dict() else {
                        continue;
                    };
                    let style = document.get_key(range, "S");
                    let style = style
                        .as_name()
                        .map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned());
                    match style.as_deref() {
                        Some("r" | "R") => roman = true,
                        Some("D") => decimal = true,
                        _ => {}
                    }
                    if document.get_key(range, "P").as_string().is_some()
                        && document.get_key(range, "St").as_integer().is_some()
                    {
                        prefixed_with_start = true;
                    }
                    styles.push(
                        style.map_or_else(|| "(no /S)".to_owned(), |name| format!("/{name}")),
                    );
                }
                styles.sort();
                styles.dedup();
                let all_three = roman && decimal && prefixed_with_start;
                answers.label_ranges = Some(format!(
                    "{} range(s), {} — the example's three shapes: {}",
                    ranges.len(),
                    styles.join(" "),
                    if all_three { "all present" } else { "not all" }
                ));
            }
        }
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
