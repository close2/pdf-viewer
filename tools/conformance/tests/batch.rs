//! `tools/batch.sh commit` stages the whole population, `tools/batch.sh close` refuses a worktree
//! holding work nobody committed, and `tools/batch.sh install` puts what a person runs into the
//! main checkout's `target/` and nowhere else.
//!
//! Not a conformance question, and it lives here for `bounded.rs`'s reason: this is the crate
//! whose gates read the repository's own tools, and a guard broken by an edit should fail the
//! round that broke it rather than the merge that trusts it.
//!
//! # What it guards
//!
//! The merge of batch thirty-five chained `git add`, `git commit`, `git merge --ff-only` and
//! `tools/batch.sh close` on one line. `git add` refused a pathspec — a file already staged as
//! deleted, which matches nothing on disk — the `;` let the commit run with that one deletion, the
//! fast-forward took it, and `close`, which removes the worktree with `--force`, deleted 128
//! uncommitted files. ADR 1313 records the incident and the two guards; these tests hold both
//! against a throwaway repository, never against the batch's own worktree.
//!
//! - `close` refuses a worktree with anything uncommitted outside `scratchpad/`, names what is
//!   there, and removes nothing; a spelling of `--force` is refused rather than read as a branch.
//! - `commit` stages by name the same population `close` refuses on, counts it against the index,
//!   and commits the incident's own shape — a staged deletion beside new and modified files —
//!   whole, leaving `scratchpad/` out.
//! - `check` reads that same population for its findings about added files, so a path git would
//!   print quoted is placed by its real directory: under `scratchpad/` it is no finding, and
//!   anywhere else it is named as itself.
//! - Every `cargo test … --test` line of `gates()` and of `doc/todo/02` names a file whose
//!   `#[ignore]` attributes match the flag the line gives it, so no gate is green having run zero
//!   tests (ADR 1392).
//! - `install` refuses a worktree with uncommitted work and a batch `main` has not been
//!   fast-forwarded to, then builds the programs and libraries a person runs, installs them into
//!   the main checkout's `target/` — the one path outside the worktree this script writes — and
//!   writes the commit they were built from beside them (ADR 1511). Its names are every program of
//!   a package under `crates/`, `quorra-retrieve`, and every C library, held against the
//!   workspace's own manifests.

#![expect(
    clippy::expect_used,
    reason = "test code: a gate that cannot build a throwaway repository has not found a defect, \
              and reporting that as one would be worse than stopping"
)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn repository_root() -> &'static Path {
    // `CARGO_MANIFEST_DIR` is `<root>/tools/conformance`, so two levels up is the root. This
    // cannot fail for a crate that is in the workspace, which is the only way this test runs.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the manifest directory of a workspace member has two ancestors")
}

/// A repository with `main`, one tracked file, and a copy of `tools/batch.sh` — so that the
/// script's own root, which it derives from where it lives, is this repository and not the tree.
struct Sandbox {
    base: PathBuf,
}

impl Sandbox {
    fn new(name: &str) -> Self {
        Self::with_files(name, &[])
    }

