//! ISO 32000-2 clause 13's media file handed to this machine's own player, at the level the reader
//! set (ADR 3201).
//!
//! `CLAUDE.md`'s clause 13 entry keeps playing video out of this program, admits audio decoded and
//! presented by it (`doc/questions/A946`), and admits "the media file saved or handed to this
//! machine's own player (`ask` by default, under *A document's restrictions are the reader's to
//! set*)". A window that plays §13.3's sounds itself asks the same level through
//! [`handing_here`] (ADR 3301). `viewer_core` has already done the
//! document's half by the time a host sees [`viewer_core::Event::Media`]: §13.2.2's viability
//! judged the rendition for the one player this program has, Table 286's `/TF` was asked at the
//! restrictions level, and an embedded clip's stream was decoded. What is left is this machine's
//! half — whether another program is started on a file a *document* supplied — and that is asked
//! here, once, at [`MediaFiles`]' four levels (`doc/questions/A254`, RFC 0009 section 11 question 3,
//! RFC 0015 section 3).
//!
//! **Three things are decided before the level, and refused at every level including `open`**,
//! because a reader who set `open` said that a player may be started on a clip and nothing more:
//!
//! - an embedded clip whose bytes begin as an executable or a script does is not handed to any
//!   handler — `launched`'s test, one act over (ADR 2171);
//! - a file outside the document is resolved by `resolve_import`'s two rules and by nothing
//!   looser, so a name outside the document's own directory is never followed (ADRs 1155, 1227);
//! - a URL is handed over only under a scheme [`MEDIA_SCHEMES`] names, so a document does not get
//!   to choose which of this machine's handlers runs (ADR 1155's reason).
//!
//! **The opener is the XDG desktop portal** (ADR 3202): `org.freedesktop.portal.OpenURI`'s
//! `OpenFile` for a file, given the file as a descriptor so that the call is the same inside a
//! Flatpak as outside one, and its `OpenURI` for a URL. The unconfined host makes the call; the
//! renderer's system-call set is not touched. [`Opener`] is the seam a test double stands in.

use std::fs::File;
use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use super::resolve_import;

mod calls;
mod sweep;

pub use calls::{PortalCalls, Told};
pub use sweep::{MEDIA_DIRECTORY_PREFIX, SWEPT_AFTER, sweep};

/// The word a person types to say what this reader does with a media file a document hands to a
/// player.
///
/// [`super::LAUNCHED_DOCUMENTS`]' four words and direction — `refuse`, `ask`, `warn`, `open` —
/// because the act is the same kind: a file a document supplied, given to a program this machine
/// chooses. **A value of its own** for ADR 1227's reason one act along: a reader who set
/// `--launched-documents=open` said what becomes of a launch action's file, and has said nothing
/// about a clip a screen annotation plays (ADR 3201).
pub const MEDIA_FILES: &str = "--media-files=";

/// The schemes a media clip's URL may be handed to this machine's opener under.
///
/// §7.11.5 lets a file specification be a URL and §13.2.4.2's clip data be one, and a player is
/// handed it as it stands. [`super::LINK_SCHEMES`]' reasoning, narrowed by one: `mailto` names no
/// media (ADR 3201).
pub const MEDIA_SCHEMES: [&str; 2] = ["http", "https"];

/// What this reader does when a document hands a media file to this machine's player.
///
/// `CLAUDE.md` principle 3's four levels over the one act [`handing`] decides. Named for what is
/// handed rather than `Media`, so that it is never read as `viewer_core::Media`, the clip itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MediaFiles {
    /// Hand nothing over, and say what the document asked to play.
    Refuse,
    /// Put the clip to the person first, and hand it over — or play it, in a window that plays
    /// it ([`handing_here`]) — on a `yes`.
    ///
    /// The default, as `CLAUDE.md`'s clause 13 entry states it and A254 settled it: a document
    /// never starts a program on this machine by itself, and a reader is not refused the act the
    /// clause describes.
    #[default]
    Ask,
    /// Hand it over or play it, and say afterwards that it was done without asking.
    Warn,
    /// Hand it over or play it.
    Open,
}

impl MediaFiles {
    /// All four, in the order a person reads them: least permissive to most.
    pub const ALL: [Self; 4] = [Self::Refuse, Self::Ask, Self::Warn, Self::Open];

