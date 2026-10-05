//! A `tools/state.sh` section reads the tree and never writes it.
//!
//! Not a conformance question; it lives here for `state_sections.rs`'s reason — this is the crate
//! whose gates read the repository's own files rather than a PDF.
//!
//! # What it guards
//!
//! `CLAUDE.md` names `tools/state.sh` as *the command that counts*, and a command that counts may
//! not write. Six rounds edit one worktree at once and the merge fast-forwards the main checkout
//! from it, so a counting run that rewrites a shared file — byte-identical or not — is a write
//! nobody asked for, and the next fast-forward refuses over it. The `ledger` section did exactly
//! that: it ran the generator, which rewrote `doc/conformance/ledger.toml` in canonical key order
//! on every run. The generator now writes only under `--write` (ADR 1487).
//!
//! # How, and what it cannot see
//!
//! A section is checked by construction rather than by running it, because a run would put
//! `cargo` inside `cargo test` on one build directory, and because six rounds editing the tree
//! make any before-and-after comparison of it a measurement of the siblings. The script is lexed —
//! comments, here-documents and quoted text set aside, a `$( … )` inside double quotes kept as
//! code — and every command in it is asked five things:
//!
//! 1. **A redirection writes only to `/dev/null`, to another descriptor, or to a file the script
//!    made with `mktemp`.**
//! 2. **No command whose job is to write is run**: `rm` (except of a `mktemp` file), `cp`, `mv`,
//!    `touch`, `tee`, `mkdir`, `ln`, `sed -i`, and the `git` subcommands that change a repository.
//! 3. **`git status` and `git diff` carry `--no-optional-locks`**, in the script and in every
//!    `tools/*.py` it runs: without it both refresh the index's stat cache and take `index.lock`
//!    to write it back — in a worktree being merged, or in the owner's checkout.
//! 4. **A `tools/*.py` is run with `PYTHONDONTWRITEBYTECODE=1`** (trap 69).
//! 5. **A `conformance` binary that can write is run without `--write`, and accepts it** — so its
//!    writing is behind a flag no section passes.
//!
//! What this cannot see is stated rather than hidden: a `cargo test` a section runs is a test
//! binary whose writes are its own (this crate's tests write under `std::env::temp_dir()` only,
//! which an `strace` of every `quick` section confirmed when ADR 1487 was written), and a binary of
//! another package is read only through rule 2's words. The walks outside `quick` write what their
//! gates write, and are the merge's.
//!
//! **`tools/batch.sh install` is not a state section and is not read here.** It is a batch
//! subcommand, run once by the orchestrator after the fast-forward, and it writes by design —
//! exactly the main checkout's gitignored `target/`, the programs and `target/installed-from` —
//! which `tests/batch.rs` holds against a throwaway repository (ADR 1511). `tools/state.sh
//! binaries` reads what it wrote and writes nothing.

#![expect(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "test code: a gate that cannot read its own repository has not found a defect, and \
              reporting that as one would be worse than stopping"
)]

use std::collections::BTreeSet;
use std::path::Path;

/// Where the repository root is, relative to this crate's manifest.
fn repository_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the manifest directory of a workspace member has two ancestors")
}

/// The calls by which a Rust source writes a file.
const WRITE_CALLS: [&str; 7] = [
    "fs::write",
    "File::create",
    "OpenOptions",
    "create_dir",
    "fs::rename",
    "remove_file",
    "fs::copy",
];

/// Commands whose job is to write.
const WRITING_COMMANDS: [&str; 7] = ["rm", "cp", "mv", "touch", "tee", "mkdir", "ln"];

/// The `git` subcommands that change a repository, its index or its working tree.
const WRITING_GIT: [&str; 16] = [
    "add",
    "commit",
    "checkout",
    "switch",
    "restore",
    "stash",
    "reset",
    "rm",
    "mv",
    "merge",
    "rebase",
    "apply",
    "clean",
    "worktree",
    "update-index",
    "gc",
];

/// The `git` subcommands that take `index.lock` to refresh the stat cache unless told not to.
const LOCKING_GIT: [&str; 2] = ["status", "diff"];

/// Lexes a shell script into its code: comments, here-document bodies and quoted text removed.
///
/// A quoted span that is a lone variable — `"$sorted"`, `"${file}"` — is kept as that variable,
/// because a redirection's target is written that way; a `$( … )` inside double quotes is kept as
/// code, because it runs. Every other quoted span becomes `Q`, so a heading or a filter that
/// mentions a command is not mistaken for running one. A backslash-newline joins the two lines.
fn code_of(script: &str) -> String {
    let chars: Vec<char> = script.chars().collect();
    let mut position = 0;
    lex(&chars, &mut position, false)
}

