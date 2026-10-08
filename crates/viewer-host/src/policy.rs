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
//! **And the fourth** — how much of what a *document* asserts over its reader this program obeys.
//! `viewer_core::Command::Restrict` is the value and `CLAUDE.md` states the rule it exists for: a
//! document's restrictions "are the reader's to set" and "**it shall always be possible to turn
//! them off**". [`IGNORE_RESTRICTIONS`] is the word that turns them off and [`refused`] is the
//! sentence that names it, and they are one unit here for the reason ADR 0604 records: apart, a
//! window can say the word without taking it. [`RESTRICTIONS`] is the rest of that vocabulary: all
//! four levels, one operation at a time (ADR 1144).
//!
//! **And a fifth, which used to be four words inside `pdf_model::action::refused`** — whether a
//! submit-form action's request leaves this machine. §12.7.6.2's `shall` is to "transmit the
//! names and values of selected interactive form fields" to a URL, and
//! `pdf_model::submission::compose` has answered every part of that which is about the
//! *document*; what reaches a host is one question about *this machine*, the same kind the
//! §12.7.6.4 paragraph asks about a filesystem. [`may_submit`] is where it is asked, at one of
//! [`Submissions`]' four levels, and [`submission_note`] is the sentence, and they are one unit
//! here for [`refused`]'s reason (ADR 1062). What carries a `yes` out is `crate::submit`, the one
//! HTTP client in the tree (ADR 1291).
//!
//! **And a sixth** — whether a link's URI is handed to whatever this machine opens one with.
//! §12.6.4.8 says "[a] URI action causes a URI to be resolved", and the same division applies for
//! the third time: what the URI *is* — Table 210's `/URI` against Table 211's `/Base`, then
//! [`resolve_uri`] against the location of the document itself, which is a fact about this machine
//! and so cannot be the core's — is answered before a host sees it, and [`may_open_uri`] is the one
//! question left (ADR 1079). It is asked at one of [`Links`]'s four levels, because starting
//! another program on a string a document chose is exactly the decision `CLAUDE.md` principle 3
//! says must be able to become a question: [`link`] carries the level out and [`answered`] takes
//! the person's word back (ADR 1155).
//!
//! **And a seventh, which ADR 1039 named** — §12.8.1's third question, *is the signer anyone to
//! believe*. RFC 5280 section 6.1.1 makes the trust anchors input (d) of nine and says whose choice
//! they are: "The selection of a trust anchor is a matter of policy: it could be the top CA in a
//! hierarchical PKI, the CA that issued the verifier's own certificate(s), or any other CA in a
//! network PKI." `pdf-signature` holds no root, reads no file and asks no clock; this module is the
//! party with all three. [`trust_anchors`] is where the question is asked, and it answers *nobody*
//! unless a person said otherwise — which is ADR 1039's decision unchanged rather than a default
//! chosen here. ADR 1076.
//!
//! **And a ninth** — §8.11.4.4's `User` and `Language` usage categories, which ask who is reading
//! and in what language. Table 100 says what a document may assert about its audience and the
//! clause says what a processor does with it: match the names "with the user's identification",
//! and select content "based on the language and locale of the application". Neither is a fact
//! the file holds, and a document that could assert who is reading would be choosing its own
//! audience. [`audience`] is where the question is asked, and the answer is *nobody, in no stated
//! language* unless a person said otherwise. ADR 1106.
//!
//! **And the signature policy's copy** — §12.8.3.4.4's explicit-policy profile hands its attribute
//! to ETSI EN 319 122-1 clause 5.2.9, whose URL qualifier names where a copy of the policy
//! document can be obtained. Fetching it tells that server this document is being validated here,
//! which is the act class [`may_fetch_import`] already asks about, so it is asked at the same
//! [`Submissions`] level by [`may_fetch_signature_policy`], and [`signature_policy_fetched`] is the
//! sentence that goes beside the signature's verdict: bound and shown, a digest that does not
//! match, or not fetched at this level — with the constraints the clause also requires enforced
//! named as not enforced, because no syntax this program holds states them (ADRs 1709, 1728).

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
/// has.** A command line is not a menu; what it is is the channel a person can reach today, in all
/// three windows and the C ABI's caller alike, and the levels behind it are the ones a menu will
/// set when there is one. `--restrictions=off` is still the whole policy at one level, which is
/// what [`IGNORE_RESTRICTIONS`] says in one word.
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
/// **A face that cannot ask answers no, out loud.** Going ahead on an unanswered question would be
/// the *off* level under another name, and not going ahead is what a closed dialogue means
/// everywhere else — `pdf-transform` makes the same choice for a pipe with `Refusal::Unanswered`
/// and `pdf-fuse` for a mount.
///
/// **The three windows put the question** ([`crate::restriction::asked`], ADR 1145). What still
/// says this is `quorra-confined`, which performs none of the operations a document restricts — no
/// edit, no copy gesture, no level of its own — so the event cannot reach it; the arm and this
/// sentence are what keep that from being a silence if one ever does.
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
    /// The name does not reach a file in the document's directory or a directory below it.
    ///
    /// Checked component by component as a path rather than as a string, so that a separator this
    /// platform recognises and this program does not cannot slip through: `../secrets`,
    /// `/etc/passwd`, `sub/../../x` and a Windows drive letter are all refused by the same rule.
    #[error("{name} is not a file in or below the document's own directory")]
    OutsideTheDocumentsDirectory {
        /// The name the document wrote, unchanged.
        name: String,
    },
}

/// Where a file a document names may be read from, under the narrowest policy that performs it.
///
/// Two rules, and they are the whole policy:
///
/// - the name must be a **relative** file specification whose every component names a directory
///   or file *below* where it stands, so `..`, `.`, an empty component, an absolute path and a
///   drive-relative one are all refused;
/// - it is resolved against the directory the open document is in, and nowhere else.
///
/// §7.11.2.2 is the construction the first rule admits and nothing wider: "[a] file specification
/// that does not begin with a SOLIDUS shall be a relative file specification giving the location of
/// the file relative to that of the PDF file containing it", and its EXAMPLE 1 is a relative
/// specification into a subdirectory, `ArtFiles/Figure1.pdf`. The rule exists to keep a document to
/// its own directory; a directory below it is inside it, and `..` is the one component that leaves
/// it. ADR 1369.
///
/// §7.11.2.1 is how the name is split: the SOLIDUS separates components, and one preceded by a
/// REVERSE SOLIDUS is part of a component — which no file name on this platform can hold, so such a
/// component is refused by the same per-component check as `..`.
///
/// Pure, so that the policy is testable without a filesystem and without a window — which is what
/// `tests/host_mappings.rs` does. Reading the bytes is [`read_import`]'s.
///
/// # Errors
///
/// [`ImportRefusal`], one variant per rule above.
pub fn resolve_import(directory: Option<&Path>, name: &str) -> Result<PathBuf, ImportRefusal> {
    let directory = directory.ok_or(ImportRefusal::NoDirectory)?;
    let outside = || ImportRefusal::OutsideTheDocumentsDirectory {
        name: name.to_owned(),
    };
    let mut path = directory.to_path_buf();
    for component in file_specification_components(name) {
        let mut parts = Path::new(&component).components();
        let (Some(Component::Normal(single)), None) = (parts.next(), parts.next()) else {
            return Err(outside());
        };
        path.push(single);
    }
    if path.as_path() == directory {
        return Err(outside());
    }
    Ok(path)
}

