//! Plans, fetches and surveys a chunk of the `SafeDocs` corpora.
//!
//! ```text
//! safedocs corpora                                     # what is addressable; no network
//! safedocs plan  --archive 0000 --count 16             # what a chunk holds and what it costs
//! safedocs fetch --archive 0000 --count 16 --download  # the same, and then transfer it
//! safedocs list                                        # what the cache holds
//! safedocs survey                                      # run this tree over the cache
//! ```
//!
//! **`fetch` without `--download` is `plan`**, and says so. The number is printed before the
//! bytes move, and a plan larger than `--budget-mb` (32 by default) is refused whether or not
//! `--download` is present — naming, in the refusal, the `--budget-mb` that would admit it.
//!
//! **That is a bound on accident and not on the person.** `--budget-mb` has no ceiling, and
//! `--all` takes every member of an archive:
//!
//! ```text
//! safedocs fetch --archive 0042 --all --budget-mb 2048 --download   # about 1.2 GiB, asked for
//! ```
//!
//! which on an unmetered connection is a reasonable thing to want. What cannot happen is
//! getting a gigabyte without having typed the number: the archive is addressed through its
//! own central directory — a `HEAD` and one 64 KiB range — and a fetch is one contiguous byte
//! range over the members the chunk named.

#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "a command-line tool whose entire output is a report"
)]

use std::path::PathBuf;

