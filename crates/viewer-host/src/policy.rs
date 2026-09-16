//! The decisions `doc/todo/30` says a host owns, and why they are a host's.
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
//!
//! **And the fourth, which was three string literals until the seven-hundred-and-twenty-first
//! session** — how much of what a *document* asserts over its reader this program obeys.
//! `viewer_core::Command::Restrict` is the value and `CLAUDE.md` states the rule it exists for:
//! a document's restrictions "are the reader's to set" and "**it shall always be possible to turn
//! them off**". [`IGNORE_RESTRICTIONS`] is the word that turns them off and [`refused`] is the
//! sentence that names it, and they are one unit here for the reason ADR 0604 records: they were
//! apart, and two of the three windows said the word without taking it. [`RESTRICTIONS`] is the
//! rest of that vocabulary: all four levels, one operation at a time, since the owner lifted
//! `doc/todo/38`'s no-interface deferral (ADR 1144).
//!
//! **And a fifth, which used to be four words inside `pdf_model::action::refused`** — whether a
//! submit-form action's request leaves this machine. §12.7.6.2's `shall` is to "transmit the
//! names and values of selected interactive form fields" to a URL, and
//! `pdf_model::submission::compose` has answered every part of that which is about the
//! *document*; what reaches a host is one question about *this machine*, the same kind the
//! §12.7.6.4 paragraph asks about a filesystem. [`may_submit`] is where it is asked and
//! [`submission_note`] is the sentence, and they are one unit here for [`refused`]'s reason
//! (ADR 1062).
//!
//! **And a sixth, which was a `println!` in each of four windows** — whether a link's URI is
//! handed to whatever this machine opens one with. §12.6.4.8's `shall` is that "[a] URI action
//! causes a URI to be resolved", and the same division applies for the third time: what the URI
//! *is* — Table 210's `/URI` against Table 211's `/Base`, then [`resolve_uri`] against the
//! location of the document itself, which is a fact about this machine and so cannot be the
//! core's — is answered before a host sees it, and [`may_open_uri`] is the one question left
//! (ADR 1079).
//!
//! **And a seventh, which ADR 1039 named in the one-thousand-and-twenty-second session and left
//! unbuilt for forty rounds** — §12.8.1's third question, *is the signer anyone to believe*. RFC
//! 5280 section 6.1.1 makes the trust anchors input (d) of nine and says whose choice they are:
//! "The selection of a trust anchor is a matter of policy: it could be the top CA in a
//! hierarchical PKI, the CA that issued the verifier's own certificate(s), or any other CA in a
//! network PKI." `pdf-signature` holds no root, reads no file and asks no clock; this module is
//! the party with all three. [`trust_anchors`] is where the question is asked, and it answers
//! *nobody* unless a person said otherwise — which is ADR 1039's decision unchanged rather than a
//! default chosen here. ADR 1076.
//!
//! **And a ninth** — §8.11.4.4's `User` and `Language` usage categories, which ask who is reading
//! and in what language. Table 100 says what a document may assert about its audience and the
//! clause says what a processor does with it: match the names "with the user's identification",
//! and select content "based on the language and locale of the application". Neither is a fact
//! the file holds, and a document that could assert who is reading would be choosing its own
//! audience. [`audience`] is where the question is asked, and the answer is *nobody, in no stated
//! language* unless a person said otherwise. ADR 1106.

use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use pdf_model::optional_content::{Audience, Reader};
use pdf_model::submission::{Method, Submission};
use pdf_signature::trust::Supply;
use pdf_signature::verdict::Acceptance;
use pdf_signature::x509::Instant;
use viewer_core::{Extraction, Purpose, ReferenceFiles, TrustPolicy};

/// The word a person types to turn a document's restrictions off, in every host that has a
/// command line.
///
/// **One constant rather than three string literals, and a defect this tree shipped is why.**
/// `viewer_core::Event::Refused` was answered in all three windows with a sentence naming
/// `--ignore-restrictions`, and only `quorra` took the word: `quorra-gtk` and
/// `quorra-qt` answered *"--ignore-restrictions is not an option this program has"* and left,
/// so each of them told a person the way out of a refusal and then refused the way out. That is
/// `CLAUDE.md`'s one non-negotiable sentence about restrictions — "it shall always be possible to
/// turn them off" — true in one host of three, and the sentence saying otherwise was the copy
/// that made it look closed (ADR 0604).
///
/// **It names a level rather than a state**, and since the default became `off` it is the level a
/// host would have had anyway: what it still does is say so out loud, in the sentence
/// [`refused`] prints and in a parser that takes the word. [`RESTRICTIONS`] is the rest of the
/// vocabulary — the other three levels, one operation at a time.
pub const IGNORE_RESTRICTIONS: &str = "--ignore-restrictions";

