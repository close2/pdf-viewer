//! Where a clause of ISO 32000-2 is in ISO 32000-1:2008, for the rows that bind a PDF/A-2 target.
//!
//! # Two base standards, one convention, and a sweep that used to be prose
//!
//! Every `§` in this crate is an ISO 32000-2 number — `doc/habits.md`'s convention, and what lets
//! `cargo test -p conformance` check each one against `doc/md/`. PDF/A-4 adheres to that edition
//! and PDF/A-2 to ISO 32000-1:2008 (ISO 19005-2 section 5.1), so a reader checking a part 2
//! verdict against the edition the file actually adheres to resolves each number *there*. Most
//! resolve to the same subclause under the same title. **The ones that do not resolve to a
//! different real subclause rather than failing to resolve**, which is worse than a dangling
//! pointer: ISO 32000-2 removed *Font subsets* as a subclause of its own, so everything after
//! ISO 32000-1's 9.6.4 moved up one, and §9.6.4 names Type 3 fonts in one edition and font subsets
//! in the other. The nine-hundred-and-seventy-fifth session found eight such numbers by hand (ADR
//! 0986 section 4) and wrote each file's share of them into that file's module comment, with the
//! sentence "everything else this file cites was checked and agrees" — a claim about a sweep
//! nobody could re-run.
//!
//! This module is that sweep made a table and a test. [`SHIFTS`] records every cited number whose
//! counterpart in ISO 32000-1:2008 is not the same number under the same title, and
//! [`tests::every_citation_resolves_in_the_edition_a_part_two_file_adheres_to`] holds it against
//! **every** `§` in this crate's sources, both ways: a citation not in the table has to resolve in
//! the 2008 edition to a heading of the same title, and an entry in the table has to still be
//! needed. So the sentence *every citation in this crate resolves in the edition it names* is a
//! test rather than a claim, and it fails by name — with both editions' titles printed — when a
//! round cites a number that shifted without saying so, or when an entry here goes stale.
//!
//! # What the table does not do
//!
//! It does not rename a single citation. Each `§` in this crate is correct as an ISO 32000-2
//! number, and repeating the 2008 number at every site would be many copies of one fact. The
//! per-file tables in `crate::table::fonts`, `crate::table::graphics` and `crate::survey` stay
//! where a reader meets the numbers; this is the one place that holds all of them and the only
//! place a test reads.
//!
//! The numbers are stored without their `§`, because a section sign here would be one more
//! citation for the conformance gate to count and it would count it correctly for nothing.

