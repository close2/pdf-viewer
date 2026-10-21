//! ISO 32000-2 §12.6.4.6's launched file that is not a PDF: a program, started never, or a
//! document, handed to this machine's own handler at the level the reader set (ADR 2171).
//!
//! Table 207 makes `/F` "[t]he application that shall be launched or the document that shall be
//! opened or printed", and §12.6.4.6 opens with the same two acts: "[a] launch action launches an
//! application or opens or prints a document". §7.5.2's header tells a PDF from everything else and
//! `viewer_core` opens a PDF itself; **nothing in the standard tells an application from any other
//! document**, so which of the two a file that is not a PDF is taken to be is this host's choice,
//! made here once and written down as one:
//!
//! - **A program** is a file this machine would execute: one whose permissions let it be executed,
//!   one that begins as an executable or a script does, or one whose name ends in a kind this
//!   machine's handlers run rather than show ([`RUN_BY_HANDLERS`]). It is never started, at any
//!   level, which is ADR 1358's departure and principle 3's reason: this reader starts no program on
//!   a document's say-so.
//! - **A document** is every other file, and handing one to the desktop's handler is the act
//!   `CLAUDE.md`'s clause-13 entry admits for a media file — "handed to this machine's own player"
//!   at the reader's level — on the same footing, so it is [`LaunchedDocuments`]' four levels,
//!   `refuse` by default.
//!
//! The classification errs one way only: a document taken for a program is refused with a sentence,
//! and a program taken for a document would be started by a handler. So every test that can say
//! *program* is asked, and none that could say *document* overrides one.

use std::path::Path;

use super::URI_HANDLER;

/// The word a person types to say what this reader does with a launched file that is a document
/// and not a PDF.
///
/// [`super::REMOTE_DOCUMENTS`]' four words and direction, for the act [`LaunchedDocuments`] governs.
/// **A value of its own** for ADR 1227's reason one act along: a reader who set
/// `--remote-documents=open` said which PDFs this program may parse, and has said nothing about
/// another program on this machine being given a file a document named.
pub const LAUNCHED_DOCUMENTS: &str = "--launched-documents=";

/// File-name endings this machine's handlers run rather than show, so a file named so is a
/// program whatever its permissions say.
///
/// `desktop` is the one that matters most and is the reason the list exists: the freedesktop.org
/// handler `xdg-open` starts the application a `.desktop` entry names. The rest are the kinds a
/// desktop's handler hands to an interpreter or a loader on the systems this program is built for.
/// Compared without regard to case.
pub const RUN_BY_HANDLERS: [&str; 15] = [
    "appimage", "bat", "cmd", "com", "command", "desktop", "exe", "jar", "msi", "ps1", "run",
    "scr", "sh", "vbs", "wsf",
];

/// The first bytes of a file that is executed rather than shown: an ELF image, a PE image, a script
/// with an interpreter line, and the four Mach-O and fat-binary magics.
const EXECUTABLE_STARTS: [&[u8]; 7] = [
    b"\x7fELF",
    b"MZ",
    b"#!",
    b"\xfe\xed\xfa\xce",
    b"\xfe\xed\xfa\xcf",
    b"\xce\xfa\xed\xfe",
    b"\xcf\xfa\xed\xfe",
];

/// What this reader does when a launch action names a file that is a document and not a PDF.
///
/// `CLAUDE.md` principle 3's four levels over the one act [`launched`] decides.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LaunchedDocuments {
    /// Hand nothing over, and say which file the document named.
    ///
    /// **The default**: a document is not a reader's to hand to another program until the reader
    /// says it is, and the clause-13 media file it mirrors waits for the same word (ADR 2171).
    #[default]
    Refuse,
    /// Put the file to the person first, and hand it over on a `yes`.
    Ask,
    /// Hand it over, and say afterwards that it was handed over without asking.
    Warn,
    /// Hand it over.
    Open,
}

impl LaunchedDocuments {
    /// All four, in the order a person reads them: least permissive to most.
    pub const ALL: [Self; 4] = [Self::Refuse, Self::Ask, Self::Warn, Self::Open];

    /// The word [`LAUNCHED_DOCUMENTS`] takes for this level.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Refuse => "refuse",
            Self::Ask => "ask",
            Self::Warn => "warn",
            Self::Open => "open",
        }
    }

    /// The level a word names, or `None` for a word that names none.
    #[must_use]
    pub fn parse(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|level| level.as_str() == word)
    }
}

