//! Which documents carry an ECMAScript, where it hangs, and what it calls — the census RFC 0008
//! draws its line on.
//!
//! §12.6.4.17 makes a script's *dispatch* the standard's and its *contents* ISO 21757-1's, and a
//! decision about which contents to support has to rest on what the world's scripts actually do.
//! So this walk asks three questions of every document it can open, and none of them needs an
//! engine:
//!
//! 1. **Does it carry a script at all?** Any `/S /JavaScript` action, anywhere the object table
//!    reaches, plus §12.6.4.14's rendition action carrying Table 218's `/JS`.
//! 2. **At which sites?** Table 32's `/JavaScript` name tree, the catalog's `/OpenAction` and
//!    Table 200's five document events, Table 198's two page events, Table 197's ten annotation
//!    events, Table 199's four field events, an annotation's or an outline item's own `/A`, and a
//!    rendition action's `/JS`. A `/Next` chain is followed from each, bounded.
//! 3. **What does the script call?** A tokeniser — not a parser, and never an engine — over the
//!    script text: an identifier chain followed by `(` is a call (`AFNumber_Format`,
//!    `app.alert`, `this.getField`), a chain rooted at a host object is a property use
//!    (`event.value`), and a `.name(` after a `)` or `]` is a method on a result
//!    (`.setFocus`). Comments and string literals are skipped; regular-expression literals are
//!    not recognised, which over-counts nothing this census ranks.
//!
//! **What is reported is the number the line is drawn on**: of the documents that carry a script,
//! the share whose every call is inside a candidate core — the `AF*` form library, the field and
//! document read-write surface, `app.alert`, `util`, `color`, `event` and ECMAScript's own
//! built-ins — against the share that calls something the RFC's Tier 2 excludes (the network, the
//! filesystem, other documents, the application's menus and dialogs, privileged functions). A
//! name in neither list is reported by name, so the long tail is counted rather than assumed.
//!
//! ```sh
//! find -L doc/pdf.js/test/pdfs -maxdepth 1 -name '*.pdf' > /tmp/paths
//! find -L doc/corpora corpus-cache -name '*.pdf' >> /tmp/paths
//! cargo run --release -p pdf-model --example javascript_census -- @/tmp/paths
//! ```
//!
//! An argument beginning with `@` names a file of paths, one to a line; a directory is walked
//! recursively, following symbolic links as `is_dir` does. Every document is read in parallel,
//! with a panic counted rather than fatal: a census over a crawl of hostile files that dies on
//! one of them measures nothing.

#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "a measurement whose output is its purpose"
)]

use std::collections::{BTreeMap, BTreeSet};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};

use pdf_syntax::{Dictionary, Document, Object, ObjectId, text_string, tree};
use rayon::prelude::*;

/// How many nodes of one indirect object's body are walked before the census gives up on it.
///
/// `pdf-syntax` bounds parse depth rather than the *breadth* an object's body can have, and a
/// file built to make a reader loop is part of every crawl.
const MAX_NODES: usize = 100_000;

/// How many actions a `/Next` chain is followed through from one site.
const MAX_CHAIN: usize = 64;

/// How many bytes of one script the tokeniser reads. Real form scripts are hundreds of bytes;
/// a document-level library is tens of kilobytes; anything past this is a script to look at.
const MAX_SCRIPT_BYTES: usize = 1 << 20;

/// The largest file this census reads whole.
///
/// The Tika crawl holds a six-gigabyte fuzzing artefact and the `SafeDocs` crawl fifteen documents
/// past this bound; four threads reading such files into memory beside their parsed objects is
/// what stopped the first run at the walk's data limit. A file past the bound is *counted* — per
/// corpus, so the report says what it did not read — rather than read, because a census that dies
/// on one document measures nothing (trap 38: the bound is stated where it is applied).
const MAX_FILE_BYTES: u64 = 128 << 20;

/// How many names the report prints in its ranked lists.
const TOP: usize = 60;

/// How many witness files are named beside an excluded call.
const WITNESSES: usize = 8;

/// The corpora the population is split by, longest prefix first so that `doc/corpora-own` (if a
/// tree ever has one) cannot be counted under `doc/corpora`.
const CORPORA: [(&str, &str); 5] = [
    ("corpus-cache/tika-issue-tracker/", "Tika issue tracker"),
    ("corpus-cache/openpreserve/", "openpreserve"),
    ("corpus-cache/safedocs/", "SafeDocs crawl"),
    ("doc/pdf.js/test/pdfs/", "pdf.js test files"),
    ("doc/corpora/", "doc/corpora"),
];

/// The host objects a property chain is recorded under when it is not a call.
const HOST_ROOTS: &[&str] = &[
    "event", "this", "app", "util", "color", "global", "Net", "SOAP", "Collab", "security",
    "console", "identity", "display", "border",
];

