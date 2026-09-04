//! The layout, as one declarative table: path pattern → generator → write mapping.
//!
//! RFC 0003 section 7 makes this table the single place the design lives — "the faces contain
//! *no* layout knowledge — adding `fonts/` one day is a core change that both faces grow
//! simultaneously". So a row states three things and nothing states them anywhere else: what the
//! path looks like, what produces its content, and **what a write to it means**. The write
//! mappings are declared here and performed at the point of the call, or refused there with the
//! sentence saying why ([`crate::Refused`]) — which is the difference between a design with a
//! hole in it and a program that pretends the hole is not there (trap 5).
//!
//! # Why a table rather than a `match` over path components
//!
//! A `match` answers "what is this path" and nothing else. This table also answers "what is in
//! the tree", "what generates each entry", "which clause each generator rests on" and "what a
//! write here would be" — which is what a face needs to enumerate the root, what an ADR needs to
//! review the design, and what [`crate::Vfs::shortfalls`] needs to say out loud which declared
//! behaviour is not built. One population, derived once.
//!
//! # One departure from RFC 0003 section 4, and the argument for it
//!
//! The RFC draws `images/` as one flat directory whose entries are `0001-01.jpg` — page ordinal,
//! index within the page. This table draws it as `images/NNNN/MM.ext`, a directory per page, and
//! the reason is the RFC's *own* principle one clause earlier: section 5.1 requires that
//! "nothing walks the whole document to answer `ls doc.pdf/`". A flat `images/` cannot be listed
//! without knowing every image's file form, and an image's file form is decided by its codec and
//! by whether §8.9.6's mask travels beside it — which is known only once the image has been
//! extracted. So a flat listing costs a decode of every image in the document, and a listing
//! that *predicted* the names instead would sometimes name a file that a read then could not
//! produce. Per page, a listing costs one page and **is** the read's own answer: both are the
//! names one `pdf_transform::images` run's outputs took. The naming information the RFC asked
//! for — which page, which index within it — is unchanged; only the solidus moved.
//!
//! It is a departure from an approved document, so it is written down rather than absorbed:
//! `doc/todo/58` carries it for the owner to overrule.
//!
//! # Names
//!
//! All ASCII, generated, and stable within a generation of the document (RFC 0003 section 4).
//! Ordinals rather than ISO 32000-2 §12.4.2's page labels, and the reason is a *silence* in that
//! clause rather than a sentence in it: §12.4.2 states how a label is built out of a numbering
//! style, a prefix and a start value, and it states no uniqueness rule anywhere, so two pages of
//! one document may carry one label and a directory may not hold two entries under one name. An
//! ordinal is a position and is unique by construction. The label is answered beside the page in
//! `meta/outline.json`, where a repeat costs nothing.

use pdf_transform::Operation;

use crate::path::Captures;

/// Whether a row names a directory or a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Listed, never read.
    Directory,
    /// Read, never listed.
    File,
}

