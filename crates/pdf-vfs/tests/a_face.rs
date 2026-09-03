//! A face, in process: the four verbs a KIO worker and a FUSE daemon both reduce to.
//!
//! RFC 0003 section 7 recommends "core + FUSE face first (pure Rust, testable in this tree's own
//! harnesses, no external toolchain)", and this is that harness one step earlier: **the core
//! driven the way a face drives it**, with no kernel interface, no `/dev/fuse` and no Qt. What it
//! exercises is exactly what `kio_archive`'s base class implements — `listDir`, `stat` and `get`
//! — plus the `open`-then-`read` split a mount needs, so a face written on top of this has
//! nothing left to discover about the core.
//!
//! Three properties are worth naming because they are the round's claims rather than its
//! coverage:
//!
//! - **`cp` *is* page extraction.** `a_page_out_of_the_mount_is_the_transform_suites_own_piece`
//!   holds the bytes at `pages/0002.pdf` to the bytes `pdf_transform::apply` writes for the same
//!   page, exactly. If that ever stops being true, this crate has grown a second implementation
//!   of the thing RFC 0003 section 7 forbids it to have.
//! - **The text is the extraction identity.** RFC 0003 section 4: "a caller that greps the mount
//!   is grepping the same bytes the oracle's text gates measure".
//! - **A document that changes under the mount changes the tree, and never splices.** RFC 0003
//!   section 5.4, and it is a correctness property rather than a nicety.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "test code: a fixture that cannot exercise the rule must fail loudly"
)]

use std::sync::Arc;

use pdf_vfs::layout::{Kind, Write};
use pdf_vfs::worker::InProcessWorkers;
use pdf_vfs::{Config, MemoryBacking, Refused, Vfs, VfsError};

/// A committed document, which every checkout has once `doc/specifications.zip` is unpacked.
fn committed(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc")
        .join(name)
}

/// The five-page annex, which has an outline, §12.4.2 labels and pictures.
const FIVE_PAGES: &str = "PDF20_AN001-BPC.pdf";
/// The fourteen-page annex about associated files. Here for being a *different* document of a
/// different length, which is what the consistency tests need.
const FOURTEEN_PAGES: &str = "PDF20_AN002-AF.pdf";
/// The ten-page PDF Declarations note, which files two §7.11.4 embedded files under names
/// holding a COLON — so it is the witness for sanitisation as well as for extraction.
const WITH_ATTACHMENTS: &str = "PDF-Declarations.pdf";
/// The seventy-two-page tagged-PDF guide, whose page 35 places images and whose page 1 does not.
const WITH_IMAGES: &str = "Tagged-PDF-Best-Practice-Guide.pdf";

/// A tree over a document held in memory, so a test can replace it under the mount.
///
/// The backing is shared rather than moved, because "the file changed" has to be something the
/// test *does* to the same object the tree is reading — a second backing would be a second
/// document and would prove nothing about the generation key.
fn mounted(name: &str) -> (Arc<MemoryBacking>, Vfs) {
    let bytes = std::fs::read(committed(name)).expect("a committed document");
    let backing = Arc::new(MemoryBacking::new(name, bytes));
    let vfs = Vfs::new(
        Box::new(SharedBacking(Arc::clone(&backing))),
        Box::new(InProcessWorkers),
        Config::default(),
    );
    (backing, vfs)
}

/// A backing the test and the tree both hold, so replacing the bytes is a thing the test can do.
#[derive(Debug)]
struct SharedBacking(Arc<MemoryBacking>);

impl pdf_vfs::generation::Backing for SharedBacking {
    fn generation(&self) -> std::io::Result<pdf_vfs::generation::Generation> {
        self.0.generation()
    }
    fn bytes(&self) -> std::io::Result<pdf_syntax::FileBytes> {
        self.0.bytes()
    }
    fn describe(&self) -> String {
        self.0.describe()
    }
}

/// Every name in a listing, sorted, so an assertion is about the set rather than the order.
fn names(vfs: &Vfs, path: &str) -> Vec<String> {
    let mut out: Vec<String> = vfs
        .list(path)
        .unwrap_or_else(|error| panic!("{path}: {error}"))
        .into_iter()
        .map(|entry| entry.name)
        .collect();
    out.sort();
    out
}