/// Calls RFC 0008's Tier 2 excludes, by exact chain, and the reason each is outside.
///
/// The list is the RFC's rather than Adobe's privileged-context marker: Adobe gates `app.openDoc`
/// and `this.submitForm` on nothing, and this project's line is the process boundary, which is
/// stricter — anything that leaves the process, reaches the filesystem, another document, the
/// application's own chrome, or persists across documents.
const TIER_2: &[(&str, &str)] = &[
    ("Net.HTTP.request", "network"),
    ("Net.Discovery", "network"),
    ("SOAP.connect", "network"),
    ("SOAP.request", "network"),
    ("SOAP.response", "network"),
    ("SOAP.queryServices", "network"),
    ("SOAP.resolveService", "network"),
    (
        "app.launchURL",
        "network (a §12.6.4.8 URI action is the sanctioned route)",
    ),
    (
        "this.submitForm",
        "network (a §12.7.6.2 submit-form action is the sanctioned route)",
    ),
    (
        "submitForm",
        "network (a §12.7.6.2 submit-form action is the sanctioned route)",
    ),
    ("this.getURL", "network"),
    ("this.mailDoc", "mail"),
    ("this.mailForm", "mail"),
    ("mailDoc", "mail"),
    ("mailForm", "mail"),
    ("app.mailMsg", "mail"),
    ("app.mailGetAddrs", "mail"),
    ("this.saveAs", "filesystem"),
    ("saveAs", "filesystem"),
    ("this.exportDataObject", "filesystem"),
    ("this.importDataObject", "filesystem"),
    ("this.createDataObject", "filesystem"),
    ("this.exportAsFDF", "filesystem"),
    ("this.exportAsXFDF", "filesystem"),
    ("this.exportAsText", "filesystem"),
    ("this.importAnFDF", "filesystem"),
    ("this.importAnXFDF", "filesystem"),
    ("this.importTextData", "filesystem"),
    ("this.importXFAData", "filesystem"),
    ("this.exportXFAData", "filesystem"),
    ("util.readFileIntoStream", "filesystem"),
    ("app.browseForDoc", "filesystem"),
    ("app.getPath", "filesystem"),
    ("app.openDoc", "another document"),
    ("app.newDoc", "another document"),
    ("app.newFDF", "another document"),
    ("app.openFDF", "another document"),
    ("this.closeDoc", "another document (closes this one)"),
    ("this.print", "printing"),
    ("print", "printing"),
    ("this.getPrintParams", "printing"),
    ("app.execMenuItem", "the application's chrome"),
    ("app.addMenuItem", "the application's chrome"),
    ("app.addSubMenu", "the application's chrome"),
    ("app.hideMenuItem", "the application's chrome"),
    ("app.addToolButton", "the application's chrome"),
    ("app.removeToolButton", "the application's chrome"),
    ("app.hideToolbarButton", "the application's chrome"),
    ("app.execDialog", "a dialog the document lays out"),
    (
        "app.findComponent",
        "installs a plug-in into the application",
    ),
    ("app.popUpMenu", "a menu the document lays out"),
    ("app.popUpMenuEx", "a menu the document lays out"),
    ("app.trustedFunction", "privileged"),
    ("app.trustPropagatorFunction", "privileged"),
    ("app.beginPriv", "privileged"),
    ("app.endPriv", "privileged"),
    ("global.setPersistent", "persistence across documents"),
    ("global.subscribe", "persistence across documents"),
    ("Collab.addStateModel", "collaboration"),
    ("Collab.removeStateModel", "collaboration"),
    ("Collab.documentToStream", "collaboration"),
    ("this.addScript", "mutates the document's scripts"),
    ("this.removeScript", "mutates the document's scripts"),
    (
        "this.addWatermarkFromText",
        "authoring, excluded by CLAUDE.md",
    ),
    (
        "this.addWatermarkFromFile",
        "authoring, excluded by CLAUDE.md",
    ),
    ("this.flattenPages", "mutates the document's structure"),
    (
        "this.spawnPageFromTemplate",
        "mutates the document's structure",
    ),
];

/// Method names that, called on a result (`getField(...).x(`), mean Tier 2 whatever the receiver.
const TIER_2_METHODS: &[&str] = &[
    "browseForFileToSubmit",
    "signatureSign",
    "signatureSetSeedValue",
    "buttonImportIcon",
    "setAction",
    "submitForm",
];

/// Calls RFC 0008's Tier 1 admits, by exact chain: field and document reads and writes through
/// the edit log, alerts, timers bounded to the document, navigation of this document, and `util`.
const TIER_1: &[&str] = &[
    "this.getField",
    "getField",
    "this.getNthFieldName",
    "this.calculateNow",
    "this.resetForm",
    "resetForm",
    "this.getAnnot",
    "this.getAnnots",
    "this.getPageLabel",
    "this.getPageBox",
    "this.getPageRotation",
    "this.getPageNumWords",
    "this.getPageNthWord",
    "this.getOCGs",
    "this.gotoNamedDest",
    "this.scroll",
    "this.getLinks",
    "this.getIcon",
    "this.getTemplate",
    "this.getDataObject",
    "app.alert",
    "app.beep",
    "app.response",
    "app.setTimeOut",
    "app.setInterval",
    "app.clearTimeOut",
    "app.clearInterval",
    "app.goBack",
    "app.goForward",
    "util.printf",
    "util.printd",
    "util.printx",
    "util.scand",
    "util.crackURL",
    "util.spansToXML",
    "util.xmlToSpans",
    "util.streamFromString",
    "util.stringFromStream",
    "color.convert",
    "color.equal",
    "console.println",
    "console.show",
    "console.clear",
    "this.syncAnnotScan",
    "syncAnnotScan",
];

/// ECMAScript's own global functions and constructors: calls the engine answers, not the host.
const BUILTINS: &[&str] = &[
    "parseInt",
    "parseFloat",
    "isNaN",
    "isFinite",
    "String",
    "Number",
    "Boolean",
    "Array",
    "Object",
    "Date",
    "RegExp",
    "Error",
    "Function",
    "Math",
    "JSON",
    "encodeURIComponent",
    "decodeURIComponent",
    "encodeURI",
    "decodeURI",
    "escape",
    "unescape",
    "eval",
    "Symbol",
    "Promise",
];

/// Language keywords counted per document, to say how much of ECMAScript real scripts use.
const KEYWORDS: &[&str] = &[
    "function",
    "var",
    "let",
    "const",
    "for",
    "while",
    "do",
    "if",
    "switch",
    "try",
    "catch",
    "new",
    "class",
    "typeof",
    "with",
    "return",
    "this",
    "throw",
    "instanceof",
    "in",
];

/// How one call was classified.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Kind {
    /// One of Acrobat's `AF*` form-format functions.
    Af,
    /// A Tier 1 host call.
    Tier1,
    /// A Tier 2 host call — excluded, with a reason.
    Tier2,
    /// ECMAScript's own.
    Builtin,
    /// A function the document itself defines.
    Defined,
    /// None of the above: the long tail, reported by name.
    Unknown,
}

