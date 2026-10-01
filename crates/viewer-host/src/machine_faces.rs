//! A character the interface's compiled-in faces lack, asked of the machine off the thread that
//! draws.
//!
//! # Why this is here and not in either window
//!
//! ADR 1382 answered `quorra`'s tab strip: a character §9.6.2.2's fourteen do not state is asked of
//! the machine, by *file*, through the covering search §9.7.4.2's substituted composite fonts use —
//! [`pdf_font::substitute::installed_covering`]. `quorra-qt` has the same question about the same
//! strings, and a toolkit that falls back by *family* answers it wrongly on a machine whose covering
//! face shares its family name with Latin ones (ADR 1406). So the question is asked once, here, and
//! both windows take the answer: `quorra` draws the face through its own chrome, and `quorra-qt`
//! registers the same file with Qt's font database.
//!
//! # Why the search is on a thread of its own
//!
//! `CLAUDE.md`'s second principle: "[n]o system font enumeration … on the launch path". The search
//! reads font files until one covers the character, and the first time it runs it walks the whole
//! catalogue — measured on this machine at a second with the fonts out of the page cache and a
//! hundred milliseconds with them in (ADR 1406). A window that asked on its drawing thread held its
//! first present, or the frame after it, for that long. So [`MachineFaces::ask`] never searches: it
//! answers what is known, queues what is not, and the thread that searches wakes the window when an
//! answer lands. Until then the character is whatever the window draws for a character it has no
//! face for, which in `quorra` is the box — and the window draws again when the face arrives.
//!
//! One thread and one queue rather than one thread per character: a line of Japanese asks for every
//! character at once, and every search after the first is answered by the face the first one found
//! ([`pdf_font::substitute::face_covers`]), which a pool of parallel walks would not know about.
//!
//! # A word is asked for as a word
//!
//! A character asked alone is answered by the first face found that states it, which is right for
//! a character and wrong for a word: an Arabic word whose first form one face states and whose
//! last form only another does would be drawn in two faces, their weights and their joins not
//! meeting in the middle of it. So a window asks [`MachineFaces::ask_word`] for every character of
//! a word its own faces lack, the search looks for one face stating all of them, and only where the
//! machine has no such face does the window fall back to [`MachineFaces::ask`] per character
//! (ADR 1430).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Condvar, Mutex, PoisonError};

/// One character, in one style: the unit a face is asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Wanted {
    /// The character no compiled-in face states.
    pub character: char,
    /// Whether a bold face is asked for.
    pub bold: bool,
    /// Whether an italic face is asked for.
    pub italic: bool,
}

/// The characters of one word that a window's own faces lack, in one style: the unit a word's face
/// is asked for (ADR 1430).
///
/// Held sorted and without repeats, so that the same word asked twice, or two words spelling the
/// same letters, is one question and one search.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Word {
    /// The characters, sorted, each once.
    characters: Box<[char]>,
    /// Whether a bold face is asked for.
    bold: bool,
    /// Whether an italic face is asked for.
    italic: bool,
}

impl Word {
    /// The word spelling `characters`, in the style given.
    #[must_use]
    pub fn new(characters: &[char], bold: bool, italic: bool) -> Self {
        let mut characters = characters.to_vec();
        characters.sort_unstable();
        characters.dedup();
        Self {
            characters: characters.into_boxed_slice(),
            bold,
            italic,
        }
    }

    /// The characters asked for, sorted, each once.
    #[must_use]
    pub fn characters(&self) -> &[char] {
        &self.characters
    }
}

/// One question on the searching thread's queue.
#[derive(Debug, Clone)]
enum Asked {
    /// A face stating one character.
    Character(Wanted),
    /// One face stating every character of a word.
    Word(Word),
}

/// What is known about one [`Wanted`] or [`Word`] at the moment it is asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Answer {
    /// Face number `.0` of [`MachineFaces::face`] states a glyph for it.
    Face(usize),
    /// The machine was asked and offers no face for it.
    Nothing,
    /// The search is queued or running; the window is woken when it lands.
    Pending,
}

/// What the window supplies to be woken with, from the searching thread.
type Wake = Arc<dyn Fn() + Send + Sync>;

