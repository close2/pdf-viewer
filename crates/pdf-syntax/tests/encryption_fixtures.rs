//! ISO 32000-2 §7.6, on documents this file encrypts by the clause's own algorithms.
//!
//! # Why these exist beside the corpus witnesses
//!
//! `tests/encryption.rs` decrypts files real producers encrypted, which is a statement about the
//! clause rather than about this crate's self-consistency — and it reads them from the optional
//! `doc/pdf.js` checkout, so on a machine without it every one of those tests returns having read
//! nothing. Eighteen ledger rows of §7.6 were held by them alone (ADR 1497).
//!
//! The documents here are encrypted by this file, step by step from the clause's own algorithms,
//! and never by `pdf_syntax`: Algorithm 2's key, Algorithms 3 to 5's `/O` and `/U`, Algorithm 1's
//! per-object key, Algorithms 2.B, 8, 9 and 10's revision 6 entries are each written below under
//! their clause number. So the reader under test meets the clause written a second time, by
//! somebody reading the same steps, and a document it opens is one the clause's writer would have
//! produced. What this cannot catch is a step both writings misread the same way — which is what
//! the producers' files in `tests/encryption.rs` remain for.
//!
//! Every document is the same seven objects: a catalogue, a page tree, a page whose content stream
//! is encrypted, a font, an information dictionary whose `/Title` is an encrypted string, the
//! encryption dictionary, and — for the crypt filter fixture — a second content stream. What is
//! asserted is the plaintext: a wrong key gives bytes uniform over the whole range, and these have
//! to come out as the operators and the title that were put in.

#![expect(
    clippy::panic,
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    reason = "test code: a fixture that cannot exercise the rule must fail loudly, and the \
              arithmetic and truncations are the clause's own — byte counters XORed into a key, \
              object numbers split into their low-order bytes, a remainder modulo 3"
)]

use aes::cipher::{BlockCipherEncrypt, BlockModeEncrypt, KeyInit, KeyIvInit, StreamCipher};
use aes::{Aes128, Aes256};
use md5::Md5;
use pdf_syntax::{Document, Limits, Object, SyntaxError};
use sha2::Digest;

/// The 32-byte padding string of §7.6.4.3.2 step (a), verbatim from the clause.
const PADDING: [u8; 32] = [
    0x28, 0xBF, 0x4E, 0x5E, 0x4E, 0x75, 0x8A, 0x41, 0x64, 0x00, 0x4E, 0x56, 0xFF, 0xFA, 0x01, 0x08,
    0x2E, 0x2E, 0x00, 0xB6, 0xD0, 0x68, 0x3E, 0x80, 0x2F, 0x0C, 0xA9, 0xFE, 0x64, 0x53, 0x69, 0x7A,
];

/// The page's operators, before encryption.
const CONTENT: &[u8] = b"BT /F1 12 Tf 72 100 Td (A page the clause encrypted) Tj ET";

/// The information dictionary's `/Title`, before encryption.
const TITLE: &[u8] = b"Encrypted by the clause's steps";

/// The second content stream of the crypt filter fixture, written in the clear.
const IN_THE_CLEAR: &[u8] = b"0 0 1 rg";

/// The first element of the file identifier, which Algorithms 2 and 5 hash.
const ID0: [u8; 16] = *b"fixture-id-first";

/// The user password every fixture is opened with.
const USER: &str = "user";

/// The owner password every fixture is opened with.
const OWNER: &str = "owner";

/// RC4 under `key`. The cipher is §7.6.3.1's and not the clause's to define; the key schedule is
/// the dependency's, and what this file writes is which key, over which bytes, how many times.
fn rc4(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut out = data.to_vec();
    let mut cipher = rc4::Rc4::new_from_slice(key).expect("RC4 takes 1 to 256 key bytes");
    cipher.apply_keystream(&mut out);
    out
}

/// MD5 over several pieces in order.
fn md5(pieces: &[&[u8]]) -> [u8; 16] {
    let mut hash = Md5::new();
    for piece in pieces {
        hash.update(piece);
    }
    hash.finalize().into()
}

/// §7.6.4.3.2 step (a): "Pad or truncate the resulting password string to exactly 32 bytes."
fn padded(password: &str) -> [u8; 32] {
    let bytes = password.as_bytes();
    let mut out = PADDING;
    let used = bytes.len().min(32);
    out[..used].copy_from_slice(&bytes[..used]);
    out[used..].copy_from_slice(&PADDING[..32 - used]);
    out
}

