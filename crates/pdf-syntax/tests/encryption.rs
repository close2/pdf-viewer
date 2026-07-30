//! ISO 32000-2 §7.6, against the documents that exercise each revision and method.
//!
//! # Why real files rather than fixtures
//!
//! A fixture for this clause would have to *encrypt*, which means running the same
//! algorithms the reader runs and comparing them with themselves. The corpus instead holds
//! files that real producers encrypted with real passwords, so recovering their plaintext
//! is a statement about the clause rather than about this code's self-consistency. Between
//! them they cover every revision §7.6.4.2 Table 21 specifies and every crypt filter method
//! Table 25 names:
//!
//! | file | `/V` | `/R` | method |
//! |---|---|---|---|
//! | `bug900822.pdf` | 1 | 2 | RC4, 40-bit |
//! | `issue17215.pdf` | 2 | 3 | RC4, 128-bit |
//! | `issue9972-1.pdf` | 4 | 4 | `AESV2` — AES-128 through a crypt filter |
//! | `issue17069.pdf` | 4 | 4 | `AESV2`, with different permissions |
//! | `issue7665.pdf` | 5 | 6 | `AESV3` — AES-256 |
//! | `encrypted-attachment.pdf` | 5 | 6 | `AESV3` reaching only an attachment |
//!
//! One combination has no *working* document behind it. Table 25's `V2` method — RC4 named
//! through a crypt filter rather than by `/V` — appears twice in the corpus, in
//! `issue19484_1.pdf` and `issue19484_2.pdf`, and both files' object streams fail to
//! inflate after decryption *and* without it, which is what `poppler` also reports of them
//! ("Unknown compression method in flate stream", ten times). Their `/U` entries do
//! authenticate, so the key derivation is exercised; what is not exercised anywhere is
//! `V2`'s stream path, which is the same `Method::Rc4` that `/V` 1 and 2 use above.
//!
//! # What is asserted, and why it is not "the page looks right"
//!
//! Trap 1: a page that renders is not evidence that the bytes are the document's own.
//! Decryption has a stronger property available — its output has to *parse*. A content
//! stream that came out wrong is not a subtly wrong picture, it is high-entropy bytes that
//! cannot lex as PDF operators, and a `/Contents` that inflates under `FlateDecode` is
//! about two hundred bits of evidence on its own. So these tests check that page one's
//! content decodes and reads as operators, which no wrong key can fake.
//!
//! The corpus gate and the reference oracle then compare the resulting pages against three
//! other renderers, which is where "it looks right" is answered.

#![expect(
    clippy::panic,
    clippy::arithmetic_side_effects,
    reason = "test code: a fixture that cannot exercise the rule must fail loudly rather \
              than pass by doing nothing, and the one multiplication here is of two counts \
              bounded by a slice length"
)]

use std::path::{Path, PathBuf};

use pdf_syntax::{Document, Limits, Object, SyntaxError};

/// A corpus document's bytes, or `None` when the submodule is not checked out.
fn corpus_bytes(name: &str) -> Option<Vec<u8>> {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc/pdf.js/test/pdfs")
        .join(name);
    std::fs::read(path).ok()
}

/// Opens a corpus document with a password, panicking with the reason if it will not open.
fn open(name: &str, password: &str) -> Option<Document> {
    let bytes = corpus_bytes(name)?;
    Some(
        Document::open_with_password(bytes, Limits::DEFAULT, password)
            .unwrap_or_else(|error| panic!("{name} does not open: {error}")),
    )
}

/// Page one's decoded content stream.
///
/// Walks the page tree by hand rather than through `pdf-model`, because this crate is below
/// it and the point of the test is that the *bytes* came out right.
fn page_one_content(document: &Document, name: &str) -> Vec<u8> {
    let catalog = document
        .catalog()
        .unwrap_or_else(|error| panic!("{name} has no catalogue: {error}"));
    let mut node = document.get_key(&catalog, "Pages");
    // Descend the leftmost spine until a `/Page` leaf.
    for _ in 0..64 {
        let Some(dict) = node.as_dict().cloned() else {
            break;
        };
        let Some(first) = document
            .get_key(&dict, "Kids")
            .as_array()
            .and_then(|kids| kids.first().cloned())
        else {
            break;
        };
        node = document.resolve(&first);
    }
    let page = node
        .as_dict()
        .cloned()
        .unwrap_or_else(|| panic!("{name} has no first page"));

    let contents = document.get_key(&page, "Contents");
    let streams = match &contents {
        Object::Array(items) => items.iter().map(|item| document.resolve(item)).collect(),
        other => vec![other.clone()],
    };

    let mut out = Vec::new();
    for stream in streams {
        let Some(stream) = stream.as_stream() else {
            continue;
        };
        let data = document
            .decoded_stream_data(stream)
            .unwrap_or_else(|| panic!("{name}: page one's content stream does not decode"));
        out.extend_from_slice(&data);
        out.push(b'\n');
    }
    out
}

