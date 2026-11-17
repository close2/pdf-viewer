//! Where a program this workspace builds is found, from the path of the program that is running.
//!
//! **One rule, for every lookup in the tree that finds a program or a directory from its own
//! path** (ADR 2914). A shipped program finds its worker beside itself; a test finds the same
//! worker in the build directory it was linked into, and where Cargo puts that test is Cargo's
//! choice, which changed: up to 1.99 a test executable sat in `<profile>/deps/`, and from 1.100 it
//! sits in `<profile>/build/<package>/<hash>/out/`, while programs stay in `<profile>/` under both.
//!
//! So the search is: beside the executable, then one directory up — the two places a shipped
//! program and a test under the old layout ever looked — and then the Cargo profile directory,
//! where [`cargo_profile_directory`] recognises one. The third place is new only for an
//! executable inside `<profile>/build/<package>/<hash>/out/`, a directory Cargo writes and no
//! installation does, so a shipped program's search outside a build directory is unchanged.
//!
//! What Cargo states outright is preferred wherever it is available: a test of the package that
//! owns a program reads `env!("CARGO_BIN_EXE_<name>")`, and a test's scratch files go under
//! `env!("CARGO_TARGET_TMPDIR")`. Neither reaches a library that a test of *another* package
//! links, and neither reaches a shipped program, which is why this search exists at all.

use std::path::{Path, PathBuf};

/// Why an executable's path names no Cargo profile directory.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error(
    "{} is not in a Cargo profile's `deps/`, `examples/` or `build/<package>/<hash>/out/` \
     directory, so it names no profile directory",
    executable.display()
)]
pub struct NotInCargoProfile {
    /// The executable whose path was read.
    pub executable: PathBuf,
}

/// The Cargo profile directory (`<target>/<profile>`) an executable was built into.
///
/// Read from the path alone, by the shape of the directories Cargo writes:
///
/// - `<profile>/deps/<file>` — a test executable up to Cargo 1.99, and a dependency's file under
///   every version;
/// - `<profile>/examples/<file>` — an example, under both layouts;
/// - `<profile>/build/<package>/<hash>/out/<file>` — a test executable from Cargo 1.100.
///
/// A program directly in `<profile>/` is not recognised: its directory is a profile directory
/// only by convention, and a path such as `/usr/bin/quorra` would otherwise name `/usr/bin` one.
/// Nothing on the file system is read, so the answer is the same for a path that does not exist.
///
/// # Errors
///
/// [`NotInCargoProfile`], naming the executable, where the path has none of the three shapes.
pub fn cargo_profile_directory(executable: &Path) -> Result<&Path, NotInCargoProfile> {
    let not_in_profile = || NotInCargoProfile {
        executable: executable.to_path_buf(),
    };
    let named = |directory: Option<&Path>, name: &str| {
        directory
            .and_then(Path::file_name)
            .is_some_and(|file_name| file_name == name)
    };
    // `ancestors()` yields the path itself first, so index 1 is the executable's directory.
    let above: Vec<&Path> = executable.ancestors().take(6).collect();
    let directory = above.get(1).copied();
    let profile = if named(directory, "deps") || named(directory, "examples") {
        above.get(2).copied()
    } else if named(directory, "out") && named(above.get(4).copied(), "build") {
        above.get(5).copied()
    } else {
        None
    };
    profile
        .filter(|profile| profile.file_name().is_some())
        .ok_or_else(not_in_profile)
}

/// The places a program is searched for, in order, for an executable at `executable`.
///
/// Beside it, one directory up, and the Cargo profile directory where the path names one; a
/// place already listed is not listed twice.
#[must_use]
pub fn search_places(executable: &Path) -> Vec<PathBuf> {
    let directory = executable.parent().unwrap_or(executable);
    let mut places = vec![directory.to_path_buf()];
    if let Some(parent) = directory.parent() {
        places.push(parent.to_path_buf());
    }
    if let Ok(profile) = cargo_profile_directory(executable)
        && !places.iter().any(|place| place == profile)
    {
        places.push(profile.to_path_buf());
    }
    places
}

