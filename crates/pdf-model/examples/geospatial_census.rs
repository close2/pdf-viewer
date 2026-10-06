//! Which documents carry §12.10's geospatial measure, how they state its coordinate system, and
//! which projections and codes they name — the census `doc/questions/A171`'s build is scoped by.
//!
//! §12.10.3's Table 270 lets a file state its system two ways, and the two cost different builds:
//! a `/WKT` string carries the projection method and every parameter inline, so it needs a
//! grammar and the method's formulas and no registry; an `/EPSG` code names a system whose
//! parameters live in the IOGP's registry, so it needs a table carried from it. This walk answers
//! which of the two the world's files use, which Well Known Text dialect, which projection
//! methods, and which codes — over the crawl, with the curated corpora beside it as the control
//! (ADR 1586).
//!
//! # Two stages, and why the second is not skipped
//!
//! **A byte needle first.** Each file is scanned for `/GCS`, `/DCS`, `/WKT`, `/EPSG`,
//! `/Measure` and `/GEO` before anything is parsed, which is what makes a walk of a hundred
//! gigabytes cost minutes. **A needle is not an answer**, for two reasons, so the second stage
//! parses: a dictionary inside a §7.5.7 object stream is compressed and no needle sees it, so a
//! file stating `/ObjStm` is parsed whatever its needles say; and a needle hit is a name, while
//! the census counts *structures* — a dictionary with a `/GCS` entry whose value is a
//! dictionary, which is Table 269's own requirement of a geospatial measure (ADR 0405 is why a
//! census over names measures at the wrong granularity).
//!
//! `/LGIDict` is counted beside them and reported apart: it is the pre-ISO geospatial encoding of
//! Adobe's extension and the OGC's best practice, which ISO 32000-2 does not define, so a document
//! carrying only it is evidence about the world and not a §12.10 beneficiary.
//!
//! ```sh
//! find -L corpus-cache -name '*.pdf' > scratchpad/paths
//! find -L doc/pdf.js/test/pdfs -maxdepth 1 -name '*.pdf' >> scratchpad/paths
//! find -L doc/corpora -name '*.pdf' >> scratchpad/paths
//! cargo run --release -p pdf-model --example geospatial_census -- @scratchpad/paths
//! ```
//!
//! An argument beginning with `@` names a file of paths, one to a line; a directory is walked
//! recursively. Every document is read in parallel with a panic counted rather than fatal: a
//! census over a crawl of hostile files that dies on one of them measures nothing.

#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::arithmetic_side_effects,
    clippy::too_many_lines,
    reason = "a measurement whose output is its purpose: its sums count a finite population and its \
              offsets stay inside one file's bytes, and its report is one function read top to \
              bottom"
)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::Instant;

use pdf_syntax::{Dictionary, Document, Object, ObjectId};
use rayon::prelude::*;

/// How many nodes of one indirect object's body are walked before the census gives up on it,
/// because `pdf-syntax` bounds the depth an object's body can have and not its breadth.
const MAX_NODES: usize = 100_000;

/// The largest file read whole.
///
/// Larger than other censuses' bound on purpose: a georeferenced map is a large file, and the
/// population this census looks for is exactly the one a smaller bound would skip. A file past
/// it is counted per corpus rather than read (trap 38: the bound is stated where it is applied).
const MAX_FILE_BYTES: u64 = 512 << 20;

/// How many witness files are named beside each method and code.
const WITNESSES: usize = 3;

/// The corpora the population is split by; the crawl first, because it is the instrument.
const CORPORA: [(&str, &str); 5] = [
    ("corpus-cache/safedocs/", "SafeDocs crawl"),
    ("corpus-cache/tika-issue-tracker/", "Tika issue tracker"),
    ("corpus-cache/openpreserve/", "openpreserve"),
    ("doc/pdf.js/test/pdfs/", "pdf.js test files"),
    ("doc/corpora/", "doc/corpora"),
];

/// The byte needles, in the order the report prints them. The last two are not §12.10's.
const NEEDLES: [&[u8]; 8] = [
    b"/GCS",
    b"/DCS",
    b"/WKT",
    b"/EPSG",
    b"/Measure",
    b"/GEO",
    b"/LGIDict",
    b"/ObjStm",
];

