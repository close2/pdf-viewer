//! ISO 32000-2 §7.6.5's witness set: the documents a public-key build is tested against.
//!
//! # Why this file exists before the build does
//!
//! §7.6.5 is refused by name (ADR 1134) and waits for the trigger `doc/questions/A66` names.
//! `doc/questions/A168` rules that the documents below are **not** that trigger — a published
//! test key is held by nobody in particular — and makes them the end-to-end witness set the
//! build is tested against when a real trigger fires. This file is where that set is written
//! down as something that fails: each document's `/Recipients` names the certificate of a
//! keystore the `PDFBox` corpus ships beside it, and a later build extends each case from "the
//! recipient is this keystore's certificate" to "and the plaintext `PDFBox`'s own test states
//! comes out".
//!
//! §7.6.5.1 is the reader's one `shall` here, and it is the step these tests stop short of: "the
//! PDF reader shall scan the recipient list for which the content is encrypted and shall attempt
//! to find a match with a certificate that belongs to the user." Table 27 puts the list in the
//! crypt filter dictionary for `/SubFilter /adbe.pkcs7.s5`, and each item is a CMS
//! `EnvelopedData` (RFC 5652 section 6.1) whose recipient is named by issuer and serial number.
//!
//! # Where each side of the comparison comes from
//!
//! **The recipients** are read by this crate's own parser — the cross-reference table, the
//! encryption dictionary as an indirect object, its `/CF` — and walked as DER by the short
//! reader below. It is a test-local reader rather than `pdf_signature::der` because that crate
//! sits *above* this one, and a dev-dependency upward is the cycle `doc/crate-map.md` forbids;
//! `A66`'s shared crate below both is where the build moves it.
//!
//! **The certificates** are evidence, not this tree's reading. Both keystores keep their
//! certificate bag under `pbeWithSHA1And40BitRC2-CBC` (RFC 7292 appendix C), a cipher nothing in
//! this tree implements, so the issuer and serial below were read with
//! `openssl pkcs12 -legacy -nokeys -in <keystore> -passin pass:<password> | openssl x509 -noout
//! -issuer -serial`, with the passwords `PDFBox`'s `TestPublicKeyEncryption` passes: for
//! `PDFBOX-4421-keystore.pfx`, `CN=testnutzer`, serial `5F609C62`; for `PDFBOX-5249.p12` (empty
//! password), `CN=test`, serial `60FFD550`. Each keystore holds one certificate, self-issued.
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
    reason = "test code: a witness that cannot be read must fail loudly"
)]

use std::path::{Path, PathBuf};

use pdf_syntax::xref::{self, Location};
use pdf_syntax::{FileBytes, Limits, Object, Parser};

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