/// A file specification string's components, by §7.11.2.1's escape.
///
/// "The SOLIDUS is a generic component separator", and a SOLIDUS preceded by a REVERSE SOLIDUS is
/// part of the component — the REVERSE SOLIDUS "shall be removed in processing the string". A
/// leading SOLIDUS gives an empty first component, which is how an absolute specification reaches
/// the caller's check.
fn file_specification_components(name: &str) -> Vec<String> {
    let mut components = vec![String::new()];
    let mut characters = name.chars().peekable();
    while let Some(character) = characters.next() {
        match character {
            '\\' if characters.peek() == Some(&'/') => {
                characters.next();
                if let Some(last) = components.last_mut() {
                    last.push('/');
                }
            }
            '/' => components.push(String::new()),
            other => {
                if let Some(last) = components.last_mut() {
                    last.push(other);
                }
            }
        }
    }
    components
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

/// How many bytes of a person's chosen file this program will take into memory.
///
/// §12.7.5.3 states no bound and could not: it is a fact about this machine rather than about any
/// document. `CLAUDE.md` principle 3 asks for explicit memory budgets by name — "Rust does not
/// prevent resource exhaustion" — and the whole file is read at once because §12.7.6.2's body is
/// composed in memory. Sixty-four mebibytes is chosen as comfortably larger than anything a form
/// attachment plausibly is and far below what a window may hold, and a file over it is refused
/// with its size said out loud rather than read and then dropped (ADR 1216).
pub const CHOSEN_FILE_LIMIT: u64 = 64 * 1024 * 1024;

/// Whether this host may put a **file chooser** in front of a person for this control.
///
/// §12.7.5.3, Table 231 bit 21, states what such a field's text is:
///
/// > If the FileSelect flag ( PDF 1.4 ) is set, the field shall function as a file-select
/// > control. In this case, the field's text represents the pathname of a file whose contents
/// > shall be submitted as the field's value
///
/// So the chooser is offered for that flag and for nothing else: a pathname under a field that
/// does not carry it would be the wrong value under the right name, which is the same sentence
/// [`crate::form::edit_of`] already refuses on (trap 5). Every other text field takes text, and
/// putting a chooser on one would be this program deciding what a document's field is for.
///
/// **The gate is a function rather than a condition at each call site**, for the reason
/// [`read_chosen`] records and `doc/todo/38` needs: the policy is asked once, in a place a host
/// can supply, so the *ask* and *warn* levels attach here and nowhere else. It is asked twice on
/// one path and deliberately — once by a window deciding whether to offer the affordance at all,
/// and once by [`crate::form::edit_of`] when a path comes back — because a window that offered a
/// chooser it would then refuse has asked a person for something it will not use (ADR 1240).
///
/// # Errors
///
/// The sentence to say to the person: this field is not one whose value is a file.
pub fn may_choose_file(control: Option<&crate::form::ControlKind>) -> Result<(), String> {
    match control {
        Some(crate::form::ControlKind::Entry {
            file_select: true, ..
        }) => Ok(()),
        _ => Err(
            "this field's value is its text rather than a file, so there is no file to choose \
             for it (ISO 32000-2 §12.7.5.3, Table 231 bit 21)"
                .to_owned(),
        ),
    }
}

/// The bytes of §12.7.5.3's file-select control, from a path a **person** named.
///
/// Table 231 bit 21 makes the field's text "the pathname of a file whose contents shall be
/// submitted as the field's value", and the contents are the half `pdf-model` cannot have:
/// `CLAUDE.md` principle 3 gives that process no filesystem. This is the read that follows, and
/// it is a host's for the same reason [`read_import`] is.
///
/// **It is not [`read_import`] and the difference is who named the path.** §12.7.6.4's file is
/// named by the *document*, so it is resolved against the document's own directory and nothing
/// else — a document naming `/etc/passwd` gets a refusal. A file-select control's path is typed
/// or chosen by the person at the keyboard, and confining *that* to the document's directory
/// would be this program refusing its reader access to their own files. So the path is taken as
/// given, with two bounds that are about this machine rather than about trust:
/// [`CHOSEN_FILE_LIMIT`], and a refusal for anything that is not a regular file.
///
/// **A refusal rather than a hard `no`, for the reason [`may_submit`] gives**: the policy is asked
/// once, in a place a host can supply, so `doc/todo/38`'s *ask* and *warn* levels attach here and
/// nowhere else. §7.6.4.1's Table 22 bit 9 — filling in an existing interactive form field — is
/// asked as `pdf_model::restriction::Operation::FillInForm` by `viewer_core::Viewer` before the
/// edit is logged, which is the *document's* own restriction on the same act.
///
/// # Errors
///
/// The sentence to say to the person: the path names nothing, names something that is not a file,
/// names one this process may not read, or names one over [`CHOSEN_FILE_LIMIT`].
pub fn read_chosen(pathname: &str) -> Result<Vec<u8>, String> {
    if pathname.is_empty() {
        return Err("no file is named, so there is nothing to submit for this field".to_owned());
    }
    let path = Path::new(pathname);
    let stated = std::fs::metadata(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    if !stated.is_file() {
        return Err(format!("{} is not a file", path.display()));
    }
    if stated.len() > CHOSEN_FILE_LIMIT {
        return Err(format!(
            "{} is {} bytes, over this program's {CHOSEN_FILE_LIMIT}-byte limit for a file-select \
             control",
            path.display(),
            stated.len()
        ));
    }
    std::fs::read(path).map_err(|error| format!("cannot read {}: {error}", path.display()))
}

/// The document a **person** chose or named, held open where the core reads it.
///
/// Every route by which a reader asks for a document ends here: a file dialogue in a window, a
/// typed path, a second path on a command line. It is the one place those paths become bytes, so
/// that `CLAUDE.md`'s four levels and `doc/todo/38`'s controls attach to a single function rather
/// than to three windows' key handlers — [`may_choose_file`]'s argument, for the chooser that opens
/// a document rather than the one that fills a field (ADR 1275).
///
/// **Taken as given, which is [`read_chosen`]'s rule and not [`read_import`]'s**: the path is the
/// reader's own, so it is not confined to anybody's directory. What is refused is what this
/// program cannot open — a path that names nothing, or names something that is not a regular file —
/// and it is refused here, by name, rather than as a parser's complaint about the bytes of a
/// directory. No size bound, and the difference from [`CHOSEN_FILE_LIMIT`] is where the bytes go:
/// a document is opened on disk and read where its offsets point (ADR 0809), so its size is not
/// what this process holds.
///
/// Opening a document is no restriction any document asserts — Table 22's `/P` is about what may
/// be done to a document once it is open, and §12.11.6's requirements are asked by
/// `viewer_core::Viewer` as the document opens, under the window's levels.
///
/// # Errors
///
/// The sentence to say to the person: the path names nothing, names something that is not a
/// file, or names one this process may not open.
pub fn open_chosen(path: &Path) -> Result<pdf_syntax::FileBytes, String> {
    let stated = std::fs::metadata(path)
        .map_err(|error| format!("cannot open {}: {error}", path.display()))?;
    if !stated.is_file() {
        return Err(format!(
            "{} is not a file, so there is no document to open",
            path.display()
        ));
    }
    pdf_syntax::FileBytes::on_disk(path)
        .map_err(|error| format!("cannot open {}: {error}", path.display()))
}

/// Which clause asked for a file, in the word a host prints in front of its sentence.
///
/// **One function rather than a literal at each call site**, for the reason [`refused`] records:
/// three windows each wrote `"import-data: …"` by hand, so the day a second purpose arrived every
/// one of them would have told a person that a `/GoToE` was a form import. The word is the
/// action's own name in Table 201, which is what a person can look up.
///
/// [`Purpose::NamedPage`] is the one that is not an action's name, because the act is not an
/// action: it is Table 253's entry reached from inside §12.7.6.4's import, so the word is the
/// entry's own subject and a person reading a status line sees it beside the `import-data`
/// sentence that caused it (ADR 1239).
#[must_use]
pub const fn asked_for(purpose: Purpose) -> &'static str {
    match purpose {
        Purpose::ImportData => "import-data",
        Purpose::TargetRoot => "GoToE",
        Purpose::RemoteDocument => "GoToR",
        Purpose::NamedPage => "named page",
        Purpose::ThreadDocument => "Thread",
        Purpose::LaunchDocument => "Launch",
    }
}

/// What this reader would do with the file, in the sentence a person is asked about.
///
/// Four of the six purposes reach [`asked_to_open_remote`], and they do different things with
/// the bytes: three replace the document on the screen — one of them only if the bytes prove to be
/// a PDF — and one draws a page of the second file into the document being read. A person deciding whether to let a file be opened is deciding about
/// *that*, so the sentence is per purpose rather than per act (trap 5, ADR 1239).
///
/// `beside` is `viewer_core::Event::NeedsFile`'s: Table 203's `/NewWindow true`, which every
/// window in this tree answers by offering `viewer_core::Command::Beside` a name, so the file opens
/// in a tab of its own and the document being read stays open. Table 203 says that under `false`
/// "the destination document replaces the current document in the same window", and leaves the
/// absent entry to the processor's preference, which here is the same replacement (ADR 1335).
const fn what_would_happen(purpose: Purpose, beside: bool) -> &'static str {
    match purpose {
        Purpose::NamedPage => {
            "This reader would read that file and draw one of its pages into the document you are \
             reading (ISO 32000-2 §12.7.8.3.2, §12.7.8.3.3)."
        }
        Purpose::ThreadDocument => {
            "This reader would open that file in place of the one you are reading, at the article \
             bead the link names (ISO 32000-2 §12.6.4.7)."
        }
        Purpose::ImportData => {
            "This reader would put that file's form values into the document you are reading (ISO \
             32000-2 §12.7.6.4)."
        }
        Purpose::TargetRoot => {
            "This reader would open that file in place of the one you are reading (ISO 32000-2 \
             §12.6.4.4)."
        }
        Purpose::RemoteDocument if beside => {
            "This reader would open that file beside the one you are reading, in a tab of its own, \
             because the link asks for a new window (ISO 32000-2 §12.6.4.3, Table 203)."
        }
        Purpose::RemoteDocument => {
            "This reader would open that file in place of the one you are reading (ISO 32000-2 \
             §12.6.4.3)."
        }
        // Table 207's `/F` may name an application, and the question has to say what happens to
        // one as plainly as to a document: nothing is started either way (ADR 1358).
        Purpose::LaunchDocument if beside => {
            "This reader would open that file beside the one you are reading, in a tab of its own, \
             because the launch action asks for a new window — if it is a PDF. If it is anything \
             else it is an application, and this reader starts none (ISO 32000-2 §12.6.4.6, Table \
             207)."
        }
        Purpose::LaunchDocument => {
            "This reader would open that file in place of the one you are reading — if it is a PDF. \
             If it is anything else it is an application, and this reader starts none (ISO 32000-2 \
             §12.6.4.6, Table 207)."
        }
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

/// The schemes a submission may be sent to, whatever the level.
///
/// **The narrowest list that still performs what the clause describes**, and [`LINK_SCHEMES`]'s
/// shape for [`LINK_SCHEMES`]'s reason. Table 239's `/F` is "[a] URL file specification … giving
/// the uniform resource locator (URL) of the script at the Web server that will process the
/// submission", and Table 240's own vocabulary is HTTP's — bit 4 chooses "an HTTP GET request"
/// over "a POST request" — so a URL naming anything but a Web server is outside what the clause is
/// about. `file` is absent for ADR 1155's reason, one act over: a document that could choose a path
/// on this machine to write its form into is §12.7.6.4's hazard reversed. `mailto` is absent
/// although a link may use it, because a submission is an entity body with a media type and no
/// handler this program starts takes one. ADR 1291.
pub const SUBMIT_SCHEMES: [&str; 2] = ["http", "https"];

/// What this reader does when §12.7.6.2's composed request is ready to leave the machine.
///
/// `CLAUDE.md`'s four levels over the one act [`may_submit`] decides, spelled in [`Links`]'s
/// direction rather than [`RESTRICTIONS`]'s — `refuse`, `ask`, `warn`, `send` — for ADR 1155's
/// reason: the subject is this machine doing something a *document* asked for, so the permissive
/// end is the one where the request is sent, and a reader who read `off` as permissive would have
/// turned the wrong way in the same menu that holds the restrictions. ADR 1291.
///
/// **Global rather than per document**, which the owner's answer to `doc/questions/Q98` settled:
/// what a reader decides here is what their machine sends to the network, and that is a fact about
/// the person rather than about the file that asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Submissions {
    /// Send nothing: the submission is declined and the request said out loud.
    Refuse,
    /// Put the request to the person first, and send it on a `yes`.
    ///
    /// **The default**, and the owner's (`doc/questions/Q98`): a document must not be able to send
    /// a form from this machine by itself, and a reader must not be refused by their own viewer
    /// the act the clause describes. [`Links::Ask`]'s argument, one act over.
    #[default]
    Ask,
    /// Send it, and say afterwards what was sent where.
    Warn,
    /// Send it without asking.
    Send,
}

impl Submissions {
    /// All four, least permissive first — the order a menu offers them in.
    pub const ALL: [Self; 4] = [Self::Refuse, Self::Ask, Self::Warn, Self::Send];

    /// The word a person reads for this level.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Refuse => "refuse",
            Self::Ask => "ask",
            Self::Warn => "warn",
            Self::Send => "send",
        }
    }

    /// The level a word names, or `None` for a word that names none.
    #[must_use]
    pub fn parse(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|level| level.as_str() == word)
    }
}

/// What a host does about one §12.7.6.2 submission, under the level the reader set.
///
/// [`Opening`]'s four arms, for its reason: a policy with four levels answers in four ways, and a
/// host matching three would have a level that silently behaved like another. Closed, and **not**
/// `#[non_exhaustive]`, for `doc/ui-boundary.md`'s reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sending {
    /// Hand it to `crate::submit::Submitter` now.
    Send,
    /// Hand it over now, and say this beside what comes back.
    Warn(String),
    /// Put this question to the person, and send it on a `yes`.
    Ask(crate::restriction::Question),
    /// Send nothing. The sentence says why, for [`submission_note`].
    Refuse(String),
}

/// Whether §12.7.6.2's composed submission may be **transmitted** from this machine.
///
/// §12.7.6.2 says an interactive PDF processor "shall transmit the names and values of selected
/// interactive form fields to a specified uniform resource locator (URL)", and every part of that
/// sentence except the verb is a question about the document, which
/// `pdf_model::submission::compose` has already answered. The verb is a network request, sent by
/// `crate::submit` from the host side of `CLAUDE.md` principle 3's boundary: the process that read
/// the file still has no network and acquires none.
///
/// **The one place the level is read**, for ADR 1062's reason: the policy is asked once, in a place
/// a host can supply, and every window asks it here rather than wording its own.
///
/// **Two questions are answered before the level is consulted, and the order is the point** —
/// [`may_open_uri`]'s. A URL that names no scheme names no Web server at any level, and a scheme
/// outside [`SUBMIT_SCHEMES`] is not one this machine sends a form to however permissive the
/// reader is — so neither is a thing *send* turns on. A third follows them for the same reason: a
/// URL that fails `crate::submit::check_url` is not handed to the TLS stack at any level (ADRs
/// 1291 and 1327).
#[must_use]
pub fn may_submit(submission: &Submission, level: Submissions) -> Sending {
    if scheme_of(&submission.url).is_none() {
        return Sending::Refuse(
            "Table 239's /F states no scheme this reader could read, so it names no Web server \
             (ISO 32000-2 §12.7.6.2, RFC 3986 section 3.1)"
                .to_owned(),
        );
    }
    if let Some(why) = unreachable_server(&submission.url, "sends a form to") {
        return Sending::Refuse(why);
    }
    match level {
        Submissions::Refuse => Sending::Refuse(format!(
            "this reader is set to send no form ({}: {}); {} puts the request to you first",
            crate::restriction::SUBMITTING,
            Submissions::Refuse.as_str(),
            Submissions::Ask.as_str()
        )),
        Submissions::Ask => Sending::Ask(asked_to_submit(submission)),
        Submissions::Warn => Sending::Warn(format!(
            "it was sent without asking you first, because this reader is set to {} ({})",
            Submissions::Warn.as_str(),
            crate::restriction::SUBMITTING
        )),
        Submissions::Send => Sending::Send,
    }
}

