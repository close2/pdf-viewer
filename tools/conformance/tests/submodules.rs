//! Every submodule this tree declares is still tracked as a submodule.
//!
//! Not a conformance question, and it lives here because this is the crate whose gate already
//! runs against the repository's own files rather than against a PDF. What it guards is a
//! failure that has reached `main` twice in one session and is invisible from inside the
//! worktree that causes it.
//!
//! A round works in a `git worktree`, where `doc/arlington-pdf-model`, `doc/pdf.js` and the four
//! `doc/corpora/*` directories are empty — a submodule's content belongs to the checkout that
//! initialised it. The cheap fix is a symlink to the primary checkout, and it works: the build
//! script finds its TSVs, the corpus gates find their PDFs, every test passes. Then `git add -A`
//! records the symlink **over** the gitlink, mode `160000` becomes `120000`, and the submodule
//! stops being tracked at all. On `main` the link points at itself and the data is gone.
//!
//! The reason it needs a gate rather than a habit is that **the round that does it cannot see
//! it**. Its own gates pass, because inside its worktree the symlink resolves to real content.
//! The damage exists only in the commit, and only once the commit is somewhere else — which is
//! exactly the shape of defect a test is for.
//!
//! What this asserts is the *mode*, not the revision. Bumping a submodule is ordinary work;
//! replacing one with a symlink never is.

#![expect(
    clippy::expect_used,
    reason = "test code: a gate that cannot run `git` in its own repository has not found a \
              defect, and reporting that as one would be worse than stopping"
)]

use std::path::Path;
use std::process::Command;

/// Where the repository root is, relative to this crate's manifest.
fn repository_root() -> &'static Path {
    // `CARGO_MANIFEST_DIR` is `<root>/tools/conformance`, so two levels up is the root. This
    // cannot fail for a crate that is in the workspace, which is the only way this test runs.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the manifest directory of a workspace member has two ancestors")
}

/// The paths `.gitmodules` declares, in the order it declares them.
fn declared_submodules(root: &Path) -> Vec<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["config", "-f", ".gitmodules", "--get-regexp", r"\.path$"])
        .output()
        .expect("git is on the path wherever this workspace builds");

    assert!(
        output.status.success(),
        ".gitmodules could not be read, so this gate cannot say anything"
    );

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.split_once(' '))
        .map(|(_key, path)| path.trim().to_owned())
        .collect()
}

/// The paths the index records as gitlinks — mode `160000`.
fn tracked_gitlinks(root: &Path) -> Vec<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "--stage"])
        .output()
        .expect("git is on the path wherever this workspace builds");

    assert!(output.status.success(), "the index could not be listed");

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let mode = fields.next()?;
            (mode == "160000").then(|| line.split('\t').nth(1))?
        })
        .map(str::to_owned)
        .collect()
}

#[test]
fn every_declared_submodule_is_still_tracked_as_one() {
    let root = repository_root();
    let declared = declared_submodules(root);

    assert!(
        !declared.is_empty(),
        "`.gitmodules` declares no submodule, which means this gate is measuring nothing — \
         either the file moved or the `git config` invocation stopped working"
    );

    let tracked = tracked_gitlinks(root);
    let missing: Vec<&String> = declared
        .iter()
        .filter(|path| !tracked.contains(path))
        .collect();

    assert!(
        missing.is_empty(),
        "{} of {} declared submodule(s) are no longer tracked as gitlinks: {missing:?}\n\
         \n\
         This is almost always a symlink committed over a submodule from inside a worktree — \
         see this file's module comment. To restore them, from the repository root, with \
         <base> a commit that still had them:\n\
         \n    git ls-tree -r <base> | awk '$2==\"commit\"{{print $4, $3}}' | while read p s; \
         do rm -f \"$p\"; git rm -q --cached \"$p\" 2>/dev/null; \
         git update-index --add --cacheinfo 160000,$s,\"$p\"; done\n",
        missing.len(),
        declared.len(),
    );
}