    /// As [`Sandbox::new`], with `files` committed on `main` before the batch opens.
    fn with_files(name: &str, files: &[(&str, &str)]) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_nanos());
        let base =
            std::env::temp_dir().join(format!("batch-{name}-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(base.join("repo/tools")).expect("a temporary directory");
        let sandbox = Self { base };
        std::fs::copy(
            repository_root().join("tools/batch.sh"),
            sandbox.repo().join("tools/batch.sh"),
        )
        .expect("tools/batch.sh copies");
        std::fs::write(sandbox.repo().join("a.txt"), "a\n").expect("a tracked file");
        for (relative, contents) in files {
            let path = sandbox.repo().join(relative);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).expect("a directory in the repository");
            }
            std::fs::write(path, contents).expect("a file in the repository");
        }
        sandbox.git(&sandbox.repo(), &["init", "-q", "-b", "main"]);
        sandbox.git(&sandbox.repo(), &["add", "-A"]);
        sandbox.git(&sandbox.repo(), &["commit", "-q", "-m", "base"]);
        let opened = sandbox.batch(&["open", "batch-test"]);
        assert!(opened.status.success(), "open failed: {}", text(&opened));
        sandbox
    }

    fn repo(&self) -> PathBuf {
        self.base.join("repo")
    }

    fn worktree(&self) -> PathBuf {
        self.base.join("wt")
    }

    fn command(&self, program: &str, directory: &Path) -> Command {
        let mut command = Command::new(program);
        command
            .current_dir(directory)
            .env("BATCH_WORKTREE", self.worktree())
            // `open` warms a workspace it finds; a throwaway one is built by the test that needs it.
            .env("BATCH_WARM", "0")
            // The throwaway workspace names its own build directory; an inherited one would put a
            // stand-in called `quorra` beside the real one.
            .env_remove("CARGO_TARGET_DIR")
            .env_remove("CARGO_BUILD_TARGET_DIR")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_AUTHOR_NAME", "test")
            .env("GIT_AUTHOR_EMAIL", "test@invalid")
            .env("GIT_COMMITTER_NAME", "test")
            .env("GIT_COMMITTER_EMAIL", "test@invalid");
        command
    }

    fn git(&self, directory: &Path, arguments: &[&str]) -> Output {
        let output = self
            .command("git", directory)
            .args(arguments)
            .output()
            .expect("git runs");
        assert!(
            output.status.success(),
            "git {arguments:?} failed: {}",
            text(&output)
        );
        output
    }

    /// The script, run from the repository — outside the worktree, as `close` requires.
    fn batch(&self, arguments: &[&str]) -> Output {
        self.command("bash", &self.repo())
            .arg(self.repo().join("tools/batch.sh"))
            .args(arguments)
            .output()
            .expect("bash runs tools/batch.sh")
    }

    fn write(&self, relative: &str, contents: &str) {
        let path = self.worktree().join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("a directory in the worktree");
        }
        std::fs::write(path, contents).expect("a file in the worktree");
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.base);
    }
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn close_refuses_a_worktree_with_uncommitted_work_and_removes_nothing() {
    let sandbox = Sandbox::new("close");
    sandbox.write("new.txt", "new\n");
    sandbox.write("a.txt", "changed\n");

    let refused = sandbox.batch(&["close", "batch-test"]);
    let said = text(&refused);
    assert!(
        !refused.status.success(),
        "close removed a dirty worktree: {said}"
    );
    assert!(
        said.contains("2 uncommitted path(s)"),
        "the refusal does not count: {said}"
    );
    assert!(
        said.contains("new.txt") && said.contains("a.txt"),
        "the refusal does not name what is there: {said}"
    );
    assert!(
        sandbox.worktree().join("new.txt").exists(),
        "the refusal deleted the work"
    );

    let forced = sandbox.batch(&["close", "--force"]);
    assert!(
        !forced.status.success(),
        "close accepted --force: {}",
        text(&forced)
    );
    assert!(
        sandbox.worktree().join("new.txt").exists(),
        "--force deleted the work"
    );

    let misnamed = sandbox.batch(&["close", "batch-other"]);
    assert!(
        !misnamed.status.success(),
        "close accepted a branch the worktree is not on"
    );
}

#[test]
fn commit_stages_the_whole_population_and_close_then_needs_the_fast_forward() {
    let sandbox = Sandbox::new("commit");
    // The incident's own shape: a deletion already staged, beside new and changed files.
    sandbox.git(&sandbox.worktree(), &["rm", "-q", "a.txt"]);
    sandbox.write("b.txt", "b\n");
    sandbox.write("c/d.txt", "d\n");
    sandbox.write("scratchpad/r1/notes.txt", "never committed\n");
    sandbox.write("scratchpad/message", "the batch\n");

    let message = sandbox.worktree().join("scratchpad/message");
    let message = message.to_str().expect("a UTF-8 temporary path");
    let committed = sandbox.batch(&["commit", message]);
    let said = text(&committed);
    assert!(committed.status.success(), "commit failed: {said}");
    assert!(
        said.contains("population 3 path(s), staged 3 path(s)"),
        "the count is not printed: {said}"
    );
    assert!(
        said.contains("now 0 (must be 0)"),
        "work was left behind: {said}"
    );
    assert!(
        !said.contains("merge --ff-only batch-test;"),
        "commit chained the fast-forward: {said}"
    );

    let names = sandbox.git(
        &sandbox.worktree(),
        &["show", "--no-renames", "--name-status", "--format=", "HEAD"],
    );
    let names = String::from_utf8_lossy(&names.stdout);
    for line in ["D\ta.txt", "A\tb.txt", "A\tc/d.txt"] {
        assert!(
            names.lines().any(|name| name == line),
            "{line} is not in the commit:\n{names}"
        );
    }
    assert!(
        !names.contains("scratchpad/"),
        "scratchpad/ was committed:\n{names}"
    );

    let early = sandbox.batch(&["close", "batch-test"]);
    assert!(
        !early.status.success(),
        "close ran before the fast-forward: {}",
        text(&early)
    );
    assert!(
        sandbox.worktree().exists(),
        "the early close removed the tree"
    );

    sandbox.git(&sandbox.repo(), &["merge", "-q", "--ff-only", "batch-test"]);
    let closed = sandbox.batch(&["close", "batch-test"]);
    assert!(
        closed.status.success(),
        "close refused a finished batch: {}",
        text(&closed)
    );
    assert!(!sandbox.worktree().exists(), "close left the worktree");
}