/// The word a person types to set a level *per restriction* — `CLAUDE.md`'s four, one operation at
/// a time.
///
/// **The user interface `doc/todo/38` said this program owed, in the one form every host already
/// has.** A command line is not a menu and this file said so for five hundred sessions; what it is
/// is the channel a person can reach today, in all three windows and the C ABI's caller alike, and
/// the levels behind it are the ones a menu will set when there is one. `--restrictions=off` is
/// still the whole policy at one level, which is what [`IGNORE_RESTRICTIONS`] says in one word.
pub const RESTRICTIONS: &str = "--restrictions=";

/// Reads [`RESTRICTIONS`]'s list onto a policy, or says what is wrong with it.
///
/// `copy:ask,annotate:on,fill:warn` — an operation's word, a colon, one of `off`, `on`, `ask`,
/// `warn`; the operations are `viewer_core::RestrictionPolicy::word`'s and the levels
/// `pdf_model::restriction::Level::as_str`'s, so a person reads the same six words and the same
/// four everywhere this program takes them. A bare level with no colon sets **all six**, which is
/// the spelling `pdf-transform`'s `--restrictions=on` has had since ADR 0803 and is why this takes
/// the same word.
///
/// `standing` is what the policy is before the list is read, so that two of these on one command
/// line compose rather than the second forgetting the first.
///
/// # Errors
///
/// A sentence naming the word that is wrong and what the alternatives are, for a host to print. An
/// empty list is an error rather than a no-op: a person who typed `--restrictions=` meant
/// something.
pub fn restrictions(
    list: &str,
    standing: viewer_core::RestrictionPolicy,
) -> Result<viewer_core::RestrictionPolicy, String> {
    use pdf_model::restriction::Level;
    use viewer_core::{RestrictionLevel, RestrictionPolicy};

    let named = |level: Level| match level {
        Level::Off => RestrictionLevel::Off,
        Level::On => RestrictionLevel::On,
        Level::Ask => RestrictionLevel::Ask,
        Level::Warn => RestrictionLevel::Warn,
    };
    let levels = "off, on, ask or warn";
    if list.is_empty() {
        return Err(format!(
            "--restrictions= wants {levels}, or an operation and one of them"
        ));
    }
    let mut policy = standing;
    for entry in list.split(',') {
        match entry.split_once(':') {
            None => {
                let level = Level::parse(entry)
                    .ok_or_else(|| format!("--restrictions: {entry:?} is not {levels}"))?;
                policy = RestrictionPolicy::uniform(named(level));
            }
            Some((operation, level)) => {
                let operation = RestrictionPolicy::operation_named(operation).ok_or_else(|| {
                    let words: Vec<&str> = RestrictionPolicy::OPERATIONS
                        .iter()
                        .map(|operation| RestrictionPolicy::word(*operation))
                        .collect();
                    format!(
                        "--restrictions: {operation:?} is not one of {}",
                        words.join(", ")
                    )
                })?;
                let level = Level::parse(level)
                    .ok_or_else(|| format!("--restrictions: {level:?} is not {levels}"))?;
                policy = policy.with(operation, named(level));
            }
        }
    }
    Ok(policy)
}

/// The word a person types to let a window's confined worker be given the machine's own faces.
///
/// **`doc/todo/59`'s resource port, as a window's setting.** It is not a permission and it widens
/// nothing: the worker's system-call set is unchanged and it still cannot name a path. What the
/// word turns on is that the *host* — which already opens the document — will match a description
/// the worker sends against the faces installed here, open the file, and hand the descriptor
/// across. Without it a document naming an uninstalled CJK or Arabic face is drawn from the
/// compiled-in Latin faces and the shortfall is reported under §9.10.2 (ADR 0870), which is what
/// this program did before the port existed.
///
/// **Off by default**, like every other decision in this module: the trade ADR 0880 writes down is
/// that the faces are then parsed in an *unconfined* process, and that is a reader's to make.
pub const MACHINE_FONTS: &str = "--machine-fonts";

/// The environment name for the same setting, for a window nobody typed a command line at.
///
/// A window is usually started from a desktop entry or a file manager, so the flag above reaches
/// only the person who runs it from a terminal. This is the same channel, and the same poor
/// interface, ADR 0875 gave the KIO face its restriction level over — and for the same reason: it
/// is what exists until `doc/todo/38`'s user interface is asked for.
pub const MACHINE_FONTS_VARIABLE: &str = "PDF_VIEWER_MACHINE_FONTS";

