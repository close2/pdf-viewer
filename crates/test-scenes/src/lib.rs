//! Canonical display lists shared by backend tests and the comparison harness.
//!
//! These exist so that every backend is tested against *identical* input. If each
//! backend's tests built their own scenes, the two could drift apart, and a
//! cross-backend comparison would no longer be evidence of anything: a difference
//! could just as easily mean the scenes differed as that a backend was wrong.
//!
//! This is a normal dependency rather than a dev-dependency, because the comparison
//! harness in `tools/` will use the same scenes to check backends against external
//! reference renderers.

#![forbid(unsafe_code)]

mod ocr;
mod type4;

pub use ocr::{
    OCR_ASCENT, OCR_BASELINE, OCR_CID_COUNT, OCR_DESCENT, OCR_FIRST_WORD, OCR_FONT_SIZE, OCR_PAGE,
    OCR_SECOND_WORD, OCR_SECOND_X, OCR_TEXT_X, OcrFont, ocr_advance_for_gid, ocr_gid_for_cid,
    scanned_ocr_pdf,
};
pub use type4::{TYPE4_PAGE, TYPE4_SPLIT, Type4Comments, type4_comment_pair};

use std::sync::Arc;

use pdf_render::display_list::Clip;
use pdf_render::{
    BlendMode, Color, Command, DisplayList, FillRule, Paint, Path, PathCommand, Point, Size,
    SoftMask, SoftMaskKind, Stroke, Transform,
};

/// A4 in PDF user-space units: 210mm x 297mm at 72 units per inch.
pub const A4: Size = Size {
    width: 595.0,
    height: 842.0,
};

/// Opaque red.
pub const RED: Color = Color {
    r: 1.0,
    g: 0.0,
    b: 0.0,
    a: 1.0,
};
/// Opaque green.
pub const GREEN: Color = Color {
    r: 0.0,
    g: 1.0,
    b: 0.0,
    a: 1.0,
};
/// Opaque blue.
pub const BLUE: Color = Color {
    r: 0.0,
    g: 0.0,
    b: 1.0,
    a: 1.0,
};

/// Builds an axis-aligned rectangle as a closed path.
#[must_use]
pub fn rect(x0: f32, y0: f32, x1: f32, y1: f32) -> Path {
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(x0, y0)));
    path.push(PathCommand::LineTo(Point::new(x1, y0)));
    path.push(PathCommand::LineTo(Point::new(x1, y1)));
    path.push(PathCommand::LineTo(Point::new(x0, y1)));
    path.push(PathCommand::Close);
    path
}

/// A scene exercising an axis-aligned fill, a nested clip, and a thick stroke.
///
/// Deliberately built from axis-aligned edges at integer coordinates, so that with
/// antialiasing disabled a fill covers whole pixels exactly and assertions can be
/// exact rather than approximate. The one diagonal is in [`diagonal_stroke`], where
/// antialiasing differences are the point.
///
/// Contents, in page coordinates (origin bottom-left):
///
/// - a red square from (100,100) to (300,300);
/// - a green square from (400,400) to (500,500), clipped to its lower-left quarter, so
///   that a backend ignoring clips is caught;
/// - a 10-unit blue stroke along y=600.
///
/// # Panics
///
/// Cannot panic in practice: the only fallible step is registering a clip, which
/// fails only past `u32::MAX` clips, and this scene registers one.
#[must_use]
#[expect(
    clippy::expect_used,
    reason = "a single add_clip cannot exhaust a u32 clip index; a Result here would \
              push an impossible error case onto every caller"
)]
pub fn basic() -> DisplayList {
    let mut list = DisplayList::new(A4);

    list.push(Command::Fill {
        path: Arc::new(rect(100.0, 100.0, 300.0, 300.0)),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Solid(RED),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });

    let clip = list
        .add_clip(Clip {
            path: rect(400.0, 400.0, 450.0, 450.0),
            transform: Transform::IDENTITY,
            fill_rule: FillRule::NonZero,
            parent: None,
        })
        .expect("a single clip is always addressable");

    list.push(Command::Fill {
        path: Arc::new(rect(400.0, 400.0, 500.0, 500.0)),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Solid(GREEN),
        clip: Some(clip),
        mask: None,
        blend: BlendMode::Normal,
    });

    let mut line = Path::new();
    line.push(PathCommand::MoveTo(Point::new(50.0, 600.0)));
    line.push(PathCommand::LineTo(Point::new(545.0, 600.0)));
    list.push(Command::Stroke {
        path: Arc::new(line),
        transform: Transform::IDENTITY,
        stroke: Stroke {
            width: 10.0,
            ..Stroke::default()
        },
        paint: Paint::Solid(BLUE),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });

    list
}

/// A transparency group over a backdrop, with its own constant alpha and blend mode.
///
/// ISO 32000-2 §11.4.1: a group's elements are composited to one colour and opacity and the
/// result painted once. A backend that instead applies the group's alpha to each element
/// paints the band where the two squares overlap twice, and one that ignores the group
/// entirely paints it opaque — so the scene fails in the axis the defect moves.
///
/// It fails at the right *magnitude* too, which is the harder half: the group covers about a
/// fifth of the page, so a wrong overlap is tens of thousands of channels rather than the few
/// hundred a corner-sized scene would give, and cannot pass under
/// `MAX_DIFFERING_FRACTION`. Half of it lies over the green backdrop and half over the
/// unpainted page, which is also where §11.4.7's page group shows: a blend mode over an
/// unpainted area sees nothing there, not the medium's white.
#[must_use]
pub fn transparency_group() -> DisplayList {
    let mut list = DisplayList::new(A4);

    list.push(Command::Fill {
        path: Arc::new(rect(50.0, 400.0, 545.0, 700.0)),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Solid(GREEN),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });

    let elements = vec![
        Command::Fill {
            path: Arc::new(rect(100.0, 200.0, 400.0, 500.0)),
            transform: Transform::IDENTITY,
            fill_rule: FillRule::NonZero,
            paint: Paint::Solid(RED),
            clip: None,
            mask: None,
            blend: BlendMode::Normal,
        },
        Command::Fill {
            path: Arc::new(rect(250.0, 350.0, 550.0, 650.0)),
            transform: Transform::IDENTITY,
            fill_rule: FillRule::NonZero,
            paint: Paint::Solid(BLUE),
            clip: None,
            mask: None,
            blend: BlendMode::Normal,
        },
    ];

    list.push(Command::Group {
        commands: elements,
        alpha: 0.5,
        clip: None,
        mask: None,
        blend: BlendMode::Multiply,
        isolated: true,
        knockout: false,
        alpha_is_shape: true,
        blending: None,
    });

    list
}

