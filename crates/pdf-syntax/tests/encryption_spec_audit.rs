//! Review 2131's audit of ISO 32000-2 §7.6, encryption
//! (`doc/reviews/2131-specification-audit-7-6.md`): one failing test per behaviour finding.
//!
//! Each test states a sentence of §7.6 (or of §7.5.8.2, which §7.6.2's `/ID` exception reaches)
//! and a document encrypted by this file from the clause's own steps — the builders below are
//! `encryption_fixtures.rs`'s, written a second time by somebody reading Algorithms 1 to 5, and
//! never `pdf_syntax`'s. Every test carries a control in the same body showing the fixture opens
//! and decrypts, so that a failure is the finding and not a fixture that cannot be read. The
//! failing ones are listed in `doc/reviews/2131-specification-audit-7-6/expected-failures.json`.

#![expect(
    clippy::panic,
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    reason = "test code: a fixture that cannot exercise the rule must fail loudly, and the \
              arithmetic and truncations are the clause's own — byte counters XORed into a key, \
              object numbers split into their low-order bytes, offsets written as four bytes"
)]

use aes::cipher::{BlockModeEncrypt, KeyInit, KeyIvInit, StreamCipher};
use aes::{Aes128, Aes256};
use md5::Md5;
use pdf_syntax::{Document, Limits, Object, ObjectId, SyntaxError};
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

/// A revision 2, 3 or 4 document, with what its encryption dictionary states.
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

/// A revision 4 key under `older`, with the `/O` and `/U` entries Algorithms 3 and 5 make.
struct Keyed {
    key: Vec<u8>,
    owner_entry: Vec<u8>,
    user_entry: Vec<u8>,
}

fn keyed(older: Older) -> Keyed {
    let owner_entry = owner_entry(OWNER, USER, older);
    let key = file_key(USER, &owner_entry, older);
    let user_entry = user_entry(&key, older);
    Keyed {
        key,
        owner_entry,
        user_entry,
    }
}

/// A `/V` 4 `/R` 4 standard encryption dictionary over `keyed`, with `crypt_filters` and
/// `extra` written into it.
fn revision_4_dictionary(older: Older, keyed: &Keyed, crypt_filters: &str, extra: &str) -> String {
    format!(
        "<< /Filter /Standard /V 4 /R 4 /Length 128 {crypt_filters} /O {} /U {} /P {} {extra} >>",
        hex_string(&keyed.owner_entry),
        hex_string(&keyed.user_entry),
        older.flags,
    )
}

/// The first seven objects of [`document`] — catalogue, page tree, page, encrypted content
/// stream, font, information dictionary with an encrypted `/Title`, encryption dictionary — with
/// `catalog_extra` and `page_extra` written into the catalogue and the page, so that a test can
/// add objects from 8 on.
fn seven_objects(
    encrypt_dictionary: &str,
    key: &[u8],
    method: Method,
    catalog_extra: &str,
    page_extra: &str,
) -> Vec<Vec<u8>> {
    vec![
        format!("<< /Type /Catalog /Pages 2 0 R {catalog_extra} >>").into_bytes(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 200] \
             /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R {page_extra} >>"
        )
        .into_bytes(),
        stream(&encrypt(method, key, 4, CONTENT), ""),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
        format!(
            "<< /Title {} >>",
            hex_string(&encrypt(method, key, 6, TITLE))
        )
        .into_bytes(),
        encrypt_dictionary.as_bytes().to_vec(),
    ]
}

/// The trailer entries every fixture here states: §7.6.2's `/ID`, never encrypted.
fn trailer_entries() -> String {
    format!(
        "/Root 1 0 R /Info 6 0 R /Encrypt 7 0 R /ID [{} {}]",
        hex_string(&ID0),
        hex_string(b"fixture-id-other")
    )
}

/// One object's stream data, decoded, by object number.
fn data_of(document: &Document, number: u32) -> Option<Vec<u8>> {
    let object = document.get(ObjectId::new(number, 0));
    let stream = object.as_stream().expect("a stream object");
    document
        .decoded_stream_data(stream)
        .map(|data| data.to_vec())
}

const REVISION_4: Older = Older {
    revision: 4,
    length: 16,
    flags: -3904,
    encrypt_metadata: true,
};

/// An XMP packet, standing for a metadata stream's content.
const XMP: &[u8] =
    b"<?xpacket begin='' id='W5M0MpCehiHzreSzNTczkc9d'?><x:xmpmeta xmlns:x='adobe:ns:meta/'/>";

