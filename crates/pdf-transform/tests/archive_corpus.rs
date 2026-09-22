//! `archive` over the veraPDF corpus: three properties, over a population of witnesses.
//!
//! `crates/pdf-transform/tests/archive.rs` states what the converter *does*, from a fixture built
//! out of the clauses. This states what it may never do, over a few hundred documents nobody here
//! wrote:
//!
//! 1. **A document the validator already finds conforming converts, and the output still
//!    conforms.** The conversion has nothing to decide, so anything it changed was a change no
//!    failed requirement asked for — which is the rule the whole verb rests on.
//! 2. **Nothing panics, and nothing comes back as an error.** A conversion either writes a file or
//!    refuses by name; `crate::Refusal` is for a source that could not be read at all, and a
//!    corpus document that reaches one is a bug in this verb rather than a fact about the file.
//! 3. **No glyph moves.** Every glyph advance the output's font dictionaries state is one the
//!    source stated. `doc/pdf-a-conversion-limits.md` section 4.9 offers three ways to make a
//!    font dictionary agree with its embedded program and calls the third — restating the
//!    dictionary — **never**, because ISO 32000-2 §9.2.4 makes those numbers what a reader
//!    positions glyphs by; a converter that took it would move every line of every page it
//!    touched and nothing in its report would say so. [`every_stated_width`] is the check, and
//!    it is absolute rather than sampled: the whole set of statements, before and after.
//!
//! **The corpus is a population, never an answer key** — `crates/pdf-archive/tests/corpus.rs` has
//! that argument in full, and `CLAUDE.md` principle 5 is where it comes from. Nothing here treats
//! veraPDF's own verdict as the expected value: the expected value is *this tree's* validator run
//! twice, before and after, which is a statement about the converter and not about the standard.
//!
//! ```sh
//! cargo test --profile gates -p pdf-transform --test archive_corpus -- --ignored --nocapture
//! ```
//!
//! `doc/veraPDF-corpus` is 239 MB and not part of a checkout, so this is `#[ignore]`d and says so
//! when the submodule is absent.

// no sandbox worker: the converter and the validator it runs twice rewrite and read the object graph; neither interprets a page or decodes an image (`tools/conformance/tests/sandbox_gates.rs`).

#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::print_stdout,
    reason = "test code whose entire product is a report a person reads, and whose two \
              properties must fail loudly when a corpus document breaks one"
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use pdf_archive::{Flavour, Level, Target, Verdict};
use pdf_syntax::{Document, Limits};
use pdf_transform::archive::{
    ArchivePlan, Authorisations, Decision, ExternalData, Loss, PLACEMENT_ON_PAGE, Preservation,
};
use pdf_transform::tool::ToolOutputs;
use pdf_transform::{Budget, MemorySinks, Plan, Policy, Source, apply};

/// Every target, with the corpus directory whose documents were written for it.
///
/// **All six, and the four small ones are the point.** A target nothing exercises is a target
/// whose refusals nobody has seen: `PDF_A-4f` holds eleven documents and `PDF_A-2a` twenty-seven,
/// which is few enough to prove nothing statistically and enough to catch a panic, an `Err`, or a
/// conforming document this verb quietly breaks. The directory names are veraPDF's own.
const TARGETS: [(&str, Target); 6] = [
    ("PDF_A-2b", Target::Two(Level::B)),
    ("PDF_A-2u", Target::Two(Level::U)),
    ("PDF_A-2a", Target::Two(Level::A)),
    ("PDF_A-4", Target::Four(Flavour::Plain)),
    ("PDF_A-4f", Target::Four(Flavour::F)),
    ("PDF_A-4e", Target::Four(Flavour::E)),
];

/// How many of a target's refused requirements the sweep prints.
///
/// Enough to see the shape of what is left and short enough to read; the tail is a long list of
/// requirements one document each failed, which ranks nothing.
const MOST_REFUSED: usize = 10;