/// Whether this host offers its confined worker the machine's faces.
///
/// `said` is whether [`MACHINE_FONTS`] appeared on the command line. The environment is consulted
/// only when it did not, so a person who typed the word gets it whatever the environment says, and
/// a word the variable does not define is *off* rather than a guess — the same rule ADR 0875
/// applies to a restriction level a face does not know.
#[must_use]
pub fn offers_machine_fonts(said: bool) -> bool {
    if said {
        return true;
    }
    matches!(
        std::env::var(MACHINE_FONTS_VARIABLE).as_deref(),
        Ok("on" | "1" | "true" | "yes")
    )
}

/// What a window says when [`viewer_core::Event::Refused`] arrives.
///
/// The notes are the document's own reason, which `pdf_model::restriction` worded; what this adds
/// is the two things only a *host* can say — that this reader chose to obey, and which word makes
/// it stop. Both hosts wrote this sentence for themselves and `viewer-ui` wrote a third; the third
/// copy is where they stop agreeing, and here it is where two of them came to name a flag they did
/// not have.
#[must_use]
pub fn refused(notes: &[String]) -> String {
    format!(
        "{} — this reader is obeying that; {IGNORE_RESTRICTIONS} turns it off (CLAUDE.md: a \
         document's restrictions are the reader's to set)",
        notes.join("; ")
    )
}

/// What a window says when [`viewer_core::Event::Warned`] arrives.
///
/// The notes already say the operation was done; what a host adds is which level did it, so that
/// a person reading a status bar can tell a warning from a refusal without reading to the end.
#[must_use]
pub fn warned(notes: &[String]) -> String {
    format!(
        "{} — this reader is set to warn rather than obey (CLAUDE.md: a document's restrictions \
         are the reader's to set)",
        notes.join("; ")
    )
}

/// What a window with no way to put a question says when [`viewer_core::Event::Asking`] arrives,
/// beside the [`viewer_core::Command::Answer`] it sends with `proceed: false`.
///
/// **A window that cannot ask answers no, out loud.** Going ahead on an unanswered question would
/// be the *off* level under another name, and not going ahead is what a closed dialogue means
/// everywhere else — `pdf-transform` makes the same choice for a pipe with `Refusal::Unanswered`.
/// The three windows have no dialogue for this yet, by the owner's word that the gestures follow
/// the mockups (`doc/todo/38`); until one does, this sentence is what keeps the *ask* level from
/// silently behaving like *on*.
#[must_use]
pub fn unanswerable(notes: &[String]) -> String {
    format!(
        "{} — this window cannot ask yet, so it answered no; {IGNORE_RESTRICTIONS} turns the \
         restriction off",
        notes.join("; ")
    )
}

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

/// Which clause asked for a file, in the word a host prints in front of its sentence.
///
/// **One function rather than a literal at each call site**, for the reason [`refused`] records:
/// three windows each wrote `"import-data: …"` by hand, so the day a second purpose arrived every
/// one of them would have told a person that a `/GoToE` was a form import. The word is the
/// action's own name in Table 201, which is what a person can look up.
#[must_use]
pub const fn asked_for(purpose: Purpose) -> &'static str {
    match purpose {
        Purpose::ImportData => "import-data",
        Purpose::TargetRoot => "GoToE",
    }
}

/// What a host says when it will not supply a file a document named.
///
/// The refusal is the filesystem's or [`resolve_import`]'s; what this adds is *which* clause is
/// going without, because a person watching a status line sees only the sentence.
#[must_use]
pub fn supply_note(purpose: Purpose, refusal: &str) -> String {
    format!("{}: declined — {refusal}", asked_for(purpose))
}

/// Whether §12.7.6.2's composed submission may be **transmitted** from this machine.
///
/// §12.7.6.2 says an interactive PDF processor "shall transmit the names and values of selected
/// interactive form fields to a specified uniform resource locator (URL)", and every part of
/// that sentence except the verb is a question about the document, which
/// `pdf_model::submission::compose` has already answered. The verb is a network request, and
/// `CLAUDE.md` principle 3 gives the process that read the file neither a network nor any way to
/// acquire one — so the answer here is a refusal, and it is a refusal about *this program*
/// rather than about the file.
///
/// **A function rather than a refusal written at each call site**, for the reason
/// [`may_open_extracted`] gives and ADR 1062 repeats: the policy is asked once, in a place a
/// host can supply, so that a host which *does* have a network — or `doc/todo/38`'s *ask* and
/// *warn* levels — is a change here and nowhere else. A refusal that cannot become an "ask" is
/// the thing `CLAUDE.md` says to avoid, and one spelled out in four windows is exactly that.
///
/// # Errors
///
/// The sentence to say to the person. Every host in this tree gets one today.
pub fn may_submit() -> Result<(), String> {
    Err("no network — CLAUDE.md principle 3 gives this program none".to_owned())
}

