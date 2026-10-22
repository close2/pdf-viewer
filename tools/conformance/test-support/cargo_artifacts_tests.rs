//! Controls of the attestation boundary without rebuilding a workspace per case.

use super::{Artifacts, Target, profile, select};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::process::Command;

fn artifact(path: &Path, fresh: bool) -> Value {
    json!({"reason":"compiler-artifact", "package_id":"path+file:///workspace/example#0.1.0",
        "target":{"name":"example", "kind":["cdylib","staticlib","rlib"],
            "crate_types":["cdylib","staticlib","rlib"]},
        "filenames":[path], "executable":null, "fresh":fresh})
}

fn selected(messages: &[Value], directory: &Path) -> Result<PathBuf, String> {
    let stream = messages
        .iter()
        .map(Value::to_string)
        .collect::<Vec<_>>()
        .join("\n");
    select(
        stream.as_bytes(),
        "path+file:///workspace/example#0.1.0",
        Target::Library("example"),
        directory,
    )
}

#[test]
fn native_profiles_are_names_not_optimization_guesses() {
    let root = Path::new("/external/target");
    for (directory, expected) in [
        ("debug", "dev"),
        ("release", "release"),
        ("gates", "gates"),
        ("custom", "custom"),
    ] {
        let executable = root.join(directory).join("deps/client-hash");
        assert_eq!(
            profile(&executable, root).expect("native layout").0,
            expected
        );
    }
    for executable in [
        root.join("debug/client"),
        root.join("x86_64-unknown-linux-gnu/debug/deps/client"),
        PathBuf::from("/copied/deps/client"),
    ] {
        assert!(profile(&executable, root).is_err());
    }
}

#[test]
fn cargo_attestation_selects_exact_package_target_and_dynamic_library() {
    let temp = tempfile::tempdir().expect("private directory");
    let library = temp.path().join("libexample.so");
    std::fs::write(&library, b"fixture").expect("fixture");
    let expected = artifact(&library, true);
    let mut unrelated = expected.clone();
    unrelated["package_id"] = json!("path+file:///other/example#0.1.0");
    let mut target = expected.clone();
    target["target"]["name"] = json!("other");
    let mut rlib = expected.clone();
    rlib["target"]["kind"] = json!(["rlib"]);
    rlib["target"]["crate_types"] = json!(["rlib"]);
    assert_eq!(
        selected(
            &[
                unrelated.clone(),
                target.clone(),
                rlib.clone(),
                expected.clone()
            ],
            temp.path()
        )
        .expect("fresh artifact is valid"),
        library.canonicalize().expect("fixture")
    );
    for messages in [
        vec![],
        vec![unrelated],
        vec![target],
        vec![rlib],
        vec![expected.clone(), expected],
    ] {
        assert!(selected(&messages, temp.path()).is_err());
    }
}

#[test]
fn stale_matching_file_cannot_authorize_a_wrong_profile_build() {
    let temp = tempfile::tempdir().expect("private directory");
    let gates = temp.path().join("gates");
    let debug = temp.path().join("debug");
    for directory in [&gates, &debug] {
        std::fs::create_dir(directory).expect("profile directory");
    }
    let stale = gates.join("libexample.so");
    let reported = debug.join("libexample.so");
    std::fs::write(&stale, b"stale gates library").expect("seed stale library privately");
    std::fs::write(&reported, b"new default-profile library").expect("reported library");
    let refused = selected(&[artifact(&reported, true)], &gates)
        .expect_err("a successful default build cannot attest gates");
    assert!(refused.contains("outside selected profile"));
    assert_eq!(
        std::fs::read(&stale).expect("stale file preserved"),
        b"stale gates library"
    );
    assert_eq!(
        selected(&[artifact(&stale, true)], &gates)
            .expect("matching fresh build attests unchanged file"),
        stale
    );
    assert!(select(b"not JSON", "example", Target::Library("example"), &gates).is_err());
}

#[test]
fn workers_require_the_reported_executable_and_matching_kind() {
    let temp = tempfile::tempdir().expect("private directory");
    let worker = temp.path().join("worker");
    std::fs::write(&worker, b"worker").expect("fixture");
    let mut message = json!({"reason":"compiler-artifact", "package_id":"exact-id",
        "target":{"name":"worker", "kind":["bin"], "crate_types":["bin"]},
        "executable":worker,"filenames":[],"fresh":true});
    let choose = |message: &Value| {
        select(
            message.to_string().as_bytes(),
            "exact-id",
            Target::Executable("worker"),
            temp.path(),
        )
    };
    assert_eq!(choose(&message).expect("worker"), worker);
    message["executable"] = Value::Null;
    assert!(choose(&message).is_err());
}

#[test]
fn fixture_child() {
    let Some(record) = std::env::var_os("QUORRA_NATIVE_FIXTURE_PROBE") else {
        return;
    };
    let artifacts = Artifacts::new(Path::new(env!("CARGO_MANIFEST_DIR")))
        .expect("the child resolves its actual Cargo workspace and profile");
    let fixture = artifacts.fixtures().join("same-name.pdf");
    std::fs::write(&fixture, b"private").expect("private file");
    std::fs::write(record, fixture.as_os_str().as_encoded_bytes()).expect("record private path");
}

#[test]
fn separate_processes_get_disjoint_writable_paths_and_clean_them() {
    let temp = tempfile::tempdir().expect("records");
    let executable = std::env::current_exe().expect("test binary");
    let mut children = Vec::new();
    let records = [temp.path().join("one"), temp.path().join("two")];
    for record in &records {
        children.push(
            Command::new(&executable)
                .args([
                    "--exact",
                    "cargo_artifacts::tests::fixture_child",
                    "--nocapture",
                ])
                .env("QUORRA_NATIVE_FIXTURE_PROBE", record)
                .spawn()
                .expect("probe child"),
        );
    }
    for mut child in children {
        assert!(child.wait().expect("child exits").success());
    }
    let paths = records
        .map(|record| PathBuf::from(std::fs::read_to_string(record).expect("path recorded")));
    assert_ne!(paths[0], paths[1]);
    assert!(paths.iter().all(|path| !path.exists()));
}