    /// The word [`MEDIA_FILES`] takes for this level.
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

/// Reads [`MEDIA_FILES`]' word onto a level, or says what is wrong with it.
///
/// # Errors
///
/// The sentence to print, naming every word this option takes — `links`' shape.
pub fn media_files(word: &str) -> Result<MediaFiles, String> {
    MediaFiles::parse(word).ok_or_else(|| {
        format!(
            "{MEDIA_FILES}{word}: no such level. One of {}",
            MediaFiles::ALL.map(MediaFiles::as_str).join(", ")
        )
    })
}

/// What would be given to this machine's opener, once the checks before the level have passed.
#[derive(Debug, Clone, PartialEq)]
pub enum Handed {
    /// An embedded clip, or a sound's samples written as RIFF WAVE, to be written to a private
    /// temporary file of this program's own naming.
    Written {
        /// What the document called it, for the sentences.
        name: String,
        /// The file's name in the temporary directory: [`file_name_for`]'s, never the document's.
        file_name: String,
        /// The file.
        bytes: Vec<u8>,
    },
    /// A file beside the document, resolved by `resolve_import`.
    File {
        /// What the document called it.
        name: String,
        /// Where it resolved to, which is what would be handed over.
        path: PathBuf,
    },
    /// §7.11.5's URL, under one of [`MEDIA_SCHEMES`].
    Url(String),
    /// A sound object's own samples, played in this window rather than handed to anything
    /// ([`handing_here`], ADR 3301).
    Here {
        /// What the document called it.
        name: String,
        /// Table 305's entries and the samples as §13.3 stores them.
        sound: viewer_core::SoundData,
    },
}

impl Handed {
    /// What is handed over, in a phrase for a sentence.
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::Written { name, bytes, .. } => format!("{name:?}, {} bytes", bytes.len()),
            Self::File { name, path } => format!("the file {name:?} ({})", path.display()),
            Self::Url(url) => url.clone(),
            Self::Here { name, sound } => {
                format!("{name:?}, {} bytes of samples", sound.data.len())
            }
        }
    }
}

/// What a host does about one [`viewer_core::Event::Media`], under the level the reader set.
///
/// [`super::Launched`]'s arms for its reason: four levels answer in three ways once *warn* is a
/// hand-over with a sentence, and a host matching fewer would have a level that silently behaved
/// like another.
#[derive(Debug, Clone, PartialEq)]
pub enum Handing {
    /// Hand it to [`hand_media`] now, then say the sentence it returns, with `note` after it.
    Hand {
        /// What is handed over.
        handed: Handed,
        /// [`MediaFiles::Warn`]'s sentence, said after the hand-over.
        note: Option<String>,
    },
    /// Put this question to the person, and hand it over on a `yes`.
    Ask {
        /// What would be handed over.
        handed: Handed,
        /// The words, from [`asked_to_play`].
        question: crate::restriction::Question,
    },
    /// Do not hand it over. The sentence says why.
    Refuse(String),
}

/// What becomes of the media file a document handed this window, under `level`.
///
/// `document` is the document's label for the sentences, `directory` the directory the document
/// was opened from — `resolve_import`'s first argument, `None` where it came from no file — and
/// `content_type` Table 285's `/CT`, which chooses the temporary file's name.
#[must_use]
pub fn handing(
    document: &str,
    directory: Option<&Path>,
    content_type: Option<&str>,
    media: viewer_core::Media,
    level: MediaFiles,
) -> Handing {
    let handed = match media {
        viewer_core::Media::Embedded { name, bytes } => {
            if begins_as_a_program(&bytes) {
                return Handing::Refuse(declined(
                    document,
                    &name,
                    "its bytes begin as an executable or a script does, and this reader starts \
                     no program a document carries (ISO 32000-2 §12.6.4.6's rule, applied here)",
                ));
            }
            Handed::Written {
                file_name: file_name_for(content_type),
                name,
                bytes,
            }
        }
        viewer_core::Media::File(name) => match resolve_import(directory, &name) {
            Ok(path) => Handed::File { name, path },
            Err(refusal) => {
                return Handing::Refuse(declined(document, &name, &refusal.to_string()));
            }
        },
        viewer_core::Media::Url(url) => {
            if !super::scheme_of(&url)
                .is_some_and(|scheme| MEDIA_SCHEMES.contains(&scheme.as_str()))
            {
                return Handing::Refuse(declined(
                    document,
                    &url,
                    &format!(
                        "it names no scheme this reader hands a player ({}), and a document does \
                         not get to choose which of this machine's handlers runs",
                        MEDIA_SCHEMES.join(", ")
                    ),
                ));
            }
            Handed::Url(url)
        }
    };
    leveled(document, content_type, handed, level)
}

