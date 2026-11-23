use std::cell::RefCell;
use std::fs::File;
use std::io::Read as _;
use std::path::{Path, PathBuf};

use super::{
    Handed, Handing, MEDIA_FILES, MediaFiles, Opener, Portal, PortalCalls, SWEPT_AFTER,
    file_name_for, gvariant_string, hand_media, handing, handing_without_helper, is_a_sound,
    media_files, sweep,
};

/// A directory of the test's own, removed afterwards.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("viewer-host-media-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("a scratch directory");
        Self(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// The opener a test stands in for the portal: it records what it was given, reading a file's
/// bytes through the descriptor it was handed, as the portal's player would.
#[derive(Default)]
struct Recorded {
    files: RefCell<Vec<(PathBuf, Vec<u8>)>>,
    uris: RefCell<Vec<String>>,
}

impl Opener for Recorded {
    fn open_file(&self, mut file: File, path: &Path) -> Result<(), String> {
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .map_err(|error| error.to_string())?;
        self.files.borrow_mut().push((path.to_owned(), bytes));
        Ok(())
    }

    fn open_uri(&self, uri: &str) -> Result<(), String> {
        self.uris.borrow_mut().push(uri.to_owned());
        Ok(())
    }

    fn name(&self) -> String {
        "the recording opener".to_owned()
    }
}

fn embedded(bytes: &[u8]) -> viewer_core::Media {
    viewer_core::Media::Embedded {
        name: "intro.wav".to_owned(),
        bytes: bytes.to_vec(),
    }
}

const RIFF: &[u8] = b"RIFF\x24\x00\x00\x00WAVEfmt ";

#[test]
fn the_default_is_ask_and_the_word_reads_four_levels() {
    assert_eq!(MediaFiles::default(), MediaFiles::Ask);
    for level in MediaFiles::ALL {
        assert_eq!(media_files(level.as_str()), Ok(level));
    }
    let wrong = media_files("play").expect_err("not a level");
    assert!(
        wrong.contains(MEDIA_FILES) && wrong.contains("refuse, ask, warn, open"),
        "{wrong}"
    );
}

/// `CLAUDE.md`'s four levels, each answered as its name says (ADR 3201).
#[test]
fn each_level_answers_as_it_is_named() {
    let at = |level| handing("a.pdf", None, Some("audio/wav"), embedded(RIFF), level);
    assert!(matches!(
        at(MediaFiles::Refuse),
        Handing::Refuse(said) if said.contains("--media-files=refuse") && said.contains("§13.2.4.2")
    ));
    assert!(matches!(
        at(MediaFiles::Ask),
        Handing::Ask { question, handed: Handed::Written { .. } }
            if question.reasons.contains("intro.wav")
                && question.reasons.contains("audio/wav")
                && question.choice.contains("--media-files=open")
    ));
    assert!(matches!(
        at(MediaFiles::Warn),
        Handing::Hand { note: Some(note), .. } if note.contains("without asking")
    ));
    assert!(matches!(
        at(MediaFiles::Open),
        Handing::Hand { note: None, handed: Handed::Written { file_name, .. } }
            if file_name == "clip.wav"
    ));
}

/// **A window without its sound helper hands a sound on, at the reader's level, and says why**
/// (ADR 3417): the file goes where any window that plays nothing sends it — never `Handed::Here`,
/// which nothing would play — the sentence naming the missing helper rides beside the hand-over
/// and inside the question, and a refusal stays the refusal it was.
#[test]
fn a_window_without_its_helper_hands_a_sound_on_and_says_why() {
    const MISSING: &str = "the sound helper `quorra-sound` was not found next to /bin/x";
    let at = |level| {
        handing_without_helper(
            "a.pdf",
            None,
            Some("audio/wav"),
            embedded(RIFF),
            MISSING,
            level,
        )
    };
    assert!(matches!(
        at(MediaFiles::Open),
        Handing::Hand { handed: Handed::Written { file_name, bytes, .. }, note: Some(note) }
            if file_name == "clip.wav" && bytes == RIFF
                && note.contains(MISSING) && note.contains("this machine's player")
    ));
    assert!(matches!(
        at(MediaFiles::Warn),
        Handing::Hand { note: Some(note), .. }
            if note.contains(MISSING) && note.contains("without asking")
    ));
    assert!(matches!(
        at(MediaFiles::Ask),
        Handing::Ask { question, handed: Handed::Written { .. } }
            if question.reasons.contains("intro.wav") && question.reasons.contains(MISSING)
    ));
    assert!(matches!(
        at(MediaFiles::Refuse),
        Handing::Refuse(said) if said.contains("--media-files=refuse") && !said.contains(MISSING)
    ));
    // Control: the same clip with no helper missing carries no such sentence.
    assert!(matches!(
        handing(
            "a.pdf",
            None,
            Some("audio/wav"),
            embedded(RIFF),
            MediaFiles::Open
        ),
        Handing::Hand { note: None, .. }
    ));
}

/// A sound by its samples or its type, and nothing else: a movie handed on for want of a sound
/// helper would be a false sentence (ADR 3417).
#[test]
fn a_sound_is_known_by_its_samples_or_its_type() {
    assert!(is_a_sound(None, true));
    assert!(is_a_sound(Some("audio/mpeg"), false));
    assert!(is_a_sound(Some("AUDIO/X-AIFF"), false));
    assert!(is_a_sound(Some("audio/vnd.wave"), false));
    assert!(!is_a_sound(Some("video/mp4"), false));
    assert!(!is_a_sound(Some("aud"), false));
    assert!(!is_a_sound(None, false));
}

/// The level is the reader's alone: nothing a document states reaches it, so the most permissive
/// clip a document can hand over — an embedded file of a known type, which Table 286's `/TF` has
/// already let through by the time a host sees it — is still put to the person at the default,
/// and still refused at `refuse` (ADR 3201).
#[test]
fn a_document_cannot_raise_the_level() {
    let clips = [
        (Some("video/mp4"), embedded(b"\x00\x00\x00\x18ftypmp42")),
        (
            None,
            viewer_core::Media::Url("https://example.org/a.mp4".to_owned()),
        ),
    ];
    for (content_type, media) in clips {
        assert!(matches!(
            handing(
                "a.pdf",
                None,
                content_type,
                media.clone(),
                MediaFiles::default()
            ),
            Handing::Ask { .. }
        ));
        assert!(matches!(
            handing("a.pdf", None, content_type, media, MediaFiles::Refuse),
            Handing::Refuse(_)
        ));
    }
}

/// What is refused before the level is refused at `open` too: a clip that is a program, a file
/// outside the document's directory, a URL under a scheme no player is handed.
#[test]
fn what_no_level_lets_through() {
    let directory = Path::new("/documents");
    let refused = |media, expected: &str| match handing(
        "a.pdf",
        Some(directory),
        None,
        media,
        MediaFiles::Open,
    ) {
        Handing::Refuse(said) => assert!(said.contains(expected), "{said}"),
        other => panic!("refused at open: {other:?}"),
    };
    refused(embedded(b"\x7fELF\x02\x01"), "executable");
    refused(embedded(b"#!/bin/sh\n"), "script");
    refused(
        viewer_core::Media::File("../elsewhere/a.wav".to_owned()),
        "a.wav",
    );
    refused(viewer_core::Media::File("/etc/a.wav".to_owned()), "a.wav");
    refused(
        viewer_core::Media::Url("file:///etc/passwd".to_owned()),
        "scheme",
    );
    refused(
        viewer_core::Media::Url("mailto:a@example.org".to_owned()),
        "scheme",
    );
    // And a file with no document directory to resolve against names nothing.
    assert!(matches!(
        handing(
            "a.pdf",
            None,
            None,
            viewer_core::Media::File("a.wav".to_owned()),
            MediaFiles::Open
        ),
        Handing::Refuse(_)
    ));
    // A file beside the document is resolved there, and only there.
    assert!(matches!(
        handing(
            "a.pdf",
            Some(directory),
            None,
            viewer_core::Media::File("clips/a.wav".to_owned()),
            MediaFiles::Open
        ),
        Handing::Hand { handed: Handed::File { path, .. }, .. }
            if path == Path::new("/documents/clips/a.wav")
    ));
}

/// The temporary file's name is this program's, chosen by Table 285's `/CT`.
#[test]
fn the_file_is_named_by_its_type_and_never_by_the_document() {
    assert_eq!(file_name_for(Some("audio/vnd.wave; codec=1")), "clip.wav");
    assert_eq!(file_name_for(Some("VIDEO/MP4")), "clip.mp4");
    assert_eq!(file_name_for(Some("application/x-desktop")), "clip");
    assert_eq!(file_name_for(None), "clip");
    let Handing::Hand {
        handed: Handed::Written { file_name, .. },
        ..
    } = handing(
        "a.pdf",
        None,
        Some("application/octet-stream"),
        viewer_core::Media::Embedded {
            name: "start.desktop".to_owned(),
            bytes: b"[Desktop Entry]".to_vec(),
        },
        MediaFiles::Open,
    )
    else {
        panic!("an embedded file of no type this table names is handed over")
    };
    assert_eq!(file_name, "clip");
}

/// The hand-off writes the clip's own bytes to a file of its own, readable by its owner alone, and
/// hands the opener a descriptor that reads them back.
#[test]
fn the_written_file_carries_the_clips_bytes_and_type() {
    let scratch = Scratch::new("written");
    let opener = Recorded::default();
    let Handing::Hand { handed, .. } = handing(
        "a.pdf",
        None,
        Some("audio/wav"),
        embedded(RIFF),
        MediaFiles::Open,
    ) else {
        panic!("open hands it over")
    };
    let said = hand_media("a.pdf", &handed, &scratch.0, &opener).expect("handed over");
    assert!(said.contains("the recording opener"), "{said}");
    let files = opener.files.borrow();
    let [(path, bytes)] = files.as_slice() else {
        panic!("one file handed over: {files:?}")
    };
    assert_eq!(bytes.as_slice(), RIFF);
    assert_eq!(
        path.file_name().and_then(|name| name.to_str()),
        Some("clip.wav")
    );
    assert_eq!(
        std::fs::read(path).expect("the file stays for the player"),
        RIFF
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let parent = path.parent().expect("a directory of its own");
        let mode = std::fs::metadata(parent)
            .expect("stated")
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o700, "{mode:o}");
    }
}

