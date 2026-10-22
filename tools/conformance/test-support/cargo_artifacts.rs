//! Native integration tests consume artifacts attested by their own Cargo build.
//!
//! The test's output directory chooses the profile; Cargo's JSON chooses the file. Writable
//! fixtures belong to this invocation, never to the shared build directory. This boundary is
//! deliberately test-only and native-only: a cross target needs an explicit compiler/runner.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

/// One native test invocation and the workspace that builds its dependencies.
#[derive(Debug)]
pub(super) struct Artifacts {
    workspace: PathBuf,
    profile: String,
    directory: PathBuf,
    packages: BTreeMap<String, String>,
    fixtures: tempfile::TempDir,
}

#[derive(Clone, Copy)]
enum Target<'a> {
    Library(&'a str),
    Executable(&'a str),
}

impl Target<'_> {
    fn name(self) -> &'static str {
        match self {
            Self::Library(_) => "cdylib",
            Self::Executable(_) => "bin",
        }
    }

    fn target(self) -> String {
        match self {
            Self::Library(package) => package.replace('-', "_"),
            Self::Executable(name) => name.to_owned(),
        }
    }
}

fn cargo(workspace: &Path) -> Command {
    let mut command = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
    command.current_dir(workspace);
    command
}

fn successful(output: &Output, request: &str) -> Result<(), String> {
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "{request} failed ({}):\n{}\n{}",
            output.status,
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout)
        ))
    }
}

fn profile(executable: &Path, target_directory: &Path) -> Result<(String, PathBuf), String> {
    let deps = executable.parent().ok_or("test executable has no parent")?;
    if deps.file_name().is_none_or(|name| name != "deps") {
        return Err(format!(
            "test is not in a Cargo deps directory: {}",
            executable.display()
        ));
    }
    let directory = deps.parent().ok_or("test has no profile directory")?;
    if directory.parent() != Some(target_directory) {
        return Err(format!(
            "native test {} is not directly under Cargo target directory {}; cross-target/copied tests need an explicit runner contract",
            executable.display(),
            target_directory.display()
        ));
    }
    let name = directory
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("profile name is not UTF-8")?;
    let selected = match name {
        "debug" => "dev",
        "release" => "release",
        custom => custom,
    };
    Ok((selected.to_owned(), directory.to_path_buf()))
}

impl Artifacts {
    /// Resolve exact workspace package identities once, and reject an unrelated test layout.
    ///
    /// # Errors
    /// Refuses failed metadata, ambiguous packages, non-native layouts or unavailable private storage.
    pub(super) fn new(workspace: &Path) -> Result<Self, String> {
        let workspace = workspace
            .canonicalize()
            .map_err(|error| error.to_string())?;
        let output = cargo(&workspace)
            .args(["metadata", "--no-deps", "--format-version", "1"])
            .output()
            .map_err(|error| format!("Cargo metadata: {error}"))?;
        successful(&output, "Cargo metadata")?;
        let metadata: Value =
            serde_json::from_slice(&output.stdout).map_err(|error| error.to_string())?;
        let target = metadata["target_directory"]
            .as_str()
            .ok_or("metadata has no target_directory")?;
        let target = Path::new(target)
            .canonicalize()
            .map_err(|error| error.to_string())?;
        let executable = std::env::current_exe().map_err(|error| error.to_string())?;
        let (profile, directory) = profile(&executable, &target)?;
        let mut packages = BTreeMap::new();
        for package in metadata["packages"]
            .as_array()
            .ok_or("metadata has no packages")?
        {
            let name = package["name"].as_str().ok_or("package has no name")?;
            let id = package["id"].as_str().ok_or("package has no id")?;
            if packages.insert(name.to_owned(), id.to_owned()).is_some() {
                return Err(format!("ambiguous workspace package {name}"));
            }
        }
        let fixtures = tempfile::Builder::new()
            .prefix("quorra-native-test-")
            .tempdir()
            .map_err(|error| format!("private fixtures: {error}"))?;
        Ok(Self {
            workspace,
            profile,
            directory,
            packages,
            fixtures,
        })
    }