#[test]
fn the_root_names_the_layouts_own_directories_and_reads_nothing_else() {
    let (_backing, vfs) = mounted(FIVE_PAGES);
    assert_eq!(
        names(&vfs, "/"),
        vec!["attachments", "images", "meta", "pages", "renders", "text"]
    );
    for entry in vfs.list("/").expect("the root lists") {
        assert_eq!(
            entry.kind,
            Kind::Directory,
            "{} is not a directory",
            entry.name
        );
    }
    // RFC 0003 section 5.1: "listing the root names six directories and reads nothing but the
    // page count".
    assert_eq!(vfs.pages().expect("a page count"), 5);
}

#[test]
fn pages_are_named_by_ordinal_at_the_documents_own_width() {
    let (_backing, vfs) = mounted(FIVE_PAGES);
    assert_eq!(
        names(&vfs, "/pages"),
        vec!["0001.pdf", "0002.pdf", "0003.pdf", "0004.pdf", "0005.pdf"]
    );
    // One spelling, and only one: the layout's width is the document's, with four as the floor.
    assert!(matches!(
        vfs.stat("/pages/1.pdf"),
        Err(VfsError::NoSuchPath(_))
    ));
    assert!(matches!(
        vfs.stat("/pages/0006.pdf"),
        Err(VfsError::NoSuchPath(_))
    ));
}

#[test]
fn a_page_out_of_the_mount_is_the_transform_suites_own_piece() {
    use pdf_transform::split::{Pieces, SplitPlan};
    use pdf_transform::{Budget, MemorySinks, Plan, Policy, Source, apply};

    let (_backing, vfs) = mounted(FIVE_PAGES);
    let from_mount = vfs.open("/pages/0002.pdf").expect("a page comes out");

    let bytes = std::fs::read(committed(FIVE_PAGES)).expect("a committed document");
    let sinks = MemorySinks::new();
    let plan = Plan::Split(SplitPlan {
        source: 0,
        pages: "2".parse().expect("a selection"),
        pieces: Pieces::EachPage,
        names: "%d".parse().expect("a pattern"),
    });
    apply(
        &plan,
        &[Source::new(bytes)],
        &sinks,
        &Policy::default(),
        &Budget::default(),
    )
    .expect("the transform writes the piece");
    let expected = sinks.into_outputs();
    let (_, piece) = expected.first().expect("one piece");

    assert_eq!(
        from_mount.bytes(),
        piece.as_slice(),
        "the mount's page is not the transform suite's piece"
    );
}

#[test]
fn a_stat_states_the_true_size_because_it_generated_the_bytes() {
    let (_backing, vfs) = mounted(FIVE_PAGES);
    for path in ["/pages/0003.pdf", "/text/0003.txt", "/meta/info.json"] {
        let attributes = vfs
            .stat(path)
            .unwrap_or_else(|error| panic!("{path}: {error}"));
        let handle = vfs
            .open(path)
            .unwrap_or_else(|error| panic!("{path}: {error}"));
        assert_eq!(attributes.kind, Kind::File, "{path}");
        assert_eq!(attributes.size, Some(handle.len()), "{path}");
        // What a kernel does with the stated size: read exactly that many bytes and get them.
        let all = handle.read(0, usize::MAX);
        assert_eq!(u64::try_from(all.len()).expect("a length"), handle.len());
    }
    let directory = vfs.stat("/pages").expect("a directory stats");
    assert_eq!(directory.kind, Kind::Directory);
    assert_eq!(directory.size, None);
}

#[test]
fn a_read_answers_at_an_offset_and_stops_at_the_end() {
    let (_backing, vfs) = mounted(FIVE_PAGES);
    let handle = vfs.open("/pages/0001.pdf").expect("a page");
    assert_eq!(handle.read(0, 5), b"%PDF-");
    let length = handle.len();
    assert_eq!(handle.read(length.saturating_sub(2), 64).len(), 2);
    assert!(handle.read(length, 1).is_empty());
    assert!(handle.read(length.saturating_add(1000), 1).is_empty());
}

#[test]
fn a_pages_text_is_the_interpreters_own_readback_byte_for_byte() {
    let (_backing, vfs) = mounted(FIVE_PAGES);
    let bytes = std::fs::read(committed(FIVE_PAGES)).expect("a committed document");
    let document = pdf_syntax::Document::open(bytes).expect("it opens");
    let pages = pdf_model::Pages::new(&document);
    let page = pages.get(2).expect("page three");
    let expected = pdf_model::interpret(&document, &page).text;

    let handle = vfs.open("/text/0003.txt").expect("the text comes out");
    assert_eq!(
        handle.bytes(),
        expected.as_bytes(),
        "the mount's text is not the extraction identity"
    );
}

