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
//! | `issue21579.pdf` | 5 | 5 | `AESV3` under the `ExtensionLevel` 3 extension |
//! | `issue7665.pdf` | 5 | 6 | `AESV3` — AES-256 |
//! | `encrypted-attachment.pdf` | 5 | 6 | `AESV3` reaching only an attachment |
//!
//! **One fixture is assembled here, and it does not break the paragraph above.** The
//! revision-5 tests at the end of this file build a document around `issue21579.pdf`'s own
//! `/U`, `/UE`, `/Perms` and page-one ciphertext — a real producer's bytes, unchanged — so what
//! is fabricated is the catalogue, the page tree and the cross-reference table, none of which
//! any cipher touches. That gives §7.6.4.2's newest revision a regression test the submodule
//! cannot take away while leaving the assertion exactly where it was: bytes this code did not
//! encrypt have to come back out as their producer's operators.
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
              than pass by doing nothing, and the arithmetic is on counts bounded by a slice \
              length or on hexadecimal digits this file's own constants supply"
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
///
/// Where the passwords come from: each is published in the pdf.js issue or pull request the
/// file is named after, which is worth saying because it is the only reason this test can
/// exist. `pr6531_1.pdf`'s is in pull request #6531's discussion, and the file is the one that
/// request was about — a document with a user password and **no** owner password, which pdf.js
/// was opening without asking for either. This covers all eight of the corpus's
/// password-protected documents. `print_protection.pdf`'s was the last one found and it is in
/// no issue at all: it is typed into pdf.js's own browser test,
/// `test/integration/viewer_spec.mjs`, which is the only place that file is used.
#[test]
fn a_document_with_a_password_opens_with_it_and_not_without() {
    let cases = [
        ("issue15893_reduced.pdf", "test"),
        ("issue3371.pdf", "ELXRTQWS"),
        ("bug1782186.pdf", "Hello"),
        ("issue6010_1.pdf", "abc"),
        ("issue6010_2.pdf", "\u{E6}\u{F8}\u{E5}"),
        ("saslprep-r6.pdf", "S\u{AA}SL\u{AD}prep"),
        ("pr6531_1.pdf", "asdfasdf"),
        ("print_protection.pdf", "1234"),
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
        assert_eq!(checked, 8, "every listed document should have been checked");
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
/// Both write `/StmF /Identity /StrF /Identity` with a `StdCF` the embedded file stream
/// reaches through its own `/Crypt` specifier, and name the same filter in Table 20's `/EFF`
/// beside it; and neither authenticates against the empty password — nor against any
/// password, by three independent implementations of Algorithm 2.A. The clause binds the
/// failure to the data rather than to the file: authorization is needed "before the stream
/// can be accessed", so the page displays and the attachment does not.
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

/// Table 20's `/EFF`, which decides an embedded file stream that names no filter of its own.
///
/// ISO 32000-2 §7.6.2, Table 20:
///
/// > The name of the crypt filter that shall be used when encrypting embedded file streams
/// > that do not have their own crypt filter specifier
///
/// and, for the case this test's second half pins:
///
/// > If this entry is not present, and the embedded file stream does not contain a crypt
/// > filter specifier, the stream shall be encrypted using the default stream crypt filter
/// > specified by StmF .
///
/// # Why the fixture is a corpus document with an entry deleted
///
/// The two files above state both routes to `StdCF` at once — the stream's own `/Crypt`
/// specifier *and* `/EFF` — so the reader's answer is the same whether or not it reads the
/// second, and no document in the corpus states `/EFF` alone. Blanking the specifier with
/// spaces leaves every byte offset in the file where the cross-reference table says it is,
/// so what is opened is a real producer's document with exactly one entry removed, and the
/// only thing that can decide its attachment is the entry this test is about.
///
/// Both halves of the sentence are asserted, because only the pair distinguishes reading
/// `/EFF` from refusing every attachment: with `/EFF` the stream takes `StdCF`, whose
/// AES-256 needs a key this document authenticates nobody for, so §7.6.6 refuses it; with
/// `/EFF` blanked as well the stream falls back to `/StmF`, which is `/Identity` here, and
/// the bytes come back exactly as the file wrote them.
#[test]
fn an_embedded_file_stream_with_no_crypt_specifier_takes_the_eff_filter() {
    let Some(original) = corpus_bytes("encrypted-attachment.pdf") else {
        return;
    };
    // The ciphertext object 8 holds, read straight out of the file so that the assertions
    // below compare against the document's own bytes rather than against a count.
    let ciphertext = {
        let at = find(&original, b"8 0 obj")
            + original[find(&original, b"8 0 obj")..]
                .windows(7)
                .position(|window| window == b"stream\n")
                .expect("object 8 is a stream")
            + "stream\n".len();
        original[at..at + 784].to_vec()
    };

    let without_specifier = blank(
        &original,
        b"/Filter [ /Crypt ]\n/DecodeParms [ <<\n/Name /StdCF\n>> ]\n",
    );
    let document = Document::open(without_specifier).expect("the body is not encrypted");
    let stream = document
        .get(pdf_syntax::ObjectId::new(8, 0))
        .as_stream()
        .expect("object 8 is the embedded file stream")
        .clone();
    assert!(
        stream.data.is_empty(),
        "/EFF names StdCF, whose key this document authenticates nobody for, so §7.6.6 \
         refuses the stream rather than handing back its ciphertext"
    );

    let without_either = blank(
        &blank(
            &original,
            b"/Filter [ /Crypt ]\n/DecodeParms [ <<\n/Name /StdCF\n>> ]\n",
        ),
        b"/EFF /StdCF\n",
    );
    let document = Document::open(without_either).expect("the body is not encrypted");
    let stream = document
        .get(pdf_syntax::ObjectId::new(8, 0))
        .as_stream()
        .expect("object 8 is the embedded file stream")
        .clone();
    assert_eq!(
        &*stream.data,
        &ciphertext[..],
        "with no /EFF the stream takes /StmF, which is /Identity, so its bytes pass through"
    );
}

/// The offset of `needle` in `haystack`, which the fixtures above require to exist.
fn find(haystack: &[u8], needle: &[u8]) -> usize {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
        .unwrap_or_else(|| {
            panic!(
                "the fixture should contain {}",
                String::from_utf8_lossy(needle)
            )
        })
}

/// Replaces one occurrence of `entry` with spaces, leaving every byte offset unchanged.
///
/// White space between a dictionary's entries is what §7.2.3 makes it, so a blanked entry is
/// a dictionary that never held it — and the cross-reference table still points at the
/// objects it did before.
fn blank(bytes: &[u8], entry: &[u8]) -> Vec<u8> {
    let at = find(bytes, entry);
    let mut out = bytes.to_vec();
    out[at..at + entry.len()].fill(b' ');
    out
}

/// A revision Table 21 does not list is refused by name, rather than drawing noise.
///
/// **Its other half used to be `/R 5`, and the eight-hundred-and-eighty-seventh session
/// implemented that instead** — the argument is ADR 0820 and the module comment of
/// `crypt.rs`. What is left here is the arm that survives: Table 21 lists 2, 3, 4, 5 and 6
/// and nothing else, and `/R` is a direct integer in the encryption dictionary, so one byte
/// of `issue21579.pdf` makes a `/R 7` out of it with every offset and every other entry
/// unchanged. That is a revision this clause does not define, exercised by a real document
/// rather than by a fragment.
///
/// **The assertion names the clause rather than the revision, and that is the whole of what
/// makes this a test** (eight-hundred-and-fifty-third session). Its deleted sibling asserted
/// `contains("/R 5")` and passed with the refusal removed, because `crypt_filters` declined
/// the same file for §7.6.4.1's method pairing, whose sentence begins "/R 5 with a crypt
/// filter method" and contains the substring too. A refusal is only *by name* if the name
/// distinguishes it from the other refusals the same document can reach.
#[test]
fn an_unspecified_revision_is_refused_by_name() {
    let Some(bytes) = corpus_bytes("issue21579.pdf") else {
        return;
    };
    let mut seven = bytes;
    let at = find(&seven, b"/R 5");
    seven[at + 3] = b'7';
    let opened = Document::open_with_password(seven, Limits::DEFAULT, "");
    match opened {
        Err(SyntaxError::UnsupportedEncryption { detail }) => {
            assert!(
                detail.contains("/R 7 is not a revision §7.6.4 defines"),
                "an undefined revision should be named as one: got {detail:?}"
            );
        }
        other => panic!("expected an unsupported-encryption error, got {other:?}"),
    }
}

/// A `/R 5` document naming a crypt filter method its own key length cannot take.
///
/// §7.6.4.1 states the pairing for revisions 4 and 6 and says nothing about 5, so revision
/// 5's side of it comes from Table 20's `/V` 5 entry — §7.6.3.3's Algorithm 1.A "with a file
/// encryption key length of 256 bits", which among Table 25's methods is `AESV3` alone. One
/// byte of `issue21579.pdf` turns its `/CFM /AESV3` into `/CFM /AESV2`, and the key the
/// extension's own algorithm derives is 32 bytes, which AES-128 cannot take. Refused by name
/// rather than handed to a cipher that would decline it several layers further down.
#[test]
fn a_revision_five_document_naming_an_aes_128_filter_is_refused() {
    let Some(mut bytes) = corpus_bytes("issue21579.pdf") else {
        return;
    };
    let at = find(&bytes, b"/CFM /AESV3");
    bytes[at + 10] = b'2';
    let opened = Document::open_with_password(bytes, Limits::DEFAULT, "p\u{E4}ssw\u{F6}rt");
    match opened {
        Err(SyntaxError::UnsupportedEncryption { detail }) => {
            assert!(
                detail.contains("/R 5 with a crypt filter method"),
                "the pairing should name itself: got {detail:?}"
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

/// §7.6.4.2 Table 22 bit 3, on the file the corpus keeps for exactly that.
///
/// `print_protection.pdf` is not a rendering test anywhere. pdf.js uses it in one browser test
/// — "Printing can be disallowed for some pdfs (bug 1978985)" — which types `1234`, waits for
/// the text layer and then asserts that the print button is *hidden*; the file is not in
/// `test_manifest.json`, so it never enters their reference-image set at all.
///
/// Two things are worth pinning here. Its `/P` is −3392, which clears bit 3, so a reader that
/// honours permissions may not print it. And **`1234` is its owner password**, not its user
/// password — which matters because §7.6.4.1 says authenticating that way "should allow full
/// (owner) access", so the restriction the file states is one this reader would be entitled to
/// ignore, and pdf.js enforces it anyway. Nothing here prints, so the question does not arise;
/// recording which password matched is what keeps it from arising silently later.
#[test]
fn an_owner_password_authenticates_and_the_print_bit_is_still_read() {
    let Some(document) = open("print_protection.pdf", "1234") else {
        return;
    };
    let permissions = document.permissions().expect("the document is encrypted");
    assert!(permissions.owner, "1234 matches /O here, not /U");
    assert!(!permissions.print, "/P -3392 clears bit 3");
    assert!(!permissions.print_faithfully, "and bit 12 with it");
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

// ---------------------------------------------------------------------------------------
// §7.6.4.2 Table 21's revision 5, and the fixture that does not need the corpus
// ---------------------------------------------------------------------------------------

/// `issue21579.pdf`'s `/U`, the 48 bytes Algorithm 2.A reads as a hash, a validation salt and
/// a key salt.
///
/// The five constants here are a real producer's, lifted verbatim out of the one revision-5
/// document `doc/pdf.js` holds, and the fixture below is assembled around them. That is what
/// keeps this file's opening argument intact while giving the clause a test the submodule
/// cannot take away: **the ciphertext was not produced by this code**, so recovering the
/// plaintext under it is a statement about the algorithm rather than about our
/// self-consistency. What the fixture supplies is the *container* — a catalogue, a page tree
/// and a cross-reference table — which no cipher touches.
const USER_ENTRY: &str = "1A535BA22F2DD1CF9F37CF7540FB3A2D0F82D9EA49DB2F1E1CFD5118952D3DCC\
                          34C814AD2CC29313002D79DE31DF86A0";

/// The same document's `/UE`: the file encryption key, wrapped under the user password.
const USER_ENCRYPTION: &str = "EE4139E2E22728C58B69A73CB2AEA4F0A8F759F895C5D1295751820432A238B6";

/// The same document's `/Perms` — §7.6.4.4.9's Algorithm 10 block, keyed by the file key.
const PERMS: &str = "CB21FA8DCEEDECEA128E0F81D834B379";

/// The same document's page-one content stream, AES-256 ciphertext with its initialisation
/// vector in front.
const CONTENT: &str = "C7DB7E278725358D2C49C2E3484E065C57C1E330102E61843D4122C6FBFAAAF6\
                       2B72D613B1D048882805E6C9CA6D82CE269138F7A09D517C45736167E16541E4\
                       44C976BA99CC2762A4615B507BFE2D1992E48B81A11B1E125147D4B1B57E6111\
                       7A0E37E5C9ADC29113557CD22468871A733DC200FB4739FEDD3BFE3C32E02FB9\
                       138D72145C7ADA1FDDBF363C0356D0AAEA672FA0A46470AD92BB99454A5A1F58";

/// What that stream decrypts to, which is the assertion no wrong key can satisfy.
const CONTENT_PLAINTEXT: &str = "(repro1a: AES-256 R5, password 'passwoert' with umlauts) Tj";

/// The user password of the document those constants came from.
const USER_PASSWORD: &str = "p\u{E4}ssw\u{F6}rt";

/// An owner password chosen here, and the `/O` and `/OE` that go with it.
///
/// The corpus's revision-5 document authenticates its owner half against nothing this tree
/// knows, so the owner branch of Algorithm 2.A — the one salted with the whole 48-byte `/U`
/// string — would be untested without these. They were computed **outside this tree**, by
/// Python's `hashlib` and `cryptography` running the extension's own steps, and they wrap the
/// *same* file encryption key `/UE` above wraps. So the owner branch is not checked against
/// itself: it is checked against the real ciphertext, which comes out readable only if the key
/// it reaches is the key the document was encrypted with.
const OWNER_PASSWORD: &str = "eigent\u{FC}mer";

/// `/O` for [`OWNER_PASSWORD`]: `SHA-256(password ‖ validation salt ‖ U₄₈)`, then the two salts.
const OWNER_ENTRY: &str = "37339979E2EE6D29992924840F9851C2C5DE0F106CD37200D4ABC619ACB7097E\
                           01020304050607081112131415161718";

/// `/OE` for [`OWNER_PASSWORD`]: the file key under `SHA-256(password ‖ key salt ‖ U₄₈)`.
const OWNER_ENCRYPTION: &str = "368A47EE4DDABB0D0B78025BE58C6F309D4DDCC4C8CD41FF7B719A113031A183";

/// Bytes from a hexadecimal constant, white space ignored.
fn hex(text: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(text.len() / 2);
    let mut high: Option<u8> = None;
    for byte in text.bytes().filter(|byte| !byte.is_ascii_whitespace()) {
        let nibble = match byte {
            b'0'..=b'9' => byte - b'0',
            b'A'..=b'F' => byte - b'A' + 10,
            b'a'..=b'f' => byte - b'a' + 10,
            other => panic!("{} is not a hexadecimal digit", other as char),
        };
        match high.take() {
            None => high = Some(nibble),
            Some(first) => out.push(first * 16 + nibble),
        }
    }
    assert!(
        high.is_none(),
        "a hexadecimal constant has an even number of digits"
    );
    out
}

/// A PDF string in §7.3.4.3's hexadecimal form, which is what an encryption dictionary's
/// entries are written as and what keeps arbitrary bytes out of the file's token stream.
fn hex_string(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2 + 2);
    out.push('<');
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(out, "{byte:02X}");
    }
    out.push('>');
    out
}

/// Assembles numbered objects into a file with a §7.5.4 cross-reference table.
///
/// Written out rather than reached for, because the point of the fixture is that nothing
/// about it is this crate's *reading* of a file: the offsets are computed from the bytes as
/// they are laid down, so `Document::open` takes the fast path a well-formed file takes and
/// never falls back to §C.4's rebuild — which would make the test pass for a document whose
/// table was wrong.
fn assemble(objects: &[Vec<u8>], trailer_entries: &str) -> Vec<u8> {
    let mut out: Vec<u8> = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }

    let table_at = out.len();
    let size = objects.len() + 1;
    out.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
    for offset in &offsets {
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!("trailer\n<< /Size {size} {trailer_entries} >>\nstartxref\n{table_at}\n%%EOF\n")
            .as_bytes(),
    );
    out
}

/// How one revision-5 fixture differs from the next.
struct FixtureFive {
    /// The plaintext `/P`, which Algorithm 13's block is entitled to overrule.
    flags: i32,
    /// Bytes of NUL appended to `/U` and `/O` beyond their 48 significant ones.
    ///
    /// 8 of the 41 revision-5 documents among the 90 535 in `doc/pdf.js`, `doc/corpora/` and
    /// `corpus-cache/` write both entries as **127** bytes —
    /// 48 significant followed by 79 zeros — and they are exactly the 8 whose catalogue
    /// declares `/Extensions << /ADBE << /BaseVersion /1.7 /ExtensionLevel 3 >> >>`, which is
    /// the extension this revision belongs to. Algorithm 2.A reads three fixed sections out of
    /// the front of each string and salts the owner branch with "the 48-byte U string", so the
    /// tail carries nothing; a reader that demanded a length of exactly 48 would refuse a
    /// fifth of the population.
    padding: usize,
}

/// A complete revision-5 document, built around a real producer's ciphertext.
fn revision_five_document(fixture: &FixtureFive) -> Vec<u8> {
    let pad = |entry: &str| -> String {
        let mut bytes = hex(entry);
        bytes.resize(48 + fixture.padding, 0);
        hex_string(&bytes)
    };

    let content = hex(CONTENT);
    let mut stream = format!("<< /Length {} >>\nstream\n", content.len()).into_bytes();
    stream.extend_from_slice(&content);
    stream.extend_from_slice(b"\nendstream");

    let flags = fixture.flags;
    let objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 460 200] \
           /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>"
            .to_vec(),
        stream,
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
        format!(
            "<< /Filter /Standard /V 5 /R 5 /Length 256 \
             /CF << /StdCF << /CFM /AESV3 /Length 32 /AuthEvent /DocOpen >> >> \
             /StmF /StdCF /StrF /StdCF \
             /O {} /U {} /OE {} /UE {} /Perms {} /P {flags} /EncryptMetadata true >>",
            pad(OWNER_ENTRY),
            pad(USER_ENTRY),
            hex_string(&hex(OWNER_ENCRYPTION)),
            hex_string(&hex(USER_ENCRYPTION)),
            hex_string(&hex(PERMS)),
        )
        .into_bytes(),
    ];

    assemble(&objects, "/Root 1 0 R /Encrypt 6 0 R")
}

/// The fixture's page one, decrypted, as text.
fn fixture_page_one(document: &Document) -> String {
    String::from_utf8_lossy(&page_one_content(document, "the revision-5 fixture")).into_owned()
}

/// §7.6.4.2 Table 21's revision 5, on a document assembled here rather than in the corpus.
///
/// Four things at once, because they are one algorithm: the user password authenticates
/// against `/U`'s hash under a single SHA-256, `/UE` unwraps to the file encryption key,
/// §7.6.3.3's Algorithm 1.A decrypts the page with it, and §7.6.4.1's default user password
/// is refused. The plaintext is the assertion — a wrong key gives bytes uniform over the whole
/// range, and this one has to come out as the producer's own operators.
#[test]
fn a_revision_five_document_opens_with_its_user_password() {
    let bytes = revision_five_document(&FixtureFive {
        flags: -1084,
        padding: 0,
    });

    let refused = Document::open_with_password(bytes.clone(), Limits::DEFAULT, "");
    assert!(
        matches!(refused, Err(SyntaxError::PasswordRequired)),
        "the empty password is not this document's, got {:?}",
        refused.err()
    );

    let document = Document::open_with_password(bytes, Limits::DEFAULT, USER_PASSWORD)
        .expect("the user password should authenticate at revision 5");
    let content = fixture_page_one(&document);
    assert!(
        content.contains(CONTENT_PLAINTEXT),
        "the page should decrypt to the producer's own operators, got {content:?}"
    );

    let permissions = document.permissions().expect("the document is encrypted");
    assert!(
        !permissions.owner,
        "the user password matched, not the owner's"
    );
    assert_eq!(
        permissions.revision, 5,
        "Table 21's /R travels with the flags"
    );
}

/// The owner half of Algorithm 2.A at revision 5 — the branch salted with the 48-byte `/U`.
///
/// §7.6.4.1: authenticating with the owner password "should allow full (owner) access", which
/// is what [`pdf_syntax::Permissions::owner`] carries. The key it reaches has to be the *same*
/// key the user branch reaches, and the page proves it: both unwrap to the file key the real
/// ciphertext was encrypted under.
#[test]
fn a_revision_five_owner_password_gives_owner_access() {
    let bytes = revision_five_document(&FixtureFive {
        flags: -1084,
        padding: 0,
    });

    let document = Document::open_with_password(bytes, Limits::DEFAULT, OWNER_PASSWORD)
        .expect("the owner password should authenticate at revision 5");
    assert!(
        document
            .permissions()
            .expect("the document is encrypted")
            .owner,
        "this password matches /O, not /U"
    );
    assert!(
        fixture_page_one(&document).contains(CONTENT_PLAINTEXT),
        "the owner branch should reach the same file encryption key"
    );
}

/// A password that is neither is a wrong password, not a broken file.
#[test]
fn a_revision_five_document_refuses_a_password_that_is_neither() {
    let bytes = revision_five_document(&FixtureFive {
        flags: -1084,
        padding: 0,
    });
    let refused = Document::open_with_password(bytes, Limits::DEFAULT, "not the password");
    assert!(
        matches!(refused, Err(SyntaxError::PasswordRequired)),
        "got {:?}",
        refused.err()
    );
}

/// §7.6.4.4.12's Algorithm 13 at revision 5: the encrypted block outranks the plaintext `/P`.
///
/// §7.6.4.3.3 step (f) says the decrypted bytes **are** the user permissions, and the fixture
/// puts that beyond doubt by writing a `/P` of −1 in the clear — every position granted — while
/// the block the producer encrypted holds −1084. A reader that skipped the block at revision 5
/// would report a document with no restrictions at all.
#[test]
fn a_revision_five_perms_block_outranks_the_plaintext_flags() {
    let bytes = revision_five_document(&FixtureFive {
        flags: -1,
        padding: 0,
    });
    let document = Document::open_with_password(bytes, Limits::DEFAULT, USER_PASSWORD)
        .expect("the user password should authenticate");
    let permissions = document.permissions().expect("the document is encrypted");

    // −1084 is 0xFFFFFBC4: positions 3, 9 and 12 set, positions 4, 5, 6 and 11 clear.
    assert!(permissions.print, "bit 3");
    assert!(!permissions.modify, "bit 4");
    assert!(!permissions.copy, "bit 5");
    assert!(!permissions.annotate, "bit 6");
    assert!(permissions.fill_forms, "bit 9");
    assert!(!permissions.assemble, "bit 11");
    assert!(permissions.print_faithfully, "bit 12");
}

/// The `/U` and `/O` shape eight of the corpus's revision-5 documents actually write.
///
/// See [`FixtureFive::padding`]: the `ExtensionLevel` 3 files pad both entries to 127 bytes with
/// NUL, and Algorithm 2.A's three sections and its "48-byte U string" are all in the front.
#[test]
fn a_revision_five_document_padded_to_127_bytes_opens() {
    let bytes = revision_five_document(&FixtureFive {
        flags: -1084,
        padding: 79,
    });
    let document = Document::open_with_password(bytes, Limits::DEFAULT, USER_PASSWORD)
        .expect("48 significant bytes followed by NUL is what the extension's files write");
    assert!(fixture_page_one(&document).contains(CONTENT_PLAINTEXT));

    // And the owner branch, whose salt is the 48-byte prefix rather than the whole string.
    let bytes = revision_five_document(&FixtureFive {
        flags: -1084,
        padding: 79,
    });
    let document = Document::open_with_password(bytes, Limits::DEFAULT, OWNER_PASSWORD)
        .expect("the owner hash is salted with /U's first 48 bytes");
    assert!(
        document
            .permissions()
            .expect("the document is encrypted")
            .owner
    );
}

/// And the corpus's own revision-5 document, which is where the constants above came from.
///
/// The fixture is what makes this clause testable without the submodule; this is what makes
/// the fixture honest. If the two ever disagree, the file is right and the fixture is wrong.
#[test]
fn the_corpus_revision_five_document_opens() {
    let Some(document) = open("issue21579.pdf", USER_PASSWORD) else {
        return;
    };
    let content = page_one_content(&document, "issue21579.pdf");
    assert!(
        reads_as_a_content_stream(&content),
        "/R 5 should decrypt to readable operators"
    );
    assert!(String::from_utf8_lossy(&content).contains(CONTENT_PLAINTEXT));

    let permissions = document.permissions().expect("the document is encrypted");
    assert_eq!(permissions.revision, 5);
    assert!(!permissions.owner, "pässwört is the user password here");
    assert!(!permissions.annotate, "/P -1084 clears bit 6");
}
