//! ISO 32000-2 §7.6.5's witness set: the documents a public-key build is tested against.
//!
//! # What the set proves
//!
//! `doc/questions/A168` makes the documents below the end-to-end witness set of §7.6.5's
//! public-key security handler (`doc/questions/A800` ordered the handler built). Each document's
//! `/Recipients` names the certificate of a keystore the `PDFBox` corpus ships beside it, and the
//! handler opens each one with that keystore's certificate and private key and gives back the page
//! text `PDFBox`'s own test states.
//!
//! §7.6.5.1 is the reader's one `shall` here: "the PDF reader shall scan the recipient list for
//! which the content is encrypted and shall attempt to find a match with a certificate that
//! belongs to the user." Table 27 puts the list in the crypt filter dictionary for `/SubFilter
//! /adbe.pkcs7.s5`, and each item is a CMS `EnvelopedData` (RFC 5652 section 6.1) whose recipient
//! is named by issuer and serial number.
//!
//! # Where each side of the comparison comes from
//!
//! **The recipients** are read by this crate's own parser — the cross-reference table, the
//! encryption dictionary as an indirect object, its `/CF` — and walked as DER by the short
//! reader below — a reader of the test's own, so that the recipient the handler matches is
//! checked by something other than the handler's `pdf_pki::enveloped`.
//!
//! **The certificates** are evidence, not this tree's reading. Both keystores keep their
//! certificate bag under `pbeWithSHA1And40BitRC2-CBC` (RFC 7292 appendix C), a cipher nothing in
//! this tree implements, so the issuer and serial below were read with
//! `openssl pkcs12 -legacy -nokeys -in <keystore> -passin pass:<password> | openssl x509 -noout
//! -issuer -serial`, with the passwords `PDFBox`'s `TestPublicKeyEncryption` passes: for
//! `PDFBOX-4421-keystore.pfx`, `CN=testnutzer`, serial `5F609C62`; for `PDFBOX-5249.p12` (empty
//! password), `CN=test`, serial `60FFD550`. Each keystore holds one certificate, self-issued.
//!
//! **The keys** are the same keystores' private keys, converted to the form a host hands
//! [`RecipientKey::new`] — the certificate's DER and RFC 5958's `OneAsymmetricKey` — with
//! `openssl pkcs12 -legacy -nocerts -nodes … | openssl pkcs8 -topk8 -nocrypt -outform DER` and
//! `openssl pkcs12 -legacy -nokeys … | openssl x509 -outform DER`, into
//! `tests/public_key_witnesses/`. They are Apache-2.0 data of `PDFBox`'s (`doc/third-party-data.md`
//! under `doc/corpora/pdfbox`), published test keys that belong to no reader. A format conversion
//! is all `openssl` contributes: what each test expects back is the text `PDFBox`'s
//! `TestPublicKeyEncryption` states for the page, and the clause's algorithm is what has to produce
//! it.
//!
//! # The fifth document
//!
//! `3006236.pdf` of the `SafeDocs` crawl is the fifth document in the corpus census that names
//! the handler, and it is encrypted to a certificate whose issuer is a device
//! (`zune-tuner://windowsphone/…`) and whose key is nowhere. It is asserted to match neither
//! keystore, so that "four of the five" stays a checked sentence.

#![expect(
    clippy::panic,
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: a witness that cannot be read must fail loudly, and the test-local readers \
              step through bytes of a known size"
)]

use std::path::{Path, PathBuf};

use pdf_syntax::xref::{self, Location};
use pdf_syntax::{Document, FileBytes, Limits, Object, Parser, RecipientKey, SyntaxError};

/// `id-envelopedData`, 1.2.840.113549.1.7.3 (RFC 5652 section 6.1), as DER content octets.
const ENVELOPED_DATA: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x07, 0x03];

/// `rsaEncryption`, 1.2.840.113549.1.1.1 (RFC 8017 appendix A.1).
const RSA_ENCRYPTION: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x01];