/// What becomes of a media clip in a window that plays §13.3's sounds itself (ADR 3301).
///
/// A clip carrying a sound object's own samples is **played here, under the same four levels**:
/// the level is the reader's one answer to whether a document's media is played at all, and
/// playing it in this window starts no program on this machine, so the route needs no switch of
/// its own (RFC 0015 section 4.2). Nothing is checked before the level, because nothing is handed
/// to a program and no file or URL is followed: the samples go to the confined media worker. Every
/// other clip — a file, a URL, an embedded clip in a format this program does not decode — is
/// [`handing`]'s.
#[must_use]
pub fn handing_here(
    document: &str,
    directory: Option<&Path>,
    content_type: Option<&str>,
    media: viewer_core::Media,
    sound: Option<viewer_core::SoundData>,
    level: MediaFiles,
) -> Handing {
    let Some(sound) = sound else {
        return handing(document, directory, content_type, media, level);
    };
    let name = match media {
        viewer_core::Media::Embedded { name, .. }
        | viewer_core::Media::Url(name)
        | viewer_core::Media::File(name) => name,
    };
    leveled(document, content_type, Handed::Here { name, sound }, level)
}

/// [`MediaFiles`]' four answers over what would be handed over or played.
fn leveled(
    document: &str,
    content_type: Option<&str>,
    handed: Handed,
    level: MediaFiles,
) -> Handing {
    if matches!(handed, Handed::Here { .. }) {
        return match level {
            MediaFiles::Refuse => Handing::Refuse(not_played(
                document,
                &handed.describe(),
                &format!(
                    "this reader is set to play no media a document asks for ({MEDIA_FILES}{}); \
                     {MEDIA_FILES}{} puts it to you first",
                    MediaFiles::Refuse.as_str(),
                    MediaFiles::Ask.as_str()
                ),
            )),
            MediaFiles::Ask => Handing::Ask {
                question: asked_to_play(document, content_type, &handed),
                handed,
            },
            MediaFiles::Warn => Handing::Hand {
                handed,
                note: Some(format!(
                    "it was played without asking you first, because this reader is set to \
                     {MEDIA_FILES}{}",
                    MediaFiles::Warn.as_str()
                )),
            },
            MediaFiles::Open => Handing::Hand { handed, note: None },
        };
    }
    match level {
        MediaFiles::Refuse => Handing::Refuse(declined(
            document,
            &handed.describe(),
            &format!(
                "this reader is set to hand no media file to this machine's player \
                 ({MEDIA_FILES}{}); {MEDIA_FILES}{} puts it to you first",
                MediaFiles::Refuse.as_str(),
                MediaFiles::Ask.as_str()
            ),
        )),
        MediaFiles::Ask => Handing::Ask {
            question: asked_to_play(document, content_type, &handed),
            handed,
        },
        MediaFiles::Warn => Handing::Hand {
            handed,
            note: Some(format!(
                "it was handed over without asking you first, because this reader is set to \
                 {MEDIA_FILES}{}",
                MediaFiles::Warn.as_str()
            )),
        },
        MediaFiles::Open => Handing::Hand { handed, note: None },
    }
}

/// The sentence a window says where a sound is not played here.
#[must_use]
pub fn not_played(document: &str, what: &str, why: &str) -> String {
    format!("{document} plays a sound — {what} (ISO 32000-2 §13.3); not played: {why}")
}

/// The sentence a window says where a clip is not handed over.
#[must_use]
pub fn declined(document: &str, what: &str, why: &str) -> String {
    format!(
        "{document} hands a media clip to this machine's player — {what} (ISO 32000-2 §13.2.4.2, \
         §13.3, §13.4); declined: {why}"
    )
}

