//! The three decisions `doc/todo/30` says a host owns, and why they are a host's.
//!
//! `viewer_core`'s rule 2 is that the crate has no filesystem: "[a] document naming a file is a
//! document asking this machine for something, and whether to give it is not a rendering
//! decision." Every decision below is that rule reaching a person.
//!
//! **§12.7.6.4** — a form's import-data action names a file, and the clause makes performing it a
//! `shall` while saying nothing at all about *which* files a document may name, because that is a
//! property of the processor rather than of the format. So the policy is stated here, in a host,
//! and it is deliberately the narrowest one that still performs the action.
//!
//! **§O.2.1** — a fragment identifier may name an embedded file, and the annex says in the same
//! row that a processor "may choose to prompt the user or even prevent opening of the file". A
//! URI is somebody else's sentence far more often than a click is, so the two are not one request
//! and a host has to be able to decline the first (ADR 0310). **Two questions rather than one**,
//! since ADR 0431: showing that file in this reader is the `shall` in the row above, and writing
//! it into somebody's directory is the act the caution is about, so they are asked separately and
//! answered differently.
//!
//! **§7.6.4.1** — an encrypted document asks for a password. The clause requires a processor to
//! try the empty user password and then to ask; asking is a window, and a window is a host's.
//! `viewer_core::Event::PasswordRequired` is where that arrives and each host's own window is
//! what puts the platform's secure entry in front of it.

use std::path::{Component, Path, PathBuf};

use viewer_core::Extraction;

/// Why a file a document named was not supplied.
///
/// Typed rather than a string, and every one of them is said out loud: trap 5 on the one path
/// where this host declines to do something a document asked for.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ImportRefusal {
    /// The document did not come from a directory, so there is nothing to resolve against.
    ///
    /// A document opened from a pipe or from bytes this program was handed has no neighbourhood,
    /// and inventing one — the working directory, a home directory — would be answering a
    /// question about *this machine* that nobody asked.
    #[error("the document is not in a known directory")]
    NoDirectory,
    /// The name is not a single path component beside the document.
    ///
    /// Checked as a path rather than as a string, so that a separator this platform recognises
    /// and this program does not cannot slip through: `../secrets`, `/etc/passwd` and a Windows
    /// drive letter are all refused by the same rule.
    #[error("{name} is not a plain file name beside the document")]
    NotAPlainName {
        /// The name the document wrote, unchanged.
        name: String,
    },
}

/// Where §12.7.6.4's named file may be read from, under the narrowest policy that performs it.
///
/// Two rules, and they are the whole policy:
///
/// - the name must be a single path component, so `../…`, an absolute path and a drive-relative
///   one are all refused;
/// - it is resolved against the directory the open document is in, and nowhere else.
///
/// Pure, so that the policy is testable without a filesystem and without a window — which is what
/// `tests/host_mappings.rs` does. Reading the bytes is [`read_import`]'s. (This named a
/// `tests/import_policy.rs` that has never existed in this tree, found by `doc/todo/01`'s eighth
/// sweep on the round it became a program.)
///
/// # Errors
///
/// [`ImportRefusal`], one variant per rule above.
pub fn resolve_import(directory: Option<&Path>, name: &str) -> Result<PathBuf, ImportRefusal> {
    let directory = directory.ok_or(ImportRefusal::NoDirectory)?;
    let named = Path::new(name);
    let mut components = named.components();
    let (Some(Component::Normal(single)), None) = (components.next(), components.next()) else {
        return Err(ImportRefusal::NotAPlainName {
            name: name.to_owned(),
        });
    };
    Ok(directory.join(single))
}

/// The bytes of §12.7.6.4's file, or the sentence saying why not.
///
/// The two halves are separate because only one of them is a decision: [`resolve_import`] is the
/// policy and this is the input/output that follows it, which is also why the policy is the half
/// with tests.
///
/// # Errors
///
/// The refusal, worded for a person, whether it came from the policy or from the filesystem.
pub fn read_import(directory: Option<&Path>, name: &str) -> Result<Vec<u8>, String> {
    let path = resolve_import(directory, name).map_err(|refusal| refusal.to_string())?;
    std::fs::read(&path).map_err(|error| format!("cannot read {}: {error}", path.display()))
}

/// Whether §7.11.4's extracted bytes may be **opened as a document** in this reader.
///
/// **A different question from [`may_write_extracted`], and Annex O asks both of them.** ISO 32000-2
/// §O.2.1, Table Annex O.3's `ef` row, states the requirement first —
///
/// > When used as part of a PDF open parameter, the PDF processor shall open the embedded file
/// > contained within the EmbeddedFiles name tree identified by name .
///
/// — and the caution second, about the same act: "[s]ecurity should be strongly considered when
/// opening an embedded file … a PDF processor may choose to prompt the user or even prevent
/// opening of the file."
///
/// **Both hosts' answers are `Ok` today, and that is a choice with a reason rather than a default.**
/// Showing a file in this reader and writing it into somebody's directory are different acts with
/// different costs: the first is what the `shall` above requires and stays inside a process that
/// `CLAUDE.md`'s principle 3 gives no filesystem and no network, and the second leaves something
/// behind on the machine after the window is closed. So the narrower policy is taken where it
/// costs the annex nothing — the write — and the requirement is carried out where the annex states
/// one.
///
/// It is a function rather than an `Ok(())` inlined at the call site for the reason `CLAUDE.md`'s
/// principle 3 gives: the *policy* is asked once, in a place a host can supply, so that
/// `doc/todo/38`'s *ask* and *warn* levels are a change here and nowhere else. ADR 0431.
///
/// # Errors
///
/// The sentence to say to the person, where the file is not to be opened. No level built today
/// produces one.
pub fn may_open_extracted(asked: Extraction) -> Result<(), String> {
    match asked {
        Extraction::Asked | Extraction::Fragment => Ok(()),
    }
}

/// Whether §7.11.4's extracted bytes may be written to disk without asking a person first.
///
/// **The third decision this module holds, and the annex that needs it says why.** ISO 32000-2
/// §O.2.1, Table Annex O.3's `ef`:
///
/// > Security should be strongly considered when opening an embedded file. When opening a file
/// > that is not from a trusted source, a PDF processor may choose to prompt the user or even
/// > prevent opening of the file.
///
/// The annex attaches that to the one parameter whose effect is a *file* and to no other, and §O.1
/// says why it is different from a click: a fragment identifier is "useful primarily when referring
/// to them from external to the PDF such as a web page or web API", so the sentence that named the
/// file is frequently not the reader's. [`Extraction`] is `viewer-core` saying which of the two
/// happened; this is the one place the three hosts decide what to do about it.
///
/// **`prevent` rather than `prompt`, and it is a choice rather than a reading**: the annex offers
/// both and none of these three hosts has a dialogue to prompt with, so the narrower of the two is
/// taken and said out loud. `doc/todo/38`'s *ask* level is where this becomes the other one, and
/// nothing here has to be revisited for it — the policy is already asked in one place, off a value
/// a host can see. ADR 0310.
///
/// # Errors
///
/// The sentence to say to the person, where the file is not to be written.
pub fn may_write_extracted(asked: Extraction) -> Result<(), String> {
    match asked {
        Extraction::Asked => Ok(()),
        Extraction::Fragment => Err(
            "the URI's fragment asked for this embedded file rather than a person, so it was not \
             written to disk — open it from the files panel to extract it (ISO 32000-2 §O.2.1)"
                .to_owned(),
        ),
    }
}
