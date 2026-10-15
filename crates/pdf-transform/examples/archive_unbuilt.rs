//! Which refusal sites still read "not built yet", and how many documents reach each.
//!
//! ```sh
//! cargo run --release -p pdf-transform --example archive_unbuilt -- <dir>...
//! ```
//!
//! `quorra-transform archive --remedy-sites` lists every site a configuration may answer and says
//! which have no remedy built; it needs no document, which is its point. This adds the second
//! denominator `CLAUDE.md` asks work to be chosen from: over the documents under each directory
//! given, how many fail each such site's requirement for each of the six targets. A site no
//! document reaches is still owed — the count ranks the work, it does not excuse any of it.
//!
//! **The predicate is the listing's own**, the one its "N of M sites not built yet" trailer counts:
//! no `discard` authorises a loss there, and no tool, supplied fact, fetched file, in-place
//! `preserve` or condition the decision table settles answers it. The count is of documents the
//! validator reports failing the requirement, which is the population the converter refuses over
//! at that site.

#![expect(
    clippy::print_stdout,
    reason = "an example whose whole product is a table a person reads"
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use pdf_archive::Target;
use pdf_syntax::{Document, FileBytes, Limits};
use pdf_transform::archive::{Site, Standing, census, sites};

/// Whether `--remedy-sites` counts a site in its "not built yet" trailer: the same predicate.
fn unbuilt(site: &Site) -> bool {
    !(site.built_discard.is_some()
        || site.takes_a_tool
        || site.takes_a_supplied_fact
        || site.takes_a_fetched_file
        || site.preserves_in_place.is_some()
        || site.conditional.is_some())
}

/// Every `.pdf` under `directory`, symbolic links followed, in path order.
fn documents(directory: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            documents(&path, out);
        } else if path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("pdf"))
        {
            out.push(path);
        }
    }
}

fn main() {
    let mut paths = Vec::new();
    for directory in std::env::args().skip(1) {
        documents(Path::new(&directory), &mut paths);
    }
    paths.sort();

    // Per target, the unbuilt sites and the documents failing each.
    let mut counts: BTreeMap<String, BTreeMap<&'static str, usize>> = BTreeMap::new();
    let mut opened = 0_usize;
    for path in &paths {
        let Ok(bytes) = FileBytes::on_disk(path) else {
            continue;
        };
        let Ok(document) = Document::open_with_limits(bytes, Limits::DEFAULT) else {
            continue;
        };
        opened = opened.saturating_add(1);
        for target in Target::ALL {
            let wanted: Vec<&'static str> = sites(target)
                .into_iter()
                .filter(unbuilt)
                .map(|site| site.requirement)
                .collect();
            let report = pdf_archive::check(&document, target);
            let row = counts.entry(target.to_string()).or_default();
            for requirement in &wanted {
                row.entry(requirement).or_default();
            }
            for failed in report.failures() {
                if let Some(seen) = row.get_mut(failed.id) {
                    *seen = seen.saturating_add(1);
                }
            }
        }
    }

    println!("{opened} of {} documents opened", paths.len());
    for target in Target::ALL {
        let Some(row) = counts.get(&target.to_string()) else {
            continue;
        };
        // Which kind of refusal each site is, because the four are different facts: the fence,
        // no conforming file for this target, the caller's own choice, and a gap named as one.
        let kinds: BTreeMap<&str, &str> = census(target)
            .map(|(requirement, standing)| {
                let kind = match standing {
                    Standing::Refused(because) => because.word(),
                    other => other.word(),
                };
                (requirement.id, kind)
            })
            .collect();
        let mut ranked: Vec<(&&str, &usize)> = row.iter().collect();
        ranked.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
        println!("\n{target}: {} unbuilt site(s)", ranked.len());
        for (requirement, documents) in ranked {
            let kind = kinds.get(requirement).copied().unwrap_or("-");
            println!("  {documents:>5}  {kind:<16} {requirement}");
        }
    }
}
