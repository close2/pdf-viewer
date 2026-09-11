//! `doc/todo/00` step 7: an ambiguous page where we draw less than every reference.
//!
//! # What it is, and why it is not a ranking
//!
//! `doc/todo/00`'s steps 1 to 6 take a page at a time off a ranking of *distance*. A ranking
//! cannot see missing content, because a page that draws less than everybody is not necessarily
//! far from anybody — `issue19634.pdf` sat at 0.85 from its nearest reference with a quarter of
//! its marks absent, and `rc_annotation.pdf` sat at 0.73 having drawn no annotation at all. What
//! sees those is one number over the artefacts the gate has already written — **our ink minus the
//! lightest reference's, over every page the gate calls `ambiguous`**, sorted ascending. A large
//! negative gap is content we are not drawing; a large positive one is
//! content nobody else is. It is `doc/todo/00` step 7, and it has produced the bucket's first
//! positive result and two of its defects.
//!
//! # Why it is a program, three hundred rounds after it was first run
//!
//! Because it was a recipe. `doc/todo/01`'s standing rule is that a sweep described in prose is
//! a sweep each round rebuilds from the paragraph, and this one had been rebuilt at least fifteen
//! times. Two of those rebuilds are recorded as having cost the round that made them: one took
//! the ink with a greyscale of its own and moved the head by a quarter of a level, which is the
//! size of the movement the sweep is watched for; another read the *undiagnosed* list instead of
//! the gate's own `ambiguous` lines and so swept a hundred pages where the population is eight
//! hundred.
//!
//! **And the rebuild in the nine-hundred-and-seventy-fourth session found the shape that makes a
//! program worth more than a corrected paragraph.** The gate prints a page from a labelled corpus
//! as `pdfbox/cweb.pdf page 10` and writes its artefacts to `pdfbox/cweb/p10/cweb-p10-ours.png` —
//! the label is a *directory* in the path and not part of the file's own name. A loop written
//! from the recipe's `<target>/tmp/oracle/<stem>/p<n>/` joins the whole printed name into the
//! file name, finds nothing, and **skips the page in silence**: that loop measured 775 of the
//! 838 pages it had listed, and the head, the alarm count and every printed row looked exactly as
//! they always had. It had also listed one page fewer than the gate printed, which is the same
//! defect in the population rather than in the path. That is trap 25 with the sign
//! reversed — a hand-written population that fails to name something that exists, where finding
//! nothing there reads as a pass — and the answer to it is [`Measurement::missing`]: this sweep
//! states the denominator it read and a page it could not measure is a failure of the instrument
//! rather than a line in the middle of a report.
//!
//! # The four corrections, which are the sweep and not decoration
//!
//! The first three were paid for by a round that ran the loop without them (`doc/todo/00` step
//! 7); the fourth by the round that wrote this program:
//!
//! 1. **A reference that drew nothing is dropped before the minimum is taken** ([`lightest`]). A
//!    blank is not a lower bound on the geometry, and leaving it in turns another program's
//!    failure into our surplus — four pages came back at +21 to +29 of 255 that way. What the
//!    *positive* side is good for is exactly that: finding a reference that failed.
//! 2. **The population is every page the gate calls `ambiguous`**, not the undiagnosed ones
//!    ([`pages`]). Diagnosing a page takes it off the undiagnosed list, so a sweep reading that
//!    list loses a page the moment somebody explains it — and the instrument that sees content we
//!    are not drawing would go blind in proportion to the work done on it.
//! 3. **A page the gate reports on is expected to be light**, so the label the gate prints is
//!    carried through to the report ([`Page::reported`]). Drawing less ink is what an incomplete
//!    interpretation *is*, made visible.
//! 4. **A panel file is not a render** ([`Page::without`]). `mutool draw` creates its `-o` file
//!    before it decides it cannot draw the page, so a refusal leaves a **zero-byte** PNG in the
//!    evidence directory — trap 3's second instance, which the gate already handles by judging
//!    the page without that reference and saying so on its own line. To a sweep reading the
//!    directory the page looks complete, and the blank is either a lower bound of zero (which is
//!    correction 1's defect one step earlier) or a page it cannot measure at all. So the
//!    exclusions are read off the gate's report beside the population, and `issue21436.pdf` is
//!    the witness: `mupdf.png` is zero bytes on disk and the gate's line reads
//!    `[judged without: mupdf did not render: … invalid page number: -1]`.
//!
//! # The alarm
//!
//! [`ALARM`] is −1 of 255, and its meaning is a reading list rather than a verdict: the standing
//! result is three names at or past it on documents the gate calls complete, every one of them
//! diagnosed and held by an `AMBIGUOUS_*` group. A new name there is the finding. This sweep does
//! not ratchet, because the group lists live in the gate and a note is a person's to write — it
//! exits non-zero only when it could not measure a page it was given, which is a statement about
//! the instrument.
//!
//! # What it cannot see, said once so that a later null is readable
//!
//! Its population is the gate's `ambiguous` lines, so a page that moves between `ambiguous` and
//! `agrees` or `contradicted` is invisible at both ends; a page whose consensus was divided has
//! no artefact directory, because the gate deletes one it does not need; and every figure is a
//! **difference between two programs**, so a reference re-rendered by a newer binary moves a row
//! with nothing of ours having changed.