/// What produces a row's content.
///
/// Every one of these is either a [`pdf_transform::Plan`] applied through the seam or a
/// `pdf-model` reader called directly. **None of them is a new implementation of anything**,
/// which is RFC 0003 section 7's requirement on this crate and the reason the list is worth
/// reading as a whole: if a row needs a construction nothing else in the tree has, that is a
/// finding about the row rather than a licence to write one here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Generator {
    /// The six directories of RFC 0003 section 4, named without reading anything but the page
    /// count.
    Root,
    /// One name per page, from the page count alone.
    PageOrdinals,
    /// `pdf_transform::split`, one page selected: the transform suite's page extraction, which
    /// is the flagship of RFC 0003 section 4 — `cp` *is* page extraction.
    ExtractedPage,
    /// One name per offered resolution.
    Resolutions,
    /// One name per page, at one resolution.
    RenderOrdinals,
    /// `pdf_transform::render`, one page selected, at the directory's dots per inch.
    RenderedPage,
    /// One name per page, each a directory of that page's images.
    ImagePageOrdinals,
    /// `pdf_transform::images`, one page selected: the *names its outputs took*, which is why
    /// a listing of this directory and a read out of it cannot disagree — they are one call.
    ImageInventory,
    /// One of those outputs, by the name the transform's sink was opened with: the codec's own
    /// stream where the codec has a file form, decoded PNG where it has not, and §8.9.6's mask
    /// beside it where the native form has nowhere to put one.
    ExtractedImage,
    /// One name per page, plus `document.txt`.
    TextOrdinals,
    /// `pdf_model::interpret`'s own readback, byte for byte.
    PageText,
    /// Every page's readback in page order, separated by a form feed.
    DocumentText,
    /// `pdf_transform::attachments`, listing only: §7.11.4's embedded files by their stated
    /// names.
    AttachmentInventory,
    /// `pdf_transform::attachments`, one file saved.
    ExtractedAttachment,
    /// The three names of `meta/`.
    MetaNames,
    /// `pdf_model::metadata::Information`, as JSON.
    Information,
    /// The catalog's `/Metadata` stream, decoded, byte for byte.
    MetadataStream,
    /// `pdf_model::outline::Outline`, as JSON.
    Outline,
}

impl Generator {
    /// Which of `pdf_model::restriction`'s operations *reading* a row of this shape is.
    ///
    /// [`Write::operation`]'s counterpart for the read side, and pinned the same way. Most rows
    /// answer `None`: a listing reads the document's shape rather than its content, and §14.3's
    /// metadata, §12.3.3's outline and a page's text are named by no Table 22 bit this tree can
    /// consult — `doc/todo/38` holds bit 5's "copy or otherwise extract text and graphics" open
    /// for the day a host can say *this is a copy*.
    #[must_use]
    pub const fn operation(self) -> Option<Operation> {
        match self {
            // A page out of the mount is `pdf_transform split`'s own output, and a split is
            // Table 22 bit 11's "[a]ssemble the document".
            Self::ExtractedPage => Some(Operation::Assemble),
            // Table 22 bit 3, "Print the document": a page raster is what a print driver makes.
            Self::RenderedPage => Some(Operation::Print),
            // Table 22 bit 5, "[c]opy or otherwise extract text and graphics from the document",
            // where the extraction is unambiguously a file written out.
            //
            // **The inventory is the extraction**, which is this table's own departure note: a
            // listing of `images/NNNN/` is one `pdf_transform::images` run's output names, so it
            // performs the operation whether or not a byte is read afterwards.
            Self::ImageInventory | Self::ExtractedImage | Self::ExtractedAttachment => {
                Some(Operation::Extract)
            }
            Self::Root
            | Self::PageOrdinals
            | Self::Resolutions
            | Self::RenderOrdinals
            | Self::ImagePageOrdinals
            | Self::TextOrdinals
            | Self::PageText
            | Self::DocumentText
            | Self::AttachmentInventory
            | Self::MetaNames
            | Self::Information
            | Self::MetadataStream
            | Self::Outline => None,
        }
    }
}

/// What creating or overwriting a path of a row's shape means, and what deleting one means.
///
/// Two answers rather than one, because a file verb is two verbs: `cp new.pdf pages/0004.pdf`
/// inserts a document's pages at that position and `rm pages/0004.pdf` deletes one page, and a
/// row that stated a single "write mapping" could not say both. RFC 0003 section 5.2's table has
/// exactly this shape — a verb column and a meaning column — and this is that table as data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WriteMapping {
    /// What creating or overwriting means here.
    pub on_write: Write,
    /// What deleting means here.
    pub on_delete: Write,
}

impl WriteMapping {
    /// A row where neither verb means anything, for one reason.
    const fn refused(reason: Reason) -> Self {
        Self {
            on_write: Write::Refused(reason),
            on_delete: Write::Refused(reason),
        }
    }