/// `id-at-commonName`, 2.5.4.3 (RFC 5280 appendix A.1).
const COMMON_NAME: &[u8] = &[0x55, 0x04, 0x03];

/// A keystore the `PDFBox` corpus ships, and the certificate it holds.
struct Keystore {
    file: &'static str,
    common_name: &'static str,
    serial: &'static [u8],
}

/// `PDFBOX-4421-keystore.pfx`: `CN=testnutzer`, serial `5F609C62`.
const TESTNUTZER: Keystore = Keystore {
    file: "PDFBOX-4421-keystore.pfx",
    common_name: "testnutzer",
    serial: &[0x5F, 0x60, 0x9C, 0x62],
};

/// `PDFBOX-5249.p12`: `CN=test`, serial `60FFD550`.
const TEST: Keystore = Keystore {
    file: "PDFBOX-5249.p12",
    common_name: "test",
    serial: &[0x60, 0xFF, 0xD5, 0x50],
};

/// The `PDFBox` corpus's encryption fixtures, where the four documents and both keystores sit.
fn pdfbox_encryption() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc/corpora/pdfbox/pdfbox/src/test/resources/org/apache/pdfbox/encryption")
}

/// One recipient as `EnvelopedData` names it: the issuer's common name and the serial number.
#[derive(Debug, PartialEq, Eq)]
struct Recipient {
    common_name: String,
    serial: Vec<u8>,
}

/// One DER value: its tag and its content octets.
#[derive(Clone, Copy)]
struct Tlv<'a> {
    tag: u8,
    content: &'a [u8],
}

/// Splits `bytes` into its consecutive DER values (X.690 section 8.1), definite lengths only.
fn values(mut bytes: &[u8]) -> Vec<Tlv<'_>> {
    let mut out = Vec::new();
    while let [tag, first, rest @ ..] = bytes {
        let (length, rest) = if *first < 0x80 {
            (usize::from(*first), rest)
        } else {
            let octets = usize::from(first & 0x7F);
            assert!(
                (1..=4).contains(&octets) && rest.len() >= octets,
                "a definite long-form length"
            );
            let length = rest[..octets]
                .iter()
                .fold(0usize, |sum, byte| (sum << 8) | usize::from(*byte));
            (length, &rest[octets..])
        };
        assert!(rest.len() >= length, "a value inside its container");
        out.push(Tlv {
            tag: *tag,
            content: &rest[..length],
        });
        bytes = &rest[length..];
    }
    assert!(bytes.is_empty(), "no trailing octets after the last value");
    out
}

/// A `DirectoryString` as text (RFC 5280 section 4.1.2.4): the two witnesses' keystores write
/// `UTF8String` or `PrintableString`, and the fifth document's issuer a `BMPString`, which is
/// UTF-16 big-endian.
fn directory_string(value: Tlv<'_>) -> String {
    match value.tag {
        0x0C | 0x13 => String::from_utf8(value.content.to_vec())
            .unwrap_or_else(|_| panic!("a UTF8String or PrintableString is UTF-8")),
        0x1E => {
            let units: Vec<u16> = value
                .content
                .chunks_exact(2)
                .map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
                .collect();
            String::from_utf16(&units).unwrap_or_else(|_| panic!("a BMPString is UTF-16"))
        }
        other => panic!("a DirectoryString, not tag {other:#04x}"),
    }
}