/// What a host says about a submission, whether it sends it or declines.
///
/// The request in one line, because the person who pressed the button is owed *where it was
/// going and what would have gone* rather than the word "declined" on its own. What the
/// composition did not do is not repeated here: `viewer_core` has already put every sentence of
/// `Submission::owed` into an `Event::Reported`, and a host that said both would say them twice.
#[must_use]
pub fn submission_note(submission: &Submission, refused: Option<&str>) -> String {
    let method = match submission.method {
        Method::Get => "GET",
        Method::Post => "POST",
    };
    let what = format!(
        "{method} {} ({}), {} field(s), {} byte(s)",
        submission.url,
        submission.media_type,
        submission.fields,
        submission.body.len(),
    );
    match refused {
        Some(why) => format!("submit-form: declined — {why}. It would have been {what}"),
        None => format!("submit-form: {what}"),
    }
}

/// §12.6.4.8's URI, against the location of the document itself where the action left it partial.
///
/// Table 211's `/Base` is applied in `pdf_model::action` and is the whole of what the *document*
/// states about resolving one. What the clause says when the document states nothing is not a
/// silence and never was:
///
/// > If no base URI is specified, such partial URIs shall be interpreted relative to the location
/// > of the document itself.
///
/// The location of the document is a fact about this machine rather than about the file — a core
/// that "reads through what it was handed, never a path" (`viewer_core` rule 2) cannot know it and
/// every window built on this crate does, because each of them opened the file. So the `shall` is
/// carried out here, by RFC 3986 section 5's own algorithm through [`pdf_model::uri::resolve`],
/// against the `file` URL of the path the host opened. ADR 1079.
///
/// A reference that is already absolute is returned untouched and the path is not even consulted;
/// so is one whose document has no usable location, and [`uri_note`] says so rather than letting a
/// partial reference pass for a resolved one.
#[must_use]
pub fn resolve_uri(document: Option<&Path>, uri: &str) -> String {
    if pdf_model::uri::is_absolute(uri) {
        return uri.to_owned();
    }
    match document.and_then(file_url) {
        Some(base) => pdf_model::uri::resolve(&base, uri),
        None => uri.to_owned(),
    }
}

/// The `file` URL of an absolute path, for [`resolve_uri`]'s base.
///
/// RFC 8089 section 2: "[a] file URI takes the form of `file://<host>/<path>`", with an empty host
/// meaning this machine. Every byte outside RFC 3986's unreserved set is percent-encoded and the
/// separator is not, which is the conservative encoding: encoding a character that need not be is
/// harmless, and leaving one that must be would put a `?` or a `#` from a file name into the
/// query or the fragment of the URI a link resolves to.
///
/// `None` for a relative path — a base has to be absolute for section 5's algorithm to mean
/// anything — and for one that is not UTF-8, which is a narrowing this tree takes deliberately
/// rather than guessing an encoding for somebody's directory name.
fn file_url(path: &Path) -> Option<String> {
    use std::fmt::Write as _;

    let text = path.is_absolute().then(|| path.to_str())??;
    let mut url = String::from("file://");
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' => {
                url.push(char::from(byte));
            }
            // `fmt::Write for String` never answers `Err`, which is why nothing is dropped here:
            // the discard is the infallibility rather than a swallowed failure.
            _ => {
                let _ = write!(url, "%{byte:02X}");
            }
        }
    }
    Some(url)
}