/// Every loss authorised, which is what makes one of this sweep's two runs a list of *gaps*.
///
/// A loss nobody authorised is a question put to a user rather than something the converter
/// cannot do, and a run that left them unauthorised would rank
/// `doc/pdf-a-conversion-limits.md`'s section 3 beside its section 5 — a document waiting for an
/// answer beside one no answer reaches.
///
/// **Both runs are reported, because the two numbers answer different questions and one of them
/// silently replacing the other would misstate the converter.** The authorised run says what this
/// converter can reach; the default run says what a person who types the command and answers
/// nothing actually gets, and that is the number a release note would have to carry. Session 952
/// moved this sweep from the default to the authorised run and the converted count went from 98
/// to 405 at PDF/A-2b — most of which is the *question* being answered rather than a gap being
/// closed.
fn authorise_everything() -> Authorisations {
    let mut authorised = Authorisations::default();
    for loss in Loss::ALL {
        authorised.authorise(loss);
    }
    authorised
}

/// The corpus root, or `None` where the submodule is not checked out.
fn corpus() -> Option<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/veraPDF-corpus");
    root.is_dir().then_some(root)
}

/// Every PDF under one of the corpus's directories, in a stable order.
fn documents(root: &Path, part: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    collect(&root.join(part), &mut out);
    out.sort();
    out
}

/// One directory, recursively.
fn collect(directory: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, out);
        } else if path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("pdf"))
        {
            out.push(path);
        }
    }
}

/// What one target's sweep found.
#[derive(Debug, Default)]
struct Tally {
    /// Documents the validator found conforming before the conversion.
    conforming: usize,
    /// Of those, how many converted into a file that still conforms.
    stayed: usize,
    /// Documents the validator found failing that this verb nonetheless converted.
    converted: usize,
    /// Documents refused by name.
    refused: usize,
    /// Documents whose bytes this tree does not open at all, which is not this verb's business.
    unreadable: usize,
    /// For each requirement that stopped a conversion, how many documents it stopped.
    ///
    /// **The next slice's work list, counted rather than guessed.** A refusal is by name, so the
    /// name is what says where the converter's remaining gaps are and how much each is worth;
    /// `CLAUDE.md`'s rule about derived facts is why this is printed by the sweep rather than
    /// written down anywhere.
    refusals: BTreeMap<&'static str, usize>,
    /// How each stream that keeps its data outside the file says where those bytes are.
    ///
    /// ISO 19005-2 section 6.1.7.1 and ISO 19005-4 section 6.1.6.1 are answered by embedding the
    /// bytes, and who may resolve them depends on which form \u{a7}7.11 the producer used: a
    /// name beside the document is a file this program reads under `doc/adr/1155`'s rule, and
    /// \u{a7}7.11.5's URL is a fetch it does not perform. **Counted rather than written down**,
    /// because which form the world's files use is the fact that decides whether the built
    /// resolution reaches any of them (`doc/adr/1199`).
    external: BTreeMap<&'static str, usize>,
}

/// Every glyph advance a document's font dictionaries state, in one comparable list.
///
/// ISO 32000-2 §9.6.2.1's `/Widths` for a simple font and §9.7.4.3's `/W` and `/DW` for a
/// `CIDFont` — plus §9.8.1's `/MissingWidth`, which is what a simple font's unlisted codes are —
/// gathered over every object the cross-reference table reaches. Sorted rather than keyed by
/// object, because what is being asserted is that the *set of statements* is the same and not
/// that the serializer chose the same numbering.
///
/// **Only a dictionary whose `/Type` is `Font` or `FontDescriptor` is looked at**, because `/W`
/// is not only a `CIDFont`'s: §7.5.8.2 gives a cross-reference stream one too, and the
/// serializer writes a cross-reference stream where a source wrote a table.
fn every_stated_width(document: &Document) -> Vec<String> {
    let mut stated = Vec::new();
    for number in document.xref().object_numbers().collect::<Vec<_>>() {
        let object = document.get(pdf_syntax::ObjectId::new(number, 0));
        let Some(dict) = object.as_dict() else {
            continue;
        };
        let kind = document.get_key(dict, "Type");
        let kind = kind.as_name().map(|name| name.as_bytes().to_vec());
        if !matches!(kind.as_deref(), Some(b"Font" | b"FontDescriptor")) {
            continue;
        }
        for key in ["Widths", "W", "DW", "DW2", "W2", "MissingWidth"] {
            let value = document.get_key(dict, key);
            if !value.is_null() {
                stated.push(format!("{key}={:?}", document.resolve(&value)));
            }
        }
    }
    stated.sort_unstable();
    stated
}