/// Why a URL with a scheme is not one this machine sends anything to, or `None` where it is.
///
/// The two checks [`may_submit`] and [`may_fetch_import`] make before either reads the level, in
/// their order: a scheme outside [`SUBMIT_SCHEMES`] names no Web server this reader talks to, and
/// a URL that fails `crate::submit::check_url` is not handed to the TLS stack (ADR 1327). `act`
/// is the verb phrase the first sentence uses — what this reader would have done with that server.
fn unreachable_server(url: &str, act: &str) -> Option<String> {
    let scheme = scheme_of(url)?;
    if !SUBMIT_SCHEMES.contains(&scheme.as_str()) {
        return Some(format!(
            "{scheme}: is not one of the schemes this reader {act} ({}), and a document does not \
             get to choose what on this machine is read or written in its name",
            SUBMIT_SCHEMES.join(", ")
        ));
    }
    if let Err(refusal) = crate::submit::check_url(url) {
        return Some(format!(
            "{url} is not sent: {refusal}; a host is checked before it reaches the TLS stack \
             (ADR 1327)"
        ));
    }
    None
}

/// What a window puts in front of a person at [`Submissions::Ask`].
///
/// [`asked_to_open`]'s two-string shape. The first string says *where* and *what*, whole: the URL,
/// the method, the media type and the size are what a person can judge a submission by, and a
/// question that left one out would be asking for a decision without its subject.
#[must_use]
pub fn asked_to_submit(submission: &Submission) -> crate::restriction::Question {
    crate::restriction::Question {
        reasons: format!(
            "This document asks to send its form from this machine: {}.",
            request_line(submission)
        ),
        choice: format!(
            "You have set this reader to ask before a form is sent ({submitting}: {}). \"{}\" sends \
             this one and leaves the level where it is; \"{}\" sends nothing. Setting \
             {submitting} to {} in the restrictions menu stops the question being asked, and {} \
             stops forms being sent at all.",
            Submissions::Ask.as_str(),
            crate::restriction::GO_AHEAD,
            crate::restriction::DO_NOT,
            Submissions::Send.as_str(),
            Submissions::Refuse.as_str(),
            submitting = crate::restriction::SUBMITTING,
        ),
    }
}

/// The request in one line: method, URL, media type, fields and bytes.
fn request_line(submission: &Submission) -> String {
    let method = match submission.method {
        Method::Get => "GET",
        Method::Post => "POST",
    };
    format!(
        "{method} {} ({}), {} field(s), {} byte(s)",
        submission.url,
        submission.media_type,
        submission.fields,
        submission.body.len(),
    )
}

/// What a host says about a submission, whether it sends it or declines.
///
/// The request in one line, because the person who pressed the button is owed *where it was
/// going and what would have gone* rather than the word "declined" on its own. What the
/// composition did not do is not repeated here: `viewer_core` has already put every sentence of
/// `Submission::owed` into an `Event::Reported`, and a host that said both would say them twice.
#[must_use]
pub fn submission_note(submission: &Submission, refused: Option<&str>) -> String {
    let what = request_line(submission);
    match refused {
        Some(why) => format!("submit-form: declined — {why}. It would have been {what}"),
        None => format!("submit-form: {what}"),
    }
}

/// Whether the file an import names is an **absolute URI**, which is fetched rather than read.
///
/// Table Annex O.4's `fdf`: "The URI shall be either a relative or absolute URI to an FDF or XFDF
/// file." A relative one is a file beside the document and is [`resolve_import`]'s; an absolute one
/// names its own scheme, which is RFC 3986 section 4.1's whole distinction and
/// `pdf_model::uri::is_absolute`'s answer, and it goes to [`may_fetch_import`]. A scheme this reader
/// does not fetch from is still routed there, so that `file:` or `ftp:` is refused by the sentence
/// that names the scheme rather than by one about the document's directory (ADR 1527).
#[must_use]
pub fn import_is_fetched(name: &str) -> bool {
    pdf_model::uri::is_absolute(name)
}

/// Whether the form data an import names by an absolute URI may be **fetched** from that server.
///
/// Table Annex O.4's `fdf` is "Open the document and then import the data from the specified FDF
/// or XFDF file", and an absolute URI makes that a GET over the network. That tells the server this
/// document was opened here, which is §12.7.6.2's act class — a machine contacting a server on a
/// document's word — so **the level is [`Submissions`]'** rather than a fifth one invented, read
/// here and nowhere else (`CLAUDE.md` principle 3's shape; ADR 1527).
///
/// [`may_submit`]'s order, for its reason: a URI with a scheme outside [`SUBMIT_SCHEMES`] and one
/// that fails `crate::submit::check_url` are refused at every level, and only then is the level
/// read. **Where the name came from is not consulted**: a fragment can arrive from a person's
/// command line or from a document's own `ef` remainder, and an import-data action may name a URL
/// too, and nothing here could tell them apart.
#[must_use]
pub fn may_fetch_import(url: &str, level: Submissions) -> Sending {
    if scheme_of(url).is_none() {
        return Sending::Refuse(format!(
            "{url} states no scheme this reader could read, so it names no Web server (RFC 3986 \
             section 3.1)"
        ));
    }
    if let Some(why) = unreachable_server(url, "fetches form data from") {
        return Sending::Refuse(why);
    }
    match level {
        Submissions::Refuse => Sending::Refuse(format!(
            "this reader is set to send nothing to a server on a document's behalf ({}: {}); {} \
             puts the request to you first",
            crate::restriction::SUBMITTING,
            Submissions::Refuse.as_str(),
            Submissions::Ask.as_str()
        )),
        Submissions::Ask => Sending::Ask(asked_to_fetch(url)),
        Submissions::Warn => Sending::Warn(format!(
            "it was fetched without asking you first, because this reader is set to {} ({})",
            Submissions::Warn.as_str(),
            crate::restriction::SUBMITTING
        )),
        Submissions::Send => Sending::Send,
    }
}

/// What a window puts in front of a person when [`may_fetch_import`] answers
/// [`Submissions::Ask`]: the URL whole, and what fetching it would do.
#[must_use]
pub fn asked_to_fetch(url: &str) -> crate::restriction::Question {
    crate::restriction::Question {
        reasons: format!(
            "This document asks for its form data from a server: GET {url}. Fetching it tells \
             that server this document was opened here, and what it answers is imported into the \
             form."
        ),
        choice: format!(
            "You have set this reader to ask before anything is sent to a server on a document's \
             behalf ({submitting}: {}). \"{}\" fetches this one and leaves the level where it is; \
             \"{}\" fetches nothing. Setting {submitting} to {} in the restrictions menu stops the \
             question being asked, and {} stops anything being fetched at all.",
            Submissions::Ask.as_str(),
            crate::restriction::GO_AHEAD,
            crate::restriction::DO_NOT,
            Submissions::Send.as_str(),
            Submissions::Refuse.as_str(),
            submitting = crate::restriction::SUBMITTING,
        ),
    }
}

/// What a host says about a fetched import, whether it fetches it or declines.
#[must_use]
pub fn fetch_note(url: &str, refused: Option<&str>) -> String {
    match refused {
        Some(why) => format!("import-data: declined — {url} was not fetched: {why}"),
        None => format!("import-data: fetching GET {url}"),
    }
}

/// Whether the copy of a signature's policy document at the URL the signature names may be
/// **fetched**.
///
/// ETSI EN 319 122-1 clause 5.2.9.2's URL qualifier says where a copy of the policy document can
/// be obtained, and §12.8.3.4.4 makes clause 5.2.9's rules apply to a `PAdES-E-EPES` signature.
/// A GET to that URL tells its server that this document is being validated here, which is
/// [`may_fetch_import`]'s act class — a machine contacting a server on a document's word — so the
/// level is [`Submissions`]' and nothing new: one network level, read in the places that use the
/// network (ADRs 1527, 1728). [`may_submit`]'s order, for its reason: a scheme outside
/// [`SUBMIT_SCHEMES`] and a URL that fails `crate::submit::check_url` are refused at every level
/// before the level is read.
#[must_use]
pub fn may_fetch_signature_policy(
    policy: &pdf_signature::policy::PublishedPolicy,
    level: Submissions,
) -> Sending {
    if scheme_of(&policy.url).is_none() {
        return Sending::Refuse(format!(
            "{} states no scheme this reader could read, so it names no Web server (RFC 3986 \
             section 3.1)",
            policy.url
        ));
    }
    if let Some(why) = unreachable_server(&policy.url, "fetches a signature policy from") {
        return Sending::Refuse(why);
    }
    match level {
        Submissions::Refuse => Sending::Refuse(format!(
            "this reader is set to send nothing to a server on a document's behalf ({}: {}); {} \
             puts the request to you first",
            crate::restriction::SUBMITTING,
            Submissions::Refuse.as_str(),
            Submissions::Ask.as_str()
        )),
        Submissions::Ask => Sending::Ask(asked_to_fetch_policy(policy)),
        Submissions::Warn => Sending::Warn(format!(
            "it was fetched without asking you first, because this reader is set to {} ({})",
            Submissions::Warn.as_str(),
            crate::restriction::SUBMITTING
        )),
        Submissions::Send => Sending::Send,
    }
}

/// What a window puts in front of a person when [`may_fetch_signature_policy`] answers
/// [`Submissions::Ask`]: the URL whole, the policy it is for, and what fetching it does.
#[must_use]
pub fn asked_to_fetch_policy(
    policy: &pdf_signature::policy::PublishedPolicy,
) -> crate::restriction::Question {
    crate::restriction::Question {
        reasons: format!(
            "A signature in this document was made under signature policy {}, and says a copy of \
             that policy's document is at {}. Fetching it tells that server this document is \
             being validated here; what comes back is checked against the digest the signer \
             signed and, if it is theirs, opened beside this document.",
            policy.identifier, policy.url
        ),
        choice: format!(
            "You have set this reader to ask before anything is sent to a server on a document's \
             behalf ({submitting}: {}). \"{}\" fetches this one and leaves the level where it is; \
             \"{}\" fetches nothing. Setting {submitting} to {} in the restrictions menu stops the \
             question being asked, and {} stops anything being fetched at all.",
            Submissions::Ask.as_str(),
            crate::restriction::GO_AHEAD,
            crate::restriction::DO_NOT,
            Submissions::Send.as_str(),
            Submissions::Refuse.as_str(),
            submitting = crate::restriction::SUBMITTING,
        ),
    }
}

