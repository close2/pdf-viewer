//! `pdffs <file.pdf> <mountpoint>` — RFC 0003 section 7's FUSE face, as a program.
//!
//! ```text
//! pdffs doc.pdf mnt/            # mount, in the foreground
//! ls mnt/pages/                 # one extractable single-page PDF per page
//! cp mnt/pages/0007.pdf ~/      # `cp` *is* page extraction
//! fusermount3 -u mnt/           # done
//! ```
//!
//! # What this program is, and what it is not
//!
//! It is a **frontend**, in RFC 0003 section 6's sense, and the section's own diagram says what
//! that costs it: "two thin, privileged FRONTENDS: file I/O, caching, verb mapping — and NOT ONE
//! BYTE of PDF parsing". So the document is *opened* here and *parsed* in `pdf-vfs-worker`, a
//! separate program under seccomp-BPF, Landlock and an address-space ceiling, reached through
//! [`pdf_vfs::ConfinedWorkers`]. There is no switch to turn that off: a mount is fed hostile
//! bytes by anything that opens a folder, which is what makes it "the most exposed surface this
//! project would ship".
//!
//! The **invalidation thread** is here rather than in the library, and that placement is RFC 0003
//! section 5.4's requirement rather than a preference: the notifications must be issued "from a
//! separate task — separate because issuing them synchronously from a request handler can
//! deadlock against the kernel (documented libfuse hazard)".

#![forbid(unsafe_code)]
#![warn(missing_docs)]
// A program's job is to talk to a person, and this one has no other channel: RFC 0003 section 5.3
// makes the mount's own standard error the place a refusal's sentence goes, because FUSE has none.
#![expect(
    clippy::print_stderr,
    reason = "this binary's diagnostics are its only message channel — RFC 0003 section 5.3"
)]

use std::ffi::{OsStr, OsString};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use fuser::{INodeNo, MountOption, Notifier, SessionACL};
use pdf_fuse::{Face, Mount};
use pdf_vfs::{ConfinedWorkers, FileBacking, MachineFaces, Vfs};

/// How often the notifier thread asks whether the document has changed.
///
/// **A poll rather than an inotify watch, and it is a stated choice.** RFC 0003 section 5.4 names
/// inotify, and the generation key is what an inotify event would send us to ask anyway —
/// "(mtime, size, last `startxref` offset)", which the core validates before every answer. So a
/// watch would be a second dependency and a second thing that can be wrong about one question.
/// What the poll costs is up to this interval of staleness in a *file manager's cached listing*,
/// never in an answer: every operation validates the key itself. What it buys is that a file the
/// mount is not looking at is not watched.
const POLL: Duration = Duration::from_secs(1);

/// What the command line said.
#[derive(Debug, PartialEq, Eq)]
struct Arguments {
    /// The document to mount.
    document: PathBuf,
    /// Where to mount it.
    mountpoint: PathBuf,
    /// Whether other users may see the mount. Off by default, which RFC 0003 section 7 states.
    allow_other: bool,
    /// Whether the confined worker is offered the faces installed on this machine.
    ///
    /// `doc/todo/59`'s resource port, as a flag — the command line's answer to the owner's
    /// "the cli would wrap the access with a flag". Off by default, because the port is a `can`
    /// rather than a `must` and a mount that says nothing is the mount that shipped before it.
    faces: MachineFaces,
}

/// The usage line, which is also the whole of this program's interface.
const USAGE: &str =
    "usage: pdffs [--allow-other] [--foreground] [--machine-fonts] <file.pdf> <mountpoint>";

/// Reads the command line, or says what is wrong with it.
///
/// `--foreground` is accepted and is what this program always does: there is no fork, because a
/// daemon that detached would have nowhere to put the refusal sentences RFC 0003 section 5.3
/// requires it to log. Accepting the flag rather than rejecting it is the kinder half of saying
/// so, and the sentence below says it once at start-up.
fn arguments(raw: impl Iterator<Item = OsString>) -> Result<Arguments, String> {
    let mut positional = Vec::new();
    let mut allow_other = false;
    let mut faces = MachineFaces::Withheld;
    for argument in raw {
        match argument.to_str() {
            Some("--allow-other") => allow_other = true,
            // The whole of this face's answer to `doc/todo/59`: one word, off unless it is
            // written. What it turns on is *this* process opening a font file and handing its
            // descriptor to the worker — never the worker opening anything.
            Some("--machine-fonts") => faces = MachineFaces::Offered,
            Some("--foreground") => {}
            Some("--help" | "-h") => return Err(USAGE.to_owned()),
            Some(flag) if flag.starts_with("--") => {
                return Err(format!("{flag} is not an option of this program\n{USAGE}"));
            }
            _ => positional.push(PathBuf::from(argument)),
        }
    }
    let [document, mountpoint] = positional.as_slice() else {
        return Err(format!(
            "expected a document and a mount point, got {}\n{USAGE}",
            positional.len()
        ));
    };
    Ok(Arguments {
        document: document.clone(),
        mountpoint: mountpoint.clone(),
        allow_other,
        faces,
    })
}

fn main() -> std::process::ExitCode {
    let arguments = match arguments(std::env::args_os().skip(1)) {
        Ok(arguments) => arguments,
        Err(why) => {
            eprintln!("{why}");
            return std::process::ExitCode::FAILURE;
        }
    };
    match run(&arguments) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(why) => {
            eprintln!("pdffs: {why}");
            std::process::ExitCode::FAILURE
        }
    }
}

