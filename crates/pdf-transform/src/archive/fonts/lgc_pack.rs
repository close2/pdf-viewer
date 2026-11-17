//! The Latin-Greek-Cyrillic pack: Noto Serif in four styles and Noto Sans Mono in two, downloaded
//! beside this program, for a simple font whose shown codes the compiled-in serif or fixed face
//! cannot all draw (ADR 2630's decision, ADR 2934's build).
//!
//! # Why the face changes for the whole font, and only where it must
//!
//! ISO 19005-2 section 6.2.11.4.1 and ISO 19005-4 section 6.2.10.4.1 ask a program of every font a
//! page renders, and ISO 19005-2 section 6.2.11.8 forbids a shown code that reaches `.notdef`. The
//! Foxit serif and fixed faces this program ships hold the Standard Latin set and nothing of Greek,
//! Cyrillic or Latin Extended-A, so a `Times-Roman` showing a beta has had no answer but a
//! photograph of its page. A face of the pack answers it: one program for the font, so every code
//! is drawn from one design and nothing is pieced together from two. It is asked for only after
//! the operator's face, the machine's face the window drew, and the shipped face, in that order,
//! and only where the shipped face lacks a shown code ([`super::face_for`]); an `Adobe-Identity`
//! font re-keyed from its `/ToUnicode` asks it after the shipped face and the CJK pack
//! ([`super::rekeyed`], ADR 2378's order).
//!
//! The face drawn is the pack's, not the producer's and not the one the window drew, which is ADR
//! 2162's named loss `machine-face`: `as-if-printed` discards it and `as-shown` stops on it.
//!
//! # What the programs are
//!
//! Bare CFF programs, which ISO 32000-2 §9.9's Table 124 admits under a `Type1` font dictionary as
//! `/FontFile3 /Type1C`, the form the Foxit faces already take; a simple font reaches each glyph by
//! its name (§9.6.5), and every glyph is named by the Adobe Glyph List. `tools/lgc-pack.py` builds
//! them; `doc/lgc-pack.md` is the layout.
//!
//! # Where the pack comes from
//!
//! **The caller's**, as the CJK pack is ([`super::pack`]): `apply` opens no path, the binary looks
//! beside its executable and where `--lgc-pack` points, and this module holds each program to the
//! digest below, so bytes that are not the builder's are refused as not this pack.

use std::sync::{Arc, OnceLock};

use pdf_font::substitute::{Family, Request};
use sha2::{Digest as _, Sha256};

use super::pack::{PackFiles, PackGap, hex};

/// The notice every pack program carries, verbatim: the family's own `OFL.txt`.
pub(super) const LICENCE: &str = include_str!("../../../../../data/lgc-pack/LICENSE.txt");

/// One face of the pack.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LgcFace {
    /// Noto Serif at weight 400.
    SerifRegular,
    /// Noto Serif at weight 700.
    SerifBold,
    /// Noto Serif Italic at weight 400.
    SerifItalic,
    /// Noto Serif Italic at weight 700.
    SerifBoldItalic,
    /// Noto Sans Mono at weight 400.
    MonoRegular,
    /// Noto Sans Mono at weight 700.
    MonoBold,
}

impl LgcFace {
    /// Every face the pack holds, in the order `SHA256SUMS` lists them.
    pub const ALL: [Self; 6] = [
        Self::SerifRegular,
        Self::SerifBold,
        Self::SerifItalic,
        Self::SerifBoldItalic,
        Self::MonoRegular,
        Self::MonoBold,
    ];

    /// The face standing in for the shipped face a request is answered by, where the pack holds
    /// one: the serif face's four styles, and the fixed face's two upright ones, since Noto Sans
    /// Mono is published in no italic and an upright face is not the producer's italic.
    #[must_use]
    pub const fn for_request(request: Request) -> Option<Self> {
        match (request.family, request.bold, request.italic) {
            (Family::Serif, false, false) => Some(Self::SerifRegular),
            (Family::Serif, true, false) => Some(Self::SerifBold),
            (Family::Serif, false, true) => Some(Self::SerifItalic),
            (Family::Serif, true, true) => Some(Self::SerifBoldItalic),
            (Family::Monospace, false, false) => Some(Self::MonoRegular),
            (Family::Monospace, true, false) => Some(Self::MonoBold),
            _ => None,
        }
    }