/// Reads the one recipient of a `/Recipients` item, asserting the shape the census found.
///
/// `ContentInfo` → `[0]` → `EnvelopedData` → `recipientInfos` → one `KeyTransRecipientInfo` of
/// version 0, whose `rid` is `IssuerAndSerialNumber` and whose key is wrapped by
/// `rsaEncryption` (RFC 5652 sections 6.1 and 6.2.1).
fn recipient(item: &[u8]) -> Recipient {
    let [content_info] = values(item)[..] else {
        panic!("a /Recipients item is one ContentInfo");
    };
    let [content_type, explicit] = values(content_info.content)[..] else {
        panic!("ContentInfo is a type and its content");
    };
    assert_eq!(
        content_type.content, ENVELOPED_DATA,
        "the content is EnvelopedData"
    );
    assert_eq!(explicit.tag, 0xA0, "the content is [0] EXPLICIT");
    let [enveloped] = values(explicit.content)[..] else {
        panic!("[0] holds one EnvelopedData");
    };
    // version, recipientInfos (a SET), encryptedContentInfo: no originatorInfo in any witness.
    let fields = values(enveloped.content);
    let infos = fields
        .iter()
        .find(|field| field.tag == 0x31)
        .unwrap_or_else(|| panic!("EnvelopedData has its recipientInfos"));
    let [info] = values(infos.content)[..] else {
        panic!("exactly one recipient, as the census found in every document");
    };
    assert_eq!(
        info.tag, 0x30,
        "a KeyTransRecipientInfo, which is a SEQUENCE"
    );
    let [version, rid, algorithm, encrypted_key] = values(info.content)[..] else {
        panic!("KeyTransRecipientInfo has four fields");
    };
    assert_eq!(
        version.content,
        [0],
        "version 0: rid is IssuerAndSerialNumber"
    );
    let [algorithm_oid, ..] = values(algorithm.content)[..] else {
        panic!("an AlgorithmIdentifier");
    };
    assert_eq!(
        algorithm_oid.content, RSA_ENCRYPTION,
        "the key is wrapped under RSA"
    );
    assert_eq!(
        encrypted_key.tag, 0x04,
        "the wrapped key is an OCTET STRING"
    );
    let [issuer, serial] = values(rid.content)[..] else {
        panic!("IssuerAndSerialNumber is an issuer and a serial");
    };
    assert_eq!(serial.tag, 0x02, "the serial is an INTEGER");
    let common_name = values(issuer.content)
        .into_iter()
        .flat_map(|rdn| values(rdn.content))
        .find_map(|attribute| match values(attribute.content)[..] {
            [oid, value] if oid.content == COMMON_NAME => Some(directory_string(value)),
            _ => None,
        })
        .unwrap_or_else(|| panic!("the issuer states a common name"));
    Recipient {
        common_name,
        serial: serial.content.to_vec(),
    }
}

/// The recipients of a document's default crypt filter, read without decrypting anything.
///
/// The encryption dictionary is never itself encrypted (§7.6.1), so it is parsed where the
/// cross-reference table puts it. `None` where the document is not on this machine.
fn recipients_of(path: &Path) -> Option<Vec<Recipient>> {
    let Ok(bytes) = std::fs::read(path) else {
        eprintln!("skipped: {} is not on this machine", path.display());
        return None;
    };
    let file = FileBytes::from(bytes.as_slice());
    let table = xref::read(&file, Limits::DEFAULT)
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    let reference = table
        .trailer()
        .get("Encrypt")
        .and_then(Object::as_reference)
        .unwrap_or_else(|| panic!("{} states an indirect /Encrypt", path.display()));
    let Some(Location::Offset(offset)) = table.location(reference.number) else {
        panic!(
            "{}: the encryption dictionary is at an offset",
            path.display()
        );
    };
    let (_, encrypt) = Parser::at(&bytes, offset, Limits::DEFAULT)
        .parse_indirect_object()
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    let encrypt = encrypt.as_dict().cloned().unwrap_or_default();
    let name = |key: &str| {
        encrypt
            .get(key)
            .and_then(Object::as_name)
            .map(|name| name.as_bytes().to_vec())
    };
    assert_eq!(name("Filter").as_deref(), Some(&b"Adobe.PubSec"[..]));
    assert_eq!(name("SubFilter").as_deref(), Some(&b"adbe.pkcs7.s5"[..]));
    let filter = encrypt
        .get("CF")
        .and_then(Object::as_dict)
        .and_then(|filters| filters.get("DefaultCryptFilter"))
        .and_then(Object::as_dict)
        .unwrap_or_else(|| panic!("{}: /CF names DefaultCryptFilter", path.display()));
    let items = filter
        .get("Recipients")
        .and_then(Object::as_array)
        .unwrap_or_else(|| panic!("{}: Table 27's /Recipients is an array", path.display()));
    Some(
        items
            .iter()
            .map(|item| recipient(item.as_string().expect("each item is a byte string")))
            .collect(),
    )
}

