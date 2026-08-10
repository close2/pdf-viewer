//! Where fetched members live, and the manifest that makes one re-fetchable exactly.
//!
//! `corpus-cache/safedocs/<corpus>/<archive>/<member>`, which `.gitignore` covers. Nothing
//! here is ever committed; what may be committed is a file *promoted* out of it into this
//! tree's own test set, which `doc/adr/0258` decides with a budget attached.
//!
//! # The manifest
//!
//! One tab-separated line per member, appended as it arrives. It carries the corpus, the
//! archive, the member's name, its size and the SHA-256 of its bytes — which is what
//! `doc/adr/0258`'s names-only rule needs when the promotion budget is spent: the pair
//! *(archive, member)* fetches the file again and the digest says whether it is the same one.
//! Tab-separated rather than JSON because a person greps it and because a format nobody has
//! to parse cannot go out of step with a parser.

use std::path::{Path, PathBuf};

use crate::{Error, zip};

/// A directory of fetched members.
#[derive(Debug, Clone)]
pub struct Cache {
    root: PathBuf,
}

/// One member, on disk.
#[derive(Debug, Clone)]
pub struct Fetched {
    /// Where it was written.
    pub path: PathBuf,
    /// Its name inside the archive.
    pub member: String,
    /// How many bytes it holds.
    pub bytes: u64,
    /// The SHA-256 of those bytes, lowercase hexadecimal.
    pub digest: String,
}

impl Cache {
    /// The cache under `root`, which is `corpus-cache/safedocs` unless a caller says
    /// otherwise.
    #[must_use]
    pub fn at(root: PathBuf) -> Self {
        Self { root }
    }

    /// The default location, relative to the workspace root.
    #[must_use]
    pub fn default_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus-cache/safedocs")
    }

    /// Where this cache keeps its files.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Writes one member and records it in the manifest.
    ///
    /// # Errors
    ///
    /// [`Error::Cache`] when the directory cannot be made or the file cannot be written.
    pub fn store(
        &self,
        corpus: &str,
        archive: &str,
        entry: &zip::Entry,
        bytes: &[u8],
    ) -> Result<Fetched, Error> {
        use sha2::Digest as _;

        // The member's name comes out of somebody else's archive, so it is used as a name and
        // never as a path: anything that is not a plain file name is refused rather than
        // sanitised, because a sanitised traversal is still a caller who asked for one.
        let leaf = Path::new(&entry.name);
        let plain = leaf
            .file_name()
            .is_some_and(|name| name == leaf.as_os_str());
        if !plain {
            return Err(Error::Cache {
                path: entry.name.clone(),
                source: std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "a member name that is not a plain file name is not written to disk",
                ),
            });
        }
        let directory = self.root.join(corpus).join(archive);
        std::fs::create_dir_all(&directory).map_err(|source| Error::Cache {
            path: directory.display().to_string(),
            source,
        })?;
        let path = directory.join(&entry.name);
        std::fs::write(&path, bytes).map_err(|source| Error::Cache {
            path: path.display().to_string(),
            source,
        })?;

        // Hexadecimal by table rather than by `format!`, so that the digest is built without a
        // formatting machine that can return an error nobody can handle.
        let digest: String = sha2::Sha256::digest(bytes)
            .iter()
            .flat_map(|byte| {
                // Both indices are below 16 by construction, so neither can be out of range.
                [HEX[usize::from(*byte) / 16], HEX[usize::from(*byte) % 16]]
            })
            .collect();
        let fetched = Fetched {
            path,
            member: entry.name.clone(),
            bytes: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
            digest,
        };
        self.record(corpus, archive, &fetched)?;
        Ok(fetched)
    }

    /// Appends one line to the manifest.
    fn record(&self, corpus: &str, archive: &str, fetched: &Fetched) -> Result<(), Error> {
        use std::io::Write as _;

        let path = self.root.join("manifest.tsv");
        let fresh = !path.exists();
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|source| Error::Cache {
                path: path.display().to_string(),
                source,
            })?;
        let mut write = |line: &str| {
            writeln!(file, "{line}").map_err(|source| Error::Cache {
                path: path.display().to_string(),
                source,
            })
        };
        if fresh {
            write("# corpus\tarchive\tmember\tbytes\tsha256")?;
        }
        write(&format!(
            "{corpus}\t{archive}\t{}\t{}\t{}",
            fetched.member, fetched.bytes, fetched.digest
        ))
    }

    /// Every PDF in the cache, sorted, or an empty list when nothing has been fetched.
    #[must_use]
    pub fn documents(&self) -> Vec<PathBuf> {
        let mut found = Vec::new();
        collect(&self.root, &mut found);
        found.sort();
        found
    }
}

/// The sixteen digits a SHA-256 is written with.
const HEX: [char; 16] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f',
];

/// Walks a directory tree, collecting `.pdf` files.
fn collect(directory: &Path, into: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, into);
        } else if path.extension().is_some_and(|extension| extension == "pdf") {
            into.push(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str) -> zip::Entry {
        zip::Entry {
            name: name.to_owned(),
            local_header: 0,
            compressed: 0,
            uncompressed: 0,
            crc32: 0,
            method: 0,
        }
    }

    /// A member name is somebody else's string; it names a file in the cache and nothing
    /// above it.
    #[test]
    fn a_member_name_that_is_a_path_is_refused_rather_than_sanitised() {
        let root = std::env::temp_dir().join(format!("safedocs-{}", std::process::id()));
        let cache = Cache::at(root.clone());
        assert!(
            cache
                .store("c", "0000", &entry("../escape.pdf"), b"x")
                .is_err()
        );
        assert!(cache.store("c", "0000", &entry("a/b.pdf"), b"x").is_err());
        let ok = cache
            .store("c", "0000", &entry("0000000.pdf"), b"x")
            .unwrap();
        assert_eq!(ok.bytes, 1);
        // SHA-256 of the single byte "x", from RFC 6234's own construction.
        assert_eq!(
            ok.digest,
            "2d711642b726b04401627ca9fbac32f5c8530fb1903cc4db02258717921a4881"
        );
        assert_eq!(cache.documents().len(), 1);
        drop(std::fs::remove_dir_all(&root));
    }
}
