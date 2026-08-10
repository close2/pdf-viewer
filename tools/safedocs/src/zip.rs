//! Enough of the ZIP format to take a few members out of an archive nobody may download.
//!
//! This reads the structures ISO/IEC 21320-1 and PKWARE's APPNOTE.TXT define, in the order a
//! random-access reader has to: the end-of-central-directory record at the tail, then the
//! central directory it points at, then — for the members a chunk names — the local file
//! header that sits immediately before each member's data.
//!
//! # Why not a ZIP crate
//!
//! Every ZIP crate in the ecosystem reads a `Read + Seek`, which over HTTP means either
//! downloading the object or writing a seekable adapter that issues range requests, and the
//! adapter is most of what is here anyway. What this needs is three structures and no
//! writing, encryption or archive-creation surface at all — so it is 200 lines rather than a
//! dependency, and every offset in it is checked arithmetic over untrusted input.
//!
//! # ZIP64
//!
//! Handled where the corpus needs it. The archives are 1 GB to 3.7 GB, so a 32-bit offset
//! still fits, but nothing here assumes that: the ZIP64 end-of-central-directory record is
//! read when the locator is present, and a central directory entry whose size or offset is
//! the `0xFFFF_FFFF` escape reads the real value out of extra field `0x0001`.

use crate::Error;

/// What the archive's own bytes could not be made to say.
#[derive(Debug, thiserror::Error)]
pub enum ArchiveError {
    /// No end-of-central-directory record in the tail that was read.
    #[error(
        "no ZIP end-of-central-directory record in the last {0} bytes: this object is not a \
         ZIP archive, or its trailing comment is longer than that"
    )]
    NoDirectory(usize),
    /// A structure ran off the end of the bytes holding it.
    #[error("the {structure} at offset {at} runs past the {available} bytes available")]
    Truncated {
        /// Which structure.
        structure: &'static str,
        /// Where it starts.
        at: u64,
        /// How many bytes there were.
        available: usize,
    },
    /// A signature that should have been there was not.
    #[error("expected a {structure} signature at offset {at} and found {found:#010x}")]
    BadSignature {
        /// Which structure.
        structure: &'static str,
        /// Where it should have been.
        at: u64,
        /// What was there instead.
        found: u32,
    },
    /// The archive is spread over more than one file.
    #[error("this archive is split across {0} disks, which this reader does not join")]
    Split(u32),
    /// A member the caller asked for is not in the archive.
    #[error("no member named {0:?} in this archive")]
    NoSuchMember(String),
    /// A compression method other than stored or deflate.
    #[error("member {name:?} uses compression method {method}, which this reader does not do")]
    UnknownMethod {
        /// The member.
        name: String,
        /// The method number APPNOTE.TXT assigns.
        method: u16,
    },
}

/// One entry of the central directory.
#[derive(Debug, Clone)]
pub struct Entry {
    /// The member's name inside the archive.
    pub name: String,
    /// Where its local file header begins in the object.
    pub local_header: u64,
    /// How many bytes its compressed data occupies.
    pub compressed: u64,
    /// How many bytes it inflates to.
    pub uncompressed: u64,
    /// The CRC-32 the archive records for the inflated bytes.
    pub crc32: u32,
    /// APPNOTE.TXT's compression method: 0 stored, 8 deflate.
    pub method: u16,
}

/// An archive's central directory, read without transferring the archive.
#[derive(Debug)]
pub struct Directory {
    /// Every member, in the order the directory lists them.
    pub entries: Vec<Entry>,
    /// Where the central directory itself starts, which is where the last member's data ends.
    pub directory_offset: u64,
}

impl Directory {
    /// The member with that name, and its position in the listing.
    ///
    /// # Errors
    ///
    /// [`ArchiveError::NoSuchMember`] when the archive does not hold it.
    pub fn find(&self, name: &str) -> Result<(usize, &Entry), Error> {
        self.entries
            .iter()
            .enumerate()
            .find(|(_, entry)| entry.name == name)
            .ok_or_else(|| ArchiveError::NoSuchMember(name.to_owned()).into())
    }