/// What differs between the revision-4-and-earlier fixtures.
#[derive(Clone, Copy)]
struct Older {
    /// Table 21's `/R`: 2, 3 or 4.
    revision: u8,
    /// The file encryption key's length in bytes — Algorithm 2's `n`.
    length: usize,
    /// Table 22's `/P`.
    flags: i32,
    /// Table 21's `/EncryptMetadata`, which Algorithm 2 step (f) hashes when it is false.
    encrypt_metadata: bool,
}

/// §7.6.4.4.2 Algorithm 3, steps (a) to (d): the RC4 key the owner password yields.
fn owner_key(owner: &str, older: Older) -> Vec<u8> {
    // Step (b): MD5 of the padded owner password.
    let mut digest = md5(&[&padded(owner)]);
    // Step (c): "(Security handlers of revision 3 or greater) Do the following 50 times".
    if older.revision >= 3 {
        for _ in 0..50 {
            digest = md5(&[&digest]);
        }
    }
    // Step (d): the first n bytes, n being 5 at revision 2.
    let n = if older.revision == 2 { 5 } else { older.length };
    digest[..n].to_vec()
}

/// §7.6.4.4.2 Algorithm 3: the encryption dictionary's `/O`.
fn owner_entry(owner: &str, user: &str, older: Older) -> Vec<u8> {
    let key = owner_key(owner, older);
    // Steps (e) and (f): RC4 of the padded user password.
    let mut value = rc4(&key, &padded(user));
    // Step (g): nineteen more, under the key XORed with the counter 1 to 19.
    if older.revision >= 3 {
        for counter in 1..=19u8 {
            let round: Vec<u8> = key.iter().map(|byte| byte ^ counter).collect();
            value = rc4(&round, &value);
        }
    }
    value
}

/// §7.6.4.3.2 Algorithm 2: the file encryption key a user password yields.
fn file_key(user: &str, owner_entry: &[u8], older: Older) -> Vec<u8> {
    let mut hash = Md5::new();
    // Steps (a) and (b).
    hash.update(padded(user));
    // Step (c): the `/O` entry.
    hash.update(owner_entry);
    // Step (d): `/P` as a 32-bit unsigned number, low-order byte first.
    hash.update(older.flags.to_le_bytes());
    // Step (e): the first element of the file identifier.
    hash.update(ID0);
    // Step (f): revision 4 or greater, metadata not encrypted.
    if older.revision >= 4 && !older.encrypt_metadata {
        hash.update([0xFF; 4]);
    }
    // Step (g).
    let mut digest: [u8; 16] = hash.finalize().into();
    let n = if older.revision == 2 { 5 } else { older.length };
    // Step (h): fifty more MD5s, each over the first n bytes of the last.
    if older.revision >= 3 {
        for _ in 0..50 {
            digest = md5(&[&digest[..n]]);
        }
    }
    // Step (i).
    digest[..n].to_vec()
}

/// §7.6.4.4.3 Algorithm 4 (revision 2) and §7.6.4.4.4 Algorithm 5 (revisions 3 and 4): `/U`.
fn user_entry(key: &[u8], older: Older) -> Vec<u8> {
    if older.revision == 2 {
        // Algorithm 4 step (b): the padding string under the file encryption key.
        return rc4(key, &PADDING);
    }
    // Algorithm 5 steps (b) and (c): MD5 of the padding string and the identifier.
    let digest = md5(&[&PADDING, &ID0]);
    // Step (d), then step (e)'s nineteen rounds under the key XORed with 1 to 19.
    let mut value = rc4(key, &digest);
    for counter in 1..=19u8 {
        let round: Vec<u8> = key.iter().map(|byte| byte ^ counter).collect();
        value = rc4(&round, &value);
    }
    // Step (f): "Append 16 bytes of arbitrary padding".
    value.extend_from_slice(&[0xA5; 16]);
    value
}

/// §7.6.3.2 Algorithm 1 steps (a) to (d): the key one object's strings and streams take.
fn object_key(file_key: &[u8], number: u32, aes: bool) -> Vec<u8> {
    let number = number.to_le_bytes();
    let mut extended = file_key.to_vec();
    // Step (b): the low-order 3 bytes of the object number and 2 of the generation, low first.
    extended.extend_from_slice(&number[..3]);
    extended.extend_from_slice(&[0, 0]);
    if aes {
        extended.extend_from_slice(b"sAlT");
    }
    // Steps (c) and (d): "the first (n + 5) bytes, up to a maximum of 16".
    let digest = md5(&[&extended]);
    digest[..(file_key.len() + 5).min(16)].to_vec()
}