#[test]
fn commit_refuses_an_empty_population_and_a_message_it_would_commit() {
    let sandbox = Sandbox::new("empty");
    sandbox.write("scratchpad/message", "the batch\n");
    let message = sandbox.worktree().join("scratchpad/message");
    let nothing = sandbox.batch(&["commit", message.to_str().expect("a UTF-8 temporary path")]);
    assert!(
        !nothing.status.success(),
        "commit made an empty commit: {}",
        text(&nothing)
    );

    sandbox.write("message.txt", "the batch\n");
    let inside = sandbox.worktree().join("message.txt");
    let inside = sandbox.batch(&["commit", inside.to_str().expect("a UTF-8 temporary path")]);
    assert!(
        !inside.status.success(),
        "commit committed its own message file: {}",
        text(&inside)
    );
}

/// `check`'s added-file findings read `population`, the listing `commit` stages from, so a path
/// `git status --porcelain` would print quoted and escaped — a space and a non-ASCII character —
/// is excluded by the directory it is really in, and one outside `scratchpad/` is named as itself.
#[test]
fn check_reads_the_population_commit_stages_so_a_quoted_path_is_placed_by_its_real_directory() {
    let sandbox = Sandbox::new("quoted");
    sandbox.write("scratchpad/r1/w/\u{5bf9} page one.pdf", "scratch\n");
    sandbox.write("copies/\u{e9}t\u{e9} copy.bin", "a stray binary\n");
    let checked = sandbox.batch(&["check"]);
    let report = text(&checked);
    let line = report
        .lines()
        .find(|line| line.starts_with("untracked, unexpected extension"))
        .expect("check prints the unexpected-extension line");
    assert!(
        line.ends_with("1 file(s)"),
        "the scratchpad path was counted, or the stray one missed: {report}"
    );
    assert!(
        report.contains("    copies/\u{e9}t\u{e9} copy.bin"),
        "the stray file is not named by its real path: {report}"
    );
    assert!(
        !report.contains("scratchpad/"),
        "a path under scratchpad/ was reported: {report}"
    );
}

/// Where a package's integration test called `name` is, if the package is in one of the tree's
/// three roots.
fn test_file(package: &str, name: &str) -> Option<PathBuf> {
    ["crates", "raster/crates", "tools"]
        .iter()
        .flat_map(|root| {
            let tests = repository_root().join(root).join(package).join("tests");
            [
                tests.join(format!("{name}.rs")),
                tests.join(name).join("main.rs"),
            ]
        })
        .find(|path| path.is_file())
}

/// How many `#[test]` functions a file holds, ignored and not.
///
/// A test's attributes are the lines between the item before it — the last line that is a lone
/// `}` — and its `fn`; one of them starting `#[ignore` makes it ignored. A doc comment that
/// *mentions* `#[ignore]` starts with `///` and is not an attribute, and `#[cfg_attr(miri, ignore
/// …)]` ignores a test under Miri alone, so neither counts.
fn test_shape(text: &str) -> (usize, usize) {
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let (mut ignored, mut run) = (0usize, 0usize);
    let mut attributes_from = 0usize;
    for (at, line) in lines.iter().enumerate() {
        if *line == "}" {
            attributes_from = at.saturating_add(1);
            continue;
        }
        let is_fn =
            line.starts_with("fn ") || line.starts_with("pub fn ") || line.starts_with("async fn ");
        if !is_fn {
            continue;
        }
        let attributes = lines.get(attributes_from..at).unwrap_or_default();
        if attributes.iter().any(|line| line.starts_with("#[test]")) {
            if attributes.iter().any(|line| line.starts_with("#[ignore")) {
                ignored = ignored.saturating_add(1);
            } else {
                run = run.saturating_add(1);
            }
        }
        attributes_from = at.saturating_add(1);
    }
    (ignored, run)
}