/// A file beside the document is handed over as it is, and one that is a program is not.
#[test]
fn a_file_beside_the_document_is_handed_over_unless_it_is_a_program() {
    let scratch = Scratch::new("beside");
    std::fs::write(scratch.0.join("a.wav"), RIFF).expect("written");
    std::fs::write(scratch.0.join("run.sh"), b"echo").expect("written");
    let opener = Recorded::default();
    let hand = |name: &str| {
        let Handing::Hand { handed, .. } = handing(
            "a.pdf",
            Some(&scratch.0),
            None,
            viewer_core::Media::File(name.to_owned()),
            MediaFiles::Open,
        ) else {
            panic!("open hands it over")
        };
        hand_media("a.pdf", &handed, &scratch.0, &opener)
    };
    hand("a.wav").expect("handed over");
    assert_eq!(opener.files.borrow().len(), 1);
    let refused = hand("run.sh").expect_err("a program");
    assert!(refused.contains("program"), "{refused}");
    assert_eq!(
        opener.files.borrow().len(),
        1,
        "nothing more was handed over"
    );
    let missing = hand("gone.wav").expect_err("no such file");
    assert!(missing.contains("cannot read"), "{missing}");
}

#[test]
fn a_url_goes_to_the_openers_uri_call() {
    let opener = Recorded::default();
    let Handing::Hand { handed, .. } = handing(
        "a.pdf",
        None,
        None,
        viewer_core::Media::Url("https://example.org/a.mp4".to_owned()),
        MediaFiles::Open,
    ) else {
        panic!("open hands it over")
    };
    hand_media("a.pdf", &handed, Path::new("/nowhere"), &opener).expect("handed over");
    assert_eq!(*opener.uris.borrow(), ["https://example.org/a.mp4"]);
    assert!(opener.files.borrow().is_empty());
}