/// A knockout group whose elements overlap, half of them transparent (§11.4.6).
///
/// ISO 32000-2 §11.4.6: "[i]n a knockout group, each individual element shall be composited
/// with the group's initial backdrop rather than with the stack of preceding elements in the
/// group", so "[a]t any given point, only the topmost object enclosing the point shall
/// contribute to the result colour and opacity of the group as a whole".
///
/// # What this scene can fail at, and what it cannot
///
/// Knockout and ordinary compositing agree wherever the upper element is opaque and blends
/// Normal, which is why the report this implementation replaced fired on a narrower
/// condition than "the group is a knockout group". A scene of opaque squares would therefore
/// pass with knockout unimplemented — the shape of trap 2's magnitude lesson, one level up.
///
/// So the upper element of each overlapping pair *composites*: the first pair overlaps a
/// half-transparent blue over an opaque red, where knockout shows the page through the
/// overlap and ordinary compositing shows purple; the second overlaps a `Multiply` green
/// over an opaque red, where knockout shows plain green and ordinary compositing shows the
/// product. Both bands are ~150×150 points, which is tens of thousands of channels rather
/// than an edge's worth.
#[must_use]
pub fn knockout_group() -> DisplayList {
    /// A half-transparent blue: the alpha is *opacity*, which is what makes it legal to
    /// draw a knockout element with Porter-Duff Source.
    const HALF_BLUE: Color = Color {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 0.5,
    };

    let mut list = DisplayList::new(A4);

    // A backdrop, so that "the page shows through" is a colour rather than nothing: the
    // group's own initial backdrop is transparent either way, and this is what it is
    // composited *onto* afterwards.
    list.push(Command::Fill {
        path: Arc::new(rect(40.0, 100.0, 555.0, 750.0)),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Solid(GREEN),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });

    let fill = |path, paint, blend| Command::Fill {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Solid(paint),
        clip: None,
        mask: None,
        blend,
    };

    // A diagonal edge, so that the comparison covers partially covered pixels rather than
    // only pixel-aligned ones. Both backends reach §11.4.6 through their own library's
    // arithmetic and they are not the same arithmetic at a fractional coverage — see
    // `render-gpu`'s `knock_out` for the exact difference — so the scene has to contain an
    // edge that is not axis-aligned or the agreement would be about the easy half.
    let mut wedge = Path::new();
    wedge.push(PathCommand::MoveTo(Point::new(430.0, 130.0)));
    wedge.push(PathCommand::LineTo(Point::new(540.0, 430.0)));
    wedge.push(PathCommand::LineTo(Point::new(430.0, 430.0)));
    wedge.push(PathCommand::Close);

    list.push(Command::Group {
        commands: vec![
            fill(rect(60.0, 450.0, 310.0, 700.0), RED, BlendMode::Normal),
            fill(
                rect(160.0, 550.0, 410.0, 730.0),
                HALF_BLUE,
                BlendMode::Normal,
            ),
            fill(rect(60.0, 130.0, 310.0, 380.0), RED, BlendMode::Normal),
            fill(rect(160.0, 200.0, 410.0, 430.0), GREEN, BlendMode::Multiply),
            fill(rect(420.0, 140.0, 550.0, 420.0), RED, BlendMode::Normal),
            fill(wedge, HALF_BLUE, BlendMode::Normal),
        ],
        alpha: 1.0,
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
        isolated: true,
        knockout: true,
        alpha_is_shape: false,
        blending: None,
    });

    list
}

/// A knockout group whose elements state their shapes apart from their alpha (§11.4.6).
///
/// [`knockout_group`] is the case where an element's shape *is* the coverage it is drawn
/// with. This is the other one, and the clause names it:
///
/// > The existence of the knockout feature is the main reason for maintaining a separate
/// > shape value rather than only a single alpha that combines shape and opacity.
///
/// Each pair here is an opaque red rectangle with a **nested group** over it, painted at half
/// alpha. §11.4.6 knocks the red out within the *group's* shape — the union of its elements'
/// shapes, which is 1 wherever the group marks — and then adds the group's half-opaque
/// result, so the overlap shows half blue over the page's green backdrop. A backend that read
/// the shape off the alpha would knock out only half of the red and produce a purple band; a
/// backend that composited the second stage with source-over rather than addition would
/// produce a quarter of the red instead of none.
///
/// The second pair's group holds a wedge, for [`knockout_group`]'s reason: the two backends
/// reach the pair of operators through different libraries, and an axis-aligned scene would
/// hold them to the easy half.
#[must_use]
pub fn knockout_stated_shape() -> DisplayList {
    let mut list = DisplayList::new(A4);

    // The page behind the group, so that "the red is gone" is a colour rather than nothing.
    list.push(Command::Fill {
        path: Arc::new(rect(40.0, 100.0, 555.0, 750.0)),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Solid(GREEN),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });

    let fill = |path: Path, paint| Command::Fill {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Solid(paint),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    };
    // A group of one opaque mark, painted at `alpha`, beside the shape it knocks out with:
    // the same mark at full opacity, which is §11.6.4.2's shape of the group's element and
    // therefore of the group.
    let shaped = |path: Path, alpha: f32| Command::Shaped {
        object: Box::new(Command::Group {
            commands: vec![fill(path.clone(), BLUE)],
            alpha,
            clip: None,
            mask: None,
            blend: BlendMode::Normal,
            isolated: true,
            knockout: false,
            alpha_is_shape: true,
            blending: None,
        }),
        shape: Box::new(Command::Group {
            commands: vec![fill(path, Color::WHITE)],
            alpha: 1.0,
            clip: None,
            mask: None,
            blend: BlendMode::Normal,
            isolated: true,
            knockout: false,
            alpha_is_shape: true,
            blending: None,
        }),
    };

    let mut wedge = Path::new();
    wedge.push(PathCommand::MoveTo(Point::new(160.0, 200.0)));
    wedge.push(PathCommand::LineTo(Point::new(410.0, 200.0)));
    wedge.push(PathCommand::LineTo(Point::new(410.0, 430.0)));
    wedge.push(PathCommand::Close);

    list.push(Command::Group {
        commands: vec![
            fill(rect(60.0, 450.0, 310.0, 700.0), RED),
            shaped(rect(160.0, 550.0, 410.0, 730.0), 0.5),
            fill(rect(60.0, 130.0, 310.0, 380.0), RED),
            shaped(wedge, 0.5),
        ],
        alpha: 1.0,
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
        isolated: true,
        knockout: true,
        alpha_is_shape: false,
        blending: None,
    });

    list
}

/// A non-isolated group whose element blends with the page behind it (§11.4.4, §11.4.5).
///
/// ISO 32000-2 §11.4.5 defines the *other* backdrop, and it is the one every rasterising
/// library gives a layer:
///
/// > An isolated group is one whose elements shall be composited onto a fully transparent
/// > initial backdrop rather than onto the group's backdrop.
///
/// §11.4.4's own model is the group's backdrop, and it can only be seen where an element
/// blends: with every element painting Normal the backdrop is composited in and removed
/// again exactly (its NOTE 3), which is why this scene's element carries `Multiply`.
///
/// The green page under a `Multiply` blue leaves no blue at all; drawn on transparency the
/// blue survives whole. The group is painted at `alpha 0.5`, because a non-isolated group
/// whose result composites at 1.0 under Normal is NOTE 5's flattening and needs no group at
/// all — so this is the smallest scene the construction is *needed* for.
///
/// # What each backend does with it
///
/// `render-cpu` and `render-quorra` draw it and are held to each other on it
/// (`headless_quorra.rs`'s `cpu_and_quorra_agree_on_a_non_isolated_group`): quorra's
/// `GroupSpec` carries Table 145's `/I` since the four-hundred-and-thirty-eighth session, so a
/// group's buffer can begin as a copy of what is under it. `render-gpu` still refuses — a
/// Vello layer begins fully transparent and a scene cannot read what it has drawn so far — and
/// that refusal is tested against this scene, which is what keeps it from becoming silent.
#[must_use]
pub fn non_isolated_group() -> DisplayList {
    let mut list = DisplayList::new(A4);

    list.push(Command::Fill {
        path: Arc::new(rect(40.0, 100.0, 555.0, 750.0)),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Solid(GREEN),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });

    list.push(Command::Group {
        commands: vec![Command::Fill {
            path: Arc::new(rect(100.0, 200.0, 450.0, 600.0)),
            transform: Transform::IDENTITY,
            fill_rule: FillRule::NonZero,
            paint: Paint::Solid(BLUE),
            clip: None,
            mask: None,
            blend: BlendMode::Multiply,
        }],
        alpha: 0.5,
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
        isolated: false,
        knockout: false,
        alpha_is_shape: true,
        blending: None,
    });

    list
}