/// What one document, or the whole population, comes to.
#[derive(Default)]
struct Counts {
    /// Files per corpus.
    files: BTreeMap<&'static str, usize>,
    /// Files over [`MAX_FILE_BYTES`], per corpus.
    too_large: BTreeMap<&'static str, usize>,
    /// Files that could not be read or parsed (or panicked), per corpus.
    unopenable: BTreeMap<&'static str, usize>,
    /// Files parsed, per corpus.
    parsed: BTreeMap<&'static str, usize>,
    /// Documents per needle per corpus.
    needles: BTreeMap<(&'static str, usize), usize>,
    /// Documents carrying a Table 269 geospatial measure, per corpus.
    geospatial: BTreeMap<&'static str, usize>,
    /// Documents carrying an `/LGIDict` dictionary, per corpus.
    lgi: BTreeMap<&'static str, usize>,
    /// Documents stating a system by `/WKT`, per corpus.
    by_wkt: BTreeMap<&'static str, usize>,
    /// Documents stating a system by `/EPSG`, per corpus.
    by_epsg: BTreeMap<&'static str, usize>,
    /// Documents with a system dictionary stating `/EPSG` and no `/WKT` — the only documents an
    /// EPSG table would serve, per corpus.
    epsg_alone: BTreeMap<&'static str, usize>,
    /// Documents with a system dictionary stating both, which Table 270 forbids of a `GEOGCS`.
    both: BTreeMap<&'static str, usize>,
    /// Distinct WKT strings, with the documents stating each.
    distinct: BTreeMap<String, usize>,
    /// Documents stating a `PROJCS` system, per corpus.
    projected: BTreeMap<&'static str, usize>,
    /// Documents per WKT dialect.
    dialects: BTreeMap<String, usize>,
    /// Documents per projection method, as the WKT names it.
    methods: BTreeMap<String, usize>,
    /// Documents per EPSG code, keyed by how it was stated.
    codes: BTreeMap<String, usize>,
    /// Documents per ellipsoid named in a WKT string.
    ellipsoids: BTreeMap<String, usize>,
    /// Documents per angular or linear unit named in a WKT string.
    units: BTreeMap<String, usize>,
    /// Documents per parameter name stated in a projected system's WKT.
    parameters: BTreeMap<String, usize>,
    /// Documents per reading of this tree's reader, over every `/GCS` a document states.
    readings: BTreeMap<String, usize>,
    /// Witness paths per method or code key.
    witnesses: BTreeMap<String, BTreeSet<String>>,
}

impl Counts {
    /// Adds another document's counts to these.
    fn absorb(&mut self, other: Self) {
        fn merge<K: Ord>(into: &mut BTreeMap<K, usize>, from: BTreeMap<K, usize>) {
            for (key, value) in from {
                *into.entry(key).or_insert(0) += value;
            }
        }
        merge(&mut self.files, other.files);
        merge(&mut self.too_large, other.too_large);
        merge(&mut self.unopenable, other.unopenable);
        merge(&mut self.parsed, other.parsed);
        merge(&mut self.needles, other.needles);
        merge(&mut self.geospatial, other.geospatial);
        merge(&mut self.lgi, other.lgi);
        merge(&mut self.by_wkt, other.by_wkt);
        merge(&mut self.by_epsg, other.by_epsg);
        merge(&mut self.epsg_alone, other.epsg_alone);
        merge(&mut self.both, other.both);
        merge(&mut self.distinct, other.distinct);
        merge(&mut self.readings, other.readings);
        merge(&mut self.projected, other.projected);
        merge(&mut self.dialects, other.dialects);
        merge(&mut self.methods, other.methods);
        merge(&mut self.codes, other.codes);
        merge(&mut self.ellipsoids, other.ellipsoids);
        merge(&mut self.units, other.units);
        merge(&mut self.parameters, other.parameters);
        for (key, paths) in other.witnesses {
            let into = self.witnesses.entry(key).or_default();
            for path in paths {
                if into.len() < WITNESSES {
                    into.insert(path);
                }
            }
        }
    }
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

/// Whether `haystack` holds `needle` anywhere — a plain search, with the first byte located by
/// `position` so that a file without a `/` near a candidate costs one comparison per byte.
fn holds(haystack: &[u8], needle: &[u8]) -> bool {
    let Some((&first, rest)) = needle.split_first() else {
        return true;
    };
    let mut from = 0_usize;
    while let Some(offset) = haystack
        .get(from..)
        .and_then(|tail| tail.iter().position(|&b| b == first))
    {
        let at = from + offset + 1;
        if haystack.get(at..at + rest.len()) == Some(rest) {
            return true;
        }
        from = at;
    }
    false
}

/// One coordinate system dictionary, as a census reads it.
struct System {
    /// Whether `/Type` is `PROJCS`.
    projected: bool,
    /// `/EPSG`.
    epsg: Option<i64>,
    /// `/WKT`.
    wkt: Option<String>,
}

/// What this tree's own reader makes of one `/GCS` and its `/GPTS`: the method it projects by
/// and whether every registration point came back as degrees, or the refusal it gave — the
/// census asked of `pdf_model::geospatial` itself, so the count of documents served is the
/// build's and not an estimate (ADR 1586).
fn reading(gcs: &System, points: Vec<[f64; 2]>) -> String {
    use pdf_model::geospatial::ReferenceSystem;
    use pdf_model::measurement::{CoordinateSystem, Geospatial};
    let system = CoordinateSystem {
        projected: gcs.projected,
        epsg: gcs.epsg,
        wkt: gcs.wkt.clone(),
    };
    let method = match system.reference_system() {
        Ok(ReferenceSystem::Projected(projected)) => projected.projection.method.name(),
        Ok(ReferenceSystem::Geographic(_)) => "a geographic system",
        Err(refusal) if !gcs.projected => {
            // A geographic `/GCS` needs no projection: Table 269 states its points in degrees.
            return format!("reads: GEOGCS, points as stated (its WKT refused: {refusal})");
        }
        Err(refusal) => return format!("refused: {refusal}"),
    };
    let count = points.len();
    let geospatial = Geospatial {
        coordinate_system: Some(system),
        local_points: points.clone(),
        geographic_points: points,
        ..Geospatial::default()
    };
    match geospatial.registration_geographic() {
        Ok(degrees)
            if degrees.iter().all(|(position, _)| {
                position.latitude.abs() <= 90.0 && position.longitude.abs() <= 180.0
            }) =>
        {
            format!(
                "reads: {method}, {} registration point(s) to degrees",
                if count == 0 { "no" } else { "every" }
            )
        }
        Ok(_) => format!("reads: {method}, a point outside the earth"),
        Err(refusal) => format!("refused at a point: {refusal}"),
    }
}

/// `/GPTS` taken pairwise, as Table 269 states it.
fn pairs(document: &Document, dict: &Dictionary) -> Vec<[f64; 2]> {
    let value = document.get_key(dict, "GPTS");
    let Some(array) = value.as_array() else {
        return Vec::new();
    };
    array
        .chunks_exact(2)
        .filter_map(|pair| {
            Some([
                document.resolve(pair.first()?).as_number()?,
                document.resolve(pair.get(1)?).as_number()?,
            ])
        })
        .collect()
}

/// Table 270's two entries and Table 271's `/Type`, from the dictionary a key names.
fn system(document: &Document, dict: &Dictionary, key: &str) -> Option<System> {
    let value = document.get_key(dict, key);
    let dict = value.as_dict()?;
    Some(System {
        projected: document
            .get_key(dict, "Type")
            .as_name()
            .is_some_and(|name| name.as_bytes() == b"PROJCS"),
        epsg: document.get_key(dict, "EPSG").as_integer(),
        wkt: match document.get_key(dict, "WKT") {
            Object::String(bytes) => Some(String::from_utf8_lossy(&bytes).into_owned()),
            _ => None,
        },
    })
}

/// The keyword a WKT string opens with, which is what tells the dialects apart: ISO 19162's
/// WKT2 spells a CRS `PROJCRS`, `GEOGCRS`, `GEODCRS` or their long forms, the older WKT1 spells
/// it `PROJCS`, `GEOGCS` or `GEOCCS`.
fn dialect(wkt: &str) -> String {
    let keyword: String = wkt
        .trim_start()
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect::<String>()
        .to_ascii_uppercase();
    let family = match keyword.as_str() {
        "PROJCS" | "GEOGCS" | "GEOCCS" | "COMPD_CS" | "LOCAL_CS" | "VERT_CS" => "WKT1",
        "PROJCRS" | "PROJECTEDCRS" | "GEOGCRS" | "GEOGRAPHICCRS" | "GEODCRS" | "GEODETICCRS"
        | "BOUNDCRS" | "COMPOUNDCRS" | "VERTCRS" | "ENGCRS" => "WKT2",
        _ => "neither",
    };
    format!("{family} {keyword}")
}

/// Every double-quoted string that follows one of `keywords` and an opening bracket.
fn quoted_after<'a>(wkt: &'a str, keywords: &[&str]) -> Vec<&'a str> {
    let upper = wkt.to_ascii_uppercase();
    let mut out = Vec::new();
    for keyword in keywords {
        let mut from = 0_usize;
        while let Some(offset) = upper.get(from..).and_then(|tail| tail.find(keyword)) {
            let start = from + offset;
            from = start + keyword.len();
            // A whole keyword: not the tail of a longer one (`DATUM` inside `VDATUM`).
            if start > 0
                && upper
                    .as_bytes()
                    .get(start - 1)
                    .is_some_and(|b| b.is_ascii_alphabetic() || *b == b'_')
            {
                continue;
            }
            let rest = wkt.get(from..).unwrap_or("").trim_start();
            let Some(rest) = rest.strip_prefix(['[', '(']) else {
                continue;
            };
            let Some(rest) = rest.trim_start().strip_prefix('"') else {
                continue;
            };
            if let Some(end) = rest.find('"') {
                out.push(&rest[..end]);
            }
        }
    }
    out
}