/// What one document contributes.
#[derive(Default)]
struct Counts {
    /// Documents opened, per corpus.
    opened: BTreeMap<&'static str, usize>,
    /// Documents that did not open, per corpus.
    unopenable: BTreeMap<&'static str, usize>,
    /// Documents past [`MAX_FILE_BYTES`], not read, per corpus.
    too_large: BTreeMap<&'static str, usize>,
    /// Documents with at least one script, per corpus.
    scripted: BTreeMap<&'static str, usize>,
    /// Documents stating a `/S /JavaScript` dictionary anywhere, whether or not a site reached it.
    any_action: usize,
    /// Documents per site.
    sites: BTreeMap<&'static str, usize>,
    /// Distinct scripts per site.
    scripts_at: BTreeMap<&'static str, usize>,
    /// Documents naming each call chain, and occurrences.
    names: BTreeMap<String, (usize, usize)>,
    /// Documents naming each property chain rooted at a host object.
    properties: BTreeMap<String, usize>,
    /// Documents naming each Tier 2 chain, with witnesses.
    excluded: BTreeMap<&'static str, BTreeSet<String>>,
    /// Documents naming each unknown bare call.
    unknown: BTreeMap<String, usize>,
    /// Documents using each keyword.
    keywords: BTreeMap<&'static str, usize>,
    /// Documents whose every script is one `AF*` call and nothing else.
    single_af: usize,
    /// Documents whose every call is `AF*`, Tier 1, a built-in, a defined function or a method.
    core: usize,
    /// Documents calling at least one Tier 2 chain.
    tier_2: usize,
    /// Documents calling a Tier 2 chain that is refused outright — not one of the calls that
    /// become a request under an existing policy (`launchURL`, `submitForm`) or wait on printing.
    tier_2_hard: usize,
    /// Scripted documents that are neither core nor Tier 2: an unknown name somewhere.
    tail: usize,
    /// Documents with a script stored as a stream rather than a string.
    as_stream: usize,
    /// Documents with a script the tokeniser cut at [`MAX_SCRIPT_BYTES`].
    truncated: usize,
    /// Documents using `eval`, an arrow function, `let`/`const` or `class` — ES2015 and later.
    modern: usize,
    /// Bytes of script text, distinct per document, summed.
    bytes: u64,
    /// The largest single script.
    largest: usize,
    /// The largest script's document.
    largest_in: String,
    /// Documents by how many distinct scripts they carry: 1, 2–5, 6–20, more.
    by_count: [usize; 4],
}

impl Counts {
    fn absorb(&mut self, other: Self) {
        for (key, n) in other.opened {
            add(&mut self.opened, key, n);
        }
        for (key, n) in other.unopenable {
            add(&mut self.unopenable, key, n);
        }
        for (key, n) in other.too_large {
            add(&mut self.too_large, key, n);
        }
        for (key, n) in other.scripted {
            add(&mut self.scripted, key, n);
        }
        self.any_action = self.any_action.saturating_add(other.any_action);
        for (key, n) in other.sites {
            add(&mut self.sites, key, n);
        }
        for (key, n) in other.scripts_at {
            add(&mut self.scripts_at, key, n);
        }
        for (key, (docs, uses)) in other.names {
            let entry = self.names.entry(key).or_default();
            entry.0 = entry.0.saturating_add(docs);
            entry.1 = entry.1.saturating_add(uses);
        }
        for (key, n) in other.properties {
            let entry = self.properties.entry(key).or_default();
            *entry = entry.saturating_add(n);
        }
        for (key, files) in other.excluded {
            self.excluded.entry(key).or_default().extend(files);
        }
        for (key, n) in other.unknown {
            let entry = self.unknown.entry(key).or_default();
            *entry = entry.saturating_add(n);
        }
        for (key, n) in other.keywords {
            add(&mut self.keywords, key, n);
        }
        self.single_af = self.single_af.saturating_add(other.single_af);
        self.core = self.core.saturating_add(other.core);
        self.tier_2 = self.tier_2.saturating_add(other.tier_2);
        self.tier_2_hard = self.tier_2_hard.saturating_add(other.tier_2_hard);
        self.tail = self.tail.saturating_add(other.tail);
        self.as_stream = self.as_stream.saturating_add(other.as_stream);
        self.truncated = self.truncated.saturating_add(other.truncated);
        self.modern = self.modern.saturating_add(other.modern);
        self.bytes = self.bytes.saturating_add(other.bytes);
        if other.largest > self.largest {
            self.largest = other.largest;
            self.largest_in = other.largest_in;
        }
        for (mine, theirs) in self.by_count.iter_mut().zip(other.by_count) {
            *mine = mine.saturating_add(theirs);
        }
    }
}

/// Adds `by` to one tally, saturating.
fn add(counts: &mut BTreeMap<&'static str, usize>, key: &'static str, by: usize) {
    let entry = counts.entry(key).or_default();
    *entry = entry.saturating_add(by);
}

/// Which corpus a path belongs to.
fn corpus_of(path: &str) -> &'static str {
    CORPORA
        .iter()
        .find(|(prefix, _)| path.contains(prefix))
        .map_or("other", |(_, name)| name)
}

