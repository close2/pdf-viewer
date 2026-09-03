//! A file open on disk reads as the same file held in memory, object for object — and what a
//! reader that seeks must refuse.
//!
//! ADR 0809's whole argument is that the interpretation is a function of the bytes alone,
//! whichever way they are held: a parser over a window is given the same slice it would see from
//! the offset in a whole file, and the window is accepted only where the parse examined nothing
//! at its end. These tests are that argument run rather than stated. The first two open every
//! document both ways and compare every object the cross-reference table names; the rest are
//! the hostile shapes a seeking reader meets — an offset past the end, a stated length that runs
//! off the file, a `/Prev` chain that loops across two sections, an object longer than a window —
//! each pinned to the answer the in-memory reader gives, which the rest of this crate's tests
//! pin to the standard.

#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::print_stdout,
    reason = "test code: an explanatory panic is the intended failure, and the walk's output \
              is worth reading"
)]

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use pdf_syntax::{Document, FileBytes, ObjectId, SyntaxResult};

/// The specification PDFs shipped in `doc/`.
fn shipped() -> Vec<PathBuf> {
    let doc = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&doc)
        .expect("doc/ is readable")
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|extension| extension == "pdf"))
        .collect();
    files.sort();
    files
}

/// A directory of this test's own, named after the process so parallel rounds cannot share it.
fn scratch(name: &str) -> PathBuf {
    let directory =
        std::env::temp_dir().join(format!("pdf-syntax-on-disk-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&directory).expect("a temporary directory");
    directory
}

/// Opens `path` both ways and compares everything the two documents say.
///
/// Returns how many objects were compared, or the shared refusal where neither opened.
fn agree(path: &Path) -> Result<usize, String> {
    let memory = Document::open(pdf_syntax::read_file(path).expect("readable"));
    let disk = Document::open(FileBytes::on_disk(path).expect("opens on disk"));
    let (memory, disk) = match (memory, disk) {
        (Ok(memory), Ok(disk)) => (memory, disk),
        (Err(from_memory), Err(from_disk)) => {
            assert_eq!(
                from_memory,
                from_disk,
                "{}: the refusals differ",
                path.display()
            );
            return Err(from_memory.to_string());
        }
        (memory, disk) => panic!(
            "{}: opened one way and not the other — in memory {:?}, on disk {:?}",
            path.display(),
            memory.as_ref().err(),
            disk.as_ref().err()
        ),
    };
    assert_eq!(
        memory.trailer(),
        disk.trailer(),
        "{}: the trailers differ",
        path.display()
    );
    assert_eq!(
        memory.was_recovered(),
        disk.was_recovered(),
        "{}",
        path.display()
    );
    assert_eq!(memory.catalog(), disk.catalog(), "{}", path.display());
    let numbers: Vec<u32> = memory.xref().object_numbers().collect();
    assert_eq!(
        numbers,
        disk.xref().object_numbers().collect::<Vec<u32>>(),
        "{}: the tables name different objects",
        path.display()
    );
    for number in &numbers {
        let id = ObjectId::new(*number, 0);
        assert_eq!(
            memory.get(id),
            disk.get(id),
            "{}: object {number} differs",
            path.display()
        );
    }
    assert_eq!(
        memory.misfiled_objects(),
        disk.misfiled_objects(),
        "{}",
        path.display()
    );
    assert!(disk.scan_refused().is_none(), "{}", path.display());
    assert!(
        disk.bytes().read_failure().is_none(),
        "{}: {:?}",
        path.display(),
        disk.bytes().read_failure()
    );
    Ok(numbers.len())
}

/// Every document in `doc/`, both ways, every object.
#[test]
fn every_shipped_document_reads_the_same_on_disk_as_in_memory() {
    let mut compared = 0usize;
    for path in shipped() {
        match agree(&path) {
            Ok(objects) => compared = compared.saturating_add(objects),
            Err(refusal) => println!("{}: refused both ways: {refusal}", path.display()),
        }
    }
    assert!(compared > 100_000, "{compared} objects compared");
}

/// The whole pdf.js corpus, both ways, every object. Not a gate: run it after a change to how
/// the file is read, with `-- --ignored`.
#[test]
#[ignore = "walks the pdf.js corpus; run by hand after a change to how the file is read"]
fn every_corpus_document_reads_the_same_on_disk_as_in_memory() {
    let corpus = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/pdf.js/test/pdfs");
    let Ok(entries) = std::fs::read_dir(&corpus) else {
        println!("{} is not checked out; nothing walked", corpus.display());
        return;
    };
    let mut paths: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|extension| extension == "pdf"))
        .collect();
    paths.sort();
    let (mut documents, mut objects, mut refused) = (0usize, 0usize, 0usize);
    for path in &paths {
        match agree(path) {
            Ok(compared) => {
                documents = documents.saturating_add(1);
                objects = objects.saturating_add(compared);
            }
            Err(_) => refused = refused.saturating_add(1),
        }
    }
    println!(
        "{documents} documents agree on {objects} objects; {refused} refused both ways the same"
    );
    assert!(documents > 900, "{documents} documents agreed");
}