/// What a window puts in front of a person at [`MediaFiles::Ask`].
///
/// [`super::asked_to_open`]'s two-string shape and for its reason: no window words a question for
/// itself.
#[must_use]
pub fn asked_to_play(
    document: &str,
    content_type: Option<&str>,
    handed: &Handed,
) -> crate::restriction::Question {
    let kind = content_type.map_or_else(String::new, |kind| format!(" ({kind})"));
    let how = match handed {
        Handed::Written { .. } => {
            "This reader would write it to a private temporary file and hand that file to this \
             machine's opener (the desktop portal), which starts the player this machine opens \
             such a file with."
        }
        Handed::File { .. } => {
            "This reader would hand that file to this machine's opener (the desktop portal), \
             which starts the player this machine opens such a file with."
        }
        Handed::Url(_) => {
            "This reader would hand that URL to this machine's opener (the desktop portal), which \
             starts the program this machine opens such a URL with."
        }
        Handed::Here { .. } => {
            return crate::restriction::Question {
                reasons: format!(
                    "{document} asks to play a sound{kind}: {}. This reader would play it in this \
                     window, its samples decoded in a confined worker of its own and converted \
                     for this machine's audio device (ISO 32000-2 §13.3); no other program is \
                     started.",
                    handed.describe()
                ),
                choice: format!(
                    "You have set this reader to ask before a document's media is played \
                     ({MEDIA_FILES}{}). \"{}\" plays this one and leaves the level where it is; \
                     \"{}\" keeps it silent. {MEDIA_FILES}{} stops the question being asked, and \
                     {MEDIA_FILES}{} stops media being played at all.",
                    MediaFiles::Ask.as_str(),
                    crate::restriction::GO_AHEAD,
                    crate::restriction::DO_NOT,
                    MediaFiles::Open.as_str(),
                    MediaFiles::Refuse.as_str()
                ),
            };
        }
    };
    crate::restriction::Question {
        reasons: format!(
            "{document} asks to play a media clip{kind}: {}. This reader plays no audio or video \
             itself (ISO 32000-2 §13.2.4.2, §13.3, §13.4). {how}",
            handed.describe()
        ),
        choice: format!(
            "You have set this reader to ask before handing a media file to a player \
             ({MEDIA_FILES}{}). \"{}\" hands this one over and leaves the level where it is; \"{}\" \
             keeps it here. {MEDIA_FILES}{} stops the question being asked, and {MEDIA_FILES}{} \
             stops media files being handed over at all.",
            MediaFiles::Ask.as_str(),
            crate::restriction::GO_AHEAD,
            crate::restriction::DO_NOT,
            MediaFiles::Open.as_str(),
            MediaFiles::Refuse.as_str()
        ),
    }
}

/// The sentence a window says where a person answered [`crate::restriction::DO_NOT`].
#[must_use]
pub fn media_declined(document: &str, handed: &Handed) -> String {
    if matches!(handed, Handed::Here { .. }) {
        return not_played(
            document,
            &handed.describe(),
            &format!("you answered \"{}\"", crate::restriction::DO_NOT),
        );
    }
    declined(
        document,
        &handed.describe(),
        &format!("you answered \"{}\"", crate::restriction::DO_NOT),
    )
}

/// What starts a player on this machine: the seam between the decision and the desktop.
///
/// [`Portal`] is the one a window holds; a test stands a double in its place.
pub trait Opener {
    /// Hands an open file to this machine's opener. `path` is where it is, for the sentences.
    ///
    /// # Errors
    ///
    /// The sentence to say where the opener could not be reached.
    fn open_file(&self, file: File, path: &Path) -> Result<(), String>;

    /// Hands a URL to this machine's opener.
    ///
    /// # Errors
    ///
    /// The sentence to say where the opener could not be reached.
    fn open_uri(&self, uri: &str) -> Result<(), String>;

    /// The opener's name in a sentence.
    fn name(&self) -> String;
}

/// The XDG desktop portal's `org.freedesktop.portal.OpenURI`, called through `gdbus` (ADR 3202).
///
/// **The portal rather than `xdg-open`, and `gdbus` rather than a D-Bus library.** `OpenFile`
/// takes the file as a descriptor, so the same call works in a Flatpak, where no path of this
/// program's is visible to the player, and outside one; `xdg-open` is a script that chooses a
/// different route per desktop. `gdbus` ships with `GLib` on every desktop the portal runs on and
/// passes a descriptor of its own as a `h` argument, which keeps an asynchronous D-Bus runtime out
/// of every window for one call — `CLAUDE.md`'s "no heavy runtime" — and keeps the call a child
/// process, as [`super::URI_HANDLER`]'s already is. The file goes in as `gdbus`'s standard input,
/// descriptor 0, so no descriptor table is arranged by hand.
///
/// A failure after the spawn reaches the window through `told` where a window gave one
/// ([`PortalCalls::portal`]), and the terminal where none did (ADR 3219).
#[derive(Debug, Clone)]
pub struct Portal {
    /// The program that makes the call: `gdbus`, or a double in a test.
    pub program: PathBuf,
    /// Where the waiting thread says what became of the call.
    pub told: Option<Told>,
}

