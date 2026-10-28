//! A Screen or Movie annotation's clip kept as an embedded file at PDF/A-4f and PDF/A-4e.
//!
//! ISO 19005-4 section 6.3.1 does not permit the Sound, Screen and Movie types, and Annexes A.2
//! and B.4 admit an embedded file of any type at the two flavours. So the annotation goes, its
//! poster is what the `preserve` row keeps, and with `keep-media = "attach"` the specification the
//! annotation played is filed in the `/EmbeddedFiles` tree as the producer wrote it.

use super::*;

/// The row at the part 4 subtype requirement, keeping the poster and the clip.
const KEEP_THE_CLIP: &str = "[site.\"annotations/subtype-defined-in-iso-32000-2\"]\n\
     remedy = \"preserve\"\nplacement = \"append\"\nkeep-media = \"attach\"\n";

/// [`KEEP_THE_CLIP`]'s plan and nothing else: section 6.6.1 does not admit the rendition action the
/// Screen annotation carries either, and the removal that row answers carries the action with the
/// annotation, under the row's own decision (ADR 2568).
fn keeping_the_clip(target: Target) -> ArchivePlan {
    plan_from(KEEP_THE_CLIP, target)
}

/// A part 4 page with a Screen annotation playing an embedded clip through §13.2.3.2's media
/// rendition, and a Movie annotation whose §13.4 movie names a file outside this one.
fn a_screen_and_a_movie() -> Vec<u8> {
    Conforming {
        page: "/Annots [6 0 R 8 0 R]".to_owned(),
        objects: vec![
            "<< /Type /Annot /Subtype /Screen /Rect [20 30 40 60] /F 4 /AP << /N 11 0 R >> \
             /A << /S /Rendition /OP 0 /AN 6 0 R /R << /S /MR /C << /S /MCD /CT (video/mp4) \
             /D 7 0 R >> >> >> >>"
                .to_owned(),
            "<< /Type /Filespec /F (clip.mp4) /UF (clip.mp4) /EF << /F 10 0 R /UF 10 0 R >> \
             /AFRelationship /Unspecified >>"
                .to_owned(),
            "<< /Type /Annot /Subtype /Movie /Rect [60 30 80 60] /F 4 /AP << /N 11 0 R >> \
             /Movie << /F 9 0 R >> >>"
                .to_owned(),
            // The plant: a movie file this document does not hold, which no tree may file.
            "<< /Type /Filespec /F (outside.mov) /UF (outside.mov) >>".to_owned(),
        ],
        binary_objects: vec![
            stream(
                "/Type /EmbeddedFile /Subtype /video#2Fmp4 \
                 /Params << /ModDate (D:20260101000000Z) >> /Length 8",
                b"\0\0\0\x18ftyp",
            ),
            stream(
                "/Type /XObject /Subtype /Form /BBox [0 0 20 30] /Length 15",
                b"0 0 20 30 re f\n",
            ),
        ],
        ..Conforming::default()
    }
    .build()
}

#[test]
fn a_screen_annotation_s_clip_stays_as_an_embedded_file_at_the_two_flavours() {
    let source = a_screen_and_a_movie();
    for target in [Target::Four(Flavour::F), Target::Four(Flavour::E)] {
        let (report, output) = convert_with_plan(&source, &keeping_the_clip(target));
        let conversion = conversion(&report);
        let output = output.unwrap_or_else(|| panic!("{target}: {}", conversion.render()));
        let held = holds(&output, target);
        assert_eq!(
            held.verdict(),
            Verdict::Conforms,
            "{target}: {}",
            held.render()
        );
        if let Some(directory) = std::env::var_os("MEDIA_CLIP_ARTIFACTS") {
            let path = std::path::Path::new(&directory)
                .join(format!("clip-{}.pdf", target.to_string().replace('/', "")));
            std::fs::write(path, &output).expect("the artifact is written");
        }
        let kept = &conversion.kept_attachments;
        assert_eq!(kept.len(), 1, "{target}: the embedded clip only: {kept:?}");
        let clip = kept.first().expect("one kept file");
        assert_eq!(clip.name, "clip.mp4");
        assert_eq!(clip.annotation, "the Screen annotation in object 6 0");
        let document =
            Document::open_with_limits(output.clone(), Limits::DEFAULT).expect("the output opens");
        let catalog = document.catalog().expect("a catalog");
        let names = document.get_key(&catalog, "Names");
        let tree = document.get_key(names.as_dict().expect("names"), "EmbeddedFiles");
        let entries = document.get_key(tree.as_dict().expect("a tree"), "Names");
        let entries = entries.as_array().expect("one node");
        assert_eq!(entries.len(), 2, "{target}: one key and one value");
        let specification = document.resolve(entries.get(1).expect("a value"));
        let specification = specification.as_dict().expect("a specification");
        let stream = document.get_key(
            document
                .get_key(specification, "EF")
                .as_dict()
                .expect("an EF"),
            "F",
        );
        let pdf_syntax::Object::Stream(stream) = stream else {
            panic!("{target}: the clip's stream is kept");
        };
        assert_eq!(
            document.decoded_stream_data(&stream).as_deref(),
            Some(&b"\0\0\0\x18ftyp"[..]),
            "{target}: the producer's bytes, unchanged"
        );
        let page = pdf_model::Pages::new(&document).get(0).expect("a page");
        assert!(
            document
                .get_key(&page.dict, "Annots")
                .as_array()
                .is_none_or(<[_]>::is_empty),
            "{target}: both annotations are gone"
        );
    }
}

#[test]
fn keeping_a_clip_is_refused_where_every_embedded_file_must_conform() {
    // ISO 19005-4 section 6.9's first item at PDF/A-4, and ISO 19005-2 section 6.8 at part 2.
    for (text, target) in [
        (KEEP_THE_CLIP.to_owned(), Target::Four(Flavour::Plain)),
        (
            KEEP_THE_CLIP.replace("iso-32000-2", "iso-32000-1"),
            Target::Two(Level::B),
        ),
    ] {
        assert!(
            pdf_transform::archive::Configuration::read(&text, target).is_err(),
            "{target}"
        );
    }
    // And away from a `preserve` row at a subtype requirement.
    let discard = KEEP_THE_CLIP.replace("\"preserve\"", "\"discard\"");
    assert!(
        pdf_transform::archive::Configuration::read(&discard, Target::Four(Flavour::F)).is_err()
    );
}
