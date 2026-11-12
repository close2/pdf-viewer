//! An elliptic-curve private key, and SEC 1's Diffie–Hellman primitive over it.
//!
//! ISO 32000-2 §7.6.5.3 has the key that encrypts the enveloped data "encrypted for each
//! recipient, using that recipient's public key", and RFC 5652 section 6.2.2 is how that is done
//! for a recipient whose public key is for key agreement rather than encryption: the sender
//! combines an ephemeral private key with the recipient's public key into a pairwise key, and the
//! recipient recovers the same pairwise key from its own private key and the sender's public key.
//! RFC 5753 section 3.1 fixes that combination for elliptic curves as SEC 1's "Elliptic Curve
//! Diffie-Hellman Primitive" (section 3.3.1), or as the cofactor form of NIST SP 800-56A, which
//! differs from it only on a curve whose cofactor is not 1 — and every curve this module computes
//! on has cofactor 1 (SEC 2's `h = 01` for P-256, P-384 and P-521; RFC 5639 section 3's `h = 1` for
//! the Brainpool curves), so one primitive serves both.
//!
//! # The private key is an input, and it is a secret
//!
//! It arrives as bytes a host supplied, never from the document (ADR 1134): RFC 5915's
//! `ECPrivateKey`, or that structure inside RFC 5958's `OneAsymmetricKey` (RFC 5208's
//! `PrivateKeyInfo`) as RFC 5915 section 1 describes, with `id-ecPublicKey` or `id-ecDH` and the
//! curve as its algorithm. The scalar is held in `elliptic_curve`'s `SecretKey`, which keeps it
//! out of every `Debug` and zeroes it when dropped, and the multiplication is the curve
//! package's constant-time one.
//!
//! # Which curves
//!
//! The five of ISO/TS 32002 Table 3's six that this program already computes on for signatures:
//! P-256, P-384 and P-521, and brainpoolP256r1 and brainpoolP384r1, each from its reviewed
//! `RustCrypto` package (`doc/stack.md`). brainpoolP512r1 has no such package, and a key on it is
//! refused by name, as `pdf_signature::ecdsa` refuses a signature on it (ADR 2821).

use p256::elliptic_curve::group::Curve as _;
use p256::elliptic_curve::point::AffineCoordinates;
use p256::elliptic_curve::sec1::{FromSec1Point, ModulusSize, ToSec1Point};
use p256::elliptic_curve::{
    AffinePoint, CurveArithmetic, FieldBytes, FieldBytesSize, ProjectivePoint, PublicKey, SecretKey,
};

use crate::der::{DerError, INTEGER, OCTET_STRING, Reader, SEQUENCE, Value};
use crate::x509::EcCurve;

/// RFC 5480 section 2.1.1's `id-ecPublicKey`, the unrestricted algorithm identifier.
const ID_EC_PUBLIC_KEY: &[u8] = const_oid::db::rfc5912::ID_EC_PUBLIC_KEY.as_bytes();
/// RFC 5480 section 2.1.2's `id-ecDH`, a key "restricted to" Elliptic Curve Diffie-Hellman.
const ID_EC_DH: &[u8] = const_oid::db::rfc5912::ID_EC_DH.as_bytes();

/// SEC 1 section 2.3.3's leading octets this module accepts on a point: 0x02 and 0x03 for the
/// compressed form, 0x04 for the uncompressed one.
///
/// RFC 5753 section 7.2: "Implementations MUST support uncompressed keys, MAY support compressed
/// keys, and MUST NOT support hybrid keys" — the hybrid form's 0x06 and 0x07 are refused here
/// rather than left to whatever the point decoder accepts.
const POINT_FORMS: [u8; 3] = [0x02, 0x03, 0x04];

