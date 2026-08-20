//! Re-runs the check every round that fixed a document outside the gates wrote down.
//!
//! **Why this exists is a merge, not a document.** The corpus, oracle and quorra gates walk
//! `doc/pdf.js`; a fix found by ranking the `SafeDocs` crawl is measured once, by the round that
//! makes it, in a tree that does not yet contain its neighbours' work. Two branches that touch
//! no common line can then defeat each other with every gate green — which is what session 623
//! found, and what cost three sessions to attribute (ADR 0458). `doc/todo/03` states the rule
//! this discharges: **a round that fixes a document no gate covers records the check, not just
//! the result, and the merge round re-runs those checks.**
//!
//! The rows live in `doc/checks/fixed-documents.toml`, which is the appendable half; this file
//! is the one command that runs them. Each row states two things a round can observe about a
//! page without a second renderer in the room:
//!
//! - **what it reports** — [`pdf_model::Interpretation::unsupported`], which is what caught the
//!   regression that led here;
//! - **how much ink it lays down** — because a third of the seeded documents were *silent* both
//!   before and after their fix, drawn black or blank or inverted with nothing to say so, and a
//!   report-only check could not see one of them come back.
//!
//! **Not a gate over the crawl and deliberately not one.** It holds documents whose defect
//! somebody has already read against a clause; it is a regression check, so a row appears here
//! only once its fix is understood.
//!
//! It **skips, loudly, where the documents are absent** — `corpus-cache` is a machine-local
//! crawl and not in the repository — because a smaller population is the one reason a check
//! like this can be silent without being wrong.

#![expect(
    clippy::print_stdout,
    reason = "a gate whose output is what a person reads to see what moved"
)]

use std::path::{Path, PathBuf};

use pdf_render::{Rasterizer as _, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

/// What a row says about a page's ink.
///
/// A named pair rather than an `Option`, because "this row deliberately pins nothing" is a
/// statement a round makes and not the absence of one.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Ink {
    /// The fix this row stands for was about a report rather than about pixels.
    Unpinned,
    /// The page's ink must lie inside this band.
    Band {
        /// Lowest ink the page may lay down.
        low: f64,
        /// Highest.
        high: f64,
    },
}

/// One row of `doc/checks/fixed-documents.toml`.
#[derive(Debug)]
struct Row {
    /// Where the document is, relative to the repository root.
    path: String,
    /// Which page, one-based.
    page: usize,
    /// The session that fixed it, for the history file a reader will want next.
    session: u32,
    /// Substrings that must each appear in some report, and no other report may appear.
    ///
    /// Empty means the page must report nothing at all, which is a stronger statement than
    /// "nothing unexpected" and is the one most of these rows want.
    reports: Vec<String>,
    /// What the row pins about the page's ink.
    ink: Ink,
    /// What the defect was, in one line.
    why: String,
}

/// The scalar this check calls a page's ink: the mean of `255 − luma` over the raster.
///
/// **Ours, over our own raster, and not the ranking's number.** `doc/todo/00` step 7 measures
/// ink through `ImageMagick` over panels from four renderers in order to *compare* renderers;
/// nothing is being compared here, so what is wanted is a number this tree can reproduce
/// exactly from its own pixels, and this is that. The two are the same quantity and will not
/// be the same figure, which is why they have the formula written beside them.
///
/// Rec. 709 luma, and alpha ignored for the reason `doc/todo/00` gives about `-alpha off`:
/// folding an opacity channel into a grey mean halves it.
fn ink(pixels: &[u8]) -> f64 {
    let mut total = 0.0_f64;
    let mut count = 0_u64;
    for pixel in pixels.chunks_exact(4) {
        let [red, green, blue] = [pixel[0], pixel[1], pixel[2]].map(f64::from);
        total += 255.0 - 0.2126_f64.mul_add(red, 0.7152_f64.mul_add(green, 0.0722 * blue));
        count = count.saturating_add(1);
    }
    if count == 0 {
        return 0.0;
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "a pixel count of a page; f64 is exact to 2^53"
    )]
    let pixels = count as f64;
    total / pixels
}

/// The repository root, from this crate's manifest directory.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Reads an `ink` value: `low .. high`, or empty for [`Ink::Unpinned`].
fn band(value: &str, at: usize) -> Result<Ink, String> {
    if value.is_empty() {
        return Ok(Ink::Unpinned);
    }
    let (low, high) = value
        .split_once("..")
        .ok_or_else(|| format!("line {at}'s ink is not `low .. high`"))?;
    let number = |part: &str| {
        part.trim()
            .parse::<f64>()
            .map_err(|_| format!("line {at}'s ink bound is not a number"))
    };
    Ok(Ink::Band {
        low: number(low)?,
        high: number(high)?,
    })
}

