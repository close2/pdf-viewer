//! §7.6.5.1's reader identity — a certificate and its private key — held so that the key cannot
//! be printed and does not outlive its use.
//!
//! # Why this is not [`crate::Secret`]
//!
//! [`crate::Secret`] is §7.6.4.1's password: text a person types a character at a time, sized by
//! the standard's 127-byte cut, and read by a prompt that draws a bullet per character. A private
//! key is none of that. It is bytes a host read off a disk whole — RFC 5958's `OneAsymmetricKey`,
//! RFC 8017's `RSAPrivateKey` or RFC 5915's `ECPrivateKey`, which are DER and not UTF-8 — and it
//! arrives with the certificate it belongs to. So it is its own type, and it carries the two of
//! [`crate::Secret`]'s three obligations that apply to it (ADR 1134 decision 2, ADR 2822):
//!
//! - **it does not print.** [`Debug`] says how long the certificate and the key are and nothing of
//!   either; the one way to read the key is [`RecipientSecret::reveal_key`], named so that the
//!   moment it happens is visible at its call site;
//! - **it does not linger.** The key's buffer is overwritten when the value is dropped, with
//!   [`crate::Secret`]'s best-effort `black_box` and for its reason: a volatile write is `unsafe`
//!   code, which this crate forbids.
//!
//! The third, "it does not grow into a second copy", is about a buffer edited in place, and this
//! one is handed over whole and never edited.
//!
//! # Why it is held for the window and not sent with each open
//!
//! §7.6.5.1 has the reader "find a match with a certificate that belongs to the user": the key is
//! a fact about *who is reading*, like [`crate::Command::Trust`]'s anchors and
//! [`crate::Command::Audience`]'s names, and it opens every document enveloped to that reader. A
//! password belongs to one file and is asked for per file; a key is supplied once. So
//! [`crate::Command::Recipient`] hands it to the viewer, which holds it until a host sends another
//! or none, and an open reads it only after the file has said it needs one — a document that is
//! not encrypted to recipients never touches it (ADR 2822).

/// The reader's own certificate and private key, for §7.6.5.1's match.
///
/// Built by a host from bytes it read itself — the confined worker names no path (`CLAUDE.md`
/// principle 3) — and handed to [`crate::Command::Recipient`]. Neither half is checked here:
/// [`pdf_syntax::RecipientKey::new`] reads both when a document asks for them, and a key that does
/// not read is said then, as [`pdf_syntax::SyntaxError::RecipientKey`].
///
/// **Not [`Clone`]**, for [`crate::Secret`]'s reason: a copy is a second buffer to clear.
pub struct RecipientSecret {
    /// The certificate, X.509's DER. Public: it is what a recipient list names.
    certificate: Vec<u8>,
    /// The private key's DER, in one of the three encodings `pdf_pki::key` reads.
    key: Vec<u8>,
}

impl RecipientSecret {
    /// A certificate and the private key that belongs to it, both DER.
    ///
    /// The vectors move in rather than being copied, so there is one buffer for the key; what
    /// this cannot do is clear a copy the caller made before handing it over.
    #[must_use]
    pub fn new(certificate: Vec<u8>, key: Vec<u8>) -> Self {
        Self { certificate, key }
    }

    /// The certificate's DER.
    #[must_use]
    pub fn certificate(&self) -> &[u8] {
        &self.certificate
    }

    /// The private key's DER.
    ///
    /// Named for what it does: the callers that should reach for it are the one about to hand
    /// the key to [`pdf_syntax::RecipientKey::new`] and the confined protocol's encoder, which
    /// carries it into the worker where the document's bytes are.
    #[must_use]
    pub fn reveal_key(&self) -> &[u8] {
        &self.key
    }
}

/// The two lengths, and not one byte of the key.
impl std::fmt::Debug for RecipientSecret {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "RecipientSecret(certificate {} byte(s), key {} byte(s))",
            self.certificate.len(),
            self.key.len()
        )
    }
}

impl Drop for RecipientSecret {
    fn drop(&mut self) {
        // The key's own allocation, cleared in place; `black_box` is what stops the write being
        // removed as dead, as in `Secret::drop`. The certificate is public and left as it is.
        self.key.fill(0);
        std::hint::black_box(&mut self.key);
    }
}

#[cfg(test)]
mod tests {
    use super::RecipientSecret;

    /// The key's bytes never reach a trace: the derived-`Debug` hazard `crate::secret` describes.
    #[test]
    fn debug_names_the_lengths_and_not_the_key() {
        let secret = RecipientSecret::new(vec![0x30, 0x01, 0x00], b"PRIVATE-KEY-BYTES".to_vec());
        let printed = format!("{secret:?}");
        assert_eq!(
            printed,
            "RecipientSecret(certificate 3 byte(s), key 17 byte(s))"
        );
        assert!(!printed.contains("PRIVATE"));
    }
}