#[test]
fn a_string_is_quoted_for_gvariant() {
    assert_eq!(gvariant_string("https://a/b"), "'https://a/b'");
    assert_eq!(gvariant_string("it's \\"), r"'it\'s \\'");
}

/// The portal's call as `gdbus` is given it: the interface, the method, the parent window, the
/// file as descriptor 0 and an empty options dictionary.
#[test]
fn the_portal_call_names_the_interface_and_the_descriptor() {
    let command = Portal::default().command("OpenFile", "@h 0");
    assert_eq!(command.get_program(), "gdbus");
    let arguments: Vec<_> = command
        .get_args()
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        arguments,
        [
            "call",
            "--session",
            "--dest",
            "org.freedesktop.portal.Desktop",
            "--object-path",
            "/org/freedesktop/portal/desktop",
            "--method",
            "org.freedesktop.portal.OpenURI.OpenFile",
            "''",
            "@h 0",
            "@a{sv} {}",
        ]
    );
}

/// The portal's call made, with a double in `gdbus`'s place that records its arguments and reads
/// the descriptor it was given: the bytes that arrive are the clip's.
#[cfg(unix)]
#[test]
fn the_portal_is_called_with_the_file_as_its_descriptor() {
    use std::os::unix::fs::PermissionsExt as _;
    let scratch = Scratch::new("portal");
    let double = scratch.0.join("gdbus");
    std::fs::write(
        &double,
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$0.arguments\"\ncat > \"$0.part\" && mv \"$0.part\" \"$0.read\"\n",
    )
    .expect("the double is written");
    std::fs::set_permissions(&double, std::fs::Permissions::from_mode(0o755)).expect("its mode");
    let portal = Portal {
        program: double.clone(),
        told: None,
    };
    let handed = Handed::Written {
        name: "intro.wav".to_owned(),
        file_name: "clip.wav".to_owned(),
        bytes: RIFF.to_vec(),
    };
    hand_media("a.pdf", &handed, &scratch.0, &portal).expect("the portal was called");
    let read = scratch.0.join("gdbus.read");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    while !read.exists() {
        assert!(
            std::time::Instant::now() < deadline,
            "the double was never called"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert_eq!(std::fs::read(&read).expect("read"), RIFF);
    let arguments = std::fs::read_to_string(scratch.0.join("gdbus.arguments")).expect("read");
    assert!(
        arguments.contains("org.freedesktop.portal.OpenURI.OpenFile\n''\n@h 0\n"),
        "{arguments}"
    );
}

/// A directory under `scratch` named `name`, holding one file, as a hand-off leaves one.
fn left_behind(scratch: &Scratch, name: &str) -> PathBuf {
    let directory = scratch.0.join(name);
    std::fs::create_dir(&directory).expect("a directory");
    std::fs::write(directory.join("clip.wav"), RIFF).expect("a clip");
    directory
}

/// The sweep removes this program's directories older than [`SWEPT_AFTER`] and nothing else: not
/// a younger one, not the one just written, not a name of another form, not a plain file and not
/// a symbolic link to a directory (ADR 3218).
#[test]
fn the_sweep_removes_only_this_programs_old_directories() {
    let scratch = Scratch::new("sweep");
    let old = left_behind(&scratch, "quorra-media-1-2-0");
    let ours = left_behind(&scratch, "quorra-media-3-4-0");
    let other = left_behind(&scratch, "another-program-1");
    std::fs::write(scratch.0.join("quorra-media-file"), b"x").expect("a plain file");
    let outside = Scratch::new("sweep-outside");
    std::fs::write(outside.0.join("kept"), b"x").expect("a file outside");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&outside.0, scratch.0.join("quorra-media-link"))
        .expect("a symbolic link");
    let now = std::time::SystemTime::now();
    assert_eq!(sweep(&scratch.0, &ours, now), Vec::<String>::new());
    assert!(old.exists(), "a directory younger than the bound is kept");
    let later = now
        .checked_add(SWEPT_AFTER)
        .and_then(|bound| bound.checked_add(std::time::Duration::from_secs(60)))
        .expect("a day from now is a time");
    assert_eq!(sweep(&scratch.0, &ours, later), Vec::<String>::new());
    assert!(!old.exists(), "a directory older than the bound is removed");
    assert!(ours.exists(), "the directory just written is kept");
    assert!(other.exists(), "a name of another form is kept");
    assert!(scratch.0.join("quorra-media-file").exists());
    assert!(outside.0.join("kept").exists(), "a link is not followed");
}

/// A hand-off sweeps: a directory left by an earlier one and made older than the bound is gone
/// once the next clip has been written.
#[cfg(unix)]
#[test]
fn the_next_hand_off_sweeps_an_old_directory() {
    let scratch = Scratch::new("next-hand-off");
    let old = left_behind(&scratch, "quorra-media-5-6-0");
    let long_ago = std::time::SystemTime::now()
        .checked_sub(SWEPT_AFTER)
        .and_then(|bound| bound.checked_sub(std::time::Duration::from_secs(60)))
        .expect("a day ago is a time");
    File::open(&old)
        .and_then(|directory| directory.set_modified(long_ago))
        .expect("the directory's time is set back");
    let handed = Handed::Written {
        name: "intro.wav".to_owned(),
        file_name: "clip.wav".to_owned(),
        bytes: RIFF.to_vec(),
    };
    let opener = Recorded::default();
    hand_media("a.pdf", &handed, &scratch.0, &opener).expect("handed over");
    assert!(!old.exists(), "the old directory was swept");
    let written = &opener.files.borrow()[0].0;
    assert!(written.exists(), "the new clip is kept for its player");
}

/// A double standing in for `gdbus`, answering with `status` and saying `stderr`.
#[cfg(unix)]
fn double(scratch: &Scratch, status: u8, stderr: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt as _;
    let double = scratch.0.join("gdbus");
    std::fs::write(
        &double,
        format!("#!/bin/sh\ncat > /dev/null\necho '{stderr}' >&2\nexit {status}\n"),
    )
    .expect("the double is written");
    std::fs::set_permissions(&double, std::fs::Permissions::from_mode(0o755)).expect("its mode");
    double
}

/// Looks for the calls' answers as a window's timer does, until none is out.
fn answered(calls: &mut PortalCalls) -> Vec<String> {
    let started = std::time::Instant::now();
    let mut said = Vec::new();
    while calls.interval().is_some() {
        assert!(
            started.elapsed() < std::time::Duration::from_secs(30),
            "the call never answered"
        );
        said.extend(calls.collect());
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    said.extend(calls.collect());
    said
}

/// A portal that fails after its call has started is said to the window whose calls it was, with
/// the portal's own words, and a call that succeeds says nothing more (ADR 3219).
#[cfg(unix)]
#[test]
fn a_portal_failure_after_the_call_reaches_the_window() {
    let handed = Handed::Url("https://example.org/clip.mp4".to_owned());
    let failing = Scratch::new("portal-fails");
    let mut calls = PortalCalls::new();
    assert_eq!(calls.interval(), None);
    let portal = calls.portal_through(double(&failing, 1, "no such portal"));
    hand_media("a.pdf", &handed, &failing.0, &portal).expect("the call was started");
    let said = answered(&mut calls);
    assert_eq!(said.len(), 1, "{said:?}");
    assert!(
        said[0].contains("https://example.org/clip.mp4")
            && said[0].contains("did not open it")
            && said[0].contains("no such portal"),
        "{said:?}"
    );
    let taking = Scratch::new("portal-takes");
    let portal = calls.portal_through(double(&taking, 0, ""));
    hand_media("a.pdf", &handed, &taking.0, &portal).expect("the call was started");
    assert_eq!(answered(&mut calls), Vec::<String>::new());
}