/// The sixteen corners of the process-ink cube this tree assumes for `DeviceCMYK`, as sRGB.
///
/// The same table `pdf_model::colour`'s `CMYK_CORNERS` holds and for the same reason; it is
/// copied here because `test-scenes` sits below `pdf-model` and this fixture needs a press
/// without interpreting a document to get one. The index's bits select which side of each
/// axis a corner sits on, `c` in the least significant and `k` in the most — which is the
/// order [`pdf_render::BlendingSpace::new`] documents for its grid.
#[rustfmt::skip]
const PROCESS_INKS: [[u8; 3]; 16] = [
    //  R    G    B      c m y k
    [255, 255, 255], // 0 0 0 0  paper
    [  0, 173, 239], // 1 0 0 0  process cyan
    [236,   0, 140], // 0 1 0 0  process magenta
    [ 46,  49, 146], // 1 1 0 0  blue
    [255, 242,   0], // 0 0 1 0  process yellow
    [  0, 166,  80], // 1 0 1 0  green
    [237,  28,  36], // 0 1 1 0  red
    [ 54,  54,  57], // 1 1 1 0  three-colour black
    [ 35,  31,  32], // 0 0 0 1  process black
    [  0,  15,  36], // 1 0 0 1
    [ 36,   0,   0], // 0 1 0 1
    [  0,   0,   2], // 1 1 0 1
    [ 28,  26,   0], // 0 0 1 1
    [  0,  19,   0], // 1 0 1 1
    [ 34,   0,   0], // 0 1 1 1
    [  0,   0,   0], // 1 1 1 1  registration
];

/// A page whose four marks are ink, composited in a four-component blending colour space
/// (ISO 32000-2 §11.3.4, §11.4.7).
///
/// > All page-level compositing shall be done in the default blending colour space of the
/// > page, and the entire result shall then, if the colour spaces are not equivalent, be
/// > converted to the native colour space of the output device before being composited with
/// > the context-dependent backdrop.
///
/// and §11.3.4 applies the compositing formula per component:
///
/// > The i th component of the result colour 𝐶𝑟 shall be obtained by applying the compositing
/// > formula to the i th components of the constituent colours
///
/// So a rasteriser with three channels composites
/// four by drawing the page **twice**: once carrying the additive complements of cyan,
/// magenta and yellow, once carrying the complement of black, with identical geometry,
/// shapes and opacities. [`pdf_render::blending`] puts the pair back together where the
/// clause puts the conversion, before the medium.
///
/// # What each mark is for
///
/// Six marks, in page coordinates, and each answers a different question:
///
/// | mark | inks | what it catches |
/// |---|---|---|
/// | paper, the whole sheet | none | a backend that leaves the page transparent, where nothing composites at all |
/// | half registration black, x 80–300, y 500–700 | all four at ½ | the clause itself: per component the pixel is ½ of each ink, and the cube's own average is **(76, 66, 64)**, not the (128, 128, 128) that averaging the two *converted* colours gives |
/// | process black alone, x 340–520, y 500–700 | k = 1 | a backend that draws the chromatic list and drops the other: this mark is **white** in the chromatic raster and (35, 31, 32) only if the second one was drawn |
/// | process cyan alone, x 80–300, y 180–380 | c = 1 | the chromatic half, which must survive the recombination unchanged at (0, 173, 239) |
/// | a `Hue` pair, x 340–520, y 180–380 | 0.2 0.6 0.9 0.4 under 0.7 0.1 0.3 0 | §11.3.5.3 over four components — see below |
///
/// The second row is the arithmetic the fixture exists for, and the third is what tells a
/// backend that renders one raster from a backend that renders two.
///
/// # The fifth row, and what it is the only witness for
///
/// §11.3.5.3's four modes are non-separable, so the black component gets a rule of its own:
///
/// > For the K component, the result shall be the K component of Cb for the Hue , Saturation ,
/// > and Color blend modes; it shall be the K component of Cs for the Luminosity blend mode.
///
/// Both marks are opaque, so §11.3.3 reduces to `Cr = B(Cb, Cs)` and the pixel *is* the blend
/// function. The clause's own arithmetic on the complements gives `0.977 0.277 0.511` in ink,
/// and the K is the backdrop's **0.4**, which the cube converts to **(12, 88, 90)**. Taking the
/// source's K instead — the rule one bullet down, for `Luminosity` — would be (19, 138, 141),
/// so this mark cannot pass by accident.
///
/// **Nothing is mapped for it in either list.** The black raster is neutral in all three
/// channels, and on a neutral pair the clause's own `Sat`, `SetSat`, `SetLum` and `Lum` return
/// the backdrop for `Hue`, `Saturation` and `Color` and the source for `Luminosity` — so a
/// backend that implements §11.3.5.3 gets the black component's rule with it. That identity is
/// what this mark holds each backend to, and `render-cpu`'s `blend` module derives it (ADR
/// 0277).
///
/// # What each backend does with it
///
/// `render-cpu` draws it, and `render-quorra` since the four-hundred-and-thirty-ninth
/// session: two `Target::Readback` renders against one device, which quorra's own
/// `two_rasters.rs` holds (`doc/QUORRA_FEEDBACK.md` section 17.1). `render-gpu` refuses the
/// list by name — a Vello scene renders one raster and the backend has no place to hold the
/// second — and that refusal is tested against this scene so it cannot become silent.
///
/// # Panics
///
/// Cannot panic: [`pdf_render::BlendingSpace::new`] returns `None` only for a grid that is
/// not `side⁴` samples with `side` at least two, and this one is 2⁴ = 16.
#[must_use]
#[expect(
    clippy::expect_used,
    reason = "sixteen samples is a grid of side two by construction; a Result here would \
              push an impossible error case onto every caller"
)]
pub fn four_component_page() -> DisplayList {
    /// One ink mark: its rectangle, its four ink components, the alpha it paints at and the
    /// mode it paints under.
    struct Mark {
        rect: (f32, f32, f32, f32),
        inks: [f32; 4],
        alpha: f32,
        blend: BlendMode,
    }

    let marks = [
        Mark {
            rect: (40.0, 100.0, 555.0, 750.0),
            inks: [0.0, 0.0, 0.0, 0.0],
            alpha: 1.0,
            blend: BlendMode::Normal,
        },
        Mark {
            rect: (80.0, 500.0, 300.0, 700.0),
            inks: [1.0, 1.0, 1.0, 1.0],
            alpha: 0.5,
            blend: BlendMode::Normal,
        },
        Mark {
            rect: (340.0, 500.0, 520.0, 700.0),
            inks: [0.0, 0.0, 0.0, 1.0],
            alpha: 1.0,
            blend: BlendMode::Normal,
        },
        Mark {
            rect: (80.0, 180.0, 300.0, 380.0),
            inks: [1.0, 0.0, 0.0, 0.0],
            alpha: 1.0,
            blend: BlendMode::Normal,
        },
        // §11.3.5.3's pair: a backdrop, then a `Hue` over it. Opaque, so the pixel is the
        // blend function itself and nothing about the alphas can absorb an error.
        Mark {
            rect: (340.0, 180.0, 520.0, 380.0),
            inks: [0.2, 0.6, 0.9, 0.4],
            alpha: 1.0,
            blend: BlendMode::Normal,
        },
        Mark {
            rect: (340.0, 180.0, 520.0, 380.0),
            inks: [0.7, 0.1, 0.3, 0.0],
            alpha: 1.0,
            blend: BlendMode::Hue,
        },
    ];

    // Both halves carry §11.3.4's *additive complements*, so the compositing formula sees
    // what that clause requires with nothing complemented around it: a component of 1 unit
    // of ink is a channel of 0. The chromatic list carries cyan, magenta and yellow; the
    // black list carries the fourth component in all three of its channels, of which
    // `pdf_render::blending::resolve` reads the first.
    let half = |chromatic: bool| {
        let mut list = DisplayList::new(A4);
        for mark in &marks {
            let (x0, y0, x1, y1) = mark.rect;
            let [cyan, magenta, yellow, black] = mark.inks;
            let channels = if chromatic {
                [1.0 - cyan, 1.0 - magenta, 1.0 - yellow]
            } else {
                [1.0 - black; 3]
            };
            list.push(Command::Fill {
                path: Arc::new(rect(x0, y0, x1, y1)),
                transform: Transform::IDENTITY,
                fill_rule: FillRule::NonZero,
                paint: Paint::Solid(Color {
                    r: channels[0],
                    g: channels[1],
                    b: channels[2],
                    a: mark.alpha,
                }),
                clip: None,
                mask: None,
                blend: mark.blend,
            });
        }
        list
    };

    let grid: Arc<[[f32; 3]]> = PROCESS_INKS
        .iter()
        .map(|corner| corner.map(|channel| f32::from(channel) / 255.0))
        .collect();
    let space = pdf_render::BlendingSpace::new(2, grid)
        .expect("sixteen samples is a grid of side two on four axes");

    let mut chromatic = half(true);
    chromatic.set_blending(space, half(false));
    chromatic
}