    /// The invocation's private executable, writable PDF and native build directory.
    pub(super) fn fixtures(&self) -> &Path {
        self.fixtures.path()
    }

    /// Build and select this package's C dynamic library in the test's own profile.
    ///
    /// # Errors
    /// Refuses a failed build or a missing, ambiguous or wrong-profile artifact.
    pub(super) fn library(&self, package: &str) -> Result<PathBuf, String> {
        self.build(package, Target::Library(package), &[])
    }

    /// Build and select one worker, preserving its explicitly requested features.
    ///
    /// # Errors
    /// Refuses a failed build or a missing, ambiguous or wrong-profile artifact.
    pub(super) fn executable(
        &self,
        package: &str,
        name: &str,
        features: &[&str],
    ) -> Result<PathBuf, String> {
        self.build(package, Target::Executable(name), features)
    }

    fn build(
        &self,
        package: &str,
        target: Target<'_>,
        features: &[&str],
    ) -> Result<PathBuf, String> {
        let id = self
            .packages
            .get(package)
            .ok_or_else(|| format!("no workspace package {package}"))?;
        let mut command = cargo(&self.workspace);
        command.args([
            "build",
            "-p",
            id,
            "--profile",
            &self.profile,
            "--message-format=json-render-diagnostics",
        ]);
        match target {
            Target::Library(_) => {
                command.arg("--lib");
            }
            Target::Executable(name) => {
                command.args(["--bin", name]);
            }
        }
        if !features.is_empty() {
            command.args(["--features", &features.join(",")]);
        }
        let request = format!(
            "Cargo {package}/{} profile {} in {}",
            target.target(),
            self.profile,
            self.directory.display()
        );
        let output = command
            .output()
            .map_err(|error| format!("{request}: {error}"))?;
        successful(&output, &request)?;
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
        let artifact = select(&output.stdout, id, target, &self.directory)
            .map_err(|error| format!("{request}: {error}"))?;
        println!("native artifact: {request} -> {}", artifact.display());
        Ok(artifact)
    }
}

fn select(
    json: &[u8],
    package: &str,
    target: Target<'_>,
    directory: &Path,
) -> Result<PathBuf, String> {
    let mut found = Vec::new();
    for line in json
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
    {
        let message: Value =
            serde_json::from_slice(line).map_err(|error| format!("invalid Cargo JSON: {error}"))?;
        if message["reason"] != "compiler-artifact"
            || message["package_id"] != package
            || message["target"]["name"] != target.target()
        {
            continue;
        }
        let includes = |field: &str| {
            message["target"][field]
                .as_array()
                .is_some_and(|values| values.iter().any(|value| value == target.name()))
        };
        if !includes("kind") || !includes("crate_types") {
            continue;
        }
        match target {
            Target::Executable(_) => {
                if let Some(path) = message["executable"].as_str() {
                    found.push(PathBuf::from(path));
                }
            }
            Target::Library(_) => {
                for filename in message["filenames"]
                    .as_array()
                    .ok_or("artifact has no filenames")?
                {
                    let path = PathBuf::from(
                        filename
                            .as_str()
                            .ok_or("artifact filename is not a string")?,
                    );
                    if path.extension().is_some_and(|extension| extension == "so") {
                        found.push(path);
                    }
                }
            }
        }
    }
    if found.len() != 1 {
        return Err(format!(
            "expected exactly one artifact, got {}: {found:?}",
            found.len()
        ));
    }
    let path = found
        .remove(0)
        .canonicalize()
        .map_err(|error| format!("reported artifact cannot be resolved: {error}"))?;
    let directory = directory
        .canonicalize()
        .map_err(|error| error.to_string())?;
    if path.parent() != Some(directory.as_path()) {
        return Err(format!(
            "reported artifact {} is outside selected profile {}",
            path.display(),
            directory.display()
        ));
    }
    Ok(path)
}

#[cfg(test)]
#[path = "cargo_artifacts_tests.rs"]
mod tests;