/// What the window's thread and the searching thread share.
#[derive(Default)]
struct State {
    /// Every face found, in the order found, as the bytes the search read.
    faces: Vec<Arc<[u8]>>,
    /// What each character asked for came to; absent while it is still [`Answer::Pending`].
    answered: BTreeMap<Wanted, Option<usize>>,
    /// Characters queued and not yet answered: what a second ask answers [`Answer::Pending`] from,
    /// and what [`MachineFaces::settle`] waits to see empty.
    pending: BTreeSet<Wanted>,
    /// What each word asked for came to: the one face stating all of it, or `None` where the
    /// machine has no such face and the window falls back to a face per character.
    words: BTreeMap<Word, Option<usize>>,
    /// Words queued and not yet answered, as [`State::pending`] is for characters.
    pending_words: BTreeSet<Word>,
    /// Whether an answer has landed since [`MachineFaces::take_arrivals`] last asked.
    arrived: bool,
    /// The queue to the searching thread, made when the first character or word is asked for.
    queue: Option<Sender<Asked>>,
    /// How to wake the window that asked.
    wake: Option<Wake>,
}

/// The shared half, behind one lock and one condition.
#[derive(Default)]
struct Shared {
    /// Everything [`State`] holds.
    state: Mutex<State>,
    /// Signalled whenever an entry leaves [`State::pending`].
    settled: Condvar,
}

impl Shared {
    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// The machine's faces for characters a window's own faces lack, found without holding the window.
///
/// Cheap to make: nothing is read and no thread exists until the first [`Self::ask`] for a
/// character nobody has answered, so a window whose text its own faces cover never starts one.
#[derive(Default, Clone)]
pub struct MachineFaces {
    /// The state the searching thread writes answers into.
    shared: Arc<Shared>,
}

impl std::fmt::Debug for MachineFaces {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let state = self.shared.lock();
        f.debug_struct("MachineFaces")
            .field("faces", &state.faces.len())
            .field("answered", &state.answered.len())
            .field("pending", &state.pending.len())
            .field("words", &state.words.len())
            .finish_non_exhaustive()
    }
}

impl MachineFaces {
    /// No faces, no answers and no thread.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// What to call, from the searching thread, when an answer lands.
    ///
    /// A window's event loop is the thing to wake: `winit`'s `EventLoopProxy`, or nothing at all
    /// for a window that polls [`Self::take_arrivals`] on a timer of its own.
    pub fn wake_with(&self, wake: impl Fn() + Send + Sync + 'static) {
        self.shared.lock().wake = Some(Arc::new(wake));
    }

    /// What is known about `wanted`, and the search queued where nothing is.
    ///
    /// Never reads a font file on the calling thread, except on a machine that cannot start the
    /// searching thread at all, where the one search is run here rather than the character being
    /// given up.
    pub fn ask(&self, wanted: Wanted) -> Answer {
        let mut state = self.shared.lock();
        match state.answered.get(&wanted) {
            Some(Some(index)) => return Answer::Face(*index),
            Some(None) => return Answer::Nothing,
            None if state.pending.contains(&wanted) => return Answer::Pending,
            None => {}
        }
        // A face already found is asked on this thread: it is one `cmap` lookup, and it is what
        // answers every character of a line after its first.
        if let Some(index) = state
            .faces
            .iter()
            .position(|face| pdf_font::substitute::face_covers(face, &[wanted.character]))
        {
            state.answered.insert(wanted, Some(index));
            return Answer::Face(index);
        }
        if state.queue.is_none() {
            state.queue = self.start();
        }
        let queued = state
            .queue
            .as_ref()
            .is_some_and(|queue| queue.send(Asked::Character(wanted)).is_ok());
        if queued {
            state.pending.insert(wanted);
            return Answer::Pending;
        }
        drop(state);
        // No thread could be started, so the search is run here: a character drawn late is still
        // better than a box that never becomes one.
        let found = search(wanted);
        let mut state = self.shared.lock();
        let answer = record(&mut state, wanted, found);
        answer.map_or(Answer::Nothing, Answer::Face)
    }

