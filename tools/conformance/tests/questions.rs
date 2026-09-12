//! `doc/questions/`'s own rule, checked: one number, one question, and an answer that can be
//! addressed to it.
//!
//! Not a conformance question, and it lives here for the reason `bounded.rs`, `sandbox_gates.rs`,
//! `submodules.rs` and `workspaces.rs` do: this is the crate whose gates read the repository's own
//! files rather than a PDF, and `cargo test -p conformance` is the last line of `doc/todo/02` §2's
//! sequence — which every merge round runs, and a merge is where this defect is made.
//!
//! # What it guards
//!
//! The directory's convention is that a question is a `Q` file and the owner answers it with an
//! **`A` file of the same name**, whose three-line header — `Status`, `Given`, `Owes` — is the
//! index, because an answer is not always a completion: it can defer (`A03`), dissolve the
//! question (`A19`, `A49`), answer half of it (`A51`), or be replaced by a later one (`A15` and
//! `A17`, un-held by `A46`). That is the 2026-09-12 amendment; until it, an `A` file's existence
//! was read as *answered*, and the answers here that were not completions sat in the index as
//! though they were. A `Q` with no `A` is still open, and no round needs to read anything else.
//! It rests on a number identifying exactly one question — and on
//! 2026-09-04 two rounds took `Q27`, each by reading `ls doc/questions/` on a branch that could not
//! see the other's, and the collision reached `main` three rounds later in a merge with every gate
//! green. Nothing dangled, because each reference cited a filename rather than a number; what broke
//! is that **one of the two could never be answered** — an `A27` would have named a question and
//! not said which. Session 934 renumbered one and wrote [ADR 0908]; this is the half that keeps it
//! from happening again quietly.
//!
//! [ADR 0908]: ../../../doc/adr/0908-two-questions-called-q27.md
//!
//! # What it does not guard
//!
//! It cannot make the collision *impossible*: two branches still cannot see each other, and the
//! allocator is outside the tree — the block of numbers a round is given in its instruction. What
//! this makes impossible is a duplicate surviving the merge that creates it, which is the step
//! where somebody is looking. `doc/questions/README.md` states both halves.
//!
//! **Gaps are not a defect and are not checked.** A round takes a number from its own reserved
//! block, so the numbers here are sparse by construction, and a test that demanded a run from 1
//! would fail every round that used its block honestly.

#![expect(
    clippy::expect_used,
    reason = "test code: a gate that cannot read its own repository's questions directory has \
              not found a defect, and reporting that as one would be worse than stopping"
)]
#![expect(
    clippy::print_stdout,
    reason = "the gate prints the index it checked, which is what makes a passing run worth \
              reading"
)]

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// The directory this test is about, relative to the repository root.
const QUESTIONS: &str = "doc/questions";

/// The one file in it that is not a question.
const README: &str = "README.md";

fn repository_root() -> &'static Path {
    // `CARGO_MANIFEST_DIR` is `<root>/tools/conformance`, so two levels up is the root. This
    // cannot fail for a crate that is in the workspace, which is the only way this test runs.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the manifest directory of a workspace member has two ancestors")
}

/// One file of `doc/questions/`, split into the three parts its name is made of.
struct Entry {
    /// `Q` for a question, `A` for the owner's answer.
    letter: char,
    /// The number the two share, as written — so that `Q7` and `Q07` are told apart rather than
    /// silently reconciled, which is a collision of its own shape.
    number: String,
    /// Everything after the number's hyphen, without the `.md`.
    slug: String,
    /// The whole file name, for the report.
    file: String,
}