/// The recursive half of [`code_of`]: lexes from `position` to the end, or, when `nested`, to the
/// `)` that closes the `$(` the caller consumed.
fn lex(chars: &[char], position: &mut usize, nested: bool) -> String {
    let mut out = String::new();
    let mut depth = 0usize;
    let mut heredocs: Vec<String> = Vec::new();
    while let Some(&c) = chars.get(*position) {
        *position = position.saturating_add(1);
        match c {
            '\\' => {
                if chars.get(*position) == Some(&'\n') {
                    out.push(' ');
                } else if let Some(&next) = chars.get(*position) {
                    out.push(next);
                }
                *position = position.saturating_add(1);
            }
            '#' if out.is_empty() || out.ends_with(|p: char| p.is_whitespace() || p == ';') => {
                while chars.get(*position).is_some_and(|&n| n != '\n') {
                    *position = position.saturating_add(1);
                }
            }
            '\'' => {
                let start = *position;
                while chars.get(*position).is_some_and(|&n| n != '\'') {
                    *position = position.saturating_add(1);
                }
                let content: String = chars[start..(*position).min(chars.len())].iter().collect();
                *position = position.saturating_add(1);
                push_quoted(&mut out, &content);
            }
            '"' => {
                let (content, code) = double_quoted(chars, position);
                push_quoted(&mut out, &content);
                out.push_str(&code);
            }
            '$' if chars.get(*position) == Some(&'(') => {
                *position = position.saturating_add(1);
                out.push_str(" ; ");
                out.push_str(&lex(chars, position, true));
                out.push_str(" ; ");
            }
            '(' => {
                depth = depth.saturating_add(1);
                out.push(c);
            }
            ')' if nested && depth == 0 => return out,
            ')' => {
                depth = depth.saturating_sub(1);
                out.push(c);
            }
            '<' if chars.get(*position) == Some(&'<')
                && chars.get(position.saturating_add(1)) != Some(&'<') =>
            {
                *position = position.saturating_add(1);
                if chars.get(*position) == Some(&'-') {
                    *position = position.saturating_add(1);
                }
                let start = *position;
                while chars
                    .get(*position)
                    .is_some_and(|&n| n == '\'' || n == '"' || n.is_alphanumeric() || n == '_')
                {
                    *position = position.saturating_add(1);
                }
                let word: String = chars[start..*position]
                    .iter()
                    .filter(|&&n| n != '\'' && n != '"')
                    .collect();
                heredocs.push(word);
            }
            '\n' => {
                out.push('\n');
                for terminator in heredocs.drain(..) {
                    skip_here_document(chars, position, &terminator);
                }
            }
            _ => out.push(c),
        }
    }
    out
}

/// Reads a double-quoted span from just after its opening quote: its text, and the code of every
/// `$( … )` inside it, which runs.
fn double_quoted(chars: &[char], position: &mut usize) -> (String, String) {
    let mut content = String::new();
    let mut code = String::new();
    while let Some(&n) = chars.get(*position) {
        *position = position.saturating_add(1);
        match n {
            '"' => break,
            '\\' => {
                if let Some(&escaped) = chars.get(*position) {
                    content.push(escaped);
                }
                *position = position.saturating_add(1);
            }
            '$' if chars.get(*position) == Some(&'(') => {
                *position = position.saturating_add(1);
                code.push_str(" ; ");
                code.push_str(&lex(chars, position, true));
                code.push_str(" ; ");
                content.push_str("$()");
            }
            _ => content.push(n),
        }
    }
    (content, code)
}

/// Skips a here-document's body, from the line after its opener to the line holding `terminator`.
fn skip_here_document(chars: &[char], position: &mut usize, terminator: &str) {
    loop {
        let start = *position;
        while chars.get(*position).is_some_and(|&n| n != '\n') {
            *position = position.saturating_add(1);
        }
        let line: String = chars[start..*position].iter().collect();
        *position = position.saturating_add(1);
        if line.trim() == terminator || *position >= chars.len() {
            break;
        }
    }
}

/// Pushes a quoted span's stand-in: the variable it names, or `Q`.
fn push_quoted(out: &mut String, content: &str) {
    let bare = content
        .strip_prefix("${")
        .and_then(|rest| rest.strip_suffix('}'))
        .or_else(|| content.strip_prefix('$'));
    match bare {
        Some(name) if !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_') => {
            out.push('$');
            out.push_str(name);
        }
        _ => out.push('Q'),
    }
}

