//! The command line, and the two helpers every module of this window says a message or a path with.
//!
//! Moved out of the dispatch file with `Host`'s methods (ADR 2352).

use std::path::PathBuf;

/// The command line: a document, optionally `--trace[=topics]`, `--cpu` and two levels.
pub(crate) struct Arguments {
    /// The file to open.
    pub(crate) path: PathBuf,
    /// Which trace topics to print.
    pub(crate) topics: u8,
    /// `--cpu`: no graphics device — the flagship's flag with the flagship's meaning (ADR
    /// 0221): no instance is created, so no driver is loaded, and the window presents through
    /// the processor.
    pub(crate) processor: bool,
    /// `--machine-fonts`: `doc/todo/59`'s resource port, off unless it is asked for.
    ///
    /// It gives the worker no capability at all — the descriptor is opened *here* — and what it
    /// buys is a page that names an uninstalled face drawn in that face rather than in a
    /// compiled-in Latin one (ADRs 0870, 0880).
    pub(crate) faces: bool,
    /// §12.8.1's anchors, §8.10.4's target documents and §8.11.4.4's reader, in the words every
    /// window takes (ADRs 1580, 1581).
    pub(crate) reader: viewer_host::ReaderWords,
    /// §12.6.4.8: what this window does when a link asks for a URI, per [`viewer_host::LINKS`].
    pub(crate) links: viewer_host::Links,
    /// §12.6.4.3: what it does when a document names another PDF file, per
    /// [`viewer_host::REMOTE_DOCUMENTS`].
    pub(crate) remote_documents: viewer_host::RemoteDocuments,
    /// §12.7.6.2's form and an import named by an absolute URI, per [`viewer_host::SUBMISSIONS`]
    /// (ADR 2076).
    pub(crate) submissions: viewer_host::Submissions,
    /// §12.6.4.6's launched document that is not a PDF, per [`viewer_host::LAUNCHED_DOCUMENTS`]
    /// (ADR 2171).
    pub(crate) launched_documents: viewer_host::LaunchedDocuments,
}

/// What the usage line names, after the program's name.
const USAGE: &str = "[--trace[=topics]] [--cpu] [--links=L] [--remote-documents=R] \
                     [--submissions=S] [--launched-documents=D]";

/// A message's `Debug` form cut to a line, for the trace.
///
/// The cut is what the other hosts apply, and it was load-bearing here before it was cosmetic:
/// `Command::Open` used to carry the whole document as a `Vec<u8>` whose `Debug` is five
/// characters a byte, and the first run of this program wrote three quarters of a megabyte of
/// trace for one open. `pdf_syntax::FileBytes` prints its length and how it is held now, and a
/// `Secret` prints no password whatever the length, so what the cut protects is a long event.
pub(crate) fn brief(message: &impl std::fmt::Debug) -> String {
    let whole = format!("{message:?}");
    if whole.chars().count() > 120 {
        let mut cut: String = whole.chars().take(120).collect();
        cut.push('…');
        return cut;
    }
    whole
}

/// A file's own name, or the whole path where it has none.
pub(crate) fn file_name(path: &std::path::Path) -> String {
    path.file_name().map_or_else(
        || path.to_string_lossy().into_owned(),
        |name| name.to_string_lossy().into_owned(),
    )
}

/// The four of the settings words this window carries out: a link, a file the document names, a
/// form sent to a server and a launched document handed to this machine's own handler, each put to
/// the reader on a card (ADRs 2040, 2076, 2171).
const CARRIED: [&str; 4] = [
    viewer_host::LINKS,
    viewer_host::REMOTE_DOCUMENTS,
    viewer_host::SUBMISSIONS,
    viewer_host::LAUNCHED_DOCUMENTS,
];

/// Why a settings word this window does not carry is refused: what the window does instead.
pub(crate) fn not_carried(word: &str) -> &'static str {
    if word == viewer_host::EMBEDDED_DOCUMENTS {
        "this window holds one document and opens none beside it, so an embedded document is \
         refused at every level"
    } else if word == viewer_host::SEPARATIONS {
        "this window asks for no separation simulation: its pages are drawn in the worker as \
         the screen shows them"
    } else {
        "this window sets no level for a document's restrictions: each operation stands at \
         off, where none is refused"
    }
}

/// Reads the arguments or says why not; the usage line is the refusal.
pub(crate) fn arguments() -> Arguments {
    let mut path = None;
    let mut topics = 0u8;
    let mut processor = false;
    let mut faces = false;
    let mut typed = viewer_host::policy::Words::default();
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        // Read as every window reads them (ADRs 1581, 2120, 2156): the reader's words on this
        // side, which has the filesystem, carried across the wire; a settings word this window
        // has no level for refused by name rather than taken and not obeyed.
        let taken = typed
            .take(std::ffi::OsStr::new(&argument), &mut arguments)
            .unwrap_or_else(|complaint| {
                eprintln!("{complaint}");
                std::process::exit(2);
            });
        if let Some(viewer_host::policy::Taken::Setting(word)) = taken
            && !CARRIED.contains(&word)
        {
            eprintln!("{argument}: {}", not_carried(word));
            std::process::exit(2);
        }
        if taken.is_some() {
            // Taken, and in force.
        } else if argument == "--trace" {
            topics = u8::MAX;
        } else if argument == "--cpu" {
            processor = true;
        } else if argument == viewer_host::MACHINE_FONTS {
            faces = true;
        } else if let Some(list) = argument.strip_prefix("--trace=") {
            topics = match viewer_host::parse_topics(list) {
                Ok(topics) => topics,
                Err(said) => {
                    eprintln!("{said}");
                    std::process::exit(2);
                }
            };
        } else if path.is_none() {
            path = Some(PathBuf::from(argument));
        } else {
            eprintln!(
                "usage: quorra-confined {USAGE} [{}] {} document.pdf",
                viewer_host::MACHINE_FONTS,
                viewer_host::ReaderWords::USAGE
            );
            std::process::exit(2);
        }
    }
    let Some(path) = path else {
        eprintln!(
            "usage: quorra-confined {USAGE} [{}] {} document.pdf",
            viewer_host::MACHINE_FONTS,
            viewer_host::ReaderWords::USAGE
        );
        std::process::exit(2);
    };
    // RFC 0008 section 6.3's level is pinned here, and a person who asked for another is told so
    // rather than given a word this window would not obey (`doc/questions/A193`, ADR 1616).
    let asked = typed.reader().scripts;
    if asked != viewer_host::Scripts::Off {
        eprintln!(
            "note: {} {}: {}",
            viewer_host::SCRIPTS,
            asked.as_str(),
            viewer_host::SCRIPTS_PINNED
        );
        typed.scripts(viewer_host::Scripts::Off);
    }
    let (settings, reader) = typed.into_parts();
    Arguments {
        path,
        topics,
        processor,
        // The word, or the environment for a window nobody typed a command line at.
        faces: viewer_host::offers_machine_fonts(faces),
        reader,
        links: settings.links,
        remote_documents: settings.remote_documents,
        submissions: settings.submissions,
        launched_documents: settings.launched_documents,
    }
}