/// Reads the rows, or says what is wrong with the file.
///
/// A hand-written parser for a hand-written file, following `tools/conformance`'s precedent of
/// reading a small subset rather than buying a parser. **Anything it does not recognise is an
/// error rather than a skipped line**, which is the whole difference between a check a round
/// can append to and a check a round can silently append nothing to.
fn rows(text: &str) -> Result<Vec<Row>, String> {
    let mut found = Vec::new();
    let mut partial: Option<Partial> = None;
    for (index, line) in text.lines().enumerate() {
        let at = index.saturating_add(1);
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line == "[[document]]" {
            if let Some(previous) = partial.take() {
                finish(previous, at, &mut found)?;
            }
            partial = Some(Partial::default());
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(format!("line {at} is neither a key nor a header: {line}"));
        };
        let Some(row) = partial.as_mut() else {
            return Err(format!("line {at} states a key before any [[document]]"));
        };
        assign(row, key.trim(), value.trim(), at)?;
    }
    if let Some(last) = partial {
        finish(last, text.lines().count(), &mut found)?;
    }
    Ok(found)
}

/// A row under construction: every field optional until the row ends.
#[derive(Default)]
struct Partial {
    /// See [`Row::path`].
    path: Option<String>,
    /// See [`Row::page`].
    page: Option<usize>,
    /// See [`Row::session`].
    session: Option<u32>,
    /// See [`Row::reports`].
    reports: Option<Vec<String>>,
    /// See [`Row::ink`].
    ink: Option<Ink>,
    /// See [`Row::why`].
    why: Option<String>,
}

/// Turns a finished [`Partial`] into a [`Row`], or says which field it lacks.
fn finish(partial: Partial, at: usize, into: &mut Vec<Row>) -> Result<(), String> {
    let Partial {
        path: Some(path),
        page: Some(page),
        session: Some(session),
        reports: Some(reports),
        ink: Some(ink),
        why: Some(why),
    } = partial
    else {
        return Err(format!(
            "the row ending at line {at} is missing one of path, page, session, reports, ink, why"
        ));
    };
    into.push(Row {
        path,
        page,
        session,
        reports,
        ink,
        why,
    });
    Ok(())
}

/// Reads one `key = value` line into the row being built.
fn assign(row: &mut Partial, key: &str, value: &str, at: usize) -> Result<(), String> {
    let quoted = || {
        value
            .strip_prefix('"')
            .and_then(|rest| rest.strip_suffix('"'))
            .map(str::to_owned)
            .ok_or_else(|| format!("line {at}'s value is not a quoted string"))
    };
    // Spelled out rather than shared, because the two fields have different types and one
    // closure cannot parse both.
    let unreadable = |what: &str| format!("line {at}'s {what} is not a number");
    match key {
        "path" => row.path = Some(quoted()?),
        "why" => row.why = Some(quoted()?),
        "page" => row.page = Some(value.parse().map_err(|_| unreadable("page"))?),
        "session" => row.session = Some(value.parse().map_err(|_| unreadable("session"))?),
        "reports" => row.reports = Some(list(value, at)?),
        "ink" => row.ink = Some(band(value, at)?),
        other => return Err(format!("line {at} states an unknown key `{other}`")),
    }
    Ok(())
}

/// Reads a `reports` value: a bracketed list of quoted substrings, possibly empty.
fn list(value: &str, at: usize) -> Result<Vec<String>, String> {
    let inner = value
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
        .ok_or_else(|| format!("line {at}'s reports is not a [list]"))?;
    inner
        .split(',')
        .map(str::trim)
        .filter(|piece| !piece.is_empty())
        .map(|piece| {
            piece
                .strip_prefix('"')
                .and_then(|rest| rest.strip_suffix('"'))
                .map(str::to_owned)
                .ok_or_else(|| format!("line {at} holds an unquoted report"))
        })
        .collect()
}

/// What one document actually does now.
struct Observed {
    /// Every report the interpretation carries, as the page would name them.
    reports: Vec<String>,
    /// The page's ink, or `None` where no target could be sized.
    ink: Option<f64>,
}

/// Opens, interprets and rasterises one page.
fn observe(path: &Path, page: usize) -> Result<Observed, String> {
    let bytes = std::fs::read(path).map_err(|error| format!("unreadable: {error}"))?;
    let document = Document::open(bytes).map_err(|error| format!("does not open: {error}"))?;
    let pages = pdf_model::Pages::new(&document);
    let index = page.checked_sub(1).ok_or_else(|| "page 0".to_owned())?;
    let page = pages
        .get(index)
        .ok_or_else(|| format!("no page {}", index.saturating_add(1)))?;
    let interpretation = pdf_model::interpret(&document, &page);
    let reports = interpretation
        .unsupported
        .iter()
        .map(|report| format!("{report:?}"))
        .collect();
    // The same scale and budget `open_one` uses, so that a person reproducing a row by hand
    // gets the same picture. 64 MiB is the display list's own target budget.
    let ink = TargetSpec::for_page(&interpretation.display_list, 1.0, 64 << 20)
        .ok()
        .and_then(|target| {
            CpuRasterizer::new()
                .rasterize(&interpretation.display_list, target)
                .ok()
        })
        .map(|raster| ink(&raster.data));
    Ok(Observed { reports, ink })
}

