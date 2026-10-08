//! No comment in the tree's Rust sources names a round by its number.
//!
//! Not a conformance question; it lives here for `state_sections.rs`'s reason — this is the crate
//! whose gates read the repository's own files rather than a PDF.
//!
//! # What it guards
//!
//! `CLAUDE.md`'s comment rule ("Where knowledge lives") says a comment carries the current reason
//! and the ADR that argued it, never the round that changed it (ADR 1023). `CLAUDE.md`'s grep and
//! `spelled_ordinals.rs` hold the spelled ordinal and the capitalised `Session` form, and the
//! tree's own spelling of a round's history is neither: it is the word `round` followed by the
//! session's digits, as a parenthesis after a finding or as the subject of "measured" (ADR 1680
//! section 4). This sweep names every comment line that holds it — the word `round` or `rounds`,
//! in either case, at the start of a word, then whitespace, then three or four digits that end a
//! word. The whitespace may be a line break inside one comment, because a wrapped paragraph puts
//! the word at the end of one line and the number at the start of the next, and the tree had two
//! sites of exactly that shape when the sweep was written.
//!
//! # What it reads as a comment
//!
//! The source is lexed, not grepped, because the rule is about comments and a grep cannot tell a
//! comment from a string. A `//` comment of any kind and a `/* */` comment, nested, are read; a
//! string literal — plain, byte or C, and raw with any number of `#` — and a character literal are
//! skipped, so a fixture that writes the phrase as data is code, as `doc/adr/1680` section 4 rules
//! of the write-side corpus walk's title string. A lifetime is told from a character literal by the
//! quote that closes the literal and that a lifetime never has.
//!
//! # The held list
//!
//! A site in a file another round owns while this sweep lands stays for that file's owner, one
//! crate per round as ADR 1023 section 5 asks. Each is held by its count, with `==`, so a fall is
//! written down rather than banked (ADR 1637) and a file not on the list fails at its first site.
//! The list may only shrink: an entry whose count reaches zero leaves it.

#![expect(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "test code: a gate that cannot read its own repository has not found a defect, and \
              reporting that as one would be worse than stopping"
)]

use std::fs;
use std::path::{Path, PathBuf};

/// The files whose sites are their owners' to rewrite, with the count of comment lines each holds.
const HELD: [(&str, usize); 0] = [];

/// The directories whose Rust sources the rule reaches.
const ROOTS: [&str; 4] = ["crates", "tools", "raster", "fuzz"];

/// Where the repository root is, relative to this crate's manifest.
fn repository_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the manifest directory of a workspace member has two ancestors")
}

/// Every `.rs` file under `directory`, build output and hidden directories left out.
fn rust_sources(directory: &Path, into: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries {
        let path = entry.expect("a directory entry is readable").path();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        if name.starts_with('.') || name == "target" {
            continue;
        }
        if path.is_dir() {
            rust_sources(&path, into);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            into.push(path);
        }
    }
}

/// Whether `character` can continue an identifier, which is what decides where a word starts.
fn is_identifier(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}

/// Where the lexer is: in code, in a comment, or inside a literal it skips.
#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    /// Outside every comment and literal.
    Code,
    /// Inside a `//` comment, until the end of the line.
    LineComment,
    /// Inside a `/* */` comment, at this depth of nesting.
    BlockComment(usize),
    /// Inside a string literal that takes escapes.
    Text,
    /// Inside a raw string literal closed by a quote and this many `#`.
    Raw(usize),
}

/// The number of `#` a raw string opens with, if a raw string's `r` is at `at`: the `r` starts a
/// token (or follows a `b` or `c` that does), and the `#` run after it ends in a quote. A raw
/// identifier, `r#name`, has no quote there.
fn raw_opening(chars: &[char], at: usize) -> Option<usize> {
    let before = at
        .checked_sub(1)
        .and_then(|index| chars.get(index))
        .copied();
    let starts = match before {
        None => true,
        Some('b' | 'c') => at
            .checked_sub(2)
            .and_then(|index| chars.get(index))
            .is_none_or(|previous| !is_identifier(*previous)),
        Some(previous) => !is_identifier(previous),
    };
    if !starts {
        return None;
    }
    let hashes = chars
        .get(at.saturating_add(1)..)?
        .iter()
        .take_while(|character| **character == '#')
        .count();
    (chars.get(at.saturating_add(1).saturating_add(hashes)) == Some(&'"')).then_some(hashes)
}