impl Tally {
    /// Counts how each of one conversion's external streams named its data.
    fn note_external(&mut self, report: &pdf_transform::Report) {
        let Some(conversion) = &report.archive else {
            return;
        };
        for stream in &conversion.external_data {
            let seen = self.external.entry(how_it_names(&stream.data)).or_default();
            *seen = seen.saturating_add(1);
        }
    }
}

/// How one stream says where its data is, in the words the census counts.
fn how_it_names(data: &ExternalData) -> &'static str {
    match data {
        ExternalData::Named {
            components,
            absolute,
            ..
        } if a_plain_name(components, *absolute) => {
            "a plain file name, which this program's own rule would read"
        }
        ExternalData::Named { .. } => {
            "a file specification of more than one component, or an absolute one"
        }
        ExternalData::AtUrl(_) => "a URL",
        ExternalData::NoneNamed => "no file at all, so the keys alone go",
        ExternalData::Unreadable => "a file specification this reader cannot read",
    }
}

/// `doc/adr/1155`'s rule, as a caller of this library applies it.
///
/// One path component and nothing else, so `../secrets`, `/etc/passwd` and a drive-relative name
/// are refused by the same check. The census asks it because *which form the world's producers
/// used* is what decides whether the built resolution reaches any of them; the rule itself is the
/// caller's, which is why it is stated at a caller rather than inside the conversion.
fn a_plain_name(components: &[Vec<u8>], absolute: bool) -> bool {
    !absolute
        && matches!(components, [single]
            if !single.is_empty() && single != b"." && single != b"..")
}

/// Converts every corpus document under `part` to `target` and states the three properties.
fn sweep(root: &Path, part: &str, target: Target, authorised: Authorisations) -> Tally {
    let mut tally = Tally::default();
    for path in documents(root, part) {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(document) = Document::open_with_limits(bytes.clone(), Limits::DEFAULT) else {
            tally.unreadable = tally.unreadable.saturating_add(1);
            continue;
        };
        let conformed = pdf_archive::check(&document, target).verdict() == Verdict::Conforms;
        drop(document);

        let kept = bytes.clone();
        let sinks = MemorySinks::new();
        let report = apply(
            &Plan::Archive(ArchivePlan {
                source: 0,
                names: "out.pdf".parse().expect("a pattern"),
                target,
                authorised,
                profile: None,
                substitute_fonts: true,
                supplied_fonts: BTreeMap::new(),
                departures: Vec::new(),
                claim_conformance: false,
                derivations: Vec::new(),
                supplies: Vec::new(),
                preservations: Vec::new(),
                resolutions: Vec::new(),
                tool_outputs: ToolOutputs::new(),
                external_data: BTreeMap::new(),
            }),
            &[Source::new(bytes)],
            &sinks,
            &Policy::default(),
            &Budget::default(),
        );
        // Property 2: a conversion refuses by name or writes a file. An `Err` here is this verb
        // failing to read a document the line above read.
        let report = report.unwrap_or_else(|refusal| {
            panic!(
                "{}: the conversion errored rather than refusing: {refusal}",
                path.display()
            )
        });
        let output = sinks.into_outputs().pop().map(|(_, bytes)| bytes);

        match (conformed, output) {
            // Property 1, both halves: nothing was decided, and what came out still conforms.
            (true, Some(output)) => {
                tally.conforming = tally.conforming.saturating_add(1);
                let conversion = report.archive.as_ref().expect("a conversion is reported");
                assert!(
                    conversion.decided.is_empty(),
                    "{}: already conforming, and the converter decided {:?}",
                    path.display(),
                    conversion.decided
                );
                let held = Document::open_with_limits(output, Limits::DEFAULT)
                    .expect("the converted document re-opens");
                assert_eq!(
                    pdf_archive::check(&held, target).verdict(),
                    Verdict::Conforms,
                    "{}: conformed before the conversion and not after",
                    path.display()
                );
                tally.stayed = tally.stayed.saturating_add(1);
            }
            (true, None) => panic!(
                "{}: conformed already and no file was written",
                path.display()
            ),
            (false, Some(output)) => {
                // **Property 3: no glyph moves.** `doc/pdf-a-conversion-limits.md` section 4.9
                // sets out three ways to make a font dictionary's widths agree with its
                // program's and calls the third — restating the dictionary — *never*, because
                // ISO 32000-2 §9.2.4 makes those numbers what positions the glyphs. A converter
                // that took it would move every line of every page it touched, and nothing in
                // the report would say so. So it is checked where it can be checked absolutely:
                // every width every font in the output states is the width the source stated.
                let held = Document::open_with_limits(output, Limits::DEFAULT)
                    .expect("the converted document re-opens");
                let source =
                    Document::open_with_limits(kept, Limits::DEFAULT).expect("the source re-opens");
                assert_eq!(
                    every_stated_width(&held),
                    every_stated_width(&source),
                    "{}: a width the file states was rewritten, which moves the text",
                    path.display()
                );
                tally.converted = tally.converted.saturating_add(1);
            }
            (false, None) => {
                tally.refused = tally.refused.saturating_add(1);
                let conversion = report.archive.as_ref().expect("a conversion is reported");
                for decided in &conversion.decided {
                    if matches!(
                        decided.decision,
                        Decision::Refused(_) | Decision::Unauthorised { .. }
                    ) {
                        let seen = tally.refusals.entry(decided.requirement).or_default();
                        *seen = seen.saturating_add(1);
                    }
                }
            }
        }
        tally.note_external(&report);
    }
    tally
}