/// Fetches a signature policy's document from the URL the signature names, blocking, and says what
/// it is.
///
/// `crate::submit::fetch`'s client, checks and bounds, so a window calls this off its event thread
/// for that module's reason. Asked first, at [`may_fetch_signature_policy`]: a window that has not
/// had [`Sending::Send`] or [`Sending::Warn`] from it does not call this. `warned` is the
/// [`Sending::Warn`] sentence, where the level gave one.
#[must_use]
pub fn fetch_signature_policy(
    policy: &pdf_signature::policy::PublishedPolicy,
    warned: Option<&str>,
) -> crate::submit::Reply {
    signature_policy_fetched(policy, crate::submit::fetch(&policy.url), warned)
}

/// What a fetch of a signature policy's document comes to: the document, opened beside this one,
/// or a sentence.
///
/// **Only a copy that is bound is shown as the signer's policy.** ETSI EN 319 122-1 clause 5.2.10's
/// note makes the signed digest the one thing that catches a substituted policy document in an
/// unsigned store, and a server is no more signed than a store is, so a copy whose octets do not
/// digest to the signed value, or that the signer committed to no particular copy of, is said and
/// not opened. A bound copy that is a PDF — §7.5.2's header, which may follow arbitrary bytes — is
/// [`crate::submit::Reply::Document`]: the window opens it beside the document whose signature
/// named it, as it opens a PDF a server answered a form with. Only a 2xx is read, for
/// `crate::submit::reply`'s reason.
///
/// **And every outcome says what is not done.** §12.8.3.4.4's validator "shall enforce signature
/// policy constraints", the constraints are in the policy document, and their syntax is whichever
/// specification the signature names under clause 5.2.9.2 — which no signature in reach names
/// (ADR 1709). So the sentence ends by saying the constraints were not enforced and why, rather
/// than letting a bound policy read as an enforced one.
#[must_use]
pub fn signature_policy_fetched(
    policy: &pdf_signature::policy::PublishedPolicy,
    answer: Result<crate::submit::Response, crate::submit::TransmitError>,
    warned: Option<&str>,
) -> crate::submit::Reply {
    use crate::submit::Reply;
    use pdf_signature::policy::Published;
    use std::fmt::Write as _;

    let mut note = format!("signature policy {}: ", policy.identifier);
    let response = match answer {
        Ok(response) => response,
        Err(error) => {
            // `fmt::Write for String` never answers `Err`: the discard is that infallibility.
            let _ = write!(
                note,
                "its document could not be fetched from {}: {error}",
                policy.url
            );
            return Reply::Say(closed(note, policy, warned));
        }
    };
    let media = response.media_type.as_deref().unwrap_or("no media type");
    let _ = write!(
        note,
        "{} answered {} ({media}, {} byte(s))",
        policy.url,
        response.status,
        response.body.len()
    );
    if !(200..300).contains(&response.status) {
        if let Some(location) = &response.location {
            let _ = write!(
                note,
                " and points to {location}, which this reader does not follow on a document's \
                 behalf"
            );
        }
        note.push_str(", so no copy of the policy's document came back");
        return Reply::Say(closed(note, policy, warned));
    }
    match policy.binding(&response.body) {
        Published::Matches { digest } => {
            let _ = write!(
                note,
                ", and that copy is the document the signer committed to: it hashes to the {} \
                 digest they signed over it",
                digest.name()
            );
            if is_a_pdf(&response.body) {
                note.push_str(". It is opened beside this document");
                Reply::Document {
                    bytes: response.body,
                    note: closed(note, policy, warned),
                }
            } else {
                note.push_str(
                    ". It is not a PDF, so this reader cannot show it; its media type is above",
                );
                Reply::Say(closed(note, policy, warned))
            }
        }
        Published::DoesNotMatchTheFetchedOctets {
            digest,
            specification,
        } => {
            let _ = write!(
                note,
                ", and that copy does not hash to the {} digest the signer signed over it, so it \
                 is not shown as their policy. That is not by itself a different policy: what \
                 goes into the digest is decided by the specification the policy is written \
                 under (ETSI EN 319 122-1 clause 5.2.9.1), which {}",
                digest.name(),
                specification.as_ref().map_or_else(
                    || "this signature does not name and leaves to its context".to_owned(),
                    |named| format!("this signature names as {}", specification_named(named)),
                )
            );
            Reply::Say(closed(note, policy, warned))
        }
        Published::PolicyHashNotKnown => {
            note.push_str(
                ", and the signature commits to the policy by name and to no particular copy of \
                 its document (ETSI EN 319 122-1 clause 5.2.9.1's all-zero digest), so that copy \
                 cannot be bound to it and is not shown as the signer's",
            );
            Reply::Say(closed(note, policy, warned))
        }
        Published::UnderAnotherFunction { algorithm } => {
            let _ = write!(
                note,
                ", and the digest that would bind that copy to the signature is under \
                 {algorithm}, which this program does not compute, so the two were not compared \
                 and the copy is not shown as the signer's"
            );
            Reply::Say(closed(note, policy, warned))
        }
    }
}

/// What a host says about a signature policy's document it did not fetch: the URL, and why.
#[must_use]
pub fn signature_policy_declined(
    policy: &pdf_signature::policy::PublishedPolicy,
    why: &str,
) -> String {
    closed(
        format!(
            "signature policy {}: its document at {} was not fetched — {why} — so whether that \
             copy is the one the signer committed to is not known here",
            policy.identifier, policy.url
        ),
        policy,
        None,
    )
}

/// The two sentences every signature-policy note ends with: the level's warning where it gave
/// one, and what §12.8.3.4.4 also requires that this program does not do.
fn closed(
    mut note: String,
    policy: &pdf_signature::policy::PublishedPolicy,
    warned: Option<&str>,
) -> String {
    if let Some(warned) = warned {
        note.push_str(" — ");
        note.push_str(warned);
    }
    note.push_str(
        ". §12.8.3.4.4 also requires a signature handler to enforce the policy's constraints, and \
         this program does not: ",
    );
    match &policy.specification {
        Some(specification) => {
            note.push_str("they are written under ");
            note.push_str(&specification_named(specification));
            note.push_str(", a specification it does not hold");
        }
        None => note.push_str(
            "the signature names no specification for the policy's syntax (ETSI EN 319 122-1 \
             clause 5.2.9.2), so no syntax this program holds states them",
        ),
    }
    note
}

/// How a signature named the specification its policy document is written under.
fn specification_named(specification: &pdf_signature::policy::Specification) -> String {
    use pdf_signature::policy::Specification;
    match specification {
        Specification::ObjectIdentifier(oid) => format!("the specification identified by {oid}"),
        Specification::Uri(uri) => format!("the specification at {uri}"),
    }
}

/// Whether `bytes` carry §7.5.2's header, anywhere: the clause's NOTE 1 admits arbitrary bytes
/// before it, so a header found later is still the file's.
fn is_a_pdf(bytes: &[u8]) -> bool {
    bytes.windows(5).any(|window| window == b"%PDF-")
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

/// The word a person types to say what this reader does with a link's URI.
///
/// `CLAUDE.md`'s four levels, spelled for the one act they govern here: `refuse`, `ask`, `warn`
/// and `open`. [`RESTRICTIONS`]'s four words are deliberately **not** reused, and the reason is
/// that they would mean the opposite thing: there *off* is the permissive end, because the subject
/// is a restriction the document asserts and turning it off lets the operation through. The
/// subject here is this machine starting another program on a string a document chose, so the
/// permissive end is `open`, and a reader who read `off` as permissive would have turned the wrong
/// way. ADR 1155.
pub const LINKS: &str = "--links=";

/// What this reader does when §12.6.4.8's action reaches a resolved URI.
///
/// `CLAUDE.md` principle 3's four levels over the one decision [`may_open_uri`] takes. The levels
/// are the same four and the *direction* is the other one, which is what [`LINKS`] spells out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Links {
    /// Hand it to nothing: the link is declined and the URI said out loud.
    Refuse,
    /// Put the URI to the person first, and hand it over on a `yes`.
    ///
    /// **The default, and it is a choice rather than a convenience.** Two things have to hold at
    /// once: a document never reaches another program on this machine by itself, and a reader is
    /// not refused by their own viewer the act the clause describes. *Ask* is the only level that
    /// is both — nothing is handed over without a person pressing a key, and nobody has to
    /// configure the program before a link works. [`Links::Refuse`] is one word away for a reader
    /// who wants neither the question nor the link, and a face with no dialogue answers this level
    /// with [`unanswerable`] and hands nothing over, which is what keeps *ask* from behaving like
    /// *open* in silence. ADR 1155.
    #[default]
    Ask,
    /// Hand it over, and say afterwards what was handed to what.
    Warn,
    /// Hand it over without asking.
    Open,
}

impl Links {
    /// All four, in the order a person reads them: least permissive to most.
    pub const ALL: [Self; 4] = [Self::Refuse, Self::Ask, Self::Warn, Self::Open];

