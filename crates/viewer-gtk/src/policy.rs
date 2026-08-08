//! The two decisions `doc/todo/30` says a host owns, and why they are a host's.
//!
//! `viewer_core`'s rule 2 is that the crate has no filesystem: "[a] document naming a file is a
//! document asking this machine for something, and whether to give it is not a rendering
//! decision." Both decisions below are that rule reaching a person.
//!
//! **§12.7.6.4** — a form's import-data action names a file, and the clause makes performing it a
//! `shall` while saying nothing at all about *which* files a document may name, because that is a
//! property of the processor rather than of the format. So the policy is stated here, in a host,
//! and it is deliberately the narrowest one that still performs the action.
//!
//! **§7.6.4.1** — an encrypted document asks for a password. The clause requires a processor to
//! try the empty user password and then to ask; asking is a window, and a window is a host's.
//! `viewer_core::Event::PasswordRequired` is where that arrives and `src/host.rs` is what puts a
//! [`gtk4::PasswordEntry`] in front of it.

use std::path::{Component, Path, PathBuf};

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
/// `tests/import_policy.rs` does. Reading the bytes is [`read_import`]'s.
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