#[test]
#[ignore = "needs doc/veraPDF-corpus, which is 239 MB and not part of a checkout"]
fn a_conforming_document_stays_conforming_and_nothing_errors() {
    let Some(root) = corpus() else {
        println!("doc/veraPDF-corpus is not here; nothing to sweep");
        return;
    };
    let mut converted = 0_usize;
    let nothing_authorised = Authorisations::default();
    let everything = authorise_everything();
    for (part, target) in TARGETS {
        // Two runs, because they answer different questions. The default one is what a person who
        // types the command and answers nothing gets; the authorised one is what this converter
        // can reach, and only its refusals rank a *gap*.
        let plain = sweep(&root, part, target, nothing_authorised);
        let tally = sweep(&root, part, target, everything);
        println!(
            "archive {target}: {} conforming, {} of them still conforming after the conversion; \
             {} failing documents converted, {} refused by name, {} unreadable",
            tally.conforming, tally.stayed, tally.converted, tally.refused, tally.unreadable
        );
        println!(
            "    answering nothing, which is what a user gets by default: {} converted, \
             {} refused",
            plain.converted, plain.refused
        );
        assert_eq!(
            plain.conforming, plain.stayed,
            "a conforming document stays conforming when no loss is authorised either"
        );
        assert_eq!(
            tally.conforming, tally.stayed,
            "every document that conformed before the conversion conforms after it"
        );
        let mut ranked: Vec<_> = tally.refusals.iter().collect();
        ranked.sort_by_key(|(id, count)| (std::cmp::Reverse(**count), **id));
        for (id, count) in ranked.iter().take(MOST_REFUSED) {
            println!("    {count:>4} refused on {id}");
        }
        for (form, count) in &tally.external {
            println!("    {count:>4} stream(s) keep their data outside the file, named as {form}");
        }
        converted = converted.saturating_add(tally.converted);
    }
    assert!(
        converted > 0,
        "the corpus holds documents these rewrites fix, and none was fixed"
    );
}

/// What the preserve/relocation census found at one target.
///
/// **The extension `doc/adr/1123` owes** (trap 8): the appended-page census counted marks kept;
/// this counts *where* they are kept, now that there are two answers. Its calibration is
/// `tests/archive.rs`'s three fixtures — the clean relocation, the extra `Q`, the overlap — each of
/// which plants one of these outcomes and confirms it is named.
#[derive(Debug, Default)]
struct Preserved {
    /// Documents whose forbidden-annotation marks a configured `preserve` had to place somewhere.
    documents: usize,
    /// Marks put back on the producer's own page, where §12.5.5 had them (`doc/adr/1123`).
    relocated: usize,
    /// Marks a refusal sent to an appended page instead (`doc/adr/1099`'s fallback).
    appended: usize,
    /// For each refusal sentence, how many marks it sent to an appended page.
    declined: BTreeMap<String, usize>,
}

