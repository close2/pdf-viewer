//! ISO 32000-2 §7.6 on the way out: a whole file written under the standard security handler.
//!
//! # What a round trip is worth here, and what it is not
//!
//! Writing a file and reading it back with the same crate is self-consistency, and on its own
//! it would prove only that two halves of one module agree. Three things make these tests more
//! than that.
//!
//! **The two halves are not one algorithm run backwards.** §7.6.4.4.7's Algorithm 8 and
//! §7.6.4.4.8's Algorithm 9 compute `/U`, `/UE`, `/O` and `/OE`; what reads them is
//! §7.6.4.4.10's Algorithm 11, §7.6.4.4.11's Algorithm 12 and §7.6.4.3.3's Algorithm 2.A steps
//! (d) and (e) — code this crate has run against real producers' files since long before it
//! could write one (`encryption.rs` is that suite). A reader that already opens seven documents
//! no code here encrypted is a witness the writer is being checked against.
//!
//! **The clause states what the bytes are, and that is asserted directly.** `/U` and `/O` are
//! 48 bytes laid out in three named sections, `/UE` and `/OE` are 32, and the salts in the
//! entries have to be the bytes the randomness source handed over. None of that is a round
//! trip.
//!
//! **The `/Perms` test is a tamper, not a read-back.** §7.6.4.3.3 step (f) makes the decrypted
//! block authoritative over the plaintext `/P` beside it, so the discriminating question is
//! what a reader says when the two disagree — and a `/Perms` this writer got wrong would not
//! survive it. Asking a correctly written file what its permissions are would be answered by
//! `/P` alone (trap 27).
//!
//! # The randomness is an argument
//!
//! §7.6.4.4.7 step (a) wants "16 random bytes of data using a strong random number generator"
//! and §7.6.3.3 a fresh initialisation vector per string and stream. Every one of those bytes
//! enters through `Entropy`, so these tests supply a counter and the file they get is a
//! function of the plan — which is how a byte-for-byte determinism assertion is possible at all
//! for an encrypted output.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "test code: a fixture that cannot exercise the rule must fail loudly rather than \
              pass by doing nothing"
)]

use std::fmt::Write as _;

use pdf_syntax::object::{Name, Object, ObjectId};
use pdf_syntax::serialize::{
    Access, Assembly, Entropy, Form, ObjectStreams, Options, Protection, SerializeError, serialize,
    serialize_encrypted,
};
use pdf_syntax::{Document, Limits, SyntaxError, Version};

/// A randomness source that counts, so that a written file is a function of its plan.
///
/// Not a cipher and not a substitute for one: it stands where the platform's source stands so
/// that a test can state the bytes it expects. `SystemEntropy` is what a program supplies.
struct Counter(u8);

impl Entropy for Counter {
    fn fill(&mut self, out: &mut [u8]) -> bool {
        for byte in out.iter_mut() {
            *byte = self.0;
            self.0 = self.0.wrapping_add(1);
        }
        true
    }
}

/// A randomness source that has none.
struct Refuses;

impl Entropy for Refuses {
    fn fill(&mut self, _: &mut [u8]) -> bool {
        false
    }
}

/// Assembles a small file out of object bodies, with a §7.5.4 classic cross-reference table.
fn file_of(bodies: &[&str]) -> Vec<u8> {
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, body) in bodies.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R /ID [<0102> <0304>] >>\nstartxref\n{xref_at}\n\
         %%EOF\n"
    );
    out.into_bytes()
}

/// The plaintext the tests protect: a page, a content stream, a string and a metadata stream.
fn source_bytes() -> Vec<u8> {
    file_of(&[
        "<< /Type /Catalog /Pages 2 0 R /Metadata 5 0 R >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Contents 4 0 R \
         /Resources << /ProcSet [/PDF] >> /Annots [6 0 R] >>",
        "<< /Length 26 >>\nstream\n0 0 1 rg 10 10 50 50 re f\nendstream",
        "<< /Type /Metadata /Subtype /XML /Length 11 >>\nstream\n<x:xmpmeta/>\nendstream",
        "<< /Type /Annot /Subtype /Text /Rect [0 0 10 10] /Contents (a secret note) >>",
    ])
}

/// How many objects [`source_bytes`] holds.
const OBJECTS: u32 = 6;

/// Opens bytes without a password, failing loudly.
fn open(bytes: Vec<u8>) -> Document {
    Document::open_with_limits(bytes, Limits::DEFAULT).expect("the fixture opens")
}

