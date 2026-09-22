//! ISO 32000-2 §12.7.8's Table 253 `/F` and §12.6.4.7's Table 209 `/F`: two more files a
//! document names, asked for the way Table 203's already was.
//!
//! Both entries say the same thing in the same shape. Table 253: "[t]he file containing the named
//! page. If this entry is absent, it shall be assumed that the page resides in the associated PDF
//! file." Table 209: "[t]he file containing the thread. If this entry is absent, the thread is in
//! the current file." So the presence of the entry is the whole test, and what its presence means
//! is a second PDF — which `doc/ui-boundary.md`'s rule 2 keeps out of this crate and `CLAUDE.md`
//! principle 3 keeps away from the process that parses one.
//!
//! The named page is the harder of the two and is why this file exists: it is a **second** host
//! question raised while §12.7.6.4's first is being applied, so the assertions below are about
//! ordering as much as about content — the import answers, and only then does the next question
//! go out. Every one of them excludes a named wrong answer: a page drawn from the document that
//! asked rather than from the one that arrived, an appearance left standing after a decline, a
//! thread resolved against the source's own `/Threads`.

use viewer_core::{Command, DocumentId, Event, Purpose, Viewer};

/// The document under test.
const DOCUMENT: DocumentId = DocumentId(1);

/// Assembles a document from object bodies given as bytes, with object 1 the catalog.
fn assembled(objects: &[Vec<u8>]) -> Vec<u8> {
    let mut out: Vec<u8> = b"%PDF-2.0\n".to_vec();
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", index.saturating_add(1)).as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref_at = out.len();
    let size = objects.len().saturating_add(1);
    out.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
    for offset in &offsets {
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!("trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n")
            .as_bytes(),
    );
    out
}

/// Every sentence an event list carries, in order.
fn notes(events: &[Event]) -> Vec<String> {
    events
        .iter()
        .flat_map(|event| match event {
            Event::Reported { notes, .. } => notes.clone(),
            _ => Vec::new(),
        })
        .collect()
}

/// Every file an event list asked for, with the purpose that asked.
fn asked_for(events: &[Event]) -> Vec<(Purpose, String)> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::NeedsFile { purpose, name, .. } => Some((*purpose, name.clone())),
            _ => None,
        })
        .collect()
}

/// A form with one push button, and a link whose action imports §12.7.8's form data.
fn form() -> Vec<u8> {
    assembled(&[
        b"<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] \
          /DR << /Font << /Helv 6 0 R >> >> >> >>"
            .to_vec(),
        b"<< /Type /Pages /Count 1 /Kids [3 0 R] >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Annots [4 0 R 5 0 R] >>".to_vec(),
        b"<< /Type /Annot /Subtype /Widget /Rect [20 40 180 70] /F 4 /FT /Btn /Ff 65536 \
          /T (press) /DA (/Helv 12 Tf 0 g) >>"
            .to_vec(),
        b"<< /Type /Annot /Subtype /Link /Rect [0 0 20 20] \
          /A << /Type /Action /S /ImportData /F (data.fdf) >> >>"
            .to_vec(),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"
            .to_vec(),
    ])
}

/// §12.7.8's form data, whose one field's `/APRef` names a page in a second file.
fn data() -> Vec<u8> {
    b"%FDF-1.2\n1 0 obj\n<< /FDF << /Fields [ << /T (press) /APRef \
      << /N << /Name (stamp) /F (library.pdf) >> >> >> ] >> >>\nendobj\n\
      trailer\n<< /Root 1 0 R >>\n%%EOF\n"
        .to_vec()
}

/// The second file: §12.7.7's `/Templates` tree names one page, which carries visible marks.
fn library() -> Vec<u8> {
    let marks = b"BT /Helv 12 Tf 0 g 2 8 Td (elsewhere) Tj ET\n".to_vec();
    assembled(&[
        b"<< /Type /Catalog /Pages 2 0 R \
          /Names << /Templates << /Names [(stamp) 4 0 R] >> >> >>"
            .to_vec(),
        b"<< /Type /Pages /Count 1 /Kids [3 0 R] >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] >>".to_vec(),
        b"<< /Type /Template /MediaBox [0 0 160 30] \
          /Resources << /Font << /Helv 6 0 R >> >> /Contents 5 0 R >>"
            .to_vec(),
        {
            let mut stream = format!("<< /Length {} >>\nstream\n", marks.len()).into_bytes();
            stream.extend_from_slice(&marks);
            stream.extend_from_slice(b"endstream");
            stream
        },
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"
            .to_vec(),
    ])
}

