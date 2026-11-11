//! The private key a recipient holds, whichever of RFC 5652's two certificate-bound recipient
//! kinds it opens.
//!
//! ISO 32000-2 §7.6.5.1's reader "shall attempt to find a match with a certificate that belongs
//! to the user", and the certificate's key decides how the match is opened: an RSA key opens a
//! key-transport recipient (RFC 5652 section 6.2.1, [`crate::rsa`]), and an elliptic-curve key a
//! key-agreement one (section 6.2.2, [`crate::ec`]). A host hands over one key in one of the
//! encodings both modules read, and this type is the one place that tells them apart, so that a
//! caller holds "the user's key" without knowing which family it is.

use crate::ec::{self, EcError};
use crate::rsa::{self, RsaError};

/// A recipient's private key.
#[derive(Debug)]
pub enum PrivateKey {
    /// An RSA key, RFC 8017's.
    Rsa(rsa::PrivateKey),
    /// An elliptic-curve key, RFC 5915's.
    Ec(ec::PrivateKey),
}

/// Why a private key could not be read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum KeyError {
    /// The bytes are an RSA key that does not read.
    #[error("{0}")]
    Rsa(RsaError),
    /// The bytes are an elliptic-curve key that does not read.
    #[error("{0}")]
    Ec(EcError),
    /// The bytes are neither family's key.
    #[error(
        "the private key is neither an RSA key (RFC 8017) nor an elliptic-curve key (RFC 5915)"
    )]
    Algorithm,
}

impl PrivateKey {
    /// Reads a private key in any of the encodings [`ec::PrivateKey::from_der`] and
    /// [`rsa::PrivateKey::from_der`] read: RFC 5958's `OneAsymmetricKey`, RFC 5915's
    /// `ECPrivateKey` or RFC 8017's `RSAPrivateKey`.
    ///
    /// # Errors
    ///
    /// [`KeyError::Ec`] or [`KeyError::Rsa`] for a key of that family that does not read, and
    /// [`KeyError::Algorithm`] for bytes that are neither family's.
    pub fn from_der(bytes: &[u8]) -> Result<Self, KeyError> {
        match ec::PrivateKey::from_der(bytes) {
            Ok(key) => Ok(Self::Ec(key)),
            // The elliptic-curve reader says "not mine" only after reading far enough to know
            // the structure is another family's, so every other refusal is about an EC key.
            Err(EcError::NotEc) => match rsa::PrivateKey::from_der(bytes) {
                Ok(key) => Ok(Self::Rsa(key)),
                Err(RsaError::NotRsa) => Err(KeyError::Algorithm),
                Err(error) => Err(KeyError::Rsa(error)),
            },
            Err(error) => Err(KeyError::Ec(error)),
        }
    }
}