/// Whether §12.6.4.8's resolved URI may be **handed to whatever this machine opens one with**.
///
/// §12.6.4.8 says "[a] URI action causes a URI to be resolved", of a string the clause introduces
/// as one that "identifies (resolves to) a resource on the Internet" — so the verb is reaching
/// that resource, and everything before it is a question about the document that
/// `pdf_model::action` and [`resolve_uri`] have answered. Reaching it is a program this one would
/// have to start, on a string the *document* chose, and that is a decision about this machine.
///
/// **A function rather than a refusal written at each call site**, for [`may_submit`]'s reason and
/// ADR 1062's: the policy is asked once, in a place a host can supply, so a host that does open
/// links — or `doc/todo/38`'s *ask* and *warn* levels, which is what a person would want in front
/// of this one — is a change here and nowhere else. Four windows each said this in a `println!` of
/// their own until the thousand-and-sixty-fifth session, which is exactly the shape `CLAUDE.md`
/// calls a refusal that cannot become an "ask".
///
/// # Errors
///
/// The sentence to say to the person. Two of them, because the clause makes two different things
/// go wrong: a reference nothing could resolve names no resource at all, and a resolved one is
/// declined by this machine.
pub fn may_open_uri(uri: &str) -> Result<(), String> {
    if pdf_model::uri::is_absolute(uri) {
        return Err(
            "this reader opens nothing itself — handing a string the document chose to \
                    another program is a decision about this machine, not about the file"
                .to_owned(),
        );
    }
    Err(
        "it is still a relative reference and names no resource: the document states no /Base \
         and this window could not name its own location (ISO 32000-2 §12.6.4.8)"
            .to_owned(),
    )
}

/// What a host says about a URI action, whether it opens it or declines.
///
/// The URI in one line, because a person who clicked a link is owed *where it went* rather than
/// the word "declined": a link this reader will not follow is still a link whose target somebody
/// may want to copy. [`submission_note`]'s shape, for [`submission_note`]'s reason.
#[must_use]
pub fn uri_note(uri: &str, refused: Option<&str>) -> String {
    match refused {
        Some(why) => format!("link: declined — {why}. The document asked for {uri}"),
        None => format!("link: {uri}"),
    }
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

/// The word a person types to name the certification authorities this reader will believe.
///
/// **ADR 1039's "a host that wants either reads it and passes the anchors in", as a word.** That
/// decision priced two defaults — a compiled-in root programme and the platform's certificate
/// store — and refused both, for three reasons of which the third stands whatever happens to the
/// first two: a list compiled in is a policy hard-coded where no host can reach it, and
/// `CLAUDE.md` principle 3 says the policy is asked once in a place a host can supply. This is
/// that place, and the value after the word is a directory.
///
/// **Not a user interface for it**, which `doc/todo/38` says is not to be built until the project
/// owner asks: it is one policy value [`viewer_core::Command::Trust`] carries, supplied the way
/// this host supplies the sandbox, the backend and the page to open at.
pub const TRUST_ANCHORS: &str = "--trust-anchors";

/// The word a person types to act on a signature whose revocation status could not be determined.
///
/// **ADR 1067's rule is what this does not touch.** A [`pdf_signature::revocation::Revocation::
/// Good`] is "only ever the output of arithmetic this program performed", and an absence of
/// material stays `Unknown` with its reason kept whatever anybody types. What this word decides is
/// whether a *reader* will act on a verdict resting on one — RFC 5280 section 6.3.3's remedy is to
/// fetch a newer list, which is the one branch a document cannot supply and principle 3 gives this
/// program no network for.
pub const ACCEPT_UNKNOWN_REVOCATION: &str = "--accept-unknown-revocation";

/// The word a person types to name the files a reference `XObject` may import a page from.
///
/// **ISO 32000-2 §8.10.4 addresses a `shall` to each of two classes of processor**, and which one
/// this program is depends on whether the target file is in front of it. `CLAUDE.md` principle 3
/// gives the renderer no filesystem, so it never is unless a person puts it there — and a
/// *document* whose `/F` could name a path this machine then opened would be a document choosing
/// what is read off this disk, which is the thing that principle exists to prevent. So the value
/// after the word is a directory, nothing in it is opened on a document's say-so, and §14.4's
/// identifier is what decides which file a reference names.
///
/// **Not a user interface for it**, on [`TRUST_ANCHORS`]'s rule: it is one policy value
/// [`viewer_core::Command::References`] carries, supplied the way this host supplies every other.
pub const REFERENCE_FILES: &str = "--reference-files";

/// The words a person types to say who is reading, for ISO 32000-2 §8.11.4.4's `User` category.
///
/// Three of them because Table 100 makes `/Type` decide what the names beside it mean — "either
/// Ind (individual), Ttl (title or position), or Org (organisation)" — so a document asking which
/// organisation a reader belongs to is asking something a reader's own name does not answer. Each
/// may be repeated; a value is one name.
///
/// **Not a user interface for it**, on [`TRUST_ANCHORS`]'s rule: this is one policy value
/// [`viewer_core::Command::Audience`] carries, supplied the way this host supplies every other.
pub const READER_NAME: &str = "--reader-name";

/// The same, for Table 100's `Ttl`: the title or position held.
pub const READER_TITLE: &str = "--reader-title";

/// The same, for Table 100's `Org`: the organisation.
pub const READER_ORGANISATION: &str = "--reader-organisation";

/// The word a person types to say what language this application is in (§8.11.4.4's `Language`).
///
/// The value is §14.9.2.2's language tag — "a Language-Tag as defined in BCP 47" — such as
/// `es-MX`. Unset is *nobody has said*, under which the category is reported unanswered rather
/// than guessed from a locale this host would have to read the environment for: `CLAUDE.md`
/// principle 2 keeps environment reading off the launch path, and principle 3 keeps it out of
/// anything that decides a mark.
pub const INTERFACE_LANGUAGE: &str = "--interface-language";

/// Why one file in the anchor directory did not become an anchor.
///
/// Typed rather than a string, and every one of them is said out loud: trap 5 on a path where this
/// host declines to do something a *reader* asked for. A person who pointed at a directory of six
/// files and got four anchors is owed the two names, because the verdicts that follow were computed
/// under a store they did not supply.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AnchorRefusal {
    /// The directory could not be listed.
    #[error("cannot read the trust anchor directory {directory}: {error}")]
    DirectoryUnreadable {
        /// The directory as the person named it.
        directory: String,
        /// What the filesystem said.
        error: String,
    },
    /// One file in it could not be read.
    #[error("cannot read {name} in the trust anchor directory: {error}")]
    FileUnreadable {
        /// The file's name within the directory.
        name: String,
        /// What the filesystem said.
        error: String,
    },
    /// A file holds neither a DER certificate nor a PEM `CERTIFICATE` block.
    #[error("{name} in the trust anchor directory is neither DER nor a PEM CERTIFICATE block")]
    NotACertificateFile {
        /// The file's name within the directory.
        name: String,
    },
}