    /// Where the data of the entry at `index` must end, at the latest.
    ///
    /// The next entry's local header, or — for the last entry — the start of the central
    /// directory. A ZIP lays its members out consecutively, so this is what makes a
    /// contiguous run of members one byte range rather than one request each.
    #[must_use]
    pub fn extent_after(&self, index: usize) -> u64 {
        self.entries
            .get(index.saturating_add(1))
            .map_or(self.directory_offset, |next| next.local_header)
    }
}

/// Reads a little-endian `u16` at `at`.
fn u16_at(bytes: &[u8], at: usize) -> Option<u16> {
    let slice: [u8; 2] = bytes.get(at..at.checked_add(2)?)?.try_into().ok()?;
    Some(u16::from_le_bytes(slice))
}

/// Reads a little-endian `u32` at `at`.
fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    let slice: [u8; 4] = bytes.get(at..at.checked_add(4)?)?.try_into().ok()?;
    Some(u32::from_le_bytes(slice))
}

/// Reads a little-endian `u64` at `at`.
fn u64_at(bytes: &[u8], at: usize) -> Option<u64> {
    let slice: [u8; 8] = bytes.get(at..at.checked_add(8)?)?.try_into().ok()?;
    Some(u64::from_le_bytes(slice))
}

/// Where the central directory is, read out of a tail of the object.
///
/// `tail_start` is the offset within the object at which `tail` begins, so that the offsets
/// this returns are the object's rather than the buffer's.
///
/// # Errors
///
/// [`ArchiveError`] when no end-of-central-directory record is present, when it is truncated,
/// or when it describes a split archive.
pub fn locate_directory(tail: &[u8], tail_start: u64) -> Result<(u64, u64, u64), Error> {
    // APPNOTE 4.3.16: the record is 22 bytes plus a comment of up to 65 535, and it is the
    // last thing in the file, so it is found by scanning backwards for its signature.
    const EOCD: u32 = 0x0605_4b50;
    const EOCD64: u32 = 0x0606_4b50;
    const EOCD64_LOCATOR: u32 = 0x0706_4b50;

    let at = (0..tail.len())
        .rev()
        .find(|&at| u32_at(tail, at) == Some(EOCD))
        .ok_or(ArchiveError::NoDirectory(tail.len()))?;

    let disks = u16_at(tail, at.saturating_add(4)).unwrap_or(0);
    if disks != 0 {
        return Err(ArchiveError::Split(u32::from(disks).saturating_add(1)).into());
    }
    let count = u16_at(tail, at.saturating_add(10)).ok_or(ArchiveError::Truncated {
        structure: "end-of-central-directory record",
        at: tail_start.saturating_add(at as u64),
        available: tail.len(),
    })?;
    let size = u32_at(tail, at.saturating_add(12)).unwrap_or(0);
    let offset = u32_at(tail, at.saturating_add(16)).unwrap_or(0);

    // APPNOTE 4.3.15: when any of the three is the escape value, the real ones are in the
    // ZIP64 record, found through a locator that sits immediately before this record.
    let escaped = count == u16::MAX || size == u32::MAX || offset == u32::MAX;
    if escaped {
        let locator = at.checked_sub(20).ok_or(ArchiveError::Truncated {
            structure: "ZIP64 end-of-central-directory locator",
            at: tail_start.saturating_add(at as u64),
            available: tail.len(),
        })?;
        if u32_at(tail, locator) != Some(EOCD64_LOCATOR) {
            return Err(ArchiveError::BadSignature {
                structure: "ZIP64 end-of-central-directory locator",
                at: tail_start.saturating_add(locator as u64),
                found: u32_at(tail, locator).unwrap_or(0),
            }
            .into());
        }
        let record = u64_at(tail, locator.saturating_add(8)).unwrap_or(0);
        // The record is normally inside the tail already; when it is not, the caller has to
        // read further back, and saying so is better than reading a wrong number.
        let within = record
            .checked_sub(tail_start)
            .and_then(|within| usize::try_from(within).ok())
            .ok_or(ArchiveError::Truncated {
                structure: "ZIP64 end-of-central-directory record",
                at: record,
                available: tail.len(),
            })?;
        if u32_at(tail, within) != Some(EOCD64) {
            return Err(ArchiveError::BadSignature {
                structure: "ZIP64 end-of-central-directory record",
                at: record,
                found: u32_at(tail, within).unwrap_or(0),
            }
            .into());
        }
        let count = u64_at(tail, within.saturating_add(32)).unwrap_or(0);
        let size = u64_at(tail, within.saturating_add(40)).unwrap_or(0);
        let offset = u64_at(tail, within.saturating_add(48)).unwrap_or(0);
        return Ok((offset, size, count));
    }

    Ok((u64::from(offset), u64::from(size), u64::from(count)))
}

