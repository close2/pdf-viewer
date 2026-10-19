//! A conversion to ISO 19005 (PDF/A) asked for in a window: the word that chooses it, the run on a
//! thread of its own, and the file written beside the document.
//!
//! **Why the run is here and not in `viewer-core`.** The converter is `pdf_transform`'s, and that
//! crate depends on `viewer-core`, so the core can only grant the operation:
//! [`viewer_core::Command::Archive`] asks the reader's level for Table 22's bit 11 and answers with
//! [`viewer_core::Event::Archiving`], which carries the file. What follows is the host's, and it is
//! written once here for the three windows and the C library rather than four times (ADR 1954):
//!
//! - **The profile list is `pdf_transform::profiles`'s**, compiled in once, so a window offers the
//!   profiles `quorra-transform archive --profile` takes and refuses the names it refuses.
//! - **A tool a profile declares is run through `pdf_transform::executor::execute`**, the one shared
//!   executor `doc/questions/A54` settled, on the converter's own thread: a pass returns the
//!   requests, they are run, and the next pass has their results — so "it just happens" in a
//!   window exactly as on the command line.
//! - **The run is on a thread of its own**, because it validates the whole document twice and a
//!   window that waited for it would stop answering for as long as that takes. The thread reports
//!   on a channel the window drains on the timer it already runs for a server's answers
//!   ([`crate::submit::LOOK`]), or wakes a loop that can be woken.
//! - **The file goes beside the document**, as `.edited.pdf` does for a save: `<name>.pdfa-2b.pdf`
//!   for PDF/A-2b, never over anything the person has.
//!
//! What a window does *not* do that the command line does: it supplies no font program, adds no
//! output intent profile of its own, fetches no stream data a file keeps outside itself, and
//! departs from the standard never — each is an operator's statement at a call site, which a key
//! press is not. A profile that names a departure is refused with the command that can make it.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

use pdf_archive::Target;
use pdf_transform::archive::{ArchivePlan, Conversion, UNTRUSTED_INPUT_WARNING};
use pdf_transform::profiles::{self, Contribution};
use pdf_transform::tool::ToolOutputs;
use pdf_transform::{Budget, Level, MemorySinks, Plan, Policy, Report, Source};
use viewer_core::{Archiving, Command, DocumentId, Secret};

/// The word that chooses a window's conversion: `--archive <target>:<profile>`.
pub const ARCHIVE: &str = "--archive";

/// What the confined window says when its archive key is pressed.
///
/// A conversion parses the whole document again and writes a new one, and the confined window's
/// process is the side of its pipe that parses no document — so the window that offers one is
/// one of the other three, and this sentence says so rather than a key doing nothing (ADR 1954).
pub const NOT_IN_A_CONFINED_WINDOW: &str = "this window converts nothing to ISO 19005: a \
     conversion reads the whole document again, and this window's process reads none — quorra, \
     quorra-gtk and quorra-qt convert with --archive <target>:<profile> and Shift+A (ADR 1954)";

/// How many passes a conversion may make asking for tools before it stops.
///
/// The command line's bound and its reason: a request's identifier is a function of the
/// document, so one extra pass is all a correct implementation needs, and the bound is a ceiling
/// on a defect rather than on a profile (`CLAUDE.md` principle 3).
const MOST_TOOL_PASSES: usize = 4;