/// Copies every object of a document into an assembly, keeping its catalog as the root.
fn copy_whole(assembly: &mut Assembly<'_>, highest: u32) {
    for number in 1..=highest {
        assembly
            .copy(0, ObjectId::new(number, 0))
            .expect("the assembly takes the object");
    }
    assembly.set_root(ObjectId::new(1, 0));
}

/// Writes `source` whole, encrypted as `protection` asks, from a counting randomness source.
fn protect(source: &Document, protection: &Protection<'_>, options: Options) -> Vec<u8> {
    let mut assembly = Assembly::new(vec![source]);
    copy_whole(&mut assembly, OBJECTS);
    let mut out = Vec::new();
    let mut entropy = Counter(0);
    serialize_encrypted(
        &assembly,
        Version { major: 1, minor: 7 },
        options,
        protection,
        &mut entropy,
        &mut out,
    )
    .expect("the file is written");
    out
}

/// `protect` with the defaults every test but the object-stream one wants.
fn protect_simple(source: &Document, user: &str, owner: &str) -> Vec<u8> {
    protect(
        source,
        &Protection {
            user_password: user,
            owner_password: owner,
            access: Access::ALL,
            encrypt_metadata: true,
        },
        Options::new(Form::Table),
    )
}

/// The `/Encrypt` dictionary of a written file, read back through this tree's own reader.
fn encrypt_dictionary(document: &Document) -> pdf_syntax::Dictionary {
    let reference = document
        .trailer()
        .get("Encrypt")
        .and_then(Object::as_reference)
        .expect("§7.6.2: an encrypted file's trailer names its encryption dictionary");
    document
        .get(reference)
        .as_dict()
        .cloned()
        .expect("the entry reaches a dictionary")
}

/// One byte string entry of a dictionary.
fn string_of(dict: &pdf_syntax::Dictionary, key: &str) -> Vec<u8> {
    dict.get(key)
        .and_then(Object::as_string)
        .unwrap_or_else(|| panic!("{key} is a byte string"))
        .to_vec()
}

/// Page one's content stream, decoded.
fn page_content(document: &Document) -> Vec<u8> {
    let page = document
        .get(ObjectId::new(3, 0))
        .as_dict()
        .cloned()
        .expect("object 3 is the page");
    let contents = page
        .get("Contents")
        .and_then(Object::as_reference)
        .expect("the page names its content stream");
    let object = document.get(contents);
    let stream = object.as_stream().expect("it is a stream");
    document
        .decoded_stream_data(stream)
        .expect("the content stream decodes")
        .to_vec()
}

/// The two passwords open the file, and everything inside it comes back as the source wrote it.
///
/// §7.6.4.1 fixes the two answers: the user password "should allow additional operations to be
/// performed according to the user access permissions", the owner's "should allow full (owner)
/// access". A wrong one is neither, and `Document::open_with_password` refuses it — the file
/// has a user password, so §7.6.4.1's first attempt with the padding string fails too.
#[test]
fn a_written_file_opens_on_either_password_and_refuses_a_third() {
    let source = open(source_bytes());
    let written = protect_simple(&source, "reader", "keeper");

    for password in ["reader", "keeper"] {
        let opened = Document::open_with_password(written.clone(), Limits::DEFAULT, password)
            .unwrap_or_else(|error| panic!("{password} opens the file: {error}"));
        assert!(opened.is_encrypted(), "the output states an /Encrypt");
        assert_eq!(
            page_content(&opened),
            b"0 0 1 rg 10 10 50 50 re f\n".to_vec(),
            "§7.6.3.3: the content stream comes back as the producer wrote it",
        );
        let annotation = opened.get(ObjectId::new(6, 0));
        let dict = annotation.as_dict().expect("object 6 is the annotation");
        assert_eq!(
            dict.get("Contents").and_then(Object::as_string),
            Some(&b"a secret note"[..]),
            "§7.6.2: a string is encrypted on the way out and decrypted on the way in",
        );
    }

    let refused = Document::open_with_password(written, Limits::DEFAULT, "neither");
    assert!(
        matches!(refused, Err(SyntaxError::PasswordRequired)),
        "a password that is neither is refused, not opened",
    );
}

/// The secret is not in the file, which is the whole point of writing it encrypted.
#[test]
fn the_plaintext_is_not_in_the_bytes() {
    let source = open(source_bytes());
    let written = protect_simple(&source, "reader", "keeper");
    for plaintext in [&b"a secret note"[..], &b"0 0 1 rg"[..]] {
        assert!(
            !written
                .windows(plaintext.len())
                .any(|window| window == plaintext),
            "§7.6.2 applies encryption to all strings and streams",
        );
    }
    // §7.6.2's first exception, which is also Table 15's requirement that an encrypted file's
    // `/ID` strings "shall be direct objects and shall be unencrypted".
    assert!(
        written.windows(3).any(|window| window == b"/ID"),
        "the trailer still states §14.4's identifier",
    );
}