#[test]
fn the_whole_documents_text_is_its_pages_joined_by_a_form_feed() {
    let (_backing, vfs) = mounted(FIVE_PAGES);
    let whole = vfs.open("/text/document.txt").expect("the concatenation");
    let mut expected: Vec<u8> = Vec::new();
    for page in 1..=5 {
        if page > 1 {
            expected.push(0x0c);
        }
        let path = format!("/text/{page:04}.txt");
        expected.extend_from_slice(vfs.open(&path).expect("a page's text").bytes());
    }
    assert_eq!(whole.bytes(), expected.as_slice());
}

#[test]
fn meta_answers_three_clauses_and_lists_xmp_only_where_the_document_states_one() {
    let (_backing, vfs) = mounted(FIVE_PAGES);
    let listed = names(&vfs, "/meta");
    assert!(listed.contains(&String::from("info.json")));
    assert!(listed.contains(&String::from("outline.json")));

    let info = vfs.open("/meta/info.json").expect("§14.3.3's dictionary");
    let text = String::from_utf8(info.bytes().to_vec()).expect("JSON is UTF-8");
    for key in [
        "title", "author", "subject", "keywords", "creator", "producer", "created", "modified",
        "trapped",
    ] {
        assert!(
            text.contains(&format!("\"{key}\"")),
            "info.json has no {key}"
        );
    }

    let outline = vfs.open("/meta/outline.json").expect("§12.3.3's outline");
    let text = String::from_utf8(outline.bytes().to_vec()).expect("JSON is UTF-8");
    assert!(text.contains("\"items\""));
    assert!(text.contains("\"page\""));

    // §14.3.2's stream is the one file whose existence is the document's to state: it is listed
    // exactly when it can be read, which is the property a `cp -r` of `meta/` depends on.
    let states_xmp = listed.contains(&String::from("xmp.xml"));
    match vfs.open("/meta/xmp.xml") {
        Ok(handle) => {
            assert!(states_xmp, "xmp.xml reads and was not listed");
            assert!(
                handle.bytes().windows(4).any(|window| window == b"<?xp")
                    || handle.bytes().windows(5).any(|window| window == b"<?xml")
                    || handle.bytes().windows(7).any(|window| window == b"x:xmpme"),
                "the packet is not the XML §14.3.2 says it is"
            );
        }
        Err(VfsError::NoSuchPath(_)) => {
            assert!(!states_xmp, "xmp.xml was listed and cannot be read");
        }
        Err(error) => panic!("xmp.xml: {error}"),
    }
}

#[test]
fn an_image_is_read_under_exactly_the_name_its_own_listing_gave() {
    let (_backing, vfs) = mounted(WITH_IMAGES);
    // `images/` is one directory per page, and its listing costs the page count and nothing
    // else — which is the whole argument for the departure `crate::layout` records.
    assert_eq!(names(&vfs, "/images").len(), 72);
    assert!(
        vfs.list("/images/0001")
            .expect("a page with no images")
            .is_empty(),
        "page one of this document places no image"
    );

    let listed = vfs.list("/images/0035").expect("page 35's images");
    assert!(!listed.is_empty(), "page 35 places an image");
    for entry in listed {
        assert_eq!(entry.kind, Kind::File);
        let path = format!("/images/0035/{}", entry.name);
        let handle = vfs
            .open(&path)
            .unwrap_or_else(|error| panic!("{path}: {error}"));
        assert!(!handle.is_empty(), "{path} is empty");
    }
    // A name the listing did not give is not there, whatever it looks like.
    assert!(matches!(
        vfs.open("/images/0035/99.png"),
        Err(VfsError::NoSuchPath(_))
    ));
}

#[test]
fn renders_offers_the_resolutions_the_core_decided_and_no_others() {
    let (_backing, vfs) = mounted(FIVE_PAGES);
    assert_eq!(names(&vfs, "/renders"), vec!["150dpi", "300dpi"]);
    assert_eq!(names(&vfs, "/renders/150dpi").len(), 5);
    let handle = vfs.open("/renders/150dpi/0001.png").expect("a drawn page");
    assert_eq!(
        handle.read(0, 8),
        b"\x89PNG\r\n\x1a\n",
        "the render is not a PNG"
    );
    assert!(matches!(
        vfs.stat("/renders/72dpi/0001.png"),
        Err(VfsError::NoSuchPath(_))
    ));
}