/// AES in CBC mode over whole blocks, with no padding: the mode every algorithm here uses.
fn cbc(key: &[u8], iv: &[u8], data: &[u8]) -> Vec<u8> {
    assert!(
        data.len().is_multiple_of(16),
        "CBC without padding takes whole blocks"
    );
    let mut out = data.to_vec();
    let (blocks, _) = out.as_chunks_mut::<16>();
    match key.len() {
        16 => {
            let mut cipher =
                cbc::Encryptor::<Aes128>::new_from_slices(key, iv).expect("a 16-byte key");
            for block in blocks {
                cipher.encrypt_block(block.into());
            }
        }
        32 => {
            let mut cipher =
                cbc::Encryptor::<Aes256>::new_from_slices(key, iv).expect("a 32-byte key");
            for block in blocks {
                cipher.encrypt_block(block.into());
            }
        }
        other => panic!("no AES key is {other} bytes"),
    }
    out
}

/// AES in CBC mode with §7.6.3.1's padding, the initialisation vector stored first.
///
/// §7.6.3.1: "For an original message length of M, the pad shall consist of 16 - (M modulo 16)
/// bytes whose value shall also be 16 - (M modulo 16)."
fn aes_cbc(key: &[u8], iv: [u8; 16], data: &[u8]) -> Vec<u8> {
    let pad = 16 - data.len() % 16;
    let mut padded = data.to_vec();
    padded.resize(data.len() + pad, pad as u8);
    let mut out = iv.to_vec();
    out.extend_from_slice(&cbc(key, &iv, &padded));
    out
}

/// How one document's strings and streams are encrypted.
#[derive(Clone, Copy)]
enum Method {
    /// §7.6.3.2 Algorithm 1 with RC4.
    Rc4,
    /// §7.6.3.2 Algorithm 1 with AES-128 and the `sAlT` extension.
    Aes128,
    /// §7.6.3.3 Algorithm 1.A: AES-256 under the file encryption key itself.
    Aes256,
    /// The `Identity` crypt filter of Table 26: the data as it is.
    Identity,
}

/// Encrypts the data of object `number` under `method`.
fn encrypt(method: Method, file_key: &[u8], number: u32, data: &[u8]) -> Vec<u8> {
    // A fixed vector: §7.6.3.2 asks for a random one, and nothing a reader does may depend on it.
    let iv = [number as u8; 16];
    match method {
        Method::Rc4 => rc4(&object_key(file_key, number, false), data),
        Method::Aes128 => aes_cbc(&object_key(file_key, number, true), iv, data),
        Method::Aes256 => aes_cbc(file_key, iv, data),
        Method::Identity => data.to_vec(),
    }
}

/// A PDF hexadecimal string (§7.3.4.3).
fn hex_string(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut out = String::from("<");
    for byte in bytes {
        let _ = write!(out, "{byte:02X}");
    }
    out.push('>');
    out
}

/// A stream object's text around `data`, with `extra` dictionary entries.
fn stream(data: &[u8], extra: &str) -> Vec<u8> {
    let mut out = format!("<< /Length {} {extra} >>\nstream\n", data.len()).into_bytes();
    out.extend_from_slice(data);
    out.extend_from_slice(b"\nendstream");
    out
}

/// Assembles numbered objects into a file with a §7.5.4 cross-reference table whose offsets are
/// those of the bytes as laid down, so the reader takes a well-formed file's path.
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