/// Preserves every forbidden annotation's marks at `target` and tallies where each ended up.
fn preserve_sweep(root: &Path, part: &str, target: Target) -> Preserved {
    let mut census = Preserved::default();
    let sites = [
        "annotations/subtype-defined-in-iso-32000-1",
        "annotations/subtype-defined-in-iso-32000-2",
    ];
    for path in documents(root, part) {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        if Document::open_with_limits(bytes.clone(), Limits::DEFAULT).is_err() {
            continue;
        }
        let sinks = MemorySinks::new();
        let report = apply(
            &Plan::Archive(ArchivePlan {
                source: 0,
                names: "out.pdf".parse().expect("a pattern"),
                target,
                authorised: authorise_everything(),
                profile: None,
                substitute_fonts: true,
                supplied_fonts: BTreeMap::new(),
                departures: Vec::new(),
                claim_conformance: false,
                derivations: Vec::new(),
                supplies: Vec::new(),
                preservations: sites
                    .iter()
                    .map(|site| Preservation {
                        site: (*site).to_owned(),
                    })
                    .collect(),
                resolutions: Vec::new(),
                tool_outputs: ToolOutputs::new(),
                external_data: BTreeMap::new(),
            }),
            &[Source::new(bytes)],
            &sinks,
            &Policy::default(),
            &Budget::default(),
        );
        let report = report.unwrap_or_else(|refusal| {
            panic!(
                "{}: the conversion errored rather than refusing: {refusal}",
                path.display()
            )
        });
        let Some(conversion) = report.archive.as_ref() else {
            continue;
        };
        let marks: Vec<_> = conversion
            .preserved
            .iter()
            .filter(|row| sites.contains(&row.site))
            .collect();
        if marks.is_empty() {
            continue;
        }
        census.documents = census.documents.saturating_add(1);
        for row in marks {
            // A relocated row declines nothing and carries the on-page placement; a fallback names
            // its refusal. The two are exclusive, which the assertion keeps honest.
            match row.declined {
                None => {
                    assert_eq!(
                        row.placement,
                        PLACEMENT_ON_PAGE,
                        "{}: a mark that declined nothing was not relocated",
                        path.display()
                    );
                    census.relocated = census.relocated.saturating_add(1);
                }
                Some(reason) => {
                    census.appended = census.appended.saturating_add(1);
                    let seen = census.declined.entry(reason.to_owned()).or_default();
                    *seen = seen.saturating_add(1);
                }
            }
        }
    }
    census
}

#[test]
#[ignore = "needs doc/veraPDF-corpus, which is 239 MB and not part of a checkout"]
fn preserving_forbidden_annotations_relocates_where_it_can() {
    let Some(root) = corpus() else {
        println!("doc/veraPDF-corpus is not here; nothing to sweep");
        return;
    };
    let mut relocated = 0_usize;
    for (part, target) in TARGETS {
        let census = preserve_sweep(&root, part, target);
        println!(
            "preserve {target}: {} document(s) drive the relocation remedy; {} mark(s) relocated \
             onto the producer's page, {} sent to an appended page",
            census.documents, census.relocated, census.appended
        );
        let mut ranked: Vec<_> = census.declined.iter().collect();
        ranked.sort_by_key(|(reason, count)| (std::cmp::Reverse(**count), (*reason).clone()));
        for (reason, count) in ranked {
            let head: String = reason.chars().take(60).collect();
            println!("    {count:>4} appended, because {head}…");
        }
        relocated = relocated.saturating_add(census.relocated);
    }
    println!("preserve: {relocated} appearance(s) relocated onto their producer's own page in all");
}