/// Asserts that `document` is encrypted to `keystore`'s certificate, and that the keystore is
/// still where the witness set says it is.
fn is_encrypted_to(document: &Path, keystore: &Keystore) {
    let Some(recipients) = recipients_of(document) else {
        return;
    };
    assert_eq!(
        recipients,
        [Recipient {
            common_name: keystore.common_name.to_owned(),
            serial: keystore.serial.to_vec(),
        }],
        "{} is encrypted to {}'s certificate",
        document.display(),
        keystore.file
    );
    assert!(
        pdfbox_encryption().join(keystore.file).is_file(),
        "{} ships beside the documents it decrypts",
        keystore.file
    );
}

/// `PDFBox`'s `testReadPubkeyEncryptedAES128` and `…AES256`: `PDFBOX-4421-keystore.pfx`.
#[test]
fn the_pdfbox_4421_documents_are_encrypted_to_its_keystore() {
    for name in ["AESkeylength128.pdf", "AESkeylength256.pdf"] {
        is_encrypted_to(&pdfbox_encryption().join(name), &TESTNUTZER);
    }
}

/// `PDFBox`'s `testReadPubkeyEncryptedAES128withMetadataExposed` and `…AES256…`:
/// `PDFBOX-5249.p12`.
#[test]
fn the_pdfbox_5249_documents_are_encrypted_to_its_keystore() {
    for name in ["AES128ExposedMeta.pdf", "AES256ExposedMeta.pdf"] {
        is_encrypted_to(&pdfbox_encryption().join(name), &TEST);
    }
}

/// The corpus census's four `PDFBOX-4421-*.pdf`: the bug report's attachments, the first two
/// byte-identical to `AESkeylength128.pdf` and `AESkeylength256.pdf`, all four encrypted to the
/// same certificate. Skips where the machine-local crawl is absent.
#[test]
fn the_census_s_four_are_encrypted_to_the_same_keystore() {
    let batch = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../corpus-cache/tika-issue-tracker/batch1/PDFBOX");
    for index in 0..4 {
        is_encrypted_to(&batch.join(format!("PDFBOX-4421-{index}.pdf")), &TESTNUTZER);
    }
}

/// The census's fifth document is encrypted to neither keystore.
#[test]
fn the_fifth_document_matches_no_keystore_in_the_tree() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../corpus-cache/safedocs/cc-main-2021-31/3006/3006236.pdf");
    let Some(recipients) = recipients_of(&path) else {
        return;
    };
    let [only] = &recipients[..] else {
        panic!("one recipient");
    };
    assert!(
        only.common_name.starts_with("zune-tuner://windowsphone/"),
        "the recipient is a device, not {:?}",
        only.common_name
    );
    for keystore in [TESTNUTZER, TEST] {
        assert_ne!(only.serial, keystore.serial, "not {}", keystore.file);
    }
}

/// A keystore's certificate and private key, as a host would hand them over.
fn recipient_key(name: &str) -> RecipientKey {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/public_key_witnesses");
    let read = |suffix: &str| {
        std::fs::read(directory.join(format!("{name}.{suffix}.der"))).expect("a key fixture")
    };
    RecipientKey::new(read("certificate"), &read("key")).expect("the keystore's key reads")
}

/// Page one's content streams, decoded and joined.
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
    streams
        .iter()
        .flat_map(|item| {
            let stream = item.as_stream().expect("a content stream");
            document
                .decoded_stream_data(stream)
                .expect("it decodes")
                .to_vec()
        })
        .collect()
}