/// **The one place a host answers "which anchors, if any", and the answer is *none* by default.**
///
/// `directory` is what [`TRUST_ANCHORS`] named, or `None` where nobody typed it — and `None`
/// produces [`TrustPolicy::default`], which is [`Supply::none`], which is
/// [`pdf_signature::trust::Trust::NoAnchorSupplied`] for every signature in every document. That
/// chain is the whole of ADR 1039's decision and it is unchanged by this function existing.
///
/// Every file in the directory is offered, in the order the filesystem lists them sorted by name so
/// that two runs over one directory supply the same store in the same order. Two encodings are
/// taken, because both are what a person has on a disk: a file beginning with X.690's `SEQUENCE`
/// tag is DER, and anything else is scanned for PEM `-----BEGIN CERTIFICATE-----` blocks, of which
/// a file may hold any number. Nothing else about a file is checked here — not its extension, not
/// its dates, not its own signature — because `pdf_signature::trust::TrustAnchor::of` states why
/// there would be nothing to check against: "The trust anchor information is trusted because it was
/// delivered to the path processing procedure by some trustworthy out-of-band procedure."
///
/// **The instant comes from this machine's clock**, which is RFC 5280 section 6.1.1's input (b) and
/// is a host's for the same reason the anchors are: `viewer-core` has no clock (rule 3 of
/// `doc/ui-boundary.md`) and `pdf-signature` asks none. A clock before the epoch answers zero
/// rather than panicking, because a verdict is not the place to discover a misconfigured machine.
///
/// The refusals are returned rather than printed: what a host does with them is the host's, and
/// `viewer_core::notes` says the rest of the sentence beside the verdict itself.
#[must_use]
pub fn trust_anchors(
    directory: Option<&Path>,
    accept_unknown_revocation: bool,
) -> (TrustPolicy, Vec<AnchorRefusal>) {
    let acceptance = if accept_unknown_revocation {
        Acceptance::UnknownRevocationAccepted
    } else {
        Acceptance::RevocationMustBeGood
    };
    let Some(directory) = directory else {
        // The acceptance is carried even with no anchors, because dropping a policy value a person
        // typed would make the two words interact: nothing here computes a verdict without an
        // anchor, and a value that survives is a value the next `Command::Trust` does not have to
        // re-derive.
        return (
            TrustPolicy {
                anchors: Supply::none(),
                acceptance,
            },
            Vec::new(),
        );
    };
    let mut refused = Vec::new();
    let mut names: Vec<PathBuf> = match std::fs::read_dir(directory) {
        Ok(entries) => entries.flatten().map(|entry| entry.path()).collect(),
        Err(error) => {
            refused.push(AnchorRefusal::DirectoryUnreadable {
                directory: directory.display().to_string(),
                error: error.to_string(),
            });
            return (
                TrustPolicy {
                    anchors: Supply::none(),
                    acceptance,
                },
                refused,
            );
        }
    };
    names.sort();
    let mut certificates = Vec::new();
    for path in names {
        if path.is_dir() {
            continue;
        }
        let name = path.file_name().map_or_else(
            || path.display().to_string(),
            |name| name.to_string_lossy().into_owned(),
        );
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) => {
                refused.push(AnchorRefusal::FileUnreadable {
                    name,
                    error: error.to_string(),
                });
                continue;
            }
        };
        let found = certificates_in(&bytes);
        if found.is_empty() {
            refused.push(AnchorRefusal::NotACertificateFile { name });
            continue;
        }
        for (index, der) in found.into_iter().enumerate() {
            let labelled = if index == 0 {
                name.clone()
            } else {
                format!("{name} (certificate {})", index.saturating_add(1))
            };
            certificates.push((labelled, der));
        }
    }
    let at = Instant::from_unix_seconds(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .and_then(|since| i64::try_from(since.as_secs()).ok())
            .unwrap_or(0),
    );
    (
        TrustPolicy {
            anchors: Supply::of(
                certificates,
                format!("{} (--trust-anchors)", directory.display()),
                at,
            ),
            acceptance,
        },
        refused,
    )
}