/// A non-isolated **knockout** group whose elements blend against the group's own backdrop
/// (ISO 32000-2 §11.4.6).
///
/// > In a knockout group, each individual element shall be composited with the group's
/// > initial backdrop rather than with the stack of preceding elements in the group.
///
/// and for a non-isolated group that backdrop is not transparency: "[a] nonisolated
/// knockout group composites its topmost enclosing element with the group's backdrop." So a
/// backend has to retain the backdrop beside the accumulation and composite each element
/// against *it* — the construction ADR 0307 priced and ADR 0327 built — and every element
/// arrives as a [`Command::Shaped`] so the weighted average's factor is stated per pixel.
///
/// # The arithmetic, in a 100-unit page
///
/// The page is opaque red, `B = (1, 0, 0)`. Element 1 is opaque blue under `Multiply`, so
/// its composite with `B` is `(0, 0, 0)` — black only because the blend saw the page.
/// Element 2 is green at opacity 0.3 under Normal: `0.7 × B + 0.3 × green =
/// (178.5, 76.5, 0)`, and within its shape it **replaces** element 1's black rather than
/// compositing over it, which is the knockout. Its left edge sits at x = 30.5 so one device
/// column carries shape ½, where the weighted average gives `(89, 38, 0)` — the pixel that
/// separates this construction from source-over (`(0, 38, 0)`) and from the
/// transparent-backdrop staged pair (element 1 blue rather than black).
///
/// # What each backend does with it
///
/// `render-cpu` draws it (`group_constructions.rs` holds the pixels); `render-gpu` and
/// `render-quorra` refuse it by name — neither can retain a backdrop beside a layer's
/// accumulation — and their refusals are tested against this scene so they cannot become
/// silent.
#[must_use]
pub fn knockout_group_on_its_own_backdrop() -> DisplayList {
    let square = Size {
        width: 100.0,
        height: 100.0,
    };
    let fill =
        |x0: f32, y0: f32, x1: f32, y1: f32, colour: Color, blend: BlendMode| Command::Fill {
            path: Arc::new(rect(x0, y0, x1, y1)),
            transform: Transform::IDENTITY,
            fill_rule: FillRule::NonZero,
            paint: Paint::Solid(colour),
            clip: None,
            mask: None,
            blend,
        };
    // The element's shape half is its geometry painted opaque white under Normal, which is
    // what `pdf-model`'s `stated_shape` derives for a solid fill.
    let shaped =
        |x0: f32, y0: f32, x1: f32, y1: f32, colour: Color, blend: BlendMode| Command::Shaped {
            object: Box::new(fill(x0, y0, x1, y1, colour, blend)),
            shape: Box::new(fill(x0, y0, x1, y1, Color::WHITE, BlendMode::Normal)),
        };
    let mut list = DisplayList::new(square);
    list.push(fill(0.0, 0.0, 100.0, 100.0, RED, BlendMode::Normal));
    list.push(Command::Group {
        commands: vec![
            shaped(10.0, 10.0, 60.0, 60.0, BLUE, BlendMode::Multiply),
            shaped(
                30.5,
                30.0,
                80.0,
                80.0,
                Color {
                    r: 0.0,
                    g: 1.0,
                    b: 0.0,
                    a: 0.3,
                },
                BlendMode::Normal,
            ),
        ],
        alpha: 1.0,
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
        isolated: false,
        knockout: true,
        alpha_is_shape: false,
        blending: None,
    });
    list
}

/// An isolated group compositing in a four-component blending colour space of its own
/// (ISO 32000-2 §11.6.6, §11.7.2).
///
/// §11.7.2: "all blending and compositing computations shall be done in that space", so the
/// group's elements are two lists — the additive complements of cyan, magenta and yellow,
/// and of black — and the pair resolves through the space's grid before the group is
/// painted onto its parent ([`pdf_render::GroupBlending`]). The page itself states no
/// space, which is the corpus's own shape (`bug1721218_reduced.pdf`).
///
/// # The arithmetic
///
/// The group holds paper and registration black at constant alpha ½ over it, so the covered
/// pixels carry half of each of the four inks; the cube at `(½, ½, ½, ½)` is the mean of
/// its sixteen corners, **(76, 66, 64)** — against the (128, 128, 128) that converting each
/// colour first and compositing on the device gives, ADR 0251's 51-of-255 gap one scope
/// down from the page. The inner mark's right edge sits at x = 80.5, so one device column
/// carries a quarter of each ink and holds the grid at `(¼, ¼, ¼, ¼)`.
///
/// # What each backend does with it
///
/// `render-cpu` draws it (`group_constructions.rs`); `render-gpu` and `render-quorra`
/// refuse it by name — the pair resolves per pixel after the group composites, which a
/// scene under composition cannot — and their refusals are tested against this scene.
///
/// # Panics
///
/// Cannot panic: [`pdf_render::BlendingSpace::new`] returns `None` only for a grid that is
/// not `side⁴` samples with `side` at least two, and this one is 2⁴ = 16.
#[must_use]
#[expect(
    clippy::expect_used,
    reason = "sixteen samples is a grid of side two by construction; a Result here would \
              push an impossible error case onto every caller"
)]
pub fn group_in_its_own_blending_space() -> DisplayList {
    let square = Size {
        width: 100.0,
        height: 100.0,
    };
    let fill = |x0: f32, y0: f32, x1: f32, y1: f32, colour: Color| Command::Fill {
        path: Arc::new(rect(x0, y0, x1, y1)),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Solid(colour),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    };
    // Paper is no ink and registration black is every ink, in all four components alike, so
    // the two halves of the pair are the same two marks: complement 1 everywhere for paper,
    // complement 0 at alpha ½ for the black.
    let elements = || {
        vec![
            fill(10.0, 10.0, 90.0, 90.0, Color::WHITE),
            fill(
                20.0,
                20.0,
                80.5,
                80.0,
                Color {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 0.5,
                },
            ),
        ]
    };
    let grid: Arc<[[f32; 3]]> = PROCESS_INKS
        .iter()
        .map(|corner| corner.map(|channel| f32::from(channel) / 255.0))
        .collect();
    let space = pdf_render::BlendingSpace::new(2, grid)
        .expect("sixteen samples is a grid of side two on four axes");
    let mut list = DisplayList::new(square);
    list.push(fill(0.0, 0.0, 100.0, 100.0, Color::WHITE));
    list.push(Command::Group {
        commands: elements(),
        alpha: 1.0,
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
        isolated: true,
        knockout: false,
        alpha_is_shape: false,
        blending: Some(Box::new(pdf_render::GroupBlending::FourComponents {
            space,
            black: elements(),
        })),
    });
    list
}