    /// Whether either verb names an operation, which is what makes the row a shortfall while the
    /// write side is unbuilt.
    #[must_use]
    pub fn declares_an_operation(self) -> bool {
        !matches!(self.on_write, Write::Refused(_)) || !matches!(self.on_delete, Write::Refused(_))
    }
}

/// One meaning a file verb has on one row.
///
/// RFC 0003 section 5.2's five supported verbs and section 5.3's four refusals, stated together
/// because they are one decision per row. Every mapping other than [`Write::Refused`] is built:
/// `crate::Vfs::create` reads this to decide whether it may accept a byte, and `crate::Vfs::flush`
/// reads it again to decide which `pdf_transform::update::Edit` the bytes become. A refusal is
/// therefore always by design and never a generic "read-only file system" — which is what would
/// have made the design unreadable from outside.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Write {
    /// Copying a PDF into `pages/` inserts its pages at the position the name states, through
    /// the transform layer's page insertion, saved as §7.5.6's append.
    InsertPages,
    /// Deleting `pages/NNNN.pdf` deletes that page, through the transform layer's page
    /// deletion.
    DeletePage,
    /// Copying a file into `attachments/` embeds it: §7.11.4's stream and §7.7.4's name tree
    /// gain an entry, by incremental update.
    EmbedFile,
    /// Deleting an attachment removes the entry from the name tree, by incremental update.
    RemoveAttachment,
    /// Overwriting `meta/info.json` sets the §14.3.3 entries it states.
    SetInformation,
    /// Refused, with the reason the refusal is worded from.
    Refused(Reason),
}

impl Write {
    /// Which of `pdf_model::restriction`'s operations performing this write is.
    ///
    /// **The broker's half of `CLAUDE.md` principle 3's question**, and it is here rather than
    /// in `crate::Vfs` because this table is the one place a path's meaning is stated. Held to
    /// `pdf_transform::Plan::operation`'s own answer by `tests/a_write.rs` rather than merely
    /// said to agree with it (ADR 0874).
    #[must_use]
    pub const fn operation(self) -> Option<Operation> {
        match self {
            // Table 22 bit 11 names both in as many words: "[a]ssemble the document (insert,
            // rotate, or delete pages …)".
            Self::InsertPages | Self::DeletePage => Some(Operation::Assemble),
            // Bit 4's residual — "[m]odify the contents of the document by operations other than
            // those controlled by bits 6, 9, and 11" — which is where an embedded file and
            // §14.3.3's entries both fall (ADR 0802).
            Self::EmbedFile | Self::RemoveAttachment | Self::SetInformation => {
                Some(Operation::Modify)
            }
            // A verb this tree will not perform asks no policy: there is no operation to permit.
            Self::Refused(_) => None,
        }
    }
}

/// Why a write is refused outright rather than merely unbuilt.
///
/// Each is RFC 0003 section 5.3's, and each is a decision about *semantics* rather than about
/// effort: building these would mean a file verb whose meaning this program would have to
/// invent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    /// A directory itself: the tree's shape is the document's, not the caller's.
    LayoutIsNotWritable,
    /// Editing text through a byte stream has no honest in-place semantics.
    TextIsNotAByteStream,
    /// Replacing an image is a plausible transform operation later, and is not one now.
    ImageReplacementNotDesigned,
    /// A derived artefact: writing it would be writing the answer instead of the question.
    Derived,
    /// Renaming within `pages/` is a reorder, and position-names make its meaning ambiguous.
    ReorderIsAmbiguous,
    /// RFC 0003 section 5.2 states five write verbs, and this is not one of them.
    NotOneOfTheFiveVerbs,
}