    /// The word [`LINKS`] takes for this level.
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

/// Reads [`LINKS`]'s word onto a level, or says what is wrong with it.
///
/// # Errors
///
/// The sentence to print, naming every word this option takes — [`restrictions`]'s shape, because
/// a person meets both on the same command line.
pub fn links(word: &str) -> Result<Links, String> {
    Links::parse(word).ok_or_else(|| {
        format!(
            "{LINKS}{word}: no such level. One of {}",
            Links::ALL.map(Links::as_str).join(", ")
        )
    })
}

/// The schemes this reader will hand to another program, whatever the level.
///
/// **A documented choice, and the narrowest one that still performs what the clause describes.**
/// §12.6.4.8 introduces the string as one that "identifies (resolves to) a resource on the
/// Internet", so a scheme naming something else on this machine is outside what the clause is
/// about — and a document free to pick any scheme would be picking which of this machine's
/// handlers runs. `file` is deliberately absent although [`resolve_uri`] produces one for every
/// partial reference beside the document: opening a file a *document* named is §12.7.6.4's hazard
/// one clause over, where [`read_import`] answers it with a directory a **person** supplied, and
/// nothing here may be looser than that. Widening this list is a change in one place. ADR 1155.
pub const LINK_SCHEMES: [&str; 3] = ["http", "https", "mailto"];

/// The program this machine opens a URI with.
///
/// The desktop's own handler rather than a browser named here: which program a scheme belongs to
/// is the person's setting and not this reader's, and a viewer that named one would be choosing it
/// for them.
pub const URI_HANDLER: &str = "xdg-open";

/// What a host does about one §12.6.4.8 URI, under the level the reader set.
///
/// `pdf_model::restriction::Verdict`'s four arms, for the reason that type has them: a policy with
/// four levels answers in four ways, and a host matching three of them would have a level that
/// silently behaved like another one. Closed, and **not** `#[non_exhaustive]`, for
/// `doc/ui-boundary.md`'s reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Opening {
    /// Hand it to [`open_uri`] now.
    Proceed,
    /// Hand it to [`open_uri`] now, and say this afterwards.
    Warn(String),
    /// Put this question to the person, and hand it over on a `yes`.
    Ask(crate::restriction::Question),
    /// Do not hand it over. The sentence says why, for [`uri_note`].
    Refuse(String),
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
/// ADR 1062's: the policy is asked once, in a place a host can supply, so a level a reader sets is
/// a value here rather than four windows' worth of editing (ADR 1079). `CLAUDE.md` principle 3 is
/// what makes that shape the requirement — "[a] refusal that cannot become an 'ask' is the thing
/// to avoid" — and [`Links`] is the ask.
///
/// **Two questions are answered before the level is consulted, and the order is the point.** A
/// reference nothing could resolve names no resource at any level, and a scheme outside
/// [`LINK_SCHEMES`] is one this machine does not hand to a handler however permissive the reader
/// is — so neither is a thing *open* turns on. What the level decides is the one act that is left.
#[must_use]
pub fn may_open_uri(uri: &str, level: Links) -> Opening {
    if !pdf_model::uri::is_absolute(uri) {
        return Opening::Refuse(
            "it is still a relative reference and names no resource: the document states no /Base \
             and this window could not name its own location (ISO 32000-2 §12.6.4.8)"
                .to_owned(),
        );
    }
    let Some(scheme) = scheme_of(uri) else {
        return Opening::Refuse(
            "it states no scheme this reader could read, so there is nothing to open it with \
             (ISO 32000-2 §12.6.4.8, RFC 3986 section 3.1)"
                .to_owned(),
        );
    };
    if !LINK_SCHEMES.contains(&scheme.as_str()) {
        return Opening::Refuse(format!(
            "{scheme}: is not one of the schemes this reader hands to another program ({}), and a \
             document does not get to choose which of this machine's handlers runs",
            LINK_SCHEMES.join(", ")
        ));
    }
    match level {
        Links::Refuse => Opening::Refuse(format!(
            "this reader is set to open nothing itself ({LINKS}{}); {LINKS}{} puts the URI to you \
             first",
            Links::Refuse.as_str(),
            Links::Ask.as_str()
        )),
        Links::Ask => Opening::Ask(asked_to_open(uri)),
        Links::Warn => Opening::Warn(format!(
            "it was handed to {URI_HANDLER} without asking you first, because this reader is \
             set to {LINKS}{}",
            Links::Warn.as_str()
        )),
        Links::Open => Opening::Proceed,
    }
}

/// RFC 3986 section 3.1's scheme, lower-cased.
///
/// The grammar is a letter followed by letters, digits, `+`, `.` and `-`, up to the first colon.
/// Lower-cased because that section accepts any case while asking a producer for lower case, so a
/// `HTTP:` link is the same link and a comparison against [`LINK_SCHEMES`] that missed it would
/// refuse for the wrong reason.
///
/// A *policy* question rather than URI arithmetic, which is why it is answered here and not in
/// `pdf_model::uri`: what is decided is which of this machine's handlers a document may start, and
/// `pdf_model::uri::resolve` neither knows that nor should.
fn scheme_of(uri: &str) -> Option<String> {
    let colon = uri.find(':')?;
    let scheme = uri.get(..colon)?;
    let mut characters = scheme.chars();
    if !characters.next()?.is_ascii_alphabetic() {
        return None;
    }
    characters
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '+' | '.' | '-'))
        .then(|| scheme.to_ascii_lowercase())
}

/// What a window puts in front of a person at [`Links::Ask`].
///
/// [`crate::restriction::asked`]'s two-string shape, because a reader who has met one of this
/// program's questions has met the other and no window may word either for itself. What differs is
/// the subject: the restriction prompt says what the *document* asserts, and this one says what the
/// document asked this machine to **start** — so the URI is in the first string whole and
/// unabbreviated, because the URI is the only thing a person can judge this by.
#[must_use]
pub fn asked_to_open(uri: &str) -> crate::restriction::Question {
    crate::restriction::Question {
        reasons: format!(
            "A link in this document asks to open {uri}. This reader would hand that URI to \
             {URI_HANDLER}, which starts whichever program this machine opens that scheme with."
        ),
        choice: format!(
            "You have set this reader to ask before opening a link ({LINKS}{}). \"{}\" opens this \
             one and leaves the level where it is; \"{}\" leaves it unopened. {LINKS}{} stops the \
             question being asked, and {LINKS}{} stops links being opened at all.",
            Links::Ask.as_str(),
            crate::restriction::GO_AHEAD,
            crate::restriction::DO_NOT,
            Links::Open.as_str(),
            Links::Refuse.as_str()
        ),
    }
}

/// Hands §12.6.4.8's URI to [`URI_HANDLER`], which is the act the clause describes:
///
/// > A URI action causes a URI to be resolved.
///
/// **The scheme is checked again here, and that is not a duplicate of [`may_open_uri`]'s check.**
/// This function is what actually starts a program on a string a *document* chose, so it is the
/// last place the guarantee can be made, and a host that reached it without asking the policy
/// would otherwise hand over anything. The check is a `contains` over three words; what getting it
/// wrong costs is a document choosing which handler on this machine runs.
///
/// **The child is waited for on a thread of its own.** [`URI_HANDLER`] may run for as long as the
/// program it starts, so a window that waited would stop drawing; and a child nobody waits for
/// stays a zombie until this process exits. The thread is also where the handler's own failure is
/// read — a scheme with no handler is the common one — and it is said rather than discarded, which
/// is trap 5 on a path a person clicked.
///
/// # Errors
///
/// The sentence to say to the person, where nothing was started at all: a scheme this reader does
/// not hand over, or a machine with no [`URI_HANDLER`] on its path.
pub fn open_uri(uri: &str) -> Result<(), String> {
    if !scheme_of(uri).is_some_and(|scheme| LINK_SCHEMES.contains(&scheme.as_str())) {
        return Err(format!(
            "{uri} names no scheme this reader hands to another program ({})",
            LINK_SCHEMES.join(", ")
        ));
    }
    let mut child = std::process::Command::new(URI_HANDLER)
        .arg(uri)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|error| format!("{URI_HANDLER} could not be started: {error}"))?;
    let handled = uri.to_owned();
    std::thread::Builder::new()
        .name("uri-handler".to_owned())
        .spawn(move || match child.wait() {
            Ok(status) if status.success() => {}
            Ok(status) => eprintln!("note: {URI_HANDLER} {handled}: {status}"),
            Err(error) => {
                eprintln!("note: {URI_HANDLER} {handled} could not be waited for: {error}");
            }
        })
        .map_err(|error| format!("no thread to wait for {URI_HANDLER} on: {error}"))?;
    Ok(())
}

/// What a host does about one §12.6.4.8 URI: a sentence to say, or a question to put.
///
/// **One entry point for four faces, and [`crate::keys`]'s argument is the reason.** What a window
/// is obeying is shared and what a dialogue looks like is a toolkit's, so the act, the level and
/// every sentence around them are decided once here, and a `gtk4::Window`, a `QDialog` and a card
/// this program draws are what a host supplies. A host that composed its own would be the third
/// copy where two stop agreeing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Link {
    /// Say this. Whatever act the level called for has already been carried out.
    Say(String),
    /// Put this question to the person, and hand the answer to [`answered`].
    Ask(crate::restriction::Question),
}

/// The whole of what a host does about one resolved §12.6.4.8 URI, minus the dialogue.
///
/// [`may_open_uri`] is the decision and this is the decision carried out: three of the four levels
/// end in a sentence and the fourth ends in a question. A host matches two arms rather than four,
/// which is what keeps *ask* from being the level a window forgets to implement.
#[must_use]
pub fn link(uri: &str, level: Links) -> Link {
    match may_open_uri(uri, level) {
        Opening::Proceed => Link::Say(handed_over(uri, None)),
        Opening::Warn(note) => Link::Say(handed_over(uri, Some(&note))),
        Opening::Ask(question) => Link::Ask(question),
        Opening::Refuse(why) => Link::Say(uri_note(uri, Some(&why))),
    }
}

/// What a host does with the answer to [`Link::Ask`], and the sentence to say about it.
///
/// A decline is said out loud rather than passed over in silence: the person asked for a link and
/// is owed the fact that it was not opened, which is [`crate::restriction::declined`]'s reason one
/// clause over.
#[must_use]
pub fn answered(uri: &str, proceed: bool) -> String {
    if proceed {
        return handed_over(uri, None);
    }
    uri_note(
        uri,
        Some(&format!("you answered \"{}\"", crate::restriction::DO_NOT)),
    )
}

/// [`open_uri`], with the one line the person is owed about what happened.
///
/// `note` is what the level adds after the fact — [`Links::Warn`]'s sentence — and it is appended
/// only where the URI actually went somewhere, because a warning about an act that did not happen
/// would be a sentence about nothing.
fn handed_over(uri: &str, note: Option<&str>) -> String {
    match open_uri(uri) {
        Ok(()) => match note {
            Some(note) => format!("{} — {note}", uri_note(uri, None)),
            None => uri_note(uri, None),
        },
        Err(why) => uri_note(uri, Some(&why)),
    }
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

/// The word a person types to ask for ISO 32000-2 §10.8.3's separation simulation.
///
/// `on` or `off`, and **not** one of `CLAUDE.md`'s four levels: those are for what a *document*
/// asserts over its reader, and §10.8.3 is the reader asking for a different picture of their own
/// document. There is nothing here to ask a person about and nothing to warn them of, so two
/// words are the whole vocabulary (ADR 1189 section 2 draws the same line about `/SA`).
///
/// **Off unless a person says otherwise**, which is what §10.8.1 leaves it as — "[w]hether
/// separations are produced is up to the processing software" — and what §10.8.2 describes a
/// screen doing: the alternate colour space and its tint transform, with the overprint controls
/// ignored. ADR 1228.
pub const SEPARATIONS: &str = "--separations=";

/// Reads [`SEPARATIONS`]'s word, or says what is wrong with it.
///
/// # Errors
///
/// The sentence to print, naming both words this option takes — [`links`]'s shape, because a
/// person meets them on the same command line.
pub fn separations(word: &str) -> Result<bool, String> {
    match word {
        "on" => Ok(true),
        "off" => Ok(false),
        _ => Err(format!(
            "{SEPARATIONS}{word}: no such setting. One of on, off"
        )),
    }
}

/// What a window says when §10.8.3's simulation is turned on or off.
///
/// One sentence in one place, for [`refused`]'s reason: three windows each writing it is where
/// two of them stop agreeing. It names the clause because a person who has just seen every colour
/// on the page change is owed what changed it.
#[must_use]
pub fn separations_note(simulating: bool) -> String {
    if simulating {
        format!(
            "separation simulation is on ({SEPARATIONS}on): colours are drawn as a press making \
             separations would produce them (ISO 32000-2 §10.8.3)"
        )
    } else {
        format!(
            "separation simulation is off ({SEPARATIONS}off): colours are drawn through each \
             space's own alternate and tint transform (ISO 32000-2 §10.8.2)"
        )
    }
}

/// The values a person's command line sets, carried into a window as one thing.
///
/// **One argument rather than four, and `viewer-ui`'s own `Policies` is the precedent**: these are
/// one subject — the decisions `CLAUDE.md` principle 3 says are a *reader's* rather than a
/// document's — and a window's constructor taking them one by one is a signature that grows by one
/// every time this module answers another clause. What is *not* here is anything a document states:
/// each of these is a fact about the person at the keyboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Settings {
    /// How much of what a document asserts over its reader this window obeys (`RESTRICTIONS`).
    pub restrictions: viewer_core::RestrictionPolicy,
    /// What §12.6.4.8's link does on this machine ([`LINKS`]).
    pub links: Links,
    /// What §12.6.4.3's remote go-to does ([`REMOTE_DOCUMENTS`]).
    pub remote_documents: RemoteDocuments,
    /// What §O.2.1's `ef` does ([`EMBEDDED_DOCUMENTS`]).
    pub embedded_documents: EmbeddedDocuments,
    /// What a submission or a fetched import sends to a server ([`SUBMISSIONS`]).
    pub submissions: Submissions,
    /// Whether §10.8.3's separation simulation is asked for ([`SEPARATIONS`]).
    pub separations: bool,
    /// Whether a document's scripts run ([`SCRIPTS`]).
    pub scripts: Scripts,
}