/// The seven objects every fixture has, the encryption dictionary's text given.
///
/// `streams` encrypts the content stream (object 4), `strings` the `/Title` (object 6). With a
/// `crypt_filter`, object 8 is a second content stream in the clear whose `/Filter` array opens
/// with §7.4.10's `Crypt` naming `Identity`.
fn document(
    encrypt_dictionary: &str,
    file_key: &[u8],
    streams: Method,
    strings: Method,
    crypt_filter: bool,
) -> Vec<u8> {
    let contents = if crypt_filter {
        "[4 0 R 8 0 R]"
    } else {
        "4 0 R"
    };
    let mut objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 200] \
             /Resources << /Font << /F1 5 0 R >> >> /Contents {contents} >>"
        )
        .into_bytes(),
        stream(&encrypt(streams, file_key, 4, CONTENT), ""),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
        format!(
            "<< /Title {} >>",
            hex_string(&encrypt(strings, file_key, 6, TITLE))
        )
        .into_bytes(),
        encrypt_dictionary.as_bytes().to_vec(),
    ];
    if crypt_filter {
        objects.push(stream(
            IN_THE_CLEAR,
            "/Filter [/Crypt] /DecodeParms [<< /Type /CryptFilterDecodeParms /Name /Identity >>]",
        ));
    }
    assemble(
        &objects,
        &format!(
            "/Root 1 0 R /Info 6 0 R /Encrypt 7 0 R /ID [{} {}]",
            hex_string(&ID0),
            hex_string(b"fixture-id-other")
        ),
    )
}

/// A revision 2, 3 or 4 document, with what its encryption dictionary states.
struct OlderDocument {
    bytes: Vec<u8>,
    owner_entry: Vec<u8>,
}

/// A document under the standard security handler at revision 4 or earlier.
///
/// `crypt_filters` is the `/CF`, `/StmF` and `/StrF` text of a `/V` 4 dictionary, or empty.
fn older_document(
    older: Older,
    version: u8,
    crypt_filters: &str,
    methods: (Method, Method),
) -> OlderDocument {
    let owner_entry = owner_entry(OWNER, USER, older);
    let key = file_key(USER, &owner_entry, older);
    let user_entry = user_entry(&key, older);
    let dictionary = format!(
        "<< /Filter /Standard /V {version} /R {} /Length {} {crypt_filters} /O {} /U {} /P {} \
         /EncryptMetadata {} >>",
        older.revision,
        older.length * 8,
        hex_string(&owner_entry),
        hex_string(&user_entry),
        older.flags,
        older.encrypt_metadata,
    );
    OlderDocument {
        bytes: document(&dictionary, &key, methods.0, methods.1, false),
        owner_entry,
    }
}

/// Page one's content streams, decoded and joined by newlines.
fn page_content(document: &Document) -> Vec<u8> {
    let catalog = document.catalog().expect("a catalogue");
    let pages = document.get_key(&catalog, "Pages");
    let kids = document.get_key(pages.as_dict().expect("a page tree"), "Kids");
    let page = document.resolve(kids.as_array().and_then(<[Object]>::first).expect("a kid"));
    let contents = document.get_key(page.as_dict().expect("a page"), "Contents");
    let streams: Vec<Object> = match &contents {
        Object::Array(items) => items.iter().map(|item| document.resolve(item)).collect(),
        other => vec![other.clone()],
    };
    let mut out = Vec::new();
    for (index, item) in streams.iter().enumerate() {
        if index > 0 {
            out.push(b'\n');
        }
        let stream = item.as_stream().expect("a content stream");
        out.extend_from_slice(&document.decoded_stream_data(stream).expect("it decodes"));
    }
    out
}

/// The information dictionary's `/Title`, as the reader decrypted it.
fn title(document: &Document) -> Vec<u8> {
    let info = document.get_key(document.trailer(), "Info");
    document
        .get_key(info.as_dict().expect("an information dictionary"), "Title")
        .as_string()
        .expect("a /Title string")
        .to_vec()
}

/// Opens `bytes` with `password`, or says why not.
fn open(bytes: &[u8], password: &str) -> Result<Document, SyntaxError> {
    Document::open_with_password(bytes.to_vec(), Limits::DEFAULT, password)
}

/// Asserts what every fixture owes: §7.6.4.1's empty default refused, the user and owner
/// passwords each authenticated as what they are, a third refused, and the plaintext back.
fn opens_as_the_clause_says(bytes: &[u8], revision: u8) {
    assert!(
        matches!(open(bytes, ""), Err(SyntaxError::PasswordRequired)),
        "the empty default password is not this document's"
    );
    assert!(
        matches!(open(bytes, "neither"), Err(SyntaxError::PasswordRequired)),
        "a password that is neither is refused"
    );
    for (password, owner) in [(USER, false), (OWNER, true)] {
        let document = open(bytes, password).unwrap_or_else(|error| {
            panic!("{password:?} does not open revision {revision}: {error}")
        });
        let permissions = document.permissions().expect("the document is encrypted");
        assert_eq!(
            permissions.owner, owner,
            "{password:?} authenticates as what it is"
        );
        assert_eq!(permissions.revision, revision);
        assert_eq!(
            page_content(&document),
            CONTENT,
            "{password:?}: the page decrypts"
        );
        assert_eq!(title(&document), TITLE, "{password:?}: the string decrypts");
    }
}