/// Opens the form, clicks the link and answers §12.7.6.4's question with [`data`].
fn importing() -> (Viewer, Vec<Event>) {
    let mut viewer = Viewer::new(400, 400, 1.0);
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: form().into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    viewer
        .handle(Command::Activate(pdf_syntax::ObjectId::new(5, 0)))
        .for_each(drop);
    let events = viewer
        .handle(Command::Supply {
            purpose: Purpose::ImportData,
            bytes: Some(data()),
        })
        .collect();
    (viewer, events)
}

/// The second question is raised by applying the answer to the first.
#[test]
fn a_named_page_in_another_file_is_asked_for_while_the_import_is_applied() {
    let (_, events) = importing();
    assert_eq!(
        asked_for(&events),
        [(Purpose::NamedPage, "library.pdf".to_owned())],
        "the name is the FDF file's own words, and the purpose is not the import's"
    );
    let said = notes(&events);
    assert!(
        said.iter().any(|note| note.contains("waiting")
            && note.contains("library.pdf")
            && note.contains("/APRef /N")),
        "the person is told what is being asked for and why: {said:?}"
    );
    assert!(
        !said.iter().any(|note| note.contains("declined")),
        "nothing is refused yet: {said:?}"
    );
}

/// The supplied file's page becomes the button's appearance, said out loud.
#[test]
fn a_supplied_file_gives_the_button_its_appearance() {
    let (mut viewer, _) = importing();
    let events: Vec<Event> = viewer
        .handle(Command::Supply {
            purpose: Purpose::NamedPage,
            bytes: Some(library()),
        })
        .collect();
    let said = notes(&events);
    assert!(
        said.iter()
            .any(|note| note.contains("library.pdf gave 1 appearance(s) and 0 page(s)")),
        "{said:?}"
    );
    assert!(
        !said.iter().any(|note| note.contains("declined")),
        "{said:?}"
    );
    assert!(
        asked_for(&events).is_empty(),
        "nothing is left outstanding, so no further question goes out"
    );
}

/// A host that supplies nothing leaves the button its own artwork and says which button.
#[test]
fn a_file_nobody_supplies_is_named_rather_than_forgotten() {
    let (mut viewer, _) = importing();
    let events: Vec<Event> = viewer
        .handle(Command::Supply {
            purpose: Purpose::NamedPage,
            bytes: None,
        })
        .collect();
    let said = notes(&events);
    assert!(
        said.iter().any(|note| note.contains("declined")
            && note.contains("library.pdf")
            && note.contains("was not supplied")),
        "{said:?}"
    );
    assert!(
        asked_for(&events).is_empty(),
        "one file was declined and there was no second"
    );
}

/// Bytes that are not a PDF are a refusal with the file named, not a silent nothing.
#[test]
fn a_supplied_file_that_is_not_a_pdf_is_named() {
    let (mut viewer, _) = importing();
    let events: Vec<Event> = viewer
        .handle(Command::Supply {
            purpose: Purpose::NamedPage,
            bytes: Some(b"not a PDF at all".to_vec()),
        })
        .collect();
    let said = notes(&events);
    assert!(
        said.iter()
            .any(|note| note.contains("cannot read library.pdf")),
        "{said:?}"
    );
    assert!(
        said.iter()
            .any(|note| note.contains("declined") && note.contains("/APRef /N")),
        "and the reference it was for is named too: {said:?}"
    );
}

/// A source document whose link follows §12.6.4.7's thread into another file.
fn threading(action: &str) -> Vec<u8> {
    assembled(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Count 1 /Kids [3 0 R] >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Annots [4 0 R] >>".to_vec(),
        format!("<< /Type /Annot /Subtype /Link /Rect [0 0 50 50] /A {action} >>").into_bytes(),
    ])
}