/// Why an elliptic-curve private key could not be read or used.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum EcError {
    /// The bytes are not DER this crate can read.
    #[error("the private key does not parse as DER: {0}")]
    Der(DerError),
    /// The bytes are DER but neither RFC 5915's `ECPrivateKey` nor RFC 5958's
    /// `OneAsymmetricKey` around one; the string names the member at fault.
    #[error("the private key is not RFC 5915's ECPrivateKey: {0}")]
    Malformed(&'static str),
    /// The key is another algorithm's: RFC 8017's RSA, or a `OneAsymmetricKey` naming neither
    /// `id-ecPublicKey` nor `id-ecDH`.
    #[error("the private key is not an elliptic-curve key")]
    NotEc,
    /// The key is on a curve this program does not compute on; the string is its name, or its
    /// object identifier in dotted form.
    #[error("the private key is on {0}, which this reader does not compute on")]
    Curve(String),
    /// RFC 5915's `privateKey` is not an integer from 1 to the curve's order less one.
    #[error("the private key's scalar is out of range for its curve")]
    Scalar,
    /// The other party's point is not SEC 1's encoding of a point on this key's curve other than
    /// the identity, or is in the hybrid form RFC 5753 section 7.2 forbids.
    #[error("the public point is not a point on the private key's curve")]
    Point,
}

impl From<DerError> for EcError {
    fn from(error: DerError) -> Self {
        Self::Der(error)
    }
}

/// The scalar, typed by its curve so that the arithmetic is the curve package's own.
enum Scalar {
    /// P-256.
    P256(SecretKey<p256::NistP256>),
    /// P-384.
    P384(SecretKey<p384::NistP384>),
    /// P-521.
    P521(SecretKey<p521::NistP521>),
    /// brainpoolP256r1.
    BrainpoolP256r1(SecretKey<bp256::BrainpoolP256r1>),
    /// brainpoolP384r1.
    BrainpoolP384r1(SecretKey<bp384::BrainpoolP384r1>),
}

/// An elliptic-curve private key: its curve and its scalar.
pub struct PrivateKey {
    /// The curve the key's parameters name.
    curve: EcCurve,
    /// The secret scalar `d`.
    scalar: Scalar,
}

impl std::fmt::Debug for PrivateKey {
    /// The curve and nothing else: the scalar is the secret.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PrivateKey")
            .field("curve", &self.curve.name())
            .finish_non_exhaustive()
    }
}

impl PrivateKey {
    /// Reads RFC 5915's `ECPrivateKey`, bare or inside RFC 5958's `OneAsymmetricKey`.
    ///
    /// The three structures a host may hand over are told apart by their second member: a
    /// `OneAsymmetricKey` follows its version with the `privateKeyAlgorithm` `SEQUENCE`, an
    /// `ECPrivateKey` with the `privateKey` `OCTET STRING`, and RFC 8017's `RSAPrivateKey` with
    /// the modulus `INTEGER`.
    ///
    /// # Errors
    ///
    /// [`EcError::Der`] or [`EcError::Malformed`] for bytes that are none of the structures,
    /// [`EcError::NotEc`] for another algorithm's key, [`EcError::Curve`] for a curve this
    /// program does not compute on, and [`EcError::Scalar`] for a scalar outside the curve's
    /// range.
    pub fn from_der(bytes: &[u8]) -> Result<Self, EcError> {
        let outer = single(bytes)?;
        let mut members = outer.children()?;
        let version = members.next_value()?.ok_or(EcError::Malformed("version"))?;
        if version.identifier != INTEGER {
            return Err(EcError::Malformed("version"));
        }
        let second = members
            .next_value()?
            .ok_or(EcError::Malformed("privateKey"))?;
        match second.identifier {
            SEQUENCE => {
                // RFC 5915 section 1: "id-ecPublicKey, id-ecDH, or id-ecMQV (from [RFC5480])
                // with the namedCurve as the parameters in the privateKeyAlgorithm field".
                // `id-ecMQV` restricts the key to ECMQV, which this reader does not run.
                let mut algorithm = second.children()?;
                let identifier = algorithm
                    .next_value()?
                    .and_then(|value| value.object_identifier())
                    .ok_or(EcError::Malformed("privateKeyAlgorithm"))?;
                if identifier != ID_EC_PUBLIC_KEY && identifier != ID_EC_DH {
                    return Err(EcError::NotEc);
                }
                let named = algorithm
                    .next_value()?
                    .and_then(|value| value.object_identifier())
                    .ok_or(EcError::Malformed("privateKeyAlgorithm's namedCurve"))?;
                // "b) ECPrivateKey in the PrivateKey field, which is an OCTET STRING."
                let inner = members
                    .next_value()?
                    .filter(|value| value.identifier == OCTET_STRING)
                    .ok_or(EcError::Malformed("PrivateKeyInfo.privateKey"))?;
                Self::from_ec_private_key(single(inner.contents)?, Some(named))
            }
            OCTET_STRING => Self::from_ec_private_key(outer, None),
            _ => Err(EcError::NotEc),
        }
    }