/// The outermost `AUTHORITY["EPSG","n"]` (WKT1) or `ID["EPSG",n]` (WKT2): the code of the system
/// itself, which both dialects write last, at the top level.
fn top_authority(wkt: &str) -> Option<String> {
    let upper = wkt.to_ascii_uppercase();
    let mut depth = 0_i32;
    let mut last = None;
    for (index, c) in upper.char_indices() {
        match c {
            '[' | '(' => {
                depth += 1;
                if depth == 2 {
                    let head = upper[..index].trim_end();
                    if head.ends_with("AUTHORITY") || head.ends_with(",ID") || head.ends_with(" ID")
                    {
                        let body: String = upper[index + 1..]
                            .chars()
                            .take_while(|c| *c != ']' && *c != ')')
                            .filter(|c| *c != '"')
                            .collect();
                        last = Some(body.replace(' ', ""));
                    }
                }
            }
            ']' | ')' => depth -= 1,
            _ => {}
        }
    }
    last
}

/// What one document's systems come to.
fn record(counts: &mut Counts, corpus: &'static str, path: &str, systems: &[(&str, System)]) {
    let mut dialects = BTreeSet::new();
    let mut methods = BTreeSet::new();
    let mut codes = BTreeSet::new();
    let mut ellipsoids = BTreeSet::new();
    let mut units = BTreeSet::new();
    let mut parameters = BTreeSet::new();
    let (mut wkt, mut epsg, mut projected) = (false, false, false);
    let (mut epsg_alone, mut both) = (false, false);
    let mut distinct = BTreeSet::new();
    for (role, system) in systems {
        let kind = if system.projected { "PROJCS" } else { "GEOGCS" };
        projected |= system.projected && *role == "GCS";
        epsg_alone |= system.epsg.is_some() && system.wkt.is_none();
        both |= system.epsg.is_some() && system.wkt.is_some();
        if let Some(code) = system.epsg {
            epsg = true;
            codes.insert(format!("/EPSG {code} ({role}, {kind})"));
        }
        if let Some(text) = &system.wkt {
            wkt = true;
            dialects.insert(dialect(text));
            distinct.insert(text.clone());
            for method in quoted_after(text, &["PROJECTION", "METHOD"]) {
                methods.insert(method.to_owned());
            }
            for name in quoted_after(text, &["SPHEROID", "ELLIPSOID"]) {
                ellipsoids.insert(name.to_owned());
            }
            for name in quoted_after(text, &["UNIT", "ANGLEUNIT", "LENGTHUNIT"]) {
                units.insert(name.to_owned());
            }
            for name in quoted_after(text, &["PARAMETER"]) {
                parameters.insert(name.to_owned());
            }
            if let Some(code) = top_authority(text) {
                codes.insert(format!("WKT {code} ({role}, {kind})"));
            }
        }
    }
    let bump = |map: &mut BTreeMap<&'static str, usize>, on: bool| {
        if on {
            *map.entry(corpus).or_insert(0) += 1;
        }
    };
    bump(&mut counts.by_wkt, wkt);
    bump(&mut counts.by_epsg, epsg);
    bump(&mut counts.epsg_alone, epsg_alone);
    bump(&mut counts.both, both);
    for text in distinct {
        *counts.distinct.entry(text).or_insert(0) += 1;
    }
    bump(&mut counts.projected, projected);
    for (map, keys) in [
        (&mut counts.dialects, dialects.clone()),
        (&mut counts.methods, methods.clone()),
        (&mut counts.codes, codes.clone()),
        (&mut counts.ellipsoids, ellipsoids),
        (&mut counts.units, units),
        (&mut counts.parameters, parameters),
    ] {
        for key in keys {
            *map.entry(key).or_insert(0) += 1;
        }
    }
    for key in methods.into_iter().chain(codes).chain(dialects) {
        counts
            .witnesses
            .entry(key)
            .or_default()
            .insert(path.to_owned());
    }
}