/// The variables the script assigns from `mktemp`, as `$name`.
fn temporaries(code: &str) -> BTreeSet<String> {
    code.split(|c: char| c.is_whitespace() || c == ';')
        .filter_map(|word| word.strip_suffix('='))
        .map(|name| format!("${name}"))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter(|name| {
            let assignment = format!("{}= ; mktemp", &name[1..]);
            code.contains(&assignment)
        })
        .collect()
}

/// Every simple command in lexed code, as its words.
fn commands(code: &str) -> Vec<Vec<String>> {
    code.split(['\n', ';', '|', '&', '(', ')', '{', '}', '`'])
        .map(|command| {
            command
                .split_whitespace()
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .filter(|words| !words.is_empty())
        .collect()
}

/// Every redirection target in lexed code that is not `/dev/null`, a descriptor or a temporary.
fn stray_redirections(code: &str, temporaries: &BTreeSet<String>) -> Vec<String> {
    let mut found = Vec::new();
    let chars: Vec<char> = code.chars().collect();
    let mut index = 0;
    while index < chars.len() {
        if chars[index] != '>' {
            index = index.saturating_add(1);
            continue;
        }
        // `<(…)`'s mirror and a comparison are not redirections.
        if matches!(chars.get(index.saturating_add(1)), Some('(' | '=')) {
            index = index.saturating_add(2);
            continue;
        }
        let mut cursor = index;
        while matches!(chars.get(cursor), Some('>')) {
            cursor = cursor.saturating_add(1);
        }
        while chars.get(cursor).is_some_and(|c| *c == ' ' || *c == '\t') {
            cursor = cursor.saturating_add(1);
        }
        let start = cursor;
        while chars
            .get(cursor)
            .is_some_and(|c| !c.is_whitespace() && !matches!(c, ';' | '|' | ')'))
        {
            cursor = cursor.saturating_add(1);
        }
        let target: String = chars[start..cursor].iter().collect();
        let allowed =
            target.starts_with('&') || target == "/dev/null" || temporaries.contains(&target);
        if !allowed {
            found.push(format!("a redirection into `{target}`"));
        }
        index = cursor.max(index.saturating_add(1));
    }
    found
}

/// The script's words, the `tools/*.py` it runs, and the `conformance` binaries it runs, read
/// against the five rules; each finding is a sentence naming the command.
fn findings(script: &str, root: &Path) -> (Vec<String>, usize) {
    let code = code_of(script);
    let temporaries = temporaries(&code);
    let mut found = stray_redirections(&code, &temporaries);
    let all = commands(&code);
    for words in &all {
        let line = words.join(" ");
        let has = |word: &str| words.iter().any(|w| w == word);
        for (index, word) in words.iter().enumerate() {
            let next = words.get(index.saturating_add(1)).map(String::as_str);
            if WRITING_COMMANDS.contains(&word.as_str()) && index == first_command(words) {
                let of_temporary = word == "rm"
                    && words[index.saturating_add(1)..]
                        .iter()
                        .filter(|w| !w.starts_with('-'))
                        .all(|w| temporaries.contains(w));
                if !of_temporary {
                    found.push(format!("`{line}` writes"));
                }
            }
            if word == "sed"
                && words[index.saturating_add(1)..]
                    .iter()
                    .any(|w| w.starts_with("-i"))
            {
                found.push(format!("`{line}` edits in place"));
            }
            if word == "git" {
                let subcommand = words[index.saturating_add(1)..]
                    .iter()
                    .scan(false, |skip, w| {
                        let this = *skip;
                        *skip = w == "-C";
                        Some((this, w))
                    })
                    .find(|(skipped, w)| !skipped && !w.starts_with('-'))
                    .map(|(_, w)| w.as_str());
                if let Some(subcommand) = subcommand {
                    if WRITING_GIT.contains(&subcommand) {
                        found.push(format!("`{line}` changes the repository"));
                    }
                    if LOCKING_GIT.contains(&subcommand) && !has("--no-optional-locks") {
                        found.push(format!("`{line}` takes index.lock"));
                    }
                }
            }
            if word == "python3"
                && let Some(script) = next.filter(|s| s.starts_with("tools/"))
            {
                if !has("PYTHONDONTWRITEBYTECODE=1") {
                    found.push(format!("`{line}` may write __pycache__ (trap 69)"));
                }
                if let Ok(source) = std::fs::read_to_string(root.join(script))
                    && source.contains("\"git\"")
                    && LOCKING_GIT
                        .iter()
                        .any(|sub| source.contains(&format!("\"{sub}\"")))
                    && !source.contains("\"--no-optional-locks\"")
                {
                    found.push(format!(
                        "{script} runs git status or diff, which takes index.lock"
                    ));
                }
            }
        }
        if let Some(binary) = conformance_binary(words) {
            let path = root.join(format!("tools/conformance/src/bin/{binary}.rs"));
            let source = std::fs::read_to_string(&path).unwrap_or_default();
            let writes = WRITE_CALLS.iter().any(|call| source.contains(call));
            if writes && (has("--write") || !source.contains("\"--write\"")) {
                found.push(format!(
                    "`{line}` runs a binary that writes, without its writing behind `--write`"
                ));
            }
        }
    }
    (found, all.len())
}

/// The index of the command word: past assignments and the wrappers this script runs a command
/// through (`run` and `prose_line` with their quoted title and filter, `env`, `tools/bounded.sh --`).
fn first_command(words: &[String]) -> usize {
    words
        .iter()
        .position(|word| {
            !(word.contains('=')
                || word == "Q"
                || word == "env"
                || word == "run"
                || word == "prose_line"
                || word == "command"
                || word == "tools/bounded.sh"
                || word == "--")
        })
        .unwrap_or(0)
}

/// The binary a `cargo run … -p conformance … --bin <name>` command runs, if it is one.
fn conformance_binary(words: &[String]) -> Option<&str> {
    let cargo = words.iter().position(|w| w == "cargo")?;
    let rest = &words[cargo..];
    let is_run = rest.get(1).is_some_and(|w| w == "run");
    let package = rest
        .windows(2)
        .any(|pair| pair[0] == "-p" && pair[1] == "conformance");
    let binary = rest
        .windows(2)
        .find(|pair| pair[0] == "--bin")
        .map(|pair| pair[1].as_str());
    (is_run && package).then_some(binary).flatten()
}

/// Every command `tools/state.sh` runs is a read: no redirection into the tree, no writing command,
/// no `git` that takes a lock, no `tools/*.py` that can leave bytecode, and no `conformance` binary
/// that writes without being asked to.
#[test]
fn every_state_section_reads_and_never_writes() {
    let root = repository_root();
    let script = std::fs::read_to_string(root.join("tools/state.sh"))
        .expect("tools/state.sh is this check's population");
    let (found, commands) = findings(&script, root);
    let ledger = code_of(&script).contains("--bin ledger");
    println!(
        "read-only: {commands} command(s) in tools/state.sh, {} finding(s)",
        found.len()
    );
    assert!(
        commands > 100 && ledger,
        "{commands} command(s), and the ledger section {} found: the lexer is measuring nothing",
        if ledger { "was" } else { "was not" }
    );
    assert!(
        found.is_empty(),
        "a state section writes, and a command that counts may not (ADR 1487):\n  {}",
        found.join("\n  ")
    );
}

/// The calibration: each rule finds the shape it exists for, and passes the shapes the script
/// legitimately uses beside it.
#[test]
fn the_check_finds_each_planted_write_and_passes_each_read() {
    let root = repository_root();
    let planted = "\
section_x() {
    # a comment that says rm -rf / > out.txt is not code
    heading \"cargo run -q -p conformance --bin ledger -- --write\" \"ls fuzz/<t>\"
    run \"ledger\" '.' cargo run -q -p conformance --bin ledger
    run \"ledger\" '.' cargo run -q -p conformance --bin ledger -- --write
    cargo build -p x >/dev/null 2>&1 || status=1
    printf '%s\\n' \"$x\" > doc/out.txt
    sorted=$(mktemp)
    printf '%s' \"$y\" | sed \"s/^/a:/\" >> \"$sorted\"
    rm -f \"$sorted\"
    rm -f doc/keep.md
    python3 tools/comment-history.py
    PYTHONDONTWRITEBYTECODE=1 python3 tools/main-checkout.py
    git -C \"$main\" status --porcelain
    git --no-optional-locks status --porcelain
    x=\"$(git rev-parse --show-toplevel 2>/dev/null)\"
    sed -i 's/a/b/' doc/x.md
    cat <<'TEXT'
git commit -m nothing > nowhere
TEXT
}
";
    let (found, _) = findings(planted, root);
    let expected = [
        "a redirection into `doc/out.txt`",
        "`rm -f doc/keep.md` writes",
        "`python3 tools/comment-history.py` may write __pycache__ (trap 69)",
        "`git -C $main status --porcelain` takes index.lock",
        "`sed -i Q doc/x.md` edits in place",
        "`run Q Q cargo run -q -p conformance --bin ledger -- --write` runs a binary that writes, \
         without its writing behind `--write`",
    ];
    for sentence in expected {
        assert!(
            found.iter().any(|f| f == sentence),
            "the check did not find {sentence:?}; it found:\n  {}",
            found.join("\n  ")
        );
    }
    assert_eq!(
        found.len(),
        expected.len(),
        "the check found something it should have passed:\n  {}",
        found.join("\n  ")
    );
}