/// 2131-B01. Table 21's `/EncryptMetadata` "\[i]ndicates whether the document-level metadata stream
/// (see 14.3.2, "Metadata streams") shall be encrypted", and Algorithm 2's NOTE 1 says of its
/// step (f) that "\[t]his provision pertains only to document-level XMP metadata". A page's own
/// metadata stream (§14.3.2 lets any component carry one) is therefore encrypted under `/StmF`
/// like every other stream when `/EncryptMetadata` is false, and has to be decrypted.
#[test]
fn only_the_document_level_metadata_stream_is_left_in_the_clear_spec_audit() {
    let older = Older {
        encrypt_metadata: false,
        ..REVISION_4
    };
    let keyed = keyed(older);
    let dictionary = revision_4_dictionary(
        older,
        &keyed,
        "/CF << /StdCF << /Type /CryptFilter /CFM /AESV2 /Length 16 /AuthEvent /DocOpen >> >> \
         /StmF /StdCF /StrF /StdCF",
        "/EncryptMetadata false",
    );
    let mut objects = seven_objects(
        &dictionary,
        &keyed.key,
        Method::Aes128,
        "/Metadata 8 0 R",
        "/Metadata 9 0 R",
    );
    // 8: the document-level metadata stream, in the clear as Table 21 allows.
    objects.push(stream(XMP, "/Type /Metadata /Subtype /XML"));
    // 9: the page's metadata stream, encrypted under /StmF like any other component's.
    objects.push(stream(
        &encrypt(Method::Aes128, &keyed.key, 9, XMP),
        "/Type /Metadata /Subtype /XML",
    ));
    let bytes = assemble(&objects, &trailer_entries());

    // Control: the key is right (the page decrypts) and the document-level packet is plaintext.
    let document = open(&bytes, USER).expect("the fixture opens with its user password");
    assert!(!document.was_recovered(), "exact-xref fixture");
    assert_eq!(page_content(&document), CONTENT);
    assert_eq!(data_of(&document, 8).as_deref(), Some(XMP));

    // The finding: the page's metadata stream was encrypted, and reads as its plaintext.
    assert_eq!(
        data_of(&document, 9).as_deref(),
        Some(XMP),
        "Table 21: /EncryptMetadata false leaves only the document-level metadata stream in the \
         clear; a page's metadata stream is decrypted under /StmF"
    );
}

/// 2131-B02. Table 20's `/CF`: "Any keys in the CF dictionary that are listed in "Table 26 -
/// Standard crypt filter names" shall be ignored by a PDF processor." An `/Identity` entry with a
/// `/Length` of its own therefore says nothing about the file encryption key, which Table 20's
/// `/V` 4 row fixes at 128 bits and the `StdCF` filter states as 16 bytes.
#[test]
fn an_identity_cf_entry_does_not_set_the_key_length_spec_audit() {
    let older = REVISION_4;
    let keyed = keyed(older);
    let filters = "/StdCF << /Type /CryptFilter /CFM /V2 /Length 16 /AuthEvent /DocOpen >>";
    let control = assemble(
        &seven_objects(
            &revision_4_dictionary(
                older,
                &keyed,
                &format!("/CF << {filters} >> /StmF /StdCF /StrF /StdCF"),
                "",
            ),
            &keyed.key,
            Method::Rc4,
            "",
            "",
        ),
        &trailer_entries(),
    );
    let control = open(&control, USER).expect("the control opens with its user password");
    assert_eq!(page_content(&control), CONTENT);
    assert_eq!(title(&control), TITLE);

    let with_identity = assemble(
        &seven_objects(
            &revision_4_dictionary(
                older,
                &keyed,
                &format!(
                    "/CF << /Identity << /Length 40 >> {filters} >> /StmF /StdCF /StrF /StdCF"
                ),
                "",
            ),
            &keyed.key,
            Method::Rc4,
            "",
            "",
        ),
        &trailer_entries(),
    );
    let document = open(&with_identity, USER).expect(
        "Table 20: a /CF entry named Identity shall be ignored, so the same key opens the file",
    );
    assert_eq!(page_content(&document), CONTENT);
    assert_eq!(title(&document), TITLE);
}

