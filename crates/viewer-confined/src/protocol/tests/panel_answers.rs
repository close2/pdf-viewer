//! Every panel answer and the located point, round-tripped.

use super::*;

/// The reader's place, out and back, in both directions and every magnification.
///
/// **The one answer whose bytes a host hands straight back**, so this round trip is what the
/// exactness of a restore across a worker's death rests on: a magnification rounded or a
/// scroll truncated here would put the reader somewhere near where they were (ADR 0737).
#[test]
fn a_view_crosses_and_comes_back_unchanged() {
    for zoom in [
        Zoom::FitPage,
        Zoom::FitWidth,
        Zoom::FitHeight,
        Zoom::Scale(1.0 / 3.0),
        Zoom::In,
        Zoom::Out,
    ] {
        let view = viewer_core::Viewing {
            page: 41,
            zoom,
            scroll: (-1.5, 1.0 / 7.0),
        };
        let Reply::View(read) = round_trip(&Answer::View(view)) else {
            panic!("a view comes back as one");
        };
        assert_eq!(read, view, "a view changed on the way through");
    }
}

/// A value for every field of every type the eleven panel answers are made of.
///
/// **Nothing here is a default.** A round trip over `Default::default()` would pass with an
/// encoder that wrote nothing and a decoder that read nothing, which is the exact defect this
/// module has to be free of: an encoding that drops a field is a panel showing less on the
/// confined path than off it, and no gate in this tree looks at a panel. So every `Option` is
/// `Some`, every `bool` is the opposite of its default where the clause states one, and every
/// list has more than one element.
fn a_populated_outline() -> pdf_model::outline::Outline {
    use pdf_model::destination::{Destination, Target, View};
    use pdf_model::outline::{Item, Outline};

    let leaf = Item {
        id: ObjectId::new(9, 2),
        title: "a leaf".to_owned(),
        destination: Some(Destination {
            target: Target::Number(11),
            view: View::FitR {
                rect: [1.0, 2.0, 3.0, 4.0],
            },
        }),
        open: false,
        italic: true,
        bold: false,
        colour: [0.25, 0.5, 0.75],
        children: Vec::new(),
    };
    Outline {
        items: vec![
            Item {
                id: ObjectId::new(4, 0),
                title: "a section".to_owned(),
                destination: Some(Destination {
                    target: Target::Object(ObjectId::new(5, 0)),
                    view: View::Xyz {
                        left: Some(10.0),
                        top: None,
                        zoom: Some(2.0),
                    },
                }),
                open: true,
                italic: false,
                bold: true,
                colour: [1.0, 0.0, 0.0],
                children: vec![leaf],
            },
            Item {
                id: ObjectId::new(6, 0),
                title: "an item with no destination".to_owned(),
                destination: None,
                open: false,
                italic: false,
                bold: false,
                colour: [0.0, 0.0, 0.0],
                children: Vec::new(),
            },
        ],
        stated_count: Some(-3),
    }
}

/// One of every shape §12.3.5's collection dictionary can take.
fn a_populated_collection() -> pdf_model::collection::Collection {
    use pdf_model::collection::{
        Collection, Colours, Field, FieldKind, Folder, Item, Layout, NameRestriction, Navigator,
        Sort, Split, SplitDirection, Value, View,
    };

    let mut values = std::collections::BTreeMap::new();
    values.insert(
        "author".to_owned(),
        Value {
            data: pdf_syntax::Object::String(b"a name".as_slice().into()),
            prefix: Some("by ".to_owned()),
        },
    );
    values.insert(
        "revision".to_owned(),
        Value {
            data: pdf_syntax::Object::Real(1.5),
            prefix: None,
        },
    );

    let mut schema = std::collections::BTreeMap::new();
    schema.insert(
        "author".to_owned(),
        Field {
            kind: FieldKind::Text,
            name: "Author".to_owned(),
            order: Some(2),
            visible: false,
            editable: true,
        },
    );
    schema.insert(
        "custom".to_owned(),
        Field {
            kind: FieldKind::Other("Widget".to_owned()),
            name: "Custom".to_owned(),
            order: None,
            visible: true,
            editable: false,
        },
    );

    Collection {
        schema,
        initial: Some("first.pdf".to_owned()),
        view: View::Navigator,
        sort: Some(Sort {
            fields: vec!["author".to_owned(), "revision".to_owned()],
            ascending: vec![true, false],
        }),
        navigator: Some(Navigator {
            layouts: vec![
                Layout::View(View::Tile),
                Layout::FilmStrip,
                Layout::FreeForm,
                Layout::Linear,
                Layout::Tree,
                Layout::Custom("Mosaic".to_owned()),
            ],
        }),
        colours: Colours {
            background: Some([0.1, 0.2, 0.3]),
            card_background: Some([0.4, 0.5, 0.6]),
            card_border: None,
            primary_text: Some([0.7, 0.8, 0.9]),
            secondary_text: Some([1.0, 1.0, 1.0]),
        },
        split: Some(Split {
            direction: SplitDirection::Vertical,
            position: Some(30.0),
        }),
        folders: Some(Folder {
            id: 0,
            name: "root".to_owned(),
            description: Some("everything".to_owned()),
            item: Item {
                values: values.clone(),
            },
            has_thumbnail: true,
            children: vec![Folder {
                id: 7,
                name: "drafts".to_owned(),
                description: None,
                item: Item { values },
                has_thumbnail: false,
                children: Vec::new(),
            }],
        }),
        // §12.3.5.2's six restrictions, one entry each, so that the codec is exercised on
        // every variant rather than on whichever one a fixture happened to break.
        invalid_names: vec![
            name_defect(NameRestriction::TextString),
            name_defect(NameRestriction::EmbeddedNull),
            name_defect(NameRestriction::Length { characters: 0 }),
            name_defect(NameRestriction::Special { character: ':' }),
            name_defect(NameRestriction::TrailingFullStop),
            name_defect(NameRestriction::Duplicate {
                normalised: "drafts".to_owned(),
            }),
        ],
    }
}