impl Reason {
    /// The sentence a face puts in front of a person, and the one a mount logs.
    ///
    /// RFC 0003 section 5.3: FUSE "returns `EROFS` for the derived directories and `EPERM` with
    /// no message channel — which is FUSE's poverty, and why the mount also logs each refusal's
    /// sentence to its own stderr/journal". So the sentence lives here, where both faces reach
    /// it, rather than in either of them.
    #[must_use]
    pub fn sentence(self) -> &'static str {
        match self {
            Self::LayoutIsNotWritable => {
                "this directory is the document's own shape: its entries appear and disappear \
                 as the document changes, and creating one here would name nothing in the file"
            }
            Self::TextIsNotAByteStream => {
                "editing a page's text through a byte stream has no honest in-place meaning — a \
                 replacement needs a caret, a line box and an answer about the font, which is \
                 RFC 0005's subject and not this face's"
            }
            Self::ImageReplacementNotDesigned => {
                "replacing an embedded image is not supported yet: it is a transform operation \
                 nobody has designed, and writing the bytes back into the stream would change \
                 what the page draws without changing what it says"
            }
            Self::Derived => {
                "this file is derived from the document and is not part of it; write to what it \
                 was derived from instead"
            }
            Self::NotOneOfTheFiveVerbs => {
                "RFC 0003 section 5.2 states five write verbs — insert pages, delete a page, \
                 embed a file, remove an embedded file, set the §14.3.3 entries — and this is \
                 not one of them; what it would mean to the document has not been designed"
            }
            Self::ReorderIsAmbiguous => {
                "a rename inside pages/ is a reorder, and these names are positions rather than \
                 identities, so \"move 0007 to 0002\" has two readings and a file manager's \
                 drag emits a storm of them; reorder with the pdf-transform command line \
                 instead"
            }
        }
    }
}

/// One row of the layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Route {
    /// The path pattern, with `NNNN` for a page ordinal, `MM` for an index within a page, `DPI`
    /// for a resolution directory, `EXT` for a file form the document chooses and `NAME` for a
    /// name the document states. Written out so that the table reads as the tree it describes.
    pub pattern: &'static str,
    /// Directory or file.
    pub kind: Kind,
    /// What produces it.
    pub generator: Generator,
    /// What writing to it and deleting it would each mean.
    pub write: WriteMapping,
    /// The clause the generator rests on, where one does.
    pub clause: Option<&'static str>,
}