/// §7.6.4.4.7 and §7.6.4.4.8 lay their entries out in three sections, and the salts are the
/// bytes the randomness source supplied.
///
/// Algorithm 8 step (a): "The 48-byte string consisting of the 32-byte hash followed by the
/// User Validation Salt followed by the User Key Salt is stored as the U key", and step (b)
/// makes `/UE` "[t]he resulting 32-byte string". Algorithm 9 says the same of `/O` and `/OE`.
/// The `Counter` hands out 0 to 31 for the file encryption key and 32 to 63 for the four
/// salts, in the order `crypt::revision6` consumes them.
#[test]
fn the_entries_are_the_three_sections_the_algorithms_name() {
    let source = open(source_bytes());
    let written = protect_simple(&source, "reader", "keeper");
    let opened =
        Document::open_with_password(written, Limits::DEFAULT, "reader").expect("the file opens");
    let dict = encrypt_dictionary(&opened);

    assert_eq!(dict.get("V").and_then(Object::as_integer), Some(5));
    assert_eq!(dict.get("R").and_then(Object::as_integer), Some(6));

    let user = string_of(&dict, "U");
    let owner = string_of(&dict, "O");
    assert_eq!(user.len(), 48, "Table 21: /U is 48 bytes long when /R is 6");
    assert_eq!(
        owner.len(),
        48,
        "Table 21: /O is 48 bytes long when /R is 6"
    );
    assert_eq!(
        string_of(&dict, "UE").len(),
        32,
        "Table 21: /UE is a 32-byte string",
    );
    assert_eq!(
        string_of(&dict, "OE").len(),
        32,
        "Table 21: /OE is a 32-byte string",
    );
    assert_eq!(
        string_of(&dict, "Perms").len(),
        16,
        "Table 21: /Perms is a 16-byte string",
    );

    let salts: Vec<u8> = (32u8..64).collect();
    assert_eq!(
        user.get(32..48),
        salts.get(..16),
        "Algorithm 8 step (a): the User Validation Salt then the User Key Salt",
    );
    assert_eq!(
        owner.get(32..48),
        salts.get(16..32),
        "Algorithm 9 step (a): the Owner Validation Salt then the Owner Key Salt",
    );
    assert_ne!(
        user.get(..32),
        owner.get(..32),
        "Algorithm 9's hash takes the 48-byte /U as well, so the two cannot coincide",
    );
}

/// §7.6.4.3.3 step (f) makes the decrypted `/Perms` authoritative, and a tampered `/P` proves
/// this writer wrote one that decrypts.
///
/// > Bytes 0-3 of the decrypted Perms entry, treated as a little-endian integer, are the user
/// > permissions. They shall match the value in the P key.
///
/// [`Access::ALL`] is Table 22's word with every permission granted, which is `-4`; the tamper
/// rewrites it as `-8`, clearing bit 3 — "Print the document". A reader that could not read the
/// block would report the tampered word, so the assertion below is about `/Perms` and nothing
/// else.
#[test]
fn a_tampered_p_loses_to_the_perms_block_this_writer_wrote() {
    let source = open(source_bytes());
    let written = protect_simple(&source, "", "keeper");
    assert_eq!(
        Access::ALL.flags(),
        -4,
        "Table 22 reserves every bit but 1 and 2, which are zero",
    );

    let at = written
        .windows(5)
        .position(|window| window == b"/P -4")
        .expect("the encryption dictionary states /P");
    let mut tampered = written.clone();
    tampered
        .get_mut(at..at.saturating_add(5))
        .expect("the window is in range")
        .copy_from_slice(b"/P -8");

    let opened = Document::open_with_password(tampered, Limits::DEFAULT, "")
        .expect("the default user password opens it");
    let permissions = opened.permissions().expect("an encrypted file has some");
    assert!(
        permissions.print,
        "§7.6.4.3.3 step (f): the decrypted /Perms outranks the /P written beside it",
    );
    assert_eq!(permissions.revision, 6, "Table 21: this writer writes /R 6");
}