/// The text page one shows: every hexadecimal string in its content stream, read through the
/// `/ToUnicode` `CMap` (§9.10.3) of the font `Tf` last selected, two-byte codes as the witnesses'
/// `Identity-H` fonts use.
///
/// The witnesses draw their text as glyph identifiers, so the page's words are only reachable
/// through the `CMap`s — which are themselves streams the handler had to decrypt. A test-local reader
/// of `bfchar` and `bfrange`, because the `CMap` parser is `pdf-model`'s, above this crate.
fn shown_text(document: &Document, content: &[u8]) -> String {
    let catalog = document.catalog().expect("a catalogue");
    let pages = document.get_key(&catalog, "Pages");
    let kids = document.get_key(pages.as_dict().expect("a page tree"), "Kids");
    let page = document.resolve(kids.as_array().and_then(<[Object]>::first).expect("a kid"));
    let resources = document.get_key(page.as_dict().expect("a page"), "Resources");
    let fonts = document.get_key(resources.as_dict().expect("resources"), "Font");
    let maps: std::collections::BTreeMap<Vec<u8>, _> = fonts
        .as_dict()
        .expect("fonts")
        .iter()
        .map(|(name, font)| {
            let font = document.resolve(font);
            let cmap = document.get_key(font.as_dict().expect("a font"), "ToUnicode");
            let cmap = document
                .decoded_stream_data(cmap.as_stream().expect("a ToUnicode stream"))
                .expect("it decodes");
            (name.as_bytes().to_vec(), to_unicode(&cmap))
        })
        .collect();
    let mut shown = String::new();
    let mut font: &[u8] = &[];
    let tokens: Vec<&[u8]> = content
        .split(u8::is_ascii_whitespace)
        .filter(|token| !token.is_empty())
        .collect();
    for (at, token) in tokens.iter().enumerate() {
        if let Some(name) = token.strip_prefix(b"/")
            && tokens.get(at + 2) == Some(&&b"Tf"[..])
        {
            font = name;
        }
        // A string's closing `>` may be followed by its operator with no space between.
        if let Some(rest) = token.strip_prefix(b"<")
            && let Some(inner) = rest.split(|&byte| byte == b'>').next()
            && let Some(map) = maps.get(font)
        {
            for pair in inner.chunks(4) {
                let code = u32::from_str_radix(std::str::from_utf8(pair).expect("ASCII"), 16)
                    .expect("hexadecimal");
                shown.extend(map.get(&code).copied().and_then(char::from_u32));
            }
        }
    }
    shown
}

/// A `ToUnicode` `CMap`'s `bfchar` and `bfrange` entries with single-code destinations.
fn to_unicode(cmap: &[u8]) -> std::collections::BTreeMap<u32, u32> {
    let mut map = std::collections::BTreeMap::new();
    let tokens: Vec<&[u8]> = cmap
        .split(u8::is_ascii_whitespace)
        .filter(|token| !token.is_empty())
        .collect();
    let hex = |token: &[u8]| -> u32 {
        let inner = token
            .strip_prefix(b"<")
            .and_then(|rest| rest.strip_suffix(b">"))
            .expect("a hexadecimal string");
        u32::from_str_radix(std::str::from_utf8(inner).expect("ASCII"), 16).expect("hexadecimal")
    };
    let mut at = 0;
    while at < tokens.len() {
        if tokens[at] == b"beginbfchar" {
            at += 1;
            while tokens[at] != b"endbfchar" {
                map.insert(hex(tokens[at]), hex(tokens[at + 1]));
                at += 2;
            }
        } else if tokens[at] == b"beginbfrange" {
            at += 1;
            while tokens[at] != b"endbfrange" {
                let (low, high, first) =
                    (hex(tokens[at]), hex(tokens[at + 1]), hex(tokens[at + 2]));
                for code in low..=high {
                    map.insert(code, first + (code - low));
                }
                at += 3;
            }
        }
        at += 1;
    }
    map
}