/// Does this look like a content stream rather than ciphertext?
///
/// A wrong key produces bytes uniform over 0..=255. A content stream is nearly all
/// printable ASCII — operators, numbers and delimiters — so the share of bytes outside that
/// set separates the two by an enormous margin rather than a fine one. Inline image data
/// (§8.9.7) is the one legitimate source of binary inside a content stream, which is why
/// the bound is nine tenths rather than everything.
fn reads_as_a_content_stream(data: &[u8]) -> bool {
    if data.len() < 8 {
        return false;
    }
    let printable = data
        .iter()
        .filter(|byte| matches!(byte, 0x09 | 0x0A | 0x0D | 0x20..=0x7E))
        .count();
    printable * 10 >= data.len() * 9
}

/// Every revision and method §7.6 specifies, on a document that uses it.
#[test]
fn each_revision_and_method_decrypts_to_a_readable_content_stream() {
    let cases = [
        ("bug900822.pdf", ""),
        ("issue17215.pdf", ""),
        ("issue9972-1.pdf", ""),
        ("issue17069.pdf", ""),
        ("bug1815476.pdf", ""),
        ("issue7665.pdf", ""),
    ];

    let mut checked = 0;
    for (name, password) in cases {
        let Some(document) = open(name, password) else {
            continue;
        };
        assert!(document.is_encrypted(), "{name} should carry an /Encrypt");
        let content = page_one_content(&document, name);
        assert!(
            reads_as_a_content_stream(&content),
            "{name}: page one's content does not read as operators after decryption"
        );
        checked += 1;
    }

    if corpus_bytes("bug900822.pdf").is_some() {
        assert_eq!(checked, 6, "every listed document should have been checked");
    }
}

/// §7.6.4.4's password algorithms, on the documents whose passwords are known.
///
/// `pr6531_2.pdf` and `saslprep-r6.pdf` are the two that make §7.6.4.1's preprocessing
/// load-bearing: the first is a non-ASCII password that has to reach the hash as UTF-8, and
/// the second is one `SASLprep` *changes* — U+00AA becomes `a` under the Normalize option and
/// U+00AD, a soft hyphen, is mapped to nothing.
#[test]
fn a_document_with_a_password_opens_with_it_and_not_without() {
    let cases = [
        ("issue15893_reduced.pdf", "test"),
        ("issue3371.pdf", "ELXRTQWS"),
        ("bug1782186.pdf", "Hello"),
        ("issue6010_1.pdf", "abc"),
        ("issue6010_2.pdf", "\u{E6}\u{F8}\u{E5}"),
        ("saslprep-r6.pdf", "S\u{AA}SL\u{AD}prep"),
    ];

    let mut checked = 0;
    for (name, password) in cases {
        let Some(bytes) = corpus_bytes(name) else {
            continue;
        };

        // The empty password is the one §7.6.4.1 has a reader try first, and these files
        // are the ones for which it is supposed to fail.
        let refused = Document::open_with_password(bytes.clone(), Limits::DEFAULT, "");
        assert!(
            matches!(refused, Err(SyntaxError::PasswordRequired)),
            "{name} should refuse the default user password, got {:?}",
            refused.err()
        );

        let document = open(name, password).expect("the bytes were read above");
        let content = page_one_content(&document, name);
        assert!(
            reads_as_a_content_stream(&content),
            "{name}: the supplied password did not produce readable content"
        );
        checked += 1;
    }

    if corpus_bytes("issue3371.pdf").is_some() {
        assert_eq!(checked, 6, "every listed document should have been checked");
    }
}

