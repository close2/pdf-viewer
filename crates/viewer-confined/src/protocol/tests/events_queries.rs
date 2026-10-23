//! Every event and every question, round-tripped.

use super::*;

/// Every event that crosses, encoded and read back.
#[expect(
    clippy::too_many_lines,
    reason = "test code: one fixture per variant of a `viewer-core` enum, and the count is \
                  that enum's"
)]
#[test]
fn every_carried_event_round_trips() {
    let document = DocumentId(2);
    let events = vec![
        Event::Opened {
            document,
            pages: 1023,
        },
        Event::OpenFailed {
            document,
            reason: "unsupported encryption".to_owned(),
        },
        Event::PasswordRequired { document },
        Event::Closed(document),
        Event::PageChanged {
            document,
            index: 3,
            label: Some("iv".to_owned()),
            of: 5,
            section: None,
        },
        Event::Damage(Rect {
            min: Point::new(0.0, 1.0),
            max: Point::new(2.0, 3.0),
        }),
        Event::Searched {
            document,
            found: Some(Found {
                page: 7,
                range: (12, 30),
            }),
            remaining: 0,
            wrapped: true,
        },
        Event::Searched {
            document,
            found: None,
            remaining: 1022,
            wrapped: false,
        },
        Event::OpenUri {
            document,
            uri: "https://example.invalid/".to_owned(),
        },
        Event::NeedsFile {
            document,
            purpose: Purpose::ImportData,
            name: "data.fdf".to_owned(),
            beside: false,
        },
        Event::NeedsFile {
            document,
            purpose: Purpose::TargetRoot,
            name: "target.pdf".to_owned(),
            beside: false,
        },
        Event::NeedsFile {
            document,
            purpose: Purpose::RemoteDocument,
            name: "chapter2.pdf".to_owned(),
            beside: true,
        },
        Event::NeedsFile {
            document,
            purpose: Purpose::NamedPage,
            name: "library.pdf".to_owned(),
            beside: false,
        },
        Event::NeedsFile {
            document,
            purpose: Purpose::ThreadDocument,
            name: "articles.pdf".to_owned(),
            beside: false,
        },
        Event::NeedsFile {
            document,
            purpose: Purpose::LaunchDocument,
            name: "file1.pdf".to_owned(),
            beside: true,
        },
        // §12.8.3.4.4: each of the three commitments and the three specification shapes.
        Event::SignaturePoliciesPublished {
            document,
            policies: vec![
                pdf_signature::policy::PublishedPolicy {
                    identifier: "2.16.724.1.3.1.1.2.1.9".to_owned(),
                    url: "https://sede.060.gob.es/politica_de_firma_anexo_1.pdf".to_owned(),
                    commitment: pdf_signature::policy::Commitment::Stated {
                        digest: pdf_signature::cms::Digest::Sha1,
                        value: vec![0x5a; 20],
                    },
                    specification: None,
                },
                pdf_signature::policy::PublishedPolicy {
                    identifier: "1.2.3".to_owned(),
                    url: "http://127.0.0.1/p.pdf".to_owned(),
                    commitment: pdf_signature::policy::Commitment::UnderAnotherFunction {
                        algorithm: "1.2.840.113549.2.5".to_owned(),
                    },
                    specification: Some(pdf_signature::policy::Specification::ObjectIdentifier(
                        "0.4.0.19122.2.1".to_owned(),
                    )),
                },
                pdf_signature::policy::PublishedPolicy {
                    identifier: "1.2.4".to_owned(),
                    url: "http://127.0.0.1/q.pdf".to_owned(),
                    commitment: pdf_signature::policy::Commitment::NotKnown,
                    specification: Some(pdf_signature::policy::Specification::Uri(
                        "http://127.0.0.1/spec".to_owned(),
                    )),
                },
            ],
        },
        Event::Transition {
            document,
            transition: Transition {
                style: Style::Fly,
                duration: 1.5,
                dimension: Dimension::Vertical,
                motion: Motion::Outward,
                direction: Direction::Degrees(315.0),
                scale: 0.5,
                opaque: true,
            },
        },
        Event::Transition {
            document,
            transition: Transition {
                style: Style::Unrecognised(Name::new(b"Swirl".to_vec())),
                duration: 1.0,
                dimension: Dimension::Horizontal,
                motion: Motion::Inward,
                direction: Direction::None,
                scale: 1.0,
                opaque: false,
            },
        },
        Event::AskingToRunScripts {
            document,
            script: "the calculate script of Total".to_owned(),
            first_line: "event.value = 1;".to_owned(),
        },
        Event::ScriptAsking {
            document,
            question: viewer_core::ScriptQuestion::Alert {
                message: "Submit now?".to_owned(),
                icon: viewer_core::AlertIcon::Question,
                buttons: viewer_core::AlertButtons::YesNoCancel,
                title: Some("Form".to_owned()),
            },
        },
        Event::ScriptAsking {
            document,
            question: viewer_core::ScriptQuestion::Response {
                question: "Your name?".to_owned(),
                title: None,
                default: "Ada".to_owned(),
                label: Some("Name".to_owned()),
                password: true,
            },
        },
        Event::Dirty {
            document,
            dirty: true,
        },
        Event::Saved {
            document,
            bytes: b"%PDF-2.0 ...".to_vec(),
        },
        // §13.2.4.2's three places a clip's data can be, each on the wire.
        Event::Media {
            document,
            content_type: Some("video/mp4".to_owned()),
            media: viewer_core::Media::Embedded {
                name: "clip.mp4".to_owned(),
                bytes: b"moov".to_vec(),
            },
        },
        Event::Media {
            document,
            content_type: None,
            media: viewer_core::Media::Url("https://example.org/clip.mp4".to_owned()),
        },
        Event::Media {
            document,
            content_type: Some("audio/mpeg".to_owned()),
            media: viewer_core::Media::File("clip.mp3".to_owned()),
        },
        Event::Extracted {
            document,
            asked: Extraction::Asked,
            name: "readme.txt".to_owned(),
            bytes: b"hello".to_vec(),
            fragment: None,
        },
        // Both of §O.2.1's provenances on the wire, because the byte that carries them is what
        // lets the host on the other side decline one and write the other.
        Event::Extracted {
            document,
            asked: Extraction::Fragment,
            name: "readme.txt".to_owned(),
            bytes: b"hello".to_vec(),
            // §O.2.1's remainder, on the wire: `#ef=readme.txt&page=3` is the case the far
            // side has to be able to carry out rather than only to be told about.
            fragment: Some("page=3".to_owned()),
        },
        Event::Refused {
            document,
            operation: Operation::Annotate,
            notes: vec!["this document's author certified it".to_owned()],
        },
        // The *ask* and *warn* levels' events and the list moving (ADR 0814).
        Event::Asking {
            document,
            operation: Operation::Modify,
            notes: vec!["bit 4 is clear".to_owned(), "and /P 1".to_owned()],
        },
        Event::Warned {
            document,
            operation: Operation::Extract,
            notes: Vec::new(),
        },
        Event::AttachmentsChanged { document },
        // §12.7.6.2. `owed` is deliberately empty on the far side — `interact::perform` has
        // already sent those sentences as an `Event::Reported`, so the encoder drops them
        // rather than have a host print them twice, and this pins that rather than hiding
        // it: an `owed` built here would come back empty and fail the comparison.
        Event::Submit {
            document,
            submission: Box::new(Submission {
                url: "https://example.invalid/cgi?x=1".to_owned(),
                method: Method::Post,
                format: Format::Fdf,
                media_type: "application/fdf".to_owned(),
                body: b"%FDF-1.2\n".to_vec(),
                fields: 3,
                owed: Vec::new(),
            }),
        },
        Event::Submit {
            document,
            submission: Box::new(Submission {
                url: "https://example.invalid/cgi".to_owned(),
                method: Method::Get,
                format: Format::HtmlForm,
                media_type: "multipart/form-data; boundary=quorra-form-data-0".to_owned(),
                body: Vec::new(),
                fields: 0,
                owed: Vec::new(),
            }),
        },
        Event::Reported {
            document,
            page: Some(0),
            notes: vec!["one".to_owned(), "two".to_owned()],
        },
        Event::Reported {
            document,
            page: None,
            notes: Vec::new(),
        },
    ];
    for event in &events {
        let encoded = encode_event(event).unwrap();
        let read = decode_event(&encoded).unwrap();
        assert_eq!(
            format!("{read:?}"),
            format!("{event:?}"),
            "an event changed on the way through"
        );
    }
}