/// The portal's bus name, object path and interface, as the XDG desktop portal documents them.
const PORTAL_DESTINATION: &str = "org.freedesktop.portal.Desktop";
const PORTAL_PATH: &str = "/org/freedesktop/portal/desktop";
const PORTAL_INTERFACE: &str = "org.freedesktop.portal.OpenURI";

impl Default for Portal {
    fn default() -> Self {
        Self {
            program: PathBuf::from("gdbus"),
            told: None,
        }
    }
}

impl Portal {
    /// The call for `method` with `arguments` after the parent window's empty handle and before the
    /// empty options dictionary — `OpenFile(s, h, a{sv})` and `OpenURI(s, s, a{sv})`.
    #[must_use]
    pub fn command(&self, method: &str, argument: &str) -> Command {
        let mut command = Command::new(&self.program);
        command
            .args(["call", "--session", "--dest", PORTAL_DESTINATION])
            .args(["--object-path", PORTAL_PATH])
            .arg("--method")
            .arg(format!("{PORTAL_INTERFACE}.{method}"))
            .args(["''", argument, "@a{sv} {}"])
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        command
    }

    fn call(&self, mut command: Command, handled: String) -> Result<(), String> {
        // Windows has no desktop portal. The `WinUI` window opens files itself through the C ABI's
        // window (ADR 3217), so this is reached on Windows only by a toolkit with no opener, and
        // there the hand-off is declined by name rather than attempted (ADR 3201).
        if cfg!(windows) {
            return Err(
                "this window has no opener on Windows, so nothing was handed to a player"
                    .to_owned(),
            );
        }
        let child = command
            .spawn()
            .map_err(|error| format!("the desktop portal could not be called: {error}"))?;
        // The reply is the portal's request handle and arrives at once; the player starts after
        // it. Waited for on a thread of its own so that a portal that is slow to activate holds
        // no window — `start_handler`'s shape — and a failure is said in the window that `told`
        // belongs to, or on the terminal where no window gave one (ADR 3219).
        let told = self.told.clone();
        if let Some(told) = &told {
            told.waiting();
        }
        let name = self.name();
        let waited = std::thread::Builder::new()
            .name("media-portal".to_owned())
            .spawn(move || {
                let failure = match child.wait_with_output() {
                    Ok(output) if output.status.success() => None,
                    Ok(output) => Some(not_opened(
                        &name,
                        &handled,
                        &format!(
                            "{} {}",
                            output.status,
                            String::from_utf8_lossy(&output.stderr).trim()
                        ),
                    )),
                    Err(error) => Some(not_opened(
                        &name,
                        &handled,
                        &format!("its call could not be waited for: {error}"),
                    )),
                };
                match told {
                    Some(told) => told.finished(failure),
                    None => {
                        if let Some(failure) = failure {
                            eprintln!("note: {failure}");
                        }
                    }
                }
            });
        if let Err(error) = waited {
            if let Some(told) = &self.told {
                told.not_waiting();
            }
            return Err(format!(
                "no thread to wait for the desktop portal on: {error}"
            ));
        }
        Ok(())
    }
}

impl Opener for Portal {
    fn open_file(&self, file: File, path: &Path) -> Result<(), String> {
        let mut command = self.command("OpenFile", "@h 0");
        command.stdin(Stdio::from(file));
        self.call(command, path.display().to_string())
    }

    fn open_uri(&self, uri: &str) -> Result<(), String> {
        let mut command = self.command("OpenURI", &gvariant_string(uri));
        command.stdin(Stdio::null());
        self.call(command, uri.to_owned())
    }

    fn name(&self) -> String {
        format!("the desktop portal ({PORTAL_INTERFACE})")
    }
}