/// The four documents, the keystore each is encrypted to, and the text `PDFBox`'s
/// `TestPublicKeyEncryption` states for each, split into the strings the page shows.
const OPENED: [(&str, &str, &[&str]); 4] = [
    ("AESkeylength128.pdf", "testnutzer", &["Key length: 128"]),
    ("AESkeylength256.pdf", "testnutzer", &["Key length: 256"]),
    (
        "AES128ExposedMeta.pdf",
        "test",
        &["AES key length: 128", "with exposed Metadata"],
    ),
    (
        "AES256ExposedMeta.pdf",
        "test",
        &["AES key length: 256", "with exposed Metadata"],
    ),
];

/// Whether `haystack` holds `needle` as consecutive bytes.
fn holds(haystack: &[u8], needle: &str) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle.as_bytes())
}

/// §7.6.5.1 to §7.6.5.3 end to end: each document opens with its keystore's key, and page one's
/// content stream — AES-128 at `/V` 4 and AES-256 at `/V` 5, under the key §7.6.5.3 digests from
/// the seed — decrypts to the text the page shows.
///
/// The two `ExposedMeta` documents state `/EncryptMetadata false` in their crypt filter, which is
/// §7.6.5.3 step c)'s case, "4 bytes with the value 0xFF if ... the document metadata is being
/// left as plaintext": a key digested without them decrypts nothing, so these two are the step's
/// witness.
#[test]
fn each_witness_opens_with_its_keystore_s_key_and_shows_its_text() {
    for (name, keystore, text) in OPENED {
        let bytes = std::fs::read(pdfbox_encryption().join(name)).expect("a witness");
        let document =
            Document::open_with_recipient(bytes, Limits::DEFAULT, &recipient_key(keystore))
                .unwrap_or_else(|error| panic!("{name} opens with {keystore}'s key: {error}"));
        let shown = shown_text(&document, &page_content(&document));
        for line in text {
            assert!(
                shown.contains(line),
                "{name}'s page shows {line:?}, not {shown:?}"
            );
        }
        // §7.6.5.3: "For adbe.pkcs7.s5 ... the permissions shall be the same as adbe.pkcs7.s4",
        // and s4's are "security handlers of revision 3 permissions".
        let permissions = document.permissions().expect("an encrypted document");
        assert_eq!(permissions.revision, 3, "{name}");
    }
}

/// Without a key, and with a key whose certificate no list names, §7.6.5.1's scan finds no match
/// and says so — the public-key handler's "password required" — rather than calling the document
/// unsupported or drawing ciphertext.
#[test]
fn a_reader_the_recipient_list_does_not_name_is_asked_for_a_certificate() {
    let bytes = std::fs::read(pdfbox_encryption().join("AESkeylength128.pdf")).expect("a witness");
    assert!(matches!(
        Document::open(bytes.clone()),
        Err(SyntaxError::RecipientRequired)
    ));
    assert!(matches!(
        Document::open_with_recipient(bytes, Limits::DEFAULT, &recipient_key("test")),
        Err(SyntaxError::RecipientRequired)
    ));
}

/// The plant: the right certificate with another keystore's private key. The scan matches, RFC
/// 8017's decoding fails, and the refusal names the key rather than the document.
#[test]
fn a_certificate_with_the_wrong_private_key_is_refused_as_the_key() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/public_key_witnesses");
    let certificate = std::fs::read(directory.join("testnutzer.certificate.der")).expect("a key");
    let other_key = std::fs::read(directory.join("test.key.der")).expect("a key");
    let mismatched = RecipientKey::new(certificate, &other_key).expect("both read");
    let bytes = std::fs::read(pdfbox_encryption().join("AESkeylength256.pdf")).expect("a witness");
    assert!(matches!(
        Document::open_with_recipient(bytes, Limits::DEFAULT, &mismatched),
        Err(SyntaxError::RecipientKey { .. })
    ));
    // And a key that is not a key at all is refused when it is handed over.
    assert!(matches!(
        RecipientKey::new(vec![0x30, 0x00], &[0x30, 0x00]),
        Err(SyntaxError::RecipientKey { .. })
    ));
}