/// 2131-B03. §7.6.6: a stream's Crypt filter decode parameters dictionary's "Name entry specifies
/// the particular crypt filter that shall be used (if missing, Identity is used)", and Table 20's
/// `/CF`: "Every crypt filter used in the document shall have an entry in this dictionary, except
/// for the standard crypt filter names". Identity is the clause's answer for a *missing* name only;
/// a name `/CF` does not define names no filter at all, and the stream's ciphertext is not its data.
#[test]
fn a_crypt_filter_name_cf_does_not_define_is_not_identity_spec_audit() {
    let older = REVISION_4;
    let keyed = keyed(older);
    let dictionary = revision_4_dictionary(
        older,
        &keyed,
        "/CF << /StdCF << /Type /CryptFilter /CFM /AESV2 /Length 16 /AuthEvent /DocOpen >> >> \
         /StmF /StdCF /StrF /StdCF",
        "",
    );
    let ciphertext = encrypt(Method::Aes128, &keyed.key, 8, IN_THE_CLEAR);
    let named = |name: &str| {
        let mut objects = seven_objects(&dictionary, &keyed.key, Method::Aes128, "", "");
        objects.push(stream(
            &ciphertext,
            &format!(
                "/Filter [/Crypt] /DecodeParms [<< /Type /CryptFilterDecodeParms /Name /{name} >>]"
            ),
        ));
        assemble(&objects, &trailer_entries())
    };

    // Control: the same ciphertext under the filter /CF defines decrypts to its plaintext.
    let control = open(&named("StdCF"), USER).expect("the control opens");
    assert_eq!(page_content(&control), CONTENT);
    assert_eq!(data_of(&control, 8).as_deref(), Some(IN_THE_CLEAR));

    let undefined = open(&named("Undefined"), USER).expect("the document itself opens");
    assert_eq!(page_content(&undefined), CONTENT);
    assert_ne!(
        data_of(&undefined, 8).as_deref(),
        Some(ciphertext.as_slice()),
        "§7.6.6 and Table 20: a /Crypt /Name that /CF does not define is not Identity; the \
         stream's ciphertext is not returned as its data"
    );
}

/// 2131-B04. Table 20's `/CF` rule binds the public-key handler too: "Any keys in the CF
/// dictionary that are listed in "Table 26 - Standard crypt filter names" shall be ignored by a PDF
/// processor." A document whose `/CF` carries an `/Identity` entry is the same document as one
/// without it, and asks a reader with no certificate for one.
#[test]
fn a_public_key_identity_cf_entry_is_ignored_spec_audit() {
    let dictionary = |identity: &str| {
        format!(
            "<< /Filter /Adobe.PubSec /SubFilter /adbe.pkcs7.s5 /V 4 /CF << /DefaultCryptFilter \
             << /CFM /AESV2 /AuthEvent /DocOpen /Recipients [<3000>] >> {identity} >> \
             /StmF /DefaultCryptFilter /StrF /DefaultCryptFilter >>"
        )
    };
    let bytes = |identity: &str| {
        assemble(
            &seven_objects(&dictionary(identity), &[], Method::Identity, "", ""),
            &trailer_entries(),
        )
    };

    // Control: the dictionary without the entry asks for a certificate.
    match open(&bytes(""), "") {
        Err(SyntaxError::RecipientRequired) => {}
        other => panic!("the control should ask for a certificate, got {other:?}"),
    }

    match open(&bytes("/Identity << /Recipients [<3000>] >>"), "") {
        Err(SyntaxError::RecipientRequired) => {}
        other => panic!(
            "Table 20: a /CF entry named Identity shall be ignored, so the document asks for a \
             certificate as the control does; got {other:?}"
        ),
    }
}

/// [`assemble`] with §7.5.8's cross-reference stream in place of a table: the stream is the last
/// object and its dictionary carries the trailer's entries.
fn assemble_with_cross_reference_stream(objects: &[Vec<u8>], trailer_entries: &str) -> Vec<u8> {
    let mut out: Vec<u8> = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = Vec::with_capacity(objects.len() + 1);
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let number = objects.len() + 1;
    let at = out.len();
    offsets.push(at);
    // Table 18's `/W [1 4 2]`: type, offset, generation.
    let mut data = vec![0u8, 0, 0, 0, 0, 0xFF, 0xFF];
    for offset in &offsets {
        data.push(1);
        data.extend_from_slice(&(*offset as u32).to_be_bytes());
        data.extend_from_slice(&[0, 0]);
    }
    out.extend_from_slice(
        format!(
            "{number} 0 obj\n<< /Type /XRef /Size {} /W [1 4 2] {trailer_entries} /Length {} >>\n\
             stream\n",
            number + 1,
            data.len()
        )
        .as_bytes(),
    );
    out.extend_from_slice(&data);
    out.extend_from_slice(format!("\nendstream\nendobj\nstartxref\n{at}\n%%EOF\n").as_bytes());
    out
}