use std::path::{Path, PathBuf};

/// The renderers the oracle keeps a panel for, in the order the report prints them.
///
/// `hayro` is here and does not vote in the gate's own arithmetic — a distinction that matters to
/// a verdict and not to this sweep, whose question is what the *lightest* program drew.
pub const REFERENCES: [&str; 4] = ["poppler", "mupdf", "ghostscript", "hayro"];

/// The gap at which a row is worth reading, in levels of 255.
///
/// `doc/todo/00` step 7's own threshold, and the only figure in this module that is a choice. It
/// was set by the run that found `rc_annotation.pdf` at −1.783 — a page 0.73 from its nearest
/// reference, which no ranking would ever have produced.
pub const ALARM: f64 = -1.0;

/// What `magick` is given after the panel's path, so that no round can take the ink its own way.
///
/// `-alpha off` is not optional and neither is the single channel. Our panels and `hayro`'s carry
/// an alpha channel and the three C references' do not, so `-colorspace Gray` without it averages
/// alpha in as a second channel and returns **exactly half** the ink — a comparison between half
/// of one number and all of another, in which the two renderers that "agree" with us are the two
/// whose file format matches ours. And a greyscale of one's own weights blue differently: taken
/// with Rec601 luma the head of this ranking moves by a quarter of a level on a page whose rules
/// are pure blue, which is the size of the movement being watched for. The recipe is part of the
/// measurement, so it lives beside the sweep rather than in each round's own script.
pub const INK_ARGUMENTS: [&str; 9] = [
    "-alpha",
    "off",
    "-channel",
    "R",
    "-colorspace",
    "Gray",
    "-format",
    "%[fx:(1-mean)*255]",
    "info:",
];

/// One page of the gate's `ambiguous` bucket.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    /// The name the gate prints, carrying a corpus label where the document has one.
    pub name: String,
    /// The corpus label, or the empty string for the unlabelled corpus.
    pub label: String,
    /// The document's file stem — its name without the `.pdf`.
    pub stem: String,
    /// The page number within the document, as the gate prints it.
    pub number: u32,
    /// Whether the gate printed `(incomplete)` — this tree reports something about the page.
    pub reported: bool,
    /// The references the gate said did not render this page, from its own `judged without`.
    ///
    /// Correction 4, and the reason it is read off the report rather than off the directory: a
    /// panel **file** is not a render. `mutool draw` creates its `-o` file before it decides it
    /// cannot draw the page, so a refusal leaves a zero-byte PNG behind (trap 3), and the gate
    /// says so on the page's own line while the directory looks complete. A sweep that reads the
    /// directory either counts that blank as a lower bound of zero — turning another program's
    /// failure into our surplus, which is correction 1 one step earlier — or refuses the page.
    pub without: Vec<String>,
}

impl Page {
    /// Where this page's evidence sits, relative to the run's artefact root.
    ///
    /// The label is a **directory** and not part of the file's own name, which is the whole of
    /// the defect this module's header describes: `pdfbox/cweb.pdf page 10` is written to
    /// `pdfbox/cweb/p10/`, and a loop that joins the printed name into the file name looks for
    /// `pdfbox/cweb/p10/pdfbox/cweb-p10-ours.png` and finds nothing.
    #[must_use]
    pub fn directory(&self) -> PathBuf {
        let within = PathBuf::from(&self.stem).join(format!("p{}", self.number));
        if self.label.is_empty() {
            within
        } else {
            PathBuf::from(&self.label).join(within)
        }
    }

    /// Our own raster for this page, relative to the run's artefact root.
    ///
    /// It is our render **after** the gate reconciled it with the smallest size any voting
    /// reference produced, which is what makes it comparable with the panels beside it and why
    /// `doc/todo/00` step 3 forbids reading our page *size* off it.
    #[must_use]
    pub fn ours(&self) -> PathBuf {
        self.directory()
            .join(format!("{}-p{}-ours.png", self.stem, self.number))
    }

