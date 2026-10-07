//! What a script learns about the viewer it runs in: `app.viewerType`, `viewerVersion`,
//! `viewerVariation`, `platform`, `language` and `formsVersion` (ADR 1615).
//!
//! **This program's own answers, never Acrobat's.** Adobe's "app properties" page lists the values
//! its own viewers answer — `Reader`, `Exchange`, `Exchange-Pro`; `Reader`, `Fill-In`, `Business
//! Tools`, `Full` — and each names a product and a packaging of Adobe's, of which this program is
//! none. The owner settled the question in `doc/questions/A193`, answer 11: the truth, this
//! program's name and its version. A script that branches on them — the commonest script there is
//! compares `viewerVersion` with 7 and 9 — then takes the branch a viewer that is not Adobe's
//! takes, which is what the document's author wrote that branch for. `platform` and `language`
//! answer from the reference's own lists, because those name facts rather than products: the
//! family of system this build runs on, and the language this program's interface is written in.

/// `app.viewerType`: the program's name.
pub const VIEWER_TYPE: &str = "quorra";

/// `app.viewerVariation`: the program's name again, since no packaging of it is sold apart from
/// another.
pub const VIEWER_VARIATION: &str = "quorra";

/// `app.language`: `ENU`, the reference's code for English — the one language this program's
/// interface is written in. A host that is translated states its own.
pub const LANGUAGE: &str = "ENU";

/// `app.platform`: the reference's three values — `WIN`, `MAC`, `UNIX` — the one that names the
/// system this build was compiled for, every system that is neither Windows nor macOS being of the
/// third family the reference names.
#[must_use]
pub const fn platform() -> &'static str {
    if cfg!(target_os = "windows") {
        "WIN"
    } else if cfg!(target_os = "macos") {
        "MAC"
    } else {
        "UNIX"
    }
}

/// `app.viewerVersion` and `app.formsVersion`: the program's release, major and minor, as the
/// number the reference types both as — `0.1.0` is `0.1`.
///
/// Read from this crate's manifest, which states the workspace's one release; a form's scripts
/// and the viewer's are the same software, so the two properties answer the same number.
#[must_use]
pub fn version() -> f64 {
    let mut parts = env!("CARGO_PKG_VERSION").split('.');
    let major = parts.next().unwrap_or("0");
    let minor = parts.next().unwrap_or("0");
    format!("{major}.{minor}").parse().unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_version_is_the_release_s_major_and_minor() {
        let manifest = env!("CARGO_PKG_VERSION");
        let expected: f64 = manifest
            .split('.')
            .take(2)
            .collect::<Vec<_>>()
            .join(".")
            .parse()
            .unwrap_or(f64::NAN);
        assert!((super::version() - expected).abs() < f64::EPSILON);
    }

    #[test]
    fn the_platform_is_one_of_the_reference_s_three() {
        assert!(["WIN", "MAC", "UNIX"].contains(&super::platform()));
    }
}
