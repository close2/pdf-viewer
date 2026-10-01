//! `doc/environment.md`'s *After a merge* section and `tools/main-checkout.py` are one list.
//!
//! Not a conformance question; it lives here for `state_sections.rs`'s reason — this is the crate
//! whose gates read the repository's own files rather than a PDF.
//!
//! # What it guards
//!
//! The script prints one line per kind of thing a merge leaves on the owner's disk; the section
//! says, per kind, what the line means and the command that clears it, "in the order it prints
//! them" (ADR 1440). The script's own docstring says a line added there owes its entry here. That
//! is two copies of one list kept by hand, and the one that drifts is the one the owner reads: a
//! kind the script prints with no entry is a count with no command beside it, and an entry the
//! script no longer prints is a command for a thing nothing measures.
//!
//! # The two populations, both derived
//!
//! - **The kinds**, from the script: every string a function other than `main` returns, prints or
//!   adds to a list of lines, whose text up to its first `{` holds a colon. Its *key* is the words before the
//!   first `:` or `,` — so `section signs: no uncommitted document …` and `section signs,
//!   uncommitted: …` are one kind, as they are one entry. An indented string is a sub-line of the
//!   kind above it. `main` itself prints the heading and the refusal to run, which are not kinds.
//!   The order is the order `main` calls the functions in.
//! - **The entries**, from the section's fenced block: every comment line opening with a
//!   backtick-quoted line, keyed the same way, in the order written.
//!
//! The kinds and the entries must be the same list in the same order, and every sub-line's key must
//! appear in the section, because each is an instruction the owner acts on (`read, removable:` is
//! what the `xargs rm` in the artefacts entry reads).

#![expect(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "test code: a gate that cannot read its own repository has not found a defect, and \
              reporting that as one would be worse than stopping"
)]

use std::path::Path;

/// Where the repository root is, relative to this crate's manifest.
fn repository_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the manifest directory of a workspace member has two ancestors")
}

/// The words before the first `:` or `,` — the part of a printed line that names its kind.
fn key(text: &str) -> Option<String> {
    let end = text.find([':', ','])?;
    let key = text[..end].trim();
    (!key.is_empty()).then(|| key.to_owned())
}

/// What one function of the script prints: its kinds' keys and its sub-lines' keys, in order.
#[derive(Debug, Default)]
struct Printed {
    kinds: Vec<String>,
    sub_lines: Vec<String>,
}

/// The string literals in `body` that open a printed line: after `return `, `return [`,
/// `lines = [`, `lines += [`, `append(` or `print(`, optionally an f-string.
fn printed(body: &str) -> Printed {
    let mut found = Printed::default();
    for opener in [
        "return ",
        "return [",
        "lines = [",
        "lines += [",
        "append(",
        "print(",
    ] {
        let mut rest = body;
        while let Some(at) = rest.find(opener) {
            rest = &rest[at.saturating_add(opener.len())..];
            let literal = rest.strip_prefix('f').unwrap_or(rest);
            let Some(literal) = literal.strip_prefix('"') else {
                continue;
            };
            let text: String = literal.chars().take_while(|c| *c != '"').collect();
            let text = text.split('{').next().unwrap_or_default();
            if !text.contains(':') {
                continue;
            }
            let Some(key) = key(text) else { continue };
            let list = if text.starts_with("  ") {
                &mut found.sub_lines
            } else {
                &mut found.kinds
            };
            if !list.contains(&key) {
                list.push(key);
            }
        }
    }
    found
}

/// The script's top-level functions, by name, with their bodies.
fn functions(script: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for line in script.lines() {
        if let Some(rest) = line.strip_prefix("def ") {
            let name = rest.split('(').next().unwrap_or_default().to_owned();
            out.push((name, String::new()));
        } else if !line.is_empty() && !line.starts_with([' ', '\t', ')', ']']) {
            // A top-level statement ends the function above it.
            out.push((String::new(), String::new()));
        } else if let Some((_, body)) = out.last_mut() {
            body.push_str(line);
            body.push('\n');
        }
    }
    out.retain(|(name, _)| !name.is_empty());
    out
}

/// The kinds the script prints, in the order `main` prints them, and every sub-line's key.
fn script_kinds(script: &str) -> (Vec<String>, Vec<String>) {
    let functions = functions(script);
    let main = functions
        .iter()
        .find(|(name, _)| name == "main")
        .map(|(_, body)| body.as_str())
        .unwrap_or_default();
    let mut called: Vec<(usize, &String, &String)> = functions
        .iter()
        .filter(|(name, _)| name != "main")
        .filter_map(|(name, body)| main.find(&format!("{name}(")).map(|at| (at, name, body)))
        .collect();
    called.sort();
    let mut kinds = Vec::new();
    let mut sub_lines = Vec::new();
    for (_, _, body) in called {
        let found = printed(body);
        for kind in found.kinds {
            if !kinds.contains(&kind) {
                kinds.push(kind);
            }
        }
        sub_lines.extend(found.sub_lines);
    }
    (kinds, sub_lines)
}

