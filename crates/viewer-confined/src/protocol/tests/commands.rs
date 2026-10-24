//! The greeting and every command, round-tripped.

use super::*;

#[test]
fn a_greeting_round_trips_and_an_unconfined_one_is_legible() {
    let confinement = Confinement {
        landlock: pdf_sandbox::lockdown::LandlockLevel::Enforced,
        address_space_limit: 4 << 30,
        system_calls: pdf_sandbox::lockdown::SystemCalls::Filtered,
    };
    assert_eq!(
        parse_handshake(&encode_handshake(confinement)),
        Some(confinement)
    );

    let none = Confinement {
        landlock: pdf_sandbox::lockdown::LandlockLevel::Unavailable,
        address_space_limit: 0,
        system_calls: pdf_sandbox::lockdown::SystemCalls::Unfiltered,
    };
    let parsed = parse_handshake(&encode_handshake(none)).unwrap();
    assert!(!parsed.is_enforced());
    assert!(parsed.shortfall().is_some());
}

#[test]
fn a_greeting_from_another_program_is_rejected() {
    let mut noise = [0u8; HANDSHAKE_LEN];
    noise[..8].copy_from_slice(b"usage: p");
    assert_eq!(parse_handshake(&noise), None);
}

/// Every command that crosses, encoded and read back.
///
/// The list is written out rather than generated: a variant added to `viewer-core` makes
/// `encode_command`'s match fail to compile, and this is where somebody then notices that
/// the new one has no round trip.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one entry per command this transport carries, and the count is `viewer-core`'s. \
                  A second list would be a list somebody forgets to add to"
)]
fn every_carried_command_round_trips() {
    let commands = vec![
        Command::Open {
            id: DocumentId(7),
            bytes: b"%PDF-2.0".to_vec().into(),
            password: Some("secret".to_owned().into()),
            fragment: Some("page=3".to_owned()),
        },
        Command::Tick { millis: 16 },
        Command::Close(DocumentId(7)),
        Command::Focus(DocumentId(7)),
        Command::Resize {
            width: 800,
            height: 1000,
            scale: 2.0,
        },
        Command::GoTo(PageTarget::Index(4)),
        Command::GoTo(PageTarget::First),
        Command::GoTo(PageTarget::Last),
        Command::GoTo(PageTarget::Next),
        Command::GoTo(PageTarget::Previous),
        Command::GoTo(PageTarget::Relative(-3)),
        Command::Zoom {
            zoom: Zoom::Scale(1.5),
            at: Some((10.0, 20.0)),
        },
        Command::Zoom {
            zoom: Zoom::FitPage,
            at: None,
        },
        Command::Zoom {
            zoom: Zoom::FitWidth,
            at: None,
        },
        Command::Zoom {
            zoom: Zoom::FitHeight,
            at: None,
        },
        Command::Zoom {
            zoom: Zoom::In,
            at: None,
        },
        Command::Zoom {
            zoom: Zoom::Out,
            at: None,
        },
        Command::Scroll { dx: -1.5, dy: 2.5 },
        // Where the reader is looking, said absolutely: the one command whose value a host
        // does not compose but echoes, so the round trip here is what the restore rests on
        // (ADR 0737).
        Command::View(viewer_core::Viewing {
            page: 7,
            zoom: Zoom::Scale(2.25),
            scroll: (-1.5, 812.5),
        }),
        Command::View(viewer_core::Viewing {
            page: 0,
            zoom: Zoom::FitWidth,
            scroll: (0.0, 0.0),
        }),
        Command::Restrict(RestrictionScope::Window(RestrictionPolicy::uniform(
            RestrictionLevel::Off,
        ))),
        Command::Restrict(RestrictionScope::Window(RestrictionPolicy::uniform(
            RestrictionLevel::On,
        ))),
        // The other two of `CLAUDE.md`'s four levels, and the answer that makes the third
        // one a level (ADR 0814).
        Command::Restrict(RestrictionScope::Window(RestrictionPolicy::uniform(
            RestrictionLevel::Ask,
        ))),
        // And a level per operation: six bytes rather than one, and a policy that is *not*
        // uniform is what catches a wire that lost the order they go in (ADR 1144).
        Command::Restrict(RestrictionScope::Window(
            RestrictionPolicy::default()
                .with(Operation::Extract, RestrictionLevel::Ask)
                .with(Operation::Annotate, RestrictionLevel::On)
                .with(Operation::Assemble, RestrictionLevel::Warn),
        )),
        // And a *scope* per policy: a document that departs from the window in one operation
        // and in none, because the absence of a departure is a fifth byte value and a wire that
        // lost it would turn *this document follows the window* into a level nobody chose
        // (ADR 1145).
        Command::Restrict(RestrictionScope::Document(
            RestrictionOverride::NONE.with(Operation::Extract, Some(RestrictionLevel::Ask)),
        )),
        Command::Restrict(RestrictionScope::Document(RestrictionOverride::NONE)),
        Command::Copy,
        // §7.6.4.2's bit 3, and all three ends of §8.11.4.5's print operation: a sheet
        // somebody chose, Table 193's "not known" — which is a different answer and not a
        // missing one — and the revert the clause requires (ADR 1180).
        Command::Print(viewer_core::Printing::Start(viewer_core::Sheet {
            media: Some([0.0, 0.0, 595.0, 842.0]),
            scale: 300.0 / 72.0,
            page_scale: 1.0,
        })),
        Command::Print(viewer_core::Printing::Paper(viewer_core::Sheet {
            media: None,
            scale: 150.0 / 72.0,
            page_scale: 0.5,
        })),
        Command::Print(viewer_core::Printing::Finish),
        // §12.8.1's third question: the empty policy every host starts with, and a populated
        // one. Both, because the empty set is not a degenerate case — it is this program's
        // standing answer (ADR 1039) and a wire that lost it would turn *nobody named one*
        // into a store nobody supplied.
        Command::Trust(TrustPolicy::default()),
        Command::Trust(TrustPolicy {
            anchors: pdf_signature::trust::Supply::of(
                vec![
                    ("root.der".to_owned(), vec![0x30, 0x03, 0x02, 0x01, 0x00]),
                    ("second.pem (certificate 2)".to_owned(), vec![0x30, 0x00]),
                ],
                "/etc/quorra/anchors (--trust-anchors)".to_owned(),
                pdf_signature::x509::Instant::from_unix_seconds(1_790_812_800),
            ),
            acceptance: Acceptance::UnknownRevocationAccepted,
        }),
        Command::Restrict(RestrictionScope::Window(RestrictionPolicy::uniform(
            RestrictionLevel::Warn,
        ))),
        Command::Answer {
            document: DocumentId(3),
            proceed: true,
        },
        Command::Answer {
            document: DocumentId(3),
            proceed: false,
        },
        Command::Present(PresentationMode::On),
        Command::Present(PresentationMode::Off),
        Command::Edit(Edit::SetField {
            field: "A.NOM".to_owned(),
            value: Entered::Text("typed".to_owned()),
        }),
        Command::Edit(Edit::SetField {
            field: "A.NOM".to_owned(),
            value: Entered::Cleared,
        }),
        // §12.7.5.4's list box: Table 233 bit 22 permits several items at once, and all three
        // of `Entered`'s shapes cross so that a confined host has no less than an unconfined
        // one (ADR 0248).
        Command::Edit(Edit::SetField {
            field: "A.NOM".to_owned(),
            value: Entered::Chosen(Vec::new()),
        }),
        Command::Edit(Edit::SetField {
            field: "A.NOM".to_owned(),
            value: Entered::Chosen(vec![0, 2, 5]),
        }),
        Command::Edit(Edit::Markup {
            kind: Markup::Squiggly,
            colour: [1.0, 0.5, 0.0],
        }),
        // §12.5.6.6's two: a rectangle a person drew, and what they typed in it (ADR 0238).
        Command::Edit(Edit::FreeText {
            from: (12.0, 34.0),
            to: (56.0, 78.0),
            colour: [0.7, 0.1, 0.1],
        }),
        Command::Edit(Edit::SetFreeText {
            annotation: ObjectId::new(19, 0),
            text: "Reviewed".to_owned(),
        }),
        // §12.5.6.4's note, retyped in its window (ADR 1726).
        Command::Edit(Edit::SetNoteText {
            annotation: ObjectId::new(20, 0),
            text: "Seen".to_owned(),
        }),
        // §7.11.4's file in both of §7.11.4.1's homes, and out again: the bytes cross whole
        // (ADR 0814).
        Command::Edit(Edit::Attach {
            bytes: b"a,b,c\n".to_vec().into(),
            name: "data.csv".to_owned(),
            description: Some("last quarter".to_owned()),
            mime: Some("text/csv".to_owned()),
            home: AttachHome::Document,
        }),
        Command::Edit(Edit::Attach {
            bytes: Vec::new().into(),
            name: "empty".to_owned(),
            description: None,
            mime: None,
            home: AttachHome::Page { at: (12.5, 34.0) },
        }),
        Command::Edit(Edit::Detach {
            name: "data.csv".to_owned(),
        }),
        Command::Undo,
        Command::Redo,
        Command::Extract {
            name: "attachment.txt".to_owned(),
        },
        Command::Save,
        // What the document says about itself, asked for rather than done by the open
        // (ADR 1044).
        Command::Report,
        Command::Select(Selection::All),
        Command::Select(Selection::None),
        // Annex O's `search` and a find bar's *next*: all three steps and both directions,
        // because a confined host has to be able to drive a search exactly as an unconfined one
        // does (ADR 0250).
        Command::Find(Find::Start {
            needle: "transparency group".to_owned(),
            direction: FindDirection::Forward,
        }),
        Command::Find(Find::Start {
            needle: String::new(),
            direction: FindDirection::Backward,
        }),
        Command::Find(Find::Continue),
        Command::Find(Find::Stop),
        Command::Focused(FocusMove::Next),
        Command::Focused(FocusMove::Previous),
        Command::Focused(FocusMove::None),
        Command::CommitField {
            field: "Lines.Price".to_owned(),
        },
        Command::Presented,
        // RFC 0008 section 6.3's level at the one value that crosses, and the ask's answer
        // (ADR 1616).
        Command::Scripts(viewer_core::Scripting::Off),
        // What a page past `MAX_PIXELS` is, both ways (ADR 1944).
        Command::Oversize(viewer_core::Oversize::Refused),
        Command::Oversize(viewer_core::Oversize::Listed),
        Command::AnswerScripts {
            document: DocumentId(4),
            proceed: true,
        },
        // A script's question answered each of the three ways (ADR 1628).
        Command::AnswerScript {
            document: DocumentId(4),
            answer: viewer_core::ScriptAnswer::Pressed(viewer_core::AlertButton::Yes),
        },
        Command::AnswerScript {
            document: DocumentId(4),
            answer: viewer_core::ScriptAnswer::Typed(Some("Ada".to_owned())),
        },
        Command::AnswerScript {
            document: DocumentId(4),
            answer: viewer_core::ScriptAnswer::Typed(None),
        },
        Command::AnswerScript {
            document: DocumentId(4),
            answer: viewer_core::ScriptAnswer::Unanswerable,
        },
        // §12.7.8's answer to a submission and to a fetched import, in each format byte
        // (ADR 2076).
        Command::Respond {
            document: DocumentId(2),
            answers: Answered::Submission,
            source: "http://127.0.0.1:8080/submit".to_owned(),
            format: DataFormat::Fdf,
            bytes: b"%FDF-1.2".to_vec(),
        },
        Command::Respond {
            document: DocumentId(3),
            answers: Answered::Import,
            source: "https://example.org/values.xfdf".to_owned(),
            format: DataFormat::Xfdf,
            bytes: Vec::new(),
        },
        Command::Respond {
            document: DocumentId(4),
            answers: Answered::Submission,
            source: String::new(),
            format: DataFormat::Other,
            bytes: b"<p>thanks</p>".to_vec(),
        },
        // What a draw of a page the window drew from its marks declined, and a draw that
        // declined nothing (ADR 2290).
        Command::Declined {
            document: DocumentId(5),
            page: 3,
            declined: vec![viewer_core::Declined {
                condition: "two of its edges cross along a common line".to_owned(),
                sentence: "a mark of 3 pieces, 4×5 pixels from (6, 7), was drawn at the \
                           library's sixteenths"
                    .to_owned(),
            }],
        },
        Command::Declined {
            document: DocumentId(5),
            page: 0,
            declined: Vec::new(),
        },
        Command::Activate(ObjectId::new(12, 1)),
        Command::SetGroup {
            group: ObjectId::new(3, 0),
            on: true,
        },
        Command::Pointer {
            at: (4.0, 5.0),
            action: PointerAction::Released,
        },
        Command::Supply {
            purpose: Purpose::ImportData,
            bytes: Some(b"%FDF-1.2".to_vec()),
        },
        Command::Supply {
            purpose: Purpose::ImportData,
            bytes: None,
        },
        // §12.6.4.4's Table 204 `/F`: the other purpose, in both of its answers, because a
        // purpose is a number on this wire and a second one is where an encoding drifts.
        Command::Supply {
            purpose: Purpose::TargetRoot,
            bytes: Some(b"%PDF-1.7".to_vec()),
        },
        Command::Supply {
            purpose: Purpose::TargetRoot,
            bytes: None,
        },
        // §12.6.4.3's Table 203 `/F`: the third purpose, on the same argument.
        Command::Supply {
            purpose: Purpose::RemoteDocument,
            bytes: Some(b"%PDF-2.0".to_vec()),
        },
        Command::Supply {
            purpose: Purpose::RemoteDocument,
            bytes: None,
        },
        // §12.7.8's Table 253 `/F` and §12.6.4.7's Table 209 `/F`: the fourth and fifth, on
        // the same argument (ADR 1239).
        Command::Supply {
            purpose: Purpose::NamedPage,
            bytes: Some(b"%PDF-1.4".to_vec()),
        },
        Command::Supply {
            purpose: Purpose::NamedPage,
            bytes: None,
        },
        Command::Supply {
            purpose: Purpose::ThreadDocument,
            bytes: Some(b"%PDF-1.3".to_vec()),
        },
        Command::Supply {
            purpose: Purpose::ThreadDocument,
            bytes: None,
        },
        // §12.6.4.6's Table 207 `/F`: the sixth, whose bytes may not be a PDF (ADR 1358).
        Command::Supply {
            purpose: Purpose::LaunchDocument,
            bytes: Some(b"#!/bin/sh".to_vec()),
        },
        Command::Supply {
            purpose: Purpose::LaunchDocument,
            bytes: None,
        },
        // §10.8.3's simulation, in both of its answers: one bit, and a bit is where an
        // encoding that wrote the wrong byte would look like the other answer.
        Command::Separations(true),
        Command::Separations(false),
        Command::Beside(None),
        Command::Beside(Some(DocumentId(9))),
        // §8.11.4.4's answers about the reader, in both the shapes that differ on the wire:
        // three lists of names with a language, and the empty answer whose language is
        // *unstated* rather than empty (§14.9.2.2 gives the empty tag its own meaning).
        Command::Audience(pdf_model::optional_content::Audience {
            reader: pdf_model::optional_content::Reader {
                individual: vec!["Alice".to_owned(), "A. Smith".to_owned()],
                title: vec!["Reviewer".to_owned()],
                organisation: vec!["Acme".to_owned()],
            },
            language: Some("es-MX".to_owned()),
        }),
        Command::Audience(pdf_model::optional_content::Audience::NONE),
        // Table 166's `/M`, in the three shapes that differ on the wire: a date with a zone,
        // one whose zone is *unstated* rather than UT (§7.9.4 keeps those apart), and the
        // host that has said nothing at all.
        Command::Clock(Some(pdf_syntax::Date {
            year: 2026,
            month: 9,
            day: 21,
            hour: 14,
            minute: 5,
            second: 30,
            offset: Some(-330),
        })),
        Command::Clock(Some(pdf_syntax::Date {
            year: 1970,
            month: 1,
            day: 1,
            hour: 0,
            minute: 0,
            second: 0,
            offset: None,
        })),
        Command::Clock(None),
    ];
    for command in &commands {
        let encoded = encode_command(command).unwrap();
        let read = decode_command(&encoded).unwrap();
        assert_eq!(
            format!("{read:?}"),
            format!("{command:?}"),
            "a command changed on the way through"
        );
    }
}