    /// One reference's raster for this page, relative to the run's artefact root.
    #[must_use]
    pub fn reference(&self, renderer: &str) -> PathBuf {
        self.directory().join(format!("{renderer}.png"))
    }

    /// Whether this reference drew the page at all, by the gate's own account.
    #[must_use]
    pub fn drew(&self, renderer: &str) -> bool {
        !self.without.iter().any(|name| name == renderer)
    }
}

/// Every page the gate's report calls `ambiguous`, in the order it printed them.
///
/// The line is the one the oracle prints for each page it does not call agreement:
///
/// ```text
///   issue16038.pdf page 1: ambiguous — ours at worst mean 4.49 worst tile 22.60 …
///   bug1050040.pdf page 1: ambiguous (incomplete) — ours at worst mean 8.83 …
/// ```
///
/// A verdict of `agrees`, `contradicted`, `not comparable` or `no render` is not this sweep's
/// population, and correction 2 is why the *undiagnosed* list is not either.
#[must_use]
pub fn pages(report: &str) -> Vec<Page> {
    report.lines().filter_map(parse).collect()
}

/// One report line, or `None` where it is not an `ambiguous` verdict.
fn parse(line: &str) -> Option<Page> {
    let (name, rest) = line.split_once(": ")?;
    let name = name.trim();
    let rest = rest.trim_start();
    let reported = if let Some(tail) = rest.strip_prefix("ambiguous (incomplete)") {
        tail.starts_with(' ') || tail.is_empty()
    } else {
        let tail = rest.strip_prefix("ambiguous")?;
        if tail.starts_with(" (") {
            return None;
        }
        false
    };

    let (document, number) = name.rsplit_once(" page ")?;
    let number = number.parse().ok()?;
    let (label, file) = document.rsplit_once('/').unwrap_or(("", document));
    let stem = file.strip_suffix(".pdf")?;
    Some(Page {
        name: name.to_owned(),
        label: label.to_owned(),
        stem: stem.to_owned(),
        number,
        reported,
        without: refused(rest),
    })
}

/// Which references the gate judged this page without, from its own trailing bracket.
///
/// The gate writes `[judged without: mupdf did not render: …]`, so the sentence to look for is
/// the renderer's name in front of *did not render* — inside that bracket and nowhere else, since
/// a renderer's own error text names paths and other programs.
fn refused(line: &str) -> Vec<String> {
    let Some(bracket) = line.split_once("[judged without: ") else {
        return Vec::new();
    };
    let bracket = bracket
        .1
        .split_once(']')
        .map_or(bracket.1, |(head, _)| head);
    REFERENCES
        .into_iter()
        .filter(|renderer| bracket.contains(&format!("{renderer} did not render")))
        .map(str::to_owned)
        .collect()
}

/// Where the run that produced this report wrote its artefacts.
///
/// Taken from the report's own last line rather than from an environment variable, because a
/// sweep run against one directory over another run's report is a comparison of two runs — and
/// the build directory moves with the worktree, which is trap 15 one file over.
#[must_use]
pub fn artefact_root(report: &str) -> Option<&str> {
    report
        .lines()
        .rev()
        .find_map(|line| line.trim().strip_prefix("artefacts under "))
        .map(str::trim)
}

/// The lightest reference that actually drew something, and its ink.
///
/// Correction 1: a panel at zero ink is a renderer that produced a blank sheet, and a blank is
/// not a lower bound on the page's geometry. Returns `None` where every reference is blank, which
/// is a page for the report's own missing list rather than a gap of zero.
#[must_use]
pub fn lightest(references: &[(String, f64)]) -> Option<&(String, f64)> {
    references
        .iter()
        .filter(|&&(_, ink)| ink > 0.0)
        .min_by(|a, b| a.1.total_cmp(&b.1))
}

/// One page's measurement: our ink, the lightest live reference's, and the difference.
#[derive(Debug, Clone, PartialEq)]
pub struct Measured {
    /// The page the figures are about.
    pub page: Page,
    /// Our own ink in levels of 255, `(1 − mean) × 255` over the recipe in [`INK_ARGUMENTS`].
    pub ours: f64,
    /// Which reference was lightest among those that drew anything.
    pub renderer: String,
    /// That reference's ink, in the same units.
    pub reference: f64,
}