/// A file in the reference directory this host will not offer the core.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ReferenceRefusal {
    /// The directory could not be listed.
    #[error("cannot read the reference file directory {directory}: {error}")]
    DirectoryUnreadable {
        /// The directory as the person named it.
        directory: String,
        /// What the filesystem said.
        error: String,
    },
    /// One file in it could not be read.
    #[error("cannot read {name} in the reference file directory: {error}")]
    FileUnreadable {
        /// The file's name within the directory.
        name: String,
        /// What the filesystem said.
        error: String,
    },
}

/// **The one place a host answers "which target documents, if any", and the answer is *none* by
/// default.**
///
/// `directory` is what [`REFERENCE_FILES`] named, or `None` where nobody typed it — and `None`
/// produces an empty [`viewer_core::ReferenceFiles`], under which every reference `XObject` in
/// every document draws §8.10.4.1's proxy and reports nothing. That is the clause's own provision
/// for a processor with no target file, and it is what this program did before this function
/// existed.
///
/// Every file in the directory is offered, sorted by name so that two runs over one directory
/// supply the same files in the same order. **Nothing here decides whether a file is a PDF**, and
/// nothing here compares a name against anything a document said: the core opens each file and
/// matches §14.4's identifier, which is the match ISO 32000-2 §14.4 itself states, and a file that
/// is not a PDF is refused by name there (`pdf_model::reference::Refusal`). What this reads is a
/// directory a *person* named, which is the whole of the decision.
///
/// The refusals are returned rather than printed: what a host does with them is the host's, and
/// `viewer_core::Viewer::reference_refusals` has the other half — the files that were read here
/// and would not open there.
#[must_use]
pub fn reference_files(directory: Option<&Path>) -> (ReferenceFiles, Vec<ReferenceRefusal>) {
    let Some(directory) = directory else {
        return (ReferenceFiles::default(), Vec::new());
    };
    let mut refused = Vec::new();
    let mut names: Vec<PathBuf> = match std::fs::read_dir(directory) {
        Ok(entries) => entries.flatten().map(|entry| entry.path()).collect(),
        Err(error) => {
            refused.push(ReferenceRefusal::DirectoryUnreadable {
                directory: directory.display().to_string(),
                error: error.to_string(),
            });
            return (ReferenceFiles::default(), refused);
        }
    };
    names.sort();
    let mut files = Vec::new();
    for path in names {
        if path.is_dir() {
            continue;
        }
        let name = path.file_name().map_or_else(
            || path.display().to_string(),
            |name| name.to_string_lossy().into_owned(),
        );
        match std::fs::read(&path) {
            Ok(bytes) => files.push((name, bytes)),
            Err(error) => refused.push(ReferenceRefusal::FileUnreadable {
                name,
                error: error.to_string(),
            }),
        }
    }
    (
        ReferenceFiles {
            files,
            source: format!("{} ({REFERENCE_FILES})", directory.display()),
        },
        refused,
    )
}