/// Every `.pdf` under one path, recursively.
fn collect(path: &Path, into: &mut Vec<PathBuf>) {
    if path.is_file() {
        into.push(path.to_path_buf());
        return;
    }
    let Ok(entries) = std::fs::read_dir(path) else {
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

/// The population, with `@file` expanded to the paths it lists and directories walked.
fn population() -> Vec<PathBuf> {
    let mut files = Vec::new();
    for argument in std::env::args().skip(1) {
        if let Some(list) = argument.strip_prefix('@') {
            match std::fs::read_to_string(list) {
                Ok(text) => {
                    for line in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
                        files.push(PathBuf::from(line));
                    }
                }
                Err(error) => eprintln!("{list}: {error}"),
            }
        } else {
            collect(Path::new(&argument), &mut files);
        }
    }
    files.sort();
    files.dedup();
    files
}

/// One script found at one site.
struct Found {
    /// The site, in the vocabulary the report prints.
    site: &'static str,
    /// The script's text.
    text: String,
    /// Whether it was stored as a stream.
    stream: bool,
    /// Whether the text was cut at [`MAX_SCRIPT_BYTES`].
    truncated: bool,
}

/// The text of Table 221's `/JS` (or Table 218's), from a string or a stream.
fn script_text(document: &Document, js: &Object) -> Option<(String, bool, bool)> {
    let (bytes, stream): (Vec<u8>, bool) = match document.resolve(js) {
        Object::String(bytes) => (bytes.to_vec(), false),
        Object::Stream(stream) => (document.decoded_stream_data(&stream)?.to_vec(), true),
        _ => return None,
    };
    let truncated = bytes.len() > MAX_SCRIPT_BYTES;
    let bytes = if truncated {
        &bytes[..MAX_SCRIPT_BYTES]
    } else {
        &bytes[..]
    };
    Some((text_string(bytes), stream, truncated))
}

/// Follows one action entry and its `/Next` chain, collecting every script on it.
fn scripts_from(document: &Document, entry: &Object, site: &'static str, into: &mut Vec<Found>) {
    let mut pending = vec![entry.clone()];
    let mut seen_ids = BTreeSet::new();
    let mut steps = 0_usize;
    while let Some(object) = pending.pop() {
        steps = steps.saturating_add(1);
        if steps > MAX_CHAIN {
            break;
        }
        if let Object::Reference(id) = &object
            && !seen_ids.insert(*id)
        {
            continue;
        }
        let resolved = document.resolve(&object);
        match resolved {
            Object::Array(items) => pending.extend(items),
            Object::Dictionary(dict) => {
                push_script(document, &dict, site, into);
                if let Some(next) = dict.get("Next") {
                    pending.push(next.clone());
                }
            }
            _ => {}
        }
    }
}

/// Records the script one action dictionary carries, if it is a script action.
fn push_script(document: &Document, dict: &Dictionary, site: &'static str, into: &mut Vec<Found>) {
    let kind = document.get_key(dict, "S");
    let Some(kind) = kind.as_name() else {
        return;
    };
    let site = match kind.as_bytes() {
        b"JavaScript" => site,
        b"Rendition" => "rendition action /JS (Table 218)",
        _ => return,
    };
    let Some(js) = dict.get("JS") else {
        return;
    };
    if let Some((text, stream, truncated)) = script_text(document, js) {
        into.push(Found {
            site,
            text,
            stream,
            truncated,
        });
    }
}

/// The site label for one additional-actions key, by the dictionary that holds the `/AA`.
fn aa_site(container: &Dictionary, document: &Document, key: &[u8]) -> &'static str {
    let type_name = document.get_key(container, "Type");
    let type_name = type_name.as_name().map(|n| n.as_bytes().to_vec());
    if type_name.as_deref() == Some(b"Catalog") {
        return match key {
            b"WC" => "catalog /AA /WC, will close (Table 200)",
            b"WS" => "catalog /AA /WS, will save (Table 200)",
            b"DS" => "catalog /AA /DS, did save (Table 200)",
            b"WP" => "catalog /AA /WP, will print (Table 200)",
            b"DP" => "catalog /AA /DP, did print (Table 200)",
            _ => "catalog /AA, a key Table 200 does not define",
        };
    }
    if type_name.as_deref() == Some(b"Page") {
        return match key {
            b"O" => "page /AA /O, open (Table 198)",
            b"C" => "page /AA /C, close (Table 198)",
            _ => "page /AA, a key Table 198 does not define",
        };
    }
    // A widget, a bare field, or a non-widget annotation. Table 199's four keys belong to a
    // field wherever the dictionary that states them sits; Table 197's ten to an annotation.
    match key {
        b"K" => "field /AA /K, keystroke (Table 199)",
        b"F" => "field /AA /F, format (Table 199)",
        b"V" => "field /AA /V, validate (Table 199)",
        b"C" => "field /AA /C, calculate (Table 199)",
        b"E" => "annotation /AA /E, enter (Table 197)",
        b"X" => "annotation /AA /X, exit (Table 197)",
        b"D" => "annotation /AA /D, down (Table 197)",
        b"U" => "annotation /AA /U, up (Table 197)",
        b"Fo" => "annotation /AA /Fo, focus (Table 197)",
        b"Bl" => "annotation /AA /Bl, blur (Table 197)",
        b"PO" => "annotation /AA /PO, page open (Table 197)",
        b"PC" => "annotation /AA /PC, page close (Table 197)",
        b"PV" => "annotation /AA /PV, page visible (Table 197)",
        b"PI" => "annotation /AA /PI, page invisible (Table 197)",
        _ => "/AA, a key no table defines",
    }
}

/// The site label for a dictionary's own `/A`.
fn a_site(container: &Dictionary, document: &Document) -> &'static str {
    if container.get("Title").is_some() {
        return "outline item /A";
    }
    let subtype = document.get_key(container, "Subtype");
    match subtype.as_name().map(pdf_syntax::Name::as_bytes) {
        Some(b"Link") => "link annotation /A",
        Some(b"Widget") => "widget annotation /A",
        Some(b"Screen") => "screen annotation /A",
        Some(_) => "annotation /A (another subtype)",
        None => "/A on a dictionary that is neither annotation nor outline item",
    }
}