impl Measured {
    /// Our ink minus the lightest live reference's. Negative is ink we are not laying down.
    #[must_use]
    pub fn gap(&self) -> f64 {
        self.ours - self.reference
    }

    /// Whether this row is at or past [`ALARM`], which is the reading list.
    #[must_use]
    pub fn alarming(&self) -> bool {
        self.gap() <= ALARM
    }
}

/// What a whole run of the sweep found, including what it could not measure.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Measurement {
    /// Every page measured, ordered by gap ascending — the negative head first.
    pub measured: Vec<Measured>,
    /// Every page the report listed that this sweep could not measure, with the reason.
    ///
    /// **This field is why the sweep is a program.** A silent skip here reads exactly like a
    /// clean run, and one skipped 63 of 838 pages for as long as the sweep was a paragraph.
    pub missing: Vec<(Page, String)>,
}

impl Measurement {
    /// Orders the rows so that the ink we are not drawing is at the top.
    pub fn rank(&mut self) {
        self.measured.sort_by(|a, b| {
            a.gap()
                .total_cmp(&b.gap())
                .then(a.page.name.cmp(&b.page.name))
        });
    }

    /// How many pages were listed, measured and skipped.
    #[must_use]
    pub fn denominator(&self) -> (usize, usize) {
        (
            self.measured.len().saturating_add(self.missing.len()),
            self.measured.len(),
        )
    }
}

/// Resolves a panel against the run's artefact root.
#[must_use]
pub fn under(root: &Path, relative: &Path) -> PathBuf {
    root.join(relative)
}

#[cfg(test)]
mod tests {
    use super::*;

    const LABELLED: &str = "  pdfbox/cweb.pdf page 10: ambiguous — ours at worst mean 3.4 \
                            worst tile 16.8 differing 5.00% ssim 0.9000; bound mean 5.00";
    const PLAIN: &str = "  issue16038.pdf page 1: ambiguous — ours at worst mean 4.49 worst \
                         tile 22.60 differing 5.00% ssim 0.9000; bound mean 1.00";
    const REPORTED: &str = "  bug1050040.pdf page 1: ambiguous (incomplete) — ours at worst \
                            mean 8.83 worst tile 25.78 differing 5.96% ssim 0.7553";

    /// Trap 13's plant, and the defect this module was written for.
    ///
    /// A page from a labelled corpus is printed with the label in its **name** and written with
    /// the label as a **directory**. The hand-written loop this replaces joined the whole printed
    /// name into the file name, so it looked for `pdfbox/cweb/p10/pdfbox/cweb-p10-ours.png`,
    /// found nothing, and skipped 63 of 838 pages without saying so.
    #[test]
    fn a_labelled_corpus_puts_its_label_in_the_directory_and_not_in_the_file_name() {
        let page = pages(LABELLED).pop().expect("one ambiguous line");
        assert_eq!(page.label, "pdfbox");
        assert_eq!(page.stem, "cweb");
        assert_eq!(page.ours(), Path::new("pdfbox/cweb/p10/cweb-p10-ours.png"));
        assert_eq!(
            page.reference("mupdf"),
            Path::new("pdfbox/cweb/p10/mupdf.png")
        );
    }

    /// The other half of the same plant: an unlabelled page has no directory of its own.
    #[test]
    fn an_unlabelled_page_resolves_directly_under_the_root() {
        let page = pages(PLAIN).pop().expect("one ambiguous line");
        assert!(page.label.is_empty());
        assert_eq!(
            page.ours(),
            Path::new("issue16038/p1/issue16038-p1-ours.png")
        );
    }

    #[test]
    fn the_gates_own_label_for_a_page_it_reports_on_is_carried_through() {
        let page = pages(REPORTED).pop().expect("one ambiguous line");
        assert!(page.reported, "the gate printed (incomplete)");
        assert!(!pages(PLAIN)[0].reported);
    }

    /// The population is the gate's `ambiguous` lines and nothing else — correction 2.
    #[test]
    fn no_other_verdict_is_in_the_population() {
        let report = "  a.pdf page 1: agrees — ours at worst mean 0.10\n  \
                      b.pdf page 1: contradicted — ours at worst mean 9.00\n  \
                      c.pdf page 1: not comparable (incomplete) — poppler returned one colour\n  \
                      d.pdf page 2: no render — the page could not be drawn\n";
        assert!(pages(report).is_empty());
    }