/// Reads `--archive`'s value, `<target>:<profile>` — `2b:as-if-printed`.
///
/// # Errors
///
/// The sentence to print where the value names no target this converter writes, or a profile it
/// does not ship; each names the ones it does.
pub fn chosen(value: &str) -> Result<Archiving, String> {
    let Some((target, profile)) = value.split_once(':') else {
        return Err(format!(
            "{ARCHIVE} wants <target>:<profile>, as in 2b:as-if-printed; the targets are 2b, 2u, \
             2a, 4, 4f and 4e, and the profiles are {}",
            profiles::names()
        ));
    };
    let target = Target::parse(target).ok_or_else(|| {
        format!(
            "{ARCHIVE} {value:?}: the targets are 2b, 2u, 2a, 4, 4f and 4e. Parts 1 and 3 are not \
             targets (doc/questions/A17)"
        )
    })?;
    let profile = profiles::named(profile).ok_or_else(|| {
        format!(
            "{ARCHIVE} {value:?}: the profiles this program ships are {}",
            profiles::names()
        )
    })?;
    Ok(Archiving {
        target,
        profile: profile.name.to_owned(),
    })
}

/// What a window's archive key does: convert as chosen, or put the choice in front of the person.
#[derive(Debug)]
pub enum Pressed {
    /// A target and a profile were chosen — by `--archive`, or by an earlier [`Choosing`] in this
    /// window — and this is the command that asks for the conversion.
    Convert(Command),
    /// Nothing was chosen yet: the window offers the six targets and the shipped profiles, and
    /// the person's choice is [`Choosing::chosen`] (ADR 1966).
    Choose(Choosing),
}

/// The archive key, pressed with `chosen` as the window holds it.
///
/// **No default target**, for `quorra-transform archive`'s reason: which part and level a
/// document has to reach is the one thing this program cannot work out for a person, so a key
/// pressed with nothing chosen asks rather than guessing — with the card or dialogue
/// [`Choosing`] describes, where a person reads each profile's own sentence before choosing it.
#[must_use]
pub fn pressed(chosen: Option<&Archiving>) -> Pressed {
    match chosen {
        Some(asked) => Pressed::Convert(Command::Archive(asked.clone())),
        None => Pressed::Choose(Choosing::new()),
    }
}

/// One shipped profile as a window offers it: the name a person chooses, and the one sentence
/// the profile's file states about itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Offered {
    /// The `[profile] name`, which is the word `--archive` and `--profile` take.
    pub name: &'static str,
    /// The `[profile] sentence`, or the sentence saying why there is none to show.
    pub sentence: String,
}

/// Every shipped profile, in [`profiles::SHIPPED`]'s order, each with its sentence.
///
/// Read from the profiles' own text each time rather than written beside their names here, so
/// that what a window offers is what `doc/profiles/` says and nothing else (ADR 1966). A profile
/// whose text does not read is still offered, with the reason in place of its sentence, because
/// choosing it is how a person reaches the conversion's own refusal of it.
#[must_use]
pub fn offered() -> Vec<Offered> {
    profiles::SHIPPED
        .iter()
        .map(|shipped| Offered {
            name: shipped.name,
            sentence: match shipped.sentence() {
                Ok(Some(sentence)) => sentence,
                Ok(None) => "(the profile states no sentence about itself)".to_owned(),
                Err(error) => format!("(the profile does not read: {error})"),
            },
        })
        .collect()
}

/// What the chooser is called, in all three windows: its card's first line and the dialogues'
/// title.
pub const CHOOSING_TITLE: &str = "Convert to ISO 19005 (PDF/A)";

/// What the chooser says under its title: what a profile is (ADR 1942), so that nobody reads
/// one as an instruction.
pub const CHOOSING: &str =
    "A profile is the least a conversion may lose, never an instruction to lose it.";

/// A target and a profile being chosen after launch: one of [`Target::ALL`] and one of
/// [`offered`], each moved through by the keyboard or set by a press, until the person
/// confirms (ADR 1966).
///
/// **The state is the window's and the lists are this module's**, the shape
/// [`crate::Restrictions`] has: `quorra`'s card, the GTK dialogue and the Qt dialogue each draw
/// these two lists and send back two indices, so three windows offer one list in one order. The
/// keyboard starts on the first of each — PDF/A-2b and `refuse-any-loss`, the profile that
/// loses nothing — and nothing converts until the person confirms.
#[derive(Debug, Clone)]
pub struct Choosing {
    /// Which of [`Target::ALL`] is chosen.
    target: usize,
    /// Which of [`Self::profiles`] is chosen.
    profile: usize,
    /// The profiles, read when the choice began.
    profiles: Vec<Offered>,
}

