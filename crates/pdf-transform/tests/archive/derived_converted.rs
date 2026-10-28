//! A `derive` remedy's second stage: the PDF the operator's program returned is converted by this
//! converter before it is attached (`doc/rfc/0014` section 5, `doc/adr/2539`).
//!
//! ISO 19005-2 section 6.8 admits an embedded file conforming to part 1 or part 2, and ISO 19005-4
//! section 6.9 one conforming to part 1, 2 or 4; what a program returns conforms to nothing anyone
//! promised. So the tool's file is converted to PDF/A-2b under a part-2 target and to PDF/A-4
//! under a part-4 one, and the archive holds that conversion's file. The stand-in program is
//! [`super::a_stand_in_converter`]: a shell script writing a known PDF.

use super::*;
use pdf_transform::archive::DerivedOutcome;

/// The embedded-file site at part 2.
const PART_TWO_SITE: &str = "embedded-files/embedded-file-is-itself-pdfa";

/// The embedded-file site at plain PDF/A-4.
const PART_FOUR_SITE: &str = "embedded-files/embedded-file-is-itself-pdfa-in-the-plain-profile";

/// A PDF/A-2b document whose name tree holds one CSV file, which section 6.8 does not admit.
fn a_part_two_document_with_an_attachment() -> Vec<u8> {
    let data = b"name,amount\ninvoice,1\n";
    let embedded = stream(
        &format!(
            "/Type /EmbeddedFile /Subtype /text#2Fcsv /Params << /Size {} >> /Length {}",
            data.len(),
            data.len()
        ),
        data,
    );
    Conforming {
        catalog: "/Names << /EmbeddedFiles << /Names [(rows.csv) 6 0 R] >> >>".to_owned(),
        objects: vec![
            "<< /Type /Filespec /F (rows.csv) /UF (rows.csv) /EF << /F 7 0 R >> >>".to_owned(),
            String::from_utf8(embedded).expect("ascii"),
        ],
        ..Conforming::part_two()
    }
    .build()
}

/// A PDF that conforms to no part as it stands: its catalog names no metadata stream, which
/// ISO 19005-2 section 6.6.2.1 and ISO 19005-4 section 6.6.2.1 both require.
fn a_pdf_without_metadata(target: Target) -> Vec<u8> {
    let fixture = match target {
        Target::Two(_) => Conforming::part_two(),
        Target::Four(_) => Conforming::default(),
    };
    Conforming {
        target,
        metadata: Packet::Unnamed,
        ..fixture
    }
    .build()
}

/// Both passes of one command: `apply` asks, the executor runs, `apply` again under `budget`.
fn both_passes(
    source: &[u8],
    plan: &mut ArchivePlan,
    budget: &Budget,
) -> (Report, Option<Vec<u8>>) {
    let once = |plan: &ArchivePlan| {
        let sinks = MemorySinks::new();
        let report = apply(
            &Plan::Archive(plan.clone()),
            &[Source::new(source.to_vec())],
            &sinks,
            &Policy::default(),
            budget,
        )
        .expect("the conversion applies");
        (report, sinks.into_outputs().pop().map(|(_, bytes)| bytes))
    };
    let (first, _) = once(plan);
    for request in &first.requested {
        let result = pdf_transform::executor::execute(request).expect("the executor runs it");
        plan.tool_outputs.insert(result);
    }
    once(plan)
}

/// The one embedded file stream an output holds, decoded, and the file specification naming it.
fn the_attachment(output: &[u8]) -> (Vec<u8>, Option<Vec<u8>>) {
    let held = Document::open_with_limits(output.to_vec(), Limits::DEFAULT).expect("it opens");
    let mut bytes = None;
    let mut relationship = None;
    for number in held.xref().object_numbers() {
        let object = held.get(ObjectId::new(number, 0));
        if let Some(stream) = object.as_stream()
            && held
                .get_key(&stream.dict, "Type")
                .as_name()
                .is_some_and(|name| name.as_bytes() == b"EmbeddedFile")
        {
            bytes = held.decoded_stream_data(stream).map(|data| data.to_vec());
        }
        if let Some(dict) = object.as_dict()
            && dict.get("EF").is_some()
        {
            relationship = held
                .get_key(dict, "AFRelationship")
                .as_name()
                .map(|name| name.as_bytes().to_vec());
        }
    }
    (bytes.expect("the embedded file stream"), relationship)
}