/// Parses a central directory out of the bytes at `directory_offset`.
///
/// # Errors
///
/// [`ArchiveError::Truncated`] or [`ArchiveError::BadSignature`] when the bytes do not hold
/// the number of entries the end record promised.
pub fn read_directory(bytes: &[u8], directory_offset: u64, count: u64) -> Result<Directory, Error> {
    const CENTRAL: u32 = 0x0201_4b50;
    /// Fixed part of a central directory file header, APPNOTE 4.3.12.
    const FIXED: usize = 46;

    let mut entries = Vec::new();
    let mut at = 0usize;
    for _ in 0..count {
        if u32_at(bytes, at) != Some(CENTRAL) {
            return Err(ArchiveError::BadSignature {
                structure: "central directory file header",
                at: directory_offset.saturating_add(at as u64),
                found: u32_at(bytes, at).unwrap_or(0),
            }
            .into());
        }
        let truncated = || ArchiveError::Truncated {
            structure: "central directory file header",
            at: directory_offset.saturating_add(at as u64),
            available: bytes.len(),
        };
        let method = u16_at(bytes, at.saturating_add(10)).ok_or_else(truncated)?;
        let crc32 = u32_at(bytes, at.saturating_add(16)).ok_or_else(truncated)?;
        let compressed = u32_at(bytes, at.saturating_add(20)).ok_or_else(truncated)?;
        let uncompressed = u32_at(bytes, at.saturating_add(24)).ok_or_else(truncated)?;
        let name_len = usize::from(u16_at(bytes, at.saturating_add(28)).ok_or_else(truncated)?);
        let extra_len = usize::from(u16_at(bytes, at.saturating_add(30)).ok_or_else(truncated)?);
        let comment_len = usize::from(u16_at(bytes, at.saturating_add(32)).ok_or_else(truncated)?);
        let local_header = u32_at(bytes, at.saturating_add(42)).ok_or_else(truncated)?;

        let name_at = at.checked_add(FIXED).ok_or_else(truncated)?;
        let extra_at = name_at.checked_add(name_len).ok_or_else(truncated)?;
        let comment_at = extra_at.checked_add(extra_len).ok_or_else(truncated)?;
        let end = comment_at.checked_add(comment_len).ok_or_else(truncated)?;
        let name = bytes
            .get(name_at..extra_at)
            .ok_or_else(truncated)
            .map(String::from_utf8_lossy)?
            .into_owned();
        let extra = bytes.get(extra_at..comment_at).ok_or_else(truncated)?;

        // APPNOTE 4.5.3: the ZIP64 extra field restores whichever of the four the fixed part
        // escaped, in this order and only for the escaped ones.
        let mut wide = zip64_fields(extra);
        let uncompressed = pick(uncompressed, &mut wide);
        let compressed = pick(compressed, &mut wide);
        let local_header = pick(local_header, &mut wide);

        entries.push(Entry {
            name,
            local_header,
            compressed,
            uncompressed,
            crc32,
            method,
        });
        at = end;
    }
    Ok(Directory {
        entries,
        directory_offset,
    })
}