/// What the separation-supply census found at one target.
///
/// **A census of its own, for [`preserve_sweep`]'s reason**: the two sweeps above answer what a
/// caller gets with nothing configured and with every *loss* authorised, and a configured remedy is
/// neither. `graphics/separations-of-one-name-agree` is answered by an operator naming which of a
/// colourant's definitions their archive means (`doc/adr/1188`), so what it converts is invisible to
/// a sweep that supplies nothing.
#[derive(Debug, Default)]
struct Agreed {
    /// Documents the supply converted that refused without it.
    converted: usize,
    /// Colourants whose definitions the supply decided, over those documents.
    colourants: usize,
    /// Documents that failed the requirement and still refused with the supply in hand.
    still_refused: usize,
}

/// Converts every document under `part` with the separation supply answered, and tallies it.
fn separation_sweep(root: &Path, part: &str, target: Target, winner: &str) -> Agreed {
    let mut census = Agreed::default();
    let site = "graphics/separations-of-one-name-agree";
    for path in documents(root, part) {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(document) = Document::open_with_limits(bytes.clone(), Limits::DEFAULT) else {
            continue;
        };
        let fails = pdf_archive::check(&document, target)
            .failures()
            .any(|judgement| judgement.id == site);
        drop(document);
        if !fails {
            continue;
        }
        let text = format!("[site.\"{site}\"]\nremedy = \"supply\"\nwinner = \"{winner}\"\n");
        let config = pdf_transform::archive::Configuration::read(&text, target)
            .expect("the configuration reads");
        let sinks = MemorySinks::new();
        let report = apply(
            &Plan::Archive(ArchivePlan {
                source: 0,
                names: "out.pdf".parse().expect("a pattern"),
                target,
                authorised: authorise_everything(),
                profile: None,
                substitute_fonts: true,
                supplied_fonts: BTreeMap::new(),
                departures: Vec::new(),
                claim_conformance: false,
                derivations: Vec::new(),
                supplies: config.supplies(target),
                preservations: Vec::new(),
                resolutions: Vec::new(),
                tool_outputs: ToolOutputs::new(),
                external_data: BTreeMap::new(),
            }),
            &[Source::new(bytes)],
            &sinks,
            &Policy::default(),
            &Budget::default(),
        );
        let report = report.unwrap_or_else(|refusal| {
            panic!(
                "{}: the conversion errored rather than refusing: {refusal}",
                path.display()
            )
        });
        let Some(conversion) = report.archive.as_ref() else {
            continue;
        };
        let decided = conversion
            .decided
            .iter()
            .find(|decided| decided.requirement == site);
        match decided.map(|decided| &decided.decision) {
            Some(Decision::Configured { .. }) => {
                census.converted = census.converted.saturating_add(1);
                census.colourants = census.colourants.saturating_add(
                    conversion
                        .supplied
                        .iter()
                        .filter(|row| row.site == site)
                        .count(),
                );
            }
            _ => census.still_refused = census.still_refused.saturating_add(1),
        }
        // Whatever the decision, the requirement must not survive a conversion that answered it:
        // an output still failing it would be a supply that wrote the wrong definitions.
        if let Some(output) = sinks.into_outputs().pop().map(|(_, bytes)| bytes) {
            let held =
                Document::open_with_limits(output, Limits::DEFAULT).expect("the output re-opens");
            assert!(
                !pdf_archive::check(&held, target)
                    .failures()
                    .any(|judgement| judgement.id == site),
                "{}: a file was written that still fails the requirement the supply answered",
                path.display()
            );
        }
    }
    census
}

#[test]
#[ignore = "needs doc/veraPDF-corpus, which is 239 MB and not part of a checkout"]
fn a_supplied_separation_winner_answers_the_documents_that_disagree() {
    let Some(root) = corpus() else {
        println!("doc/veraPDF-corpus is not here; nothing to sweep");
        return;
    };
    // Both words, because they can pick different definitions and each has to reach every
    // document: a word that answered fewer would be one an operator could choose and lose by.
    for winner in ["first", "most-used"] {
        for (part, target) in TARGETS {
            let census = separation_sweep(&root, part, target, winner);
            if census.converted == 0 && census.still_refused == 0 {
                continue;
            }
            println!(
                "separations {target} winner={winner}: {} document(s) converted, {} colourant(s) \
                 decided, {} still refused",
                census.converted, census.colourants, census.still_refused
            );
        }
    }
}