#[test]
fn at_part_two_the_tools_pdf_is_converted_to_part_two_before_it_is_attached() {
    let scratch = Scratch::make("derived-2b");
    let returned = a_pdf_without_metadata(Target::Two(Level::B));
    assert_ne!(
        holds(&returned, Target::Two(Level::B)).verdict(),
        Verdict::Conforms,
        "the premise: what the tool returns would not be admitted as it stands"
    );
    let script = a_stand_in_converter(&scratch.0, &returned);
    let target = Target::Two(Level::B);
    let mut plan = plan_from(&a_derive_configuration(PART_TWO_SITE, &script), target);
    let (report, output) = both_passes(
        &a_part_two_document_with_an_attachment(),
        &mut plan,
        &Budget::default(),
    );
    assert!(report.requested.is_empty(), "nothing is left to run");
    let output = output.unwrap_or_else(|| {
        panic!(
            "the document converts: {:?} {:?}",
            report.refused,
            report
                .archive
                .as_ref()
                .map(|conversion| &conversion.derived)
        )
    });
    assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
    let conversion = report.archive.as_ref().expect("a conversion report");
    let row = &conversion.derived[0];
    assert_eq!(row.outcome, DerivedOutcome::Attached);
    let converted = row
        .converted
        .as_ref()
        .expect("the tool's PDF was converted");
    assert_eq!(converted.target, "PDF/A-2b");
    assert_eq!(converted.placement, "attach");
    let (attached, _) = the_attachment(&output);
    assert_ne!(
        attached, returned,
        "the archive holds the conversion, not the tool's bytes"
    );
    assert_eq!(attached.len(), converted.bytes);
    assert_eq!(
        holds(&attached, Target::Two(Level::B)).verdict(),
        Verdict::Conforms,
        "and the attachment is itself PDF/A-2b, which section 6.8 admits"
    );
    let packet = the_packet_of(&Document::open(output).expect("it opens"));
    assert!(
        packet.contains("which this converter converted to PDF/A-2b"),
        "the file's own history says the attachment was converted: {packet}"
    );
}

#[test]
fn at_part_four_the_converted_attachment_states_it_is_an_alternative() {
    let scratch = Scratch::make("derived-4");
    let returned = a_pdf_without_metadata(Target::Four(Flavour::Plain));
    let script = a_stand_in_converter(&scratch.0, &returned);
    let target = Target::Four(Flavour::Plain);
    let mut plan = plan_from(&a_derive_configuration(PART_FOUR_SITE, &script), target);
    let (report, output) = both_passes(
        &a_part_four_document_with_an_attachment(Some("text#2Fcsv")),
        &mut plan,
        &Budget::default(),
    );
    let output = output.expect("the document converts");
    assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
    let (attached, relationship) = the_attachment(&output);
    assert_eq!(
        holds(&attached, Target::Four(Flavour::Plain)).verdict(),
        Verdict::Conforms
    );
    // ISO 32000-2 §7.11.3's Table 43: an alternative representation of content.
    assert_eq!(relationship.as_deref(), Some(&b"Alternative"[..]));
    let row = &report.archive.as_ref().expect("a report").derived[0];
    assert_eq!(
        row.converted
            .as_ref()
            .map(|converted| converted.target.as_str()),
        Some("PDF/A-4")
    );
}

