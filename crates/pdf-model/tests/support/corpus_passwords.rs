//! The passwords the pdf.js corpus's encrypted documents open with, in the one place every
//! corpus gate reads them from.
//!
//! # Why a gate may hold a password
//!
//! ISO 32000-2 §7.6.4.1 has a reader try the default user password first and, where that fails,
//! says "the interactive PDF processor should prompt for a password". A gate has nobody to prompt,
//! so without this table each of these documents is a page no gate compares — the clause working,
//! and a page nobody is holding any renderer to. The passwords are not secrets: each is published
//! beside the file it opens, in the pdf.js issue or pull request the file is named after or in
//! pdf.js's own test manifest (`doc/pdf.js/test/test_manifest.json`'s `"password"` entries hold
//! seven of the ten) or in its unit tests, so supplying one answers the prompt the way every reader
//! of that issue can. What stays out is a document whose `/Encrypt` does not resolve at all,
//! `PDFBOX-4352-0.pdf`, which each gate names.
//!
//! # One place, keyed by file name
//!
//! `tests/corpus.rs`, `tests/oracle.rs`, `tests/raster_golden.rs` and `tests/save_round_trip.rs`
//! here read this table and nothing else, and so, through a `#[path]`, do `render-raster`'s
//! `tests/corpus.rs`, the five `pdf-transform` corpus walks (`pages_corpus`, `split_corpus`,
//! `optimize_corpus`, `writer_corpus`, `merge_corpus`), `pdf-vfs`'s `read_corpus` and
//! `write_corpus`, `pdf-script`'s `tests/script_corpus.rs`, and `pdf-syntax`'s `tests/encryption.rs`, which opens every row with its
//! password and refuses each without it. A password that stops opening its document stops every
//! gate at once rather than one. A `#[path]` rather than a crate of its own because the table is
//! one `const` and one lookup, and every reader is a test target that already has the file's
//! only dependency, the standard library. The names are the
//! pdf.js corpus's (`doc/pdf.js/test/pdfs/`); a gate walking another corpus asks for a name only
//! where the file is that corpus's.

/// One encrypted corpus document and the password it opens with.
pub(crate) struct CorpusPassword {
    /// The document's file name under `doc/pdf.js/test/pdfs/`.
    pub(crate) name: &'static str,
    /// The password as it is published, which is what §7.6.4.1's preprocessing is applied to.
    pub(crate) password: &'static str,
    /// The same password as the reference renderers are handed it.
    ///
    /// Equal to [`Self::password`] except where §7.6.4.3.3's `SASLprep` changes the string and
    /// the references do not implement the preprocessing. There they are handed its normalised
    /// form, which reaches the same hash — the same question in the spelling they understand,
    /// measured rather than assumed: `pdftoppm`, `mutool` and `gs` each refuse `saslprep-r6.pdf`'s
    /// published password and each open the file on `SaSLprep`. `save_round_trip.rs`'s
    /// `REFERENCE_PASSWORDS` made the same measurement first.
    pub(crate) for_the_references: &'static str,
}

/// Every encrypted pdf.js corpus document whose password is published, and where.
pub(crate) const CORPUS_PASSWORDS: [CorpusPassword; 10] = [
    // bug1782186 in pdf.js's manifest; `/V 4 /R 4`.
    CorpusPassword {
        name: "bug1782186.pdf",
        password: "Hello",
        for_the_references: "Hello",
    },
    // Not in the manifest: typed into pdf.js's own unit test of it (`test/unit/api_spec.js`, "gets
    // encrypted attachments in password-protected documents"), and the same password
    // `auth-event-ef-open.pdf`'s attachment takes in the test before it. `/V 5 /R 6`, and it
    // matches as the **owner** password, which §7.6.4.1 accepts as well as the user one.
    CorpusPassword {
        name: "encrypted-attachment.pdf",
        password: "000000",
        for_the_references: "000000",
    },
    // issue15893_reduced in the manifest; `/V 2 /R 3`.
    CorpusPassword {
        name: "issue15893_reduced.pdf",
        password: "test",
        for_the_references: "test",
    },
    // issue21579 in the manifest; `/R 5`, which this reader implements (ADR 0820).
    CorpusPassword {
        name: "issue21579.pdf",
        password: "p\u{E4}ssw\u{F6}rt",
        for_the_references: "p\u{E4}ssw\u{F6}rt",
    },
    // issue3371 in the manifest; `/V 4 /R 4`.
    CorpusPassword {
        name: "issue3371.pdf",
        password: "ELXRTQWS",
        for_the_references: "ELXRTQWS",
    },
    // issue6010_1 in the manifest; `/V 5 /R 6`.
    CorpusPassword {
        name: "issue6010_1.pdf",
        password: "abc",
        for_the_references: "abc",
    },
    // issue6010_2 in the manifest; `/R 6`, and `æøå` reaches the hash as UTF-8, which `SASLprep`
    // leaves unchanged.
    CorpusPassword {
        name: "issue6010_2.pdf",
        password: "\u{E6}\u{F8}\u{E5}",
        for_the_references: "\u{E6}\u{F8}\u{E5}",
    },
    // Not in the manifest: pdf.js pull request #6531's discussion records it, a user password on
    // a file with no owner password.
    CorpusPassword {
        name: "pr6531_1.pdf",
        password: "asdfasdf",
        for_the_references: "asdfasdf",
    },
    // Not in the manifest: typed into pdf.js's own browser test, the only place the file is used.
    // It is the file's **owner** password, which §7.6.4.1 accepts as well as the user one.
    CorpusPassword {
        name: "print_protection.pdf",
        password: "1234",
        for_the_references: "1234",
    },
    // saslprep-r6 in the manifest; the one password `SASLprep` changes — U+00AA normalises to `a`
    // and U+00AD maps to nothing.
    CorpusPassword {
        name: "saslprep-r6.pdf",
        password: "S\u{AA}SL\u{AD}prep",
        for_the_references: "SaSLprep",
    },
];

/// The published password of the pdf.js corpus document called `name`, if one is recorded.
pub(crate) fn corpus_password(name: &str) -> Option<&'static CorpusPassword> {
    CORPUS_PASSWORDS.iter().find(|known| known.name == name)
}
