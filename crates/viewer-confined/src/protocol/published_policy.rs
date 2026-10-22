//! Published signature policy wire encoding and strict decoding.

use super::{ProtocolError, Reader, Writer};

/// One policy copy a signature names: its identifier and URL, the commitment, and the
/// specification. A digest crosses as its object identifier, which is what the signature stated
/// and what [`pdf_signature::cms::Digest::from_oid`] reads back (ADR 1738).
pub(super) fn write_published(
    writer: &mut Writer,
    policy: &pdf_signature::policy::PublishedPolicy,
) {
    use pdf_signature::policy::{Commitment, Specification};
    writer.str(&policy.identifier).str(&policy.url);
    match &policy.commitment {
        Commitment::Stated { digest, value } => {
            writer.u8(0).bytes(digest.oid()).bytes(value);
        }
        Commitment::UnderAnotherFunction { algorithm } => {
            writer.u8(1).str(algorithm);
        }
        Commitment::NotKnown => {
            writer.u8(2);
        }
    }
    match &policy.specification {
        None => {
            writer.u8(0);
        }
        Some(Specification::ObjectIdentifier(oid)) => {
            writer.u8(1).str(oid);
        }
        Some(Specification::Uri(uri)) => {
            writer.u8(2).str(uri);
        }
    }
}

/// [`write_published`]'s inverse; a digest this build does not compute is refused rather than
/// read as another, because the comparison a host makes is under the function the signer named.
pub(super) fn read_published(
    reader: &mut Reader<'_>,
) -> Result<pdf_signature::policy::PublishedPolicy, ProtocolError> {
    use pdf_signature::cms::Digest;
    use pdf_signature::policy::{Commitment, PublishedPolicy, Specification};
    let identifier = reader.string("a signature policy's identifier")?;
    let url = reader.string("a signature policy's URL")?;
    let commitment = match reader.u8("a signature policy's commitment")? {
        0 => {
            let oid = reader.bytes("a signature policy's digest function")?;
            // The identifier is octets rather than a number, so what the refusal names is how many
            // there were: no `Digest` writes an identifier this build cannot read back.
            let digest = Digest::from_oid(oid).ok_or(ProtocolError::Unrecognised {
                what: "a digest identifier of this many octets",
                value: u32::try_from(oid.len()).unwrap_or(u32::MAX),
            })?;
            Commitment::Stated {
                digest,
                value: reader.owned_bytes("a signature policy's digest")?,
            }
        }
        1 => Commitment::UnderAnotherFunction {
            algorithm: reader.string("a signature policy's digest function")?,
        },
        2 => Commitment::NotKnown,
        other => {
            return Err(ProtocolError::Unrecognised {
                what: "a signature policy's commitment",
                value: u32::from(other),
            });
        }
    };
    let specification = match reader.u8("a signature policy's specification")? {
        0 => None,
        1 => Some(Specification::ObjectIdentifier(
            reader.string("a specification's identifier")?,
        )),
        2 => Some(Specification::Uri(reader.string("a specification's URI")?)),
        other => {
            return Err(ProtocolError::Unrecognised {
                what: "a signature policy's specification",
                value: u32::from(other),
            });
        }
    };
    Ok(PublishedPolicy {
        identifier,
        url,
        commitment,
        specification,
    })
}