/// Every script one document carries, with its site.
fn scripts_in(document: &Document) -> (Vec<Found>, bool) {
    let mut found = Vec::new();
    let mut any_action = false;

    // Table 32's name tree, from the catalog: the one site a walk of numbered objects would
    // reach only as a bare dictionary with no site.
    if let Ok(catalog) = document.catalog() {
        let names = document.get_key(&catalog, "Names");
        if let Some(names) = names.as_dict() {
            let scripts = document.get_key(names, "JavaScript");
            if let Some(root) = scripts.as_dict() {
                let resolve = |object: &Object| document.resolve(object);
                for (_, value) in tree::name_pairs(root, &resolve) {
                    scripts_from(
                        document,
                        &value,
                        "name tree /JavaScript, document level (Table 32)",
                        &mut found,
                    );
                }
            }
        }
    }

    let numbers: Vec<u32> = document.xref().object_numbers().collect();
    for number in numbers {
        let object = document.get(ObjectId::new(number, 0));
        // Only the object's own body, never through a `Reference`: what a reference names is
        // another numbered object with its own turn in this loop. Actions themselves are followed
        // through references by `scripts_from`, because the site is the container's and the
        // script may well be a shared indirect object.
        let mut pending = vec![object];
        let mut depth = 0_usize;
        while let Some(object) = pending.pop() {
            depth = depth.saturating_add(1);
            if depth > MAX_NODES {
                break;
            }
            match object {
                Object::Array(items) => pending.extend(items),
                Object::Dictionary(dict) => {
                    visit(document, &dict, &mut found, &mut any_action);
                    pending.extend(dict.iter().map(|(_, value)| value.clone()));
                }
                Object::Stream(stream) => {
                    visit(document, &stream.dict, &mut found, &mut any_action);
                    pending.extend(stream.dict.iter().map(|(_, value)| value.clone()));
                }
                _ => {}
            }
        }
    }
    (found, any_action)
}

/// What one dictionary states: an `/AA`, an `/A`, an `/OpenAction`, or a script action itself.
fn visit(document: &Document, dict: &Dictionary, found: &mut Vec<Found>, any_action: &mut bool) {
    let stated = document.get_key(dict, "S");
    if let Some(name) = stated.as_name()
        && name.as_bytes() == b"JavaScript"
        && dict.get("JS").is_some()
    {
        *any_action = true;
    }
    if let Some(aa) = dict.get("AA") {
        let aa = document.resolve(aa);
        if let Some(aa) = aa.as_dict() {
            for (key, value) in aa.iter() {
                let site = aa_site(dict, document, key.as_bytes());
                scripts_from(document, value, site, found);
            }
        }
    }
    if let Some(a) = dict.get("A") {
        let site = a_site(dict, document);
        scripts_from(document, a, site, found);
    }
    if let Some(open) = dict.get("OpenAction") {
        let type_name = document.get_key(dict, "Type");
        if type_name
            .as_name()
            .is_some_and(|n| n.as_bytes() == b"Catalog")
        {
            scripts_from(document, open, "catalog /OpenAction (Table 29)", found);
        }
    }
}

/// What the tokeniser found in one script.
#[derive(Default)]
struct Tokens {
    /// Call chains, with occurrences.
    calls: BTreeMap<String, usize>,
    /// Methods called on a result, with occurrences.
    methods: BTreeMap<String, usize>,
    /// Property chains rooted at a host object.
    properties: BTreeSet<String>,
    /// Functions the script defines.
    defined: BTreeSet<String>,
    /// Keywords used.
    keywords: BTreeSet<&'static str>,
    /// An arrow function was seen.
    arrow: bool,
}

/// One lexical token the census cares about.
#[derive(Clone, PartialEq, Eq, Debug)]
enum Token {
    /// An identifier or keyword.
    Ident(String),
    /// One punctuation character.
    Punct(char),
}

/// A lexer good enough to find identifier chains and the punctuation around them.
///
/// Comments and string literals are skipped. A regular-expression literal is not recognised —
/// its body is lexed as code — which can add a spurious identifier or two and never removes a
/// real call, so it over-counts nothing this census ranks.
fn lex(text: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0_usize;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i.saturating_add(1)).copied();
        if c == '/' && next == Some('/') {
            while i < chars.len() && chars[i] != '\n' {
                i = i.saturating_add(1);
            }
        } else if c == '/' && next == Some('*') {
            i = i.saturating_add(2);
            while i < chars.len()
                && !(chars[i] == '*' && chars.get(i.saturating_add(1)) == Some(&'/'))
            {
                i = i.saturating_add(1);
            }
            i = i.saturating_add(2);
        } else if c == '"' || c == '\'' || c == '`' {
            i = i.saturating_add(1);
            while i < chars.len() && chars[i] != c {
                if chars[i] == '\\' {
                    i = i.saturating_add(1);
                }
                i = i.saturating_add(1);
            }
            i = i.saturating_add(1);
        } else if c.is_alphabetic() || c == '_' || c == '$' {
            let start = i;
            while i < chars.len()
                && (chars[i].is_alphanumeric() || chars[i] == '_' || chars[i] == '$')
            {
                i = i.saturating_add(1);
            }
            tokens.push(Token::Ident(chars[start..i].iter().collect()));
        } else if c.is_whitespace() {
            i = i.saturating_add(1);
        } else {
            tokens.push(Token::Punct(c));
            i = i.saturating_add(1);
        }
    }
    tokens
}