/// RFC 0003 section 4's tree, whole.
///
/// The order is the order a listing takes, and a listing of a directory is the rows whose
/// pattern is one component longer than its own.
pub static LAYOUT: &[Route] = &[
    Route {
        pattern: "/",
        kind: Kind::Directory,
        generator: Generator::Root,
        write: WriteMapping::refused(Reason::LayoutIsNotWritable),
        clause: None,
    },
    Route {
        pattern: "/pages",
        kind: Kind::Directory,
        generator: Generator::PageOrdinals,
        write: WriteMapping::refused(Reason::LayoutIsNotWritable),
        clause: Some("ISO 32000-2 §7.7.3.2"),
    },
    Route {
        pattern: "/pages/NNNN.pdf",
        kind: Kind::File,
        generator: Generator::ExtractedPage,
        write: WriteMapping {
            on_write: Write::InsertPages,
            on_delete: Write::DeletePage,
        },
        clause: Some("ISO 32000-2 §7.7.3.4"),
    },
    Route {
        pattern: "/renders",
        kind: Kind::Directory,
        generator: Generator::Resolutions,
        write: WriteMapping::refused(Reason::Derived),
        clause: None,
    },
    Route {
        pattern: "/renders/DPI",
        kind: Kind::Directory,
        generator: Generator::RenderOrdinals,
        write: WriteMapping::refused(Reason::Derived),
        clause: None,
    },
    Route {
        pattern: "/renders/DPI/NNNN.png",
        kind: Kind::File,
        generator: Generator::RenderedPage,
        write: WriteMapping::refused(Reason::Derived),
        clause: Some("ISO 32000-2 §8.3.2.3"),
    },
    Route {
        pattern: "/images",
        kind: Kind::Directory,
        generator: Generator::ImagePageOrdinals,
        write: WriteMapping::refused(Reason::LayoutIsNotWritable),
        clause: Some("ISO 32000-2 §7.7.3.2"),
    },
    Route {
        pattern: "/images/NNNN",
        kind: Kind::Directory,
        generator: Generator::ImageInventory,
        write: WriteMapping::refused(Reason::LayoutIsNotWritable),
        clause: Some("ISO 32000-2 §8.9.5"),
    },
    Route {
        pattern: "/images/NNNN/NAME",
        kind: Kind::File,
        generator: Generator::ExtractedImage,
        write: WriteMapping::refused(Reason::ImageReplacementNotDesigned),
        clause: Some("ISO 32000-2 §8.9.5"),
    },
    Route {
        pattern: "/text",
        kind: Kind::Directory,
        generator: Generator::TextOrdinals,
        write: WriteMapping::refused(Reason::LayoutIsNotWritable),
        clause: None,
    },
    Route {
        pattern: "/text/NNNN.txt",
        kind: Kind::File,
        generator: Generator::PageText,
        write: WriteMapping::refused(Reason::TextIsNotAByteStream),
        clause: Some("ISO 32000-2 §14.8.2.5.1"),
    },
    Route {
        pattern: "/text/document.txt",
        kind: Kind::File,
        generator: Generator::DocumentText,
        write: WriteMapping::refused(Reason::TextIsNotAByteStream),
        clause: Some("ISO 32000-2 §14.8.2.5.1"),
    },
    Route {
        pattern: "/attachments",
        kind: Kind::Directory,
        generator: Generator::AttachmentInventory,
        write: WriteMapping::refused(Reason::LayoutIsNotWritable),
        clause: Some("ISO 32000-2 §7.11.4"),
    },
    Route {
        pattern: "/attachments/NAME",
        kind: Kind::File,
        generator: Generator::ExtractedAttachment,
        write: WriteMapping {
            on_write: Write::EmbedFile,
            on_delete: Write::RemoveAttachment,
        },
        clause: Some("ISO 32000-2 §7.11.4"),
    },
    Route {
        pattern: "/meta",
        kind: Kind::Directory,
        generator: Generator::MetaNames,
        write: WriteMapping::refused(Reason::LayoutIsNotWritable),
        clause: None,
    },
    Route {
        pattern: "/meta/info.json",
        kind: Kind::File,
        generator: Generator::Information,
        write: WriteMapping {
            on_write: Write::SetInformation,
            on_delete: Write::Refused(Reason::NotOneOfTheFiveVerbs),
        },
        clause: Some("ISO 32000-2 §14.3.3"),
    },
    Route {
        pattern: "/meta/xmp.xml",
        kind: Kind::File,
        generator: Generator::MetadataStream,
        write: WriteMapping::refused(Reason::Derived),
        clause: Some("ISO 32000-2 §14.3.2"),
    },
    Route {
        pattern: "/meta/outline.json",
        kind: Kind::File,
        generator: Generator::Outline,
        write: WriteMapping::refused(Reason::Derived),
        clause: Some("ISO 32000-2 §12.3.3"),
    },
];

/// The rows a directory lists: those whose pattern is exactly one component longer.
#[must_use]
pub fn children(directory: &str) -> Vec<&'static Route> {
    let prefix = if directory == "/" {
        String::from("/")
    } else {
        format!("{directory}/")
    };
    LAYOUT
        .iter()
        .filter(|route| {
            route.pattern != directory
                && route
                    .pattern
                    .strip_prefix(&prefix)
                    .is_some_and(|rest| !rest.contains('/'))
        })
        .collect()
}

/// What a resolved path captured from its pattern, for the generator to use.
pub type Resolved = (&'static Route, Captures);