#[test]
fn attachments_are_listed_under_the_names_the_document_files_them_by_made_safe() {
    let (_backing, vfs) = mounted(WITH_ATTACHMENTS);
    let listed = vfs.list("/attachments").expect("§7.11.4's files");
    assert_eq!(listed.len(), 2, "this note files two embedded files");
    for entry in &listed {
        assert_eq!(entry.kind, Kind::File);
        // The document's own names hold a COLON, which is one of the bytes
        // `pdf_transform::pattern::sanitise` replaces, and neither name may be `.` or `..` or
        // carry a solidus — a directory entry is not a path.
        assert!(!entry.name.contains(':'), "{} kept its colon", entry.name);
        assert!(!entry.name.contains('/'), "{} holds a solidus", entry.name);
        assert_ne!(entry.name, ".");
        assert_ne!(entry.name, "..");
        let path = format!("/attachments/{}", entry.name);
        let handle = vfs
            .open(&path)
            .unwrap_or_else(|error| panic!("{path}: {error}"));
        assert!(!handle.is_empty(), "{path} is empty");
    }
    // Sanitisation is a mapping and not a rename: the read went through the document's own
    // name, so a name the document did not state is not a file.
    assert!(matches!(
        vfs.open("/attachments/PDF Declarations - PDF:A Extension Schema.xmp"),
        Err(VfsError::NoSuchPath(_))
    ));
}

#[test]
fn every_write_the_layout_declares_is_refused_by_the_operations_own_name() {
    let (_backing, vfs) = mounted(FIVE_PAGES);
    let declared = [
        ("/pages/0002.pdf", Write::InsertPages, Write::DeletePage),
        (
            "/attachments/anything",
            Write::EmbedFile,
            Write::RemoveAttachment,
        ),
        (
            "/meta/info.json",
            Write::SetInformation,
            Write::Refused(pdf_vfs::layout::Reason::NotOneOfTheFiveVerbs),
        ),
    ];
    for (path, on_write, on_delete) in declared {
        let mapping = vfs.write_meaning(path).expect("the layout names it");
        assert_eq!(mapping.on_write, on_write, "{path}");
        assert_eq!(mapping.on_delete, on_delete, "{path}");
        match vfs.write(path, b"whatever") {
            Err(VfsError::Refused(refused @ Refused::NotYetImplemented { .. })) => {
                let sentence = refused.sentence();
                assert!(sentence.contains("not built yet"), "{sentence}");
                assert!(sentence.contains("pdf-transform"), "{sentence}");
            }
            other => panic!("{path}: {other:?}"),
        }
    }
    // The delete mappings are declared on the same rows, so `remove` refuses them the same way —
    // and `meta/info.json`, whose deletion is *not* one of RFC 0003 section 5.2's five verbs, is
    // refused by design instead. The difference is the point of having two answers per row.
    for path in ["/pages/0002.pdf", "/attachments/anything"] {
        assert!(matches!(
            vfs.remove(path),
            Err(VfsError::Refused(Refused::NotYetImplemented { .. }))
        ));
    }
    match vfs.remove("/meta/info.json") {
        Err(VfsError::Refused(refused @ Refused::ByDesign { .. })) => {
            assert!(refused.sentence().contains("five write verbs"), "{refused}");
        }
        other => panic!("deleting info.json: {other:?}"),
    }
}

#[test]
fn the_four_refusals_of_the_rfc_are_refused_by_design_and_say_why() {
    let (_backing, vfs) = mounted(FIVE_PAGES);
    // RFC 0003 section 5.3, in order: a rename inside pages/, a write into text/, a write into
    // images/, and a write into renders/ or meta/xmp.xml.
    let rename = vfs
        .rename("/pages/0007.pdf", "/pages/0002.pdf")
        .expect_err("a reorder is refused");
    assert!(
        rename.to_string().contains("positions rather than"),
        "{rename}"
    );

    for (path, expected) in [
        ("/text/0001.txt", "no honest in-place meaning"),
        ("/images/0001/01.png", "not supported yet"),
        ("/renders/150dpi/0001.png", "derived from the document"),
        ("/meta/xmp.xml", "derived from the document"),
    ] {
        match vfs.write(path, b"x") {
            Err(VfsError::Refused(refused @ Refused::ByDesign { .. })) => {
                let sentence = refused.sentence();
                assert!(sentence.contains(expected), "{path}: {sentence}");
            }
            other => panic!("{path}: {other:?}"),
        }
    }
    assert!(matches!(
        vfs.create_directory("/pages/mine"),
        Err(VfsError::Refused(Refused::ByDesign { .. }))
    ));
}