    /// What is known about one face stating every character of `word`, and the search queued where
    /// nothing is (ADR 1430).
    ///
    /// [`Answer::Nothing`] means the machine has no single face for the word, and is the window's
    /// cue to ask [`Self::ask`] for each character instead. A face already found that states the
    /// whole word answers it at once, as it does a character. Never reads a font file on the
    /// calling thread, except where no searching thread can be started, as for [`Self::ask`].
    pub fn ask_word(&self, word: &Word) -> Answer {
        let mut state = self.shared.lock();
        match state.words.get(word) {
            Some(Some(index)) => return Answer::Face(*index),
            Some(None) => return Answer::Nothing,
            None if state.pending_words.contains(word) => return Answer::Pending,
            None => {}
        }
        if let Some(index) = held_covering(&state, word.characters()) {
            state.words.insert(word.clone(), Some(index));
            return Answer::Face(index);
        }
        if state.queue.is_none() {
            state.queue = self.start();
        }
        let queued = state
            .queue
            .as_ref()
            .is_some_and(|queue| queue.send(Asked::Word(word.clone())).is_ok());
        if queued {
            state.pending_words.insert(word.clone());
            return Answer::Pending;
        }
        drop(state);
        let found = search_word(word);
        let mut state = self.shared.lock();
        record_word(&mut state, word, found).map_or(Answer::Nothing, Answer::Face)
    }

    /// Face `index`'s bytes, as the search read them.
    #[must_use]
    pub fn face(&self, index: usize) -> Option<Arc<[u8]>> {
        self.shared.lock().faces.get(index).cloned()
    }

    /// How many faces have been found so far.
    #[must_use]
    pub fn found(&self) -> usize {
        self.shared.lock().faces.len()
    }

    /// Whether any search is queued or running.
    #[must_use]
    pub fn searching(&self) -> bool {
        let state = self.shared.lock();
        !state.pending.is_empty() || !state.pending_words.is_empty()
    }

    /// Whether an answer has landed since this was last asked, which is when a window draws again.
    #[must_use]
    pub fn take_arrivals(&self) -> bool {
        std::mem::take(&mut self.shared.lock().arrived)
    }

    /// Waits until every queued search has landed.
    ///
    /// **For a caller measuring the faces, never for a window**: a test that compares what a line
    /// draws with a machine face behind it, or an example counting coverage. A window that waited
    /// here would have put the catalogue walk back on the path this module exists to take it off.
    pub fn settle(&self) {
        let mut state = self.shared.lock();
        while !state.pending.is_empty() || !state.pending_words.is_empty() {
            state = self
                .shared
                .settled
                .wait(state)
                .unwrap_or_else(PoisonError::into_inner);
        }
    }

    /// Starts the searching thread and answers its queue, or `None` where no thread can be had.
    fn start(&self) -> Option<Sender<Asked>> {
        let (queue, asked) = channel::<Asked>();
        let shared = Arc::clone(&self.shared);
        std::thread::Builder::new()
            .name("machine-faces".to_owned())
            .spawn(move || {
                for question in asked {
                    let wake = match question {
                        Asked::Character(wanted) => answer_character(&shared, wanted),
                        Asked::Word(word) => answer_word(&shared, &word),
                    };
                    shared.settled.notify_all();
                    if let Some(wake) = wake {
                        wake();
                    }
                }
            })
            .ok()
            .map(|_| queue)
    }
}

/// Answers one character on the searching thread, and says whom to wake.
fn answer_character(shared: &Shared, wanted: Wanted) -> Option<Wake> {
    // A face found since this was queued may already answer it, which is the common case for
    // every character of a line after its first.
    let known = held_covering(&shared.lock(), &[wanted.character]);
    let found = match known {
        Some(index) => Found::Known(index),
        None => search(wanted).map_or(Found::None, Found::New),
    };
    let mut state = shared.lock();
    match found {
        Found::Known(index) => {
            state.answered.insert(wanted, Some(index));
        }
        Found::New(bytes) => {
            record(&mut state, wanted, Some(bytes));
        }
        Found::None => {
            record(&mut state, wanted, None);
        }
    }
    state.pending.remove(&wanted);
    state.arrived = true;
    state.wake.clone()
}

