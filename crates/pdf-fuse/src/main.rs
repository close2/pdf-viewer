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
//! # ISO 32000-2 §7.6.4.1's password, and why it never appears in argv
//!
//! A mount cannot prompt. The clause's second step — "the interactive PDF processor should prompt
//! for a password" — has no one to put the question to at the `readdir` that needs the answer, so
//! the password is supplied here, once, while this program still has a caller, and the mount
//! holds it for its life ([`pdf_vfs::Vfs::with_password`]). Without one a mount opens whatever
//! the clause's default user password opens and refuses the rest by name.
//!
//! **`--password-fd <n>` and no `--password`**, which is the transform suite's decision and its
//! reason: an argv password is visible in `/proc` and in every shell history. An interactive
//! prompt that suppresses echo needs a terminal-mode dependency this tree has not taken.
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
use pdf_vfs::{ConfinedWorkers, FileBacking, MachineFaces, Secret, Vfs};

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
    /// The descriptor ISO 32000-2 §7.6.4.1's password is to be read from, where one was named.
    ///
    /// **The number rather than the password, and that is the point of the field.** A `Secret` in
    /// here would be a password inside a `Debug`, a `PartialEq` and whatever a test prints; the
    /// descriptor is an integer, and [`run`] reads one line from it at the moment the mount is
    /// built.
    password_fd: Option<u32>,
}

/// The usage line, which is also the whole of this program's interface.
const USAGE: &str = "usage: quorrafs [--allow-other] [--foreground] [--machine-fonts] \
                     [--password-fd <n>] <file.pdf> <mountpoint>\n  --password-fd <n>  read \
                     §7.6.4.1's password, one line, from descriptor n; there is no --password, \
                     because argv is public";

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
    let mut password_fd = None;
    // `--password-fd 3` as well as `--password-fd=3`, because the transform suite's command line
    // accepts both spellings of every valued flag and a person moving between the two programs
    // should not have to remember which.
    let mut wants_a_descriptor = false;
    for argument in raw {
        if wants_a_descriptor {
            wants_a_descriptor = false;
            password_fd = Some(descriptor(argument.to_str().unwrap_or_default())?);
            continue;
        }
        match argument.to_str() {
            Some("--allow-other") => allow_other = true,
            // The whole of this face's answer to `doc/todo/59`: one word, off unless it is
            // written. What it turns on is *this* process opening a font file and handing its
            // descriptor to the worker — never the worker opening anything.
            Some("--machine-fonts") => faces = MachineFaces::Offered,
            Some("--foreground") => {}
            Some("--password-fd") => wants_a_descriptor = true,
            Some(flag) if flag.starts_with("--password-fd=") => {
                password_fd = Some(descriptor(&flag["--password-fd=".len()..])?);
            }
            // Named so that its absence is a decision rather than an oversight: an argv password
            // is in `/proc` and in every shell history, which is the transform suite's own reason
            // for having no such flag either.
            Some(flag) if flag == "--password" || flag.starts_with("--password=") => {
                return Err(format!(
                    "there is no --password, because argv is public — use --password-fd \
                     <n>\n{USAGE}"
                ));
            }
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
        password_fd,
    })
}

/// A descriptor number, or what is wrong with what was written.
fn descriptor(written: &str) -> Result<u32, String> {
    written
        .parse()
        .map_err(|_| format!("--password-fd wants a descriptor number, not {written:?}\n{USAGE}"))
}

/// One line from an open descriptor, without its line ending — what a script hands over.
///
/// The same shape as the transform suite's `password_from`, deliberately: two programs of one
/// project should take a password the same way, and `/dev/fd/<n>` is how a descriptor is opened
/// without a dependency. The line is moved into the [`Secret`] rather than copied, and the
/// `String` it was read into is what `Secret`'s own `Drop` then clears.
fn password_from(fd: u32) -> Result<Secret, String> {
    let file = std::fs::File::open(format!("/dev/fd/{fd}"))
        .map_err(|error| format!("--password-fd {fd}: cannot be read ({error})"))?;
    let mut line = String::new();
    std::io::BufRead::read_line(&mut std::io::BufReader::new(file), &mut line)
        .map_err(|error| format!("--password-fd {fd}: cannot be read ({error})"))?;
    while line.ends_with(['\n', '\r']) {
        line.pop();
    }
    Ok(Secret::from(line))
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
    let backing = Box::new(FileBacking::new(arguments.document.clone()));
    // RFC 0003 section 6. There is no flag for *whether* the worker is confined, and there is one
    // for what it may be handed: `--machine-fonts` is `doc/todo/59`'s port, and it widens nothing
    // the worker can reach on its own.
    let workers = Box::new(ConfinedWorkers {
        faces: arguments.faces,
    });
    let config = pdf_vfs::Config::default();
    // §7.6.4.1's password is read here and nowhere else: before the mount exists, while this
    // program still has a terminal and a caller, and once — because the mount holds it from then
    // on and a `readdir` has nobody to ask.
    let vfs = match arguments.password_fd {
        Some(fd) => Vfs::with_password(backing, workers, config, password_from(fd)?),
        None => Vfs::new(backing, workers, config),
    };
    let face = Arc::new(Face::new(
        vfs,
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
                password_fd: None,
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

    /// ISO 32000-2 §7.6.4.1's password: the descriptor, in both spellings, and never in argv.
    #[test]
    fn the_password_is_a_descriptor_and_never_an_argument() {
        let read = |words: &[&str]| arguments(words.iter().map(|word| OsString::from(*word)));
        for spelling in [
            ["--password-fd", "3", "doc.pdf", "mnt"],
            ["--password-fd=3", "doc.pdf", "mnt", "--foreground"],
        ] {
            assert_eq!(
                read(&spelling).map(|arguments| arguments.password_fd),
                Ok(Some(3)),
                "both spellings of a valued flag, as the transform suite accepts them: {spelling:?}"
            );
        }
        // The refusal names the replacement, because a person who typed `--password` has a
        // password in hand and needs to be told where to put it rather than that this is not an
        // option. An argv password is in `/proc` and in every shell history.
        for written in ["--password", "--password=hunter2"] {
            let said = read(&[written, "doc.pdf", "mnt"]).expect_err("no --password");
            assert!(
                said.contains("argv is public") && said.contains("--password-fd"),
                "the refusal says why and what to use instead: {said}"
            );
        }
        // A descriptor is a number. Trap 5 again: `--password-fd stdin` is refused rather than
        // parsed as something and silently mounted without a password.
        assert!(read(&["--password-fd", "stdin", "doc.pdf", "mnt"]).is_err());
        assert!(read(&["--password-fd=", "doc.pdf", "mnt"]).is_err());
    }
}