/// The 64-bit values in extra field `0x0001`, in the order APPNOTE 4.5.3 lists them.
fn zip64_fields(extra: &[u8]) -> std::collections::VecDeque<u64> {
    let mut at = 0usize;
    let mut wide = std::collections::VecDeque::new();
    while let (Some(tag), Some(len)) = (u16_at(extra, at), u16_at(extra, at.saturating_add(2))) {
        let body = at.saturating_add(4);
        let end = body.saturating_add(usize::from(len));
        if tag == 0x0001 {
            let mut field = body;
            while let Some(value) = u64_at(extra, field) {
                if field.saturating_add(8) > end {
                    break;
                }
                wide.push_back(value);
                field = field.saturating_add(8);
            }
        }
        if end <= at {
            break;
        }
        at = end;
    }
    wide
}

/// The 32-bit value, or the next ZIP64 one when it is the escape.
fn pick(narrow: u32, wide: &mut std::collections::VecDeque<u64>) -> u64 {
    if narrow == u32::MAX {
        wide.pop_front().unwrap_or(u64::from(narrow))
    } else {
        u64::from(narrow)
    }
}

/// Takes one member's bytes out of a buffer that begins at `span_start` in the object.
///
/// The local file header is read rather than assumed, because APPNOTE permits its extra
/// field to differ in length from the central directory's — which is the one thing that stops
/// a member's data offset from being arithmetic on the directory alone.
///
/// # Errors
///
/// [`ArchiveError`] when the header is not where the directory said, and [`Error::Corrupt`]
/// when the inflated bytes do not match the length or the CRC-32 the directory records.
pub fn extract(entry: &Entry, span: &[u8], span_start: u64) -> Result<Vec<u8>, Error> {
    use std::io::Read as _;
    const LOCAL: u32 = 0x0403_4b50;
    /// Fixed part of a local file header, APPNOTE 4.3.7.
    const FIXED: u64 = 30;

    let truncated = || ArchiveError::Truncated {
        structure: "local file header",
        at: entry.local_header,
        available: span.len(),
    };
    let at = usize::try_from(
        entry
            .local_header
            .checked_sub(span_start)
            .ok_or_else(truncated)?,
    )
    .map_err(|_| truncated())?;
    if u32_at(span, at) != Some(LOCAL) {
        return Err(ArchiveError::BadSignature {
            structure: "local file header",
            at: entry.local_header,
            found: u32_at(span, at).unwrap_or(0),
        }
        .into());
    }
    let name_len = u64::from(u16_at(span, at.saturating_add(26)).ok_or_else(truncated)?);
    let extra_len = u64::from(u16_at(span, at.saturating_add(28)).ok_or_else(truncated)?);
    let data_at = usize::try_from(
        (at as u64)
            .checked_add(FIXED)
            .and_then(|to| to.checked_add(name_len))
            .and_then(|to| to.checked_add(extra_len))
            .ok_or_else(truncated)?,
    )
    .map_err(|_| truncated())?;
    let data_end = usize::try_from(
        (data_at as u64)
            .checked_add(entry.compressed)
            .ok_or_else(truncated)?,
    )
    .map_err(|_| truncated())?;
    let compressed = span.get(data_at..data_end).ok_or_else(truncated)?;

    let plain = match entry.method {
        0 => compressed.to_vec(),
        8 => {
            let mut plain = Vec::new();
            flate2::read::DeflateDecoder::new(compressed)
                .read_to_end(&mut plain)
                .map_err(|error| Error::Corrupt {
                    member: entry.name.clone(),
                    what: format!("its deflate stream did not decode: {error}"),
                })?;
            plain
        }
        method => {
            return Err(ArchiveError::UnknownMethod {
                name: entry.name.clone(),
                method,
            }
            .into());
        }
    };

    // The two checks the archive itself makes possible, and the reason a fetch of a byte
    // range is as trustworthy as a fetch of the whole object.
    let got = u64::try_from(plain.len()).unwrap_or(u64::MAX);
    if got != entry.uncompressed {
        return Err(Error::Corrupt {
            member: entry.name.clone(),
            what: format!(
                "the archive records {} bytes and it decoded to {got}",
                entry.uncompressed
            ),
        });
    }
    let mut crc = flate2::Crc::new();
    crc.update(&plain);
    if crc.sum() != entry.crc32 {
        return Err(Error::Corrupt {
            member: entry.name.clone(),
            what: format!(
                "the archive records CRC-32 {:#010x} and the bytes hash to {:#010x}",
                entry.crc32,
                crc.sum()
            ),
        });
    }
    Ok(plain)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A one-member archive built here, so the reader is exercised without a network.
    ///
    /// Stored rather than deflated, so the fixture is legible: the two verification checks
    /// are what the test is about and they run either way.
    fn one_member_archive(name: &str, body: &[u8]) -> Vec<u8> {
        let mut crc = flate2::Crc::new();
        crc.update(body);
        let crc = crc.sum();
        let len = u32::try_from(body.len()).unwrap();
        let mut zip = Vec::new();
        // Local file header.
        zip.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        zip.extend_from_slice(&[20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        zip.extend_from_slice(&crc.to_le_bytes());
        zip.extend_from_slice(&len.to_le_bytes());
        zip.extend_from_slice(&len.to_le_bytes());
        zip.extend_from_slice(&u16::try_from(name.len()).unwrap().to_le_bytes());
        zip.extend_from_slice(&0u16.to_le_bytes());
        zip.extend_from_slice(name.as_bytes());
        zip.extend_from_slice(body);
        let directory_offset = u32::try_from(zip.len()).unwrap();
        // Central directory file header.
        zip.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        zip.extend_from_slice(&[20, 0, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        zip.extend_from_slice(&crc.to_le_bytes());
        zip.extend_from_slice(&len.to_le_bytes());
        zip.extend_from_slice(&len.to_le_bytes());
        zip.extend_from_slice(&u16::try_from(name.len()).unwrap().to_le_bytes());
        zip.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        zip.extend_from_slice(&0u32.to_le_bytes());
        zip.extend_from_slice(name.as_bytes());
        let directory_size = u32::try_from(zip.len())
            .unwrap()
            .saturating_sub(directory_offset);
        // End of central directory.
        zip.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
        zip.extend_from_slice(&[0, 0, 0, 0, 1, 0, 1, 0]);
        zip.extend_from_slice(&directory_size.to_le_bytes());
        zip.extend_from_slice(&directory_offset.to_le_bytes());
        zip.extend_from_slice(&0u16.to_le_bytes());
        zip
    }

    #[test]
    fn a_member_is_found_through_the_tail_and_verified_on_the_way_out() {
        let zip = one_member_archive("0000000.pdf", b"%PDF-1.7\n%%EOF\n");
        let (offset, size, count) = locate_directory(&zip, 0).unwrap();
        assert_eq!(count, 1);
        let from = usize::try_from(offset).unwrap();
        let to = from + usize::try_from(size).unwrap();
        let directory = read_directory(&zip[from..to], offset, count).unwrap();
        assert_eq!(directory.entries[0].name, "0000000.pdf");
        assert_eq!(directory.extent_after(0), offset);
        let plain = extract(&directory.entries[0], &zip, 0).unwrap();
        assert_eq!(plain, b"%PDF-1.7\n%%EOF\n");
    }

    /// The CRC the archive records is what makes a byte-range fetch trustworthy, so a
    /// mismatch has to be a refusal rather than a warning.
    #[test]
    fn a_member_whose_bytes_do_not_hash_to_what_the_archive_records_is_refused() {
        let mut zip = one_member_archive("0000000.pdf", b"%PDF-1.7\n%%EOF\n");
        let (offset, size, count) = locate_directory(&zip, 0).unwrap();
        let from = usize::try_from(offset).unwrap();
        let to = from + usize::try_from(size).unwrap();
        let directory = read_directory(&zip[from..to], offset, count).unwrap();
        // One byte of the member's stored data — 30 bytes of local header and an 11-byte
        // name in front of it — which the length check cannot see.
        zip[41] = b'X';
        let refused = extract(&directory.entries[0], &zip, 0).unwrap_err();
        assert!(
            format!("{refused}").contains("CRC-32"),
            "expected a CRC refusal, got {refused}"
        );
    }

    /// Nothing that is not an archive may be read as one.
    #[test]
    fn a_body_that_is_not_a_zip_says_so() {
        let refused = locate_directory(b"<html>404</html>", 0).unwrap_err();
        assert!(format!("{refused}").contains("not a ZIP archive"));
    }
}