/// The two messages that do not cross, refused by name rather than dropped.
///
/// This is the property the whole module is built around: a message that stays on one side
/// is an error a caller can read, because a boundary that silently swallowed one would be
/// indistinguishable from a viewer that did nothing.
#[test]
fn the_two_messages_that_stay_where_they_are_say_so() {
    // A real render request, because a `RenderToken` cannot be constructed outside
    // `viewer-core` — which is itself the point: the token is the confined process's own
    // bookkeeping, and there is no way for a host to make one up.
    let bytes = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/PDF20_AN001-BPC.pdf"),
    )
    .unwrap();
    let mut viewer = viewer_core::Viewer::new(400, 500, 1.0);
    let events: Vec<Event> = viewer
        .handle(Command::Open {
            id: DocumentId(1),
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .collect();
    let request = events
        .iter()
        .find_map(|event| match event {
            Event::NeedsRender(request) => Some(request.clone()),
            _ => None,
        })
        .unwrap();

    let refused = encode_command(&Command::RenderReady {
        token: request.token,
        rendered: viewer_core::Rendered::Presented,
    })
    .unwrap_err();
    assert_eq!(refused.message, "Command::RenderReady");

    let refused = encode_event(&Event::NeedsRender(request)).unwrap_err();
    assert_eq!(refused.message, "Event::NeedsRender");
}

/// Every question `viewer-core` states, encoded and read back.
///
/// **Every one crosses** (ADR 0223), so there is no list of questions refused by name. All of
/// them are here, written out rather than generated, so that a question added to `viewer-core`
/// makes `encode_query`'s match fail to compile and somebody then notices there is no round
/// trip for it.
#[test]
fn every_query_is_carried() {
    let carried = [
        Query::PageCount,
        Query::CurrentPage,
        Query::PageGeometry(2),
        Query::PageLabel(2),
        Query::LinkAt((3.0, 4.0)),
        Query::FieldAt((3.0, 4.0)),
        Query::Caret {
            at: (3.0, 4.0),
            offset: 5,
        },
        Query::Offset {
            at: (3.0, 4.0),
            point: (11.0, 12.0),
        },
        Query::FieldSelection {
            at: (3.0, 4.0),
            from: 2,
            to: 6,
        },
        Query::FreeTextAt { at: (3.0, 4.0) },
        Query::Dirty,
        Query::TimerDue,
        Query::Find("needle"),
        Query::Selection,
        Query::LogicalSelection,
        Query::Focus,
        Query::Highlight,
        Query::Frame,
        Query::Reports,
        Query::Outline,
        Query::Layers,
        Query::Attachments,
        Query::Collection,
        Query::Articles,
        Query::Thumbnail(7),
        Query::AttachmentPreview("<3>report.pdf"),
        Query::PrintPage(7),
        Query::Measure(&[[3.0, 4.0], [11.0, 12.0], [1.0, 2.0]]),
        Query::Properties,
        Query::Opening,
        Query::Preferences,
        Query::Popups,
        Query::Fields,
        Query::AccessibilityTree,
        Query::Readback,
        Query::View,
    ];
    assert_eq!(carried.len(), 36, "every question `viewer-core` states");
    for query in carried {
        let encoded = encode_query(query).unwrap();
        let read = decode_query(&encoded).unwrap();
        assert_eq!(
            format!("{:?}", read.as_query()),
            format!("{query:?}"),
            "a query changed on the way through"
        );
    }
}