const REVISION_2: Older = Older {
    revision: 2,
    length: 5,
    flags: -4,
    encrypt_metadata: true,
};

const REVISION_3: Older = Older {
    revision: 3,
    length: 16,
    flags: -3904,
    encrypt_metadata: true,
};

const REVISION_4: Older = Older {
    revision: 4,
    length: 16,
    flags: -3904,
    encrypt_metadata: true,
};

/// The `/CF` of a `/V` 4 document whose one filter is `AESV2`, with `/StmF` and `/StrF` given.
fn aesv2_filters(stream_filter: &str, string_filter: &str) -> String {
    format!(
        "/CF << /StdCF << /Type /CryptFilter /CFM /AESV2 /Length 16 /AuthEvent /DocOpen >> >> \
         /StmF /{stream_filter} /StrF /{string_filter}"
    )
}

/// Revision 2: Algorithms 2, 3 and 4 with a 40-bit key, and Algorithm 1 with RC4.
///
/// §7.6.4.4.5 Algorithm 6 step (b): "If the result of step (a) is equal to the value of the
/// encryption dictionary's U entry … the password supplied is the correct user password", and
/// §7.6.4.4.6 Algorithm 7 step (c) makes the owner password the one whose decrypted `/O` is.
#[test]
fn a_revision_2_document_opens_on_either_password_and_decrypts() {
    let built = older_document(REVISION_2, 1, "", (Method::Rc4, Method::Rc4));
    opens_as_the_clause_says(&built.bytes, 2);
}

/// Revision 3: Algorithm 2 step (h)'s fifty MD5s, Algorithm 3 steps (c) and (g), Algorithm 5's
/// nineteen RC4 rounds, and a `/U` compared "on the first 16 bytes" (Algorithm 6 step (b)).
#[test]
fn a_revision_3_document_opens_on_either_password_and_decrypts() {
    let built = older_document(REVISION_3, 2, "", (Method::Rc4, Method::Rc4));
    opens_as_the_clause_says(&built.bytes, 3);
}

/// Revision 4 through §7.6.6's `AESV2` crypt filter: Algorithm 1 with the `sAlT` extension and a
/// stored initialisation vector.
#[test]
fn a_revision_4_aes_document_opens_on_either_password_and_decrypts() {
    let built = older_document(
        REVISION_4,
        4,
        &aesv2_filters("StdCF", "StdCF"),
        (Method::Aes128, Method::Aes128),
    );
    opens_as_the_clause_says(&built.bytes, 4);
}

/// Algorithm 2 step (f): "(Security handlers of revision 4 or greater) If document metadata is
/// not being encrypted, pass 4 bytes with the value 0xFFFFFFFF to the MD5 hash function."
///
/// The key changes with that one entry, so a reader that left the four bytes out derives a key
/// under which neither the page nor the title decrypts.
#[test]
fn unencrypted_metadata_changes_the_revision_4_key() {
    let older = Older {
        encrypt_metadata: false,
        ..REVISION_4
    };
    let built = older_document(
        older,
        4,
        &aesv2_filters("StdCF", "StdCF"),
        (Method::Aes128, Method::Aes128),
    );
    opens_as_the_clause_says(&built.bytes, 4);
}

/// §7.6.2's first two exceptions: "The values for the ID entry in the trailer" and "Any strings
/// in an Encrypt dictionary".
///
/// Both are compared with the bytes this file wrote, so a reader that decrypted either would
/// hand back a different string.
#[test]
fn the_identifier_and_the_encryption_dictionary_are_read_as_written() {
    let built = older_document(REVISION_3, 2, "", (Method::Rc4, Method::Rc4));
    let document = open(&built.bytes, USER).expect("the user password opens it");
    let id = document.get_key(document.trailer(), "ID");
    let first = id
        .as_array()
        .and_then(<[Object]>::first)
        .and_then(Object::as_string)
        .expect("an /ID");
    assert_eq!(first, ID0);
    let encrypt = document.get_key(document.trailer(), "Encrypt");
    let owner = document
        .get_key(encrypt.as_dict().expect("an encryption dictionary"), "O")
        .as_string()
        .expect("/O")
        .to_vec();
    assert_eq!(owner, built.owner_entry);
}

