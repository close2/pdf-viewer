//! §7.6.5.1's reader identity: the certificate and private key a host reads off its own disk and
//! hands to the core, which hands them to the confined worker (ADR 1134 decision 2, ADR 2822).
//!
//! ISO 32000-2 §7.6.5.1: "the PDF reader shall scan the recipient list for which the content is
//! encrypted and shall attempt to find a match with a certificate that belongs to the user." Which
//! certificate belongs to the user is the one input to that scan no document can supply, so it is
//! asked here, once, beside the trust anchors, and the answer is *none* until a person names two
//! files. **The renderer never reads a key from a path** (`CLAUDE.md` principle 3): this function
//! runs in the unconfined host, and what crosses is bytes.

use std::path::Path;

use viewer_core::RecipientSecret;

use super::{base64, certificates_in};

/// The word a person types to name the certificate that belongs to them, for §7.6.5.1's match.
///
/// The value is one file, X.509's DER or a PEM `CERTIFICATE` block. **Not a user interface for
/// it**, on [`super::TRUST_ANCHORS`]'s rule: it is one policy value
/// [`viewer_core::Command::Recipient`] carries, supplied the way this host supplies every other.
pub const RECIPIENT_CERTIFICATE: &str = "--recipient-certificate";

/// The word a person types to name the private key that belongs to [`RECIPIENT_CERTIFICATE`].
///
/// The value is one file, in any of the encodings `pdf_pki::key` reads — RFC 5958's
/// `OneAsymmetricKey` (PKCS #8), RFC 8017's `RSAPrivateKey` or RFC 5915's `ECPrivateKey` — as DER
/// or as the PEM block each is written in (`PRIVATE KEY`, `RSA PRIVATE KEY`, `EC PRIVATE KEY`).
pub const RECIPIENT_KEY: &str = "--recipient-key";

/// What a window says for `Event::RecipientRequired`: the document is encrypted to recipients
/// and no certificate this reader supplied is one, and the two words that supply another.
///
/// **A sentence rather than a prompt, deliberately.** A password is typed into a card; a
/// certificate and its key are two files, and a chooser for them is a user interface
/// `doc/todo/38` does not build until the project owner asks, on [`super::TRUST_ANCHORS`]'s rule.
/// So the window answers §7.6.5.1's failed match the way it answers a document it could not open,
/// with the words that change the answer (ADR 2822).
#[must_use]
pub fn recipient_required() -> String {
    format!(
        "it is encrypted to recipients and no certificate supplied is one (ISO 32000-2 \
         §7.6.5.1): name the reader's certificate with {RECIPIENT_CERTIFICATE} and its private \
         key with {RECIPIENT_KEY}"
    )
}

/// Why the two files did not become a recipient key.
///
/// Typed, and every one said out loud (trap 5): a person who named a key and then reads "this
/// document needs a recipient key" is owed the sentence that says theirs was not taken.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RecipientRefusal {
    /// One of the two words was typed without the other; §7.6.5.1's match needs both.
    #[error("{given} was named without {missing}: a recipient is a certificate and its key")]
    OnlyOne {
        /// The word that was typed.
        given: &'static str,
        /// The word that was not.
        missing: &'static str,
    },
    /// A file could not be read.
    #[error("cannot read {path} ({word}): {error}")]
    FileUnreadable {
        /// The word that named it.
        word: &'static str,
        /// The file as the person named it.
        path: String,
        /// What the filesystem said.
        error: String,
    },
    /// The certificate file holds neither DER nor a PEM `CERTIFICATE` block, or holds several.
    #[error("{path} ({RECIPIENT_CERTIFICATE}) is not one DER or PEM certificate")]
    NotOneCertificate {
        /// The file as the person named it.
        path: String,
    },
    /// The key file holds a PEM `ENCRYPTED PRIVATE KEY`, which needs a passphrase this host does
    /// not ask for.
    #[error(
        "{path} ({RECIPIENT_KEY}) is an encrypted private key (RFC 5958 section 3), which this \
         host does not decrypt: write it out unencrypted"
    )]
    EncryptedKey {
        /// The file as the person named it.
        path: String,
    },
    /// The key file holds neither DER nor one of the three PEM private-key blocks.
    #[error("{path} ({RECIPIENT_KEY}) holds no private key in DER or PEM")]
    NotAKey {
        /// The file as the person named it.
        path: String,
    },
    /// Both files read, and `pdf_syntax` cannot use them: the certificate is not X.509 it reads,
    /// or the key is of neither family on a curve it computes on.
    #[error("{0}")]
    Unusable(String),
}

/// The three PEM labels a private key this reader takes is written under, in RFC 5958's,
/// RFC 8017's and RFC 5915's encodings — the three `pdf_pki::key` reads as DER.
const KEY_LABELS: [&str; 3] = ["PRIVATE KEY", "RSA PRIVATE KEY", "EC PRIVATE KEY"];