/// The index just past a character literal opening at `at`, or just past the quote where it opens a
/// lifetime or a label instead.
fn past_quote(chars: &[char], at: usize) -> usize {
    let next = at.saturating_add(1);
    if chars.get(next) == Some(&'\\') {
        let mut cursor = next.saturating_add(2);
        while let Some(character) = chars.get(cursor) {
            match character {
                '\'' => return cursor.saturating_add(1),
                // Not a literal after all; the line break is the lexer's to count.
                '\n' => return cursor,
                _ => cursor = cursor.saturating_add(1),
            }
        }
        return cursor;
    }
    if chars.get(next.saturating_add(1)) == Some(&'\'') {
        return next.saturating_add(2);
    }
    next
}

/// Every comment character of `source`, each with the line it is on, and a break between two
/// comments that are not on consecutive lines. A comment's own marker — `//`, a doc comment's third
/// character, `/*` and `*/` — is not part of it.
fn comment_text(source: &str) -> Vec<Option<(char, usize)>> {
    let chars: Vec<char> = source.chars().collect();
    let mut found: Vec<Option<(char, usize)>> = Vec::new();
    let mut last_line = 0_usize;
    let mut keep = |found: &mut Vec<Option<(char, usize)>>, character: char, line: usize| {
        if line != last_line {
            if line != last_line.saturating_add(1) {
                found.push(None);
            }
            // The line break the comment's two lines were joined at.
            found.push(Some((' ', line)));
            last_line = line;
        }
        found.push(Some((character, line)));
    };
    let mut state = State::Code;
    let mut line = 1_usize;
    let mut at = 0_usize;
    while let Some(&character) = chars.get(at) {
        let next = chars.get(at.saturating_add(1)).copied();
        let mut step = 1_usize;
        match state {
            State::Code => match (character, next) {
                ('/', Some('/')) => {
                    state = State::LineComment;
                    step = if matches!(chars.get(at.saturating_add(2)), Some('/' | '!')) {
                        3
                    } else {
                        2
                    };
                }
                ('/', Some('*')) => {
                    state = State::BlockComment(1);
                    step = 2;
                }
                ('"', _) => state = State::Text,
                ('\'', _) => step = past_quote(&chars, at).saturating_sub(at),
                ('r', _) => {
                    if let Some(hashes) = raw_opening(&chars, at) {
                        state = State::Raw(hashes);
                        step = hashes.saturating_add(2);
                    }
                }
                _ => {}
            },
            State::LineComment => {
                if character == '\n' {
                    state = State::Code;
                } else {
                    keep(&mut found, character, line);
                }
            }
            State::BlockComment(depth) => match (character, next) {
                ('/', Some('*')) => {
                    state = State::BlockComment(depth.saturating_add(1));
                    step = 2;
                }
                ('*', Some('/')) => {
                    state = if depth > 1 {
                        State::BlockComment(depth.saturating_sub(1))
                    } else {
                        State::Code
                    };
                    step = 2;
                }
                ('\n', _) => {}
                _ => keep(&mut found, character, line),
            },
            State::Text => match character {
                '\\' => {
                    step = 2;
                    if next == Some('\n') {
                        line = line.saturating_add(1);
                    }
                }
                '"' => state = State::Code,
                _ => {}
            },
            State::Raw(hashes) => {
                let closes = character == '"'
                    && (1..=hashes)
                        .all(|offset| chars.get(at.saturating_add(offset)) == Some(&'#'));
                if closes {
                    state = State::Code;
                    step = hashes.saturating_add(1);
                }
            }
        }
        if character == '\n' {
            line = line.saturating_add(1);
        }
        at = at.saturating_add(step.max(1));
    }
    found
}