/// Split `Q27-a-font-the-file-does-not-carry.md` into its letter, number and slug.
///
/// Returns `None` for anything that is not `<letter><digits>-<slug>.md`, which the caller reports:
/// a name this cannot read is a name no round can resolve an answer against either.
fn parse(file: &str) -> Option<Entry> {
    let stem = file.strip_suffix(".md")?;
    let mut characters = stem.chars();
    let letter = characters.next()?;
    if letter != 'Q' && letter != 'A' {
        return None;
    }
    let rest = characters.as_str();
    let hyphen = rest.find('-')?;
    let (number, slug) = rest.split_at(hyphen);
    if number.is_empty() || !number.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let slug = slug.get(1..)?;
    if slug.is_empty() {
        return None;
    }
    Some(Entry {
        letter,
        number: number.to_owned(),
        slug: slug.to_owned(),
        file: file.to_owned(),
    })
}

fn read_directory(root: &Path) -> Vec<PathBuf> {
    let directory = root.join(QUESTIONS);
    let mut files: Vec<PathBuf> = std::fs::read_dir(&directory)
        .expect("doc/questions/ is a directory of this repository")
        .map(|entry| entry.expect("a readable directory entry").path())
        .collect();
    files.sort();
    files
}

#[test]
fn every_question_has_a_number_of_its_own() {
    let root = repository_root();
    let mut wrong = String::new();

    // Keyed by number so that a duplicate is a value with more than one entry, and ordered so that
    // the report reads in the same order as the directory.
    let mut questions: BTreeMap<String, Vec<Entry>> = BTreeMap::new();
    let mut answers: BTreeMap<String, Vec<Entry>> = BTreeMap::new();

    for path in read_directory(root) {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .expect("a file name that is UTF-8")
            .to_owned();
        if name == README {
            continue;
        }
        let Some(entry) = parse(&name) else {
            let _ = writeln!(
                wrong,
                "  {QUESTIONS}/{name} is not <Q or A><number>-<slug>.md, so no answer can be \
                 addressed to it"
            );
            continue;
        };
        match entry.letter {
            'Q' => questions
                .entry(entry.number.clone())
                .or_default()
                .push(entry),
            _ => answers.entry(entry.number.clone()).or_default().push(entry),
        }
    }

    // The defect this test was written for. Two questions sharing a number is not a tidiness
    // matter: an answer names its question by number and slug, and one of the two could never
    // be answered.
    for (number, entries) in &questions {
        if entries.len() > 1 {
            let names: Vec<&str> = entries.iter().map(|e| e.file.as_str()).collect();
            let _ = writeln!(
                wrong,
                "  the number {number} is taken by {} questions — {} — so an A{number} would name \
                 a question and not say which. Renumber one out of the round's own reserved block \
                 (doc/questions/README.md)",
                entries.len(),
                names.join(", ")
            );
        }
    }
    for (number, entries) in &answers {
        if entries.len() > 1 {
            let names: Vec<&str> = entries.iter().map(|e| e.file.as_str()).collect();
            let _ = writeln!(
                wrong,
                "  the number {number} is taken by {} answers — {}",
                entries.len(),
                names.join(", ")
            );
        }
    }

    // The other half of the convention, and the half a rename breaks: an answer's name is its
    // question's with the letter changed, so a slug that drifted is an answer nobody's `ls` pairs
    // up.
    for (number, entries) in &answers {
        let Some(question) = questions.get(number).and_then(|q| q.first()) else {
            let _ = writeln!(
                wrong,
                "  {QUESTIONS}/{} answers Q{number}, which is not in this directory",
                entries[0].file
            );
            continue;
        };
        for answer in entries {
            if answer.slug != question.slug {
                let _ = writeln!(
                    wrong,
                    "  {QUESTIONS}/{} should be named for the question it answers — {} with A for \
                     Q",
                    answer.file, question.file
                );
            }
        }
    }

    assert!(wrong.is_empty(), "\n{wrong}");

    let open: Vec<&str> = questions
        .iter()
        .filter(|(number, _)| !answers.contains_key(*number))
        .filter_map(|(_, entries)| entries.first().map(|entry| entry.file.as_str()))
        .collect();
    println!(
        "{QUESTIONS}/ holds {} questions with a number apiece, {} of them with an answer beside \
         them and {} open:",
        questions.len(),
        answers.len(),
        open.len()
    );
    for file in &open {
        println!("  {file}");
    }
}