#[test]
fn a_derived_pdf_carrying_an_attachment_is_the_inner_runs_and_no_second_program_runs() {
    // The plant of this round's claim: the tool returns a PDF that itself holds a file section 6.8
    // does not admit. The inner conversion carries no program, so it answers the file with what
    // the configuration answers without one — here nothing — and the outer site is refused by name.
    let scratch = Scratch::make("derived-nested");
    let script = a_stand_in_converter(&scratch.0, &a_part_two_document_with_an_attachment());
    let target = Target::Two(Level::B);
    let mut plan = plan_from(&a_derive_configuration(PART_TWO_SITE, &script), target);
    let (report, output) = both_passes(
        &a_part_two_document_with_an_attachment(),
        &mut plan,
        &Budget::default(),
    );
    assert!(
        report.requested.is_empty(),
        "no second program is asked for: {:?}",
        report.requested
    );
    assert!(output.is_none(), "and the document is refused");
    let row = &report.archive.as_ref().expect("a report").derived[0];
    let DerivedOutcome::NotConverted(sentence) = &row.outcome else {
        panic!("the tool's PDF was not taken in: {:?}", row.outcome);
    };
    assert!(
        sentence.contains(PART_TWO_SITE),
        "naming the requirement that stopped the inner run: {sentence}"
    );
    assert!(matches!(
        decision(&report, PART_TWO_SITE),
        Decision::Refused(_)
    ));
}

#[test]
fn a_derived_pdf_past_what_the_budget_leaves_is_refused_by_name_before_it_is_opened() {
    let scratch = Scratch::make("derived-budget");
    let returned = a_pdf_without_metadata(Target::Two(Level::B));
    let script = a_stand_in_converter(&scratch.0, &returned);
    let source = a_part_two_document_with_an_attachment();
    let target = Target::Two(Level::B);
    // One byte under the tool's file: every stream of the source is far below it, so the source
    // opens and only the derived file meets the bound.
    let mut budget = Budget::default();
    budget.limits.max_stream_len = returned.len().saturating_sub(1);
    let mut plan = plan_from(&a_derive_configuration(PART_TWO_SITE, &script), target);
    let (report, output) = both_passes(&source, &mut plan, &budget);
    assert!(output.is_none());
    let row = &report.archive.as_ref().expect("a report").derived[0];
    let DerivedOutcome::NotConverted(sentence) = &row.outcome else {
        panic!("refused by the budget: {:?}", row.outcome);
    };
    assert!(
        sentence.contains(&format!("the derived file is {} byte(s)", returned.len())),
        "{sentence}"
    );
    // At the bound itself the file is opened and converted.
    budget.limits.max_stream_len = 1 << 20;
    let mut plan = plan_from(&a_derive_configuration(PART_TWO_SITE, &script), target);
    let (_, output) = both_passes(&source, &mut plan, &budget);
    assert!(
        output.is_some(),
        "under a budget that holds it, it converts"
    );
}

/// `a_derive_configuration`, with the converted file's pages appended rather than attached.
fn an_appending_configuration(site: &str, script: &std::path::Path) -> String {
    a_derive_configuration(site, script).replacen(
        "on-failure = \"stop\"\n",
        "on-failure = \"stop\"\nplacement = \"append\"\n",
        1,
    )
}

/// The page count and the embedded file streams an output holds.
fn pages_and_attachments(output: &[u8]) -> (usize, usize) {
    let held = Document::open_with_limits(output.to_vec(), Limits::DEFAULT).expect("it opens");
    let attachments = held
        .xref()
        .object_numbers()
        .filter(|number| {
            held.get(ObjectId::new(*number, 0))
                .as_stream()
                .is_some_and(|stream| {
                    held.get_key(&stream.dict, "Type")
                        .as_name()
                        .is_some_and(|name| name.as_bytes() == b"EmbeddedFile")
                })
        })
        .count();
    (pdf_model::Pages::new(&held).len(), attachments)
}