/// Reads [`LAUNCHED_DOCUMENTS`]' word onto a level, or says what is wrong with it.
///
/// # Errors
///
/// The sentence to print, naming every word this option takes — `links`' shape.
pub fn launched_documents(word: &str) -> Result<LaunchedDocuments, String> {
    LaunchedDocuments::parse(word).ok_or_else(|| {
        format!(
            "{LAUNCHED_DOCUMENTS}{word}: no such level. One of {}",
            LaunchedDocuments::ALL
                .map(LaunchedDocuments::as_str)
                .join(", ")
        )
    })
}

/// What a host does with a launched file it has read and found not to be a PDF.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Launched {
    /// A program: started at no level. The sentence says which and why.
    Program(String),
    /// Hand it to [`hand_over`] now, then say this.
    Hand(String),
    /// Put this question to the person, and hand it over on a `yes`.
    Ask(crate::restriction::Question),
    /// Do not hand it over. The sentence says why.
    Refuse(String),
}

/// What becomes of the file a launch action named, once its bytes are read — or `None` where they
/// are a PDF, which `viewer_core` opens itself.
///
/// `path` is where the remote-documents rule put the file and `name` what the document called it;
/// the level is asked only of a document, never of a program.
#[must_use]
pub fn launched(
    path: &Path,
    name: &str,
    bytes: &[u8],
    level: LaunchedDocuments,
) -> Option<Launched> {
    if viewer_core::states_a_pdf_header(bytes) {
        return None;
    }
    if let Some(why) = program(path, bytes) {
        return Some(Launched::Program(format!(
            "Launch: {name} is a program — {why} — and this reader starts no program a document \
             names (ISO 32000-2 §12.6.4.6)"
        )));
    }
    Some(match level {
        LaunchedDocuments::Refuse => Launched::Refuse(format!(
            "Launch: declined — {name} is a document that is not a PDF, and this reader is set to \
             hand no such document to this machine's own handler ({LAUNCHED_DOCUMENTS}{}); \
             {LAUNCHED_DOCUMENTS}{} puts it to you first",
            LaunchedDocuments::Refuse.as_str(),
            LaunchedDocuments::Ask.as_str()
        )),
        LaunchedDocuments::Ask => Launched::Ask(asked_to_hand_over(name, path)),
        LaunchedDocuments::Warn => Launched::Hand(format!(
            "Launch: {name} was handed to this machine's own handler ({URI_HANDLER}) without \
             asking you first, because this reader is set to {LAUNCHED_DOCUMENTS}{}",
            LaunchedDocuments::Warn.as_str()
        )),
        LaunchedDocuments::Open => Launched::Hand(handed(name)),
    })
}

/// The sentence a hand-over the person agreed to, or the `open` level, is said with.
#[must_use]
pub fn handed(name: &str) -> String {
    format!("Launch: {name} was handed to this machine's own handler ({URI_HANDLER})")
}

/// Why `path` is a program, or `None` where nothing says it is.
fn program(path: &Path, bytes: &[u8]) -> Option<&'static str> {
    if executable_by_permission(path) {
        return Some("its permissions let it be executed");
    }
    if EXECUTABLE_STARTS
        .iter()
        .any(|start| bytes.starts_with(start))
    {
        return Some("it begins as an executable or a script does");
    }
    let ending = path.extension()?.to_str()?.to_ascii_lowercase();
    RUN_BY_HANDLERS
        .contains(&ending.as_str())
        .then_some("its name ends in a kind this machine's handlers run")
}

/// Whether any of the file's execute bits is set. A file whose permissions cannot be read is taken
/// to be executable, which is the direction the module's comment says every doubt goes.
#[cfg(unix)]
fn executable_by_permission(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt as _;
    match std::fs::metadata(path) {
        Ok(stated) => stated.permissions().mode() & 0o111 != 0,
        Err(_) => true,
    }
}

/// A system with no execute bit leaves the question to the file's start and its name.
#[cfg(not(unix))]
const fn executable_by_permission(_path: &Path) -> bool {
    false
}