/// The word a person types to say which PDFs beside their own this reader will parse.
///
/// `CLAUDE.md`'s four levels over one act: `refuse`, `ask`, `warn` and `open`, the same four
/// words and the same direction as [`LINKS`], because the permissive end is again the one where
/// something the *document* named is acted on.
///
/// **One act, reached through three tables** — §12.6.4.3's Table 203 `/F`, §12.6.4.7's Table 209
/// `/F` and §12.7.8's Table 253 `/F` — which [`under_remote_documents`] is the list of and
/// ADR 1239 the argument for.
///
/// **A value of its own rather than [`Links`] reused, and the division is ADR 1155's own.** That
/// ADR put `file` outside [`LINK_SCHEMES`] on the ground that opening a file a document named is
/// a different question, decided by a person-supplied directory rather than by the link level —
/// so a reader who set `{LINKS}open` said that their browser may be started on a URL, and has
/// said nothing about which PDFs beside their document may be parsed; a reader who set
/// `{LINKS}refuse` said they want no other program started, and has not said they may not follow
/// a cross-reference inside their own set of documents. One value for both would make each of
/// those two sentences mean the other. ADR 1227.
pub const REMOTE_DOCUMENTS: &str = "--remote-documents=";

/// What this reader does when a document names another PDF file.
///
/// `CLAUDE.md` principle 3's four levels over the one decision [`remote`] takes, whichever of
/// [`under_remote_documents`]'s three purposes asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RemoteDocuments {
    /// Open nothing: the action is declined and the file the document named said out loud.
    Refuse,
    /// Put the file to the person first, and open it on a `yes`.
    ///
    /// **The default, on [`Links::Ask`]'s argument applied to this act.** Two things have to hold
    /// at once: a document never reaches a file on this disk by itself, and a reader is not
    /// refused by their own viewer the jump §12.6.4.3 describes — a set of documents that
    /// cross-reference each other is what the clause exists for. *Ask* is the only level that is
    /// both. A face with no dialogue answers it with [`unanswerable`] and supplies nothing, which
    /// is what keeps *ask* from behaving like *open* in silence. ADR 1227.
    #[default]
    Ask,
    /// Open it, and say afterwards which file was opened.
    Warn,
    /// Open it without asking.
    Open,
}

impl RemoteDocuments {
    /// All four, in the order a person reads them: least permissive to most.
    pub const ALL: [Self; 4] = [Self::Refuse, Self::Ask, Self::Warn, Self::Open];

    /// The word [`REMOTE_DOCUMENTS`] takes for this level.
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

/// Reads [`REMOTE_DOCUMENTS`]'s word onto a level, or says what is wrong with it.
///
/// # Errors
///
/// The sentence to print, naming every word this option takes — [`links`]'s shape, because a
/// person meets both on the same command line.
pub fn remote_documents(word: &str) -> Result<RemoteDocuments, String> {
    RemoteDocuments::parse(word).ok_or_else(|| {
        format!(
            "{REMOTE_DOCUMENTS}{word}: no such level. One of {}",
            RemoteDocuments::ALL.map(RemoteDocuments::as_str).join(", ")
        )
    })
}

/// Whether [`REMOTE_DOCUMENTS`]'s level decides this purpose's file, or [`read_import`] does.
///
/// **One function rather than the same pattern in three windows**, which is [`asked_for`]'s reason
/// applied to the other half of the same arm: every host matches `Event::NeedsFile` and has to
/// route it, and three copies of a three-armed pattern is where a fourth purpose arrives in two
/// windows and not the third.
///
/// The division is ADR 1227's and ADR 1239 applies it: §12.7.6.4's own file holds another
/// document's *values* and is resolved by the path rule alone, while §12.6.4.3's, §12.6.4.6's,
/// §12.6.4.7's and §12.7.8's each make this program parse a second PDF — which is what a reader
/// answered for when they set the word. §12.6.4.6's file may be an application instead, which is
/// found out after it is read and started never (ADRs 1368, 1358). §12.6.4.4's root document is the one that looks like an exception and is
/// not: its bytes are named by a document too, and it stays with the import until somebody argues
/// otherwise rather than being moved in passing.
#[must_use]
pub const fn under_remote_documents(purpose: Purpose) -> bool {
    match purpose {
        Purpose::RemoteDocument
        | Purpose::NamedPage
        | Purpose::ThreadDocument
        | Purpose::LaunchDocument => true,
        Purpose::ImportData | Purpose::TargetRoot => false,
    }
}

/// What a host does about one §12.6.4.3 file: a file to read, a question to put, or a refusal.
///
/// [`Link`]'s shape and for [`Link`]'s reason — the act, the level and every sentence around them
/// are decided once here, and what a dialogue looks like is a toolkit's. The path is carried
/// rather than the bytes because reading a file is input/output and this is a policy:
/// [`resolve_import`] is the decision and `std::fs::read` is what follows it, which is the same
/// split [`read_import`] is one half of.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Remote {
    /// Read this file and supply it. `note` is what [`RemoteDocuments::Warn`] adds afterwards.
    Supply {
        /// The file, resolved by [`resolve_import`]'s rule and nothing else.
        path: PathBuf,
        /// What to say once it has been opened, where the level asks for a sentence.
        note: Option<String>,
    },
    /// Put this question to the person; on a `yes`, read `path` and supply it.
    Ask {
        /// The file the answer is about.
        path: PathBuf,
        /// The words, from [`asked_to_open_remote`].
        question: crate::restriction::Question,
    },
    /// Supply nothing, and say this.
    Refuse(String),
}

/// Whether §12.6.4.3's named file may be **opened in this reader**, and from where.
///
/// §12.6.4.3 describes an action that "jumps to a destination in another PDF file instead of the
/// current file" and Table 203 makes `/F` "[t]he file in which the destination shall be located".
/// Which files a document may name is a property of the *processor* and not of the format, which
/// is the same sentence §12.7.6.4's import-data action is answered with — so the path is resolved
/// by [`resolve_import`]'s two rules and by nothing looser, and the level decides the act that is
/// left.
///
/// **The path rule comes before the level, and that order is the point.** A name outside the
/// document's own directory is refused at every level including *open*: a reader who said their
/// documents may cross-reference each other did not say that any file on this disk may be opened
/// on a document's say-so, and ADR 1155 fixed that nothing may be looser than the import policy.
/// What *open* turns on is the one act that is left. ADR 1227.
///
/// **Three purposes rather than one, and the level is the same for all three** (ADR 1239).
/// §12.7.8's Table 253 `/F` and §12.6.4.7's Table 209 `/F` name a second **PDF** the way Table
/// 203's does, and ADR 1227 divided the levels by what the act is: [`Links`] decides whether
/// another program on this machine is started, and this one decides which PDFs beside the
/// reader's own document this program parses. All three are that, so a reader who set one word
/// has answered for all three; what differs is the sentence, which [`what_would_happen`] supplies
/// because only two of the three replace the document on the screen.
#[must_use]
pub fn remote(
    directory: Option<&Path>,
    name: &str,
    level: RemoteDocuments,
    purpose: Purpose,
    beside: bool,
) -> Remote {
    let path = match resolve_import(directory, name) {
        Ok(path) => path,
        Err(refusal) => {
            return Remote::Refuse(supply_note(purpose, &refusal.to_string()));
        }
    };
    match level {
        RemoteDocuments::Refuse => Remote::Refuse(remote_note(
            purpose,
            name,
            Some(&format!(
                "this reader is set to open no file a document names ({REMOTE_DOCUMENTS}{}); \
                 {REMOTE_DOCUMENTS}{} puts the file to you first",
                RemoteDocuments::Refuse.as_str(),
                RemoteDocuments::Ask.as_str()
            )),
        )),
        RemoteDocuments::Ask => Remote::Ask {
            question: asked_to_open_remote(purpose, name, &path, beside),
            path,
        },
        RemoteDocuments::Warn => Remote::Supply {
            path,
            note: Some(format!(
                "it was opened without asking you first, because this reader is set to \
                 {REMOTE_DOCUMENTS}{}",
                RemoteDocuments::Warn.as_str()
            )),
        },
        RemoteDocuments::Open => Remote::Supply { path, note: None },
    }
}

/// What a window puts in front of a person at [`RemoteDocuments::Ask`].
///
/// [`asked_to_open`]'s two-string shape and for its reason: a reader who has met one of this
/// program's questions has met the other, and no window may word either for itself. What differs
/// is the subject — this one names the file the document asked for **and** the path it resolved
/// to, because the second is the only thing that says which file on this disk would be read.
/// `beside` is `viewer_core::Event::NeedsFile`'s, and says whether it would open beside the
/// document being read or in place of it.
#[must_use]
pub fn asked_to_open_remote(
    purpose: Purpose,
    name: &str,
    path: &Path,
    beside: bool,
) -> crate::restriction::Question {
    crate::restriction::Question {
        reasons: format!(
            "This document asks to open {name}, which is {} beside the document. {}",
            path.display(),
            what_would_happen(purpose, beside)
        ),
        choice: format!(
            "You have set this reader to ask before opening a file a document names \
             ({REMOTE_DOCUMENTS}{}). \"{}\" opens this one and leaves the level where it is; \
             \"{}\" leaves it unopened. {REMOTE_DOCUMENTS}{} stops the question being asked, and \
             {REMOTE_DOCUMENTS}{} stops such a file being opened at all.",
            RemoteDocuments::Ask.as_str(),
            crate::restriction::GO_AHEAD,
            crate::restriction::DO_NOT,
            RemoteDocuments::Open.as_str(),
            RemoteDocuments::Refuse.as_str()
        ),
    }
}

/// What a host says about a remote go-to, whether it opens the file or declines.
///
/// [`uri_note`]'s shape, for [`uri_note`]'s reason: a person who clicked a link is owed *what it
/// named* rather than the word "declined".
#[must_use]
pub fn remote_note(purpose: Purpose, name: &str, refused: Option<&str>) -> String {
    let clause = asked_for(purpose);
    match refused {
        Some(why) => format!("{clause}: declined — {why}. The document asked for {name}"),
        None => format!("{clause}: {name}"),
    }
}

/// What a host says about the answer to [`Remote::Ask`] when the person said no.
///
/// A decline is said out loud rather than passed over in silence, which is [`answered`]'s reason
/// one clause over. A `yes` is said by whatever the core reports about the document it opened, so
/// this has nothing to add to one.
#[must_use]
pub fn remote_declined(purpose: Purpose, name: &str) -> String {
    remote_note(
        purpose,
        name,
        Some(&format!("you answered \"{}\"", crate::restriction::DO_NOT)),
    )
}