/// One of §12.3.5.2's restrictions, against a folder and against a file in turn.
fn name_defect(
    restriction: pdf_model::collection::NameRestriction,
) -> pdf_model::collection::NameDefect {
    use pdf_model::collection::{NameDefect, NameRestriction, Named};

    let folder = matches!(
        restriction,
        NameRestriction::TextString
            | NameRestriction::Length { .. }
            | NameRestriction::TrailingFullStop
    );
    NameDefect {
        name: "a:b".to_owned(),
        owner: if folder {
            Named::Folder { id: 7 }
        } else {
            Named::File {
                key: "<7>a:b".to_owned(),
            }
        },
        restriction,
    }
}

/// Every panel answer, encoded and read back, field for field.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "eleven answers with every field of each one populated; the length is the \
                  vocabulary's, and splitting it would hide which of the eleven a failure is in"
)]
fn every_panel_answer_round_trips() {
    use pdf_model::article::{Bead, Thread};
    use pdf_model::attachment::{Attachment as FileAttachment, Relationship};
    use pdf_model::metadata::{Information, Trapped};
    use pdf_model::page::Boundary;
    use pdf_model::thumbnail::Thumbnail;
    use pdf_model::viewer_preferences::{
        Direction as TextDirection, Duplex, Opening, PageLayout, PageMode, PrintScaling,
        ViewerPreferences,
    };
    use pdf_model::xmp::{Name as XmpName, Value as XmpValue, Xmp};
    use viewer_core::{AccessibilityNode, Character, Layer, PopupWindow, TextLine};

    // §12.3.3.
    let outline = a_populated_outline();
    let Reply::Outline(read) = round_trip(&Answer::Outline(outline.clone())) else {
        panic!("an outline comes back as one");
    };
    assert_eq!(read, outline, "an outline changed on the way through");

    // §8.11.4.3.
    let layers = vec![
        Layer::Collection {
            label: Some("a heading".to_owned()),
            children: vec![Layer::Group {
                group: ObjectId::new(12, 0),
                name: Some("a layer".to_owned()),
                on: false,
                locked: true,
            }],
        },
        Layer::Group {
            group: ObjectId::new(13, 1),
            name: None,
            on: true,
            locked: false,
        },
    ];
    let Reply::Layers(read) = round_trip(&Answer::Layers(layers.clone())) else {
        panic!("a layer order comes back as one");
    };
    assert_eq!(read, layers, "a layer order changed on the way through");

    // §7.11.4. The stream is the one thing that does not cross, which is why the expected
    // value is written out rather than derived from the input: an assertion that compared
    // the two types field by field could not have been written at all.
    let attachments = vec![FileAttachment {
        name: "readme.txt".to_owned(),
        file_name: Some("readme.txt".to_owned()),
        description: Some("what this is".to_owned()),
        media_type: Some("text/plain".to_owned()),
        size: Some(42),
        created: Some("D:20240101120000Z".to_owned()),
        modified: Some("D:20240202120000Z".to_owned()),
        checksum: Some(vec![0u8; 16]),
        relationship: Relationship::Other("Annex".to_owned()),
        stream: std::sync::Arc::new(pdf_syntax::Stream {
            dict: pdf_syntax::Dictionary::new(),
            data: b"hello".as_slice().into(),
            decryption_failed: false,
        }),
        thumbnail: None,
        payload: None,
    }];
    let Reply::Attachments(read) = round_trip(&Answer::Attachments(attachments)) else {
        panic!("an attachment list comes back as one");
    };
    assert_eq!(
        read,
        vec![crate::Attachment {
            name: "readme.txt".to_owned(),
            file_name: Some("readme.txt".to_owned()),
            description: Some("what this is".to_owned()),
            media_type: Some("text/plain".to_owned()),
            size: Some(42),
            created: Some("D:20240101120000Z".to_owned()),
            modified: Some("D:20240202120000Z".to_owned()),
            checksum: Some(vec![0u8; 16]),
            relationship: Relationship::Other("Annex".to_owned()),
        }],
        "an attachment changed on the way through"
    );

    // §12.3.5.
    let collection = a_populated_collection();
    // §12.3.5.1's resolved `/D` crosses beside the dictionary: `Initial::Embedded` is the
    // variant that carries a name, so a codec that dropped the name would show here.
    let initial = pdf_model::collection::Initial::Embedded("<1>letter.pdf".to_owned());
    // Table 153's `/Sort` crosses as its *answer*, the keys in order, so a codec that
    // dropped it would leave a panel sorting by nothing and saying so nowhere.
    let order = vec!["<1>letter.pdf".to_owned(), "notes.txt".to_owned()];
    let Reply::Collection {
        collection: read,
        initial: read_initial,
        order: read_order,
    } = round_trip(&Answer::Collection {
        collection: collection.clone(),
        initial: initial.clone(),
        order: order.clone(),
    })
    else {
        panic!("a collection comes back as one");
    };
    assert_eq!(*read, collection, "a collection changed on the way through");
    assert_eq!(
        read_initial, initial,
        "the initial document changed on the way through"
    );
    assert_eq!(
        read_order, order,
        "the /Sort order changed on the way through"
    );

    // §12.4.3.
    let threads = vec![Thread {
        id: ObjectId::new(20, 0),
        title: Some("a thread".to_owned()),
        beads: vec![
            Bead {
                id: ObjectId::new(21, 0),
                page: Some(ObjectId::new(3, 0)),
                rect: Some([0.0, 1.0, 2.0, 3.0]),
            },
            Bead {
                id: ObjectId::new(22, 0),
                page: None,
                rect: None,
            },
        ],
    }];
    let Reply::Articles(read) = round_trip(&Answer::Articles(threads.clone())) else {
        panic!("article threads come back as themselves");
    };
    assert_eq!(
        read, threads,
        "an article thread changed on the way through"
    );

    // §12.3.4.
    let thumbnail = Thumbnail {
        image: pdf_render::Image {
            width: 2,
            height: 3,
            data: (0u8..24).collect::<Vec<u8>>().into(),
            interpolate: true,
            // What `decode_thumbnail` states for every thumbnail; see its comment.
            sample_alpha: pdf_render::SampleAlpha::Shape,
        },
        permitted_colour_space: false,
        permitted_subtype: false,
    };
    let Reply::Thumbnail(read) = round_trip(&Answer::Thumbnail(thumbnail.clone())) else {
        panic!("a thumbnail comes back as one");
    };
    assert_eq!(read, thumbnail, "a thumbnail changed on the way through");

    // §14.3.3 and §14.3.2.
    let information = Information {
        title: Some("a title".to_owned()),
        author: Some("an author".to_owned()),
        subject: Some("a subject".to_owned()),
        keywords: Some("keywords".to_owned()),
        creator: Some("a creator".to_owned()),
        producer: Some("a producer".to_owned()),
        created: Some("D:20200101000000Z".to_owned()),
        modified: Some("D:20210101000000Z".to_owned()),
        trapped: Trapped::Fully,
    };
    let xmp = Xmp::from_properties(vec![
        (
            XmpName {
                namespace: "http://purl.org/dc/elements/1.1/".to_owned(),
                local: "title".to_owned(),
            },
            XmpValue::Alt(vec![
                (Some("x-default".to_owned()), "a title".to_owned()),
                (None, "no language".to_owned()),
            ]),
        ),
        (
            XmpName {
                namespace: "http://purl.org/dc/elements/1.1/".to_owned(),
                local: "creator".to_owned(),
            },
            XmpValue::Seq(vec!["one".to_owned(), "two".to_owned()]),
        ),
        (
            XmpName {
                namespace: String::new(),
                local: "unqualified".to_owned(),
            },
            XmpValue::Bag(vec!["a".to_owned()]),
        ),
        (
            XmpName {
                namespace: "urn:x".to_owned(),
                local: "simple".to_owned(),
            },
            XmpValue::Text("plain".to_owned()),
        ),
        (
            XmpName {
                namespace: "urn:x".to_owned(),
                local: "structured".to_owned(),
            },
            XmpValue::Structure,
        ),
    ]);
    let Reply::Properties {
        information: read_information,
        metadata,
    } = round_trip(&Answer::Properties {
        information: information.clone(),
        metadata: Some(Ok(xmp.clone())),
    })
    else {
        panic!("properties come back as properties");
    };
    assert_eq!(*read_information, information);
    assert_eq!(metadata, Some(Ok(xmp)));

    // Every `XmpError` this build names, including the four budgets whose `&'static str` is
    // reconstructed rather than allocated.
    for error in [
        pdf_model::xmp::XmpError::Undecodable,
        pdf_model::xmp::XmpError::TooLarge { bytes: 1 << 21 },
        pdf_model::xmp::XmpError::NotText,
        pdf_model::xmp::XmpError::Malformed {
            line: 3,
            column: 7,
            detail: "unexpected token".to_owned(),
        },
        pdf_model::xmp::XmpError::Unbalanced {
            detail: "<a></b>".to_owned(),
        },
        pdf_model::xmp::XmpError::TooMuch {
            what: "nesting depth",
        },
        pdf_model::xmp::XmpError::TooMuch { what: "properties" },
        pdf_model::xmp::XmpError::TooMuch {
            what: "array items",
        },
        pdf_model::xmp::XmpError::TooMuch {
            what: "value length",
        },
    ] {
        let Reply::Properties { metadata, .. } = round_trip(&Answer::Properties {
            information: Information::default(),
            metadata: Some(Err(error.clone())),
        }) else {
            panic!("properties come back as properties");
        };
        assert_eq!(metadata, Some(Err(error)));
    }

    // Table 29 and Table 147.
    let opening = Opening {
        mode: PageMode::UseAttachments,
        layout: PageLayout::TwoPageRight,
    };
    let Reply::Opening(read) = round_trip(&Answer::Opening(opening)) else {
        panic!("an opening pair comes back as one");
    };
    assert_eq!(read, opening);

    let preferences = ViewerPreferences {
        hide_toolbar: true,
        hide_menubar: true,
        hide_window_ui: true,
        fit_window: true,
        center_window: true,
        display_doc_title: true,
        non_full_screen_page_mode: PageMode::UseOptionalContent,
        direction: TextDirection::RightToLeft,
        view_area: Boundary::Media,
        view_clip: Boundary::Bleed,
        print_area: Boundary::Trim,
        print_clip: Boundary::Art,
        print_scaling: PrintScaling::NoScaling,
        duplex: Some(Duplex::FlipShortEdge),
        pick_tray_by_pdf_size: Some(true),
        print_page_range: vec![(1, 4), (9, 9)],
        num_copies: Some(3),
        enforce_print_scaling: true,
    };
    let Reply::Preferences(read) = round_trip(&Answer::Preferences(preferences.clone())) else {
        panic!("preferences come back as preferences");
    };
    assert_eq!(*read, preferences);

    // §12.5.6.14.
    let popups = vec![
        PopupWindow {
            annotation: ObjectId::new(30, 0),
            parent: Some(ObjectId::new(31, 0)),
            note: Some(ObjectId::new(31, 0)),
            quad: [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0],
            title: Some("a title".to_owned()),
            text: Some("a note".to_owned()),
            modified: Some("D:20240101000000Z".to_owned()),
            // Table 172's `/Subj` and `/CreationDate`, which cross with the rest.
            subject: Some("a subject".to_owned()),
            created: Some("D:20231231000000Z".to_owned()),
            colour: Some(pdf_render::Color {
                r: 0.1,
                g: 0.2,
                b: 0.3,
                a: 0.4,
            }),
            // Table 172's `/RC` with its formatting, every field of a run set (ADR 1642).
            rich: Some(pdf_model::popup::RichNote {
                paragraphs: vec![pdf_model::popup::RichParagraph {
                    align: Some(pdf_model::popup::RichAlign::Centre),
                    level: 2,
                    tag: Some(pdf_model::popup::RichRun {
                        text: "1.".to_owned(),
                        families: Vec::new(),
                        size: pdf_model::popup::Measure {
                            per_base: 1.0,
                            points: 0.0,
                        },
                        bold: false,
                        italic: false,
                        colour: None,
                        underlines: 0,
                        underline_by_word: false,
                        line_through: false,
                        rise: pdf_model::popup::Measure::default(),
                        letter_spacing: pdf_model::popup::RichSpacing::default(),
                        horizontal_scale: 1.0,
                        vertical_scale: 1.0,
                    }),
                    runs: vec![pdf_model::popup::RichRun {
                        text: "bold\nred".to_owned(),
                        families: vec!["Times".to_owned(), "serif".to_owned()],
                        size: pdf_model::popup::Measure {
                            per_base: 0.66,
                            points: 3.0,
                        },
                        bold: true,
                        italic: true,
                        colour: Some(pdf_render::Color {
                            r: 1.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        underlines: 2,
                        underline_by_word: true,
                        line_through: true,
                        rise: pdf_model::popup::Measure {
                            per_base: 0.31,
                            points: -1.0,
                        },
                        letter_spacing: pdf_model::popup::RichSpacing::OfSpace(0.25),
                        horizontal_scale: 1.5,
                        vertical_scale: 0.75,
                    }],
                    // The paragraph's tab stops, which cross with it (ADR 1666).
                    tab_interval: Some(pdf_model::popup::Measure {
                        per_base: 0.0,
                        points: 36.0,
                    }),
                    // Each of a leader's three patterns crosses with its stop (ADR 1679).
                    tab_stops: vec![
                        pdf_model::popup::RichTabStop {
                            align: pdf_model::popup::RichTabAlign::Decimal,
                            at: pdf_model::popup::Measure {
                                per_base: 2.0,
                                points: 1.5,
                            },
                            leader: None,
                        },
                        pdf_model::popup::RichTabStop {
                            align: pdf_model::popup::RichTabAlign::Right,
                            at: pdf_model::popup::Measure {
                                per_base: 0.0,
                                points: 144.0,
                            },
                            leader: Some(pdf_model::popup::RichLeader {
                                pattern: pdf_model::popup::RichLeaderPattern::Dots,
                                width: None,
                            }),
                        },
                        pdf_model::popup::RichTabStop {
                            align: pdf_model::popup::RichTabAlign::After,
                            at: pdf_model::popup::Measure {
                                per_base: 0.0,
                                points: 216.0,
                            },
                            leader: Some(pdf_model::popup::RichLeader {
                                pattern: pdf_model::popup::RichLeaderPattern::Rule {
                                    style: pdf_model::popup::RichRuleStyle::Dashed,
                                    thickness: Some(pdf_model::popup::Measure {
                                        per_base: 0.0,
                                        points: 0.5,
                                    }),
                                },
                                width: None,
                            }),
                        },
                        pdf_model::popup::RichTabStop {
                            align: pdf_model::popup::RichTabAlign::Left,
                            at: pdf_model::popup::Measure {
                                per_base: 0.0,
                                points: 288.0,
                            },
                            leader: Some(pdf_model::popup::RichLeader {
                                pattern: pdf_model::popup::RichLeaderPattern::Content(
                                    "-~".to_owned(),
                                ),
                                width: Some(pdf_model::popup::Measure {
                                    per_base: 1.0,
                                    points: 0.0,
                                }),
                            }),
                        },
                    ],
                }],
                unapplied: vec!["a font scale in a popup window".to_owned()],
            }),
            // §12.5.6.2's thread, which Table 172 makes part of what this window shows.
            replies: vec![
                pdf_model::popup::Comment {
                    annotation: ObjectId::new(33, 0),
                    parent: Some(ObjectId::new(34, 0)),
                    depth: 1,
                    title: Some("a reviewer".to_owned()),
                    text: Some("a reply".to_owned()),
                    modified: Some("D:20240102000000Z".to_owned()),
                    subject: Some("a reply's subject".to_owned()),
                    created: Some("D:20240101120000Z".to_owned()),
                    rich: None,
                },
                pdf_model::popup::Comment {
                    annotation: ObjectId::new(35, 0),
                    parent: None,
                    depth: 2,
                    title: None,
                    text: None,
                    modified: None,
                    subject: None,
                    created: None,
                    rich: None,
                },
            ],
        },
        PopupWindow {
            annotation: ObjectId::new(32, 0),
            parent: None,
            note: None,
            quad: [0.0; 8],
            title: None,
            text: None,
            modified: None,
            subject: None,
            created: None,
            colour: None,
            rich: None,
            replies: Vec::new(),
        },
    ];
    let Reply::Popups(read) = round_trip(&Answer::Popups(popups.clone())) else {
        panic!("popups come back as popups");
    };
    assert_eq!(read, popups);

    // §14.7.
    let nodes = vec![
        AccessibilityNode {
            parent: None,
            role: "Document".to_owned(),
            standard: Ok(pdf_model::structure::StandardType::Document),
            name: String::new(),
            substituted: false,
            titled: false,
            language: Some("en-GB".to_owned()),
            quads: Vec::new(),
            header_scope: None,
            cell: None,
            summary: None,
            short: None,
            bounds: None,
            allocation: None,
            artifact: None,
            control: None,
            value: None,
            value_lines: Vec::new(),
            annotation: None,
            headers: Vec::new(),
            continues_a_list: false,
            continued_from: None,
            lines: Vec::new(),
            drawn: None,
            enclosed_a_refusal: false,
            decoration: pdf_model::structure::TextDecoration::None,
            decoration_colour: None,
            position: pdf_model::structure::TextPosition::Normal,
        },
        // A `Table` whose `/Summary` crossed, because §14.8.5.7 wrote it "[f]or use in
        // non-visual rendering" and the only non-visual renderer is on the host's side of
        // this boundary.
        AccessibilityNode {
            parent: Some(0),
            role: "Table".to_owned(),
            standard: Ok(pdf_model::structure::StandardType::Table),
            name: String::new(),
            substituted: false,
            titled: false,
            language: None,
            quads: Vec::new(),
            header_scope: None,
            cell: None,
            summary: Some("sales by region and quarter".to_owned()),
            short: None,
            bounds: None,
            allocation: None,
            artifact: None,
            control: None,
            value: None,
            value_lines: Vec::new(),
            annotation: None,
            headers: Vec::new(),
            continues_a_list: false,
            continued_from: None,
            lines: Vec::new(),
            drawn: None,
            enclosed_a_refusal: false,
            decoration: pdf_model::structure::TextDecoration::None,
            decoration_colour: None,
            position: pdf_model::structure::TextPosition::Normal,
        },
        // A `Figure` marks no text, so Table 379's `/BBox` is the only place it has — and
        // a rectangle that did not cross would leave a magnifier nothing to point at.
        AccessibilityNode {
            parent: Some(0),
            role: "Figure".to_owned(),
            standard: Ok(pdf_model::structure::StandardType::Figure),
            name: "a chart of sales".to_owned(),
            substituted: true,
            titled: false,
            language: None,
            quads: vec![[0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0]],
            header_scope: None,
            cell: None,
            summary: None,
            short: None,
            bounds: Some([8.0, 9.0, 10.0, 11.0]),
            allocation: None,
            artifact: None,
            control: None,
            value: None,
            value_lines: Vec::new(),
            annotation: None,
            headers: Vec::new(),
            continues_a_list: false,
            continued_from: None,
            lines: Vec::new(),
            // §14.8.3.3's content rectangle beside the stated one, because the host chooses
            // between them and a boundary that carried only one would make that choice here.
            drawn: Some([7.5, 8.5, 10.5, 11.5]),
            // And the refusal flag, because an element the host cannot point at and a host
            // that cannot say why is exactly what this boundary would otherwise hide.
            enclosed_a_refusal: true,
            decoration: pdf_model::structure::TextDecoration::None,
            decoration_colour: None,
            position: pdf_model::structure::TextPosition::Normal,
        },
        // A `TH` whose axis Table 384 states, because a header cell that crossed as a
        // column's when the document called it a row's would be read out backwards.
        AccessibilityNode {
            parent: Some(0),
            role: "TH".to_owned(),
            standard: Ok(pdf_model::structure::StandardType::TableHeader),
            name: "Region".to_owned(),
            substituted: false,
            titled: false,
            language: None,
            quads: Vec::new(),
            header_scope: Some(pdf_model::structure::HeaderScope::Row),
            // §14.8.5.4.5's grid place, which is what a host equalises a row's cells by: a
            // header at row 0 column 1 spanning two columns, in the page's second table.
            cell: Some(viewer_core::TableCell {
                table: 1,
                place: pdf_model::structure::CellPlacement {
                    row: 0,
                    column: 1,
                    row_span: 1,
                    column_span: 2,
                },
                progression: pdf_model::structure::BlockProgression::RightToLeft,
            }),
            summary: None,
            // And its `/Short`, the author's abbreviation for the repetition a screen
            // reader makes of it in front of every cell it describes.
            short: Some("Reg.".to_owned()),
            bounds: None,
            allocation: None,
            artifact: None,
            control: None,
            value: None,
            value_lines: Vec::new(),
            annotation: None,
            headers: Vec::new(),
            continues_a_list: false,
            continued_from: None,
            lines: Vec::new(),
            drawn: None,
            enclosed_a_refusal: false,
            decoration: pdf_model::structure::TextDecoration::None,
            decoration_colour: None,
            position: pdf_model::structure::TextPosition::Normal,
        },
        // §14.8.4.7.2's `Form`, which is one widget annotation: the control it names is
        // what makes a screen reader say "check box, ticked" instead of "group", so a
        // field that did not cross would be the whole of this feature lost on this
        // boundary.
        AccessibilityNode {
            parent: Some(0),
            role: "Form".to_owned(),
            standard: Ok(pdf_model::structure::StandardType::Form),
            name: String::new(),
            substituted: false,
            titled: false,
            language: None,
            quads: Vec::new(),
            header_scope: None,
            cell: None,
            summary: None,
            short: None,
            bounds: Some([12.0, 13.0, 14.0, 15.0]),
            allocation: None,
            artifact: None,
            control: Some(pdf_model::form::Control::CheckBox { on: true }),
            value: None,
            value_lines: Vec::new(),
            annotation: Some(ObjectId::new(7, 0)),
            headers: Vec::new(),
            continues_a_list: false,
            continued_from: None,
            lines: Vec::new(),
            drawn: None,
            enclosed_a_refusal: false,
            decoration: pdf_model::structure::TextDecoration::None,
            decoration_colour: None,
            position: pdf_model::structure::TextPosition::Normal,
        },
        // And a `TD` that §14.8.4.8.3's search gave the header beside it, because a cell
        // whose headers did not cross would be announced with nothing in front of it.
        AccessibilityNode {
            parent: Some(0),
            role: "TD".to_owned(),
            standard: Ok(pdf_model::structure::StandardType::TableData),
            name: "north".to_owned(),
            substituted: false,
            titled: false,
            language: None,
            quads: Vec::new(),
            header_scope: None,
            cell: None,
            summary: None,
            short: None,
            bounds: None,
            allocation: None,
            artifact: None,
            control: None,
            value: None,
            value_lines: Vec::new(),
            annotation: None,
            headers: vec![3],
            continues_a_list: false,
            continued_from: None,
            // The lines a caret moves through, which are the only field of this answer whose
            // two halves have to agree: the characters' byte counts sum to the text's length,
            // and `read_lines` refuses a message where they do not. A `TD` whose lines did not
            // cross would be a cell an assistive technology could hear and not move through.
            lines: vec![TextLine {
                text: "north".to_owned(),
                characters: vec![
                    Character {
                        bytes: 1,
                        bounds: [0.0, 0.0, 4.0, 10.0],
                    },
                    Character {
                        bytes: 4,
                        bounds: [4.0, 0.0, 20.0, 10.0],
                    },
                ],
            }],
            // A cell whose only place is what its content drew, which is the population
            // §14.8.3.3's content rectangle exists for and the one this field carries.
            drawn: Some([0.0, 0.0, 20.0, 10.0]),
            enclosed_a_refusal: false,
            decoration: pdf_model::structure::TextDecoration::None,
            decoration_colour: None,
            position: pdf_model::structure::TextPosition::Normal,
        },
        // Two `L` elements, the second continuing the first: §14.8.5.5's two entries live in
        // §14.7.6's attribute objects, which only the confined side reads, and nothing a host
        // can see says a list carries an earlier one on.
        AccessibilityNode {
            parent: Some(0),
            role: "L".to_owned(),
            standard: Ok(pdf_model::structure::StandardType::List),
            name: String::new(),
            substituted: false,
            titled: false,
            language: None,
            quads: Vec::new(),
            header_scope: None,
            cell: None,
            summary: None,
            short: None,
            bounds: None,
            allocation: None,
            artifact: None,
            control: None,
            value: None,
            value_lines: Vec::new(),
            annotation: None,
            headers: Vec::new(),
            continues_a_list: false,
            continued_from: None,
            lines: Vec::new(),
            drawn: None,
            enclosed_a_refusal: false,
            decoration: pdf_model::structure::TextDecoration::None,
            decoration_colour: None,
            position: pdf_model::structure::TextPosition::Normal,
        },
        AccessibilityNode {
            parent: Some(0),
            role: "L".to_owned(),
            standard: Ok(pdf_model::structure::StandardType::List),
            name: String::new(),
            substituted: false,
            titled: false,
            language: None,
            quads: Vec::new(),
            header_scope: None,
            cell: None,
            summary: None,
            short: None,
            bounds: None,
            allocation: None,
            artifact: None,
            control: None,
            value: None,
            value_lines: Vec::new(),
            annotation: None,
            headers: Vec::new(),
            continues_a_list: true,
            continued_from: Some(6),
            lines: Vec::new(),
            drawn: None,
            enclosed_a_refusal: false,
            decoration: pdf_model::structure::TextDecoration::None,
            decoration_colour: None,
            position: pdf_model::structure::TextPosition::Normal,
        },
        // §14.8.4.8.7's `Artifact`, with Table 385's three entries and §14.8.5.4.5's second
        // rectangle. Both are read out of §14.7.6's attribute objects, which only the
        // confined side sees, so a wire that dropped either would leave a host with a
        // nameless container and a paragraph that reserved no room.
        AccessibilityNode {
            parent: Some(0),
            role: "Artifact".to_owned(),
            standard: Ok(pdf_model::structure::StandardType::Artifact),
            name: String::new(),
            substituted: false,
            titled: false,
            language: None,
            quads: Vec::new(),
            header_scope: None,
            cell: None,
            summary: None,
            short: None,
            bounds: Some([10.0, 5.0, 90.0, 20.0]),
            allocation: Some([10.0, 1.0, 90.0, 26.0]),
            artifact: Some(pdf_model::structure::Artifact {
                kind: Some(pdf_model::structure::ArtifactKind::Pagination),
                subtype: Some("PageNum".to_owned()),
                bbox: Some([10.0, 5.0, 90.0, 20.0]),
                attached: [false, true, false, false],
            }),
            control: None,
            value: None,
            value_lines: Vec::new(),
            annotation: None,
            headers: Vec::new(),
            continues_a_list: false,
            continued_from: None,
            lines: Vec::new(),
            drawn: None,
            enclosed_a_refusal: false,
            decoration: pdf_model::structure::TextDecoration::None,
            decoration_colour: None,
            position: pdf_model::structure::TextPosition::Normal,
        },
        // A `Form` naming a text field, with §12.7.4.3's value beside its control: what a screen
        // reader reads as the field's contents (ADR 1489). A wire that dropped it would publish
        // a field filled on the confined side as an empty one on the host's.
        AccessibilityNode {
            parent: Some(0),
            role: "Form".to_owned(),
            standard: Ok(pdf_model::structure::StandardType::Form),
            name: "A".to_owned(),
            substituted: false,
            titled: false,
            language: None,
            quads: Vec::new(),
            header_scope: None,
            cell: None,
            summary: None,
            short: None,
            bounds: Some([12.0, 30.0, 64.0, 42.0]),
            allocation: None,
            artifact: None,
            control: Some(pdf_model::form::Control::Text(
                pdf_model::form::TextControl::default(),
            )),
            value: Some(pdf_model::view::ShownValue {
                text: "1".to_owned(),
                obscured: false,
            }),
            // ADR 1501: where the layout placed the character crosses beside the value.
            value_lines: vec![TextLine {
                text: "1".to_owned(),
                characters: vec![Character {
                    bytes: 1,
                    bounds: [14.0, 31.0, 20.0, 41.0],
                }],
            }],
            annotation: Some(ObjectId::new(9, 0)),
            headers: Vec::new(),
            continues_a_list: false,
            continued_from: None,
            lines: Vec::new(),
            drawn: None,
            enclosed_a_refusal: false,
            decoration: pdf_model::structure::TextDecoration::None,
            decoration_colour: None,
            position: pdf_model::structure::TextPosition::Normal,
        },
        // Two elements that are not standard types, each with the reason the reading side
        // gave, because the reason is what a host cannot work out from the name: a foreign
        // namespace's `Table` is a homonym, and it must not arrive as a table (ADR 2028).
        AccessibilityNode {
            parent: Some(0),
            role: "Table".to_owned(),
            standard: Err(pdf_model::structure::Nonstandard::Foreign),
            name: String::new(),
            substituted: false,
            titled: false,
            language: None,
            quads: Vec::new(),
            header_scope: None,
            cell: None,
            summary: None,
            short: None,
            bounds: None,
            allocation: None,
            artifact: None,
            control: None,
            value: None,
            value_lines: Vec::new(),
            annotation: None,
            headers: Vec::new(),
            continues_a_list: false,
            continued_from: None,
            lines: Vec::new(),
            drawn: None,
            enclosed_a_refusal: false,
            decoration: pdf_model::structure::TextDecoration::None,
            decoration_colour: None,
            position: pdf_model::structure::TextPosition::Normal,
        },
        AccessibilityNode {
            parent: Some(0),
            role: "Left".to_owned(),
            standard: Err(pdf_model::structure::Nonstandard::Circular),
            name: String::new(),
            substituted: false,
            titled: false,
            language: None,
            quads: Vec::new(),
            header_scope: None,
            cell: None,
            summary: None,
            short: None,
            bounds: None,
            allocation: None,
            artifact: None,
            control: None,
            value: None,
            value_lines: Vec::new(),
            annotation: None,
            headers: Vec::new(),
            continues_a_list: false,
            continued_from: None,
            lines: Vec::new(),
            drawn: None,
            enclosed_a_refusal: false,
            // Table 380's three, away from their defaults so that the round trip is seen to
            // carry each (ADR 2384).
            decoration: pdf_model::structure::TextDecoration::LineThrough,
            decoration_colour: Some([0.25, 0.5, 1.0]),
            position: pdf_model::structure::TextPosition::Sub,
        },
    ];
    // Two pages, because a column crosses several and a decoder that read one page's list
    // and stopped would pass with one. The second is deliberately **untagged**: an empty
    // list is an answer (§14.7 lets a page state no structure) and a wire that dropped the
    // entry would turn "this page says nothing" into "there is no such page".
    // Its widgets cross as a list of their own, ADR 1369's, so the untagged page carries
    // the nodes the tagged one does in the other list.
    let pages = vec![
        viewer_core::PageStructure {
            page: 3,
            nodes: nodes.clone(),
            widgets: Vec::new(),
            tagging: viewer_core::Tagging::Reached,
        },
        viewer_core::PageStructure {
            page: 4,
            nodes: Vec::new(),
            widgets: nodes.clone(),
            tagging: viewer_core::Tagging::Untagged,
        },
        // ADR 1393: a tagged document's page its structure reaches nothing on is empty too,
        // and has to arrive as that rather than as the untagged page above.
        viewer_core::PageStructure {
            page: 5,
            nodes: Vec::new(),
            widgets: Vec::new(),
            tagging: viewer_core::Tagging::Unreached { marked: true },
        },
    ];
    let Reply::Accessibility(read) = round_trip(&Answer::Accessibility(pages.clone())) else {
        panic!("a structure tree comes back as one");
    };
    assert_eq!(read.len(), pages.len());
    for (read, ours) in read.iter().zip(&pages) {
        assert_eq!(read.page, ours.page);
        assert_eq!(read.nodes, ours.nodes);
        assert_eq!(read.widgets, ours.widgets);
        assert_eq!(read.tagging, ours.tagging);
    }

    // §12.7, and one of every control §12.7.5 defines: a host on this boundary builds the
    // same form as one off it or it builds a different program.
    let fields = a_populated_form();
    let Reply::Fields(read) = round_trip(&Answer::Fields(fields.clone())) else {
        panic!("form fields come back as form fields");
    };
    assert_eq!(read, fields);

    // The nine per-code counts, each a different number: an encoder that wrote them in the
    // wrong order would pass with any two of them equal, and a host reading "3 codes no
    // /ToUnicode named" for "3 codes an Identity ordering left unaddressable" would be told
    // the wrong thing about whose gap it is. The page's own number is distinct from all nine
    // for the same reason.
    let shortfall = pdf_model::content::Shortfall {
        unnamed: pdf_model::content::UnnamedCodes {
            empty_mapping: 1,
            incomplete_to_unicode: 2,
            unlisted_name: 3,
            unnamed_cid: 4,
            unaddressable_cid: 5,
            unnamed_glyph: 6,
        },
        without_a_glyph: 7,
        reaching_a_blank_glyph: 8,
        without_a_vertical_form: 9,
    };
    let counted = vec![viewer_core::PageReadback {
        page: 10,
        shortfall,
    }];
    let Reply::Readback(read) = round_trip(&Answer::Readback(counted)) else {
        panic!("a readback shortfall comes back as one");
    };
    let [read] = read.as_slice() else {
        panic!("one page's counts crossed");
    };
    assert_eq!(read.page, 10);
    assert_eq!(read.shortfall, shortfall);
    assert_eq!(read.shortfall.unnamed.total(), 21);

    // And the sentences, per page, with the pages named: a status bar under a column that
    // took the first entry for all of them would say one page's refusals about four.
    let first: Vec<String> = vec!["a shading this reader could not draw".to_owned()];
    let second: Vec<String> = vec!["one font".to_owned(), "one image".to_owned()];
    let said = vec![
        viewer_core::PageReports {
            page: 2,
            notes: &first,
        },
        viewer_core::PageReports {
            page: 5,
            notes: &second,
        },
    ];
    let Reply::Reports(read) = round_trip(&Answer::Reports(said)) else {
        panic!("reports come back as reports");
    };
    assert_eq!(
        read,
        vec![
            crate::Reported {
                page: 2,
                notes: first,
            },
            crate::Reported {
                page: 5,
                notes: second,
            },
        ]
    );
}

/// One field per control §12.7.5 defines, with no default among them.
///
/// Seven, because §12.7.5.2 splits its own type three ways and Table 226 lets a file state no
/// `/FT` at all. Every flag is the opposite of the standard's default, every `Option` is
/// `Some` and every list has more than one element — see the note on
/// [`a_populated_outline`]: a round trip over defaults would pass with an encoder that wrote
/// nothing.
fn a_populated_form() -> Vec<viewer_core::FormField> {
    use pdf_model::form::{Choice, ChoiceControl, Control, TextControl};
    use pdf_model::view::FieldName;
    use viewer_core::{FormField, FormWidget};

    let widget = |serial: u16, on_state: Option<&str>, export: Option<&str>| FormWidget {
        annotation: ObjectId::new(u32::from(serial), 0),
        quad: [
            f32::from(serial),
            1.0,
            2.0,
            3.0,
            4.0,
            5.0,
            6.0,
            f32::from(serial),
        ],
        on_state: on_state.map(str::to_owned),
        export: export.map(str::to_owned),
        on: on_state.is_some(),
    };
    let field = |name: &str, control: Control, widgets: Vec<FormWidget>| FormField {
        name: FieldName {
            qualified: format!("outer.{name}"),
            alternative: Some(format!("the {name}")),
        },
        partial: name.to_owned(),
        control,
        value: Some(pdf_model::view::ShownValue {
            text: format!("{name}'s value"),
            obscured: false,
        }),
        displayed: Some(format!("{name}'s value, formatted")),
        read_only: true,
        required: true,
        no_export: true,
        widgets,
    };

    vec![
        field(
            "text",
            Control::Text(TextControl {
                multiline: true,
                password: true,
                file_select: true,
                do_not_spell_check: true,
                do_not_scroll: true,
                comb: Some(9),
                max_len: Some(12),
                rich_text: true,
            }),
            vec![widget(40, None, None), widget(41, None, None)],
        ),
        field(
            "choice",
            Control::Choice(ChoiceControl {
                combo: true,
                editable: true,
                multi_select: true,
                do_not_spell_check: true,
                commit_on_selection: true,
                options: vec![
                    Choice {
                        export: Some("r".to_owned()),
                        label: "Red".to_owned(),
                    },
                    Choice {
                        export: None,
                        label: "Blue".to_owned(),
                    },
                ],
                selected: vec![0, 1],
                top: 1,
            }),
            vec![widget(42, None, None)],
        ),
        field(
            "box",
            Control::CheckBox { on: true },
            vec![widget(43, Some("Yes"), Some("yes"))],
        ),
        field(
            "radio",
            Control::RadioButton {
                on: true,
                no_toggle_to_off: true,
                in_unison: true,
            },
            vec![
                widget(44, Some("visa"), Some("Visa")),
                widget(45, None, None),
            ],
        ),
        field("push", Control::PushButton, vec![widget(46, None, None)]),
        field("sign", Control::Signature, vec![widget(47, None, None)]),
        field("odd", Control::Unstated, vec![widget(48, None, None)]),
    ]
}

/// Encodes an answer and reads it back, failing loudly rather than returning a `Result`.
fn round_trip(answer: &Answer<'_>) -> Reply {
    let encoded = encode_answer(answer, &Marks::default()).expect("this answer crosses");
    decode_answer(&encoded).expect("what was written reads back")
}

/// §12.10's position crosses in each of Table 269's `/DCS` shapes — none named, degrees, an
/// easting and a northing in the system's own unit, and the refusal's sentence — and a
/// point that is refused crosses as its sentence (ADRs 1593, 1678).
#[test]
fn a_located_point_crosses_in_every_display_shape() {
    use pdf_model::geospatial::{Displayed, GeographicPosition};
    let at = |display| viewer_core::Located::At {
        latitude: 47.5,
        longitude: 9.0,
        display,
        departure: 0.000_25,
    };
    for located in [
        None,
        Some(at(None)),
        Some(at(Some(Ok(Displayed::Geographic(GeographicPosition {
            latitude: 47.5,
            longitude: -0.5,
        }))))),
        Some(at(Some(Ok(Displayed::Projected {
            easting: 500_000.0,
            northing: 5_260_729.733,
            unit: "Meter".to_owned(),
        })))),
        Some(at(Some(Err("the datums differ".to_owned())))),
        Some(viewer_core::Located::Refused(
            "the points are shaped as degrees".to_owned(),
        )),
    ] {
        let traced = pdf_model::measurement::Traced::default();
        let reply = round_trip(&Answer::Measured {
            traced: traced.clone(),
            located: located.clone(),
        });
        assert_eq!(
            reply,
            Reply::Measured {
                traced: Box::new(traced),
                located,
            }
        );
    }
}
