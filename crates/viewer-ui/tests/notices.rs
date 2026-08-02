//! The attribution this binary is obliged to carry, checked by this tree because nothing else
//! can.
//!
//! `cargo deny check licenses` reads Cargo metadata. **It cannot see vendored data**, so the
//! fourteen font programs compiled into this binary are invisible to it — and those are the ones
//! whose licences place an obligation on a *binary* distribution rather than on a source one.
//! BSD-3-Clause's second condition is the one with teeth: a redistribution in binary form must
//! reproduce the copyright notice, the conditions and the disclaimer "in the documentation
//! and/or other materials provided with the distribution", and a program with no such material
//! is not distributing them.
//!
//! So the check is a test of our own, and what it checks is the thing that actually decays: a
//! font added to `data/` without a line in `NOTICE`.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "test code: a missing notice must fail loudly, and naming the file it is missing \
              for is what makes the failure useful"
)]

use std::path::{Path, PathBuf};

/// The repository root, from this crate's manifest.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The `NOTICE` the binary carries, read from the same file `--licences` prints.
fn notice() -> String {
    std::fs::read_to_string(root().join("NOTICE")).expect("NOTICE is committed at the root")
}

#[test]
fn every_vendored_font_is_named_in_the_notice() {
    let dir = root().join("data/standard-fonts");
    let notice = notice();
    let mut checked = 0;
    for entry in std::fs::read_dir(&dir).expect("data/standard-fonts exists") {
        let path = entry.expect("a readable directory entry").path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        // Only the font programs. The licence texts, the provenance note and the digests are
        // *about* them and are named in the notice as files rather than one by one.
        let extension = path
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or_default();
        if !matches!(extension, "pfb" | "ttf") {
            continue;
        }
        // By file name, not by family. A family glob would let a fifth Liberation weight arrive
        // under a line that was written about four, and the whole point of this test is the
        // file that gets added without its notice.
        assert!(
            notice.contains(name),
            "{name} is compiled into the binary and NOTICE does not name it"
        );
        checked += 1;
    }
    assert_eq!(checked, 14, "§9.6.2.2's fourteen are what is vendored");
}

/// The `CMap`s are named as a set rather than one by one, and the set's size is the assertion.
///
/// 239 file names in `NOTICE` would be a page of noise nobody reads, and BSD-3-Clause asks for
/// the notice and the disclaimer rather than for an inventory. What this checks instead is the
/// thing that decays: that the count in `NOTICE` still matches what is on disk, and that each
/// file carries Adobe's notice inline as Adobe published it — which is the other, stronger way
/// the obligation is met.
#[test]
fn every_vendored_cmap_carries_the_notice_it_was_published_with() {
    let dir = root().join("data/cmaps");
    let notice = notice();
    let mut checked = 0;
    for entry in std::fs::read_dir(&dir).expect("data/cmaps exists") {
        let path = entry.expect("a readable directory entry").path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if matches!(name, "LICENSE_ADOBE" | "SHA256SUMS" | "PROVENANCE.md") {
            continue;
        }
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{name} is readable: {error}"));
        // The year varies by file — Adobe reissued some of them in 2020 — so the assertion is
        // on the sentence rather than on the date, and on the disclaimer that must travel with
        // it. Pinning the year would have been a test that fails on the next reissue for a
        // reason that is not a licence question.
        assert!(
            text.contains("Adobe. All rights reserved."),
            "{name} does not carry Adobe's copyright line, so it is not the file that was vetted"
        );
        assert!(
            text.contains("THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS"),
            "{name} does not carry the disclaimer BSD-3-Clause requires beside the notice"
        );
        checked += 1;
    }
    assert_eq!(checked, 239, "the count NOTICE states is the count on disk");
    assert!(
        notice.contains("All 239 of"),
        "NOTICE states a different number of CMaps than data/cmaps holds"
    );
}