/// The status an answer's header states, as `doc/questions/README.md`'s 2026-09-12 amendment
/// defines it.
///
/// The vocabulary is closed because every word in it is already load-bearing somewhere in this
/// directory: `complete` is most of them, `deferred` is `A03` — "declined for now" — `void` is
/// `A19` and `A49`, whose questions had dissolved, `partial` is `A51`, which answered half of a
/// purchase question and named the remainder, and `A15` and `A17` are superseded by `Q46`, whose
/// answer says "the 'on hold' of A15 and A17 is over". A status outside the set is a state no
/// round can act on, which is why it fails here rather than passing as prose.
enum Status {
    Complete,
    Partial,
    Deferred,
    Void,
    /// A later question whose answer replaced this one. The number is as written, so that `Q7`
    /// and `Q07` stay told apart, exactly as the question numbers do.
    Superseded(String),
}

impl Status {
    /// The word the header states, for the printed index.
    fn as_str(&self) -> &str {
        match self {
            Status::Complete => "complete",
            Status::Partial => "partial",
            Status::Deferred => "deferred",
            Status::Void => "void",
            Status::Superseded(_) => "superseded",
        }
    }
}

/// The header lines that are the index: what the answer did to its question, whose word the body
/// is, and what work the answer commands that no round has yet shown done.
struct Header {
    status: Status,
    owes: String,
}

/// Parse an answer's first three lines, reporting the first that is missing or malformed rather
/// than every fault at once — a round fixing one header fixes the neighbours in the same read.
fn parse_header(entry: &Entry, lines: [&str; 3]) -> Result<Header, String> {
    let file = entry.file.as_str();
    let [status, given, owes] = lines;

    let value = status.strip_prefix("Status: ").ok_or_else(|| {
        format!(
            "  {QUESTIONS}/{file} opens with {status:?} rather than \"Status: …\", so the \
             directory's index cannot read it (doc/questions/README.md)"
        )
    })?;
    let status = match value {
        "complete" => Status::Complete,
        "partial" => Status::Partial,
        "deferred" => Status::Deferred,
        "void" => Status::Void,
        _ => {
            let Some(number) = value.strip_prefix("superseded by Q") else {
                return Err(format!(
                    "  {QUESTIONS}/{file} states the status {value:?}, which is not one of \
                     complete, partial, deferred, void or superseded by Q<number> \
                     (doc/questions/README.md)"
                ));
            };
            if number.is_empty() || !number.chars().all(|c| c.is_ascii_digit()) {
                return Err(format!(
                    "  {QUESTIONS}/{file} is superseded by {number:?}, which is not a number an \
                     answer can point at"
                ));
            }
            Status::Superseded(number.to_owned())
        }
    };

    let value = given.strip_prefix("Given: ").ok_or_else(|| {
        format!(
            "  {QUESTIONS}/{file} has no \"Given:\" line second, so whose word the body is \
             stays unwritten (doc/questions/README.md)"
        )
    })?;
    let Some((date, medium)) = value.split_once(", ") else {
        return Err(format!(
            "  {QUESTIONS}/{file} states {value:?} rather than \"<date>, <medium>\""
        ));
    };
    let date_is_a_date = date.len() == 10
        && date.as_bytes().iter().enumerate().all(|(at, byte)| {
            byte.is_ascii_digit() || ((at == 4 || at == 7) && *byte == b'-')
        });
    if !date_is_a_date {
        return Err(format!(
            "  {QUESTIONS}/{file} gives {date:?}, which is not YYYY-MM-DD"
        ));
    }
    if medium != "by the owner's hand" && medium != "in conversation — transcribed by the round" {
        return Err(format!(
            "  {QUESTIONS}/{file} gives the medium {medium:?}, which is neither \"by the owner's \
             hand\" nor \"in conversation — transcribed by the round\""
        ));
    }

    let value = owes.strip_prefix("Owes: ").ok_or_else(|| {
        format!(
            "  {QUESTIONS}/{file} has no \"Owes:\" line third, so work the answer commands has \
             nowhere to be tracked (doc/questions/README.md)"
        )
    })?;
    if value != "none" && value.split(';').all(|item| item.trim().is_empty()) {
        return Err(format!(
            "  {QUESTIONS}/{file} owes {value:?}, which names no item — \"Owes: none\" is the \
             line for that"
        ));
    }

    Ok(Header {
        status,
        owes: value.to_owned(),
    })
}