impl Default for Choosing {
    fn default() -> Self {
        Self::new()
    }
}

impl Choosing {
    /// The choice as it begins: the first target and the first profile.
    #[must_use]
    pub fn new() -> Self {
        Self {
            target: 0,
            profile: 0,
            profiles: offered(),
        }
    }

    /// The targets, in the order offered.
    #[must_use]
    pub const fn targets(&self) -> &'static [Target] {
        &Target::ALL
    }

    /// The profiles, in the order offered.
    #[must_use]
    pub fn profiles(&self) -> &[Offered] {
        &self.profiles
    }

    /// Which target is chosen, as an index into [`Self::targets`].
    #[must_use]
    pub const fn target_index(&self) -> usize {
        self.target
    }

    /// Which profile is chosen, as an index into [`Self::profiles`].
    #[must_use]
    pub const fn profile_index(&self) -> usize {
        self.profile
    }

    /// Chooses the target at `index`; answers whether that changed anything. An index past the
    /// list chooses nothing.
    pub fn set_target(&mut self, index: usize) -> bool {
        let changed = index < Target::ALL.len() && index != self.target;
        if changed {
            self.target = index;
        }
        changed
    }

    /// Chooses the profile at `index`; answers whether that changed anything. An index past the
    /// list chooses nothing.
    pub fn set_profile(&mut self, index: usize) -> bool {
        let changed = index < self.profiles.len() && index != self.profile;
        if changed {
            self.profile = index;
        }
        changed
    }

    /// Moves the target by `by` places, stopping at either end rather than wrapping — what every
    /// other list in these windows does; answers whether it moved.
    pub fn move_target(&mut self, by: isize) -> bool {
        self.set_target(moved(self.target, by, Target::ALL.len()))
    }

    /// Moves the profile by `by` places, stopping at either end; answers whether it moved.
    pub fn move_profile(&mut self, by: isize) -> bool {
        self.set_profile(moved(self.profile, by, self.profiles.len()))
    }

    /// What the person chose, as `--archive` would have said it.
    #[must_use]
    pub fn chosen(&self) -> Archiving {
        Archiving {
            target: Target::ALL
                .get(self.target)
                .copied()
                .unwrap_or(Target::ALL[0]),
            profile: self
                .profiles
                .get(self.profile)
                .map_or(profiles::SHIPPED[0].name, |offered| offered.name)
                .to_owned(),
        }
    }
}

/// `at` moved by `by` within `0..len`, held at the ends.
fn moved(at: usize, by: isize, len: usize) -> usize {
    at.saturating_add_signed(by).min(len.saturating_sub(1))
}

/// What a window says once a person has chosen: what it converts to, and that the choice holds
/// for the rest of the window's life — the next press of the key converts without asking.
#[must_use]
pub fn remembered(asked: &Archiving) -> String {
    format!(
        "chose {} under the profile {:?}; Shift+A converts to it from now on in this window, as \
         {ARCHIVE} {}:{} would have",
        asked.target,
        asked.profile,
        target_word(asked.target),
        asked.profile
    )
}

/// The word `--archive` names a target by: `2b`, `4f`.
fn target_word(target: Target) -> String {
    target
        .to_string()
        .to_ascii_lowercase()
        .replace("pdf/a-", "")
}

/// What a window says where the chooser was closed without a choice.
pub const NOT_CHOSEN: &str = "nothing was converted: no target and profile were chosen";