/// A file assembled from objects, with a cross-reference table whose entries `place` may bend.
fn assembled(objects: &[&[u8]], place: impl Fn(usize, usize) -> usize) -> Vec<u8> {
    let mut file = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for object in objects {
        offsets.push(file.len());
        file.extend_from_slice(object);
    }
    let xref_at = file.len();
    let mut table = format!(
        "xref\n0 {}\n0000000000 65535 f \n",
        objects.len().saturating_add(1)
    );
    for (index, offset) in offsets.iter().enumerate() {
        let _ = writeln!(table, "{:010} 00000 n ", place(index, *offset));
    }
    let _ = write!(
        table,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n",
        objects.len().saturating_add(1)
    );
    file.extend_from_slice(table.as_bytes());
    file
}

const CATALOG: &[u8] = b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n";
const PAGES: &[u8] = b"2 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\n";

/// Writes `bytes` to a file of its own and opens them both ways.
fn both_ways(name: &str, bytes: &[u8]) -> (SyntaxResult<Document>, SyntaxResult<Document>) {
    let directory = scratch(name);
    let path = directory.join(format!("{name}.pdf"));
    std::fs::write(&path, bytes).expect("the fixture is written");
    let memory = Document::open(bytes.to_vec());
    let disk = Document::open(FileBytes::on_disk(&path).expect("opens on disk"));
    (memory, disk)
}

/// §7.5.4's offset pointing past the file names nothing, and the object is found by its own
/// header — a scan, which reads the file on disk whole — exactly as in memory.
#[test]
fn an_offset_past_the_end_reads_the_same_on_disk_as_in_memory() {
    let bytes = assembled(&[CATALOG, PAGES], |index, offset| {
        if index == 1 { 99_999_999 } else { offset }
    });
    let (memory, disk) = both_ways("past-the-end", &bytes);
    let (memory, disk) = (memory.expect("opens"), disk.expect("opens"));
    let pages = ObjectId::new(2, 0);
    assert_eq!(memory.get(pages), disk.get(pages));
    assert!(disk.get(pages).as_dict().is_some(), "found by its header");
    assert_eq!(memory.misfiled_objects(), disk.misfiled_objects());
    assert_eq!(disk.misfiled_objects(), vec![2]);
    assert!(disk.scan_refused().is_none());
}

/// A `/Length` that runs off the file is delimited by the `endstream` search, both ways; a
/// window shorter than the stated length is grown rather than believed.
#[test]
fn a_length_that_runs_off_the_file_reads_the_same_on_disk_as_in_memory() {
    let mut stream = b"3 0 obj\n<< /Length 6000000000 >>\nstream\n".to_vec();
    stream.extend(std::iter::repeat_n(b'q', 10_000));
    stream.extend_from_slice(b"\nendstream\nendobj\n");
    let bytes = assembled(&[CATALOG, PAGES, &stream], |_, offset| offset);
    let (memory, disk) = both_ways("length-off-the-file", &bytes);
    let (memory, disk) = (memory.expect("opens"), disk.expect("opens"));
    let id = ObjectId::new(3, 0);
    assert_eq!(memory.get(id), disk.get(id));
    let data = disk.get(id);
    let data = data.as_stream().expect("a stream").data.clone();
    assert_eq!(
        data.len(),
        10_000,
        "delimited by `endstream`, not by the number"
    );
    assert!(data.iter().all(|byte| *byte == b'q'));
}