/// Every `cargo test … -p <package> --test <name> …` a gate list writes, with the loops of
/// `tools/batch.sh`'s `gates()` expanded, as (where, package, name, whether `--ignored` is given).
fn gate_commands() -> Vec<(String, String, String, bool)> {
    let mut commands = Vec::new();
    let mut read = |source: &str, text: &str| {
        let mut looped: Vec<String> = Vec::new();
        for (number, line) in text.lines().enumerate() {
            if let Some(rest) = line.trim().strip_prefix("for t in ")
                && let Some((names, _)) = rest.split_once(';')
            {
                looped = names.split_whitespace().map(str::to_owned).collect();
                continue;
            }
            let mut from = 0;
            while let Some(found) = line.get(from..).and_then(|rest| rest.find("cargo test")) {
                let start = from.saturating_add(found);
                let command: String = line
                    .get(start..)
                    .unwrap_or_default()
                    .chars()
                    .take_while(|character| !matches!(character, '`' | '#' | '|'))
                    .collect();
                from = start.saturating_add(command.len().max(1));
                let words: Vec<&str> = command.split_whitespace().collect();
                let after = |flag: &str| {
                    words
                        .iter()
                        .position(|word| *word == flag)
                        .and_then(|at| words.get(at.saturating_add(1)))
                        .map(|word| word.trim_matches('"').to_owned())
                };
                let (Some(package), Some(name)) = (after("-p"), after("--test")) else {
                    continue;
                };
                let ignored = words.contains(&"--ignored");
                let names = if name.starts_with('$') {
                    looped.clone()
                } else {
                    vec![name]
                };
                for name in names {
                    commands.push((
                        format!("{source}:{}", number.saturating_add(1)),
                        package.clone(),
                        name,
                        ignored,
                    ));
                }
            }
        }
    };
    for source in ["tools/batch.sh", "doc/todo/02-every-round.md"] {
        let text = std::fs::read_to_string(repository_root().join(source))
            .expect("the gate lists are in the tree");
        read(source, &text);
    }
    commands
}