/// The census's `PDFBOX-4421-2.pdf` and `-3.pdf`, the attachments of the same report that are
/// not byte-identical to the corpus's two, open with the same key. Skips where the
/// machine-local crawl is absent.
#[test]
fn the_census_s_other_two_open_with_the_same_key() {
    let batch = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../corpus-cache/tika-issue-tracker/batch1/PDFBOX");
    for index in 2..4 {
        let Ok(bytes) = std::fs::read(batch.join(format!("PDFBOX-4421-{index}.pdf"))) else {
            continue;
        };
        let document =
            Document::open_with_recipient(bytes, Limits::DEFAULT, &recipient_key("testnutzer"));
        assert!(
            document.is_ok(),
            "PDFBOX-4421-{index}.pdf: {:?}",
            document.err()
        );
    }
}

/// Table 23's `/Recipients` without crypt filters, at `/SubFilter` `adbe.pkcs7.s3` and `s4`.
///
/// No document this tree holds uses either, so both are written for this test by a script that
/// states its own algorithm (`gen_s34.py`, kept with the round that wrote it): a page and an
/// information dictionary under §7.6.3.2's Algorithm 1 with RC4 and a 128-bit key, the key
/// §7.6.5.3's SHA-1 over a 20-byte seed and the one recipient list, and the list a CMS
/// `EnvelopedData` that `openssl cms -encrypt` made for `PDFBOX-4421-keystore.pfx`'s certificate
/// over the seed and Table 24's word with bits 3 and 5 set. So what each must decrypt to is what
/// the script wrote, and the permissions are read with the revision §7.6.5.3 assigns: "only those
/// specified for revision 2" at s3, and "revision 3 permissions" at s4.
#[test]
fn a_recipient_list_in_the_encryption_dictionary_opens_at_s3_and_s4() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/public_key_witnesses");
    for (sub_filter, revision) in [("s3", 2), ("s4", 3)] {
        let bytes = std::fs::read(directory.join(format!("adbe-pkcs7-{sub_filter}.pdf")))
            .expect("a fixture");
        let document =
            Document::open_with_recipient(bytes, Limits::DEFAULT, &recipient_key("testnutzer"))
                .unwrap_or_else(|error| panic!("{sub_filter} opens: {error}"));
        assert!(holds(
            &page_content(&document),
            &format!("(Public key {sub_filter}) Tj")
        ));
        let permissions = document.permissions().expect("encrypted");
        assert_eq!(permissions.revision, revision);
        assert!(
            permissions.print && permissions.copy,
            "bits 3 and 5 are set"
        );
        assert!(!permissions.modify && !permissions.annotate && !permissions.owner);
    }
}

/// Page one's content streams, each as the document resolved it, decrypted or refused.
fn content_streams(document: &Document) -> Vec<Object> {
    let catalog = document.catalog().expect("a catalogue");
    let pages = document.get_key(&catalog, "Pages");
    let kids = document.get_key(pages.as_dict().expect("a page tree"), "Kids");
    let page = document.resolve(kids.as_array().and_then(<[Object]>::first).expect("a kid"));
    let contents = document.get_key(page.as_dict().expect("a page"), "Contents");
    contents
        .as_array()
        .expect("an array of two streams")
        .iter()
        .map(|item| document.resolve(item))
        .collect()
}