/// The program named `file_name` (with its platform's suffix already on it) beside
/// `executable`, by [`search_places`]' order, or `None` where none of them holds it as a file.
#[must_use]
pub fn program(executable: &Path, file_name: &str) -> Option<PathBuf> {
    search_places(executable)
        .into_iter()
        .map(|place| place.join(file_name))
        .find(|candidate| candidate.is_file())
}

#[cfg(test)]
mod tests {
    use super::{NotInCargoProfile, cargo_profile_directory, program, search_places};
    use std::path::{Path, PathBuf};

    #[test]
    fn both_layouts_name_the_same_profile_directory() {
        let target = Path::new("/work/target");
        for (executable, profile) in [
            ("debug/deps/confinement-0123abcd", "debug"),
            (
                "debug/build/pdf-sandbox/0123abcd/out/confinement-0123abcd",
                "debug",
            ),
            ("gates/build/pdf-model/89abcdef/out/ccitt-89abcdef", "gates"),
            ("release/examples/script_peak", "release"),
            (
                "x86_64-unknown-linux-gnu/release/deps/launch_path-0",
                "x86_64-unknown-linux-gnu/release",
            ),
        ] {
            assert_eq!(
                cargo_profile_directory(&target.join(executable)),
                Ok(target.join(profile).as_path()),
                "{executable}"
            );
        }
    }

    #[test]
    fn a_path_with_no_profile_directory_is_refused_by_name() {
        for executable in [
            "/usr/bin/quorra",
            "/work/target/debug/quorra",
            "/work/target/debug/build/pdf-model/out/ccitt",
            "/work/target/debug/build/pdf-model-0123abcd/out",
            "/deps/client",
            "deps/client",
            "client",
            "",
        ] {
            let executable = Path::new(executable);
            assert_eq!(
                cargo_profile_directory(executable),
                Err(NotInCargoProfile {
                    executable: executable.to_path_buf()
                }),
                "{}",
                executable.display()
            );
        }
        let refusal = cargo_profile_directory(Path::new("/usr/bin/quorra"))
            .err()
            .map(|refusal| refusal.to_string())
            .unwrap_or_default();
        assert!(
            refusal.starts_with("/usr/bin/quorra is not in a Cargo profile"),
            "{refusal}"
        );
    }

    #[test]
    fn a_shipped_program_searches_only_beside_itself_and_one_up() {
        assert_eq!(
            search_places(Path::new("/opt/quorra/bin/quorra")),
            [
                PathBuf::from("/opt/quorra/bin"),
                PathBuf::from("/opt/quorra")
            ]
        );
        assert_eq!(
            search_places(Path::new("/work/target/debug/deps/confinement-0")),
            [
                PathBuf::from("/work/target/debug/deps"),
                PathBuf::from("/work/target/debug")
            ]
        );
        assert_eq!(
            search_places(Path::new(
                "/work/target/debug/build/pdf-sandbox/0123/out/confinement-0"
            )),
            [
                PathBuf::from("/work/target/debug/build/pdf-sandbox/0123/out"),
                PathBuf::from("/work/target/debug/build/pdf-sandbox/0123"),
                PathBuf::from("/work/target/debug"),
            ]
        );
    }

    /// The plant: a test under the new layout, with no `deps/` anywhere on its path, finds the
    /// program in the profile directory, and the same test with the profile's program removed
    /// finds nothing.
    #[test]
    fn a_program_is_found_from_a_test_that_has_no_deps_directory() {
        let target = tempfile::tempdir().expect("a private directory");
        let profile = target.path().join("debug");
        let out = profile.join("build/pdf-model/0123abcd/out");
        std::fs::create_dir_all(&out).expect("the test's directory");
        let test = out.join("ccitt-0123abcd");
        std::fs::write(&test, b"").expect("the test");
        assert_eq!(program(&test, "pdf-sandbox-worker"), None);
        let worker = profile.join("pdf-sandbox-worker");
        std::fs::write(&worker, b"").expect("the worker");
        assert_eq!(program(&test, "pdf-sandbox-worker"), Some(worker));
    }
}