#[test]
fn every_answer_says_what_it_left_open() {
    let root = repository_root();
    let mut wrong = String::new();

    // The questions, keyed by number as written, for the one check that needs them: a superseded
    // answer points at a later question, and the pointer has to resolve.
    let mut questions: BTreeMap<String, Vec<Entry>> = BTreeMap::new();
    let mut answers: Vec<Entry> = Vec::new();
    for path in read_directory(root) {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .expect("a file name that is UTF-8")
            .to_owned();
        if name == README {
            continue;
        }
        let Some(entry) = parse(&name) else {
            continue;
        };
        match entry.letter {
            'Q' => questions
                .entry(entry.number.clone())
                .or_default()
                .push(entry),
            _ => answers.push(entry),
        }
    }

    let mut headers: Vec<(String, Header)> = Vec::new();
    for entry in &answers {
        let path = root.join(QUESTIONS).join(&entry.file);
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("could not read {path:?}: {error}"));
        let mut lines = text.lines();
        let parsed = match [lines.next(), lines.next(), lines.next()] {
            [Some(status), Some(given), Some(owes)] => parse_header(entry, [status, given, owes]),
            _ => Err(format!(
                "  {QUESTIONS}/{} carries fewer than the three header lines every answer opens \
                 with (doc/questions/README.md)",
                entry.file
            )),
        };
        match parsed {
            Ok(header) => headers.push((entry.number.clone(), header)),
            Err(line) => {
                let _ = writeln!(wrong, "{line}");
            }
        }
    }

    for (number, header) in &headers {
        if let Status::Superseded(target) = &header.status {
            if !questions.contains_key(target) {
                let _ = writeln!(
                    wrong,
                    "  {QUESTIONS}/A{number} is superseded by Q{target}, which is not in this \
                     directory"
                );
            } else if target.parse::<u32>().expect("digits, checked where the header was parsed")
                <= number.parse::<u32>().expect("the same parse() that read the file name")
            {
                let _ = writeln!(
                    wrong,
                    "  {QUESTIONS}/A{number} is superseded by Q{target}, which does not come \
                     after it — an answer is replaced by a later one, not an earlier one"
                );
            }
        }
    }

    assert!(wrong.is_empty(), "\n{wrong}");

    // The index a round reads instead of opening every file, and what `tools/state.sh questions`
    // prints. The complete answers are the quiet majority; the others name themselves, and the
    // owed items name the work no round has yet shown done.
    let owed: Vec<String> = headers
        .iter()
        .filter(|(_, header)| header.owes != "none")
        .map(|(number, header)| format!("owed by A{number}: {}", header.owes))
        .collect();
    let count = |word: &str| {
        headers
            .iter()
            .filter(|(_, header)| header.status.as_str() == word)
            .count()
    };
    println!(
        "{QUESTIONS}/ holds {} answers: {} complete, {} partial, {} deferred, {} void, {} \
         superseded; {} items owed:",
        headers.len(),
        count("complete"),
        count("partial"),
        count("deferred"),
        count("void"),
        count("superseded"),
        owed.len()
    );
    for (number, header) in &headers {
        match &header.status {
            Status::Complete => {}
            Status::Superseded(target) => println!("  A{number} superseded by Q{target}"),
            _ => println!("  A{number} {}", header.status.as_str()),
        }
    }
    for item in &owed {
        println!("{item}");
    }
}