/// A document open on disk crosses as its length and a descriptor, and decodes to the same
/// file — or refuses by name where the descriptor did not come (ADR 0812).
///
/// The encoder writes no byte of the document, the descriptor is what [`command_descriptor`]
/// hands the transport, and a decoder given one holds the same open file at the same length;
/// given none, it names the field rather than answering with an empty document.
#[test]
#[cfg(unix)]
fn a_document_on_disk_crosses_as_a_descriptor_and_its_length() {
    let directory =
        std::env::temp_dir().join(format!("viewer-confined-wire-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("on-disk.pdf");
    let content = b"%PDF-2.0\n% a document on disk, not a byte of which crosses\n";
    std::fs::write(&path, content).unwrap();

    let on_disk = pdf_syntax::FileBytes::on_disk(&path).unwrap();
    let command = Command::Open {
        id: DocumentId(11),
        bytes: on_disk,
        password: None,
        fragment: Some("page=2".to_owned()),
    };
    let encoded = encode_command(&command).unwrap();
    assert!(
        encoded.len() < content.len(),
        "the document's bytes were written into the frame: {} bytes",
        encoded.len()
    );
    assert!(command_descriptor(&command).is_some());
    assert!(
        command_descriptor(&Command::Tick { millis: 1 }).is_none(),
        "only an open on disk has a descriptor"
    );

    // Without the descriptor: refused by name.
    assert_eq!(
        decode_command(&encoded).unwrap_err(),
        ProtocolError::NoDescriptor {
            what: "a document on disk"
        }
    );

    // With one — the kernel's duplicate, here made by `try_clone` — the same file.
    let duplicate: std::os::fd::OwnedFd = std::fs::File::open(&path).unwrap().into();
    let mut descriptors = vec![duplicate];
    let decoded = decode_command_holding(&encoded, &mut descriptors).unwrap();
    assert!(descriptors.is_empty(), "the descriptor was claimed");
    let Command::Open {
        id,
        bytes,
        fragment,
        ..
    } = decoded
    else {
        panic!("an Open decoded as something else");
    };
    assert_eq!(id, DocumentId(11));
    assert_eq!(fragment.as_deref(), Some("page=2"));
    assert!(bytes.is_on_disk());
    assert_eq!(bytes.len(), content.len());
    assert_eq!(bytes.whole().unwrap(), content);

    // Bytes in memory still cross as bytes, and claim no descriptor.
    let in_memory = Command::Open {
        id: DocumentId(12),
        bytes: content.to_vec().into(),
        password: None,
        fragment: None,
    };
    assert!(command_descriptor(&in_memory).is_none());
    let encoded = encode_command(&in_memory).unwrap();
    let mut unclaimed = vec![std::os::fd::OwnedFd::from(
        std::fs::File::open(&path).unwrap(),
    )];
    let decoded = decode_command_holding(&encoded, &mut unclaimed).unwrap();
    assert_eq!(unclaimed.len(), 1, "a descriptor beside bytes is nobody's");
    let Command::Open { bytes, .. } = decoded else {
        panic!("an Open decoded as something else");
    };
    assert!(!bytes.is_on_disk());
    assert_eq!(bytes.whole().unwrap(), content);
    std::fs::remove_dir_all(&directory).unwrap();
}

/// §7.6.4.1's password crosses whole, which the comparison above can no longer see.
///
/// `Command`'s `Debug` compares every field of every variant, and a `viewer_core::Secret`
/// prints how many characters it holds and not which (ADR 0545) — the property that keeps it
/// out of a launch log. That is exactly the property that would let a transport corrupt a
/// password into another of the same length with the test above still green, so the one field
/// the general check went blind to gets its own.
#[test]
fn a_password_crosses_the_transport_unchanged() {
    let command = Command::Open {
        id: DocumentId(7),
        bytes: b"%PDF-2.0".to_vec().into(),
        password: Some("m\u{fc}hsam gew\u{e4}hlt".to_owned().into()),
        fragment: None,
    };
    let read = decode_command(&encode_command(&command).unwrap()).unwrap();
    let Command::Open { password, .. } = read else {
        panic!("an Open decoded as something else");
    };
    assert_eq!(
        password.as_ref().map(viewer_core::Secret::reveal),
        Some("m\u{fc}hsam gew\u{e4}hlt"),
        "the password changed on the way through"
    );
}