/// The call chains, property chains, definitions and keywords of one script.
fn tokenise(text: &str) -> Tokens {
    let tokens = lex(text);
    let mut out = Tokens::default();
    let mut i = 0_usize;
    while i < tokens.len() {
        let Token::Ident(first) = &tokens[i] else {
            if tokens[i] == Token::Punct('=')
                && tokens.get(i.saturating_add(1)) == Some(&Token::Punct('>'))
            {
                out.arrow = true;
            }
            // `).name(` and `].name(`: a method on a result.
            if matches!(tokens[i], Token::Punct(')' | ']'))
                && tokens.get(i.saturating_add(1)) == Some(&Token::Punct('.'))
                && let Some(Token::Ident(name)) = tokens.get(i.saturating_add(2))
                && tokens.get(i.saturating_add(3)) == Some(&Token::Punct('('))
            {
                let entry = out.methods.entry(name.clone()).or_default();
                *entry = entry.saturating_add(1);
            }
            i = i.saturating_add(1);
            continue;
        };
        if let Some(keyword) = KEYWORDS.iter().find(|k| **k == first.as_str()) {
            let keyword: &'static str = keyword;
            out.keywords.insert(keyword);
            if keyword == "function"
                && let Some(Token::Ident(name)) = tokens.get(i.saturating_add(1))
            {
                out.defined.insert(name.clone());
            }
            if keyword != "this" {
                i = i.saturating_add(1);
                continue;
            }
        }
        // The chain: ident (. ident)*
        let mut chain = vec![first.clone()];
        let mut j = i.saturating_add(1);
        while tokens.get(j) == Some(&Token::Punct('.'))
            && let Some(Token::Ident(name)) = tokens.get(j.saturating_add(1))
        {
            chain.push(name.clone());
            j = j.saturating_add(2);
        }
        let name = chain.join(".");
        if tokens.get(j) == Some(&Token::Punct('(')) {
            match normalise(&name, &chain) {
                Call::Host(name) => {
                    let entry = out.calls.entry(name).or_default();
                    *entry = entry.saturating_add(1);
                }
                Call::Method(name) => {
                    let entry = out.methods.entry(name).or_default();
                    *entry = entry.saturating_add(1);
                }
            }
        } else if chain.len() > 1 && HOST_ROOTS.contains(&chain[0].as_str()) {
            out.properties.insert(name);
        }
        i = j;
    }
    out
}

/// A call, once its receiver is read off the chain.
enum Call {
    /// A chain rooted at a host object, a built-in, or a bare name.
    Host(String),
    /// A method whose receiver is a value the tokeniser cannot see: a local, a field, a result.
    Method(String),
}

/// Reads a call chain's receiver, so that `event.target.setFocus(` and `f.setFocus(` are one
/// method and `event.target.doc.resetForm(` is `this.resetForm(`.
///
/// `event.target` and `event.source` are the field the trigger fired on, so a method on either is
/// a `Field` method and `.doc` beneath them is the `Doc`; a chain rooted at a name that is neither
/// a host object nor a built-in is a local variable, and what is called on it is a method.
fn normalise(name: &str, chain: &[String]) -> Call {
    for root in ["event.target.doc.", "event.source.doc."] {
        if let Some(rest) = name.strip_prefix(root) {
            return Call::Host(format!("this.{rest}"));
        }
    }
    for root in ["event.target.", "event.source."] {
        if let Some(rest) = name.strip_prefix(root)
            && !rest.contains('.')
        {
            return Call::Method(rest.to_owned());
        }
    }
    let root = chain[0].as_str();
    if chain.len() > 1 && !HOST_ROOTS.contains(&root) && !BUILTINS.contains(&root) {
        return Call::Method(chain[chain.len().saturating_sub(1)].clone());
    }
    Call::Host(name.to_owned())
}

/// How one call chain is classified.
fn classify(name: &str, defined: &BTreeSet<String>) -> Kind {
    if tier_2_entry(name).is_some() {
        return Kind::Tier2;
    }
    if name.starts_with("AF") && !name.contains('.') {
        return Kind::Af;
    }
    if TIER_1.contains(&name) {
        return Kind::Tier1;
    }
    let root = name.split('.').next().unwrap_or(name);
    if BUILTINS.contains(&root) {
        return Kind::Builtin;
    }
    if defined.contains(name) {
        return Kind::Defined;
    }
    // Anything left rooted at a host object is a host method outside both lists.
    Kind::Unknown
}

/// The Tier 2 row for a chain, if it is one.
fn tier_2_entry(name: &str) -> Option<(&'static str, &'static str)> {
    TIER_2.iter().copied().find(|(chain, _)| *chain == name)
}

/// Whether a Tier 2 reason is a refusal outright, rather than a request an existing policy
/// answers (a link, a submission) or an act that waits on printing.
fn refused_outright(why: &str) -> bool {
    !(why.contains("sanctioned route") || why.starts_with("printing"))
}

/// Whether a script is one `AF*` call and nothing else: what a native library could dispatch
/// with no engine at all.
fn is_single_af_call(text: &str) -> bool {
    let trimmed = text.trim().trim_end_matches(';').trim_end();
    if !trimmed.starts_with("AF") {
        return false;
    }
    let Some(open) = trimmed.find('(') else {
        return false;
    };
    let name = &trimmed[..open];
    if !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return false;
    }
    trimmed.ends_with(')') && !trimmed[open..].contains(';') && !trimmed.contains('\n')
}

/// Opens one document, or counts why it was not opened.
fn open(path: &Path, corpus: &'static str, counts: &mut Counts) -> Option<Document> {
    if std::fs::metadata(path).is_ok_and(|meta| meta.len() > MAX_FILE_BYTES) {
        add(&mut counts.too_large, corpus, 1);
        return None;
    }
    let Ok(bytes) = std::fs::read(path) else {
        add(&mut counts.unopenable, corpus, 1);
        return None;
    };
    let Ok(document) = Document::open(bytes) else {
        add(&mut counts.unopenable, corpus, 1);
        return None;
    };
    add(&mut counts.opened, corpus, 1);
    Some(document)
}