/// Where a converted file is written: beside the document, its target in the name.
///
/// `drive.pdf` converted to PDF/A-2b is `drive.pdfa-2b.pdf`, for `.edited.pdf`'s reason —
/// overwriting somebody's document is a decision this program has not been given — and the
/// target is in the name so that two conversions of one document do not write over each other.
#[must_use]
pub fn beside(path: &Path, target: Target) -> PathBuf {
    let word = target
        .to_string()
        .to_ascii_lowercase()
        .replace("pdf/a-", "pdfa-");
    path.with_extension(format!("{word}.pdf"))
}

/// One conversion, finished.
#[derive(Debug)]
pub struct Converted {
    /// The document it was asked of.
    pub document: DocumentId,
    /// What was asked for.
    pub asked: Archiving,
    /// The converted file, where one was written; `None` where a requirement was refused, and
    /// [`Self::lines`] says which.
    pub written: Option<Vec<u8>>,
    /// What the window says, in order: the profile once, then the report's lines per site, then
    /// anything the run declined.
    pub lines: Vec<String>,
    /// RFC 0002 section 4.5's JSON for the conversion, with its `profile` field, where the run got
    /// as far as a report.
    pub json: Option<String>,
}

/// Converts `bytes` as `asked` says, on the calling thread, opened with `password` where the
/// window holds the one the document opened with ([`crate::password::Kept`], ADR 1967).
///
/// The level is off because the reader's level was asked already: [`viewer_core::Event::Archiving`]
/// is the grant, and arrives only once `viewer-core` asked Table 22's bit 11 at that level.
#[must_use]
pub fn convert(
    document: DocumentId,
    bytes: pdf_syntax::FileBytes,
    asked: Archiving,
    password: Option<Secret>,
) -> Converted {
    let mut lines = Vec::new();
    let source = match password {
        Some(password) => Source::with_password(bytes, password),
        None => Source::new(bytes),
    };
    let (written, json) = match planned(&asked, &mut lines) {
        Some((plan, profile)) => match passes(plan, source, &mut lines) {
            Some((report, sinks)) => said(&report, sinks, profile, &mut lines),
            None => (None, None),
        },
        None => (None, None),
    };
    Converted {
        document,
        asked,
        written,
        lines,
        json,
    }
}

/// The plan `asked` names, with the profile's own name, after the lines that say what the profile
/// contributes; `None` where it cannot be made, and the last line says why.
fn planned(asked: &Archiving, lines: &mut Vec<String>) -> Option<(ArchivePlan, &'static str)> {
    let Some(shipped) = profiles::named(&asked.profile) else {
        lines.push(format!(
            "cannot convert: {:?} is not a profile this program ships; they are {}",
            asked.profile,
            profiles::names()
        ));
        return None;
    };
    let contribution = match Contribution::read(shipped.text, asked.target) {
        Ok(contribution) => contribution,
        Err(error) => {
            lines.push(format!(
                "cannot convert: the profile {:?} does not read: {error}",
                shipped.name
            ));
            return None;
        }
    };
    if !contribution.departures.is_empty() {
        lines.push(format!(
            "cannot convert: the profile {:?} departs from ISO 19005, which a window does not do; \
             quorra-transform archive --depart-from-the-standard says so at its call site \
             (doc/rfc/0007 section 4.7.3)",
            shipped.name
        ));
        return None;
    }
    lines.push(format!(
        "converting to {} under the profile {:?}: the sites below marked as answered by this \
         configuration were answered by it",
        asked.target, shipped.name
    ));
    for unbuilt in &contribution.unbuilt {
        lines.push(format!(
            "the profile answers {:?} with `{}`, which this version does not carry out yet; that \
             site stays refused with the sentence it names",
            unbuilt.site,
            unbuilt.remedy.word()
        ));
    }
    for tool in &contribution.tools {
        lines.push(format!(
            "the profile declares the tool {:?} as {} — {UNTRUSTED_INPUT_WARNING}",
            tool.name,
            tool.program.display()
        ));
    }
    let names = match "archive.pdf".parse() {
        Ok(names) => names,
        Err(error) => {
            lines.push(format!("cannot convert: the output's name: {error}"));
            return None;
        }
    };
    let plan = ArchivePlan {
        source: 0,
        names,
        target: asked.target,
        authorised: contribution.authorised,
        profile: None,
        substitute_fonts: true,
        supplied_fonts: BTreeMap::new(),
        departures: Vec::new(),
        claim_conformance: false,
        forms: contribution.forms,
        derivations: contribution.derivations,
        supplies: contribution.supplies,
        preservations: contribution.preservations,
        resolutions: contribution.resolutions,
        tool_outputs: ToolOutputs::new(),
        external_data: BTreeMap::new(),
    };
    Some((plan, shipped.name))
}