/// One document: the needles, then — where any could be hiding — the structures.
fn measure(path: &Path) -> Counts {
    let mut counts = Counts::default();
    let name = path.to_string_lossy().into_owned();
    let corpus = corpus_of(&name);
    counts.files.insert(corpus, 1);
    if std::fs::metadata(path).is_ok_and(|meta| meta.len() > MAX_FILE_BYTES) {
        counts.too_large.insert(corpus, 1);
        return counts;
    }
    let Ok(bytes) = std::fs::read(path) else {
        counts.unopenable.insert(corpus, 1);
        return counts;
    };
    let hits: Vec<bool> = NEEDLES.iter().map(|needle| holds(&bytes, needle)).collect();
    for (index, hit) in hits.iter().enumerate() {
        if *hit {
            counts.needles.insert((corpus, index), 1);
        }
    }
    // Any needle sends the file to the parse: each of the first seven is a geospatial name, and
    // `/ObjStm` is there because a needle cannot see into a compressed object stream.
    if !hits.iter().any(|hit| *hit) {
        return counts;
    }
    let Ok(document) = Document::open(bytes) else {
        counts.unopenable.insert(corpus, 1);
        return counts;
    };
    counts.parsed.insert(corpus, 1);

    let mut systems: Vec<(&str, System)> = Vec::new();
    let mut readings: BTreeSet<String> = BTreeSet::new();
    let mut geospatial = false;
    let mut lgi = false;
    let numbers: Vec<u32> = document.xref().object_numbers().collect();
    for number in numbers {
        // Only the object's own body, never through a `Reference`: what a reference names is
        // another numbered object with its own turn in this loop, so each dictionary is read once.
        let mut pending = vec![document.get(ObjectId::new(number, 0))];
        let mut nodes = 0_usize;
        while let Some(object) = pending.pop() {
            nodes += 1;
            if nodes > MAX_NODES {
                break;
            }
            let dict = match object {
                Object::Array(items) => {
                    pending.extend(items);
                    continue;
                }
                Object::Dictionary(dict) => dict,
                Object::Stream(stream) => stream.dict.clone(),
                _ => continue,
            };
            // Table 269 requires `/GCS` of a geospatial measure dictionary, and no other
            // dictionary in ISO 32000-2 has the key, so its presence as a dictionary is the
            // structure this census counts — whatever `/Subtype` the file wrote beside it.
            if let Some(gcs) = system(&document, &dict, "GCS") {
                geospatial = true;
                let points = pairs(&document, &dict);
                if gcs.projected {
                    // Table 269 states a projected system's `/GPTS` "as eastings and northings";
                    // a pair inside ±90 by ±180 is the shape of degrees instead, which is how a
                    // file stating them the other way is told apart (ADR 1586).
                    let degrees = !points.is_empty()
                        && points
                            .iter()
                            .all(|[a, b]| a.abs() <= 90.0 && b.abs() <= 180.0);
                    let matrix = dict.get("PCSM").is_some();
                    readings.insert(format!(
                        "projected /GCS: /GPTS {} degrees, {} /PCSM",
                        if degrees {
                            "shaped as"
                        } else {
                            "not shaped as"
                        },
                        if matrix { "with" } else { "no" }
                    ));
                }
                readings.insert(reading(&gcs, points));
                systems.push(("GCS", gcs));
                if let Some(dcs) = system(&document, &dict, "DCS") {
                    systems.push(("DCS", dcs));
                }
            }
            if dict.get("LGIDict").is_some() {
                lgi = true;
            }
            pending.extend(dict.iter().map(|(_, value)| value.clone()));
        }
    }
    if geospatial {
        counts.geospatial.insert(corpus, 1);
        record(&mut counts, corpus, &name, &systems);
        for key in readings {
            counts
                .witnesses
                .entry(key.clone())
                .or_default()
                .insert(name.clone());
            *counts.readings.entry(key).or_insert(0) += 1;
        }
    }
    if lgi {
        counts.lgi.insert(corpus, 1);
    }
    counts
}