/// One document's counts.
fn measure(path: &Path) -> Counts {
    let mut counts = Counts::default();
    let name = path.to_string_lossy().into_owned();
    let corpus = corpus_of(&name);
    let Some(document) = open(path, corpus, &mut counts) else {
        return counts;
    };
    let (found, any_action) = scripts_in(&document);
    if any_action {
        counts.any_action = 1;
    }
    if found.is_empty() {
        return counts;
    }
    add(&mut counts.scripted, corpus, 1);

    // Distinct scripts, by content: a shared indirect action reached from ten widgets is one
    // script for the tokeniser and ten sites for the site table.
    let mut sites: BTreeSet<&'static str> = BTreeSet::new();
    let mut distinct: BTreeMap<u64, &Found> = BTreeMap::new();
    let mut per_site: BTreeMap<&'static str, BTreeSet<u64>> = BTreeMap::new();
    for item in &found {
        let mut hasher = DefaultHasher::new();
        item.text.hash(&mut hasher);
        let key = hasher.finish();
        sites.insert(item.site);
        distinct.entry(key).or_insert(item);
        per_site.entry(item.site).or_default().insert(key);
    }
    for site in &sites {
        add(&mut counts.sites, site, 1);
    }
    for (site, keys) in &per_site {
        add(&mut counts.scripts_at, site, keys.len());
    }
    let n = distinct.len();
    let bucket = match n {
        0 | 1 => 0,
        2..=5 => 1,
        6..=20 => 2,
        _ => 3,
    };
    counts.by_count[bucket] = 1;

    let mut all_calls: BTreeMap<String, usize> = BTreeMap::new();
    let mut all_methods: BTreeMap<String, usize> = BTreeMap::new();
    let mut properties: BTreeSet<String> = BTreeSet::new();
    let mut defined: BTreeSet<String> = BTreeSet::new();
    let mut keywords: BTreeSet<&'static str> = BTreeSet::new();
    let mut arrow = false;
    let mut every_single_af = true;
    for item in distinct.values() {
        let bytes = item.text.len();
        counts.bytes = counts
            .bytes
            .saturating_add(u64::try_from(bytes).unwrap_or(u64::MAX));
        if bytes > counts.largest {
            counts.largest = bytes;
            counts.largest_in.clone_from(&name);
        }
        if item.stream {
            counts.as_stream = 1;
        }
        if item.truncated {
            counts.truncated = 1;
        }
        if !is_single_af_call(&item.text) {
            every_single_af = false;
        }
        let tokens = tokenise(&item.text);
        for (call, n) in tokens.calls {
            let entry = all_calls.entry(call).or_default();
            *entry = entry.saturating_add(n);
        }
        for (method, n) in tokens.methods {
            let entry = all_methods.entry(method).or_default();
            *entry = entry.saturating_add(n);
        }
        properties.extend(tokens.properties);
        defined.extend(tokens.defined);
        keywords.extend(tokens.keywords);
        arrow |= tokens.arrow;
    }
    if every_single_af {
        counts.single_af = 1;
    }

    verdict(
        &mut counts,
        &name,
        &all_calls,
        &all_methods,
        properties,
        &keywords,
        &defined,
        arrow,
    );
    counts
}

/// The document's verdict — core, Tier 2, or the tail — from everything its scripts named.
#[expect(
    clippy::too_many_arguments,
    reason = "one document's tallies, gathered by `measure` and judged once"
)]
fn verdict(
    counts: &mut Counts,
    name: &str,
    all_calls: &BTreeMap<String, usize>,
    all_methods: &BTreeMap<String, usize>,
    properties: BTreeSet<String>,
    keywords: &BTreeSet<&'static str>,
    defined: &BTreeSet<String>,
    arrow: bool,
) {
    let mut has_tier_2 = false;
    let mut has_hard = false;
    let mut has_unknown = false;
    for (call, n) in all_calls {
        let entry = counts.names.entry(call.clone()).or_default();
        entry.0 = 1;
        entry.1 = *n;
        match classify(call, defined) {
            Kind::Tier2 => {
                has_tier_2 = true;
                if let Some((label, why)) = tier_2_entry(call) {
                    has_hard |= refused_outright(why);
                    counts
                        .excluded
                        .entry(label)
                        .or_default()
                        .insert(name.to_owned());
                }
            }
            Kind::Unknown => {
                has_unknown = true;
                counts.unknown.insert(call.clone(), 1);
            }
            Kind::Af | Kind::Tier1 | Kind::Builtin | Kind::Defined => {}
        }
    }
    for (method, n) in all_methods {
        let key = format!(".{method}");
        let entry = counts.names.entry(key).or_default();
        entry.0 = 1;
        entry.1 = *n;
        if let Some(label) = TIER_2_METHODS.iter().find(|m| **m == method.as_str()) {
            has_tier_2 = true;
            has_hard = true;
            counts
                .excluded
                .entry(label)
                .or_default()
                .insert(name.to_owned());
        }
    }
    for property in properties {
        counts.properties.insert(property, 1);
    }
    for keyword in keywords.iter().copied() {
        add(&mut counts.keywords, keyword, 1);
    }
    if arrow
        || all_calls.contains_key("eval")
        || counts.keywords.contains_key("let")
        || counts.keywords.contains_key("const")
        || counts.keywords.contains_key("class")
    {
        counts.modern = 1;
    }
    if has_hard {
        counts.tier_2_hard = 1;
    }
    if has_tier_2 {
        counts.tier_2 = 1;
    } else if has_unknown {
        counts.tail = 1;
    } else {
        counts.core = 1;
    }
}

/// Prints a percentage of `part` in `whole`, or a dash.
fn share(part: usize, whole: usize) -> String {
    if whole == 0 {
        return "—".to_owned();
    }
    // Precision loss past 2^52 documents is not a population this census will meet.
    #[expect(clippy::cast_precision_loss, reason = "a percentage for a report")]
    let value = 100.0 * part as f64 / whole as f64;
    format!("{value:.2}%")
}

fn main() {
    let files = population();
    eprintln!("{} PDF(s) in the population", files.len());
    let total = files
        .par_iter()
        .map(|path| {
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| measure(path))).unwrap_or_else(
                |_| {
                    let mut counts = Counts::default();
                    add(
                        &mut counts.unopenable,
                        corpus_of(&path.to_string_lossy()),
                        1,
                    );
                    counts
                },
            )
        })
        .reduce(Counts::default, |mut left, right| {
            left.absorb(right);
            left
        });

    report_population(&total);
    report_names(&total);
    report_tail(&total);
}