use safedocs::survey::Outcome;
use safedocs::{Budget, Cache, Chunk, Error, source, survey};

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("safedocs: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}

/// What the caller asked for, or why it could not be read as a request.
#[derive(Debug, thiserror::Error)]
enum Refused {
    /// The fetch or the survey itself.
    #[error("{0}")]
    Fetch(#[from] Error),
    /// A word in the first position that names no command.
    #[error("no such command: {0:?} — try corpora, plan, fetch, list or survey")]
    Command(String),
    /// An argument that should have been a number.
    #[error("{0} expects a number and was given {1:?}")]
    NotANumber(&'static str, String),
}

/// Reads the arguments and does what they say.
fn run() -> Result<(), Refused> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let command = arguments.first().cloned().unwrap_or_default();
    let rest = arguments.get(1..).unwrap_or_default();

    let flag = |name: &str| rest.iter().any(|argument| argument == name);
    let text = |name: &str| -> Option<String> {
        rest.iter()
            .position(|argument| argument == name)
            .and_then(|at| rest.get(at.saturating_add(1)).cloned())
            .or_else(|| {
                rest.iter()
                    .find_map(|argument| argument.strip_prefix(&format!("{name}=")))
                    .map(str::to_owned)
            })
    };
    let number = |name: &'static str, fallback: u64| -> Result<u64, Refused> {
        match text(name) {
            None => Ok(fallback),
            Some(value) => value
                .parse()
                .map_err(|_| Refused::NotANumber(name, value.clone())),
        }
    };

    match command.as_str() {
        "corpora" => {
            corpora();
            Ok(())
        }
        "list" => {
            list(&cache(text("--cache")));
            Ok(())
        }
        "survey" => {
            // `--dir` is what makes this more than a `SafeDocs` command: the three corpora
            // added as submodules in the four-hundred-and-twenty-second session are directories
            // of PDFs like any other, and the survey is the one report this tree has that is
            // not a ratchet. See `doc/adr/0258`.
            let roots = text("--dir").map_or_else(
                || Cache::at(cache_root(text("--cache"))).documents(),
                |named| {
                    let mut found = Vec::new();
                    for directory in named.split(',') {
                        found.extend(Cache::at(PathBuf::from(directory)).documents());
                    }
                    found
                },
            );
            survey_documents(&roots);
            Ok(())
        }
        "plan" | "fetch" => {
            let corpus = source::corpus(&text("--corpus").unwrap_or_else(|| {
                source::KNOWN
                    .first()
                    .map_or_else(String::new, |corpus| corpus.name.to_owned())
            }))
            .map_err(Refused::Fetch)?;
            let chunk = Chunk {
                corpus,
                archive: text("--archive").unwrap_or_else(|| "0000".to_owned()),
                from: usize::try_from(number("--from", 0)?).unwrap_or(usize::MAX),
                count: usize::try_from(number("--count", 8)?).unwrap_or(usize::MAX),
                named: text("--name").map_or_else(Vec::new, |names| {
                    names.split(',').map(str::to_owned).collect()
                }),
                all: flag("--all"),
            };
            let budget = Budget::mebibytes(number("--budget-mb", 32)?);
            let download = command == "fetch" && flag("--download");
            do_plan(&chunk, budget, download, &cache(text("--cache")))?;
            Ok(())
        }
        other => Err(Refused::Command(other.to_owned())),
    }
}

/// The cache a caller named, or the default beside the workspace root.
fn cache(named: Option<String>) -> Cache {
    Cache::at(cache_root(named))
}

/// Where the cache is.
fn cache_root(named: Option<String>) -> PathBuf {
    named.map_or_else(Cache::default_root, PathBuf::from)
}

/// Prints what can be addressed, without touching the network.
fn corpora() {
    println!("corpora this tool can address:");
    for corpus in source::KNOWN {
        println!(
            "  {}\n    {}\n    {} archives of about {} each, at {}/{}",
            corpus.name,
            corpus.about,
            corpus.archives,
            human(corpus.archive_bytes),
            corpus.origin,
            corpus.prefix
        );
    }
    println!(
        "\na plan reads the central directory only; a fetch asks for one byte range over the \
         members it names.\n--all takes every member of an archive, which is about a gigabyte \
         and needs --budget-mb to match:\n  safedocs fetch --archive 0042 --all --budget-mb 2048 \
         --download"
    );
}

/// Resolves a chunk, prints it, and — only with `--download` — fetches it.
fn do_plan(chunk: &Chunk, budget: Budget, download: bool, cache: &Cache) -> Result<(), Error> {
    let plan = safedocs::plan(chunk)?;
    println!(
        "{} {}: {} member(s), reading the directory cost {}",
        chunk.corpus.name,
        chunk.archive,
        plan.members.len(),
        human(plan.directory_bytes)
    );
    for entry in &plan.members {
        println!(
            "  {:<16} {:>12} compressed  {:>12} inflated",
            entry.name,
            human(entry.compressed),
            human(entry.uncompressed)
        );
    }
    println!(
        "  would transfer {} (bytes {}-{} of the archive), inflating to {}",
        human(plan.transfer()),
        plan.first,
        plan.last,
        human(plan.uncompressed())
    );
    println!("  budget {}", human(budget.bytes()));

    plan.within(budget)?;

    if !download {
        println!(
            "\nnothing was transferred. Add --download to fetch this chunk, --count or --all to \
             change what it holds."
        );
        return Ok(());
    }

    println!("\nfetching {} ...", human(plan.transfer()));
    let span = safedocs::fetch_range(&plan.url, plan.first, plan.last)?;
    let mut written = 0u64;
    for entry in &plan.members {
        let bytes = safedocs::zip::extract(entry, &span, plan.first)?;
        let fetched = cache.store(chunk.corpus.name, &chunk.archive, entry, &bytes)?;
        written = written.saturating_add(fetched.bytes);
        println!(
            "  {} {} {}",
            fetched.member,
            human(fetched.bytes),
            fetched.digest
        );
    }
    println!("cached {} into {}", human(written), cache.root().display());
    Ok(())
}

/// Prints what the cache holds.
fn list(cache: &Cache) {
    let documents = cache.documents();
    if documents.is_empty() {
        println!(
            "the cache at {} is empty; `safedocs plan` says what a chunk would cost",
            cache.root().display()
        );
        return;
    }
    let mut total = 0u64;
    for path in &documents {
        let bytes = std::fs::metadata(path).map_or(0, |metadata| metadata.len());
        total = total.saturating_add(bytes);
        println!("  {:>10}  {}", human(bytes), path.display());
    }
    println!("{} documents, {}", documents.len(), human(total));
}

/// Runs this tree over a set of documents and reports the way `tests/corpus.rs` does.
fn survey_documents(documents: &[PathBuf]) {
    if let Err(why) = survey::sandbox_available() {
        println!(
            "note: the sandboxed image decoder is unavailable ({why}), so JBIG2 and JPEG 2000 \
             will report rather than draw. Build it with `cargo build -p pdf-sandbox --bins`."
        );
    }
    if documents.is_empty() {
        println!("nothing to survey: no PDF files were found");
        return;
    }
    let started = std::time::Instant::now();
    let verdicts = survey::survey(documents);
    let elapsed = started.elapsed();

    let count = |wanted: fn(&Outcome) -> bool| {
        verdicts
            .iter()
            .filter(|verdict| wanted(&verdict.outcome))
            .count()
    };
    println!(
        "{} documents in {:.1}s: {} unopenable, {} locked, {} encrypted beyond us, {} pageless, \
         {} incomplete, {} slow",
        verdicts.len(),
        elapsed.as_secs_f64(),
        count(|outcome| matches!(outcome, Outcome::Unopenable(_) | Outcome::Unreadable(_))),
        count(|outcome| matches!(outcome, Outcome::Locked)),
        count(|outcome| matches!(outcome, Outcome::UnreadableEncryption(_))),
        count(|outcome| matches!(outcome, Outcome::Pageless)),
        count(|outcome| matches!(outcome, Outcome::Incomplete(_))),
        verdicts.iter().filter(|verdict| verdict.is_slow()).count(),
    );
    let missed: usize = verdicts
        .iter()
        .map(|verdict| verdict.codes_without_a_glyph)
        .sum();
    let silent = verdicts
        .iter()
        .filter(|verdict| verdict.codes_without_a_glyph > 0)
        .count();
    println!(
        "  codes reaching no glyph *in silence*: {missed} over {silent} documents \
         (measurement, not a gate; doc/todo/21)"
    );
    // Every one of them, not a top ten: at this population's size the ranked list *is* the
    // measurement — the four-hundred-and-thirty-fourth session had to open the documents
    // behind it — and one line apiece is nothing beside the per-document lines below.
    let mut worst: Vec<(&str, usize)> = verdicts
        .iter()
        .filter(|verdict| verdict.codes_without_a_glyph > 0)
        .map(|verdict| (verdict.name.as_str(), verdict.codes_without_a_glyph))
        .collect();
    worst.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(right.0)));
    for (name, count) in &worst {
        println!("    {count:6} {name}");
    }
    let blank: usize = verdicts
        .iter()
        .map(|verdict| verdict.codes_reaching_a_blank_glyph)
        .sum();
    let blank_documents = verdicts
        .iter()
        .filter(|verdict| verdict.codes_reaching_a_blank_glyph > 0)
        .count();
    println!(
        "  codes reaching a glyph the font draws blank: {blank} over {blank_documents} \
         documents (not a mark missed; ADR 0270)"
    );
    let mut blankest: Vec<(&str, usize)> = verdicts
        .iter()
        .filter(|verdict| verdict.codes_reaching_a_blank_glyph > 0)
        .map(|verdict| (verdict.name.as_str(), verdict.codes_reaching_a_blank_glyph))
        .collect();
    blankest.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(right.0)));
    for (name, count) in blankest.iter().take(10) {
        println!("    blank {count:6} {name}");
    }
    for verdict in &verdicts {
        match &verdict.outcome {
            Outcome::Complete => println!("  complete: {}", verdict.name),
            Outcome::Incomplete(reported) => {
                println!("  incomplete: {}: {reported}", verdict.name);
            }
            Outcome::Locked => println!("  locked: {}", verdict.name),
            Outcome::UnreadableEncryption(why) => {
                println!("  encryption we do not implement: {}: {why}", verdict.name);
            }
            Outcome::Unopenable(why) | Outcome::Unreadable(why) => {
                println!("  unusable: {}: {why}", verdict.name);
            }
            Outcome::Pageless => println!("  unusable: {}: no first page", verdict.name),
        }
        if verdict.is_slow() {
            println!("  slow: {}: {:?}", verdict.name, verdict.taken);
        }
    }
}

/// A byte count somebody has to read on a mobile connection.
fn human(bytes: u64) -> String {
    #[expect(
        clippy::cast_precision_loss,
        reason = "a byte count printed to one decimal place; the loss begins above 2^53 bytes"
    )]
    let scaled = |unit: u64| bytes as f64 / unit as f64;
    if bytes >= 1 << 30 {
        format!("{:.1} GiB", scaled(1 << 30))
    } else if bytes >= 1 << 20 {
        format!("{:.1} MiB", scaled(1 << 20))
    } else if bytes >= 1 << 10 {
        format!("{:.1} KiB", scaled(1 << 10))
    } else {
        format!("{bytes} B")
    }
}