/// The word a person types to say what this reader does when a URI's fragment names an embedded
/// document.
///
/// [`REMOTE_DOCUMENTS`]' four words and direction, for the act [`EmbeddedDocuments`] governs. The
/// menu's third group sets the same value while the window is up; the word is what a window is
/// started at, as [`LINKS`] and [`REMOTE_DOCUMENTS`] are (ADR 1331).
pub const EMBEDDED_DOCUMENTS: &str = "--embedded-documents=";

/// Reads [`EMBEDDED_DOCUMENTS`]' word onto a level, or says what is wrong with it.
///
/// # Errors
///
/// The sentence to print, naming every word this option takes — [`links`]' shape.
pub fn embedded_documents(word: &str) -> Result<EmbeddedDocuments, String> {
    EmbeddedDocuments::parse(word).ok_or_else(|| {
        format!(
            "{EMBEDDED_DOCUMENTS}{word}: no such level. One of {}",
            EmbeddedDocuments::ALL
                .map(EmbeddedDocuments::as_str)
                .join(", ")
        )
    })
}

/// The word a person types to say what this reader does when a document asks it to send something
/// to a server — [`Submissions`]' four words, for §12.7.6.2's form and Table Annex O.4's fetched
/// `fdf` alike.
///
/// The menu's third group sets the same value while the window is up; the word is what a window is
/// started at, as [`EMBEDDED_DOCUMENTS`] is (ADR 1527).
pub const SUBMISSIONS: &str = "--submissions=";

/// Reads [`SUBMISSIONS`]' word onto a level, or says what is wrong with it.
///
/// # Errors
///
/// The sentence to print, naming every word this option takes — [`links`]' shape.
pub fn submissions(word: &str) -> Result<Submissions, String> {
    Submissions::parse(word).ok_or_else(|| {
        format!(
            "{SUBMISSIONS}{word}: no such level. One of {}",
            Submissions::ALL.map(Submissions::as_str).join(", ")
        )
    })
}

/// The word a person types to say whether a document's scripts run — RFC 0008 section 6.3's four
/// levels, `--scripts off|ask|warn|on`, read by [`crate::ReaderWords`] for every window (ADR 1581).
///
/// The menu's third group sets the same value while the window is up; the word is what a window is
/// started at, as [`SUBMISSIONS`] is.
pub const SCRIPTS: &str = "--scripts";

/// Whether this reader runs a document's scripts: RFC 0008 section 6.3's policy hook, at one of
/// `CLAUDE.md`'s four levels.
///
/// **The RFC's own words, `off` to `on`**, rather than [`Links`]' `refuse` to `open`: the subject
/// is again this machine doing what a document asked, and the permissive end is again last — but
/// what a script does is *run*, and `on` says so where `open` would not. A level decides whether
/// the engine runs and never what a script may reach: every call that leaves the process is
/// refused by name at every level (RFC 0008 section 6.3, ADR 1616).
///
/// **Global rather than per document**, [`Submissions`]' reason; what *ask* asks is per document,
/// once, and its answer is the document's for as long as it is open.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Scripts {
    /// No script runs: no runner is supplied, no process is started, and every script a view state
    /// does not run itself says it was not run.
    ///
    /// **The default, and the owner's** (`doc/questions/A193`, RFC 0008's first question): `off`
    /// until the `script_corpus` gate has been green for a batch, when the default is asked again
    /// with the gate's number. The argument for *ask* everywhere else — a person is clicking
    /// something they can see — fails here, because a script runs on open and per keystroke.
    #[default]
    Off,
    /// One question per document at its first script, showing that script's first line; the
    /// answer holds until the document closes.
    Ask,
    /// Scripts run, and the report says which ran and what each made of its field.
    Warn,
    /// Scripts run, reported in the panel and nowhere louder.
    On,
}

impl Scripts {
    /// All four, least permissive first — the order a menu offers them in.
    pub const ALL: [Self; 4] = [Self::Off, Self::Ask, Self::Warn, Self::On];

    /// The word a person reads for this level.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Ask => "ask",
            Self::Warn => "warn",
            Self::On => "on",
        }
    }

    /// The level a word names, or `None` for a word that names none.
    #[must_use]
    pub fn parse(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|level| level.as_str() == word)
    }
}

/// Reads [`SCRIPTS`]' word onto a level, or says what is wrong with it.
///
/// # Errors
///
/// The sentence to print, naming every word this option takes — [`links`]' shape.
pub fn scripts(word: &str) -> Result<Scripts, String> {
    Scripts::parse(word).ok_or_else(|| {
        format!(
            "{SCRIPTS} {word}: no such level. One of {}",
            Scripts::ALL.map(Scripts::as_str).join(", ")
        )
    })
}

/// What a window sends the viewer for a level: [`viewer_core::Command::Scripts`]'s value.
///
/// **The one place a level becomes a runner**, for [`may_submit`]'s reason: every window asks it
/// here rather than building its own. At `on`, `warn` and an answered `ask` each document is handed
/// a `pdf_script_worker::ScriptWorker`, which starts the worker program at the document's first
/// trigger and never before (RFC 0008 section 6.6); at `warn` it is wrapped in [`Warning`]. At
/// `off` nothing is constructed at all.
#[must_use]
pub fn scripting(level: Scripts) -> viewer_core::Scripting {
    match level {
        Scripts::Off => viewer_core::Scripting::Off,
        Scripts::Ask => viewer_core::Scripting::Ask(std::sync::Arc::new(Workers { warn: false })),
        Scripts::Warn => viewer_core::Scripting::Run(std::sync::Arc::new(Workers { warn: true })),
        Scripts::On => viewer_core::Scripting::Run(std::sync::Arc::new(Workers { warn: false })),
    }
}

/// The environment name for how long, in milliseconds, a script's question waits on a person
/// before its runner answers it as a closed dialogue does.
///
/// `pdf_script_worker::ANSWER_WAIT` where it is unset or is not a number: the wait RFC 0008 section
/// 4.2's questions are given (ADR 1627). The name exists so that `tools/drive-windows.sh` can watch
/// a question withdrawn in each window without waiting the whole of it, the way
/// [`MACHINE_FONTS_VARIABLE`] lets a window started from no terminal take a setting (ADR 1643).
pub const SCRIPT_ANSWER_WAIT_VARIABLE: &str = "PDF_VIEWER_SCRIPT_ANSWER_WAIT_MS";

/// How long a script's question waits on a person: [`SCRIPT_ANSWER_WAIT_VARIABLE`] where it states
/// a number of milliseconds, `pdf_script_worker::ANSWER_WAIT` otherwise.
#[must_use]
pub fn script_answer_wait() -> std::time::Duration {
    std::env::var(SCRIPT_ANSWER_WAIT_VARIABLE)
        .ok()
        .and_then(|millis| millis.trim().parse::<u64>().ok())
        .map_or(
            pdf_script_worker::ANSWER_WAIT,
            std::time::Duration::from_millis,
        )
}

/// What makes each document's runner at the levels that run scripts.
#[derive(Debug)]
struct Workers {
    /// Whether each run is said, which is `warn`'s difference from `on`.
    warn: bool,
}

impl viewer_core::ScriptRunners for Workers {
    fn runner(&self) -> std::sync::Arc<dyn pdf_model::view::ScriptRunner> {
        self.runner_that_asks().0
    }

    /// The worker is its own desk: a script that asks is held in it, and the question and the
    /// answer cross through it (ADRs 1627, 1628).
    fn runner_that_asks(
        &self,
    ) -> (
        std::sync::Arc<dyn pdf_model::view::ScriptRunner>,
        Option<std::sync::Arc<dyn viewer_core::ScriptAsks>>,
    ) {
        let worker = std::sync::Arc::new(
            pdf_script_worker::ScriptWorker::new().with_answer_wait(script_answer_wait()),
        );
        let desk: std::sync::Arc<dyn viewer_core::ScriptAsks> = std::sync::Arc::new(Desk {
            worker: std::sync::Arc::clone(&worker),
        });
        let runner: std::sync::Arc<dyn pdf_model::view::ScriptRunner> = if self.warn {
            std::sync::Arc::new(Warning { worker })
        } else {
            worker
        };
        (runner, Some(desk))
    }
}

/// Where a worker's held script puts its question and is handed its answer: the two crates'
/// shapes, converted one for one — `pdf_script`'s carries a script's numbers, `viewer_core`'s a
/// window's words, and they mean the same (ADR 1628).
#[derive(Debug)]
struct Desk {
    /// The worker whose script asks.
    worker: std::sync::Arc<pdf_script_worker::ScriptWorker>,
}

impl viewer_core::ScriptAsks for Desk {
    fn take_question(&self) -> Option<viewer_core::ScriptQuestion> {
        Some(asked(self.worker.take_question()?))
    }

    fn answer(&self, answer: viewer_core::ScriptAnswer) {
        self.worker.answer(answered_with(&answer));
    }

    fn withdrawn(&self) -> bool {
        self.worker.question_withdrawn()
    }
}

/// A worker's question as a window is handed it.
fn asked(question: pdf_script::Question) -> viewer_core::ScriptQuestion {
    use viewer_core::{AlertButtons, AlertIcon};
    match question {
        pdf_script::Question::Alert {
            message,
            icon,
            buttons,
            title,
        } => viewer_core::ScriptQuestion::Alert {
            message,
            icon: match icon {
                pdf_script::Icon::Error => AlertIcon::Error,
                pdf_script::Icon::Warning => AlertIcon::Warning,
                pdf_script::Icon::Question => AlertIcon::Question,
                pdf_script::Icon::Status => AlertIcon::Status,
            },
            buttons: match buttons {
                pdf_script::Buttons::Ok => AlertButtons::Ok,
                pdf_script::Buttons::OkCancel => AlertButtons::OkCancel,
                pdf_script::Buttons::YesNo => AlertButtons::YesNo,
                pdf_script::Buttons::YesNoCancel => AlertButtons::YesNoCancel,
            },
            title,
        },
        pdf_script::Question::Response {
            question,
            title,
            default,
            label,
            password,
        } => viewer_core::ScriptQuestion::Response {
            question,
            title,
            default,
            label,
            password,
        },
    }
}

/// A window's answer as the worker's script is handed it.
fn answered_with(answer: &viewer_core::ScriptAnswer) -> pdf_script::Answer {
    use viewer_core::AlertButton;
    match answer {
        viewer_core::ScriptAnswer::Pressed(button) => pdf_script::Answer::Pressed(match button {
            AlertButton::Ok => pdf_script::Button::Ok,
            AlertButton::Cancel => pdf_script::Button::Cancel,
            AlertButton::No => pdf_script::Button::No,
            AlertButton::Yes => pdf_script::Button::Yes,
        }),
        viewer_core::ScriptAnswer::Typed(text) => pdf_script::Answer::Typed(text.clone()),
        viewer_core::ScriptAnswer::Unanswerable => pdf_script::Answer::Unanswerable,
    }
}

/// `warn`'s runner: the worker's, with one sentence per run saying that it ran and what it made of
/// its field.
///
/// The sentence joins the run's own report, which the view state prefixes with the script it is
/// about and says once; so a script run on every keystroke is said once per value it produced, not
/// once per key.
#[derive(Debug)]
struct Warning {
    /// The runner that runs it, shared with the desk its questions wait on.
    worker: std::sync::Arc<pdf_script_worker::ScriptWorker>,
}