#[test]
fn what_the_layout_declares_and_this_round_does_not_do_is_named_out_loud() {
    let (_backing, vfs) = mounted(FIVE_PAGES);
    let shortfalls = vfs.shortfalls();
    // Every row with a write mapping is a shortfall while the write side is unbuilt, and the
    // count is derived from the table rather than written down here.
    let declared = vfs
        .layout()
        .iter()
        .filter(|route| route.write.declares_an_operation())
        .count();
    assert_eq!(
        shortfalls
            .iter()
            .filter(|shortfall| shortfall.detail.contains("write side"))
            .count(),
        declared
    );
    assert!(
        shortfalls
            .iter()
            .any(|shortfall| shortfall.detail.contains("/Collection"))
    );
    assert!(
        shortfalls
            .iter()
            .any(|shortfall| shortfall.detail.contains("default user password"))
    );
}

#[test]
fn a_path_the_layout_does_not_name_is_no_such_path() {
    let (_backing, vfs) = mounted(FIVE_PAGES);
    for path in [
        "/fonts",
        "/pages/0001.png",
        "/text/0001.pdf",
        "/meta/anything.json",
        "/pages/../../etc/passwd",
        "/./pages",
        "/renders/150dpi/0001.png/more",
    ] {
        assert!(
            matches!(vfs.stat(path), Err(VfsError::NoSuchPath(_))),
            "{path} resolved"
        );
    }
    assert!(matches!(
        vfs.list("/pages/0001.pdf"),
        Err(VfsError::NotADirectory(_))
    ));
    assert!(matches!(vfs.open("/pages"), Err(VfsError::IsADirectory(_))));
}

#[test]
fn the_document_changing_under_the_mount_rebuilds_the_tree() {
    let (backing, vfs) = mounted(FIVE_PAGES);
    assert_eq!(vfs.pages().expect("a count"), 5);
    let before = vfs.open("/pages/0001.pdf").expect("a page");
    let key_before = vfs.generation().expect("a key");

    backing.replace(std::fs::read(committed(FOURTEEN_PAGES)).expect("the other document"));

    // The whole tree, not just the page count: a listing, a stat and a read all agree that this
    // is a different document. RFC 0003 section 5.4's "a changed key rebuilds the virtual tree".
    assert_eq!(vfs.pages().expect("a count"), 14);
    assert_ne!(vfs.generation().expect("a key"), key_before);
    assert_eq!(names(&vfs, "/pages").len(), 14);
    let after = vfs.open("/pages/0001.pdf").expect("a page");
    assert_ne!(
        before.bytes(),
        after.bytes(),
        "the cache served the old generation's page"
    );
    assert_ne!(before.generation(), after.generation());
}

#[test]
fn an_open_file_keeps_the_generation_it_was_opened_under() {
    let (backing, vfs) = mounted(FIVE_PAGES);
    let handle = vfs.open("/text/0001.txt").expect("a page's text");
    let bytes = handle.bytes().to_vec();
    let key = handle.generation();

    backing.replace(std::fs::read(committed(FOURTEEN_PAGES)).expect("the other document"));

    // RFC 0003 section 5.4: "an open virtual file keeps the generation it was opened under …
    // No reader ever receives a splice of two generations."
    assert_eq!(handle.bytes(), bytes.as_slice());
    assert_eq!(handle.generation(), key);
    assert_ne!(vfs.generation().expect("a key"), key);
    let reopened = vfs
        .open("/text/0001.txt")
        .expect("the new generation's text");
    assert_ne!(reopened.generation(), key);
}

#[test]
fn a_page_the_new_generation_does_not_have_stops_being_a_path() {
    let (backing, vfs) = mounted(FOURTEEN_PAGES);
    assert!(vfs.stat("/pages/0009.pdf").is_ok());
    backing.replace(std::fs::read(committed(FIVE_PAGES)).expect("the shorter document"));
    assert!(matches!(
        vfs.stat("/pages/0009.pdf"),
        Err(VfsError::NoSuchPath(_))
    ));
}