/// An isolated group compositing in a one-component blending colour space through a curve
/// (ISO 32000-2 §11.6.6, §11.7.2, §8.6.5.2).
///
/// The same geometry as [`group_in_its_own_blending_space`] — paper, and black at constant
/// alpha ½ over it — inside a group whose space is one component leaving by a curve
/// ([`pdf_render::GroupBlending::OneComponent`]): every colour in the list is its component
/// in all three channels, and the curve is sRGB's transfer function, which is what a
/// `/Gamma 1` `CalGray` group's component shows as on this device (its component is a `Y`).
///
/// # The arithmetic
///
/// The composite happens in the component, so half of black over paper is ½, and the curve
/// puts ½ at `1.055 × 0.5^(1/2.4) − 0.055 = 0.735`, **188** of 255 — against the 128 that
/// compositing in device grey gives. Column 80 is half covered, so it holds ¾ and the curve
/// at ¾.
///
/// # What each backend does with it
///
/// `render-cpu` draws it (`group_constructions.rs`); `render-gpu` and `render-quorra`
/// refuse it by name, as they refuse the four-component pair.
///
/// # Panics
///
/// Cannot panic: [`pdf_render::GreyCurve::new`] returns `None` only for fewer than two
/// samples, and this one has seventeen.
#[must_use]
#[expect(
    clippy::expect_used,
    reason = "seventeen samples is a curve by construction; a Result here would push an \
              impossible error case onto every caller"
)]
pub fn group_in_a_one_component_blending_space() -> DisplayList {
    let square = Size {
        width: 100.0,
        height: 100.0,
    };
    let fill = |x0: f32, y0: f32, x1: f32, y1: f32, colour: Color| Command::Fill {
        path: Arc::new(rect(x0, y0, x1, y1)),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Solid(colour),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    };
    let samples: Vec<[f32; 3]> = (0..=16u8)
        .map(|index| {
            let component = f32::from(index) / 16.0;
            let encoded = if component <= 0.003_130_8 {
                component * 12.92
            } else {
                1.055 * component.powf(1.0 / 2.4) - 0.055
            };
            [encoded; 3]
        })
        .collect();
    let curve =
        pdf_render::GreyCurve::new(Arc::from(samples)).expect("seventeen samples is a curve");
    let mut list = DisplayList::new(square);
    list.push(fill(0.0, 0.0, 100.0, 100.0, Color::WHITE));
    list.push(Command::Group {
        commands: vec![
            fill(10.0, 10.0, 90.0, 90.0, Color::WHITE),
            fill(
                20.0,
                20.0,
                80.5,
                80.0,
                Color {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 0.5,
                },
            ),
        ],
        alpha: 1.0,
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
        isolated: true,
        knockout: false,
        alpha_is_shape: false,
        blending: Some(Box::new(pdf_render::GroupBlending::OneComponent { curve })),
    });
    list
}

/// An isolated group compositing in a three-component CIE-based blending colour space
/// through a cube (ISO 32000-2 §11.6.6, §11.7.2, §8.6.5.3).
///
/// The same geometry as [`group_in_its_own_blending_space`] — paper, and black at constant
/// alpha ½ over it — inside a group whose space is three components leaving by a cube
/// ([`pdf_render::GroupBlending::ThreeComponents`]): every colour in the list is the
/// space's components, and the cube is a linear `CalRGB`'s — identity gamma, the sRGB
/// primaries' matrix, so its grid is the identity on linear light — with sRGB's transfer
/// function as the output curve, which is what such a group's components show as on this
/// device (its components are linear light).
///
/// # The arithmetic
///
/// The composite happens in the components, so half of black over paper is ½ on each, and
/// the output curve puts ½ at `1.055 × 0.5^(1/2.4) − 0.055 = 0.735`, **188** of 255 — against
/// the 128 that compositing on the device gives. Column 80 is half covered, so it holds ¾ and
/// the curve at ¾.
///
/// # What each backend does with it
///
/// `render-cpu` draws it (`group_constructions.rs`); `render-gpu` and `render-quorra`
/// refuse it by name, as they refuse the pair and the curve.
///
/// # Panics
///
/// Cannot panic: [`pdf_render::ColourCube::new`] returns `None` only for curves of fewer
/// than two samples or a grid that is not `side³`, and this one is two, two, eight and
/// seventeen.
#[must_use]
#[expect(
    clippy::expect_used,
    reason = "eight corners and two curves is a cube by construction; a Result here would \
              push an impossible error case onto every caller"
)]
pub fn group_in_a_three_component_blending_space() -> DisplayList {
    let square = Size {
        width: 100.0,
        height: 100.0,
    };
    let fill = |x0: f32, y0: f32, x1: f32, y1: f32, colour: Color| Command::Fill {
        path: Arc::new(rect(x0, y0, x1, y1)),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Solid(colour),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    };
    // Identity input curves, the identity grid on linear light, and sRGB's encoding out.
    let input: Vec<[f32; 3]> = vec![[0.0; 3], [1.0; 3]];
    let grid: Vec<[f32; 3]> = (0..8usize)
        .map(|corner| {
            let at = |bit: usize| if (corner >> bit) & 1 == 1 { 1.0 } else { 0.0 };
            [at(0), at(1), at(2)]
        })
        .collect();
    let output: Vec<f32> = (0..=16u8)
        .map(|index| {
            let linear = f32::from(index) / 16.0;
            if linear <= 0.003_130_8 {
                linear * 12.92
            } else {
                1.055 * linear.powf(1.0 / 2.4) - 0.055
            }
        })
        .collect();
    let cube = pdf_render::ColourCube::new(Arc::from(input), 2, Arc::from(grid), Arc::from(output))
        .expect("two curves and eight corners is a cube");
    let mut list = DisplayList::new(square);
    list.push(fill(0.0, 0.0, 100.0, 100.0, Color::WHITE));
    list.push(Command::Group {
        commands: vec![
            fill(10.0, 10.0, 90.0, 90.0, Color::WHITE),
            fill(
                20.0,
                20.0,
                80.5,
                80.0,
                Color {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 0.5,
                },
            ),
        ],
        alpha: 1.0,
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
        isolated: true,
        knockout: false,
        alpha_is_shape: false,
        blending: Some(Box::new(pdf_render::GroupBlending::ThreeComponents {
            cube,
        })),
    });
    list
}

/// All sixteen of §11.3.5's blend modes, each over the same backdrop.
///
/// # Why this scene exists, and what it can catch that nothing else could
///
/// The cross-backend scenes covered geometry, shadings, images, soft masks and a transparency
/// group, and **not one of them selected a blend mode**: every `Command` in every other scene
/// carries `BlendMode::Normal`. So the two backends' sixteen blend functions had never been
/// held to each other at all, which the thirty-seventh session found by reading §11.3.5 rather
/// than by any gate noticing.
///
/// The four in Table 135 are why that matters. Hue, Saturation, Color and Luminosity are
/// *non-separable*: each is defined by the clause's `Lum`, `ClipColor`, `SetLum` and `SetSat`
/// functions over all three components at once, so no per-component formula produces them and
/// a backend that got one subtly wrong would still produce a plausible picture. Trap 2's rule
/// in its sharpest form — a decision either backend can make alone is a decision neither has
/// made.
///
/// The backdrop is a horizontal ramp through red, green and blue and the source is a vertical
/// one, so every mode is exercised over a wide range of both operands rather than at one pair
/// of colours. Sixteen tiles, four by four, in the order [`ALL_BLEND_MODES`] lists them.
#[must_use]
pub fn blend_modes() -> DisplayList {
    /// Four by four tiles, of three bands each.
    const ACROSS: f32 = 4.0;
    /// A page whose tiles and bands both land on whole pixels at scale 1.
    ///
    /// Every other scene here is A4, and this one is not for a reason worth stating: a band
    /// edge that falls between two pixels is anti-aliased, and two rasterisers antialias
    /// differently. This scene is *about* what happens inside a region, so its edges are
    /// placed where no edge rule can enter the measurement — 480 is four tiles of 120, each
    /// of three bands of 40.
    const SIDE: f32 = 480.0;
    /// Each band is inset by this much, so that **no two rectangles share an edge**.
    ///
    /// Not tidiness: a shared edge is antialiased by both rasterisers and they antialias
    /// differently, so a seam between two colours is a difference of tens of levels that has
    /// nothing to do with the blend function. This scene is about what happens *inside* a
    /// region.
    const INSET: f32 = 2.0;

    let mut list = DisplayList::new(Size::new(SIDE, SIDE));
    let (width, height) = (SIDE / ACROSS, SIDE / ACROSS);

    for (index, blend) in ALL_BLEND_MODES.into_iter().enumerate() {
        // Laid out so that tile `index` is the `index`-th tile of the *raster*, reading
        // left to right and top to bottom: a display list's y runs up and a raster's runs
        // down, and a test that names which mode differs has to be able to find it.
        let column = f32::from(u8::try_from(index % 4).unwrap_or(0));
        let row = f32::from(u8::try_from(index / 4).unwrap_or(0));
        let left = column * width;
        let top = height.mul_add(-(row + 1.0), SIDE);

        // Three vertical bands under three horizontal ones, so each mode meets every pair
        // of primaries rather than one.
        let band = width / 3.0;

        for (index, colour) in [RED, GREEN, BLUE].into_iter().enumerate() {
            let start = band.mul_add(f32::from(u8::try_from(index).unwrap_or(0)), left);
            list.push(Command::Fill {
                path: Arc::new(rect(
                    start + INSET,
                    top + INSET,
                    start + band - INSET,
                    top + height - INSET,
                )),
                transform: Transform::IDENTITY,
                fill_rule: FillRule::NonZero,
                paint: Paint::Solid(colour),
                clip: None,
                mask: None,
                blend: BlendMode::Normal,
            });
        }

        for (index, colour) in [BLUE, GREEN, RED].into_iter().enumerate() {
            let start = (height / 3.0).mul_add(f32::from(u8::try_from(index).unwrap_or(0)), top);
            list.push(Command::Fill {
                path: Arc::new(rect(
                    left + INSET,
                    start + INSET,
                    left + width - INSET,
                    start + height / 3.0 - INSET,
                )),
                transform: Transform::IDENTITY,
                fill_rule: FillRule::NonZero,
                paint: Paint::Solid(colour),
                clip: None,
                mask: None,
                blend,
            });
        }
    }

    list
}