/// Where one ISO 32000-2 subclause is in ISO 32000-1:2008.
///
/// Four shapes, and the difference between them is what a test can check: a number that moved
/// has to exist at its new place, a title that changed has to differ, and a subject with no
/// counterpart has to be absent — or present under another subject, which is the trap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Earlier {
    /// The same subject at another number, under the same title.
    Numbered(&'static str),
    /// The same subject at another number, under a title the 2008 edition words differently.
    NumberedAndRetitled(&'static str),
    /// No counterpart in the 2008 edition.
    ///
    /// Where the *number* exists there, it names something else, and [`Shift::subject`] says
    /// what — that is the dangerous case, because the citation resolves.
    Absent,
}

/// One cited number whose counterpart in the earlier edition is not the same number and title.
#[derive(Debug, Clone, Copy)]
pub struct Shift {
    /// The ISO 32000-2 number, as this crate cites it, without its `§`.
    pub clause: &'static str,
    /// Where it is in ISO 32000-1:2008.
    pub earlier: Earlier,
    /// What the subclause is about, and what the 2008 number names instead where it names
    /// anything.
    pub subject: &'static str,
}

/// Every cited number that does not resolve, in ISO 32000-1:2008, to the same number under the
/// same title.
///
/// In ISO 32000-2's order. The test below is what keeps this complete in one direction and
/// current in the other; a round adding a citation of a number that shifted adds a row here or
/// fails the build by name.
pub static SHIFTS: &[Shift] = &[
    Shift {
        clause: "7.2.3",
        earlier: Earlier::Numbered("7.2.2"),
        subject: "the character set; ISO 32000-1's own 7.2.3 is *Comments*",
    },
    Shift {
        clause: "9.6.4",
        earlier: Earlier::Numbered("9.6.5"),
        subject: "Type 3 fonts; ISO 32000-1's own 9.6.4 is *Font subsets*, the subclause \
                  ISO 32000-2 removed, which is why everything after it moved up one",
    },
    Shift {
        clause: "9.6.5",
        earlier: Earlier::Numbered("9.6.6"),
        subject: "character encoding; ISO 32000-1's own 9.6.5 is *Type 3 fonts*",
    },
    Shift {
        clause: "9.6.5.1",
        earlier: Earlier::Numbered("9.6.6.1"),
        subject: "the general subclause of character encoding; ISO 32000-1 has no 9.6.5.1",
    },
    Shift {
        clause: "9.6.5.2",
        earlier: Earlier::Numbered("9.6.6.2"),
        subject: "encodings for Type 1 fonts; ISO 32000-1 has no 9.6.5.2",
    },
    Shift {
        clause: "9.6.5.4",
        earlier: Earlier::Numbered("9.6.6.4"),
        subject: "encodings for TrueType fonts; ISO 32000-1 has no 9.6.5.4",
    },
    Shift {
        clause: "9.9.1",
        earlier: Earlier::NumberedAndRetitled("9.9"),
        subject: "the general subclause of embedded font programs, which ISO 32000-1 states \
                  unnumbered under 9.9 itself, *Embedded font programs*",
    },
    Shift {
        clause: "10.5",
        earlier: Earlier::Numbered("10.4"),
        subject: "transfer functions; ISO 32000-1's own 10.5 is *Halftones*",
    },
    Shift {
        clause: "10.6.5",
        earlier: Earlier::Numbered("10.5.5"),
        subject: "halftone dictionaries; ISO 32000-1's own 10.6.5 is *Automatic stroke \
                  adjustment*",
    },
    Shift {
        clause: "10.6.5.6",
        earlier: Earlier::Numbered("10.5.5.6"),
        subject: "type 5 halftones, whose table is 134 there and 132 here; ISO 32000-1 has no \
                  10.6.5.6",
    },
    Shift {
        clause: "12.6.4.17",
        earlier: Earlier::NumberedAndRetitled("12.6.4.16"),
        subject: "ECMAScript actions, which ISO 32000-1 calls *JavaScript actions*; its own \
                  12.6.4.17 does not exist",
    },
    Shift {
        clause: "12.7.5.2.3",
        earlier: Earlier::Numbered("12.7.4.2.3"),
        subject: "check boxes; ISO 32000-1 has no 12.7.5.2.3",
    },
    Shift {
        clause: "12.10",
        earlier: Earlier::Absent,
        subject: "geospatial features, which ISO 32000-1 does not describe; its own 12.10 is \
                  *Document requirements*, the subject ISO 32000-2 numbers 12.11, and the one \
                  citation of it serves a PDF/A-4 subclause",
    },
    Shift {
        clause: "12.8.3.4",
        earlier: Earlier::Absent,
        subject: "CAdES signatures, which are PDF 2.0's; ISO 32000-1 has no 12.8.3.4, and the \
                  rows that cite it bind PDF/A-4 alone",
    },
    Shift {
        clause: "12.8.5",
        earlier: Earlier::Absent,
        subject: "the document timestamp dictionary, which is PDF 2.0's; ISO 32000-1's own \
                  12.8.5 is *Legal content attestations*, and the rows that cite it bind \
                  PDF/A-4 alone",
    },
    Shift {
        clause: "14.7.5.4",
        earlier: Earlier::Numbered("14.7.4.4"),
        subject: "finding structure elements from content items; ISO 32000-1's own 14.7.5.4 \
                  is *User properties*",
    },
    Shift {
        clause: "14.13.1",
        earlier: Earlier::Absent,
        subject: "associated files, which are PDF 2.0's; ISO 32000-1 has no clause 14.13, and \
                  the rows that cite it bind PDF/A-4 alone",
    },
    Shift {
        clause: "14.13.2",
        earlier: Earlier::Absent,
        subject: "embedded associated files, which are PDF 2.0's; ISO 32000-1 has no clause \
                  14.13, and the rows that cite it bind PDF/A-4 alone",
    },
    Shift {
        clause: "Q.2",
        earlier: Earlier::Absent,
        subject: "the page-content step of Annex Q's transparency method; ISO 32000-1 has no \
                  Annex Q, which is why ISO 19005-2 states the method in an Annex A of its own \
                  (ADR 0972)",
    },
];

/// Where a cited ISO 32000-2 number is in ISO 32000-1:2008, if it is not simply there under the
/// same number and title.
#[must_use]
pub fn shift_of(clause: &str) -> Option<&'static Shift> {
    SHIFTS.iter().find(|shift| shift.clause == clause)
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::{Path, PathBuf};

    use super::{Earlier, SHIFTS, shift_of};

    /// The section sign, spelled by its code point so that the conformance gate's scanner —
    /// which reads this file too — does not meet a bare one in a character literal.
    const SIGN: char = '\u{a7}';

    /// The two editions, as `doc/md/` carries them.
    ///
    /// **Fails rather than skips where either is absent**, which is what the owner decided for
    /// `doc/md/` (`doc/habits/reading-the-specification.md`): the files are gitignored and
    /// unpacked from `doc/specifications.zip`, and a machine without them is a machine on which
    /// this test cannot say anything.
    fn markdown(file: &str) -> String {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../doc/md")
            .join(file);
        std::fs::read_to_string(&path).unwrap_or_else(|error| {
            panic!(
                "{} is not readable ({error}); unpack doc/specifications.zip",
                path.display()
            )
        })
    }

    /// A clause number as either edition writes one: digits and full stops, or an annex letter
    /// followed by them.
    fn is_clause_number(text: &str) -> bool {
        let mut parts = text.split('.');
        let Some(first) = parts.next() else {
            return false;
        };
        let first_is_annex = first.len() == 1 && first.bytes().all(|b| b.is_ascii_uppercase());
        let first_is_number = !first.is_empty() && first.bytes().all(|b| b.is_ascii_digit());
        if !(first_is_annex || first_is_number) {
            return false;
        }
        let rest: Vec<&str> = parts.collect();
        if rest
            .iter()
            .any(|part| part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()))
        {
            return false;
        }
        // A bare clause number is a number; a bare annex letter is a word.
        first_is_number || !rest.is_empty()
    }

    /// Two titles compared the way a reader compares them: case, punctuation and spacing aside.
    fn folded(title: &str) -> String {
        let mut out = String::new();
        let mut space = false;
        for c in title.chars() {
            if c.is_ascii_alphanumeric() {
                if space && !out.is_empty() {
                    out.push(' ');
                }
                space = false;
                out.push(c.to_ascii_lowercase());
            } else {
                space = true;
            }
        }
        out
    }

    /// ISO 32000-2's headings, number to title, first occurrence winning: one number
    /// (`14.8.4.7.3`) is printed twice.
    fn headings_of_iso_32000_2() -> BTreeMap<String, String> {
        let mut headings = BTreeMap::new();
        for line in markdown("ISO_32000-2_sponsored_EC3.md").lines() {
            let rest = line.trim_start_matches('#');
            if rest.len() == line.len() {
                continue;
            }
            let Some((number, title)) = rest.trim_start().split_once(' ') else {
                continue;
            };
            if is_clause_number(number) {
                headings
                    .entry(number.to_owned())
                    .or_insert_with(|| title.trim().to_owned());
            }
        }
        headings
    }

    /// ISO 32000-1:2008's headings, whose conversion has no `#` markers at all.
    ///
    /// A heading there is a line that is a clause number, a space and a title; the table of
    /// contents repeats each with dot leaders and a page number. A body line that merely
    /// *starts* with a number — "14.7.4.4, "Finding …" —, a comma, a lower-case continuation —
    /// is left out by requiring the title to open the way a title does. **A bare clause number
    /// is taken from the table of contents alone**: a body line opening with a bare number is
    /// far more often a table row ("14 PNG prediction …" is Table 8's) than clause 14, and the
    /// contents page lists every top-level clause.
    fn headings_of_iso_32000_1() -> BTreeMap<String, String> {
        let mut headings = BTreeMap::new();
        for line in markdown("ISO_32000-1_2008.md").lines() {
            let line = line.trim_end();
            if line.contains(" . ") {
                let Some((number, rest)) = line.split_once(char::is_whitespace) else {
                    continue;
                };
                if number.contains('.') || !is_clause_number(number) {
                    continue;
                }
                let title = rest.split(" . ").next().unwrap_or("").trim();
                headings
                    .entry(number.to_owned())
                    .or_insert_with(|| title.trim_end_matches('.').trim().to_owned());
                continue;
            }
            if line.len() > 120 {
                continue;
            }
            let Some((number, title)) = line.split_once(' ') else {
                continue;
            };
            if !is_clause_number(number) || !number.contains('.') {
                continue;
            }
            let opens_like_a_title = match title.as_bytes() {
                [first, ..] if first.is_ascii_uppercase() => true,
                [first, second, ..] if first.is_ascii_digit() => second.is_ascii_uppercase(),
                _ => false,
            };
            if !opens_like_a_title {
                continue;
            }
            headings
                .entry(number.to_owned())
                .or_insert_with(|| title.trim().to_owned());
        }
        headings
    }

    /// Every Rust source of this crate.
    fn sources() -> Vec<PathBuf> {
        fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
            let entries = std::fs::read_dir(dir)
                .unwrap_or_else(|error| panic!("{} is not listable: {error}", dir.display()));
            for entry in entries {
                let path = entry.expect("a directory entry is readable").path();
                if path.is_dir() {
                    walk(&path, out);
                } else if path.extension().is_some_and(|ext| ext == "rs") {
                    out.push(path);
                }
            }
        }
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut out = Vec::new();
        for dir in ["src", "examples", "tests"] {
            walk(&root.join(dir), &mut out);
        }
        out.sort();
        out
    }

    /// Every distinct clause number a `§` in this crate names, and every line that puts a `§`
    /// after the spelled-out earlier edition, which `doc/habits.md`'s convention forbids.
    ///
    /// A `§` with a backtick on each side is the character being named rather than a citation
    /// (ADR 0997 section 1), and is passed over.
    fn citations() -> (BTreeSet<String>, Vec<String>) {
        let mut cited = BTreeSet::new();
        let mut after_the_earlier_edition = Vec::new();
        for path in sources() {
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("{} is not readable: {error}", path.display()));
            for (line_number, line) in (1..).zip(text.lines()) {
                for (at, _) in line.match_indices(SIGN) {
                    let before = &line[..at];
                    let (_, after) = line[at..].split_at(SIGN.len_utf8());
                    if before.ends_with('`') && after.starts_with('`') {
                        continue;
                    }
                    let number: String = after
                        .chars()
                        .take_while(|c| c.is_ascii_alphanumeric() || *c == '.')
                        .collect();
                    let number = number.trim_end_matches('.');
                    if !is_clause_number(number) {
                        continue;
                    }
                    if before.trim_end().ends_with("ISO 32000-1:2008")
                        || before.trim_end().ends_with("ISO 32000-1")
                    {
                        after_the_earlier_edition.push(format!(
                            "{}:{}: `{}`",
                            path.display(),
                            line_number,
                            line.trim()
                        ));
                    }
                    cited.insert(number.to_owned());
                }
            }
        }
        (cited, after_the_earlier_edition)
    }

    /// The table is what makes the sentence *every citation resolves in the edition it names*
    /// true, and this holds it in both directions.
    ///
    /// **Forward**: every `§` in this crate either resolves in ISO 32000-1:2008 to a heading of
    /// the same title, or is a row of [`SHIFTS`] whose entry the 2008 headings bear out — the
    /// number it names exists there, under the same title for [`Earlier::Numbered`] and a
    /// different one for [`Earlier::NumberedAndRetitled`], and for [`Earlier::Absent`] the cited
    /// number is missing there or titled otherwise. **Backward**: a row of the table is still
    /// needed, so a corrected conversion of either text, or a citation that stopped being made,
    /// fails here rather than leaving a stale entry to mislead the next reader.
    #[test]
    fn every_citation_resolves_in_the_edition_a_part_two_file_adheres_to() {
        let later = headings_of_iso_32000_2();
        let earlier = headings_of_iso_32000_1();
        assert!(
            later.len() > 900 && earlier.len() > 700,
            "{} and {} headings read, so the parser is wrong rather than the table",
            later.len(),
            earlier.len()
        );
        let (cited, after_the_earlier_edition) = citations();
        assert!(
            after_the_earlier_edition.is_empty(),
            "a `§` after the spelled-out ISO 32000-1 is checked against ISO 32000-2 (ADR 0997 \
             section 2); write the 2008 number without a sign:\n{}",
            after_the_earlier_edition.join("\n")
        );
        let same = |a: &str, b: &str| folded(a) == folded(b);
        let mut problems = Vec::new();
        for number in &cited {
            let Some(title) = later.get(number) else {
                problems.push(format!("{SIGN}{number} is not a heading of ISO 32000-2"));
                continue;
            };
            let there = earlier.get(number);
            match shift_of(number).map(|shift| shift.earlier) {
                None => match there {
                    Some(other) if same(title, other) => {}
                    Some(other) => problems.push(format!(
                        "{SIGN}{number} is *{title}* in ISO 32000-2 and *{other}* in \
                         ISO 32000-1:2008, and `editions::SHIFTS` has no row for it"
                    )),
                    None => problems.push(format!(
                        "{SIGN}{number} (*{title}*) is no heading of ISO 32000-1:2008, and \
                         `editions::SHIFTS` has no row for it"
                    )),
                },
                Some(Earlier::Numbered(moved) | Earlier::NumberedAndRetitled(moved)) => {
                    let retitled = matches!(
                        shift_of(number).map(|shift| shift.earlier),
                        Some(Earlier::NumberedAndRetitled(_))
                    );
                    match earlier.get(moved) {
                        None => problems.push(format!(
                            "{SIGN}{number} is recorded as {moved} in ISO 32000-1:2008, which has \
                             no such heading"
                        )),
                        Some(other) if same(title, other) == retitled => problems.push(format!(
                            "{SIGN}{number} (*{title}*) is recorded as {moved} in ISO 32000-1:2008 \
                             (*{other}*): the titles {}, so the row's shape is wrong",
                            if retitled { "agree" } else { "differ" }
                        )),
                        Some(_) => {}
                    }
                    if there.is_some_and(|other| same(title, other)) {
                        problems.push(format!(
                            "{SIGN}{number} carries the same number and title in both editions, so \
                             its `editions::SHIFTS` row is stale"
                        ));
                    }
                }
                Some(Earlier::Absent) => {
                    if there.is_some_and(|other| same(title, other)) {
                        problems.push(format!(
                            "{SIGN}{number} (*{title}*) is a heading of ISO 32000-1:2008 under that \
                             title, so its `editions::SHIFTS` row is stale"
                        ));
                    }
                }
            }
        }
        for shift in SHIFTS {
            if !cited.contains(shift.clause) {
                problems.push(format!(
                    "`editions::SHIFTS` records {SIGN}{}, which nothing in this crate cites",
                    shift.clause
                ));
            }
        }
        assert!(problems.is_empty(), "\n{}", problems.join("\n"));
    }

    #[test]
    fn each_shift_is_recorded_once() {
        let mut seen = BTreeSet::new();
        for shift in SHIFTS {
            assert!(
                seen.insert(shift.clause),
                "{SIGN}{} is recorded twice",
                shift.clause
            );
        }
    }

    #[test]
    fn a_clause_number_is_digits_or_an_annex_letter_followed_by_them() {
        for number in ["7", "7.2.3", "14.7.5.4", "Q.2", "F.3.11"] {
            assert!(is_clause_number(number), "{number}");
        }
        for text in ["", "Q", "x.1", "7.", "7.a", "AB.1", "1.3.x"] {
            assert!(!is_clause_number(text), "{text:?}");
        }
    }

    #[test]
    fn titles_are_compared_case_and_punctuation_aside() {
        assert_eq!(folded("Character Set"), folded("Character set"));
        assert_eq!(
            folded("Colour Space: Special Considerations"),
            folded("colour space  special considerations")
        );
        assert_ne!(folded("Type 3 fonts"), folded("Font subsets"));
    }
}