/// Every report the row expects is present, no report it does not expect is, and the ink is in
/// band.
fn judge(row: &Row, observed: &Observed) -> Vec<String> {
    let mut complaints = Vec::new();
    for wanted in &row.reports {
        if !observed
            .reports
            .iter()
            .any(|report| report.contains(wanted))
        {
            complaints.push(format!(
                "expected a report containing {wanted:?} and got none"
            ));
        }
    }
    for report in &observed.reports {
        if !row.reports.iter().any(|wanted| report.contains(wanted)) {
            complaints.push(format!("unexpected report {report}"));
        }
    }
    if let Ink::Band { low, high } = row.ink {
        match observed.ink {
            Some(measured) if measured >= low && measured <= high => {}
            Some(measured) => {
                complaints.push(format!(
                    "ink {measured:.3} is outside {low:.3} .. {high:.3}"
                ));
            }
            None => complaints.push("the page did not rasterise".to_owned()),
        }
    }
    complaints
}

/// The check itself.
///
/// `#[ignore]` for `doc/todo/02`'s reason: it walks documents that are not in the repository
/// and takes seconds rather than milliseconds, so it is a gate line rather than a unit test.
#[test]
#[ignore = "walks the machine-local crawl cache; run it from doc/todo/02's sequence"]
fn every_document_a_round_fixed_is_still_fixed() {
    let root = root();
    let list = root.join("doc/checks/fixed-documents.toml");
    let text = std::fs::read_to_string(&list)
        .unwrap_or_else(|error| panic!("{} is readable: {error}", list.display()));
    let rows =
        rows(&text).unwrap_or_else(|error| panic!("{} is well formed: {error}", list.display()));
    assert!(!rows.is_empty(), "the check holds no documents");

    let mut failures = Vec::new();
    let mut absent = 0_usize;
    let mut checked = 0_usize;
    for row in &rows {
        let path = root.join(&row.path);
        if !path.is_file() {
            absent = absent.saturating_add(1);
            continue;
        }
        checked = checked.saturating_add(1);
        match observe(&path, row.page) {
            Ok(observed) => {
                let complaints = judge(row, &observed);
                let ink = observed
                    .ink
                    .map_or_else(|| "-".to_owned(), |value| format!("{value:.3}"));
                println!(
                    "{} p{} ink {ink} reports {} — session {}",
                    row.path,
                    row.page,
                    observed.reports.len(),
                    row.session
                );
                for complaint in complaints {
                    failures.push(format!(
                        "{} p{}: {complaint}\n    {}",
                        row.path, row.page, row.why
                    ));
                }
            }
            Err(error) => {
                failures.push(format!(
                    "{} p{}: {error}\n    {}",
                    row.path, row.page, row.why
                ));
            }
        }
    }

    println!(
        "fixed-documents: {checked} checked, {absent} absent, {} rows",
        rows.len()
    );
    assert!(
        checked > 0,
        "not one of the {} documents is on this machine — `tools/safedocs fetch --download` \
         has not been run here, so this check established nothing",
        rows.len()
    );
    assert!(
        failures.is_empty(),
        "{} of {checked} documents no longer do what the round that fixed them recorded:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[cfg(test)]
mod tests {
    use super::{Ink, rows};

    /// The parser refuses what it does not understand, which is what makes an appended row
    /// that is subtly wrong fail loudly instead of vanishing.
    #[test]
    fn an_unknown_key_is_an_error_rather_than_a_skipped_line() {
        let text = "[[document]]\npath = \"a.pdf\"\ncolour = \"blue\"\n";
        let error = rows(text).expect_err("an unknown key is refused");
        assert!(error.contains("colour"), "{error}");
    }

    /// A row missing a field is a row nobody can run, and the message says which field.
    #[test]
    fn an_incomplete_row_names_what_it_lacks() {
        let text = "[[document]]\npath = \"a.pdf\"\npage = 1\n";
        let error = rows(text).expect_err("an incomplete row is refused");
        assert!(error.contains("missing"), "{error}");
    }

    #[test]
    fn a_complete_row_parses() {
        let text = "# a comment\n[[document]]\npath = \"a.pdf\"\npage = 1\nsession = 621\n\
                    reports = [\"JBIG2\"]\nink = 1.0 .. 2.0\nwhy = \"because\"\n";
        let parsed = rows(text).expect("a complete row");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].reports, vec!["JBIG2".to_owned()]);
        assert_eq!(
            parsed[0].ink,
            Ink::Band {
                low: 1.0,
                high: 2.0,
            }
        );
    }

    /// An empty `ink` is how a row says its fix was about a report rather than about pixels.
    #[test]
    fn an_empty_ink_pins_nothing() {
        let text = "[[document]]\npath = \"a.pdf\"\npage = 1\nsession = 1\nreports = []\n\
                    ink =\nwhy = \"because\"\n";
        let parsed = rows(text).expect("a row with no ink band");
        assert_eq!(parsed[0].ink, Ink::Unpinned);
        assert!(parsed[0].reports.is_empty());
    }
}