    /// The program's file name inside the pack directory.
    #[must_use]
    pub const fn file(self) -> &'static str {
        match self {
            Self::SerifRegular => "NotoSerif-Regular.cff",
            Self::SerifBold => "NotoSerif-Bold.cff",
            Self::SerifItalic => "NotoSerif-Italic.cff",
            Self::SerifBoldItalic => "NotoSerif-BoldItalic.cff",
            Self::MonoRegular => "NotoSansMono-Regular.cff",
            Self::MonoBold => "NotoSansMono-Bold.cff",
        }
    }

    /// The face's name, for the report and the output's provenance.
    #[must_use]
    pub const fn describe(self) -> &'static str {
        match self {
            Self::SerifRegular => "Noto Serif Regular, from the Latin-Greek-Cyrillic pack",
            Self::SerifBold => "Noto Serif Bold, from the Latin-Greek-Cyrillic pack",
            Self::SerifItalic => "Noto Serif Italic, from the Latin-Greek-Cyrillic pack",
            Self::SerifBoldItalic => "Noto Serif Bold Italic, from the Latin-Greek-Cyrillic pack",
            Self::MonoRegular => "Noto Sans Mono Regular, from the Latin-Greek-Cyrillic pack",
            Self::MonoBold => "Noto Sans Mono Bold, from the Latin-Greek-Cyrillic pack",
        }
    }

    /// The SHA-256 of the program `tools/lgc-pack.py` writes, which is how this program knows the
    /// bytes it was handed are the pack's; CI holds the pack it publishes to these.
    const fn digest(self) -> &'static str {
        match self {
            Self::SerifRegular => {
                "f6331aa0a1c6560f38bdee05d1352ee52eb10b3e508538d595c8b9a3436fb68d"
            }
            Self::SerifBold => "7f9eaf22ddfd9c8d86ac9c6b15ef7331d9010de8b5bea77e1895a457283e2785",
            Self::SerifItalic => "717d982dfb48fba78276620ca4998472465fe666b544903e95da5b4d98a36956",
            Self::SerifBoldItalic => {
                "bb18cffc103dc00fbb683566fd19d5058522b7ade402b79ff57870efede1de49"
            }
            Self::MonoRegular => "7628d0991bb74a7764c82b5d906a0edfebea58f1ea8a8fdd44f53c75263fae30",
            Self::MonoBold => "0efd3c29d995835d8739073487e3d627fbba10d54248c6d269451f7549f73ba8",
        }
    }

    /// Where this face sits in [`LgcFace::ALL`].
    const fn index(self) -> usize {
        match self {
            Self::SerifRegular => 0,
            Self::SerifBold => 1,
            Self::SerifItalic => 2,
            Self::SerifBoldItalic => 3,
            Self::MonoRegular => 4,
            Self::MonoBold => 5,
        }
    }
}

/// The Latin-Greek-Cyrillic pack a conversion may embed from, each program read and verified on
/// first use.
#[derive(Debug, Clone)]
pub struct LgcPack(Arc<Shared>);

/// What every clone of one pack shares: the caller's reader and each program once read. One
/// pointer, so that a plan carrying the pack stays the size it was (`clippy::large_enum_variant`
/// on `pdf_transform::Plan`).
#[derive(Debug)]
struct Shared {
    files: Arc<dyn PackFiles>,
    loaded: [Loaded; 6],
}

/// One face's program once it was asked for: its verified bytes, or why there are none.
type Loaded = OnceLock<Result<Arc<[u8]>, PackGap>>;

/// Two packs are one pack when they read through the same reader, as for [`super::CjkPack`].
impl PartialEq for LgcPack {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0.files, &other.0.files)
    }
}

impl LgcPack {
    /// A pack read through `files`, nothing of it read yet.
    #[must_use]
    pub fn new(files: Arc<dyn PackFiles>) -> Self {
        Self(Arc::new(Shared {
            files,
            loaded: std::array::from_fn(|_| OnceLock::new()),
        }))
    }

    /// The face's program, read once and held to its digest.
    ///
    /// # Errors
    ///
    /// [`PackGap::Absent`] where the caller's reader has no such file, and
    /// [`PackGap::NotThisPack`] where its bytes are not the ones `tools/lgc-pack.py` writes.
    pub fn program(&self, face: LgcFace) -> Result<Arc<[u8]>, PackGap> {
        self.0.loaded[face.index()]
            .get_or_init(|| {
                let bytes = self.0.files.read(face.file()).ok_or(PackGap::Absent)?;
                if hex(&Sha256::digest(&bytes)) == face.digest() {
                    Ok(Arc::from(bytes))
                } else {
                    Err(PackGap::NotThisPack)
                }
            })
            .clone()
    }
}

#[cfg(test)]
mod tests {
    use super::{LgcFace, LgcPack, PackFiles, PackGap};
    use pdf_font::substitute::{Family, Request};
    use std::sync::Arc;

