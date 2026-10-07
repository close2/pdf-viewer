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
//! appear in the section, because each is an instruction the owner acts on.
//!
//! # The owner's list
//!
//! After the kinds the script prints **one numbered list** of what the owner does, in the order a
//! person does it (ADR 1601). Its shape is held against a planted main checkout that owes one of
//! each kind the list can carry: the list is the last thing printed, numbered from one without a
//! gap, in the order the script's ranks state; each item carries a command in backticks or the
//! files it acts on; and nothing is said twice — no file an item names is named by another item or
//! by a line above the list.

#![expect(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "test code: a gate that cannot read its own repository has not found a defect, and \
              reporting that as one would be worse than stopping"
)]

use std::path::{Path, PathBuf};
use std::process::Command;

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

/// The owner's answers are the first kind the script prints, and the merge's check repeats them.
///
/// An answer lands in the main checkout as an untracked or modified `A` file, and every list read
/// from tracked files calls its question open until the owner commits it; the line that says so
/// is the one a merge must not miss, so it is printed before anything else, and `tools/batch.sh
/// check`, which every merge runs, prints it again by asking the script for it (ADR 1588).
#[test]
fn the_owners_answers_come_first_and_the_merge_check_repeats_them() {
    let root = repository_root();
    let script = std::fs::read_to_string(root.join("tools/main-checkout.py"))
        .expect("tools/main-checkout.py is read");
    let batch =
        std::fs::read_to_string(root.join("tools/batch.sh")).expect("tools/batch.sh is read");
    let (kinds, _) = script_kinds(&script);
    assert_eq!(
        kinds.iter().take(2).map(String::as_str).collect::<Vec<_>>(),
        ["answered", "open questions"],
        "the owner's uncommitted answers and the questions they leave open are the first two lines"
    );
    assert!(
        script.contains("\"--answers\""),
        "the script answers `--answers` with those two lines alone"
    );
    let check = batch
        .split("\ncheck_batch() {")
        .nth(1)
        .and_then(|body| body.split("\n}\n").next())
        .expect("tools/batch.sh defines check_batch");
    assert!(
        check.contains("main-checkout.py --answers"),
        "tools/batch.sh check repeats the answered line"
    );
}