/// Applies the plan until no pass asks for a tool it has not had, running each request through
/// the one shared executor (`doc/questions/A54`); `None` where `apply` refused outright, and the
/// last line says why.
fn passes(
    mut plan: ArchivePlan,
    source: Source,
    lines: &mut Vec<String>,
) -> Option<(Report, MemorySinks)> {
    let sources = [source];
    let policy = Policy {
        restrictions: Level::Off,
    };
    let budget = Budget::default();
    let mut passes = 0_usize;
    loop {
        let sinks = MemorySinks::new();
        let report = match pdf_transform::apply(
            &Plan::Archive(plan.clone()),
            &sources,
            &sinks,
            &policy,
            &budget,
        ) {
            Ok(report) => report,
            Err(refusal) => {
                lines.push(format!("cannot convert: {refusal}"));
                return None;
            }
        };
        passes = passes.saturating_add(1);
        if report.requested.is_empty() || passes > MOST_TOOL_PASSES {
            return Some((report, sinks));
        }
        let mut learned = false;
        for request in &report.requested {
            if plan.tool_outputs.get(&request.id).is_some() {
                continue;
            }
            lines.push(format!(
                "running the tool {:?} ({}) over {} — doc/rfc/0007 section 4.5",
                request.tool,
                request.program.display(),
                request.subject
            ));
            match pdf_transform::executor::execute(request) {
                Ok(result) => {
                    plan.tool_outputs.insert(result);
                    learned = true;
                }
                Err(error) => lines.push(format!("the tool did not run: {error}")),
            }
        }
        if !learned {
            return Some((report, sinks));
        }
    }
}

/// The report's lines, the file where one was written, and the JSON with its `profile` field.
fn said(
    report: &Report,
    sinks: MemorySinks,
    profile: &str,
    lines: &mut Vec<String>,
) -> (Option<Vec<u8>>, Option<String>) {
    let json = report.archive.as_ref().map(|conversion| {
        let mut named: Conversion = conversion.clone();
        named.configuration = Some(profile.to_owned());
        lines.extend(
            conversion
                .render()
                .lines()
                .filter(|line| !line.trim().is_empty())
                .map(str::to_owned),
        );
        named.to_json().render()
    });
    for warning in &report.warnings {
        match warning.page {
            Some(page) => lines.push(format!("page {page}: {}", warning.detail)),
            None => lines.push(warning.detail.clone()),
        }
    }
    for declined in &report.refused {
        lines.push(format!(
            "refused: {}: {}",
            declined.subject, declined.detail
        ));
    }
    let achieved = report
        .archive
        .as_ref()
        .is_some_and(|conversion| conversion.achieved.is_some());
    let written = if achieved {
        sinks
            .into_outputs()
            .into_iter()
            .next()
            .map(|(_, bytes)| bytes)
    } else {
        None
    };
    (written, json)
}