/// Every unpredictable byte is an argument, so the same plan writes the same file.
///
/// RFC 0002 section 9's first layer, which an encrypted output would otherwise lose: §7.6.3.3
/// requires "a 16-byte random number" in front of every string and stream, so a writer reaching
/// for the platform's source produces a different file every time.
#[test]
fn the_same_plan_and_the_same_entropy_write_the_same_bytes() {
    let source = open(source_bytes());
    let first = protect_simple(&source, "reader", "keeper");
    let second = protect_simple(&source, "reader", "keeper");
    assert_eq!(first, second, "the write is a function of its inputs");

    let unprotected = {
        let mut assembly = Assembly::new(vec![&source]);
        copy_whole(&mut assembly, OBJECTS);
        let mut out = Vec::new();
        serialize(
            &assembly,
            Version { major: 1, minor: 7 },
            Options::new(Form::Table),
            &mut out,
        )
        .expect("the plaintext file is written");
        out
    };
    assert_ne!(first, unprotected, "one of the two is encrypted");
    assert!(
        first.starts_with(b"%PDF-2.0"),
        "Table 20 introduces /V 5 at PDF 2.0, so the header says so",
    );
    assert!(
        unprotected.starts_with(b"%PDF-1.7"),
        "an unencrypted file keeps the version its caller asked for",
    );
}

/// §7.5.7's list, as Errata Collection 3's Issue #439 amends it: an encrypted document's
/// catalog is not compressed, and §7.6.2's third exception covers the members of the streams
/// that are.
///
/// > Any strings that are inside streams such as content streams and compressed object
/// > streams, which themselves are encrypted
#[test]
fn an_object_stream_carries_the_encryption_and_the_catalog_stays_outside_one() {
    let source = open(source_bytes());
    let written = protect(
        &source,
        &Protection {
            user_password: "",
            owner_password: "keeper",
            access: Access::ALL,
            encrypt_metadata: true,
        },
        Options {
            form: Form::Stream,
            object_streams: ObjectStreams::DEFAULT,
            streams: pdf_syntax::Streams::Carry,
        },
    );
    let opened = Document::open_with_password(written, Limits::DEFAULT, "")
        .expect("the default user password opens it");

    // The catalog is reachable and is not a compressed object: `xref.location` answers
    // `Location::Offset` for an object at the outermost level and `Location::InStream` for one
    // inside a carrier.
    assert!(
        matches!(
            opened.xref().location(1),
            Some(pdf_syntax::Location::Offset(_))
        ),
        "Issue #439: an encrypted document's catalog shall not be in an object stream",
    );
    let annotation = opened.get(ObjectId::new(6, 0));
    let dict = annotation.as_dict().expect("object 6 is the annotation");
    assert_eq!(
        dict.get("Contents").and_then(Object::as_string),
        Some(&b"a secret note"[..]),
        "a compressed object's string is decrypted with the carrier, not on its own",
    );
    assert_eq!(
        page_content(&opened),
        b"0 0 1 rg 10 10 50 50 re f\n".to_vec(),
        "and a stream that is not compressible is encrypted on its own",
    );
}

/// Table 21's `/EncryptMetadata` false leaves §14.3.2's stream readable, and the writer counts
/// it rather than leaving the caller to find out.
#[test]
fn encrypt_metadata_false_leaves_the_metadata_stream_in_the_clear() {
    let source = open(source_bytes());
    let written = protect(
        &source,
        &Protection {
            user_password: "",
            owner_password: "keeper",
            access: Access::ALL,
            encrypt_metadata: false,
        },
        Options::new(Form::Table),
    );
    assert!(
        written.windows(12).any(|window| window == b"<x:xmpmeta/>"),
        "Table 21: /EncryptMetadata false leaves the document-level metadata unencrypted",
    );
    let opened = Document::open_with_password(written, Limits::DEFAULT, "")
        .expect("the default user password opens it");
    let dict = encrypt_dictionary(&opened);
    assert_eq!(
        dict.get("EncryptMetadata"),
        Some(&Object::Boolean(false)),
        "the entry is stated, because its default is true",
    );
}