    /// RFC 5915 section 3's `ECPrivateKey`; `named` is the curve the enclosing
    /// `PrivateKeyInfo` states, where there is one.
    fn from_ec_private_key(key: Value<'_>, named: Option<&[u8]>) -> Result<Self, EcError> {
        if key.identifier != SEQUENCE {
            return Err(EcError::Malformed("ECPrivateKey"));
        }
        let mut members = key.children()?;
        // "version ... SHALL be set to ecPrivkeyVer1".
        let version = members
            .next_value()?
            .filter(|value| value.identifier == INTEGER)
            .ok_or(EcError::Malformed("version"))?;
        if version.contents != [1] {
            return Err(EcError::Malformed("version is not ecPrivkeyVer1"));
        }
        let private_key = members
            .next_value()?
            .filter(|value| value.identifier == OCTET_STRING)
            .ok_or(EcError::Malformed("privateKey"))?;
        // "parameters [0] ECParameters {{ NamedCurve }} OPTIONAL"; RFC 5480 section 2.1.1 permits
        // only the namedCurve choice. The optional `publicKey [1]` "exists as a convenience to
        // the consumer", and the scalar alone is the key, so it is read past.
        let mut parameters = None;
        while let Some(value) = members.next_value()? {
            if value.is_context(0) {
                parameters = Some(
                    single(value.contents)?
                        .object_identifier()
                        .ok_or(EcError::Malformed("parameters is not a namedCurve"))?,
                );
            }
        }
        let curve_oid = match (named, parameters) {
            (Some(outer), Some(inner)) if outer != inner => {
                return Err(EcError::Malformed(
                    "the ECPrivateKey's parameters name another curve than its PrivateKeyInfo",
                ));
            }
            (Some(oid), _) | (None, Some(oid)) => oid,
            // "implementations that conform to this document MUST always include the parameters
            // field", and without it nothing says which curve the scalar is for.
            (None, None) => return Err(EcError::Malformed("parameters, which names the curve")),
        };
        let curve = EcCurve::of(curve_oid).ok_or_else(|| {
            EcError::Curve(crate::x509::dotted(curve_oid).unwrap_or_else(|| "a curve".to_owned()))
        })?;
        let octets = private_key.contents;
        let scalar = match curve {
            EcCurve::P256 => Scalar::P256(secret(octets)?),
            EcCurve::P384 => Scalar::P384(secret(octets)?),
            EcCurve::P521 => Scalar::P521(secret(octets)?),
            EcCurve::BrainpoolP256r1 => Scalar::BrainpoolP256r1(secret(octets)?),
            EcCurve::BrainpoolP384r1 => Scalar::BrainpoolP384r1(secret(octets)?),
            EcCurve::BrainpoolP512r1 => return Err(EcError::Curve(curve.name().to_owned())),
        };
        Ok(Self { curve, scalar })
    }