/// The *After a merge* section: its entries' keys in order, and its whole text.
fn section_entries(environment: &str) -> (Vec<String>, String) {
    let start = environment
        .find("## After a merge")
        .expect("doc/environment.md has its After a merge section");
    let section = &environment[start..];
    let section = section
        .get(3..)
        .and_then(|rest| {
            rest.find("\n## ")
                .map(|end| &section[..end.saturating_add(3)])
        })
        .unwrap_or(section);
    let fence = section
        .split("```")
        .nth(1)
        .expect("the After a merge section holds its commands in one fenced block");
    let entries = fence
        .lines()
        .filter_map(|line| line.strip_prefix("# `"))
        .filter_map(|rest| key(rest.split('`').next().unwrap_or_default()))
        .collect();
    (entries, section.to_owned())
}

/// The kinds `tools/main-checkout.py` prints and the entries `doc/environment.md`'s *After a merge*
/// section holds are one list in one order, and every sub-line the script prints is named there.
#[test]
fn the_owners_section_and_its_printer_are_one_list() {
    let root = repository_root();
    let script = std::fs::read_to_string(root.join("tools/main-checkout.py"))
        .expect("tools/main-checkout.py is this check's first population");
    let environment = std::fs::read_to_string(root.join("doc/environment.md"))
        .expect("doc/environment.md is this check's second population");
    let (kinds, sub_lines) = script_kinds(&script);
    let (entries, section) = section_entries(&environment);
    println!(
        "owner's section: {} kind(s) printed, {} entr(ies) written, {} sub-line(s)",
        kinds.len(),
        entries.len(),
        sub_lines.len()
    );
    assert!(
        kinds.len() > 3 && entries.len() > 3,
        "{} kind(s) and {} entr(ies): the parse is measuring nothing",
        kinds.len(),
        entries.len()
    );
    assert_eq!(
        kinds, entries,
        "tools/main-checkout.py prints these kinds, in this order (left), and doc/environment.md's \
         After a merge section has entries for these (right): a kind with no entry is a count \
         with no command beside it, an entry with no kind is a command for what nothing measures"
    );
    let unnamed: Vec<&String> = sub_lines
        .iter()
        .filter(|sub| {
            !section.contains(&format!("`{sub}")) && !section.contains(&format!("{sub}:"))
        })
        .collect();
    assert!(
        unnamed.is_empty(),
        "tools/main-checkout.py prints these sub-lines and the After a merge section never names \
         them: {unnamed:?}"
    );
}

/// The calibration: a planted script and section that disagree in membership and in order are
/// told apart, and the shapes the real script uses — an f-string, a list of lines, a sub-line,
/// a second spelling of one kind — are read as the parse claims.
#[test]
fn the_parse_reads_each_shape_the_script_uses() {
    let script = "\
def b(main):
    return f\"fuzz/corpus: {n} of {m}\"

def a(main):
    lines = [f\"doc/patches: {n} owed\"]
    lines += [f\"  owed:             doc/patches/{name}\" for name in owed]
    return lines

def c(main):
    if not files:
        return [\"section signs: no uncommitted document\"]
    return [f\"section signs, uncommitted: {x}\"]

def main():
    print(f\"main checkout: {main_dir}\")
    for line in a(main_dir):
        print(line)
    print(b(main_dir))
    print(c(main_dir))
    return 0


if __name__ == \"__main__\":
    sys.exit(main())
";
    let (kinds, sub_lines) = script_kinds(script);
    assert_eq!(kinds, ["doc/patches", "fuzz/corpus", "section signs"]);
    assert_eq!(sub_lines, ["owed"]);
    let environment = "\
## After a merge: the commands

```sh
# `fuzz/corpus: N of M` — seed them:
fuzz/seeds.sh
# `doc/patches: N owed` — an `owed:` line is applied:
git apply
```

## Next
# `not: an entry`
";
    let (entries, section) = section_entries(environment);
    assert_eq!(entries, ["fuzz/corpus", "doc/patches"]);
    assert_ne!(kinds, entries, "an order that differs is a finding");
    assert!(section.contains("`owed:"));
    assert!(!section.contains("not: an entry"));
}