/// `text` in `GVariant`'s text format, so that `gdbus` reads it as one string whatever it holds.
///
/// `GLib`'s text format quotes a string in single quotes and escapes a quote or a backslash inside
/// it with a backslash.
#[must_use]
pub fn gvariant_string(text: &str) -> String {
    let mut quoted = String::with_capacity(text.len().saturating_add(2));
    quoted.push('\'');
    for character in text.chars() {
        if matches!(character, '\'' | '\\') {
            quoted.push('\\');
        }
        quoted.push(character);
    }
    quoted.push('\'');
    quoted
}

/// Carries [`Handing::Hand`] or a `yes` out: writes, opens or names the clip and gives it to
/// `opener`, answering the sentence a window says.
///
/// `directory` is where an embedded clip is written — [`media_directory`] in a window, a test's own
/// directory in a test.
///
/// # Errors
///
/// The sentence to say where nothing was handed over: the temporary file could not be written, a
/// file beside the document could not be opened or turned out to be a program, or the opener
/// could not be reached.
pub fn hand_media(
    document: &str,
    handed: &Handed,
    directory: &Path,
    opener: &dyn Opener,
) -> Result<String, String> {
    let not_handed = |why: String| declined(document, &handed.describe(), &why);
    let mut unswept = Vec::new();
    match handed {
        Handed::Written {
            file_name, bytes, ..
        } => {
            let path = write_private(directory, file_name, bytes).map_err(not_handed)?;
            if let Some(ours) = path.parent() {
                unswept = sweep(directory, ours, std::time::SystemTime::now());
            }
            let file = File::open(&path)
                .map_err(|error| not_handed(format!("cannot read {}: {error}", path.display())))?;
            opener.open_file(file, &path).map_err(not_handed)?;
        }
        Handed::File { path, .. } => {
            let mut file = File::open(path)
                .map_err(|error| not_handed(format!("cannot read {}: {error}", path.display())))?;
            let mut start = [0_u8; 8];
            let read = file
                .read(&mut start)
                .map_err(|error| not_handed(format!("cannot read {}: {error}", path.display())))?;
            if let Some(why) = super::launched::program(path, start.get(..read).unwrap_or(&[])) {
                return Err(not_handed(format!(
                    "it is a program — {why} — and this reader starts no program a document names"
                )));
            }
            // A fresh descriptor at offset zero rather than the one just read from: the player
            // reads the file from its start.
            let file = File::open(path)
                .map_err(|error| not_handed(format!("cannot read {}: {error}", path.display())))?;
            opener.open_file(file, path).map_err(not_handed)?;
        }
        Handed::Url(url) => opener.open_uri(url).map_err(not_handed)?,
        // `handing_here`'s route: a window that plays the sound hands it to its player instead,
        // and one that does not was never given this (ADR 3301).
        Handed::Here { .. } => {
            return Err(not_handed(
                "it is a sound for a window that plays it here, and this one hands nothing over"
                    .to_owned(),
            ));
        }
    }
    let mut said = format!(
        "{document}: the media clip {} was handed to {}, which starts this machine's player \
         (ISO 32000-2 §13.2.4.2)",
        handed.describe(),
        opener.name()
    );
    if !unswept.is_empty() {
        said.push_str("; an older clip's directory was not removed: ");
        said.push_str(&unswept.join("; "));
    }
    Ok(said)
}

/// What is said where `opener` took `handled` and then answered that it did not open it — the
/// portal's call failing after its spawn, or the `WinUI` window's `Launcher` answering `false`
/// (ADRs 3217, 3219).
#[must_use]
pub fn not_opened(opener: &str, handled: &str, why: &str) -> String {
    format!("{handled} was handed to {opener}, which did not open it: {why}")
}

/// [`hand_media`] as a window carries it out: through [`Portal`], into [`media_directory`], with
/// [`MediaFiles::Warn`]'s `note` after a hand-over that happened. The answer is the sentence to say
/// either way.
#[must_use]
pub fn hand_over_media(document: &str, handed: &Handed, note: Option<&str>) -> String {
    hand_over_media_to(document, handed, note, &Portal::default())
}

/// [`hand_over_media`] through `opener`: a window's own [`PortalCalls::portal`], whose failure
/// after the call is said in that window, or a host's opener (ADRs 3217, 3219).
#[must_use]
pub fn hand_over_media_to(
    document: &str,
    handed: &Handed,
    note: Option<&str>,
    opener: &dyn Opener,
) -> String {
    match hand_media(document, handed, &media_directory(), opener) {
        Ok(said) => match note {
            Some(note) => format!("{said} — {note}"),
            None => said,
        },
        Err(why) => why,
    }
}