/// Answers one word on the searching thread, and says whom to wake.
fn answer_word(shared: &Shared, word: &Word) -> Option<Wake> {
    let known = held_covering(&shared.lock(), word.characters());
    let found = if known.is_none() {
        search_word(word)
    } else {
        None
    };
    let mut state = shared.lock();
    if let Some(index) = known {
        state.words.insert(word.clone(), Some(index));
    } else {
        record_word(&mut state, word, found);
    }
    state.pending_words.remove(word);
    state.arrived = true;
    state.wake.clone()
}

/// The first face already found that states every one of `characters`.
fn held_covering(state: &State, characters: &[char]) -> Option<usize> {
    state
        .faces
        .iter()
        .position(|face| pdf_font::substitute::face_covers(face, characters))
}

/// What the searching thread found for one character.
enum Found {
    /// A face already held answers it.
    Known(usize),
    /// The machine offered a face nobody held yet.
    New(Arc<[u8]>),
    /// The machine offers nothing.
    None,
}

/// Enters an answer, keeping one entry per distinct face, and says which face it is.
fn record(state: &mut State, wanted: Wanted, found: Option<Arc<[u8]>>) -> Option<usize> {
    let index = found.map(|bytes| held(state, bytes));
    state.answered.insert(wanted, index);
    index
}

/// Enters a word's answer the way [`record`] enters a character's.
fn record_word(state: &mut State, word: &Word, found: Option<Arc<[u8]>>) -> Option<usize> {
    let index = found.map(|bytes| held(state, bytes));
    state.words.insert(word.clone(), index);
    index
}

/// Which face `bytes` is, holding it if it is new.
fn held(state: &mut State, bytes: Arc<[u8]>) -> usize {
    // The search hands back the same `Arc` for the same file, so a second character answered by a
    // face already held is not a second face.
    state
        .faces
        .iter()
        .position(|face| Arc::ptr_eq(face, &bytes))
        .unwrap_or_else(|| {
            state.faces.push(bytes);
            state.faces.len().saturating_sub(1)
        })
}

/// The one search: ADR 1382's, for one character in one style of the sans-serif family — the
/// covering search a page's substituted composite font asks too (ADR 1441), handed what the chrome
/// knows of its style: a weight of 400 or 700 and a slope, at normal width (ADR 1430).
fn search(wanted: Wanted) -> Option<Arc<[u8]>> {
    let request = chrome_request(wanted.bold, wanted.italic);
    pdf_font::substitute::installed_covering_styled(
        request,
        pdf_font::substitute::Style::of(request),
        &[wanted.character],
    )
}

/// The same search for one face stating every character of a word (ADR 1430).
fn search_word(word: &Word) -> Option<Arc<[u8]>> {
    let request = chrome_request(word.bold, word.italic);
    pdf_font::substitute::installed_covering_styled(
        request,
        pdf_font::substitute::Style::of(request),
        word.characters(),
    )
}

/// What the chrome asks the machine for: its own sans-serif family, in a weight and a slope.
fn chrome_request(bold: bool, italic: bool) -> pdf_font::substitute::Request {
    pdf_font::substitute::Request {
        family: pdf_font::substitute::Family::SansSerif,
        bold,
        italic,
        standard: false,
    }
}

/// Whether §9.6.2.2's compiled-in sans-serif face states no glyph for `character`, by either of the
/// two routes `viewer-ui`'s chrome draws through — so that a window asking the machine asks for
/// exactly the characters `quorra` would have drawn as a box.
///
/// Whitespace, control characters and U+FFFD are never asked for: a space the face cannot spell is
/// still a space, a control character has no visible form to be missing, and U+FFFD is §7.9.2.2's
/// report about the file rather than a character a face could lack.
#[must_use]
pub fn compiled_in_lacks(character: char) -> bool {
    static FACE: std::sync::OnceLock<Option<pdf_font::LoadedFont>> = std::sync::OnceLock::new();
    if character.is_whitespace()
        || character.is_control()
        || character == char::REPLACEMENT_CHARACTER
    {
        return false;
    }
    // A build whose compiled-in face will not parse has no face to lack anything, so every
    // character is asked of the machine; `pdf-font`'s own `every_compiled_in_face_parses` is what
    // catches that build.
    let Some(face) = FACE
        .get_or_init(|| pdf_font::LoadedFont::standard("Helvetica").ok())
        .as_ref()
    else {
        return true;
    };
    face.code_for(character).is_none() && face.character_glyph(character).is_none()
}