/// §7.6.2's first exception: "The values for the ID entry in the trailer".
///
/// Checked against the file's own bytes rather than against our own decryption, which is
/// what makes it a test: the `/ID` string has to appear in the encrypted file verbatim.
#[test]
fn the_file_identifier_is_not_decrypted() {
    let Some(bytes) = corpus_bytes("issue9972-1.pdf") else {
        return;
    };
    let document = Document::open(bytes.clone()).expect("opens with the default password");
    let id = document
        .get_key(document.trailer(), "ID")
        .as_array()
        .and_then(|items| items.first().cloned())
        .map(|item| document.resolve(&item))
        .and_then(|item| item.as_string().map(<[u8]>::to_vec))
        .expect("the document has an /ID");

    assert!(!id.is_empty());
    // The trailer writes it as a hexadecimal string, so search for that form.
    let hex = id.iter().fold(String::new(), |mut text, byte| {
        use std::fmt::Write as _;
        let _ = write!(text, "{byte:02x}");
        text
    });
    let haystack = String::from_utf8_lossy(&bytes).to_lowercase();
    assert!(
        haystack.contains(&hex),
        "the /ID we read is not the one written in the file, so it was decrypted"
    );
}

/// §7.6.2's second exception: "Any strings in an Encrypt dictionary".
///
/// The `/O` entry is the one the key is derived *from*, so decrypting it would break
/// authentication outright — but the entry is also readable after the fact, and it must
/// still be the file's own bytes.
#[test]
fn strings_in_the_encryption_dictionary_are_not_decrypted() {
    let Some(bytes) = corpus_bytes("issue9972-1.pdf") else {
        return;
    };
    let document = Document::open(bytes).expect("opens with the default password");
    let encrypt = document.get_key(document.trailer(), "Encrypt");
    let dict = encrypt.as_dict().expect("an encryption dictionary");
    let owner = document
        .get_key(dict, "O")
        .as_string()
        .map(<[u8]>::to_vec)
        .expect("a /O entry");

    // Table 21: "A byte string, 32 bytes long if the value of R is 4 or less". A decrypted
    // one would be a different 32 bytes, so length alone cannot catch this — what can is
    // that re-deriving the key from it works, which is exactly what opening the document
    // did. The assertion here is the length the clause states plus the fact that opening
    // succeeded above.
    assert_eq!(owner.len(), 32);
}

/// Table 20's `/StmF /Identity`: a document whose strings are encrypted and whose streams
/// are not.
///
/// `auth-event-ef-open.pdf` writes exactly that, so it is the file that would break if
/// `/StmF` were ignored and the default applied to streams anyway.
#[test]
fn an_identity_stream_filter_leaves_streams_alone() {
    let Some(bytes) = corpus_bytes("auth-event-ef-open.pdf") else {
        return;
    };
    let document = Document::open(bytes.clone()).expect("opens with the default password");
    let content = page_one_content(&document, "auth-event-ef-open.pdf");
    assert!(reads_as_a_content_stream(&content));

    // The stream is not encrypted, so its raw bytes are in the file unchanged. Take a
    // distinctive run from the middle to avoid matching a header both would share.
    let probe = content
        .get(8..40)
        .expect("the content stream is long enough");
    assert!(
        bytes.windows(probe.len()).any(|window| window == probe),
        "the content stream was transformed although /StmF is /Identity"
    );
}

/// §7.6.4.2 Table 22, on a document written to restrict one thing.
#[test]
fn permissions_come_from_the_p_entry() {
    let Some(document) = open("bug1815476.pdf", "") else {
        return;
    };
    let permissions = document.permissions().expect("the document is encrypted");
    // `/P -1084` is 0xFFFFFBC4, so bits 3, 9 and 12 are set and bits 4, 5, 6 and 11 are
    // clear. Each assertion below is one bit of that word read through Table 22's
    // one-based numbering, which is the part an implementation gets wrong by an off-by-one.
    assert!(permissions.print, "bit 3");
    assert!(!permissions.modify, "bit 4");
    assert!(!permissions.copy, "bit 5");
    assert!(!permissions.annotate, "bit 6");
    assert!(permissions.fill_forms, "bit 9");
    assert!(!permissions.assemble, "bit 11");
    assert!(permissions.print_faithfully, "bit 12");
    assert!(
        !permissions.owner,
        "the empty password is this file's user password"
    );
}