/// A stream longer than a window, with a right `/Length`, is read whole through a window that
/// the parser's own statement of what it needed has grown.
#[test]
fn a_stream_longer_than_a_window_reads_the_same_on_disk_as_in_memory() {
    let data: Vec<u8> = (0..200_000u32).map(|n| (n % 251) as u8).collect();
    let mut stream = format!("3 0 obj\n<< /Length {} >>\nstream\n", data.len()).into_bytes();
    stream.extend_from_slice(&data);
    stream.extend_from_slice(b"\nendstream\nendobj\n");
    let bytes = assembled(&[CATALOG, PAGES, &stream], |_, offset| offset);
    let (memory, disk) = both_ways("long-stream", &bytes);
    let (memory, disk) = (memory.expect("opens"), disk.expect("opens"));
    let id = ObjectId::new(3, 0);
    assert_eq!(memory.get(id), disk.get(id));
    let read = disk.get(id);
    assert_eq!(&*read.as_stream().expect("a stream").data, data.as_slice());
}

/// An object whose dictionary alone is longer than the first window — a string of many
/// kilobytes — is read whole, not cut at the window.
#[test]
fn an_object_longer_than_a_window_reads_the_same_on_disk_as_in_memory() {
    let mut long = b"3 0 obj\n<< /Note (".to_vec();
    long.extend(std::iter::repeat_n(b'n', 20_000));
    long.extend_from_slice(b") /After /TheString >>\nendobj\n");
    let bytes = assembled(&[CATALOG, PAGES, &long], |_, offset| offset);
    let (memory, disk) = both_ways("long-object", &bytes);
    let (memory, disk) = (memory.expect("opens"), disk.expect("opens"));
    let id = ObjectId::new(3, 0);
    assert_eq!(memory.get(id), disk.get(id));
    let read = disk.get(id);
    let dict = read.as_dict().expect("a dictionary");
    assert_eq!(
        dict.get("Note")
            .and_then(|note| note.as_string())
            .map(<[u8]>::len),
        Some(20_000)
    );
    assert!(
        dict.get("After").is_some(),
        "the entry after the string was read too"
    );
}

/// Two cross-reference sections whose `/Prev` entries name each other end, both ways, with the
/// same table: the cycle is detected on the offsets, which are the same numbers from a window
/// as from the whole.
#[test]
fn a_prev_loop_across_two_sections_reads_the_same_on_disk_as_in_memory() {
    let mut file = b"%PDF-1.7\n".to_vec();
    let catalog_at = file.len();
    file.extend_from_slice(CATALOG);
    let pages_at = file.len();
    file.extend_from_slice(PAGES);
    let first_at = file.len();
    // The first section names the catalogue and points back at the second.
    let second_at_guess = first_at.saturating_add(160);
    let first = format!(
        "xref\n0 2\n0000000000 65535 f \n{catalog_at:010} 00000 n \ntrailer\n<< /Size 3 /Root 1 0 R /Prev {second_at_guess} >>\n"
    );
    let mut padded = first.into_bytes();
    padded.resize(160, b' ');
    file.extend_from_slice(&padded);
    let second_at = file.len();
    assert_eq!(second_at, second_at_guess);
    // The second names the pages and points back at the first: a loop.
    let second = format!(
        "xref\n2 1\n{pages_at:010} 00000 n \ntrailer\n<< /Size 3 /Prev {first_at} >>\nstartxref\n{first_at}\n%%EOF\n"
    );
    file.extend_from_slice(second.as_bytes());

    let (memory, disk) = both_ways("prev-loop", &file);
    let (memory, disk) = (memory.expect("opens"), disk.expect("opens"));
    assert_eq!(memory.trailer(), disk.trailer());
    assert_eq!(
        memory.xref().object_numbers().collect::<Vec<u32>>(),
        disk.xref().object_numbers().collect::<Vec<u32>>()
    );
    for number in [1, 2] {
        let id = ObjectId::new(number, 0);
        assert_eq!(memory.get(id), disk.get(id));
        assert!(
            disk.get(id).as_dict().is_some(),
            "object {number} is reached"
        );
    }
    assert!(
        !disk.was_recovered(),
        "both sections were read from the chain"
    );
}
