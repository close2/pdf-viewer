//! Generates the full case-folding table from the Unicode Character Database.
//!
//! Reads `data/unicode/CaseFolding.txt` and emits one `static` Rust table into `OUT_DIR`, which
//! `src/case.rs` includes. ISO 32000-2 §12.3.5.2 sends the comparison of two names in one folder
//! to case normalization as Unicode Standard Annex #21 defines it, and that annex's caseless
//! matching is the function it calls toCasefold; the annex's own data file is where the mappings
//! live, so this is the same relationship `pdf-spec` has with the Arlington tables and
//! `pdf-font` with `data/cmaps`. ADR 1086 has the argument for generating rather than parsing:
//! `CLAUDE.md` principle 2 forbids parsed data on the launch path, and a table the compiler has
//! already laid out costs a binary search and no parse at all.
//!
//! **Full case folding, which is the file's status C plus status F**, is what the data file's
//! own usage note calls option B. Status S is the simple folding, which exists for
//! implementations that cannot let a string grow, and taking it would reintroduce exactly the
//! defect this table exists to remove. Status T is the Turkic pair, which the same note says is
//! excluded by default; `src/case.rs` states why that default is the right one for a folder name.
//!
//! # Design rules
//!
//! **Nothing is silently dropped**, which is `pdf-spec/build.rs`'s rule and is load-bearing here
//! for a different reason: a status this generator did not recognise would silently narrow the
//! table, and a narrower table answers *no collision* — the safe-looking answer that is wrong.
//! So an unknown status, a malformed code point or an out-of-order file aborts the build.

// A build script's job is to abort the build when its input is malformed, and the panic message
// is the diagnostic a developer reads — the same argument `pdf-spec/build.rs` makes.
#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "aborting the build is the intended and only useful failure mode here"
)]

use std::fmt::Write as _;
use std::path::PathBuf;

fn main() {
    let manifest = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").expect("cargo sets this"));
    let data = manifest.join("../../data/unicode/CaseFolding.txt");
    println!("cargo::rerun-if-changed={}", data.display());
    println!("cargo::rerun-if-changed=build.rs");

    let text = std::fs::read_to_string(&data)
        .unwrap_or_else(|error| panic!("{}: {error}", data.display()));
    let table = fold_table(&text);

    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("cargo sets this"));
    std::fs::write(out.join("case_folding.rs"), table).expect("writing the generated table");
}

/// The generated source: one sorted table of the mappings with status C or F.
fn fold_table(text: &str) -> String {
    let mut rows: Vec<(u32, String)> = Vec::new();
    for line in text.lines() {
        // The file documents its own format as `<code>; <status>; <mapping>; # <name>`, with
        // comments introduced by `#` and blank lines between the blocks.
        let line = line.split('#').next().unwrap_or_default().trim();
        if line.is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split(';').map(str::trim).collect();
        // Four fields: the trailing `;` leaves an empty one, which is the file's own shape.
        assert!(
            fields.len() >= 3,
            "CaseFolding.txt: {line} is not <code>; <status>; <mapping>;"
        );
        let status = fields[1];
        match status {
            // C is shared by the simple and the full folding, F is what makes a string grow, and
            // the two together are the full folding this table is.
            "C" | "F" => {}
            // S is the simple folding's substitute for an F row, and T the Turkic pair. Both are
            // recognised and skipped, so that a status this file grows later fails the build
            // rather than narrowing the table in silence.
            "S" | "T" => continue,
            other => panic!("CaseFolding.txt: unknown status {other} on {line}"),
        }
        let code = code_point(fields[0]);
        let folded: String = fields[2]
            .split_whitespace()
            .map(|point| {
                char::from_u32(code_point(point))
                    .unwrap_or_else(|| panic!("CaseFolding.txt: {point} is not a scalar value"))
            })
            .collect();
        assert!(
            !folded.is_empty(),
            "CaseFolding.txt: {line} folds to nothing"
        );
        rows.push((code, folded));
    }

    // Sorted and unique, because the lookup is a binary search and because a code point carrying
    // both a C and an F row would mean the file's two statuses had stopped being exclusive.
    rows.sort_by_key(|(code, _)| *code);
    for pair in rows.windows(2) {
        assert_ne!(
            pair[0].0, pair[1].0,
            "CaseFolding.txt: two full-folding rows for U+{:04X}",
            pair[0].0
        );
    }
    assert!(
        rows.len() > 1_000,
        "CaseFolding.txt: {} rows is too few to be the whole file",
        rows.len()
    );

    // The file names its own version on its first line, which is the only place the version
    // appears in it: `# CaseFolding-17.0.0.txt`.
    let version = text
        .lines()
        .next()
        .unwrap_or_default()
        .trim_start_matches('#')
        .trim();
    let mut source = format!(
        "/// Every character whose full case folding is not itself, sorted by code point.\n\
         ///\n\
         /// Generated by `build.rs` from `data/unicode/CaseFolding.txt` ({version}): the rows\n\
         /// with status C or F, which is that file's own definition of a full case folding.\n\
         static FOLDING: &[(char, &str)] = &[\n"
    );
    for (code, folded) in &rows {
        let character = char::from_u32(*code)
            .unwrap_or_else(|| panic!("CaseFolding.txt: U+{code:04X} is not a scalar value"));
        let _ = writeln!(
            source,
            "    ('{}', \"{}\"),",
            escaped(character),
            folded.chars().map(escaped).collect::<String>()
        );
    }
    source.push_str("];\n");
    source
}

/// One `CaseFolding.txt` field as a code point.
fn code_point(field: &str) -> u32 {
    u32::from_str_radix(field, 16)
        .unwrap_or_else(|error| panic!("CaseFolding.txt: {field} is not a code point: {error}"))
}

/// One character as a Rust escape, so that the generated file is plain ASCII whatever it holds.
fn escaped(character: char) -> String {
    format!("\\u{{{:x}}}", character as u32)
}