#[test]
fn the_notice_carries_what_each_licence_requires_verbatim() {
    let notice = notice();

    // BSD-3-Clause's three obligations, and the disclaimer it says must come with them.
    for required in [
        "Copyright 2014 PDFium Authors. All rights reserved.",
        "Redistributions in binary form must reproduce the above copyright notice",
        "Neither the name of Google Inc. nor the names of its contributors may be used to endorse",
        "THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS \"AS IS\"",
    ] {
        assert!(
            notice.contains(required),
            "NOTICE is missing the Foxit licence's: {required}"
        );
    }

    // SIL OFL 1.1's copyright lines and the reserved font name, which is the obligation that
    // decides what may be done with the files rather than only who is credited.
    for required in [
        "Copyright (c) 2012 Red Hat, Inc.",
        "with Reserved Font Name Liberation.",
        "SIL Open Font License, Version 1.1",
    ] {
        assert!(
            notice.contains(required),
            "NOTICE is missing the Liberation licence's: {required}"
        );
    }
}

#[test]
fn the_licence_texts_are_beside_what_they_cover() {
    for (dir, names) in [
        (
            "data/standard-fonts",
            ["LICENSE_FOXIT", "LICENSE_LIBERATION", "PROVENANCE.md"].as_slice(),
        ),
        ("data/cmaps", ["LICENSE_ADOBE", "PROVENANCE.md"].as_slice()),
    ] {
        licences_beside(&root().join(dir), names);
    }
}

/// Every named file is present beside the data and is not a stub.
fn licences_beside(dir: &Path, names: &[&str]) {
    {
        for name in names {
            let path = dir.join(name);
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("{} is committed: {error}", path.display()));
            assert!(text.len() > 512, "{name} is present but nearly empty");
        }
    }
}

#[test]
fn the_vendored_fonts_are_the_bytes_that_were_vetted() {
    // `SHA256SUMS` is not a security boundary — anyone who can change the fonts can change the
    // sums. It is a record that these fourteen files are the fourteen that were read, licensed
    // and measured, so that a later "where did this glyph come from" has an answer. What this
    // catches is a font replaced or truncated without the record moving with it.
    let dir = root().join("data/standard-fonts");
    let sums = std::fs::read_to_string(dir.join("SHA256SUMS")).expect("SHA256SUMS is committed");
    let mut checked = 0;
    for line in sums.lines() {
        let Some((expected, name)) = line.split_once("  ") else {
            continue;
        };
        let bytes = std::fs::read(dir.join(name))
            .unwrap_or_else(|error| panic!("{name} is committed: {error}"));
        let digest = <sha2::Sha256 as sha2::Digest>::digest(&bytes);
        let actual = digest.iter().fold(String::new(), |mut text, byte| {
            use std::fmt::Write as _;
            let _ = write!(text, "{byte:02x}");
            text
        });
        assert_eq!(actual, expected, "{name}");
        checked += 1;
    }
    assert_eq!(checked, 14, "one line per vendored font program");
}

/// The same record for the `CMap`s, where it is worth more: a mapping table that changed by a
/// line would draw the wrong glyph for one code and nothing in this tree would say so.
#[test]
fn the_vendored_cmaps_are_the_bytes_that_were_vetted() {
    let dir = root().join("data/cmaps");
    let sums = std::fs::read_to_string(dir.join("SHA256SUMS")).expect("SHA256SUMS is committed");
    let mut checked = 0;
    for line in sums.lines() {
        let Some((expected, name)) = line.split_once("  ") else {
            continue;
        };
        let bytes = std::fs::read(dir.join(name))
            .unwrap_or_else(|error| panic!("{name} is committed: {error}"));
        let digest = <sha2::Sha256 as sha2::Digest>::digest(&bytes);
        let actual = digest.iter().fold(String::new(), |mut text, byte| {
            use std::fmt::Write as _;
            let _ = write!(text, "{byte:02x}");
            text
        });
        assert_eq!(actual, expected, "{name}");
        checked += 1;
    }
    assert_eq!(
        checked, 241,
        "239 CMaps, the licence and the provenance note"
    );
}