    /// A reader holding one file under one name.
    #[derive(Debug)]
    struct One(&'static str, Vec<u8>);

    impl PackFiles for One {
        fn read(&self, file: &str) -> Option<Vec<u8>> {
            (file == self.0).then(|| self.1.clone())
        }
    }

    fn request(family: Family, bold: bool, italic: bool) -> Request {
        Request {
            family,
            bold,
            italic,
            standard: true,
        }
    }

    #[test]
    fn the_serif_face_has_four_styles_the_fixed_two_and_the_sans_none() {
        assert_eq!(
            LgcFace::for_request(request(Family::Serif, true, true)),
            Some(LgcFace::SerifBoldItalic)
        );
        assert_eq!(
            LgcFace::for_request(request(Family::Monospace, true, false)),
            Some(LgcFace::MonoBold)
        );
        // Noto Sans Mono is published upright only, and an upright face is not the italic asked.
        assert_eq!(
            LgcFace::for_request(request(Family::Monospace, false, true)),
            None
        );
        assert_eq!(
            LgcFace::for_request(request(Family::SansSerif, false, false)),
            None
        );
        for (index, face) in LgcFace::ALL.into_iter().enumerate() {
            assert_eq!(face.index(), index);
        }
    }

    #[test]
    fn a_missing_file_is_absent_and_other_bytes_are_not_this_pack() {
        let empty = LgcPack::new(Arc::new(One("nothing", Vec::new())));
        assert_eq!(empty.program(LgcFace::SerifRegular), Err(PackGap::Absent));
        let wrong = LgcPack::new(Arc::new(One("NotoSerif-Bold.cff", b"not a font".to_vec())));
        assert_eq!(wrong.program(LgcFace::SerifBold), Err(PackGap::NotThisPack));
    }

    #[test]
    fn the_notice_is_the_open_font_licence_with_the_familys_copyright_line() {
        assert!(super::LICENCE.contains("SIL OPEN FONT LICENSE Version 1.1"));
        assert!(super::LICENCE.starts_with("Copyright 2022 The Noto Project Authors"));
    }

    /// A reader of a pack directory on disk.
    #[derive(Debug)]
    struct Directory(std::path::PathBuf);

    impl PackFiles for Directory {
        fn read(&self, file: &str) -> Option<Vec<u8>> {
            std::fs::read(self.0.join(file)).ok()
        }
    }

    /// A built pack against what this file carries: the `SHA256SUMS` `tools/lgc-pack.py` wrote
    /// lists each program under the digest compiled in here and the licence under the notice's,
    /// in [`LgcFace::ALL`]'s order, and every file holds the bytes its line names. CI's
    /// `lgc-pack` job builds the pack from the Noto files fetched by digest and runs this before
    /// it publishes the zip.
    #[test]
    #[ignore = "needs a built Latin-Greek-Cyrillic pack, which is not part of a checkout: \
                QUORRA_LGC_PACK=<dir> names it (doc/lgc-pack.md), and CI's lgc-pack job runs it"]
    fn a_built_pack_holds_the_programs_whose_digests_this_file_carries() {
        use super::{LICENCE, hex};
        use sha2::{Digest as _, Sha256};

        let directory = std::path::PathBuf::from(
            std::env::var_os("QUORRA_LGC_PACK")
                .expect("QUORRA_LGC_PACK names the directory the pack was built into"),
        );
        let read = |file: &str| {
            std::fs::read(directory.join(file))
                .unwrap_or_else(|error| panic!("{}: {error}", directory.join(file).display()))
        };
        let sums = String::from_utf8(read("SHA256SUMS")).expect("SHA256SUMS is ASCII");
        let listed: Vec<(String, String)> = sums
            .lines()
            .map(|line| {
                let (digest, file) = line
                    .split_once("  ")
                    .unwrap_or_else(|| panic!("not sha256sum's format: {line:?}"));
                (file.to_owned(), digest.to_owned())
            })
            .collect();
        let mut carried: Vec<(String, String)> = LgcFace::ALL
            .into_iter()
            .map(|face| (face.file().to_owned(), face.digest().to_owned()))
            .collect();
        carried.push((
            "LICENSE.txt".to_owned(),
            hex(&Sha256::digest(LICENCE.as_bytes())),
        ));
        assert_eq!(
            listed, carried,
            "the built pack is not the one lgc_pack.rs names"
        );
        for (file, digest) in &listed {
            assert_eq!(&hex(&Sha256::digest(read(file))), digest, "{file}");
        }
        let pack = LgcPack::new(Arc::new(Directory(directory)));
        for face in LgcFace::ALL {
            assert!(pack.program(face).is_ok(), "{}", face.file());
        }
    }
}