/// §11.3.5's sixteen modes, in Table 134's order and then Table 135's.
pub const ALL_BLEND_MODES: [BlendMode; 16] = [
    BlendMode::Normal,
    BlendMode::Multiply,
    BlendMode::Screen,
    BlendMode::Overlay,
    BlendMode::Darken,
    BlendMode::Lighten,
    BlendMode::ColorDodge,
    BlendMode::ColorBurn,
    BlendMode::HardLight,
    BlendMode::SoftLight,
    BlendMode::Difference,
    BlendMode::Exclusion,
    BlendMode::Hue,
    BlendMode::Saturation,
    BlendMode::Color,
    BlendMode::Luminosity,
];

/// A large object painted through a luminosity soft mask (ISO 32000-2 §11.5.3).
///
/// Built to fail in the axis a soft-mask defect moves, and at a magnitude the tolerances can
/// see — which for a mask means three things at once:
///
/// - **The mask varies across the page.** Its group paints a green square over the left half
///   of the masked area and leaves the right half to the `/BC` backdrop, so a backend that
///   applied a constant, or that dropped the mask entirely, differs over half the object
///   rather than at an edge.
/// - **The mask's colour is not grey.** Green is the colour that separates §11.5.3's
///   `0.59 G` from the `0.7152` of Rec. 709 and the `0.7154` of the SVG luminance both
///   rasterisers offer natively: a backend using a library's own luminance is 21% of the
///   mask's range away here and identical on any grey. That is the whole reason this scene
///   is coloured.
/// - **The backdrop is not black.** `/BC` is a mid grey, so the area outside the mask
///   group's marks takes §11.6.5.1's outside-the-bounding-box value rather than zero, and a
///   backend that leaves it transparent — the natural mistake — differs over the other half.
///
/// The masked object covers a third of the page, so a wrong mask is hundreds of thousands of
/// channels rather than the few hundred `MAX_DIFFERING_FRACTION` absorbs.
///
/// # Panics
///
/// Cannot panic in practice: the only fallible step is registering a soft mask, which fails
/// only past `u32::MAX` of them, and this scene registers one.
#[must_use]
#[expect(
    clippy::expect_used,
    reason = "a single add_soft_mask cannot exhaust a u32 index; a Result here would push \
              an impossible error case onto every caller"
)]
pub fn soft_mask() -> DisplayList {
    let mut list = DisplayList::new(A4);

    // What the mask is applied over, so that a mask value below one shows as this colour
    // coming through rather than as the page.
    list.push(Command::Fill {
        path: Arc::new(rect(40.0, 100.0, 555.0, 742.0)),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Solid(BLUE),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });

    let mask = list
        .add_soft_mask(SoftMask {
            commands: vec![Command::Fill {
                path: Arc::new(rect(40.0, 300.0, 300.0, 600.0)),
                transform: Transform::IDENTITY,
                fill_rule: FillRule::NonZero,
                paint: Paint::Solid(GREEN),
                clip: None,
                mask: None,
                blend: BlendMode::Normal,
            }],
            kind: SoftMaskKind::Luminosity {
                backdrop: Color {
                    r: 0.4,
                    g: 0.4,
                    b: 0.4,
                    a: 1.0,
                },
            },
            transfer: None,
            luminance: None,
        })
        .expect("the first soft mask of this list");

    list.push(Command::Fill {
        path: Arc::new(rect(40.0, 200.0, 555.0, 700.0)),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Solid(RED),
        clip: None,
        mask: Some(mask),
        blend: BlendMode::Normal,
    });

    list
}

/// A single diagonal stroke, which is where antialiasing differences between backends
/// actually appear.
///
/// Axis-aligned geometry agrees exactly between rasterisers; a diagonal edge is where
/// coverage is computed differently. This scene therefore sets the realistic floor for
/// cross-backend comparison tolerances.
#[must_use]
pub fn diagonal_stroke() -> DisplayList {
    let mut list = DisplayList::new(A4);

    let mut line = Path::new();
    line.push(PathCommand::MoveTo(Point::new(50.0, 100.0)));
    line.push(PathCommand::LineTo(Point::new(545.0, 742.0)));
    list.push(Command::Stroke {
        path: Arc::new(line),
        transform: Transform::IDENTITY,
        stroke: Stroke {
            width: 12.0,
            ..Stroke::default()
        },
        paint: Paint::Solid(BLUE),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });

    list
}

/// A curved shape: two cubic Béziers forming a closed lens.
///
/// Curves are flattened to line segments before rasterisation, and the two backends
/// choose their own flattening tolerance, so this is the scene most likely to expose a
/// genuine geometric disagreement rather than mere edge antialiasing.
#[must_use]
pub fn curves() -> DisplayList {
    let mut list = DisplayList::new(A4);

    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(100.0, 400.0)));
    path.push(PathCommand::CurveTo(
        Point::new(200.0, 700.0),
        Point::new(400.0, 700.0),
        Point::new(500.0, 400.0),
    ));
    path.push(PathCommand::CurveTo(
        Point::new(400.0, 100.0),
        Point::new(200.0, 100.0),
        Point::new(100.0, 400.0),
    ));
    path.push(PathCommand::Close);

    list.push(Command::Fill {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Solid(RED),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });

    list
}

/// ISO 32000-2 §8.7.4.5.4's cone: the radial geometry where the clause and every two-point
/// conical gradient part company.
///
/// `/Coords [150 100 25 70 100 60]` with `/Extend [true false]` — `radial_gradients.pdf`'s own
/// cell, translated onto a 200-unit page. The centres are 80 apart and the radii differ by 35,
/// so neither circle contains the other and a point can lie on **two** blend circles at once.
/// At the ending circle's centre the two are s = 0.478 and s = 2.333, and `/Extend[1]` is
/// false, so the greater is not a circle at all and the clause's "greatest value of s" is the
/// *lesser* root. Every gradient library paints nothing there.
///
/// So all three backends leave their native gradient here and draw
/// [`pdf_render::RadialRaster`]'s bytes — which is what makes this scene worth sharing: it is
/// the one radial where agreement between two backends is agreement about an *evaluation*
/// rather than about two libraries' shaders.
#[must_use]
pub fn radial_cone() -> DisplayList {
    let mut list = DisplayList::new(Size::new(200.0, 200.0));

    list.push(Command::Fill {
        path: Arc::new(rect(10.0, 10.0, 190.0, 190.0)),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Shading(Arc::new(pdf_render::Shading {
            background: None,
            kind: Arc::new(pdf_render::ShadingKind::Radial {
                start: Point::new(150.0, 100.0),
                start_radius: 25.0,
                end: Point::new(70.0, 100.0),
                end_radius: 60.0,
                ramp: pdf_render::Ramp::sample(|t| Color::rgb(1.0 - t, 0.0, t)),
                extend: (true, false),
            }),
            transform: Transform::IDENTITY,
        })),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });

    list
}