/// Writes a finished conversion beside the document, and says what happened.
///
/// Rule 2's other half: the converter produced the bytes and the window owns the filesystem.
#[must_use]
pub fn write_beside(path: &Path, converted: &Converted) -> String {
    let Some(bytes) = &converted.written else {
        return format!(
            "no {} file was written: the lines above say which requirement refused",
            converted.asked.target
        );
    };
    let to = beside(path, converted.asked.target);
    match std::fs::write(&to, bytes) {
        Ok(()) => format!(
            "converted to {} under the profile {:?}: wrote {} bytes to {}",
            converted.asked.target,
            converted.asked.profile,
            bytes.len(),
            to.display()
        ),
        Err(error) => format!("cannot write {}: {error}", to.display()),
    }
}

/// What a window says when it starts a conversion, before the thread has said anything.
#[must_use]
pub fn starting(asked: &Archiving) -> String {
    format!(
        "converting to {} under the profile {:?} on a thread of its own; the report follows when \
         it is done",
        asked.target, asked.profile
    )
}

/// The conversions a window has running, and the ones that have finished.
///
/// [`crate::submit::Submitter`]'s shape and for its reason: held by the window for its life,
/// [`Self::start`] runs one conversion on a thread of its own, [`Self::collect`] is what the
/// window's timer calls, and [`Self::interval`] is whether that timer runs at all.
#[derive(Debug)]
pub struct Converter {
    /// Where each thread puts its [`Converted`], beside the path of the document it converted.
    sender: Sender<(PathBuf, Converted)>,
    /// Where the window takes them from.
    receiver: Receiver<(PathBuf, Converted)>,
    /// How many threads have not finished yet.
    outstanding: usize,
}

impl Default for Converter {
    fn default() -> Self {
        Self::new()
    }
}

impl Converter {
    /// A window with no conversion running.
    #[must_use]
    pub fn new() -> Self {
        let (sender, receiver) = channel();
        Self {
            sender,
            receiver,
            outstanding: 0,
        }
    }

    /// Starts one conversion on an `archive` thread of its own, of the document read from `path`,
    /// opened with `password` where the document needed one (ADR 1967).
    ///
    /// `wake` is called on that thread once the result is on the channel, for a window whose loop
    /// can be woken from another thread; a window that polls passes nothing.
    ///
    /// # Errors
    ///
    /// The sentence to say where no thread could be started, in which case nothing runs.
    pub fn start(
        &mut self,
        document: DocumentId,
        path: PathBuf,
        bytes: pdf_syntax::FileBytes,
        asked: Archiving,
        password: Option<Secret>,
        wake: Option<Box<dyn Fn() + Send>>,
    ) -> Result<(), String> {
        let sender = self.sender.clone();
        std::thread::Builder::new()
            .name("archive".to_owned())
            .spawn(move || {
                let converted = convert(document, bytes, asked, password);
                // A window that has closed has no receiver, and a result nobody can read is one
                // nobody is owed: the discard is the window's absence rather than a lost error.
                let _ = sender.send((path, converted));
                if let Some(wake) = wake {
                    wake();
                }
            })
            .map_err(|error| format!("archive: no thread to convert on: {error}"))?;
        self.outstanding = self.outstanding.saturating_add(1);
        Ok(())
    }

    /// Every conversion that has finished since the last look, with the path it was read from.
    pub fn collect(&mut self) -> Vec<(PathBuf, Converted)> {
        let arrived: Vec<(PathBuf, Converted)> = self.receiver.try_iter().collect();
        self.outstanding = self.outstanding.saturating_sub(arrived.len());
        arrived
    }