/// Table 20's `/StmF` naming `Identity` while `/StrF` names the document's filter.
///
/// §7.6.6 and Table 26: `Identity` is the filter that leaves data as it is, so the content
/// stream is in the clear while the title is encrypted — and each has to come back as written.
#[test]
fn an_identity_stream_filter_leaves_streams_and_decrypts_strings() {
    let built = older_document(
        REVISION_4,
        4,
        &aesv2_filters("Identity", "StdCF"),
        (Method::Identity, Method::Aes128),
    );
    let document = open(&built.bytes, USER).expect("the user password opens it");
    assert_eq!(page_content(&document), CONTENT);
    assert_eq!(title(&document), TITLE);
    assert!(
        built
            .bytes
            .windows(CONTENT.len())
            .any(|window| window == CONTENT),
        "the content stream is in the file unchanged"
    );
}

/// §7.4.10's `Crypt` filter: a stream whose `/Filter` array names it with `/Name /Identity` is
/// not decrypted by the document's default.
///
/// Table 20's `/StmF`: "All streams in the document, except for cross-reference streams … or
/// streams that have a Crypt entry in their Filter array …, shall be decrypted by the security
/// handler, using this crypt filter." Object 4 is AES-encrypted under `/StmF`; object 8 is in
/// the clear behind its own `Crypt` entry, and both have to come back as written.
#[test]
fn a_crypt_filter_naming_identity_overrides_the_default_stream_filter() {
    let older = REVISION_4;
    let owner_entry = owner_entry(OWNER, USER, older);
    let key = file_key(USER, &owner_entry, older);
    let dictionary = format!(
        "<< /Filter /Standard /V 4 /R 4 /Length 128 {} /O {} /U {} /P {} >>",
        aesv2_filters("StdCF", "StdCF"),
        hex_string(&owner_entry),
        hex_string(&user_entry(&key, older)),
        older.flags,
    );
    let bytes = document(&dictionary, &key, Method::Aes128, Method::Aes128, true);
    let document = open(&bytes, USER).expect("the user password opens it");
    let mut expected = CONTENT.to_vec();
    expected.push(b'\n');
    expected.extend_from_slice(IN_THE_CLEAR);
    assert_eq!(page_content(&document), expected);
}

/// §7.6.4.3.4 Algorithm 2.B, written from the clause's steps.
///
/// `input` is the original input — password, salt and, for the owner, the 48-byte `/U` — and
/// `user_key` is that `/U` again when step (a) is to repeat it, or empty.
fn hash_2b(password: &[u8], input: &[u8], user_key: &[u8]) -> [u8; 32] {
    // "Take the SHA-256 hash of the original input to the algorithm and name the resulting 32
    // bytes, K."
    let mut k: Vec<u8> = sha2::Sha256::digest(input).to_vec();
    let mut rounds = 0u32;
    loop {
        // Step (a): 64 repetitions of password, K and the user key.
        let mut sequence = password.to_vec();
        sequence.extend_from_slice(&k);
        sequence.extend_from_slice(user_key);
        let k1 = sequence.repeat(64);
        // Step (b): AES-128, CBC, no padding, key and vector the two halves of K's first 32.
        let e = cbc(&k[..16], &k[16..32], &k1);
        // Step (c): the first 16 bytes as a big-endian integer, modulo 3. 256 is 1 modulo 3, so
        // the remainder is that of the sum of the bytes.
        let remainder = e[..16].iter().map(|byte| u32::from(*byte)).sum::<u32>() % 3;
        // Step (d).
        k = match remainder {
            0 => sha2::Sha256::digest(&e).to_vec(),
            1 => sha2::Sha384::digest(&e).to_vec(),
            _ => sha2::Sha512::digest(&e).to_vec(),
        };
        rounds += 1;
        // Steps (e) and (f): from round number 64, stop once E's last byte is at most the round
        // number less 32.
        let last = u32::from(*e.last().expect("E is never empty"));
        if rounds >= 64 && last + 32 <= rounds {
            break;
        }
    }
    let mut out = [0; 32];
    out.copy_from_slice(&k[..32]);
    out
}