/// **The one place a host answers "which certificate belongs to this reader", and the answer is
/// *none* by default.**
///
/// `certificate` and `key` are what [`RECIPIENT_CERTIFICATE`] and [`RECIPIENT_KEY`] named. Both
/// unnamed is `None` with nothing to say, which is [`viewer_core::Command::Recipient`]'s default:
/// a document encrypted to recipients then raises `Event::RecipientRequired`. One without the
/// other, a file that does not read, or a pair `pdf_syntax::RecipientKey::new` refuses is `None`
/// with the refusal, read here rather than at the first document so that it is said at launch.
///
/// The key file's bytes are cleared once decoded, best effort and for `viewer_core::Secret`'s
/// reason; what the operating system's page cache holds of the file was never this program's.
#[must_use]
pub fn recipient_key(
    certificate: Option<&Path>,
    key: Option<&Path>,
) -> (Option<RecipientSecret>, Vec<RecipientRefusal>) {
    let (certificate_path, key_path) = match (certificate, key) {
        (None, None) => return (None, Vec::new()),
        (Some(_), None) => {
            return (
                None,
                vec![RecipientRefusal::OnlyOne {
                    given: RECIPIENT_CERTIFICATE,
                    missing: RECIPIENT_KEY,
                }],
            );
        }
        (None, Some(_)) => {
            return (
                None,
                vec![RecipientRefusal::OnlyOne {
                    given: RECIPIENT_KEY,
                    missing: RECIPIENT_CERTIFICATE,
                }],
            );
        }
        (Some(certificate), Some(key)) => (certificate, key),
    };
    match read_pair(certificate_path, key_path) {
        Ok(secret) => (Some(secret), Vec::new()),
        Err(refusal) => (None, vec![refusal]),
    }
}

/// The two files, read and decoded, and checked by the reader that will use them.
fn read_pair(certificate: &Path, key: &Path) -> Result<RecipientSecret, RecipientRefusal> {
    let read = |word: &'static str, path: &Path| {
        std::fs::read(path).map_err(|error| RecipientRefusal::FileUnreadable {
            word,
            path: path.display().to_string(),
            error: error.to_string(),
        })
    };
    let certificate_bytes = read(RECIPIENT_CERTIFICATE, certificate)?;
    let mut found = certificates_in(&certificate_bytes);
    let (Some(certificate_der), true) = (found.pop(), found.is_empty()) else {
        return Err(RecipientRefusal::NotOneCertificate {
            path: certificate.display().to_string(),
        });
    };
    let mut key_bytes = read(RECIPIENT_KEY, key)?;
    let key_der = key_in(&key_bytes, key);
    key_bytes.fill(0);
    std::hint::black_box(&mut key_bytes);
    let secret = RecipientSecret::new(certificate_der, key_der?);
    // Read once here so that a pair that cannot be used is said at launch, by the parser the
    // worker will run, rather than at the first document that asks for it.
    pdf_syntax::RecipientKey::new(secret.certificate().to_vec(), secret.reveal_key())
        .map_err(|error| RecipientRefusal::Unusable(error.to_string()))?;
    Ok(secret)
}

/// The private key one file holds, as DER: the file itself, or its one PEM private-key block.
///
/// X.690's `SEQUENCE` tag `0x30` begins all three encodings, so a file starting with it is offered
/// whole, for [`certificates_in`]'s reason: `pdf_pki::key` is what decides whether it is a key.
fn key_in(bytes: &[u8], path: &Path) -> Result<Vec<u8>, RecipientRefusal> {
    if bytes.first() == Some(&0x30) {
        return Ok(bytes.to_vec());
    }
    let refused = |encrypted: bool| {
        let path = path.display().to_string();
        if encrypted {
            RecipientRefusal::EncryptedKey { path }
        } else {
            RecipientRefusal::NotAKey { path }
        }
    };
    // Valid UTF-8 borrows rather than copies, so the text below is the buffer the caller clears.
    let text = String::from_utf8_lossy(bytes);
    if text.contains("-----BEGIN ENCRYPTED PRIVATE KEY-----") {
        return Err(refused(true));
    }
    for label in KEY_LABELS {
        let begin = format!("-----BEGIN {label}-----");
        let end = format!("-----END {label}-----");
        let Some(start) = text.find(&begin) else {
            continue;
        };
        let after = start.saturating_add(begin.len());
        let Some(stop) = text[after..].find(&end) else {
            continue;
        };
        return base64(&text[after..after.saturating_add(stop)]).ok_or_else(|| refused(false));
    }
    Err(refused(false))
}

#[cfg(test)]
mod tests {
    use super::{RECIPIENT_CERTIFICATE, RECIPIENT_KEY, RecipientRefusal, recipient_key};
    use std::path::{Path, PathBuf};

    /// The tree's own recipient fixtures: certificates and keys this tree generated (`ec-p256`,
    /// ADR 2821) and the `PDFBox` keystores' (`doc/third-party-data.md`).
    fn fixture(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../pdf-syntax/tests/public_key_witnesses")
            .join(name)
    }