impl pdf_model::view::ScriptRunner for Warning {
    fn run(&self, event: &pdf_model::view::ScriptEvent<'_>) -> pdf_model::view::ScriptResult {
        let mut result = self.worker.run(event);
        // A run held on a person's answer did nothing yet, so there is nothing to say it made;
        // what it makes arrives through `take_resumed`, and is said there (ADR 1628).
        if !self.worker.waiting() {
            result.report.push(ran(&result));
        }
        result
    }

    fn waiting(&self) -> bool {
        self.worker.waiting()
    }

    fn take_resumed(&self) -> Option<pdf_model::view::Resumed> {
        let mut resumed = self.worker.take_resumed()?;
        resumed.result.report.push(ran(&resumed.result));
        Some(resumed)
    }
}

/// What `warn` says of one run.
fn ran(result: &pdf_model::view::ScriptResult) -> String {
    let mut parts = vec![format!(
        "ran, because this reader is set to {} ({RUNNING_SCRIPTS})",
        Scripts::Warn.as_str()
    )];
    if let Some(value) = &result.value {
        parts.push(format!("its value is now {value:?}"));
    }
    let edits = result.edits.len();
    if edits > 0 {
        parts.push(format!("it changed the form {edits} time(s)"));
    }
    if !result.rc {
        parts.push("it refused the change it was handed".to_owned());
    }
    parts.join("; ")
}

/// The act's name under the menu's third group, and the name the sentences about its level use.
pub const RUNNING_SCRIPTS: &str = "running a document's scripts";

/// What a window puts in front of a person at [`Scripts::Ask`]: the first script a document handed
/// over, its first line, and what the two answers do.
#[must_use]
pub fn asked_to_run_scripts(script: &str, first_line: &str) -> crate::restriction::Question {
    crate::restriction::Question {
        reasons: format!(
            "This document carries scripts, and {script} is the first to run. Its first line \
             reads: {first_line}"
        ),
        choice: format!(
            "You have set this reader to {} before {RUNNING_SCRIPTS} ({SCRIPTS} {}). \"{}\" \
             runs this document's scripts, confined, until it closes; \"{}\" runs none of them. \
             The level stays where it is (RFC 0008 section 6.3).",
            Scripts::Ask.as_str(),
            Scripts::Ask.as_str(),
            crate::restriction::GO_AHEAD,
            crate::restriction::DO_NOT
        ),
    }
}

/// What a window says when a person answers [`asked_to_run_scripts`].
#[must_use]
pub fn scripts_answered(proceed: bool) -> String {
    if proceed {
        format!(
            "this document's scripts run until it closes, because you answered \"{}\"",
            crate::restriction::GO_AHEAD
        )
    } else {
        format!(
            "none of this document's scripts runs, because you answered \"{}\"",
            crate::restriction::DO_NOT
        )
    }
}

/// What `quorra-confined` says when it is asked for a level of [`SCRIPTS`] other than `off`, and
/// what it says of the level when asked: the window is pinned (`doc/questions/A193`).
///
/// The confined window's viewer is in its worker, and a runner is a process the window would have
/// to start beside it and hand across, which its wire does not carry; and it has no dialogue for
/// *ask* to put. So it runs no document script at any word, and says so rather than take a word it
/// would not obey (ADR 1616).
pub const SCRIPTS_PINNED: &str = "quorra-confined runs no document script: its level for scripts is pinned to off, because its \
     viewer is confined in a worker that a script runner cannot be handed to (RFC 0008 section \
     6.3, ADR 1616)";

/// What this reader does when §O.2.1's `ef` asks it to open an embedded document.
///
/// `CLAUDE.md` principle 3's four levels over the one decision [`may_open_extracted`] takes,
/// spelled `refuse`, `ask`, `warn` and `open` — [`Links`]' words and [`Links`]' direction, because
/// the subject is again this machine doing something a *document's* bytes asked for, so the
/// permissive end is the one where the file is opened (ADR 1155).
///
/// **A value of its own rather than [`RemoteDocuments`] reused.** That level decides which PDFs
/// *beside* the reader's own document on this disk are parsed; this one decides whether a file
/// carried *inside* it is. A reader who let their documents cross-reference each other has said
/// nothing about what a URI's fragment may unpack, and the annex attaches its caution to this act
/// alone (ADR 1331).
///
/// **Global rather than per document**, [`Submissions`]' reason: what a reader decides here is a
/// fact about the person, and it is set from the same group of the same menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EmbeddedDocuments {
    /// Open nothing: the fragment is declined and the file it named said out loud.
    Refuse,
    /// Put the file to the person first, and open it on a `yes`.
    ///
    /// **The default, and the argument is the one the owner ratified twice.** `doc/questions/A67`
    /// kept [`Links::Ask`] and `doc/questions/A98` chose [`Submissions::Ask`] so that a person is
    /// the one who decides: a document never reaches this machine by itself, and a reader is not
    /// refused by their own viewer the act the clause describes. Here the clause is §O.2.1's
    /// `shall` and its caution is the annex's own — a processor "may choose to prompt the user or
    /// even prevent opening of the file" — and *ask* is the prompt, which is the one answer that
    /// keeps both. A face with no dialogue answers it with [`unanswerable`] and opens nothing.
    /// ADR 1331.
    #[default]
    Ask,
    /// Open it, and say afterwards which file was opened and why nobody was asked.
    Warn,
    /// Open it without asking.
    Open,
}

impl EmbeddedDocuments {
    /// All four, in the order a person reads them: least permissive to most.
    pub const ALL: [Self; 4] = [Self::Refuse, Self::Ask, Self::Warn, Self::Open];

    /// The word a person reads for this level.
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

/// What a host does about one embedded document §O.2.1's `ef` named, under the level the reader
/// set.
///
/// [`Opening`]'s four arms, for its reason: a policy with four levels answers in four ways, and a
/// host matching three would have a level that silently behaved like another. Closed, and **not**
/// `#[non_exhaustive]`, for `doc/ui-boundary.md`'s reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unpacking {
    /// Open it now.
    Open,
    /// Open it now, and say this.
    Warn(String),
    /// Put this question to the person, and open it on a `yes`.
    Ask(crate::restriction::Question),
    /// Open nothing. The sentence says why.
    Refuse(String),
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
/// opening of the file." [`EmbeddedDocuments`] is that choice given to the reader: *ask* is the
/// prompt, *refuse* the prevention, and *warn* and *open* the `shall` carried out with and
/// without a sentence.
///
/// **The one place the level is read**, for [`may_submit`]'s reason and ADR 1062's: the policy is
/// asked once, in a place a host can supply, so three windows carry out one answer rather than
/// wording three. A file a *person* asked for from the files panel is not under the level at all —
/// the caution is about a sentence that is frequently not the reader's — so
/// [`Extraction::Asked`] is opened at every level. ADR 1331.
#[must_use]
pub fn may_open_extracted(asked: Extraction, name: &str, level: EmbeddedDocuments) -> Unpacking {
    if matches!(asked, Extraction::Asked) {
        return Unpacking::Open;
    }
    match level {
        EmbeddedDocuments::Refuse => Unpacking::Refuse(embedded_note(
            name,
            Some(&format!(
                "this reader is set to open no embedded document a URI names ({OPENING_EMBEDDED}: \
                 {}); {} puts the file to you first (ISO 32000-2 §O.2.1)",
                EmbeddedDocuments::Refuse.as_str(),
                EmbeddedDocuments::Ask.as_str()
            )),
        )),
        EmbeddedDocuments::Ask => Unpacking::Ask(asked_to_open_embedded(name)),
        EmbeddedDocuments::Warn => Unpacking::Warn(format!(
            "{} — it was opened without asking you first, because this reader is set to {} \
             ({OPENING_EMBEDDED})",
            embedded_note(name, None),
            EmbeddedDocuments::Warn.as_str()
        )),
        EmbeddedDocuments::Open => Unpacking::Open,
    }
}

/// The act [`EmbeddedDocuments`] governs, in the words the menu and the sentences use.
pub const OPENING_EMBEDDED: &str = "opening an embedded document";

/// What a window puts in front of a person at [`EmbeddedDocuments::Ask`].
///
/// [`asked_to_open`]'s two-string shape, for its reason. The name is the one the *document* filed
/// the file under, whole, because it is the only thing a person can judge this by — and the first
/// string says where the request came from, since §O.2.1's identifiers are "useful primarily when
/// referring to them from external to the PDF".
#[must_use]
pub fn asked_to_open_embedded(name: &str) -> crate::restriction::Question {
    crate::restriction::Question {
        reasons: format!(
            "The fragment of the URI this document was opened with asks to open {name:?}, a file \
             carried inside it (ISO 32000-2 §O.2.1's ef). It would open in a tab of its own."
        ),
        choice: format!(
            "You have set this reader to ask before {OPENING_EMBEDDED} ({}). \"{}\" opens this one \
             and leaves the level where it is; \"{}\" leaves it unopened. Setting it to {} in the \
             restrictions menu stops the question being asked, and {} stops such a file being \
             opened at all.",
            EmbeddedDocuments::Ask.as_str(),
            crate::restriction::GO_AHEAD,
            crate::restriction::DO_NOT,
            EmbeddedDocuments::Open.as_str(),
            EmbeddedDocuments::Refuse.as_str()
        ),
    }
}

/// What a host says about an embedded document §O.2.1's `ef` named, whether it opens or not.
///
/// [`uri_note`]'s shape, for its reason: a person is owed *what was named* rather than the word
/// "declined".
#[must_use]
pub fn embedded_note(name: &str, refused: Option<&str>) -> String {
    match refused {
        Some(why) => format!("ef: declined — {why}. The fragment asked for {name:?}"),
        None => format!("ef: {name:?}"),
    }
}

/// What a host says when a person answered [`Unpacking::Ask`] with a no.
///
/// Said rather than passed over, [`answered`]'s reason one clause over.
#[must_use]
pub fn embedded_declined(name: &str) -> String {
    embedded_note(
        name,
        Some(&format!("you answered \"{}\"", crate::restriction::DO_NOT)),
    )
}

/// Whether §7.11.4's extracted bytes are a document §O.2.1's `ef` asks this reader to open.
///
/// The fragment asked, and the bytes begin with §7.5.2's header. Anything else that came out —
/// a file a person asked for from the files panel, or an embedded file that is not a PDF — is a
/// file to write, which [`may_write_extracted`] decides. One test for the three windows, because
/// the day one of them opened what another wrote the three would disagree about the same URI.
#[must_use]
pub fn opens_as_document(asked: Extraction, bytes: &[u8]) -> bool {
    matches!(asked, Extraction::Fragment) && bytes.starts_with(b"%PDF-")
}

/// What a window says as it opens the embedded file §O.2.1's `ef` named, beside the one holding it.
#[must_use]
pub fn opening_embedded(name: &str, fragment: Option<&str>) -> String {
    match fragment {
        Some(rest) => format!("opening the embedded file {name:?} at `{rest}` (§O.2.1)"),
        None => format!("opening the embedded file {name:?} (§O.2.1)"),
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
/// The annex attaches that to the one parameter whose effect is a *file* and to no other, and
/// §O.2.1 says why it is different from a click: its identifiers are "useful primarily when
/// referring to them from external to the PDF such as a web page or web API", so the sentence that
/// named the file is frequently not the reader's. [`Extraction`] is `viewer-core` saying which of
/// the two happened; this is the one place the three hosts decide what to do about it.
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