#[cfg(test)]
mod tests {
    use super::{Answer, MachineFaces, Wanted, compiled_in_lacks};

    fn regular(character: char) -> Wanted {
        Wanted {
            character,
            bold: false,
            italic: false,
        }
    }

    /// The compiled-in face states Latin, Greek and Cyrillic and no Han; a space, a control
    /// character and U+FFFD are never asked for.
    #[test]
    fn what_the_compiled_in_face_lacks() {
        assert!(!compiled_in_lacks('a'));
        assert!(!compiled_in_lacks('é'));
        assert!(!compiled_in_lacks('Ж'));
        assert!(compiled_in_lacks('多'));
        assert!(!compiled_in_lacks('\u{3000}'));
        assert!(!compiled_in_lacks('\u{7}'));
        assert!(!compiled_in_lacks(char::REPLACEMENT_CHARACTER));
    }

    /// The first ask is pending and searches nowhere near the asking thread; the answer lands, the
    /// window is woken, and a second character the found face states is answered at once.
    #[test]
    fn the_first_ask_is_pending_and_the_answer_lands() {
        let faces = MachineFaces::new();
        let woken = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let count = std::sync::Arc::clone(&woken);
        faces.wake_with(move || {
            count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        });
        assert_eq!(faces.ask(regular('多')), Answer::Pending);
        assert_eq!(
            faces.ask(regular('多')),
            Answer::Pending,
            "asked twice before it lands, and queued once"
        );
        faces.settle();
        assert!(!faces.searching());
        assert!(faces.take_arrivals());
        assert!(!faces.take_arrivals(), "an arrival is taken once");
        assert_eq!(woken.load(std::sync::atomic::Ordering::SeqCst), 1);
        match faces.ask(regular('多')) {
            Answer::Face(index) => {
                assert!(faces.face(index).is_some());
                // Every face that states 多 on a machine with CJK states 边 too; where one does
                // not, the search is queued again rather than answered wrongly.
                if let Answer::Face(other) = faces.ask(regular('边')) {
                    assert_eq!(other, index, "the face already found answers it");
                }
            }
            Answer::Nothing => {
                eprintln!("skipped the face half: this machine offers no face stating 多");
            }
            Answer::Pending => panic!("settled, so nothing is pending"),
        }
    }

    /// A word is answered by one face stating all of it, even where a face already held states
    /// its other letters and not one of them (ADR 1430).
    ///
    /// The characters are Urdu "کہانی" as a label displays it: kaf initial U+FB90, heh goal medial
    /// U+FBA9, alef final, noon initial and farsi yeh final. The face the machine answers a lam
    /// with alone is asked first, as a line drawn earlier would have found it; where that face
    /// lacks U+FBA9 — as the widest face on a machine with Noto Arabic does — the answer character
    /// by character would change face inside the word.
    #[test]
    fn a_word_is_answered_by_one_face() {
        let faces = MachineFaces::new();
        let word = ['\u{FB90}', '\u{FBA9}', '\u{FE8E}', '\u{FEE7}', '\u{FBFD}'];
        assert_eq!(faces.ask(regular('\u{FEDF}')), Answer::Pending);
        faces.settle();
        let asked = super::Word::new(&word, false, false);
        assert_eq!(asked.characters().len(), word.len());
        let answer = match faces.ask_word(&asked) {
            Answer::Pending => {
                faces.settle();
                faces.ask_word(&asked)
            }
            answered => answered,
        };
        match answer {
            Answer::Face(index) => {
                let face = faces.face(index).expect("an answered face is held");
                assert!(
                    pdf_font::substitute::face_covers(&face, &word),
                    "the word's face states every letter of it"
                );
            }
            Answer::Nothing => {
                eprintln!("skipped: this machine offers no single face for the word");
            }
            Answer::Pending => panic!("settled, so nothing is pending"),
        }
        assert!(!faces.searching());
    }
}