/// AES-256 in CBC mode with no padding and a vector of zero, as Algorithms 8 and 9 step (b) wrap
/// the file encryption key.
fn wrap(key: &[u8; 32], file_key: &[u8; 32]) -> Vec<u8> {
    cbc(key, &[0; 16], file_key)
}

/// The revision 6 file encryption key: §7.6.4.4.1 asks for a random one, and a fixed one serves
/// the reader no differently.
const FILE_KEY_6: [u8; 32] = *b"a 256-bit key the fixture chose.";

/// A revision 6 document with the given user and owner passwords.
fn revision_6_document(user: &str, owner: &str, flags: i32) -> Vec<u8> {
    // Algorithm 8 step (a): validation salt, key salt, and `/U`.
    let (user_validation, user_key_salt) = (*b"uvsalt-8", *b"uksalt-8");
    let mut u = hash_2b(
        user.as_bytes(),
        &[user.as_bytes(), &user_validation].concat(),
        &[],
    )
    .to_vec();
    u.extend_from_slice(&user_validation);
    u.extend_from_slice(&user_key_salt);
    // Step (b): `/UE`.
    let ue = wrap(
        &hash_2b(
            user.as_bytes(),
            &[user.as_bytes(), &user_key_salt].concat(),
            &[],
        ),
        &FILE_KEY_6,
    );
    // Algorithm 9: the same with the owner's salts and the 48-byte `/U` after each.
    let (owner_validation, owner_key_salt) = (*b"ovsalt-8", *b"oksalt-8");
    let mut o = hash_2b(
        owner.as_bytes(),
        &[owner.as_bytes(), &owner_validation, &u].concat(),
        &u,
    )
    .to_vec();
    o.extend_from_slice(&owner_validation);
    o.extend_from_slice(&owner_key_salt);
    let oe = wrap(
        &hash_2b(
            owner.as_bytes(),
            &[owner.as_bytes(), &owner_key_salt, &u].concat(),
            &u,
        ),
        &FILE_KEY_6,
    );
    // Algorithm 10: `/P` extended to 64 bits with ones, low byte first; "T"; "adb"; four bytes
    // that are ignored; one AES-256 block under the file encryption key.
    let mut block = [0u8; 16];
    block[..8].copy_from_slice(&i64::from(flags).to_le_bytes());
    block[8] = b'T';
    block[9..12].copy_from_slice(b"adb");
    block[12..].copy_from_slice(b"rand");
    Aes256::new_from_slice(&FILE_KEY_6)
        .expect("a 32-byte key")
        .encrypt_block((&mut block).into());

    let dictionary = format!(
        "<< /Filter /Standard /V 5 /R 6 /Length 256 \
         /CF << /StdCF << /Type /CryptFilter /CFM /AESV3 /Length 32 /AuthEvent /DocOpen >> >> \
         /StmF /StdCF /StrF /StdCF /O {} /U {} /OE {} /UE {} /Perms {} /P {flags} \
         /EncryptMetadata true >>",
        hex_string(&o),
        hex_string(&u),
        hex_string(&oe),
        hex_string(&ue),
        hex_string(&block),
    );
    document(
        &dictionary,
        &FILE_KEY_6,
        Method::Aes256,
        Method::Aes256,
        false,
    )
}

/// Revision 6: Algorithm 2.A through Algorithm 2.B's hash, Algorithm 11 against `/U`,
/// Algorithm 12 against `/O`, each unwrapping its own copy of the one file encryption key, and
/// §7.6.3.3's Algorithm 1.A decrypting with it.
#[test]
fn a_revision_6_document_opens_on_either_password_and_decrypts() {
    opens_as_the_clause_says(&revision_6_document(USER, OWNER, -3904), 6);
}

/// §7.6.4.3.3 step (c) tests the owner key first, so an empty owner password is the owner's.
///
/// Algorithm 12 is what decides it: the empty string's hash, salted with the owner validation
/// salt and the 48-byte `/U`, matches `/O`'s first 32 bytes — and §7.6.4.1 says such a password
/// "should allow full (owner) access".
#[test]
fn an_empty_owner_password_opens_a_revision_6_document_as_its_owner() {
    let bytes = revision_6_document(USER, "", -3904);
    let document = open(&bytes, "").expect("the empty password is the owner's");
    assert!(document.permissions().expect("encrypted").owner);
    assert_eq!(page_content(&document), CONTENT);
    let document = open(&bytes, USER).expect("and the user password is still the user's");
    assert!(!document.permissions().expect("encrypted").owner);
}