/// The population, the sites and the verdicts.
fn report_population(total: &Counts) {
    let opened: usize = total.opened.values().sum();
    let scripted: usize = total.scripted.values().sum();
    let unopenable: usize = total.unopenable.values().sum();
    let too_large: usize = total.too_large.values().sum();
    println!("== Population");
    println!(
        "{opened} document(s) opened, {unopenable} did not, {too_large} over {MAX_FILE_BYTES} \
         bytes were not read; {scripted} carry a script ({}); {} state a /S /JavaScript \
         dictionary somewhere",
        share(scripted, opened),
        total.any_action
    );
    let mut corpora: Vec<&'static str> = total.opened.keys().copied().collect();
    corpora.extend(total.unopenable.keys().copied());
    corpora.extend(total.too_large.keys().copied());
    corpora.sort_unstable();
    corpora.dedup();
    for corpus in corpora {
        let opened = total.opened.get(corpus).copied().unwrap_or(0);
        let scripted = total.scripted.get(corpus).copied().unwrap_or(0);
        println!(
            "  {corpus:<20} {opened:>6} opened, {:>5} unopenable, {:>3} too large, {scripted:>5} \
             scripted ({})",
            total.unopenable.get(corpus).copied().unwrap_or(0),
            total.too_large.get(corpus).copied().unwrap_or(0),
            share(scripted, opened)
        );
    }

    println!("\n== Sites (documents stating a script there; distinct scripts there)");
    let mut sites: Vec<(&&str, &usize)> = total.sites.iter().collect();
    sites.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    for (site, docs) in sites {
        println!(
            "  {docs:>5} documents  {:>6} scripts  {site}",
            total.scripts_at.get(site).copied().unwrap_or(0)
        );
    }

    println!(
        "\n== Scripts per document: 1: {}, 2–5: {}, 6–20: {}, more: {}",
        total.by_count[0], total.by_count[1], total.by_count[2], total.by_count[3]
    );
    println!(
        "   {} bytes of distinct script text; the largest is {} bytes in {}; {} document(s) store a script as a stream; {} had a script cut at {} bytes",
        total.bytes,
        total.largest,
        total.largest_in,
        total.as_stream,
        total.truncated,
        MAX_SCRIPT_BYTES
    );

    println!("\n== Verdicts over the {scripted} scripted documents");
    println!(
        "  {:>5} ({}) every script is one AF* call and nothing else — Tier 0 alone serves them",
        total.single_af,
        share(total.single_af, scripted)
    );
    println!(
        "  {:>5} ({}) every call is AF*, Tier 1, a built-in, a function the document defines, or a method on a result — inside the candidate core",
        total.core,
        share(total.core, scripted)
    );
    println!(
        "  {:>5} ({}) call at least one Tier 2 name",
        total.tier_2,
        share(total.tier_2, scripted)
    );
    println!(
        "  {:>5} ({}) of those call one refused outright — not launchURL, submitForm or a print, \
         which become a request under an existing policy",
        total.tier_2_hard,
        share(total.tier_2_hard, scripted)
    );
    println!(
        "  {:>5} ({}) call a name in neither list (the long tail below)",
        total.tail,
        share(total.tail, scripted)
    );
    println!(
        "  {:>5} ({}) use eval, an arrow function, let/const or class",
        total.modern,
        share(total.modern, scripted)
    );
}

/// The ranked call names and the host properties.
fn report_names(total: &Counts) {
    println!("\n== Top {TOP} call names (documents; occurrences; class)");
    let empty = BTreeSet::new();
    let mut names: Vec<(&String, &(usize, usize))> = total.names.iter().collect();
    names.sort_by(|a, b| b.1.0.cmp(&a.1.0).then(b.1.1.cmp(&a.1.1)).then(a.0.cmp(b.0)));
    for (name, (docs, uses)) in names.iter().take(TOP) {
        let class = if let Some(method) = name.strip_prefix('.') {
            if TIER_2_METHODS.contains(&method) {
                "tier 2 (method)"
            } else {
                "method on a result"
            }
        } else {
            match classify(name, &empty) {
                Kind::Af => "AF* (Tier 0)",
                Kind::Tier1 => "tier 1",
                Kind::Tier2 => "tier 2",
                Kind::Builtin => "ECMAScript built-in",
                Kind::Defined => "defined",
                Kind::Unknown => "not in either list",
            }
        };
        println!("  {docs:>5}  {uses:>7}  {name:<40} {class}");
    }
    println!("   ({} distinct call names in all)", total.names.len());

    println!("\n== Host properties read or written (documents)");
    let mut properties: Vec<(&String, &usize)> = total.properties.iter().collect();
    properties.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    for (name, docs) in properties.iter().take(40) {
        println!("  {docs:>5}  {name}");
    }
}

/// The excluded names with witnesses, the unknown tail and the keywords.
fn report_tail(total: &Counts) {
    println!("\n== Tier 2 names the corpus calls (documents; reason; witnesses)");
    let mut excluded: Vec<(&&str, &BTreeSet<String>)> = total.excluded.iter().collect();
    excluded.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(b.0)));
    if excluded.is_empty() {
        println!("  none");
    }
    for (name, files) in excluded {
        let why = tier_2_entry(name).map_or("a method Tier 2 excludes", |(_, why)| why);
        let mut shown: Vec<&str> = files.iter().map(String::as_str).collect();
        shown.sort_unstable();
        let cut = shown.len().min(WITNESSES);
        println!(
            "  {:>5}  {name:<32} {why} — {}{}",
            files.len(),
            shown[..cut]
                .iter()
                .map(|p| p.rsplit('/').next().unwrap_or(p))
                .collect::<Vec<_>>()
                .join(", "),
            if shown.len() > cut { ", …" } else { "" }
        );
    }

    println!("\n== Unknown bare or host calls, the long tail (documents)");
    let mut unknown: Vec<(&String, &usize)> = total.unknown.iter().collect();
    unknown.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    for (name, docs) in unknown.iter().take(TOP) {
        println!("  {docs:>5}  {name}");
    }
    println!("   ({} distinct unknown names in all)", total.unknown.len());

    println!("\n== Keywords (documents using each)");
    let mut keywords: Vec<(&&str, &usize)> = total.keywords.iter().collect();
    keywords.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    for (keyword, docs) in keywords {
        println!("  {docs:>5}  {keyword}");
    }
}
