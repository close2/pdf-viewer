//! The corpora this tool knows how to address, and how an archive name becomes a URL.
//!
//! Every figure here was read off the bucket's own listing and the corpus's own `README.md`
//! rather than recalled; the round that wrote them down is 422 and `doc/adr/0258` says what
//! was listed.

use crate::Error;

/// One addressable corpus: a set of ZIP archives under a common prefix.
#[derive(Debug, Clone, Copy)]
pub struct Corpus {
    /// What a caller types.
    pub name: &'static str,
    /// One sentence about what is in it.
    pub about: &'static str,
    /// How many archives it holds.
    pub archives: u32,
    /// Roughly what one archive weighs, for a plan's prose.
    pub archive_bytes: u64,
    /// The bucket the objects live in, as an HTTPS origin.
    pub origin: &'static str,
    /// The key prefix under that origin.
    pub prefix: &'static str,
}

impl Corpus {
    /// The URL of one archive, or why the name does not fit this corpus.
    ///
    /// # Errors
    ///
    /// [`Error::NoSuchArchive`] when the name is not the four decimal digits the corpus's
    /// `README.md` specifies, or names an archive past the end of the set.
    pub fn archive_url(&self, archive: &str) -> Result<String, Error> {
        let digits: u32 = archive.parse().map_err(|_| {
            Error::NoSuchArchive(format!(
                "{archive:?} is not an archive name; {} names its archives with four decimal \
                 digits, 0000 to {:04}",
                self.name,
                self.archives.saturating_sub(1)
            ))
        })?;
        if archive.len() != 4 || digits >= self.archives {
            return Err(Error::NoSuchArchive(format!(
                "{archive:?} is outside {}'s archives, which run 0000 to {:04}",
                self.name,
                self.archives.saturating_sub(1)
            )));
        }
        // The corpus clusters its archives into directories of a thousand, named for the
        // range they hold: `zipfiles/0000-0999/0000.zip`. That is the README's scheme and
        // this is the arithmetic for it.
        let group = digits.saturating_div(1000).saturating_mul(1000);
        Ok(format!(
            "{}/{}{group:04}-{:04}/{archive}.zip",
            self.origin,
            self.prefix,
            group.saturating_add(999)
        ))
    }
}

/// Every corpus this tool can address.
///
/// **One entry, deliberately.** The `CC-MAIN-2021-31-UNSAFE` set beside it in the same bucket
/// is malware collected from the same crawl; it is not registered here, because a tool whose
/// whole job is to make a download deliberate should not carry a one-word route to a corpus
/// of live samples. Naming a URL by hand still works and is then a decision somebody made.
pub const KNOWN: &[Corpus] = &[Corpus {
    name: "cc-main-2021-31",
    about: "7 932 878 PDF files gathered from the web by Common Crawl in July and August 2021 \
            and re-fetched untruncated for DARPA SafeDocs; 1000 files per archive, about 8 TB \
            uncompressed",
    archives: 7933,
    archive_bytes: 1_300_000_000,
    origin: "https://digitalcorpora.s3.amazonaws.com",
    prefix: "corpora/files/CC-MAIN-2021-31-PDF-UNTRUNCATED/zipfiles/",
}];

/// Finds a corpus by name.
///
/// # Errors
///
/// [`Error::NoSuchCorpus`] when nothing is registered under that name.
pub fn corpus(name: &str) -> Result<&'static Corpus, Error> {
    KNOWN
        .iter()
        .find(|corpus| corpus.name == name)
        .ok_or_else(|| Error::NoSuchCorpus(name.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The README's own example: `0000.zip` sits in `zipfiles/0000-0999/`.
    #[test]
    fn an_archive_name_becomes_the_url_the_readme_specifies() {
        let corpus = corpus("cc-main-2021-31").unwrap();
        assert_eq!(
            corpus.archive_url("0000").unwrap(),
            "https://digitalcorpora.s3.amazonaws.com/corpora/files/\
             CC-MAIN-2021-31-PDF-UNTRUNCATED/zipfiles/0000-0999/0000.zip"
        );
        assert_eq!(
            corpus.archive_url("1234").unwrap(),
            "https://digitalcorpora.s3.amazonaws.com/corpora/files/\
             CC-MAIN-2021-31-PDF-UNTRUNCATED/zipfiles/1000-1999/1234.zip"
        );
    }

    /// A name outside the set is refused rather than turned into a 404 the caller has to read.
    #[test]
    fn an_archive_outside_the_set_is_refused_before_the_network() {
        let corpus = corpus("cc-main-2021-31").unwrap();
        assert!(corpus.archive_url("9999").is_err());
        assert!(corpus.archive_url("0").is_err());
        assert!(corpus.archive_url("../etc").is_err());
    }
}
