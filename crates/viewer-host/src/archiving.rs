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
use viewer_core::{Archiving, Command, DocumentId};

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

/// The command a window's archive key sends, or the sentence it says where nothing was chosen.
///
/// **No default target**, for `quorra-transform archive`'s reason: which part and level a
/// document has to reach is the one thing this program cannot work out for a person, so a key
/// pressed with nothing chosen says how to choose rather than guessing.
///
/// # Errors
///
/// That sentence, naming the word and the profiles.
pub fn command(chosen: Option<&Archiving>) -> Result<Command, String> {
    chosen.cloned().map(Command::Archive).ok_or_else(|| {
        format!(
            "no conversion was chosen: start the window with {ARCHIVE} <target>:<profile>, as in \
             {ARCHIVE} 2b:as-if-printed; the profiles are {}",
            profiles::names()
        )
    })
}

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

/// Converts `bytes` as `asked` says, on the calling thread.
///
/// The level is off because the reader's level was asked already: [`viewer_core::Event::Archiving`]
/// is the grant, and arrives only once `viewer-core` asked Table 22's bit 11 at that level.
#[must_use]
pub fn convert(document: DocumentId, bytes: pdf_syntax::FileBytes, asked: Archiving) -> Converted {
    let mut lines = Vec::new();
    let (written, json) = match planned(&asked, &mut lines) {
        Some((plan, profile)) => match passes(plan, bytes, &mut lines) {
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
    bytes: pdf_syntax::FileBytes,
    lines: &mut Vec<String>,
) -> Option<(Report, MemorySinks)> {
    let sources = [Source::new(bytes)];
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

    /// Starts one conversion on an `archive` thread of its own, of the document read from `path`.
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
        wake: Option<Box<dyn Fn() + Send>>,
    ) -> Result<(), String> {
        let sender = self.sender.clone();
        std::thread::Builder::new()
            .name("archive".to_owned())
            .spawn(move || {
                let converted = convert(document, bytes, asked);
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

    #[test]
    fn nothing_chosen_says_how_to_choose() {
        let said = command(None).expect_err("no default target");
        assert!(said.contains("--archive 2b:as-if-printed"), "{said}");
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
        let converted = convert(DocumentId(1), b"not a pdf".to_vec().into(), asked);
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
        let converted = convert(DocumentId(1), bytes.into(), asked);
        assert!(converted.written.is_some(), "{:#?}", converted.lines);
        let json = converted.json.expect("a report");
        assert!(json.contains("\"profile\": \"as-if-printed\""), "{json}");
        assert!(
            converted.lines[0].contains("under the profile \"as-if-printed\""),
            "{:?}",
            converted.lines
        );
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
