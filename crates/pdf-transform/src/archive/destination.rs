//! A destination profile whose header misstates it, restated as what its own bytes are
//! (`doc/adr/2021`).
//!
//! # Two rows, one header
//!
//! ISO 19005-2 and ISO 19005-4 section 6.2.3 require the value of a PDF/A output intent's
//! `DestOutputProfile` to be a valid ICC profile stream, and the validator holds the profile to the
//! edition its own header's version names. Two of the rows that reading reports —
//! `graphics/destination-profile-carries-the-tags-its-class-requires` and
//! `graphics/destination-profile-states-a-correct-profile-id` — can fail on the header alone, and
//! then a rewrite of the header answers them without moving a colour:
//!
//! - **A version 5 header on a profile written in ICC.1:1998-09's terms.** ICC.2:2023 builds a
//!   display or an output profile from its `AToB`/`DToB` and `BToA`/`BToD` tables and defines no
//!   matrix-and-curve form, so such a profile fails ICC.2's tag row. It is nonetheless the profile
//!   this reader draws: `pdf_model::icc::Profile::parse` takes the transform from the tags and
//!   consults the version for one thing only, how a lookup table encodes CIELAB. So the version is
//!   restated as 2.2.0, the number ICC.1:1998-09 gives itself, where every tag that edition defines
//!   carries the type it gives (`pdf_archive::icc_written_in_the_1998_edition`); bytes 84 to 99,
//!   reserved in that edition and the `Profile ID` in later ones, are cleared, because a digest
//!   there names an edition the profile is not in.
//! - **A stale `Profile ID`** under an edition whose tags the profile does carry: written as the
//!   digest of the profile's own bytes, by ICC.1:2022 section 7.2.18's method, which ICC.2:2023
//!   section 7.2.20 repeats (`pdf_model::icc::computed_id`). No transform reads the field.
//!
//! Every other shortfall is refused by name: a tag missing under the profile's own edition is the
//! producer's to supply, and a version 5 profile written in anything but the 1998 edition's types
//! is not one this module can name an edition for.

use std::sync::Arc;

use pdf_model::icc::{Identification, computed_id, stated_id};
use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId, Stream};
use pdf_syntax::serialize::flate_encode;

use super::decision::Because;

/// The destination profile every output intent names, rewritten in its header alone.
#[derive(Debug)]
pub(super) struct RestatedDestination {
    /// The profile stream every entry of the catalog's array names, in the source's numbering.
    pub(super) at: ObjectId,
    /// The stream written there: the producer's dictionary, the restated profile.
    pub(super) written: Object,
}

/// ICC.1:1998-09 section 6.1.3's own version number, 2.2.0, as the header's bytes 8 to 11 hold
/// it: the major version, then the minor version and bug-fix level in one byte, then two zeros.
const VERSION_1998: [u8; 4] = [0x02, 0x20, 0x00, 0x00];

/// Where the `Profile ID` sits in the header, ICC.1:2022 section 7.2.18.
const PROFILE_ID: std::ops::Range<usize> = 84..100;

/// The tags whose data a lookup table holds, in every held edition's naming.
const LOOKUP_TABLE_TAGS: [[u8; 4]; 10] = [
    *b"A2B0", *b"A2B1", *b"A2B2", *b"B2A0", *b"B2A1", *b"B2A2", *b"gamt", *b"pre0", *b"pre1",
    *b"pre2",
];

/// Why a destination profile whose stream does not decode is not restated.
const UNREADABLE: &str = "the destination profile every output intent names does not decode, so \
     there is no header to restate and no tag to judge it by (doc/adr/2021)";

/// Why a profile short of a tag under its own edition is not restated.
const SHORT_UNDER_ITS_EDITION: &str = "the destination profile this file's own output intent \
     carries lacks a tag the ICC edition its header names requires of a profile of its class. \
     Supplying the tag would be writing the producer's profile for them, and a different edition \
     is named only where the profile is written in that edition's own tags and types, so the \
     requirement is refused (doc/adr/2021)";

/// Why a version 5 profile not written in ICC.1:1998-09's terms is not restated.
const NOT_THE_1998_EDITION: &str = "the destination profile this file's own output intent \
     carries names ICC.2:2023 in its header and is not built the way that edition builds a \
     display or an output profile, nor written in ICC.1:1998-09's tags and types, so there is \
     no edition its own bytes are in for the header to name, and the requirement is refused \
     (doc/adr/2021)";

/// Why a version 5 profile with CIELAB lookup tables is not restated.
const LAB_TABLES: &str = "the destination profile this file's own output intent carries is \
     written in ICC.1:1998-09's tags and types under a version 5 header, but its connection \
     space is CIELAB and it holds lookup tables, whose CIELAB encoding the version decides: \
     restating the version would change the colour this reader draws through it, so the \
     requirement is refused (doc/adr/2021)";

/// The restatement, where the profile's own bytes say what its header should.
pub(super) fn restated(
    document: &Document,
    catalog: &Dictionary,
) -> Result<RestatedDestination, Because> {
    let (at, stream) = super::prepare::the_destination_object(document, catalog)?;
    let data = document
        .decoded_stream_data(&stream)
        .ok_or(Because::NotBuiltYet(UNREADABLE))?;
    let profile = restated_profile(&data)?;
    Ok(RestatedDestination {
        at,
        written: with_profile(&stream, &profile).ok_or(Because::NotBuiltYet(UNREADABLE))?,
    })
}

