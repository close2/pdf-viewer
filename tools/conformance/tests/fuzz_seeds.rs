//! `fuzz/seeds.sh`'s `x509` arm seeds every certificate this tree's fixture modules state.
//!
//! # What it guards
//!
//! `fuzz/seed_x509.py`'s third route reads the certificates the signature crate's `fixtures` modules
//! state in hexadecimal, and those are the only inputs that reach `dsa::verify` and the curve arms
//! of `ecdsa` and `eddsa` (`doc/verify.md`, under `x509`). The arm names the sources it hands the
//! seeder, so a fixture module that moves out of them seeds nothing, and the seeder's summary line
//! reports a smaller figure with nothing failing.
//!
//! The arm is run for real, from a byte copy of `fuzz/` in a temporary tree whose `crates` is a
//! link to this workspace's, so the documents the recipe also walks are absent and the run reads
//! the fixtures alone. The figure it reports is compared with the certificate constants counted
//! here, from the tree, by the naming rule `seed_x509.py`'s `FIXTURE` states.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The workspace root, canonical, since the temporary tree links to its `crates`.
fn root() -> std::io::Result<PathBuf> {
    fs::canonicalize(conformance::workspace_root())
}

/// Every `.rs` file under `dir`, recursively.
fn rust_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            rust_sources(&path, out);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            out.push(path);
        }
    }
}

/// The distinct certificates the tree's sources state as a `&str` constant whose name holds
/// `CERTIFICATE`, each as its lowercase hexadecimal digits, with the file that states it.
fn fixture_certificates(root: &Path) -> BTreeSet<(String, String)> {
    let mut sources = Vec::new();
    rust_sources(&root.join("crates"), &mut sources);
    let mut found = BTreeSet::new();
    for path in sources {
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let mut rest = text.as_str();
        while let Some(at) = rest.find("CERTIFICATE") {
            rest = &rest[at..];
            let declaration = rest.split_once(": &str = \"").filter(|(name, _)| {
                name.chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
            });
            let Some((_, body)) = declaration else {
                rest = &rest["CERTIFICATE".len()..];
                continue;
            };
            let Some((value, after)) = body.split_once('"') else {
                break;
            };
            let digits: String = value
                .chars()
                .filter(char::is_ascii_hexdigit)
                .map(|c| c.to_ascii_lowercase())
                .collect();
            let file = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .display()
                .to_string();
            found.insert((digits, file));
            rest = after;
        }
    }
    found
}

/// The number before `label` in the seeder's summary line.
fn figure_before(summary: &str, label: &str) -> Option<usize> {
    let before = summary.split(label).next()?;
    before.split_whitespace().last()?.parse().ok()
}

/// A seed run with no documents reaches every fixture certificate, and only by the fixture route.
#[test]
fn the_x509_arm_seeds_every_fixture_certificate() {
    let root = root().expect("the workspace root resolves");
    let expected = fixture_certificates(&root);
    let distinct: BTreeSet<&String> = expected.iter().map(|(digits, _)| digits).collect();
    assert!(
        !distinct.is_empty(),
        "no `…CERTIFICATE…: &str` constant under crates/, so this test counts nothing"
    );

    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_nanos());
    let tree = std::env::temp_dir().join(format!("fuzz-seeds-{}-{nanos}", std::process::id()));
    let fuzz = tree.join("fuzz");
    fs::create_dir_all(&fuzz).expect("the temporary tree");
    for entry in fs::read_dir(root.join("fuzz"))
        .expect("fuzz/ is readable")
        .flatten()
    {
        let path = entry.path();
        let copied = path.extension().is_some_and(|extension| extension == "py")
            || path.file_name().is_some_and(|name| name == "seeds.sh");
        if copied {
            fs::copy(&path, fuzz.join(entry.file_name())).expect("a fuzz/ file copies");
        }
    }
    std::os::unix::fs::symlink(root.join("crates"), tree.join("crates")).expect("crates links");

    let output = Command::new("bash")
        .arg(fuzz.join("seeds.sh"))
        .arg(tree.join("out"))
        .arg("x509")
        .output()
        .expect("bash runs the copied fuzz/seeds.sh");
    let seeds = fs::read_dir(tree.join("out").join("x509")).map_or(0, Iterator::count);
    // A directory left under the temporary directory is litter, not a failure of the test.
    let _ = fs::remove_dir_all(&tree);

    let stdout = String::from_utf8_lossy(&output.stdout);
    let report = format!("{stdout}{}", String::from_utf8_lossy(&output.stderr));
    assert!(
        output.status.success(),
        "seeds.sh exited {:?}:\n{report}",
        output.status.code()
    );
    let summary = stdout
        .lines()
        .find(|line| line.contains("distinct certificate(s)"))
        .unwrap_or_else(|| panic!("no summary line from seed_x509.py:\n{report}"));
    let routes = [
        ("first seen inside a signature", 0),
        ("stated by a document", 0),
        ("out of a fixture", distinct.len()),
    ];
    for (label, wanted) in routes {
        assert_eq!(
            figure_before(summary, label),
            Some(wanted),
            "route `{label}`: the tree states {} fixture certificate(s) in {:?}\n{report}",
            distinct.len(),
            expected
                .iter()
                .map(|(_, file)| file)
                .collect::<BTreeSet<_>>()
        );
    }
    assert_eq!(
        seeds,
        distinct.len(),
        "a fixture is kept whatever its shape\n{report}"
    );
}