/// The closed form behind [`sampled_shading`]: the colour at domain point `(x, y)`.
///
/// Stated once, here, so that a test can evaluate the same arithmetic the scene draws and
/// take its expected pixel values from it rather than from any backend — twelve full waves
/// across each axis of the unit domain, red across and green down, over a constant blue.
///
/// Twelve is chosen against the grid this scene used to be resolved at: interpolating a
/// wave of twelve periods linearly between 128 samples is off by up to
/// `(1/127)²/8 × (24π)² ≈ 0.044` of full scale — eleven eight-bit levels — where sampling
/// it at one cell per device pixel of [`sampled_shading`]'s 400-pixel domain is off by under
/// a level. A defect that resolves the grid anywhere but at the device is that many levels
/// from the closed form, which is what makes the scene's exact-value tests able to fail.
#[must_use]
pub fn sampled_colour_at(x: f32, y: f32) -> Color {
    let wave = |v: f32| 0.5 + 0.5 * (v * 12.0 * std::f32::consts::TAU).sin();
    Color::rgba(wave(x), wave(y), 0.3, 1.0)
}

/// [`sampled_colour_at`] evaluated at the centres of whatever grid a device asks for.
///
/// A test implementor of `pdf_render`'s deferred-colours vocabulary: the display list names
/// the colours and the backend resolves them once it knows the scale, exactly as
/// `pdf-model`'s function-based shading does — cell `(i, j)` of an `n × m` grid carries the
/// closed form at `((2i + 1) / 2n, (2j + 1) / 2m)`, §10.7.4's centre rule.
///
/// It honours `Patch`'s block for the same reason: a scene that answered for the whole grid
/// however small a block was asked for would draw the right picture and prove nothing about
/// the clip the backends now ask for (ADR 0408). The domain is the unit square, so the block
/// and the fraction of the domain it covers are the same numbers.
#[derive(Debug)]
struct SampledWaves;

/// The cells of a `cells`-wide axis that the fraction `low..=high` of it needs.
///
/// `pdf-model`'s own producer states this once for the real thing; this is the same rule for
/// the scene, margin included — snapped outward to whole cells and then one cell further on
/// each side, which is what `pdf_render::Patch::within` asks for.
#[expect(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::arithmetic_side_effects,
    reason = "a fraction of 0..=1 times a device pixel count, rounded in i64 — which cannot \
              overflow from a u32 — and clamped into 0..=cells before it is narrowed"
)]
fn sampled_block(low: f32, high: f32, cells: u32) -> (u32, u32) {
    if cells == 0 {
        return (0, 0);
    }
    let at = |value: f32, round: fn(f32) -> f32| round(value * cells as f32) as i64;
    let first = (at(low, f32::floor) - 1).clamp(0, i64::from(cells) - 1) as u32;
    let last = (at(high, f32::ceil) + 1).clamp(0, i64::from(cells)) as u32;
    (first, last.max(first.saturating_add(1)))
}

impl pdf_render::ColoursAtDeviceScale for SampledWaves {
    fn colours(&self, patch: pdf_render::Patch) -> pdf_render::ColourGrid {
        #[expect(
            clippy::cast_precision_loss,
            reason = "grid axes are device pixel counts, far inside f32's exact range"
        )]
        let centre = |index: u32, cells: u32| (2.0 * index as f32 + 1.0) / (2.0 * cells as f32);
        #[expect(
            clippy::cast_precision_loss,
            reason = "cell indices are device pixel counts, far inside f32's exact range"
        )]
        let fraction = |index: u32, cells: u32| index as f32 / cells as f32;

        let [u0, u1, v0, v1] = patch.within;
        let (left, right) = sampled_block(u0, u1, patch.grid.width);
        let (top, bottom) = sampled_block(v0, v1, patch.grid.height);
        let mut pixels = Vec::new();
        for row in top..bottom {
            for column in left..right {
                pixels.push(sampled_colour_at(
                    centre(column, patch.grid.width),
                    centre(row, patch.grid.height),
                ));
            }
        }
        pdf_render::ColourGrid {
            width: right.saturating_sub(left),
            height: bottom.saturating_sub(top),
            pixels: pixels.into(),
            covers: [
                fraction(left, patch.grid.width),
                fraction(right, patch.grid.width),
                fraction(top, patch.grid.height),
                fraction(bottom, patch.grid.height),
            ],
        }
    }

    fn is_opaque(&self) -> bool {
        true
    }
}

/// How large [`sampled_shading`]'s page is, in PDF units.
pub const SAMPLED_PAGE: f32 = 420.0;

/// A sampled (type 1, function-based) shading resolved at the device's own grid.
///
/// The unit domain is drawn across 400 of the page's 420 units, which is the magnitude
/// where a grid fixed when the display list was built stops being the function: at 128
/// cells the waves of [`sampled_colour_at`] interpolate eleven levels wrong, and at one
/// cell per device pixel they are exact to a level. The filled shape ends in a diagonal
/// edge, so the comparison also covers the pixels where the two constructions the backends
/// use — a padded pattern on the CPU, an image clipped to the path on quorra — meet
/// fractional coverage. The path stays inside the domain's own extent because what happens
/// *beyond* the domain is a stated divergence between those constructions, and this scene
/// is about the resolution.
#[must_use]
pub fn sampled_shading() -> DisplayList {
    let mut list = DisplayList::new(Size::new(SAMPLED_PAGE, SAMPLED_PAGE));

    // (10, 10) to (410, 410) with the top-right corner cut off diagonally.
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(10.0, 10.0)));
    path.push(PathCommand::LineTo(Point::new(410.0, 10.0)));
    path.push(PathCommand::LineTo(Point::new(410.0, 300.0)));
    path.push(PathCommand::LineTo(Point::new(210.0, 410.0)));
    path.push(PathCommand::LineTo(Point::new(10.0, 410.0)));
    path.push(PathCommand::Close);

    list.push(Command::Fill {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Shading(Arc::new(pdf_render::Shading {
            background: None,
            kind: Arc::new(pdf_render::ShadingKind::Sampled {
                domain: [0.0, 1.0, 0.0, 1.0],
                source: pdf_render::DeferredColours::new(Arc::new(SampledWaves)),
                // No program, deliberately: these waves are a closed form rather than a
                // §7.10.5 list, and this scene's job is the *grid* — the construction every
                // backend still uses and the one the oracle draws. The device-evaluated
                // program has its own witness, a real document, in
                // `pdf-model/tests/shadings.rs`.
                program: None,
            }),
            // The unit domain onto (10, 10)–(410, 410) of the page.
            transform: Transform::new(400.0, 0.0, 0.0, 400.0, 10.0, 10.0),
        })),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });

    list
}

/// The colour [`shading_background`] washes every one of its four quadrants with.
pub const WASH: Color = Color {
    r: 0.0,
    g: 0.6,
    b: 0.6,
    a: 1.0,
};