    /// A scratch directory of this test's own, removed when it is dropped.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Self {
            let directory = std::env::temp_dir()
                .join(format!("quorra-recipient-{name}-{}", std::process::id()));
            std::fs::create_dir_all(&directory).expect("a scratch directory");
            Self(directory)
        }

        fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
            let path = self.0.join(name);
            std::fs::write(&path, bytes).expect("a scratch file");
            path
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            drop(std::fs::remove_dir_all(&self.0));
        }
    }

    /// RFC 7468 section 2's textual encoding: the label, base64 at 64 columns, the label again.
    fn pem(label: &str, der: &[u8]) -> Vec<u8> {
        const ALPHABET: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        // Where each of a group's three bytes sits in its 24 bits, and each of its four sextets.
        const BYTE_SHIFT: [u32; 3] = [16, 8, 0];
        const SEXTET_SHIFT: [u32; 4] = [18, 12, 6, 0];
        let mut encoded = String::new();
        for chunk in der.chunks(3) {
            let group = chunk
                .iter()
                .zip(BYTE_SHIFT)
                .fold(0_u32, |group, (byte, shift)| {
                    group | u32::from(*byte) << shift
                });
            for (index, shift) in SEXTET_SHIFT.into_iter().enumerate() {
                if index <= chunk.len() {
                    let sextet = usize::try_from((group >> shift) & 63).expect("six bits");
                    encoded.push(char::from(ALPHABET[sextet]));
                } else {
                    encoded.push('=');
                }
            }
        }
        let mut text = format!("-----BEGIN {label}-----\n");
        for line in encoded.as_bytes().chunks(64) {
            text.push_str(std::str::from_utf8(line).expect("ASCII"));
            text.push('\n');
        }
        text.push_str("-----END ");
        text.push_str(label);
        text.push_str("-----\n");
        text.into_bytes()
    }

    /// Nothing named is nothing supplied, and nothing said.
    #[test]
    fn no_word_is_no_key() {
        let (key, refused) = recipient_key(None, None);
        assert!(key.is_none());
        assert!(refused.is_empty());
    }

    /// DER on disk is handed over byte for byte, both halves.
    #[test]
    fn a_der_pair_is_handed_over_as_read() {
        let (key, refused) = recipient_key(
            Some(&fixture("ec-p256.certificate.der")),
            Some(&fixture("ec-p256.key.der")),
        );
        assert!(refused.is_empty(), "{refused:?}");
        let key = key.expect("a key");
        assert_eq!(
            key.certificate(),
            std::fs::read(fixture("ec-p256.certificate.der")).unwrap()
        );
        assert_eq!(
            key.reveal_key(),
            std::fs::read(fixture("ec-p256.key.der")).unwrap()
        );
    }

    /// PEM decodes to the same DER the PEM was written from: RFC 7468's `CERTIFICATE` and
    /// `PRIVATE KEY` labels, with text around the blocks skipped.
    #[test]
    fn a_pem_pair_decodes_to_its_der() {
        let scratch = Scratch::new("pem");
        let certificate = std::fs::read(fixture("testnutzer.certificate.der")).unwrap();
        let private = std::fs::read(fixture("testnutzer.key.der")).unwrap();
        let mut key_text = b"Bag Attributes: none\n".to_vec();
        key_text.extend(pem("PRIVATE KEY", &private));
        let (key, refused) = recipient_key(
            Some(&scratch.write("c.pem", &pem("CERTIFICATE", &certificate))),
            Some(&scratch.write("k.pem", &key_text)),
        );
        assert!(refused.is_empty(), "{refused:?}");
        let key = key.expect("a key");
        assert_eq!(key.certificate(), certificate);
        assert_eq!(key.reveal_key(), private);
    }

    /// Each refusal is said, by name, and no key goes on.
    #[test]
    fn what_cannot_be_a_recipient_is_refused_by_name() {
        let scratch = Scratch::new("refusals");
        let certificate = fixture("test.certificate.der");
        let (key, refused) = recipient_key(Some(&certificate), None);
        assert!(key.is_none());
        assert_eq!(
            refused,
            [RecipientRefusal::OnlyOne {
                given: RECIPIENT_CERTIFICATE,
                missing: RECIPIENT_KEY
            }]
        );
        let encrypted = scratch.write("e.pem", &pem("ENCRYPTED PRIVATE KEY", &[0x30, 0x00]));
        let (key, refused) = recipient_key(Some(&certificate), Some(&encrypted));
        assert!(key.is_none());
        assert!(
            matches!(refused.as_slice(), [RecipientRefusal::EncryptedKey { .. }]),
            "{refused:?}"
        );
        let text = scratch.write("t.txt", b"not a key");
        let (key, refused) = recipient_key(Some(&certificate), Some(&text));
        assert!(key.is_none());
        assert!(
            matches!(refused.as_slice(), [RecipientRefusal::NotAKey { .. }]),
            "{refused:?}"
        );
        // A key file that is DER of no key: `pdf_syntax` refuses it, and the refusal is its own.
        let nothing = scratch.write("n.der", &[0x30, 0x03, 0x02, 0x01, 0x00]);
        let (key, refused) = recipient_key(Some(&certificate), Some(&nothing));
        assert!(key.is_none());
        assert!(
            matches!(refused.as_slice(), [RecipientRefusal::Unusable(detail)]
                if detail.contains("private key")),
            "{refused:?}"
        );
    }
}