/// A throwaway main checkout that owes one of each thing the owner's list carries but the
/// fast-forward's and the artefacts': a `§` in an uncommitted question, an answer no commit holds,
/// a question still open, an upstream report not filed, a patch owed to a pinned fork, an unseeded
/// fuzz target, a build directory over a rule of zero, an `sccache` cache over a ceiling of one
/// kibibyte, and an agent whose processes sit in two scopes, one with no limit on its path and one
/// under a slice that bounds it — beside a stranger's process, which is not the agent's.
fn planted_main_checkout(base: &Path) -> Vec<(PathBuf, String)> {
    let base_rev = "0123456789abcdef0123456789abcdef01234567";
    let process = |uid: u32, threads: u32| {
        format!("Name:\tplanted\nUid:\t{uid}\t{uid}\t{uid}\t{uid}\nThreads:\t{threads}\n")
    };
    vec![
        (base.join("host/proc/10/status"), process(4242, 3)),
        (
            base.join("host/proc/10/cgroup"),
            "0::/agent.slice/tab(1).scope\n".to_owned(),
        ),
        (base.join("host/proc/11/status"), process(4242, 5)),
        (
            base.join("host/proc/11/cgroup"),
            "0::/bounded.slice/tab(2).scope\n".to_owned(),
        ),
        (base.join("host/proc/12/status"), process(999, 7)),
        (
            base.join("host/proc/12/cgroup"),
            "0::/other.slice/tab(3).scope\n".to_owned(),
        ),
        (
            base.join("host/cgroup/agent.slice/tab(1).scope/pids.max"),
            "max\n".to_owned(),
        ),
        (
            base.join("host/cgroup/agent.slice/tab(1).scope/memory.max"),
            "max\n".to_owned(),
        ),
        (
            base.join("host/cgroup/bounded.slice/pids.max"),
            "8192\n".to_owned(),
        ),
        (
            base.join("host/cgroup/bounded.slice/memory.max"),
            "1073741824\n".to_owned(),
        ),
        (
            base.join("host/cgroup/bounded.slice/tab(2).scope/pids.max"),
            "max\n".to_owned(),
        ),
        (
            base.join("host/cgroup/bounded.slice/tab(2).scope/memory.max"),
            "max\n".to_owned(),
        ),
        (
            base.join("Cargo.toml"),
            format!(
                "[workspace]\nmembers = []\n\n[workspace.dependencies]\npkg = {{ git = \
                 \"https://example.invalid/fork\", rev = \"{base_rev}\" }}\n"
            ),
        ),
        (
            base.join(".cargo/config.toml"),
            format!(
                "[build]\ntarget-dir = \"{}\"\n",
                base.join("built").display()
            ),
        ),
        (base.join("built/debug/artefact"), "built\n".to_owned()),
        (base.join("sccache/entry"), "cached\n".to_owned()),
        (
            base.join("fuzz/Cargo.toml"),
            "[package]\nname = \"fuzz\"\n\n[[bin]]\nname = \"unseeded_target\"\n".to_owned(),
        ),
        (
            base.join("doc/patches/pkg-fix.patch"),
            format!(
                "Repository: https://example.invalid/fork\nBase: {base_rev}\n\n\
                 --- a/pkg/src/lib.rs\n+++ b/pkg/src/lib.rs\n"
            ),
        ),
        (
            base.join("doc/patches/pkg-fix.md"),
            "# pkg: a fix\n\nFor the owner to file at <https://example.invalid/issues>.\n"
                .to_owned(),
        ),
        (
            base.join("doc/questions/Q900-answered.md"),
            "# Q900\n".to_owned(),
        ),
        (
            base.join("doc/questions/Q901-still-open.md"),
            "# Q901\n".to_owned(),
        ),
    ]
}

/// The planted files that land after the commit, so the checkout holds them uncommitted: an
/// answer, and an edit to its question that puts a `§` after another standard's name.
fn planted_uncommitted(base: &Path) -> Vec<(PathBuf, String)> {
    vec![
        (
            base.join("doc/questions/A900-answered.md"),
            "# A900\n\nYes.\n".to_owned(),
        ),
        (
            base.join("doc/questions/Q900-answered.md"),
            "# Q900\n\nUnder ISO 19005-2 \u{a7}6.7's rule the page is appended.\n".to_owned(),
        ),
    ]
}

fn git_in(directory: &Path, arguments: &[&str]) {
    let status = Command::new("git")
        .current_dir(directory)
        .args(arguments)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_AUTHOR_NAME", "test")
        .env("GIT_AUTHOR_EMAIL", "test@invalid")
        .env("GIT_COMMITTER_NAME", "test")
        .env("GIT_COMMITTER_EMAIL", "test@invalid")
        .status()
        .expect("git runs");
    assert!(status.success(), "git {arguments:?} failed");
}

/// The planted agent's two scopes are both counted and the stranger's is not, and the list's item
/// writes the limits of the one scope with no limit on its path and only that one: a slice above the
/// other bounds it (ADR 1612).
fn the_agent_scope_is_read_from_its_processes(report: &str, texts: &[&str]) {
    assert!(
        report.contains("agent's cgroup: 2 scope(s) hold user 4242's processes, 1 with a limit"),
        "the stranger's scope was counted, or the bounded one missed: {report}"
    );
    let bound = texts
        .iter()
        .find(|text| text.starts_with("bound "))
        .copied()
        .unwrap_or_default();
    assert!(
        bound.contains("agent.slice/tab(1).scope/pids.max") && !bound.contains("tab(2)"),
        "the item writes the unbounded scope's limits and only those: {bound}"
    );
}