/// A gate that runs no test is green while checking nothing: `--ignored` against a file with no
/// ignored test, or no `--ignored` against a file whose every test is ignored, exits 0 having run
/// zero. Every gate line in `tools/batch.sh`'s `gates()` and `doc/todo/02` names a test binary
/// whose shape matches the flag it is given (ADR 1392).
#[test]
fn every_gate_line_runs_at_least_one_test_of_the_file_it_names() {
    let commands = gate_commands();
    assert!(
        commands.len() > 20,
        "the gate lists were not read: {commands:?}"
    );
    let mut wrong = Vec::new();
    for (place, package, name, ignored) in &commands {
        let Some(path) = test_file(package, name) else {
            wrong.push(format!(
                "{place}: -p {package} --test {name} names no test file"
            ));
            continue;
        };
        let text = std::fs::read_to_string(&path).expect("a test file is readable");
        let (ignored_tests, run_tests) = test_shape(&text);
        let selected = if *ignored { ignored_tests } else { run_tests };
        if selected == 0 {
            wrong.push(format!(
                "{place}: -p {package} --test {name} {} runs zero tests ({ignored_tests} ignored, \
                 {run_tests} not)",
                if *ignored {
                    "with --ignored"
                } else {
                    "without --ignored"
                }
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// The shape reader's own calibration: an ignored test, a plain one, and the two spellings that
/// are not an ignore.
#[test]
fn a_test_files_shape_counts_the_ignore_attribute_and_nothing_that_mentions_it() {
    let text = "#[test]\n#[ignore = \"corpus\"]\nfn walk() {\n}\n\n/// Not `#[ignore]`d.\n#[test]\n\
                #[cfg_attr(miri, ignore = \"files\")]\nfn unit() {\n    let x = 1;\n}\n";
    assert_eq!(test_shape(text), (1, 1));
}

/// The words of `tools/batch.sh`'s `<variable>="…"` line.
fn batch_list(variable: &str) -> Vec<String> {
    let script = std::fs::read_to_string(repository_root().join("tools/batch.sh"))
        .expect("tools/batch.sh is in the tree");
    let prefix = format!("{variable}=\"");
    script
        .lines()
        .find_map(|line| line.strip_prefix(prefix.as_str()))
        .and_then(|rest| rest.split_once('"'))
        .map(|(words, _)| words.split_whitespace().map(str::to_owned).collect())
        .unwrap_or_default()
}

/// A workspace whose targets carry `install`'s names, built into its own `target/` — never the
/// build directory `~/.cargo/config.toml` names, where a stand-in called `quorra` would sit
/// beside the real one.
fn stand_in_workspace() -> Vec<(String, String)> {
    let libraries = batch_list("install_libraries");
    let mut members = vec!["\"programs\"".to_owned()];
    members.extend(libraries.iter().map(|name| format!("\"{name}\"")));
    let mut files = vec![
        (
            "Cargo.toml".to_owned(),
            format!(
                "[workspace]\nmembers = [{}]\nresolver = \"3\"\n",
                members.join(", ")
            ),
        ),
        (
            ".cargo/config.toml".to_owned(),
            "[build]\ntarget-dir = \"target\"\n".to_owned(),
        ),
        (".gitignore".to_owned(), "/target\nCargo.lock\n".to_owned()),
        (
            "programs/Cargo.toml".to_owned(),
            "[package]\nname = \"programs\"\nversion = \"0.1.0\"\nedition = \"2024\"\n".to_owned(),
        ),
    ];
    for name in batch_list("install_binaries") {
        files.push((
            format!("programs/src/bin/{name}.rs"),
            "fn main() {}\n".to_owned(),
        ));
    }
    for name in &libraries {
        files.push((
            format!("{name}/Cargo.toml"),
            format!(
                "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n\
                 [lib]\ncrate-type = [\"cdylib\"]\n"
            ),
        ));
        files.push((
            format!("{name}/src/lib.rs"),
            "pub fn stand_in() {}\n".to_owned(),
        ));
    }
    files
}

/// `install` refuses until `main` names the batch's commit, then installs every file into the main
/// checkout's `target/`, records the commit beside them, and leaves the main checkout otherwise
/// exactly as it was.
#[test]
fn install_refuses_until_main_names_the_batch_then_writes_only_main_s_target() {
    let owned = stand_in_workspace();
    let files: Vec<(&str, &str)> = owned
        .iter()
        .map(|(path, contents)| (path.as_str(), contents.as_str()))
        .collect();
    let sandbox = Sandbox::with_files("install", &files);

    sandbox.write("b.txt", "b\n");
    let dirty = sandbox.batch(&["install"]);
    assert!(
        !dirty.status.success() && text(&dirty).contains("1 uncommitted path(s)"),
        "install built from uncommitted work: {}",
        text(&dirty)
    );

    sandbox.write("scratchpad/message", "the batch\n");
    let message = sandbox.worktree().join("scratchpad/message");
    let committed = sandbox.batch(&["commit", message.to_str().expect("a UTF-8 temporary path")]);
    assert!(
        committed.status.success(),
        "commit failed: {}",
        text(&committed)
    );
    let early = sandbox.batch(&["install"]);
    assert!(
        !early.status.success() && text(&early).contains("fast-forward main first"),
        "install ran before main named the batch: {}",
        text(&early)
    );
    assert!(
        !sandbox.repo().join("target").exists(),
        "a refusal wrote into the main checkout"
    );

    sandbox.git(&sandbox.repo(), &["merge", "-q", "--ff-only", "batch-test"]);
    let installed = sandbox.batch(&["install"]);
    let said = text(&installed);
    assert!(installed.status.success(), "install failed: {said}");

    let head = sandbox.git(&sandbox.repo(), &["rev-parse", "main"]);
    let head = String::from_utf8_lossy(&head.stdout).trim().to_owned();
    let record = std::fs::read_to_string(sandbox.repo().join("target/installed-from"))
        .expect("install writes target/installed-from");
    assert!(
        record.starts_with(&format!("# commit {head}\n")),
        "the record does not name main's commit {head}:\n{record}"
    );
    let mut expected: Vec<String> = batch_list("install_binaries");
    expected.extend(
        batch_list("install_libraries")
            .iter()
            .map(|name| format!("lib{}.so", name.replace('-', "_"))),
    );
    for name in &expected {
        let path = sandbox.repo().join("target").join(name);
        assert!(path.is_file(), "{name} was not installed: {said}");
        assert!(
            record
                .lines()
                .any(|line| line.ends_with(&format!("  {name}"))),
            "the record carries no checksum for {name}:\n{record}"
        );
        assert!(
            said.contains(&format!("target/{name}")),
            "{name} is not printed: {said}"
        );
    }

    // The main checkout's status, ignored files included: the only new thing is `target/`.
    let status = sandbox.git(
        &sandbox.repo(),
        &[
            "status",
            "--porcelain",
            "--ignored",
            "--untracked-files=normal",
        ],
    );
    let status = String::from_utf8_lossy(&status.stdout);
    assert_eq!(
        status.trim(),
        "!! target/",
        "install wrote into the main checkout outside target/"
    );
}

/// The binary targets of the packages directly under `directory`, and the packages that build a C
/// library, read from each manifest the way Cargo discovers them in this tree: a `[[bin]]` table's
/// `name`, every `src/bin/<name>.rs`, and `src/main.rs` as the package's own name.
fn programs_and_libraries(directory: &str) -> (Vec<String>, Vec<String>) {
    let (mut programs, mut libraries) = (Vec::new(), Vec::new());
    let Ok(entries) = std::fs::read_dir(repository_root().join(directory)) else {
        return (programs, libraries);
    };
    for entry in entries.flatten() {
        let Ok(manifest) = std::fs::read_to_string(entry.path().join("Cargo.toml")) else {
            continue;
        };
        let value = |line: &str| {
            line.split_once('=')
                .map(|(_, value)| value.trim().trim_matches('"').to_owned())
        };
        let package = manifest
            .lines()
            .skip_while(|line| line.trim() != "[package]")
            .find(|line| line.trim_start().starts_with("name"))
            .and_then(value)
            .unwrap_or_default();
        let mut tables = 0usize;
        let mut in_bin = false;
        for line in manifest.lines().map(str::trim) {
            if line.starts_with('[') {
                in_bin = line == "[[bin]]";
                tables = tables.saturating_add(usize::from(in_bin));
            } else if in_bin
                && line.starts_with("name")
                && let Some(name) = value(line)
            {
                programs.push(name);
            }
        }
        if manifest.contains("\"cdylib\"") {
            libraries.push(package.clone());
        }
        if let Ok(bins) = std::fs::read_dir(entry.path().join("src/bin")) {
            for bin in bins.flatten() {
                let path = bin.path();
                if path.extension().is_some_and(|extension| extension == "rs")
                    && let Some(stem) = path.file_stem()
                {
                    programs.push(stem.to_string_lossy().into_owned());
                }
            }
        }
        if tables == 0 && entry.path().join("src/main.rs").is_file() {
            programs.push(package);
        }
    }
    (programs, libraries)
}

/// `install`'s names against the workspace: every program of a package under `crates/` — the
/// programs a person runs, each worker among them — is installed, every name installed is a
/// binary target, and every C library is installed. A program added under `crates/` and not
/// here fails this rather than going stale under `target/` (`doc/todo/02` section 5).
#[test]
fn install_names_every_program_and_library_the_workspace_builds_for_a_person() {
    let (person_programs, libraries) = programs_and_libraries("crates");
    let (tool_programs, _) = programs_and_libraries("tools");
    let installed = batch_list("install_binaries");
    let installed_libraries = batch_list("install_libraries");
    assert!(
        installed.len() >= 6 && person_programs.len() >= 6 && tool_programs.len() >= 20,
        "a list was not read: installed {installed:?}, crates/ {person_programs:?}, tools/ \
         {tool_programs:?}"
    );
    let not_programs: Vec<&String> = installed
        .iter()
        .filter(|name| !person_programs.contains(name) && !tool_programs.contains(name))
        .collect();
    let not_installed: Vec<&String> = person_programs
        .iter()
        .filter(|name| !installed.contains(name))
        .collect();
    let libraries_missed: Vec<&String> = libraries
        .iter()
        .filter(|name| !installed_libraries.contains(name))
        .collect();
    assert!(
        not_programs.is_empty() && not_installed.is_empty() && libraries_missed.is_empty(),
        "tools/batch.sh install_binaries/install_libraries against the workspace:\n  installed and \
         no binary target: {not_programs:?}\n  a program under crates/ not installed: \
         {not_installed:?}\n  a C library not installed: {libraries_missed:?}"
    );
}