    /// How long to wait before looking again, or `None` where nothing is running — which is what
    /// stops a window's timer, so an idle window wakes for this never.
    #[must_use]
    pub const fn interval(&self) -> Option<Duration> {
        if self.outstanding == 0 {
            None
        } else {
            Some(crate::submit::LOOK)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_word_names_a_target_and_a_shipped_profile() {
        let asked = chosen("2b:as-if-printed").expect("a target and a profile");
        assert_eq!(asked.target, Target::Two(pdf_archive::Level::B));
        assert_eq!(asked.profile, "as-if-printed");
        let wrong = chosen("2b:as-printed").expect_err("no such profile");
        assert!(wrong.contains("as-shown"), "{wrong}");
        let wrong = chosen("1b:as-shown").expect_err("no such target");
        assert!(wrong.contains("A17"), "{wrong}");
        let wrong = chosen("2b").expect_err("no profile");
        assert!(wrong.contains("2b:as-if-printed"), "{wrong}");
    }

    /// With nothing chosen the key offers the six targets and every shipped profile with the
    /// sentence its file states; with something chosen it converts without asking.
    #[test]
    fn nothing_chosen_offers_the_choice_and_a_choice_converts() {
        let Pressed::Choose(mut choosing) = pressed(None) else {
            panic!("no default target: the key asks")
        };
        assert_eq!(choosing.targets(), &Target::ALL);
        let names: Vec<&str> = choosing
            .profiles()
            .iter()
            .map(|offered| offered.name)
            .collect();
        let shipped: Vec<&str> = profiles::SHIPPED
            .iter()
            .map(|shipped| shipped.name)
            .collect();
        assert_eq!(names, shipped);
        assert_eq!(
            choosing.profiles()[1].sentence,
            "Discard what printing would not have carried; touch nothing it would."
        );
        assert_eq!(choosing.chosen().profile, "refuse-any-loss");
        assert!(!choosing.move_profile(-1), "held at the top, never wrapped");
        assert!(choosing.move_profile(1));
        assert!(choosing.move_target(5));
        assert!(!choosing.set_target(6), "past the list chooses nothing");
        let asked = choosing.chosen();
        assert_eq!(
            asked,
            chosen("4e:as-if-printed").expect("the same choice, typed")
        );
        assert!(
            remembered(&asked).contains("--archive 4e:as-if-printed"),
            "{}",
            remembered(&asked)
        );
        assert!(
            matches!(pressed(Some(&asked)), Pressed::Convert(Command::Archive(again)) if again == asked)
        );
    }

    #[test]
    fn the_file_goes_beside_the_document_with_its_target_in_the_name() {
        assert_eq!(
            beside(
                Path::new("/tmp/drive.pdf"),
                Target::Two(pdf_archive::Level::B)
            ),
            PathBuf::from("/tmp/drive.pdfa-2b.pdf")
        );
        assert_eq!(
            beside(
                Path::new("a.pdf"),
                Target::Four(pdf_archive::Flavour::Plain)
            ),
            PathBuf::from("a.pdfa-4.pdf")
        );
    }

    /// A file that is not a PDF refuses with the converter's sentence, said, and writes nothing.
    #[test]
    fn a_file_that_does_not_open_is_said_and_writes_nothing() {
        let asked = chosen("2b:as-if-printed").expect("chosen");
        let converted = convert(DocumentId(1), b"not a pdf".to_vec().into(), asked, None);
        assert!(converted.written.is_none());
        assert!(
            converted
                .lines
                .iter()
                .any(|line| line.starts_with("cannot convert")),
            "{:?}",
            converted.lines
        );
    }

    /// A one-page document this tree writes converts to PDF/A-2b under `as-if-printed`, and the
    /// JSON names the profile.
    #[test]
    fn a_small_document_converts_and_the_json_names_the_profile() {
        let bytes = test_document();
        let asked = chosen("2b:as-if-printed").expect("chosen");
        let converted = convert(DocumentId(1), bytes.into(), asked, None);
        assert!(converted.written.is_some(), "{:#?}", converted.lines);
        let json = converted.json.expect("a report");
        assert!(json.contains("\"profile\": \"as-if-printed\""), "{json}");
        assert!(
            converted.lines[0].contains("under the profile \"as-if-printed\""),
            "{:?}",
            converted.lines
        );
    }

    /// A document behind §7.6.4.1's user password converts with the password the window kept,
    /// and is refused at the open without it — which is the whole of what the window owes it
    /// (ADR 1967). What the conversion then does about the encryption is the converter's.
    #[test]
    fn an_encrypted_document_converts_under_the_password_it_opened_with() {
        let bytes = encrypted(&test_document(), "user");
        let asked = chosen("2b:as-if-printed").expect("chosen");
        let refused = convert(DocumentId(1), bytes.clone().into(), asked.clone(), None);
        assert!(refused.written.is_none());
        assert!(
            refused
                .lines
                .iter()
                .any(|line| line.starts_with("cannot convert") && line.contains("password")),
            "{:?}",
            refused.lines
        );
        let opened = convert(
            DocumentId(1),
            bytes.into(),
            asked,
            Some(Secret::from("user".to_owned())),
        );
        // Past the open: the report reaches ISO 19005-2 section 6.1.3's site, which only a file
        // that was decrypted can be judged at.
        assert!(
            !opened
                .lines
                .iter()
                .any(|line| line.starts_with("cannot convert")),
            "{:?}",
            opened.lines
        );
        assert!(
            opened
                .lines
                .iter()
                .any(|line| line.contains("file-structure/no-encryption")),
            "{:?}",
            opened.lines
        );
        assert!(opened.json.is_some(), "{:?}", opened.lines);
    }

    /// `plain` under §7.6.4's standard security handler, with `user` as its user password.
    fn encrypted(plain: &[u8], user: &str) -> Vec<u8> {
        use pdf_syntax::serialize::{
            Access, Assembly, Entropy, Form, Options, Protection, serialize_encrypted,
        };
        /// A fixed source, so the file is a function of the test.
        struct Counting(u8);
        impl Entropy for Counting {
            fn fill(&mut self, out: &mut [u8]) -> bool {
                for byte in out.iter_mut() {
                    *byte = self.0;
                    self.0 = self.0.wrapping_add(1);
                }
                true
            }
        }
        let source = pdf_syntax::Document::open(plain.to_vec()).expect("the plain file opens");
        let mut assembly = Assembly::new(vec![&source]);
        for number in 1..=4 {
            assembly
                .copy(0, pdf_syntax::object::ObjectId::new(number, 0))
                .expect("the assembly takes the object");
        }
        assembly.set_root(pdf_syntax::object::ObjectId::new(1, 0));
        let mut out = Vec::new();
        serialize_encrypted(
            &assembly,
            pdf_syntax::Version { major: 1, minor: 7 },
            Options::new(Form::Table),
            &Protection {
                user_password: user,
                owner_password: "owner",
                access: Access::ALL,
                encrypt_metadata: true,
            },
            &mut Counting(0),
            &mut out,
        )
        .expect("the file is written");
        out
    }

    /// One page with one filled rectangle, its cross-reference table at the offsets it states:
    /// nothing in it a conversion to PDF/A-2b has to refuse.
    fn test_document() -> Vec<u8> {
        let content = b"0 0 1 rg 10 10 50 50 re f";
        let objects = [
            "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R >>".to_owned(),
            format!(
                "<< /Length {} >>\nstream\n{}\nendstream",
                content.len(),
                String::from_utf8_lossy(content)
            ),
        ];
        let mut file = b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n".to_vec();
        let mut offsets = Vec::new();
        for (index, object) in objects.iter().enumerate() {
            offsets.push(file.len());
            file.extend_from_slice(
                format!("{} 0 obj\n{object}\nendobj\n", index.saturating_add(1)).as_bytes(),
            );
        }
        let table = file.len();
        file.extend_from_slice(
            format!(
                "xref\n0 {}\n0000000000 65535 f \n",
                objects.len().saturating_add(1)
            )
            .as_bytes(),
        );
        for offset in offsets {
            file.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        file.extend_from_slice(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{table}\n%%EOF\n",
                objects.len().saturating_add(1)
            )
            .as_bytes(),
        );
        file
    }
}