/// What a window puts in front of a person at [`LaunchedDocuments::Ask`].
///
/// `asked_to_open_remote`'s two-string shape, naming the file the document asked for and the path
/// it resolved to, and saying which program would be given it — the desktop's handler, which is the
/// person's setting and not this reader's.
#[must_use]
pub fn asked_to_hand_over(name: &str, path: &Path) -> crate::restriction::Question {
    crate::restriction::Question {
        reasons: format!(
            "This document asks to launch {name}, which is {}. It is not a PDF, so this reader \
             would hand it to this machine's own handler ({URI_HANDLER}), which opens it in the \
             program this machine opens such a file with (ISO 32000-2 §12.6.4.6, Table 207).",
            path.display()
        ),
        choice: format!(
            "You have set this reader to ask before handing over a document that is not a PDF \
             ({LAUNCHED_DOCUMENTS}{}). \"{}\" hands this one over and leaves the level where it \
             is; \"{}\" keeps it here. {LAUNCHED_DOCUMENTS}{} stops the question being asked, and \
             {LAUNCHED_DOCUMENTS}{} stops such a file being handed over at all.",
            LaunchedDocuments::Ask.as_str(),
            crate::restriction::GO_AHEAD,
            crate::restriction::DO_NOT,
            LaunchedDocuments::Open.as_str(),
            LaunchedDocuments::Refuse.as_str()
        ),
    }
}

/// Hands a launched document to [`URI_HANDLER`], and returns once it is started.
///
/// # Errors
///
/// The sentence to say where the handler could not be started.
pub fn hand_over(path: &Path) -> Result<(), String> {
    super::start_handler(path.as_os_str(), path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::{Launched, LaunchedDocuments, launched, launched_documents};

    /// A file this crate's test writes into the target directory, with `bytes` and, on Unix,
    /// `mode` as its permissions.
    fn written(name: &str, bytes: &[u8], mode: u32) -> std::path::PathBuf {
        let directory = std::env::temp_dir().join(format!("launched-{}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("a scratch directory");
        let path = directory.join(name);
        std::fs::write(&path, bytes).expect("the file is written");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode))
                .expect("the mode is set");
        }
        #[cfg(not(unix))]
        let _ = mode;
        path
    }

    /// A PDF is the core's to open, and nothing here decides about it.
    #[test]
    fn a_pdf_is_not_this_modules_question() {
        let path = written("a.pdf", b"%PDF-2.0\n", 0o644);
        assert_eq!(
            launched(&path, "a.pdf", b"%PDF-2.0\n", LaunchedDocuments::Open),
            None
        );
    }

    /// A program is refused at every level, the most permissive included, and says why — each of
    /// the three tests that can say *program*, one at a time (ISO 32000-2 §12.6.4.6, ADR 1358).
    #[test]
    fn a_program_is_started_at_no_level() {
        let cases = [
            (written("tool", b"plain words", 0o755), "permissions"),
            (written("tool.bin", b"\x7fELF\x02\x01", 0o644), "begins as"),
            (
                written("start.desktop", b"[Desktop Entry]", 0o644),
                "name ends",
            ),
            (written("RUN.SH", b"echo", 0o644), "name ends"),
        ];
        for (path, why) in cases {
            let bytes = std::fs::read(&path).expect("read back");
            for level in LaunchedDocuments::ALL {
                match launched(&path, "named", &bytes, level) {
                    Some(Launched::Program(said)) => {
                        assert!(said.contains(why) && said.contains("§12.6.4.6"), "{said}");
                    }
                    other => panic!("{} at {level:?}: {other:?}", path.display()),
                }
            }
        }
    }

    /// A document is the level's: refused by default, put to the person at `ask`, handed over and
    /// said at `warn` and `open` (ADR 2171).
    #[test]
    fn a_document_that_is_not_a_pdf_is_the_levels() {
        let path = written("letter.txt", b"Dear reader", 0o644);
        let at = |level| launched(&path, "letter.txt", b"Dear reader", level);
        assert_eq!(LaunchedDocuments::default(), LaunchedDocuments::Refuse);
        assert!(matches!(
            at(LaunchedDocuments::Refuse),
            Some(Launched::Refuse(said)) if said.contains("--launched-documents=refuse")
        ));
        assert!(matches!(
            at(LaunchedDocuments::Ask),
            Some(Launched::Ask(question))
                if question.reasons.contains("letter.txt") && question.reasons.contains("xdg-open")
        ));
        assert!(matches!(
            at(LaunchedDocuments::Warn),
            Some(Launched::Hand(said)) if said.contains("without asking")
        ));
        assert!(matches!(
            at(LaunchedDocuments::Open),
            Some(Launched::Hand(_))
        ));
    }

    /// The word reads all four levels and names them where it is wrong.
    #[test]
    fn the_word_reads_four_levels() {
        for level in LaunchedDocuments::ALL {
            assert_eq!(launched_documents(level.as_str()), Ok(level));
        }
        let wrong = launched_documents("maybe").expect_err("not a level");
        assert!(wrong.contains("refuse, ask, warn, open"), "{wrong}");
    }
}