/// 2131-B05. §7.6.2 exempts "\[t]he values for the ID entry in the trailer", and §7.5.8.2, where a
/// cross-reference stream's dictionary is the trailer: "The cross-reference stream shall not be
/// encrypted and strings appearing in the cross-reference stream dictionary shall not be
/// encrypted." The stream object read as an object keeps its `/ID` as written.
#[test]
fn a_cross_reference_stream_dictionarys_strings_are_not_decrypted_spec_audit() {
    let older = Older {
        revision: 3,
        length: 16,
        flags: -3904,
        encrypt_metadata: true,
    };
    let keyed = keyed(older);
    let dictionary = format!(
        "<< /Filter /Standard /V 2 /R 3 /Length 128 /O {} /U {} /P {} >>",
        hex_string(&keyed.owner_entry),
        hex_string(&keyed.user_entry),
        older.flags,
    );
    let bytes = assemble_with_cross_reference_stream(
        &seven_objects(&dictionary, &keyed.key, Method::Rc4, "", ""),
        &trailer_entries(),
    );

    // Control: the file opens through its cross-reference stream, and the key is right.
    let document = open(&bytes, USER).expect("the fixture opens with its user password");
    assert!(!document.was_recovered(), "exact cross-reference stream");
    assert_eq!(page_content(&document), CONTENT);
    assert_eq!(title(&document), TITLE);

    let object = document.get(ObjectId::new(8, 0));
    let stream = object
        .as_stream()
        .expect("object 8 is the cross-reference stream");
    let first = stream
        .dict
        .get("ID")
        .and_then(Object::as_array)
        .and_then(<[Object]>::first)
        .and_then(Object::as_string)
        .map(<[u8]>::to_vec);
    assert_eq!(
        first.as_deref(),
        Some(&ID0[..]),
        "§7.5.8.2: strings in the cross-reference stream dictionary are not encrypted, and §7.6.2 \
         exempts the trailer's /ID"
    );
}

/// What an attachment holds, before any filter.
const ATTACHMENT: &[u8] = b"an attachment the producer left in the clear";

/// 2131-B06. Table 20's `/EFF` is "\[t]he name of the crypt filter that shall be used when
/// encrypting embedded file streams that do not have their own crypt filter specifier", and §7.11.4
/// Table 44 makes an embedded file stream's `/Type` "( Optional)" — so what makes a stream an
/// embedded file stream is the file specification's `/EF` naming it, not the entry. An attachment
/// without `/Type` under `/EFF /Identity` is in the clear, as one with the entry is.
#[test]
fn an_embedded_file_stream_without_a_type_takes_the_eff_filter_spec_audit() {
    let older = REVISION_4;
    let keyed = keyed(older);
    let dictionary = revision_4_dictionary(
        older,
        &keyed,
        "/CF << /StdCF << /Type /CryptFilter /CFM /AESV2 /Length 16 /AuthEvent /DocOpen >> >> \
         /StmF /StdCF /StrF /StdCF /EFF /Identity",
        "",
    );
    let with_type = |type_entry: &str| {
        let mut objects = seven_objects(&dictionary, &keyed.key, Method::Aes128, "/AF [8 0 R]", "");
        // 8: the file specification whose /EF makes object 9 an embedded file stream (§7.11.3).
        objects.push(
            b"<< /Type /Filespec /AFRelationship /Unspecified /EF << /F 9 0 R >> >>".to_vec(),
        );
        objects.push(stream(ATTACHMENT, type_entry));
        assemble(&objects, &trailer_entries())
    };

    // Control: with Table 44's optional /Type stated, /EFF /Identity leaves the data as it is.
    let control = open(&with_type("/Type /EmbeddedFile"), USER).expect("the control opens");
    assert_eq!(page_content(&control), CONTENT);
    assert_eq!(data_of(&control, 9).as_deref(), Some(ATTACHMENT));

    let untyped = open(&with_type(""), USER).expect("the fixture opens");
    assert_eq!(page_content(&untyped), CONTENT);
    assert_eq!(
        data_of(&untyped, 9).as_deref(),
        Some(ATTACHMENT),
        "Table 20's /EFF governs the stream /EF names, with or without Table 44's optional /Type"
    );
}
