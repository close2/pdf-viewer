//! `tools/fuzz.sh` under a data limit builds without the sanitiser and starts in a scratch directory.
//!
//! # What it guards
//!
//! `tools/bounded.sh` always sets `RLIMIT_DATA`, and the address sanitiser reserves its shadow memory as
//! one mapping at start that the limit refuses, so a sanitised fuzz build under the wrapper never
//! starts; and libFuzzer's `-jobs` writes its log into the directory the run starts in, which in
//! `fuzz/` is a tracked crate's directory (trap 221). The wrapper answers both by reading the limit
//! itself: under one it passes `-s none` and starts in `--scratch DIR`, or in
//! `scratchpad/r<ROUND_SESSION>/fuzz-<target>/`, and it refuses with neither (ADR 2127).
//!
//! The wrapper is run for real, against a stand-in `cargo` and `cargo-fuzz` put first on `PATH`
//! that record the directory they were started in and their arguments and print libFuzzer's two
//! coverage lines; nothing is built and nothing fuzzed. The case with no limit runs only where the
//! test's own hard limit allows it, and says so where it does not.

use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::PathBuf;
use std::process::{Command, Output};

/// A target `fuzz/Cargo.toml` declares and `doc/verify.md` gives a line, which the wrapper checks
/// before it runs anything.
const TARGET: &str = "object";

/// The data limit the bounded cases run under, in KiB: 4 GiB, a figure `tools/bounded.sh` hands a
/// fuzz campaign (`--data 4`).
const DATA_LIMIT_KIB: u64 = 4 << 20;

/// One case's directory, holding the stand-ins under `bin/` and what `cargo` recorded in `record`.
struct Case {
    /// The case's own directory under the temporary directory, removed when the case ends.
    dir: PathBuf,
    /// Where the stand-in `cargo` and `cargo-fuzz` are, first on the wrapper's `PATH`.
    bin: PathBuf,
    /// What the stand-in `cargo` appends its starting directory and arguments to.
    record: PathBuf,
}

impl Case {
    /// A fresh directory for the case named, with the stand-ins written into it.
    fn new(name: &str) -> std::io::Result<Self> {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_nanos());
        let dir = std::env::temp_dir().join(format!(
            "fuzz-wrapper-{name}-{}-{nanos}",
            std::process::id()
        ));
        let (bin, record) = (dir.join("bin"), dir.join("record"));
        fs::create_dir_all(&bin)?;
        let cargo = format!(
            "#!/usr/bin/env bash\nprintf 'pwd=%s\\nargs=%s\\n' \"$PWD\" \"$*\" >> '{}'\n\
             echo '#2\tINITED cov: 5 ft: 6 corp: 1/1b'\necho '#9\tDONE   cov: 7 ft: 8 corp: 2/2b'\n",
            record.display()
        );
        for (name, body) in [
            ("cargo", cargo.as_str()),
            ("cargo-fuzz", "#!/bin/sh\nexit 0\n"),
        ] {
            let path = bin.join(name);
            fs::write(&path, body)?;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755))?;
        }
        Ok(Self { dir, bin, record })
    }

    /// Runs the wrapper under `limit` (a `ulimit -d` argument) with the stand-ins first on `PATH`
    /// and no `ROUND_SESSION`, so that no case writes under the tree's `scratchpad/`; returns its
    /// output and what the stand-in `cargo` recorded, empty when it never ran.
    fn run(&self, limit: &str, arguments: &[&str]) -> std::io::Result<(Output, String)> {
        let path = format!(
            "{}:{}",
            self.bin.display(),
            std::env::var("PATH").unwrap_or_default()
        );
        let output = Command::new("bash")
            .arg("-c")
            .arg(format!(
                "ulimit -d {limit} || exit 97; exec tools/fuzz.sh \"$@\""
            ))
            .arg("fuzz.sh")
            .arg(TARGET)
            .args(arguments)
            .current_dir(root()?)
            .env("PATH", path)
            .env_remove("ROUND_SESSION")
            .output()?;
        let recorded = fs::read_to_string(&self.record).unwrap_or_default();
        Ok((output, recorded))
    }
}

impl Drop for Case {
    fn drop(&mut self) {
        // A directory left under the temporary directory is litter, not a failure of the case.
        let _ = fs::remove_dir_all(&self.dir);
    }
}

/// The workspace root as the wrapper's own `cd .. && pwd` spells it.
fn root() -> std::io::Result<PathBuf> {
    fs::canonicalize(conformance::workspace_root())
}

/// Under a data limit the build is unsanitised and the run starts in the scratch directory named.
#[test]
fn under_a_data_limit_the_run_is_unsanitised_and_starts_in_scratch() {
    let case = Case::new("limited").expect("a case directory");
    let start = case.dir.join("start").display().to_string();
    let (output, seen) = case
        .run(
            &DATA_LIMIT_KIB.to_string(),
            &["--from-nothing", "--scratch", &start],
        )
        .expect("bash runs tools/fuzz.sh");
    let fuzz_dir = root().expect("the workspace root").join("fuzz");
    assert!(
        output.status.success(),
        "fuzz.sh exited {:?}:\n{}{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(seen.contains(&format!("pwd={start}\n")), "{seen}");
    assert!(
        seen.contains(&format!(
            "--fuzz-dir {} -s none {TARGET} ",
            fuzz_dir.display()
        )),
        "{seen}"
    );
}

/// Under a data limit with no scratch directory and no session the wrapper refuses, and cargo is
/// never started.
#[test]
fn under_a_data_limit_without_scratch_the_run_is_refused() {
    let case = Case::new("refused").expect("a case directory");
    let (output, seen) = case
        .run(&DATA_LIMIT_KIB.to_string(), &["--from-nothing"])
        .expect("bash runs tools/fuzz.sh");
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(
        seen.is_empty(),
        "cargo ran although the wrapper refused: {seen}"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("trap 221"),
        "{output:?}"
    );
}

/// With no data limit the sanitised build runs from `fuzz/`, as `doc/verify.md`'s lines state it.
#[test]
fn without_a_data_limit_the_sanitised_run_starts_in_fuzz() {
    let case = Case::new("unlimited").expect("a case directory");
    let (output, seen) = case
        .run("unlimited", &["--from-nothing"])
        .expect("bash runs tools/fuzz.sh");
    if output.status.code() == Some(97) {
        eprintln!(
            "not tested here: this process's hard RLIMIT_DATA is set, so no child can run without a \
             data limit; the two limited cases above still ran"
        );
        return;
    }
    let fuzz_dir = root().expect("the workspace root").join("fuzz");
    assert!(output.status.success(), "{output:?}");
    assert!(
        seen.contains(&format!("pwd={}\n", fuzz_dir.display())),
        "{seen}"
    );
    assert!(!seen.contains("-s none"), "{seen}");
}