#[test]
fn appended_the_converted_pdfs_pages_follow_the_documents_and_the_attachment_goes() {
    // `derive:<tool>:append` (`doc/rfc/0014` section 5's third stage): the tool's PDF is converted
    // to the outer target itself and its pages are appended, so the attachment section 6.8 and
    // section 6.9 do not admit leaves and nothing of it is lost from what a reader sees.
    for (target, site, source) in [
        (
            Target::Two(Level::B),
            PART_TWO_SITE,
            a_part_two_document_with_an_attachment(),
        ),
        (
            Target::Four(Flavour::Plain),
            PART_FOUR_SITE,
            a_part_four_document_with_an_attachment(Some("text#2Fcsv")),
        ),
    ] {
        let scratch = Scratch::make(&format!("derived-append-{target}"));
        let returned = a_pdf_without_metadata(target);
        let script = a_stand_in_converter(&scratch.0, &returned);
        let mut plan = plan_from(&an_appending_configuration(site, &script), target);
        let (report, output) = both_passes(&source, &mut plan, &Budget::default());
        let output = output.unwrap_or_else(|| {
            panic!("{target} converts: {:?}", report.refused);
        });
        assert_eq!(
            holds(&output, target).verdict(),
            Verdict::Conforms,
            "{target}"
        );
        let (source_pages, _) = pages_and_attachments(&source);
        let (returned_pages, _) = pages_and_attachments(&returned);
        let (pages, attachments) = pages_and_attachments(&output);
        assert_eq!(
            pages,
            source_pages + returned_pages,
            "{target}: the document's pages and then the derived file's, every one"
        );
        assert_eq!(attachments, 0, "{target}: the attachment is removed");
        let conversion = report.archive.as_ref().expect("a conversion report");
        let row = &conversion.derived[0];
        assert_eq!(row.outcome, DerivedOutcome::Attached);
        let converted = row.converted.as_ref().expect("converted");
        assert_eq!(converted.placement, "append");
        assert_eq!(converted.target, target.to_string());
        assert!(matches!(
            decision(&report, site),
            Decision::Configured { .. }
        ));
        let packet = the_packet_of(&Document::open(output.clone()).expect("it opens"));
        assert!(
            packet.contains("was removed, and its pages appended from"),
            "{target}: the history says the pages are derived: {packet}"
        );
        std::fs::write(scratch.0.join("out.pdf"), &output).expect("kept for veraPDF");
        if let Ok(keep) = std::env::var("DERIVED_APPEND_KEEP") {
            std::fs::write(
                format!("{keep}/append-{}.pdf", target.to_string().replace('/', "-")),
                &output,
            )
            .expect("kept");
        }
    }
}

#[test]
fn appended_into_a_tagged_document_the_marks_are_refused_by_name() {
    // The pages are marks this converter does not describe: §14.8.2.2.1 would make them artifacts
    // of a tagged document, and their structure is the program's, so the conversion refuses.
    let scratch = Scratch::make("derived-append-tagged");
    let target = Target::Four(Flavour::Plain);
    let returned = a_pdf_without_metadata(target);
    let script = a_stand_in_converter(&scratch.0, &returned);
    let data = b"name,amount\n";
    let embedded = stream(
        &format!(
            "/Type /EmbeddedFile /Subtype /text#2Fcsv /Params << /Size {} >> /Length {}",
            data.len(),
            data.len()
        ),
        data,
    );
    let source = Conforming {
        catalog: "/MarkInfo << /Marked true >> \
                  /Names << /EmbeddedFiles << /Names [(rows.csv) 6 0 R] >> >> /AF [6 0 R]"
            .to_owned(),
        objects: vec![
            "<< /Type /Filespec /F (rows.csv) /UF (rows.csv) /AFRelationship /Data \
             /EF << /F 7 0 R >> >>"
                .to_owned(),
            String::from_utf8(embedded).expect("ascii"),
        ],
        ..Conforming::default()
    }
    .build();
    let mut plan = plan_from(&an_appending_configuration(PART_FOUR_SITE, &script), target);
    let (report, output) = both_passes(&source, &mut plan, &Budget::default());
    assert!(
        output.is_none(),
        "a tagged document takes no pages of marks"
    );
    let Decision::Refused(because) = decision(&report, PART_FOUR_SITE) else {
        panic!("refused: {:?}", decision(&report, PART_FOUR_SITE));
    };
    assert!(
        because
            .sentence()
            .contains("the derived PDF's pages would be appended to a document that describes"),
        "{}",
        because.sentence()
    );
}