/// Mounts, serves, and answers what went wrong.
fn run(arguments: &Arguments) -> Result<(), String> {
    if !arguments.document.is_file() {
        return Err(format!("{} is not a file", arguments.document.display()));
    }
    if !arguments.mountpoint.is_dir() {
        return Err(format!(
            "{} is not a directory to mount on",
            arguments.mountpoint.display()
        ));
    }
    let named = arguments.document.display().to_string();
    let face = Arc::new(Face::new(
        Vfs::new(
            Box::new(FileBacking::new(arguments.document.clone())),
            // RFC 0003 section 6. There is no flag for *whether* the worker is confined, and
            // there is one for what it may be handed: `--machine-fonts` is `doc/todo/59`'s port,
            // and it widens nothing the worker can reach on its own.
            Box::new(ConfinedWorkers {
                faces: arguments.faces,
            }),
            pdf_vfs::Config::default(),
        ),
        Box::new(move |sentence: &str| eprintln!("pdffs: {named}: {sentence}")),
    ));

    // `fuser::Config` is `#[non_exhaustive]`, so it is built from its default and then stated
    // field by field — which is also what keeps a new field of theirs from becoming ours.
    let mut config = fuser::Config::default();
    config.mount_options = vec![
        MountOption::FSName(String::from("pdffs")),
        MountOption::Subtype(String::from("pdf")),
        // A mount of a document holds no executable and no device node, and the kernel is where
        // that is enforced rather than per file.
        MountOption::NoExec,
        MountOption::NoDev,
        MountOption::NoSuid,
        MountOption::NoAtime,
    ];
    if arguments.allow_other {
        // The access-control list is where `allow_other` belongs; passing the mount option
        // beside it is what `fuser` rejects as a conflict.
        config.acl = SessionACL::All;
    }

    let session = fuser::Session::new(
        Mount::new(Arc::clone(&face)),
        arguments.mountpoint.as_path(),
        &config,
    )
    .map_err(|why| format!("the mount could not be made: {why}"))?;

    // RFC 0003 section 5.4's separate task.
    let notifier = session.notifier();
    let stop = Arc::new(AtomicBool::new(false));
    let watcher = {
        let face = Arc::clone(&face);
        let stop = Arc::clone(&stop);
        std::thread::Builder::new()
            .name(String::from("pdffs-invalidate"))
            .spawn(move || watch(&face, &notifier, &stop))
            .map_err(|why| format!("the invalidation thread could not be started: {why}"))?
    };

    let outcome = session
        .run()
        .map_err(|why| format!("the session ended in an error: {why}"));
    stop.store(true, Ordering::Relaxed);
    // Joining is what makes "unmounted" mean "nothing is still talking to the kernel"; the thread
    // is asleep for at most one [`POLL`].
    let _ = watcher.join();
    outcome
}

/// The invalidation loop: poll the generation key, and tell the kernel to forget what it holds.
///
/// One notification *set* per change rather than one per name changed, because a change of the
/// key can move any name — RFC 0003 section 5.2's ordinals renumber after every write — and there
/// is nothing cheaper that is also correct. `inval_entry` on a name the kernel is not caching
/// answers `ENOENT`, which is not a failure here: it means there was nothing to forget.
fn watch(face: &Face, notifier: &Notifier, stop: &AtomicBool) {
    let mut key = face.changed_since(None).map(|changed| changed.key);
    while !stop.load(Ordering::Relaxed) {
        std::thread::sleep(POLL);
        if stop.load(Ordering::Relaxed) {
            return;
        }
        let Some(changed) = face.changed_since(key) else {
            continue;
        };
        key = Some(changed.key);
        for (parent, ino, name) in face.known() {
            let _ = notifier.inval_entry(INodeNo(parent), OsStr::new(&name));
            let _ = notifier.inval_inode(INodeNo(ino), 0, 0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Arguments, MachineFaces, arguments};
    use std::ffi::OsString;
    use std::path::PathBuf;

    /// The command line RFC 0003 section 7 states, read.
    #[test]
    fn the_command_line_is_the_rfcs_own() {
        let read = |words: &[&str]| arguments(words.iter().map(|word| OsString::from(*word)));
        assert_eq!(
            read(&["doc.pdf", "mnt"]),
            Ok(Arguments {
                document: PathBuf::from("doc.pdf"),
                mountpoint: PathBuf::from("mnt"),
                allow_other: false,
                faces: MachineFaces::Withheld,
            }),
            "`--allow-other` is off by default, which RFC 0003 section 7 states, and so is \
             `--machine-fonts`, which `doc/todo/59` states"
        );
        assert_eq!(
            read(&["--allow-other", "--foreground", "doc.pdf", "mnt"])
                .map(|arguments| arguments.allow_other),
            Ok(true)
        );
        // `doc/todo/59`'s port: a `can` rather than a `must`, so a mount that does not name it
        // gets the worker that shipped before it.
        assert_eq!(
            read(&["--machine-fonts", "doc.pdf", "mnt"]).map(|arguments| arguments.faces),
            Ok(MachineFaces::Offered)
        );
        // Trap 5: an option this program does not have is said rather than ignored, because an
        // ignored `--read-only` is a mount somebody believes is read-only.
        assert!(read(&["--read-only", "doc.pdf", "mnt"]).is_err());
        assert!(read(&["doc.pdf"]).is_err());
        assert!(read(&["a.pdf", "b.pdf", "mnt"]).is_err());
    }
}