/// The lines of `source` on which a comment names a round by number, in order and each once.
fn round_numbers(source: &str) -> Vec<usize> {
    let text = comment_text(source);
    let character = |index: usize| -> Option<char> {
        text.get(index)
            .copied()
            .flatten()
            .map(|(character, _)| character.to_ascii_lowercase())
    };
    let mut lines: Vec<usize> = Vec::new();
    for (start, entry) in text.iter().enumerate() {
        let Some((_, line)) = entry else { continue };
        let starts_a_word = start
            .checked_sub(1)
            .and_then(character)
            .is_none_or(|previous| !is_identifier(previous));
        let word = "round"
            .chars()
            .enumerate()
            .all(|(offset, expected)| character(start.saturating_add(offset)) == Some(expected));
        if !starts_a_word || !word {
            continue;
        }
        let mut cursor = start.saturating_add(5);
        if character(cursor) == Some('s') {
            cursor = cursor.saturating_add(1);
        }
        let space_starts = cursor;
        while character(cursor).is_some_and(char::is_whitespace) {
            cursor = cursor.saturating_add(1);
        }
        let digits_start = cursor;
        while character(cursor).is_some_and(|digit| digit.is_ascii_digit()) {
            cursor = cursor.saturating_add(1);
        }
        let digits = cursor.saturating_sub(digits_start);
        let ends_a_word = character(cursor).is_none_or(|after| !is_identifier(after));
        if digits_start > space_starts
            && (3..=4).contains(&digits)
            && ends_a_word
            && lines.last() != Some(line)
        {
            lines.push(*line);
        }
    }
    lines
}

#[test]
fn the_reader_is_the_shape_it_states() {
    let planted = r##"fn planted<'a>(x: &'a str) -> char { // measured in round 911
    /// Round 908 stopped it there.
    // the subtraction is the
    // rounds
    // 935 and 938 made
    let title = "round 909 wrote this"; let raw = r#"round 101 "quoted" again"#;
    let bytes = b"round 102"; let quote = '"'; let tick = '\''; // round 1405's runs
    /* outer /* inner */ still a comment, round 923 */ let after = 1;
    // around 300 times, round 12, round 12345, rounding 400
    let r#type = 1; // RFC 0008 round 1371's shape
    // the end of the round
    let code = 1;
    // 902 is a number on a comment after code
}"##;
    assert_eq!(
        round_numbers(planted),
        [1, 2, 4, 7, 8, 10],
        "the reader is not the shape it states"
    );
}

#[test]
fn no_comment_names_a_round_by_number() {
    let root = repository_root();
    let mut sources = Vec::new();
    for directory in ROOTS {
        rust_sources(&root.join(directory), &mut sources);
    }
    sources.sort();
    let mut owed = Vec::new();
    let mut counted: Vec<(String, usize)> = Vec::new();
    let mut total = 0_usize;
    for path in &sources {
        let text = fs::read_to_string(path).expect("a source file of the tree is readable");
        let lines = round_numbers(&text);
        if lines.is_empty() {
            continue;
        }
        let relative = path
            .strip_prefix(root)
            .unwrap_or(path)
            .display()
            .to_string();
        total = total.saturating_add(lines.len());
        let held = HELD.iter().any(|(file, _)| *file == relative);
        for number in &lines {
            let line = text
                .lines()
                .nth(number.saturating_sub(1))
                .unwrap_or_default()
                .trim();
            let located = format!("{relative}:{number}: {line}");
            if held {
                println!("  held  {located}");
            } else {
                owed.push(located);
            }
        }
        counted.push((relative, lines.len()));
    }
    println!(
        "{} Rust source(s) read under {ROOTS:?}; {total} comment line(s) name a round by number, \
         {} of them in {} held file(s)",
        sources.len(),
        total.saturating_sub(owed.len()),
        HELD.len()
    );
    assert!(
        sources.len() > 1000,
        "{} source(s) read: the population is not the tree",
        sources.len()
    );
    assert!(
        owed.is_empty(),
        "a comment carries the current reason and the ADR that argued it, never the round that \
         changed it (ADR 1023); rewrite these as what is, citing the ADR the round wrote:\n{}",
        owed.join("\n")
    );
    let moved: Vec<String> = HELD
        .iter()
        .filter_map(|(file, held)| {
            let now = counted
                .iter()
                .find(|(path, _)| path == file)
                .map_or(0, |(_, count)| *count);
            (now != *held).then(|| format!("{file}: held at {held}, counted {now}"))
        })
        .collect();
    assert!(
        moved.is_empty(),
        "a held file's count moved; write the new count down, and take a file at zero off the \
         list (ADR 1637):\n{}",
        moved.join("\n")
    );
}