/// ISO 32000-2 §8.7.4.3 Table 77's `/Background`, on all four shading kinds at once.
///
/// > If present, this colour shall be used, before any painting operation involving the
/// > shading, to fill those portions of the area to be painted that lie outside the bounds of
/// > the shading object.
///
/// Every quadrant fills a square larger than the shading inside it, so every quadrant has a
/// wash and a shading in it — and the four kinds are here together because *where a shading
/// ends* is four different questions and a scene of one would have answered one of them
/// (ADR 0046, and trap 12b's "a suite of small scenes tests small scenes"):
///
/// - **axial**, bounded by `/Extend` at both ends: the band between two perpendiculars;
/// - **radial**, two nested circles with neither end extended: the outer disc;
/// - **mesh**, bounded by the triangles the file states and nothing else;
/// - **sampled**, bounded by §8.7.4.5.2's transformed domain rectangle.
///
/// It is a cross-backend scene rather than a `render-cpu` fixture because §11.6.7 makes the
/// wash and the shading **one** painting operation — "the pattern's imp licit transparency
/// group shall be filled with the specified background colour before the sh operator is
/// invoked" — so all three backends draw one [`pdf_render::ShadingRaster`], and the thing worth
/// gating is that none of them decided the geometry or the colour for itself (trap 2). The
/// quadrant boundaries are at whole units and the shadings' own boundaries are not, so the
/// comparison covers both a hard edge and the fractional coverage where a construction can part.
#[must_use]
pub fn shading_background() -> DisplayList {
    let mut list = DisplayList::new(Size::new(200.0, 200.0));
    let ramp = || pdf_render::Ramp::sample(|t| Color::rgb(1.0 - t, 0.0, t));

    // Top left: an axial band 32.5 units wide inside an 85-unit square, extended at neither
    // end, so the wash is everything to the left of x = 33.5 and right of x = 66.
    list.push(Command::Fill {
        path: Arc::new(rect(10.0, 10.0, 95.0, 95.0)),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Shading(Arc::new(pdf_render::Shading {
            background: Some(WASH),
            kind: Arc::new(pdf_render::ShadingKind::Axial {
                start: Point::new(33.5, 10.0),
                end: Point::new(66.0, 10.0),
                ramp: ramp(),
                extend: (false, false),
            }),
            transform: Transform::IDENTITY,
        })),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });

    // Top right: two nested circles, neither end extended, so the wash is the square minus
    // the outer disc. Nested rather than a cone on purpose — this is the geometry a gradient
    // library *can* express, and it is therefore the quadrant that would catch a backend
    // keeping its own gradient and losing the wash.
    list.push(Command::Fill {
        path: Arc::new(rect(105.0, 10.0, 190.0, 95.0)),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Shading(Arc::new(pdf_render::Shading {
            background: Some(WASH),
            kind: Arc::new(pdf_render::ShadingKind::Radial {
                start: Point::new(147.5, 52.5),
                start_radius: 0.0,
                end: Point::new(147.5, 52.5),
                end_radius: 30.5,
                ramp: ramp(),
                extend: (false, false),
            }),
            transform: Transform::IDENTITY,
        })),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });

    // Bottom left: two triangles covering the left half of the square, so the wash is its
    // right half plus the sliver above and below the mesh's own extent.
    let corner = |x: f32, y: f32| Point::new(x, y);
    let red = Color::rgb(1.0, 0.0, 0.0);
    let blue = Color::rgb(0.0, 0.0, 1.0);
    list.push(Command::Fill {
        path: Arc::new(rect(10.0, 105.0, 95.0, 190.0)),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Shading(Arc::new(pdf_render::Shading {
            background: Some(WASH),
            kind: Arc::new(pdf_render::ShadingKind::Mesh {
                triangles: Arc::from(vec![
                    pdf_render::Triangle {
                        points: [
                            corner(20.5, 115.5),
                            corner(52.5, 115.5),
                            corner(52.5, 179.5),
                        ],
                        corners: pdf_render::Corners::Colours([red, blue, red]),
                    },
                    pdf_render::Triangle {
                        points: [
                            corner(20.5, 115.5),
                            corner(52.5, 179.5),
                            corner(20.5, 179.5),
                        ],
                        corners: pdf_render::Corners::Colours([red, red, blue]),
                    },
                ]),
                ramp: None,
            }),
            transform: Transform::IDENTITY,
        })),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });

    // Bottom right: the unit domain carried onto a 55-unit square inside an 85-unit one, so
    // the wash is the border around it. §8.7.4.5.2 is the clause that states this quadrant
    // outright — points inside the box and outside the transformed domain "shall be painted
    // with the shading's background colour".
    list.push(Command::Fill {
        path: Arc::new(rect(105.0, 105.0, 190.0, 190.0)),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Shading(Arc::new(pdf_render::Shading {
            background: Some(WASH),
            kind: Arc::new(pdf_render::ShadingKind::Sampled {
                domain: [0.0, 1.0, 0.0, 1.0],
                source: pdf_render::DeferredColours::new(Arc::new(SampledWaves)),
                program: None,
            }),
            transform: Transform::new(55.0, 0.0, 0.0, 55.0, 120.5, 120.5),
        })),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });

    list
}

/// A page filled edge to edge, on a page whose pixel width is not a multiple of the
/// GPU row alignment.
///
/// `copy_texture_to_buffer` pads each row to a 256-byte boundary. A backend that fails
/// to strip that padding produces a progressively sheared image rather than an
/// obviously broken one, so a uniform fill at an unaligned width makes the bug
/// unmissable: any stray pixel is not the fill colour.
///
/// 101 units wide gives 404 bytes per row, which pads to 512.
#[must_use]
pub fn unaligned_full_bleed() -> DisplayList {
    let size = Size {
        width: 101.0,
        height: 37.0,
    };
    let mut list = DisplayList::new(size);

    list.push(Command::Fill {
        path: Arc::new(rect(0.0, 0.0, size.width, size.height)),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Solid(RED),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });

    list
}

/// The [`basic`] scene expressed as a minimal PDF file.
///
/// # The invariant this crate exists to hold
///
/// These bytes and [`basic`] must describe *the same page*. That pairing is what makes
/// the reference-comparison harness meaningful: it renders this PDF with `pdftoppm`,
/// `mutool` and `gs`, renders [`basic`] with our own backends, and compares. If the two
/// descriptions drifted apart, every comparison would report a difference that is
/// nobody's bug.
///
/// Keeping both in one file is the whole point — a reviewer can check them against each
/// other without leaving the module.
///
/// The content stream mirrors [`basic`] operator for operator:
///
/// ```text
/// 1 0 0 rg  100 100 200 200 re f          red square (100,100)-(300,300)
/// q  400 400 50 50 re  W n                clip to the lower-left quarter
///    0 1 0 rg  400 400 100 100 re f       green square, clipped
/// Q
/// 0 0 1 RG  10 w  50 600 m 545 600 l S    blue stroke along y=600
/// ```
///
/// Once `pdf-syntax` exists, this same file becomes a parser fixture, and the display
/// list will be produced *from* it rather than written alongside it.
#[must_use]
pub fn basic_pdf() -> Vec<u8> {
    // Mirrors `basic()`. Any edit here needs the matching edit there.
    let content = b"1 0 0 rg\n\
                    100 100 200 200 re\n\
                    f\n\
                    q\n\
                    400 400 50 50 re\n\
                    W n\n\
                    0 1 0 rg\n\
                    400 400 100 100 re\n\
                    f\n\
                    Q\n\
                    0 0 1 RG\n\
                    10 w\n\
                    50 600 m\n\
                    545 600 l\n\
                    S\n"
    .to_vec();

    let objects: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] \
             /Contents 4 0 R /Resources << >> >>",
            A4.width, A4.height
        )
        .into_bytes(),
        [
            format!("<< /Length {} >>\nstream\n", content.len()).into_bytes(),
            content,
            b"endstream".to_vec(),
        ]
        .concat(),
    ];

    assemble_pdf(&objects)
}

/// Writes numbered objects into a whole file: header, bodies, classic table, trailer.
///
/// Object `n` is `objects[n - 1]`, and the catalogue is object 1 — the two conventions every
/// builder here shares, so cross-references between fixture objects can be written as
/// literals.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "object indices and the cross-reference size are bounded by the fixture's own \
              handful of objects, so no operation here can overflow"
)]
fn assemble_pdf(objects: &[Vec<u8>]) -> Vec<u8> {
    // The header's binary comment marks the file as containing 8-bit data, which is what
    // tells tools not to treat it as text. Required by the specification for any file
    // with binary content, and harmless here.
    let mut out: Vec<u8> = b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n".to_vec();

    // Byte offsets are recorded as objects are written, because the cross-reference table
    // must point at each object's first byte. Computing them afterwards from a finished
    // buffer is the classic way to get an off-by-one that only some readers tolerate.
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }

    let xref_offset = out.len();
    let size = objects.len() + 1;
    out.extend_from_slice(format!("xref\n0 {size}\n").as_bytes());
    // Entry zero is the head of the free list, and its format is fixed by the spec.
    out.extend_from_slice(b"0000000000 65535 f \n");
    for offset in &offsets {
        // Exactly 20 bytes per entry, trailing space included: readers may index into
        // this table arithmetically rather than parsing it.
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!("trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n")
            .as_bytes(),
    );

    out
}