/// A derived PDF that draws: device colour under a graphics state, and an annotation whose
/// normal appearance §12.5.3's flags show, so the appended page has both kinds of mark to carry.
fn a_drawn_pdf_without_metadata(target: Target) -> Vec<u8> {
    let fixture = match target {
        Target::Two(_) => Conforming::part_two(),
        Target::Four(_) => Conforming::default(),
    };
    let appearance = stream(
        "/Type /XObject /Subtype /Form /BBox [0 0 100 100] /Length 18",
        b"0 G 0 0 100 100 re S",
    );
    Conforming {
        target,
        metadata: Packet::Unnamed,
        page: "/Annots [6 0 R]".to_owned(),
        resources: "/ExtGState << /G0 << /Type /ExtGState /CA 0.5 >> >>".to_owned(),
        contents: Some((
            String::new(),
            b"q /G0 gs 0 0 1 rg 10 10 50 50 re f Q".to_vec(),
        )),
        objects: vec![
            "<< /Type /Annot /Subtype /Square /Rect [100 100 200 200] /F 4 /AP << /N 7 0 R >> >>"
                .to_owned(),
            String::from_utf8(appearance).expect("ascii"),
        ],
        ..fixture
    }
    .build()
}

#[test]
fn appended_a_derived_page_carries_its_content_and_its_annotations_appearances() {
    for (target, site, source) in [
        (
            Target::Two(Level::B),
            PART_TWO_SITE,
            a_part_two_document_with_an_attachment(),
        ),
        (
            Target::Four(Flavour::Plain),
            PART_FOUR_SITE,
            a_part_four_document_with_an_attachment(Some("text#2Fcsv")),
        ),
    ] {
        let scratch = Scratch::make(&format!("derived-append-drawn-{target}"));
        let returned = a_drawn_pdf_without_metadata(target);
        let script = a_stand_in_converter(&scratch.0, &returned);
        let mut plan = plan_from(&an_appending_configuration(site, &script), target);
        let (report, output) = both_passes(&source, &mut plan, &Budget::default());
        let output = output.unwrap_or_else(|| {
            panic!("{target} converts: {:?}", report.refused);
        });
        if let Ok(keep) = std::env::var("DERIVED_APPEND_KEEP") {
            std::fs::write(
                format!("{keep}/drawn-{}.pdf", target.to_string().replace('/', "-")),
                &output,
            )
            .expect("kept");
        }
        assert_eq!(
            holds(&output, target).verdict(),
            Verdict::Conforms,
            "{target}"
        );
        let held = Document::open(output).expect("it opens");
        let tree = pdf_model::Pages::new(&held);
        let last = tree.get(tree.len() - 1).expect("the appended page");
        assert!(
            last.dict.get("Annots").is_none(),
            "{target}: the appended page states no annotation of its own"
        );
        // The page invokes one form; that form invokes the derived page and the appearance.
        let wrapper = held.get_key(&last.resources, "XObject");
        let wrapper = wrapper.as_dict().expect("the page's forms");
        let (_, wrapper) = wrapper.iter().next().expect("one form");
        let wrapper = held.resolve(wrapper);
        let wrapper = wrapper.as_stream().expect("a form");
        let inner = held.get_key(&wrapper.dict, "Resources");
        let inner = held.get_key(inner.as_dict().expect("resources"), "XObject");
        assert_eq!(
            inner.as_dict().map(|forms| forms.iter().count()),
            Some(2),
            "{target}: the derived page and its annotation's appearance"
        );
        let drawn = held.decoded_stream_data(wrapper).expect("it decodes");
        let drawn = String::from_utf8_lossy(&drawn).into_owned();
        // §12.5.5: BBox [0 0 100 100] onto Rect [100 100 200 200] is a translation by 100.
        assert!(
            drawn.contains("1 0 0 1 100 100 cm"),
            "{target}: placed where §12.5.5 puts it: {drawn}"
        );
    }
}