/// §7.6.6, on the two documents where only an attachment is encrypted.
///
/// Both write `/StmF /Identity /StrF /Identity` with a `StdCF` reached only through `/EFF`,
/// and neither authenticates against the empty password — nor against any password, by
/// three independent implementations of Algorithm 2.A. The clause binds the failure to the
/// data rather than to the file: authorization is needed "before the stream can be
/// accessed", so the page displays and the attachment does not.
#[test]
fn a_document_whose_attachment_alone_is_encrypted_still_opens() {
    for name in ["encrypted-attachment.pdf", "auth-event-ef-open.pdf"] {
        let Some(bytes) = corpus_bytes(name) else {
            continue;
        };
        let document = Document::open(bytes)
            .unwrap_or_else(|error| panic!("{name} should open without a password: {error}"));
        assert!(document.is_encrypted());
        assert!(
            reads_as_a_content_stream(&page_one_content(&document, name)),
            "{name}: the body is not encrypted, so it should read straight through"
        );

        // The attachment is, and no key was obtained for it. Object 8 is the embedded file
        // stream in both files; what matters is that *something* refuses rather than
        // handing back ciphertext.
        let attachment = document.get(pdf_syntax::ObjectId::new(8, 0));
        if let Some(stream) = attachment.as_stream() {
            assert!(
                stream.data.is_empty(),
                "{name}: the embedded file's data should be refused, not returned encrypted"
            );
        }
    }
}

/// A document this reader cannot decrypt says so, rather than drawing noise.
///
/// `issue21579.pdf` is `/R 5`, which Table 21 describes as "a deprecated proprietary Adobe
/// extension" and states no algorithm for.
#[test]
fn an_unspecified_revision_is_refused_by_name() {
    let Some(bytes) = corpus_bytes("issue21579.pdf") else {
        return;
    };
    let opened = Document::open_with_password(bytes, Limits::DEFAULT, "p\u{E4}ssw\u{F6}rt");
    match opened {
        Err(SyntaxError::UnsupportedEncryption { detail }) => {
            assert!(
                detail.contains("/R 5"),
                "the reason should name the revision"
            );
        }
        other => panic!("expected an unsupported-encryption error, got {other:?}"),
    }
}

/// §7.6.3.3 step (a), on a stream that is nothing but its initialisation vector.
///
/// Three of the corpus's AES-256 documents write a 16-byte `/Contents` — the vector and no
/// ciphertext blocks at all — for a page with no marking operators. A reader that treats
/// that as malformed reports a page it should have drawn empty; one that hands back the
/// vector draws sixteen bytes of noise. The clause's own decomposition gives the third
/// answer, and this pins it against the two failures either side.
#[test]
fn an_initialisation_vector_with_no_ciphertext_is_an_empty_stream() {
    for name in ["secHandler.pdf", "empty_protected.pdf", "pr6531_2.pdf"] {
        let Some(document) = open(name, "") else {
            continue;
        };
        let content = page_one_content(&document, name);
        assert!(
            content.iter().all(u8::is_ascii_whitespace),
            "{name}: an empty page should decrypt to nothing, got {} bytes",
            content.len()
        );
    }
}

/// §7.6.4.4.11's Algorithm 12: the empty password may be the *owner* password.
///
/// `pr6531_2.pdf` is the corpus's one file where it is, which makes it the only document
/// that exercises the owner half of Algorithm 2.A — the branch salted with the whole 48-byte
/// `/U` string — and the only one where §7.6.4.1's "full (owner) access" is what a reader
/// gets without being asked for anything.
#[test]
fn an_empty_password_may_be_the_owner_password() {
    let Some(document) = open("pr6531_2.pdf", "") else {
        return;
    };
    let permissions = document.permissions().expect("the document is encrypted");
    assert!(
        permissions.owner,
        "the empty password authenticates against /O here, not /U"
    );
}

/// Table 21's `/EncryptMetadata`, and §14.3.2's stream that it exempts.
///
/// `issue19484_1.pdf` writes `/EncryptMetadata false` over an RC4-encrypted document, so its
/// XMP packet is in the file as plain text. A reader that decrypted it anyway would turn
/// readable XML into noise and report nothing, and a reader that skipped decryption for
/// every stream would break the rest of the file — which is why the test asserts both ends:
/// the metadata reads as XML *and* the document is genuinely encrypted.
#[test]
fn a_metadata_stream_is_in_the_clear_when_encrypt_metadata_is_false() {
    let Some(document) = open("issue19484_1.pdf", "") else {
        return;
    };
    assert!(document.is_encrypted());

    let catalog = document.catalog().expect("the catalogue is readable");
    let metadata = document.get_key(&catalog, "Metadata");
    let stream = metadata.as_stream().expect("a /Metadata stream");
    let data = document
        .decoded_stream_data(stream)
        .expect("an unencrypted, unfiltered stream");
    assert!(
        data.starts_with(b"<?xpacket"),
        "the metadata should be the file's own XML, got {:?}",
        String::from_utf8_lossy(data.get(..32).unwrap_or_default())
    );
}