    /// Correction 1, both ways: a blank reference is dropped, and a live one is not.
    #[test]
    fn a_reference_that_drew_nothing_is_not_a_lower_bound() {
        let with_a_blank = [
            ("poppler".to_owned(), 3.5),
            ("mupdf".to_owned(), 0.0),
            ("hayro".to_owned(), 4.0),
        ];
        assert_eq!(
            lightest(&with_a_blank).map(|&(_, ink)| ink),
            Some(3.5),
            "the blank would have made a correct page look 3.5 heavy"
        );

        let all_live = [("poppler".to_owned(), 3.5), ("mupdf".to_owned(), 2.0)];
        assert_eq!(lightest(&all_live).map(|&(_, ink)| ink), Some(2.0));
    }

    #[test]
    fn a_page_no_reference_drew_is_not_a_gap_of_zero() {
        let blank = [("poppler".to_owned(), 0.0), ("mupdf".to_owned(), 0.0)];
        assert!(lightest(&blank).is_none());
    }

    #[test]
    fn the_root_is_read_from_the_run_that_produced_the_report() {
        let report = "  a.pdf page 1: ambiguous — ours\nartefacts under /somewhere/tmp/oracle\n";
        assert_eq!(artefact_root(report), Some("/somewhere/tmp/oracle"));
        assert_eq!(artefact_root("  a.pdf page 1: ambiguous — ours\n"), None);
    }

    #[test]
    fn the_ranking_puts_the_ink_we_are_not_drawing_first() {
        let page = |name: &str| Page {
            name: name.to_owned(),
            label: String::new(),
            stem: name.to_owned(),
            number: 1,
            reported: false,
            without: Vec::new(),
        };
        let row = |name: &str, ours: f64, reference: f64| Measured {
            page: page(name),
            ours,
            renderer: "mupdf".to_owned(),
            reference,
        };
        let mut measurement = Measurement {
            measured: vec![
                row("light", 1.0, 3.0),
                row("even", 5.0, 5.0),
                row("heavy", 9.0, 2.0),
            ],
            missing: Vec::new(),
        };
        measurement.rank();
        let order: Vec<&str> = measurement
            .measured
            .iter()
            .map(|row| row.page.name.as_str())
            .collect();
        assert_eq!(order, ["light", "even", "heavy"]);
        assert!(measurement.measured[0].alarming());
        assert!(!measurement.measured[1].alarming());
        assert_eq!(measurement.denominator(), (3, 3));
    }

    /// Correction 4, and the page that produced it.
    ///
    /// The gate judged `issue21436.pdf` without `mupdf` and said so; `mupdf.png` is zero bytes on
    /// disk beside four panels that are not. A sweep reading the directory sees five references.
    #[test]
    fn a_reference_the_gate_judged_the_page_without_is_not_a_reference() {
        const LINE: &str = "  issue21436.pdf page 1: ambiguous — ours at worst mean 3.43 worst \
                            tile 11.28 differing 2.70% ssim 0.9197; bound mean 1.00  [judged \
                            without: mupdf did not render: mupdf failed: produced no output \
                            (status Some(1)): argument error: invalid page number: -1 … cannot \
                            draw '/doc/pdf.js/test/pdfs/issue21436.pdf']";
        let page = pages(LINE).pop().expect("one ambiguous line");
        assert_eq!(page.without, ["mupdf"]);
        assert!(!page.drew("mupdf"));
        for renderer in ["poppler", "ghostscript", "hayro"] {
            assert!(page.drew(renderer), "{renderer} rendered this page");
        }
    }

    /// The other half: a page nothing was excluded from excludes nothing.
    ///
    /// The sentence is looked for **inside the bracket**, because a renderer's error text names
    /// paths and other programs — the witness above quotes a file path with `pdf` in it.
    #[test]
    fn a_page_the_gate_judged_whole_excludes_no_reference() {
        let page = pages(PLAIN).pop().expect("one ambiguous line");
        assert!(page.without.is_empty());
        for renderer in REFERENCES {
            assert!(page.drew(renderer));
        }
    }

    /// The denominator says a page was lost even when every measured row looks ordinary.
    #[test]
    fn a_page_that_could_not_be_measured_is_counted_rather_than_dropped() {
        let measurement = Measurement {
            measured: vec![Measured {
                page: pages(PLAIN).pop().expect("one line"),
                ours: 1.0,
                renderer: "mupdf".to_owned(),
                reference: 1.0,
            }],
            missing: vec![(
                pages(LABELLED).pop().expect("one line"),
                "no raster of ours".to_owned(),
            )],
        };
        assert_eq!(measurement.denominator(), (2, 1));
    }
}