/// The owner's list is one numbered list, last, in the script's order, each item with its command or
/// its files, and no file in it said twice (ADR 1601).
#[test]
fn the_owners_list_is_one_numbered_list_in_order_with_nothing_twice() {
    let base = std::env::temp_dir().join(format!("owner-list-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    for (path, contents) in planted_main_checkout(&base) {
        std::fs::create_dir_all(path.parent().expect("a planted file has a directory"))
            .expect("a planted directory");
        std::fs::write(&path, contents).expect("a planted file");
    }
    git_in(&base, &["init", "-q", "-b", "main"]);
    git_in(&base, &["add", "-A"]);
    git_in(&base, &["commit", "-q", "-m", "base"]);
    for (path, contents) in planted_uncommitted(&base) {
        std::fs::write(&path, contents).expect("a planted file");
    }
    let output = Command::new("python3")
        .arg(repository_root().join("tools/main-checkout.py"))
        .current_dir(repository_root())
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .env("MAIN_CHECKOUT", &base)
        .env("SECTION_SIGNS_BIN", env!("CARGO_BIN_EXE_section_signs"))
        .env("MAIN_CHECKOUT_BUILD_RULE_KIB", "0")
        .env("SCCACHE_DIR", base.join("sccache"))
        .env("SCCACHE_CACHE_SIZE", "1K")
        .env("MAIN_CHECKOUT_AGENT_USER", "4242")
        .env("MAIN_CHECKOUT_PROC", base.join("host/proc"))
        .env("MAIN_CHECKOUT_CGROUP_ROOT", base.join("host/cgroup"))
        .output()
        .expect("python3 runs tools/main-checkout.py");
    let _ = std::fs::remove_dir_all(&base);
    let report = String::from_utf8_lossy(&output.stdout).into_owned();
    println!("{report}");
    assert!(output.status.success(), "the script failed: {report}");

    let lines: Vec<&str> = report.lines().collect();
    let heads: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.starts_with("the owner's list, in the order to do them: "))
        .map(|(at, _)| at)
        .collect();
    assert_eq!(
        heads.len(),
        1,
        "one owner's list, not {}: {report}",
        heads.len()
    );
    let (above, items) = lines.split_at(heads[0].saturating_add(1));
    let mut texts = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let prefix = format!("  {}. ", index.saturating_add(1));
        let text = item.strip_prefix(prefix.as_str());
        assert!(
            text.is_some(),
            "an item is not numbered `{prefix}`: {item:?}"
        );
        texts.push(text.unwrap_or_default());
    }
    let count = format!("{}", texts.len());
    assert!(
        above.last().is_some_and(|head| head.ends_with(&count)),
        "the list's head does not count its {count} items: {report}"
    );
    let openings: Vec<&str> = texts
        .iter()
        .map(|text| text.split_whitespace().next().unwrap_or_default())
        .collect();
    assert_eq!(
        openings,
        [
            "write", "commit", "answer", "file", "apply", "bound", "re-seed", "prune", "decide"
        ],
        "the planted checkout's items, in the order a person does them: {report}"
    );
    the_agent_scope_is_read_from_its_processes(&report, &texts);
    let paths = |text: &str| -> Vec<String> {
        text.split(|c: char| c.is_whitespace() || "`();,".contains(c))
            .map(|word| word.split(':').next().unwrap_or_default())
            .filter(|word| {
                Path::new(word).extension().is_some()
                    && (word.starts_with("doc/") || word.starts_with("fuzz/artifacts/"))
            })
            .map(str::to_owned)
            .collect()
    };
    for (index, text) in texts.iter().enumerate() {
        assert!(
            text.contains('`') || !paths(text).is_empty() || text.contains(" at http"),
            "item {} carries neither a command nor a file: {text}",
            index + 1
        );
        for path in paths(text) {
            let elsewhere = texts
                .iter()
                .enumerate()
                .filter(|(other, _)| *other != index)
                .any(|(_, other)| paths(other).contains(&path))
                || above.iter().any(|line| line.contains(&path));
            assert!(!elsewhere, "{path} is said twice: {report}");
        }
    }
}