/// **The one place a host answers "who is reading, and in what language", and the answer is
/// *nobody* by default.**
///
/// The four arguments are what [`READER_NAME`], [`READER_TITLE`], [`READER_ORGANISATION`] and
/// [`INTERFACE_LANGUAGE`] named, each empty or `None` where nobody typed it. Under that answer
/// both of ISO 32000-2 §8.11.4.4's categories about this processor are reported unanswered, the
/// document's own configuration decides every group, and the page draws what this program drew
/// before the question could be answered at all.
///
/// **Nothing here is read off the machine**, and that is the decision rather than an omission. A
/// host could take the language from a locale and the name from a login, and both would be this
/// program deciding on a reader's behalf what a *document* gets told about them — which is the
/// same objection ADR 1039 makes to picking a trust anchor. A person says it or nobody does.
///
/// An empty language tag is discarded rather than carried: §14.9.2.2 gives it a meaning of its
/// own — "the empty text string, to indicate that the language is unknown" — and a word typed
/// with nothing after it is not a person saying their language is unknown.
#[must_use]
pub fn audience(
    names: &[String],
    titles: &[String],
    organisations: &[String],
    language: Option<&str>,
) -> Audience {
    Audience {
        reader: Reader {
            individual: names.to_vec(),
            title: titles.to_vec(),
            organisation: organisations.to_vec(),
        },
        language: language
            .filter(|tag| !tag.is_empty())
            .map(ToOwned::to_owned),
    }
}

/// Every certificate one file holds, as DER: the file itself, or each PEM block in it.
///
/// X.690 clause 8.1.2's identifier octet for a constructed `SEQUENCE` is `0x30`, and RFC 5280
/// section 4.1 makes a `Certificate` one — so a file starting with it is offered whole and read by
/// `pdf_signature::x509`, which is the only thing here entitled to decide whether it is a
/// certificate. Everything else is scanned for PEM's `-----BEGIN CERTIFICATE-----` boundaries (RFC
/// 7468, which this tree does not hold — see [`base64`]).
fn certificates_in(bytes: &[u8]) -> Vec<Vec<u8>> {
    const BEGIN: &str = "-----BEGIN CERTIFICATE-----";
    const END: &str = "-----END CERTIFICATE-----";
    if bytes.first() == Some(&0x30) {
        return vec![bytes.to_vec()];
    }
    let text = String::from_utf8_lossy(bytes);
    let mut out = Vec::new();
    let mut rest = text.as_ref();
    while let Some(start) = rest.find(BEGIN) {
        let after = start.saturating_add(BEGIN.len());
        let Some(stop) = rest[after..].find(END) else {
            break;
        };
        if let Some(der) = base64(&rest[after..after.saturating_add(stop)]) {
            out.push(der);
        }
        rest = &rest[after.saturating_add(stop).saturating_add(END.len())..];
    }
    out
}

/// RFC 4648 section 4's alphabet, decoded, with whitespace skipped and anything else refusing.
///
/// Written here rather than taken from a package because it is thirty lines and the alternative is
/// a dependency on the path that reads a stranger's file. RFC 7468 is what a PEM file's boundaries
/// are defined by and this tree does not hold it, so the scan above is written to the shape the
/// format has in practice — text before and between blocks is skipped, and a block whose base64
/// this function refuses is one certificate lost rather than a file rejected.
fn base64(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let mut accumulator: u32 = 0;
    let mut bits = 0_u32;
    let mut padding = 0_usize;
    for byte in text.bytes() {
        let value = match byte {
            b'A'..=b'Z' => u32::from(byte.wrapping_sub(b'A')),
            b'a'..=b'z' => u32::from(byte.wrapping_sub(b'a')).saturating_add(26),
            b'0'..=b'9' => u32::from(byte.wrapping_sub(b'0')).saturating_add(52),
            b'+' => 62,
            b'/' => 63,
            b'=' => {
                padding = padding.saturating_add(1);
                continue;
            }
            b' ' | b'\t' | b'\r' | b'\n' => continue,
            _ => return None,
        };
        if padding > 0 {
            return None;
        }
        accumulator = (accumulator << 6) | value;
        bits = bits.saturating_add(6);
        if bits >= 8 {
            bits = bits.saturating_sub(8);
            #[expect(
                clippy::cast_possible_truncation,
                reason = "the shift leaves exactly the eight bits this takes"
            )]
            out.push((accumulator >> bits) as u8);
        }
    }
    // The leftover bits must be the zero padding RFC 4648 section 3.5 requires, not data: a block
    // whose final group carries bits nobody encoded is not base64 this reader will act on.
    let mask = 1_u32.checked_shl(bits).unwrap_or(1).saturating_sub(1);
    (bits == 0 || accumulator & mask == 0).then_some(out)
}