/// The directory a window writes an embedded clip into: `$XDG_RUNTIME_DIR`, which is the user's
/// own, on a memory file system and emptied when the user logs out, or the system's temporary
/// directory where there is none.
///
/// **The file is not removed after the hand-over, and that is the cost written down** (ADR 3202):
/// the player opens it after the portal has answered, at a moment this program is not told, so a
/// removal would race the player. Each file is created anew under its own directory, readable by
/// its owner alone.
#[must_use]
pub fn media_directory() -> PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .filter(|directory| directory.is_absolute())
        .unwrap_or_else(std::env::temp_dir)
}

/// Writes `bytes` as `file_name` in a directory of its own under `directory`, created anew and
/// readable by its owner alone, and answers the file's path.
fn write_private(directory: &Path, file_name: &str, bytes: &[u8]) -> Result<PathBuf, String> {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_nanos());
    let mut attempt: u32 = 0;
    let private = loop {
        let candidate = directory.join(format!(
            "{MEDIA_DIRECTORY_PREFIX}{}-{stamp}-{attempt}",
            std::process::id()
        ));
        // `create_dir` and never `create_dir_all`: a directory that already exists — another
        // process's, or one planted — is not written into.
        match new_private_directory(&candidate) {
            Ok(()) => break candidate,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists && attempt < 16 => {
                attempt = attempt.saturating_add(1);
            }
            Err(error) => {
                return Err(format!(
                    "no private directory for the clip under {}: {error}",
                    directory.display()
                ));
            }
        }
    };
    let path = private.join(file_name);
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
    file.write_all(bytes)
        .and_then(|()| file.flush())
        .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
    Ok(path)
}

#[cfg(unix)]
fn new_private_directory(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::DirBuilderExt as _;
    std::fs::DirBuilder::new().mode(0o700).create(path)
}

#[cfg(not(unix))]
fn new_private_directory(path: &Path) -> std::io::Result<()> {
    std::fs::DirBuilder::new().create(path)
}

/// Whether a clip's bytes begin as an executable or a script does — `launched`'s test.
fn begins_as_a_program(bytes: &[u8]) -> bool {
    super::launched::EXECUTABLE_STARTS
        .iter()
        .any(|start| bytes.starts_with(start))
}

/// The registered media types this reader names a temporary file for, and the ending each takes.
///
/// **The name is this program's, never the document's**, and that is a choice: an opener chooses
/// the player partly by a file's ending, and a document free to name the file would be choosing
/// the handler — `launched`'s `RUN_BY_HANDLERS` is the list of endings that run rather than show.
/// A type this table does not name gets a file without an ending, which the opener reads by its
/// content (ADR 3202).
pub const MEDIA_ENDINGS: [(&str, &str); 19] = [
    ("audio/aiff", "aiff"),
    ("audio/basic", "au"),
    ("audio/flac", "flac"),
    ("audio/midi", "mid"),
    ("audio/mp4", "m4a"),
    ("audio/mpeg", "mp3"),
    ("audio/ogg", "ogg"),
    ("audio/vnd.wave", "wav"),
    ("audio/wav", "wav"),
    ("audio/x-aiff", "aiff"),
    ("audio/x-wav", "wav"),
    ("video/avi", "avi"),
    ("video/mp4", "mp4"),
    ("video/mpeg", "mpg"),
    ("video/ogg", "ogv"),
    ("video/quicktime", "mov"),
    ("video/webm", "webm"),
    ("video/x-matroska", "mkv"),
    ("video/x-msvideo", "avi"),
];

/// The temporary file's name for a clip of `content_type`: `clip`, with [`MEDIA_ENDINGS`]' ending
/// where the type is one it names.
#[must_use]
pub fn file_name_for(content_type: Option<&str>) -> String {
    // RFC 2045 section 5.1: a type and subtype are matched case-insensitively, and parameters
    // follow `;`.
    let essence = content_type
        .and_then(|kind| kind.split(';').next())
        .map(str::trim);
    essence
        .and_then(|essence| {
            MEDIA_ENDINGS
                .iter()
                .find(|(kind, _)| essence.eq_ignore_ascii_case(kind))
        })
        .map_or_else(|| "clip".to_owned(), |(_, ending)| format!("clip.{ending}"))
}

#[cfg(test)]
mod tests;