    /// The curve the key is on.
    #[must_use]
    pub fn curve(&self) -> EcCurve {
        self.curve
    }

    /// SEC 1 section 3.3.1's Elliptic Curve Diffie-Hellman Primitive: `z`, the x-coordinate of
    /// `d · Q`, as section 2.3.5's field-element octet string, where `Q` is the other party's
    /// public point in section 2.3.3's encoding.
    ///
    /// Section 3.3.1's step 1 computes "P = (xP, yP) = h·dU·QV" for the cofactor form and
    /// "P = dU·QV" for the standard one; on these curves `h` is 1 and the two are one product.
    /// Step 2: "Check that P ≠ O. If P = O, output 'invalid' and stop" — and decoding `Q` refuses
    /// the identity and every point off the curve, which on a curve of cofactor 1 is section
    /// 3.2.2.1's whole validation of a public key.
    ///
    /// # Errors
    ///
    /// [`EcError::Point`] where `point` is not a point on this key's curve other than the
    /// identity, or is in the hybrid form.
    pub fn agree(&self, point: &[u8]) -> Result<Vec<u8>, EcError> {
        if !point.first().is_some_and(|form| POINT_FORMS.contains(form)) {
            return Err(EcError::Point);
        }
        match &self.scalar {
            Scalar::P256(secret) => agree_on(secret, point),
            Scalar::P384(secret) => agree_on(secret, point),
            Scalar::P521(secret) => agree_on(secret, point),
            Scalar::BrainpoolP256r1(secret) => agree_on(secret, point),
            Scalar::BrainpoolP384r1(secret) => agree_on(secret, point),
        }
    }
}

/// RFC 5915's `privateKey`, "an octet string of length ceiling (log2(n)/8)", as the curve's
/// scalar.
///
/// An encoder that dropped leading zero octets wrote the same integer in fewer octets, so a
/// shorter string is read as the integer it is; a longer one is no scalar of this curve.
fn secret<C: CurveArithmetic>(octets: &[u8]) -> Result<SecretKey<C>, EcError> {
    let mut bytes = FieldBytes::<C>::default();
    let width = bytes.len();
    let at = width
        .checked_sub(octets.len())
        .filter(|_| !octets.is_empty())
        .ok_or(EcError::Scalar)?;
    bytes[at..].copy_from_slice(octets);
    // `from_bytes` refuses zero and every value not below the order.
    SecretKey::<C>::from_bytes(&bytes).map_err(|_| EcError::Scalar)
}

/// The primitive, once the curve is a type: one function rather than five copies.
fn agree_on<C>(secret: &SecretKey<C>, point: &[u8]) -> Result<Vec<u8>, EcError>
where
    C: CurveArithmetic,
    AffinePoint<C>:
        FromSec1Point<C> + ToSec1Point<C> + AffineCoordinates<FieldRepr = FieldBytes<C>>,
    FieldBytesSize<C>: ModulusSize,
{
    let public = PublicKey::<C>::from_sec1_bytes(point).map_err(|_| EcError::Point)?;
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "a scalar multiplication on the curve's group, which has no overflow to side-effect"
    )]
    let product: ProjectivePoint<C> = public.to_projective() * *secret.to_nonzero_scalar();
    // A nonzero scalar below the order times a point of prime order is never the identity, so
    // step 2's check cannot fire on a validated point; it is kept because the clause states it.
    if bool::from(p256::elliptic_curve::group::Group::is_identity(&product)) {
        return Err(EcError::Point);
    }
    // SEC 1 section 2.3.5: the field element as an octet string of the field's width.
    let x: FieldBytes<C> = product.to_affine().x();
    Ok(x.to_vec())
}

/// The single DER value `bytes` holds.
fn single(bytes: &[u8]) -> Result<Value<'_>, EcError> {
    Reader::new(bytes)?
        .next_value()?
        .ok_or(EcError::Malformed("empty"))
}

#[cfg(test)]
mod tests;