/// Table 27's second case: a crypt filter a stream's `/Crypt` decode parameters name, with a
/// recipient list and so a key of its own.
///
/// No document this tree holds uses one, so three are written for these tests by a script that
/// states its own algorithm (`gen_stream_recipients.py`, kept with the round that wrote it). Each
/// is `/SubFilter /adbe.pkcs7.s5` at `/V` 4 with two `AESV2` filters. `/DefaultCryptFilter` is
/// `/StmF` and `/StrF`, its `/Recipients` an array of one CMS object over a seed and Table 24's
/// word, made by `openssl cms -encrypt` for `testnutzer`'s certificate. `/OwnRecipients` is named
/// only by object 8's `/Crypt` decode parameters, and its `/Recipients` is one string, a CMS
/// object over a 20-byte seed alone — Table 27: "\[t]he enveloped data contained in the CMS object
/// shall be a 20-byte seed value", and §7.6.5.3: "the 4 bytes of permissions shall be absent".
/// Each key is §7.6.5.3's SHA-1 over its seed and its own CMS object, and each stream is
/// Algorithm 1 under its own key. So what each stream must decrypt to is the text the script
/// wrote, and no reader's output is consulted.
#[test]
fn a_stream_s_own_crypt_filter_decrypts_under_its_own_recipients_key() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/public_key_witnesses");
    let bytes = std::fs::read(directory.join("stream-recipients.pdf")).expect("a fixture");
    let document =
        Document::open_with_recipient(bytes, Limits::DEFAULT, &recipient_key("testnutzer"))
            .expect("testnutzer is a recipient of both lists");
    let content = page_content(&document);
    assert!(
        holds(&content, "(Document key) Tj"),
        "object 4, the document's key"
    );
    assert!(
        holds(&content, "(Own recipients) Tj"),
        "object 8, the filter's own key"
    );
    // The document's permissions are the document list's: §7.6.5.3 puts none in a stream's.
    let permissions = document.permissions().expect("encrypted");
    assert!(permissions.print && permissions.copy && !permissions.modify);
}

/// The same construction with `/OwnRecipients` enveloped for `test`'s certificate rather than
/// `testnutzer`'s. §7.6.6: "PDF readers and security handlers shall treat any attempt to access a
/// stream for which authorization has failed as an error", and Table 25's `/AuthEvent` decides
/// when that failure is met: at `EFOpen` the stream refuses on its own and the rest of the page
/// reads; at `DocOpen`, the default, "\[i]f authorization fails, the event shall fail", and the
/// event is the document's opening.
#[test]
fn a_reader_the_stream_s_own_list_does_not_name_is_refused_that_stream() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/public_key_witnesses");
    let read = |name: &str| std::fs::read(directory.join(name)).expect("a fixture");

    let document = Document::open_with_recipient(
        read("stream-recipients-other-efopen.pdf"),
        Limits::DEFAULT,
        &recipient_key("testnutzer"),
    )
    .expect("the document's own list names testnutzer, and the filter defers to EFOpen");
    let streams = content_streams(&document);
    let [first, second] = streams.as_slice() else {
        panic!("two content streams");
    };
    let first = first.as_stream().expect("a stream");
    assert!(!first.decryption_failed);
    assert!(holds(
        &document.decoded_stream_data(first).expect("it decodes"),
        "(Document key) Tj"
    ));
    let second = second.as_stream().expect("a stream");
    assert!(
        second.decryption_failed,
        "the filter's own list does not name testnutzer"
    );

    // `test` holds the stream's key but is not on the document's list, which /StmF needs at open.
    assert!(matches!(
        Document::open_with_recipient(
            read("stream-recipients-other-efopen.pdf"),
            Limits::DEFAULT,
            &recipient_key("test"),
        ),
        Err(SyntaxError::RecipientRequired)
    ));
    // At `DocOpen` the filter's own key is wanted at the open, and testnutzer cannot supply it.
    assert!(matches!(
        Document::open_with_recipient(
            read("stream-recipients-other-docopen.pdf"),
            Limits::DEFAULT,
            &recipient_key("testnutzer"),
        ),
        Err(SyntaxError::RecipientRequired)
    ));
}