/// A count from a per-corpus map.
fn of(map: &BTreeMap<&'static str, usize>, corpus: &str) -> usize {
    map.get(corpus).copied().unwrap_or(0)
}

/// One ranked table, every key printed: the long tail is what decides a registry.
fn ranked(
    title: &str,
    map: &BTreeMap<String, usize>,
    witnesses: &BTreeMap<String, BTreeSet<String>>,
) {
    println!("\n== {title}");
    if map.is_empty() {
        println!("  (none)");
    }
    let mut rows: Vec<(&String, &usize)> = map.iter().collect();
    rows.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    for (key, documents) in rows {
        println!("  {documents:>6}  {key}");
        for path in witnesses.get(key).into_iter().flatten() {
            println!("            {path}");
        }
    }
}

fn main() {
    let started = Instant::now();
    let files = population();
    eprintln!("{} PDF(s) in the population", files.len());
    let total = files
        .par_iter()
        .map(|path| {
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| measure(path))).unwrap_or_else(
                |_| {
                    let mut counts = Counts::default();
                    let corpus = corpus_of(&path.to_string_lossy());
                    counts.files.insert(corpus, 1);
                    counts.unopenable.insert(corpus, 1);
                    counts
                },
            )
        })
        .reduce(Counts::default, |mut left, right| {
            left.absorb(right);
            left
        });

    println!("== Population, needles and structures, per corpus");
    println!(
        "  {:<20} {:>6} {:>5} {:>5} {:>6} | {:>5} {:>5} {:>5} {:>5} {:>8} {:>5} {:>8} {:>7} | {:>4} {:>4} {:>5} {:>5} {:>4} {:>4} {:>4}",
        "corpus",
        "files",
        "large",
        "unop",
        "parsed",
        "GCS",
        "DCS",
        "WKT",
        "EPSG",
        "Measure",
        "GEO",
        "LGIDict",
        "ObjStm",
        "geo",
        "WKT",
        "EPSG",
        "alone",
        "both",
        "PROJ",
        "LGI"
    );
    for (_, corpus) in CORPORA.iter().chain(std::iter::once(&("", "other"))) {
        if of(&total.files, corpus) == 0 {
            continue;
        }
        let needle = |index: usize| total.needles.get(&(*corpus, index)).copied().unwrap_or(0);
        println!(
            "  {corpus:<20} {:>6} {:>5} {:>5} {:>6} | {:>5} {:>5} {:>5} {:>5} {:>8} {:>5} {:>8} {:>7} | {:>4} {:>4} {:>5} {:>5} {:>4} {:>4} {:>4}",
            of(&total.files, corpus),
            of(&total.too_large, corpus),
            of(&total.unopenable, corpus),
            of(&total.parsed, corpus),
            needle(0),
            needle(1),
            needle(2),
            needle(3),
            needle(4),
            needle(5),
            needle(6),
            needle(7),
            of(&total.geospatial, corpus),
            of(&total.by_wkt, corpus),
            of(&total.by_epsg, corpus),
            of(&total.epsg_alone, corpus),
            of(&total.both, corpus),
            of(&total.projected, corpus),
            of(&total.lgi, corpus),
        );
    }
    println!(
        "  (needle columns: documents whose bytes hold the name; structure columns: documents \
         with a Table 269 measure (`geo`), stating a system by /WKT or /EPSG, with a projected \
         /GCS (`PROJ`), with an /EPSG and no /WKT in one dictionary (`alone`), with both in one \
         (`both`), and with an /LGIDict, which is not ISO 32000-2's)"
    );
    let empty = BTreeMap::new();
    ranked(
        "WKT dialects (documents)",
        &total.dialects,
        &total.witnesses,
    );
    ranked(
        "Projection methods named in WKT (documents)",
        &total.methods,
        &total.witnesses,
    );
    ranked(
        "EPSG codes, by how stated (documents)",
        &total.codes,
        &total.witnesses,
    );
    ranked(
        "Ellipsoids named in WKT (documents)",
        &total.ellipsoids,
        &empty,
    );
    ranked("Units named in WKT (documents)", &total.units, &empty);
    ranked(
        "Parameters named in WKT (documents)",
        &total.parameters,
        &empty,
    );
    ranked(
        "This tree's reading of each /GCS, pdf_model::geospatial (documents)",
        &total.readings,
        &total.witnesses,
    );
    println!(
        "\n== {} distinct WKT strings, every one in full, by the documents stating it",
        total.distinct.len()
    );
    let mut texts: Vec<(&String, &usize)> = total.distinct.iter().collect();
    texts.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    for (text, documents) in texts {
        println!("  {documents:>6}  {text}");
    }
    println!("\n== Duration: {} s", started.elapsed().as_secs());
}