/// The second file: three pages and one §12.4.3 thread whose first bead is on the third.
fn articles() -> Vec<u8> {
    let mut objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R /Threads [6 0 R] >>".to_vec(),
        b"<< /Type /Pages /Count 3 /Kids [3 0 R 4 0 R 5 0 R] >>".to_vec(),
    ];
    for _ in 0..3 {
        objects.push(b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] >>".to_vec());
    }
    objects.push(b"<< /I << /Title (the article) >> /F 7 0 R >>".to_vec());
    objects.push(b"<< /T 6 0 R /P 5 0 R /R [10 10 90 90] /N 7 0 R /V 7 0 R >>".to_vec());
    assembled(&objects)
}

/// Opens the source and clicks the link that names a thread in another file.
fn threaded(action: &str) -> (Viewer, Vec<Event>) {
    let mut viewer = Viewer::new(400, 400, 1.0);
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: threading(action).into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    let events = viewer
        .handle(Command::Activate(pdf_syntax::ObjectId::new(4, 0)))
        .collect();
    (viewer, events)
}

/// Table 209's `/F` asks the host for a file, under its own purpose.
#[test]
fn the_file_a_thread_action_names_is_asked_of_the_host() {
    let (_, events) = threaded("<< /Type /Action /S /Thread /F (articles.pdf) /D 0 >>");
    assert_eq!(
        asked_for(&events),
        [(Purpose::ThreadDocument, "articles.pdf".to_owned())],
        "the purpose says which clause wants it, so a host names the right one to a person"
    );
    assert!(notes(&events).is_empty(), "{:?}", notes(&events));
}

/// The thread is resolved in the file that arrived, at the page its bead is on.
#[test]
fn a_supplied_file_is_opened_at_the_bead_the_thread_names() {
    let (mut viewer, _) = threaded("<< /Type /Action /S /Thread /F (articles.pdf) /D 0 >>");
    let events: Vec<Event> = viewer
        .handle(Command::Supply {
            purpose: Purpose::ThreadDocument,
            bytes: Some(articles()),
        })
        .collect();
    let opened: Vec<usize> = events
        .iter()
        .filter_map(|event| match event {
            Event::Opened { pages, .. } => Some(*pages),
            _ => None,
        })
        .collect();
    assert_eq!(opened, [3], "the document that arrived is the one now open");
    let turned: Vec<usize> = events
        .iter()
        .filter_map(|event| match event {
            Event::PageChanged { index, .. } => Some(*index),
            _ => None,
        })
        .collect();
    assert_eq!(
        turned.last(),
        Some(&2),
        "§12.4.3's bead names its page through Table 163's `/P`, and this one is the third of \
         the file that arrived — a page the source document does not have at all: {turned:?}"
    );
    assert!(
        notes(&events)
            .iter()
            .any(|note| note.contains("opened articles.pdf") && note.contains("at page 3")),
        "{:?}",
        notes(&events)
    );
}

/// A host that supplies nothing says so rather than leaving the click reporting nothing.
#[test]
fn a_thread_file_nobody_supplies_is_named() {
    let (mut viewer, _) = threaded("<< /Type /Action /S /Thread /F (articles.pdf) /D 0 >>");
    let events: Vec<Event> = viewer
        .handle(Command::Supply {
            purpose: Purpose::ThreadDocument,
            bytes: None,
        })
        .collect();
    assert!(
        notes(&events)
            .iter()
            .any(|note| note.contains("Thread: articles.pdf was not supplied")),
        "{:?}",
        notes(&events)
    );
}

/// §7.11.2.2's forbidden relative URL asks for nothing, because asking would mean resolving it.
///
/// The clause limits a URL-based relative specification to "paths as defined in Internet RFC
/// 3986", and a query is one of the sections it names — so this one is refused before anything is
/// asked of a host, which is the same reading §12.6.4.3 is answered with.
#[test]
fn a_thread_whose_file_is_a_forbidden_relative_url_asks_for_nothing() {
    let (_, events) = threaded(
        "<< /Type /Action /S /Thread /D 0 \
         /F << /FS /URL /F (articles.pdf?take=everything) >> >>",
    );
    assert!(
        asked_for(&events).is_empty(),
        "nothing is asked for: {:?}",
        asked_for(&events)
    );
    assert!(
        notes(&events)
            .iter()
            .any(|note| note.contains("Thread") && note.contains("§7.11.2.2")),
        "{:?}",
        notes(&events)
    );
}