/// §7.6.2's fourth exception, on the way out: a signature's `/Contents` is not encrypted.
///
/// A signature covers a byte range of the file it sits in, so a value the reader will not
/// decrypt must not be encrypted — the same rule `Document::encrypt_for_update` applies to
/// §7.5.6's append.
#[test]
fn a_signature_contents_string_is_left_alone() {
    let source = open(file_of(&[
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] >>",
        "<< /Type /Sig /ByteRange [0 0 0 0] /Contents <DEADBEEF> /Name (signer) >>",
        "<< /Type /Metadata /Subtype /XML /Length 11 >>\nstream\n<x:xmpmeta/>\nendstream",
        "<< /Type /Annot /Subtype /Text /Rect [0 0 10 10] /Contents (a secret note) >>",
    ]));
    let written = protect_simple(&source, "", "keeper");
    let opened = Document::open_with_password(written, Limits::DEFAULT, "")
        .expect("the default user password opens it");
    let signature = opened.get(ObjectId::new(4, 0));
    let dict = signature.as_dict().expect("object 4 is the signature");
    assert_eq!(
        dict.get("Contents").and_then(Object::as_string),
        Some(&[0xDE, 0xAD, 0xBE, 0xEF][..]),
        "§7.6.2: the hexadecimal /Contents of a signature dictionary is exempt",
    );
    assert_eq!(
        dict.get("Name").and_then(Object::as_string),
        Some(&b"signer"[..]),
        "and every other string in the same dictionary is not",
    );
}

/// A randomness source that refuses stops the write, because there is no weaker salt.
#[test]
fn a_refusing_randomness_source_refuses_the_file() {
    let source = open(source_bytes());
    let mut assembly = Assembly::new(vec![&source]);
    copy_whole(&mut assembly, OBJECTS);
    let mut out = Vec::new();
    let refused = serialize_encrypted(
        &assembly,
        Version { major: 1, minor: 7 },
        Options::new(Form::Table),
        &Protection::owner_only("keeper"),
        &mut Refuses,
        &mut out,
    );
    assert!(
        matches!(refused, Err(SerializeError::Entropy)),
        "§7.6.4.4.7 step (a) asks for a strong random number generator and gets none",
    );
    assert!(out.is_empty(), "nothing reached the sink");
}

/// Table 22's word is the caller's seven bits and the eleven §7.6.4.2 fixes.
///
/// Position 10 is the one worth its own line: Table 22 says "PDF readers shall ignore this bit
/// and PDF writers shall always set this bit to 1", so it is set whatever a caller asks.
#[test]
fn the_permission_word_states_the_bits_table_22_reserves() {
    let none = Access {
        print: false,
        modify: false,
        copy: false,
        annotate: false,
        fill_forms: false,
        assemble: false,
        print_faithfully: false,
    };
    let bits = none.flags().cast_unsigned();
    assert_eq!(bits & 0b11, 0, "positions 1 and 2: \"Must be zero (0)\"");
    assert_eq!(bits >> 6 & 0b11, 0b11, "positions 7 and 8: \"Must be 1\"");
    assert_eq!(bits >> 9 & 1, 1, "position 10: a writer always sets it");
    assert_eq!(bits >> 12, 0xF_FFFF, "positions 13 to 32: \"Must be 1\"");
    assert_eq!(
        none.flags(),
        -0x0D40,
        "and the seven a caller decides are all clear",
    );

    let printing = Access {
        print: true,
        ..none
    };
    assert_eq!(
        printing.flags() - none.flags(),
        1 << 2,
        "Table 22 numbers bits from 1, so position 3 is the third from the bottom",
    );
}

/// The one entry Table 21 makes conditional, written because §7.6.4.1 limits the revision to it.
#[test]
fn the_crypt_filter_is_the_one_the_clause_permits() {
    let source = open(source_bytes());
    let written = protect_simple(&source, "", "keeper");
    let opened = Document::open_with_password(written, Limits::DEFAULT, "")
        .expect("the default user password opens it");
    let dict = encrypt_dictionary(&opened);
    for entry in ["StmF", "StrF", "EFF"] {
        assert_eq!(
            dict.get(entry)
                .and_then(Object::as_name)
                .map(Name::as_bytes),
            Some(&b"StdCF"[..]),
            "Table 20: {entry} names a key of the /CF dictionary",
        );
    }
    let filters = dict
        .get("CF")
        .and_then(Object::as_dict)
        .expect("Table 20's /CF is a dictionary");
    let standard = filters
        .get("StdCF")
        .and_then(Object::as_dict)
        .expect("§7.6.4.1 limits a revision 6 handler to a filter named StdCF");
    assert_eq!(
        standard
            .get("CFM")
            .and_then(Object::as_name)
            .map(Name::as_bytes),
        Some(&b"AESV3"[..]),
        "§7.6.4.1: \"For revision 6, the filter CFM value shall be AESV3 (AES-256).\"",
    );
    assert_eq!(
        standard
            .get("AuthEvent")
            .and_then(Object::as_name)
            .map(Name::as_bytes),
        Some(&b"DocOpen"[..]),
        "and the same sentence limits it to an /AuthEvent of DocOpen",
    );
}