/// The profile's bytes with the header saying what the profile is, or why it cannot.
fn restated_profile(data: &[u8]) -> Result<Vec<u8>, Because> {
    let stated = Identification::read(data).ok_or(Because::NotBuiltYet(UNREADABLE))?;
    let mut profile = data.to_vec();
    if pdf_archive::icc_version_tag_shortfall(data).is_none() {
        let computed = computed_id(data).ok_or(Because::NotBuiltYet(UNREADABLE))?;
        if let Some(field) = profile.get_mut(PROFILE_ID) {
            field.copy_from_slice(&computed);
        }
    } else if stated.version.0 == 5 {
        if !pdf_archive::icc_written_in_the_1998_edition(data) {
            return Err(Because::NotBuiltYet(NOT_THE_1998_EDITION));
        }
        let lab_connection = data.get(20..24) == Some(b"Lab ");
        let tables = LOOKUP_TABLE_TAGS
            .iter()
            .any(|signature| pdf_model::icc::has_tag(data, *signature));
        if lab_connection && tables {
            return Err(Because::NotBuiltYet(LAB_TABLES));
        }
        if let Some(version) = profile.get_mut(8..12) {
            version.copy_from_slice(&VERSION_1998);
        }
        if let Some(field) = profile.get_mut(PROFILE_ID) {
            field.fill(0);
        }
    } else {
        return Err(Because::NotBuiltYet(SHORT_UNDER_ITS_EDITION));
    }
    // The guard where the restatement acts: the validator's own reading of the result, so that a
    // profile this rewrites is one the two rows then pass (habit 115).
    let digest_holds = stated_id(&profile).is_none_or(|id| computed_id(&profile) == Some(id));
    if pdf_archive::icc_version_tag_shortfall(&profile).is_some() || !digest_holds {
        return Err(Because::NotBuiltYet(SHORT_UNDER_ITS_EDITION));
    }
    Ok(profile)
}

/// The producer's profile stream with `profile` as its data, compressed as every stream this verb
/// writes is.
fn with_profile(stream: &Stream, profile: &[u8]) -> Option<Object> {
    let encoded = flate_encode(profile, super::COMPRESSION_LEVEL)?;
    let mut dict = stream.dict.clone();
    dict.remove("DecodeParms");
    dict.remove("Length");
    dict.insert(
        Name::new(&b"Filter"[..]),
        Object::Name(Name::new(&b"FlateDecode"[..])),
    );
    Some(Object::Stream(Arc::new(Stream {
        dict,
        data: encoded.into(),
        decryption_failed: false,
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A version 2 matrix-and-curve display profile: the shipped sRGB profile, every tag of which
    /// is one ICC.1:1998-09 defines with the type it gives, or one it does not define.
    fn shipped() -> Vec<u8> {
        include_bytes!("../../../../data/icc/sRGB2014.icc").to_vec()
    }

    #[test]
    fn a_version_five_header_on_a_1998_profile_is_restated_as_that_edition() {
        let mut profile = shipped();
        profile[8..12].copy_from_slice(&[5, 0, 0, 0]);
        assert!(pdf_archive::icc_version_tag_shortfall(&profile).is_some());
        let restated = restated_profile(&profile).expect("restated");
        assert_eq!(restated[8..12], VERSION_1998);
        assert_eq!(restated[PROFILE_ID], [0; 16]);
        assert!(pdf_archive::icc_version_tag_shortfall(&restated).is_none());
        // Nothing but the version and the reserved field moved.
        assert_eq!(restated[..8], profile[..8]);
        assert_eq!(restated[12..84], profile[12..84]);
        assert_eq!(restated[100..], profile[100..]);
    }

    #[test]
    fn a_stale_identifier_under_an_edition_that_defines_it_is_recomputed() {
        let mut profile = shipped();
        profile[8..12].copy_from_slice(&[4, 0x40, 0, 0]);
        profile[84] ^= 0xff;
        let restated = restated_profile(&profile).expect("restated");
        assert_eq!(stated_id(&restated), computed_id(&restated));
        assert_eq!(restated[..84], profile[..84]);
        assert_eq!(restated[100..], profile[100..]);
    }

    #[test]
    fn a_version_five_profile_written_in_later_types_is_refused() {
        let mut profile = shipped();
        profile[8..12].copy_from_slice(&[5, 0, 0, 0]);
        // The description tag's data opened as a multiLocalizedUnicodeType, which ICC.1:1998-09
        // does not define for it.
        tag_type(&mut profile, *b"desc").copy_from_slice(b"mluc");
        assert_eq!(
            restated_profile(&profile).err(),
            Some(Because::NotBuiltYet(NOT_THE_1998_EDITION))
        );
    }

    #[test]
    fn a_profile_short_of_a_tag_under_its_own_edition_is_refused() {
        let mut profile = shipped();
        profile[8..12].copy_from_slice(&[4, 0x40, 0, 0]);
        tag_entry(&mut profile, *b"rXYZ")[..4].copy_from_slice(b"zzzz");
        assert_eq!(
            restated_profile(&profile).err(),
            Some(Because::NotBuiltYet(SHORT_UNDER_ITS_EDITION))
        );
    }

    /// The tag table entry for `signature`: twelve bytes of signature, offset and size.
    fn tag_entry(profile: &mut [u8], signature: [u8; 4]) -> &mut [u8] {
        let count = u32::from_be_bytes(profile[128..132].try_into().expect("four bytes"));
        profile[132..]
            .chunks_exact_mut(12)
            .take(usize::try_from(count).expect("a count"))
            .find(|entry| entry[..4] == signature)
            .expect("the profile states the tag")
    }

    /// The four bytes that open the data of the tag `signature`: its type signature.
    fn tag_type(profile: &mut [u8], signature: [u8; 4]) -> &mut [u8] {
        let entry = tag_entry(profile, signature);
        let offset = u32::from_be_bytes(entry[4..8].try_into().expect("four bytes"));
        let offset = usize::try_from(offset).expect("an offset");
        profile[offset..].get_mut(..4).expect("the tag's data")
    }
}
