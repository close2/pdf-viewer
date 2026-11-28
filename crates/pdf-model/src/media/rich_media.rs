//! A rich media annotation's settings, presentation and configuration, as far as this program's
//! sound and video players present them: ISO 32000-2 §13.7.2.2's activation and deactivation
//! conditions (Tables 334 to 336), its presentation (Tables 338 to 340) and §13.7.2.3.3's
//! configuration and instances (Tables 341 to 343), its view (Tables 344 and 345) and its animation
//! (Table 337), read for `doc/todo/80` section 1 (ADRs 3854, 3855, 3865 to 3867, 3881, 3882).
//!
//! **What is read.** Table 335's `/Condition` — `XA`, `PO`, `PV` — says when the annotation is
//! activated, and Table 336's — `XD`, `PC`, `PI` — when it is deactivated; [`settings`] answers both.
//! Table 335's `/Configuration` names the configuration activation loads, "Default value: The
//! first element within the Configurations array"; [`activated`] reads it, the instances whose
//! `/Subtype` is the configuration's, and the first of them whose `/Asset` is a file of a type this
//! program's own player takes, and answers that clip, presented as Table 338's [`presentation`]
//! says: `Embedded` in the annotation's rectangle — "A interactive PDF processor shall support the
//! state Embedded" — or `Windowed` in a floating window of the host's held within the document
//! window, where the host can place one.
//!
//! **What is refused by name.** The `3D` configuration — rendering U3D and PRC artwork is excluded
//! (`CLAUDE.md`'s clause 13 entry) — and every configuration the players do not take is
//! [`activated`]'s `Err`, a sentence a person is told. Table 335's `/Scripts`, which "shall be
//! executed … by the rich media run time ECMAScript interpreter", is that run time's, ISO
//! 21757-1's, which the JavaScript exclusion keeps out; a configuration that plays says so in
//! [`Clip::unhonoured`]. Table 335's `/View` and `/Animation` are read in `views`: the view's
//! 3D scene, its snapshot and its state data said as the players read them ([`invoked_view`], ADR
//! 3881), and Table 337's animation, which drives keyframe animations a clip does not carry, set
//! aside by name ([`animation`], ADR 3882).
//!
//! **The asset's type is Table 44's `/Subtype`, else its file name's extension.** §13.7 names no
//! format for either configuration (RFC 0015 section 10.1), and Table 343's "the subtype shall
//! match the asset file type" is the producer's requirement; the matching the players need is the
//! embedded file's own stated type, the reading a movie's file has, and where it states none, the
//! file name §13.7.2.3.2 gives every asset (ADR 3867). An asset whose type is neither is not
//! played, and that is said.

use pdf_syntax::{Dictionary, Document, Object};

mod views;

pub use views::{Animation, AnimationStyle, View, animation};

use super::container;
use super::deprecated::{content_type, file};
use super::{
    Background, Clip, Data, Floating, MonitorSpecifier, Offscreen, Players, Playing, Position,
    Processor, Relative, Resize, Window, WindowKind,
};

/// Table 335's `/Condition`: when the annotation is activated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Activation {
    /// `XA`, the default: "the annotation is explicitly activated by a user action or script".
    Explicit,
    /// `PO`: "the annotation is activated as soon as the page that contains the annotation receives
    /// focus as the current page".
    PageOpen,
    /// `PV`: "the annotation is activated as soon as any part of the page that contains the
    /// annotation becomes visible".
    PageVisible,
}

/// Table 336's `/Condition`: when the annotation is deactivated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Deactivation {
    /// `XD`, the default: "the annotation is explicitly deactivated by a user action or script".
    Explicit,
    /// `PC`: "the annotation is deactivated as soon as the page that contains the annotation loses
    /// focus as the current page".
    PageClose,
    /// `PI`: "the annotation is deactivated as soon as the entire page that contains the
    /// annotation is no longer visible".
    PageInvisible,
}

/// Table 334's settings as far as they decide when a rich media annotation plays.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// Table 335's `/Condition`.
    pub activation: Activation,
    /// Table 336's `/Condition`.
    pub deactivation: Deactivation,
    /// One sentence for each condition that names no value the tables define, which is read as
    /// the default.
    pub notes: Vec<String>,
}

/// Table 333's `/RichMediaSettings` of a `RichMedia` annotation, and its two conditions; `None` for
/// an annotation of any other subtype.
///
/// "If no `RichMediaSettings` dictionary is present, the first configuration is loaded" — on the
/// default condition, `XA`, and deactivated on `XD`. A condition that is not one of the three
/// names its table lists is said, and the default stands in its place.
#[must_use]
pub fn settings(document: &Document, annotation: &Dictionary) -> Option<Settings> {
    if !is_rich_media(document, annotation) {
        return None;
    }
    let mut out = Settings {
        activation: Activation::Explicit,
        deactivation: Deactivation::Explicit,
        notes: Vec::new(),
    };
    let settings = document.get_key(annotation, "RichMediaSettings");
    let Some(settings) = settings.as_dict() else {
        return Some(out);
    };
    let condition = |key: &str| {
        let dictionary = document.get_key(settings, key);
        let condition = dictionary
            .as_dict()
            .map_or(Object::Null, |dict| document.get_key(dict, "Condition"));
        match condition {
            Object::Null => None,
            Object::Name(name) => Some(Ok(name.as_bytes().to_vec())),
            _ => Some(Err(())),
        }
    };
    match condition("Activation") {
        None => {}
        Some(Ok(name)) if name == b"XA" => {}
        Some(Ok(name)) if name == b"PO" => out.activation = Activation::PageOpen,
        Some(Ok(name)) if name == b"PV" => out.activation = Activation::PageVisible,
        Some(_) => out.notes.push(
            "Table 335's /Condition is none of XA, PO and PV, so the annotation is activated by \
             the reader, the table's default XA"
                .to_owned(),
        ),
    }
    match condition("Deactivation") {
        None => {}
        Some(Ok(name)) if name == b"XD" => {}
        Some(Ok(name)) if name == b"PC" => out.deactivation = Deactivation::PageClose,
        Some(Ok(name)) if name == b"PI" => out.deactivation = Deactivation::PageInvisible,
        Some(_) => out.notes.push(
            "Table 336's /Condition is none of XD, PC and PI, so the annotation is deactivated by \
             the reader, the table's default XD"
                .to_owned(),
        ),
    }
    Some(out)
}

/// What activating a `RichMedia` annotation loads: the clip, and the sentences a person is told
/// beside it.
#[derive(Debug, Clone)]
pub struct Activated {
    /// The configuration's instance's asset, as this program's player is handed it, presented as
    /// Table 338 states ([`presentation`]).
    pub clip: Clip,
    /// What was read differently from what the file states, or not presented as it asks, each
    /// named: a presentation shown `Embedded` in place of `Windowed`, a flag this program's player
    /// has nothing to apply to, instances of other kinds not loaded, and instances passed over
    /// for the one played.
    pub notes: Vec<String>,
}

/// What activating a `RichMedia` annotation loads, for this program's players: the clip of the
/// configuration's instance, judged for `processor`'s players and presented as Table 338 states.
///
/// The configuration is Table 335's `/Configuration`, or "the first element within the
/// Configurations array" of Table 341. Its kind is Table 342's `/Subtype`, or where it states none,
/// the first instance's — the table's "the run time shall determines the scene type by referring
/// to the type of asset file specified by the first element in the Instances array", read through
/// Table 343's "the subtype shall match the asset file type". The instance played is the first of
/// that kind whose asset the players take, the asset's type its Table 44 `/Subtype` or else its
/// file name's (ADRs 3855, 3867): a `Sound` instance's asset is a RIFF WAVE file or an MP4 file of
/// sound alone and the host plays sound; a `Video` instance's an MP4 file of `video/` type and the
/// host shows video.
///
/// # Errors
///
/// The sentence a person is told, ending where the configuration stops: an annotation of another
/// subtype or with no content, no configuration, a `3D` configuration (excluded), a kind Table 342
/// does not define, no instance of the kind, and — where no instance of the kind plays — the
/// first instance's reason: an asset that is not an embedded file, of a type the players do not
/// take, of no type this reader can tell, or of a type that is not the instance's.
pub fn activated(
    document: &Document,
    annotation: &Dictionary,
    processor: Processor<'_>,
) -> Result<Activated, String> {
    let (activation, configuration) = configuration(document, annotation)?;
    let (video, instances, others) = instances(document, &configuration)?;
    let kind_said = if video { "Video" } else { "Sound" };
    let mut notes = Vec::new();
    if others > 0 {
        notes.push(format!(
            "its configuration's {others} instance(s) of a kind other than {kind_said} are not \
             loaded: this program's players present one instance, of the configuration's kind \
             (ADR 3867)"
        ));
    }
    let mut refusals = Vec::new();
    let mut chosen = None;
    for (index, instance) in instances.iter().enumerate() {
        match asset(document, instance, video, processor) {
            Ok(found) => {
                chosen = Some(found);
                break;
            }
            Err(why) => refusals.push(format!("instance {} — {why}", index.saturating_add(1))),
        }
    }
    let Some((data, stated, guessed)) = chosen else {
        // not a repair: `instances` answers at least one instance, so a refusal is there; the
        //   empty sentence is unreachable.
        return Err(refusals.into_iter().next().unwrap_or_default());
    };
    notes.extend(refusals.into_iter().map(|refusal| {
        format!("its {kind_said} {refusal}, so the next instance of the kind is played")
    }));
    if let Some(guessed) = guessed {
        notes.push(guessed);
    }
    let mut unhonoured = Vec::new();
    if let Some(activation) = &activation {
        if !matches!(document.get_key(activation, "Scripts"), Object::Null) {
            unhonoured.push(
                "Table 335's /Scripts, because a rich media run time's ECMAScript (ISO 21757-1) \
                 is excluded from this program"
                    .to_owned(),
            );
        }
        // Table 337 drives keyframe animations, which a sound or video clip does not carry
        // (ADR 3882).
        if let Some(animation) = animation(document, activation) {
            notes.extend(animation.notes.iter().cloned());
            unhonoured.extend(animation.set_aside());
        }
    }
    // The view activation invokes: Table 315's scene, Table 344's snapshot and its state data,
    // each said as this program's players read it (ADR 3881).
    if let Some(view) = invoked_view(document, annotation) {
        notes.extend(view.notes);
    }
    let shown = presentation(document, activation.as_ref());
    notes.extend(shown.notes.iter().cloned());
    let mut clip = Clip {
        content_type: Some(stated),
        data,
        temporary_file: None,
        unhonoured,
        playing: Playing::default(),
        played_here: true,
        background: None,
        window: None,
    };
    if !video {
        // A sound shows nothing in a rectangle or a window, so the annotation keeps its
        // appearance whichever style is stated, and a toolbar would have no pictures to stand by.
        if shown.toolbar {
            notes.push(
                "Table 338's /Toolbar true is set aside for a Sound configuration, whose player \
                 shows nothing to stand a toolbar beside"
                    .to_owned(),
            );
        }
        return Ok(Activated { clip, notes });
    }
    present(document, &mut clip, &shown, processor, &mut notes);
    Ok(Activated { clip, notes })
}

/// A `Video` configuration's clip presented as Table 338's `shown` states, for `processor`'s
/// windows (ADRs 3865, 3866), with what is presented otherwise said in `notes`.
fn present(
    document: &Document,
    clip: &mut Clip,
    shown: &Presentation,
    processor: Processor<'_>,
    notes: &mut Vec<String>,
) {
    // Table 338's /Toolbar, "a toolbar should be displayed when the annotation is activated",
    // is the player's controller bar, drawn over the pictures (ADR 3866).
    clip.playing.controls = shown.toolbar;
    match shown.style {
        Style::Windowed(window) if processor.screens.windows && processor.screens.placed => {
            let right_to_left = crate::structure::document_language(document)
                .is_some_and(|tag| reads_right_to_left(&tag));
            clip.window = Some(Window {
                kind: WindowKind::Floating(window.floating(right_to_left)),
                monitor: MonitorSpecifier::Largest,
                colour: Background::DEFAULT_COLOUR,
            });
            if shown.transparent {
                notes.push(
                    "Table 338's /Transparent true has no page to show through a window of its \
                     own, so the window is drawn opaque"
                        .to_owned(),
                );
            }
        }
        Style::Windowed(_) => {
            notes.push(if processor.screens.windows {
                "Table 338's /Style Windowed is presented Embedded, in the annotation's rectangle: \
                 this window's toolkit places no window of its own, so it cannot hold one within \
                 the presentation area as §13.7.2.2.6 requires (ADR 3865)"
                    .to_owned()
            } else {
                "Table 338's /Style Windowed is presented Embedded, in the annotation's rectangle: \
                 this window opens no window of its own for a clip's pictures (ADR 3865)"
                    .to_owned()
            });
            clip.background = Some(embedded(shown.transparent));
        }
        Style::Embedded => clip.background = Some(embedded(shown.transparent)),
    }
}

/// The annotation's rectangle a `Video` configuration is played in, Table 338's `Embedded`: drawn
/// "over an opaque background prior to composition over the page content" where `/Transparent` is
/// false, and "composited over the page content using an alpha channel" where it is true. The
/// pictures this program decodes — H.264's — carry no alpha, so what shows the page through is the
/// rectangle's part no picture covers, the background painted at no opacity (ADR 3866).
fn embedded(transparent: bool) -> Background {
    Background {
        opacity: if transparent { 0.0 } else { 1.0 },
        ..Background::default()
    }
}

/// Table 338's `/Style`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Style {
    /// `Embedded`, the default: "A interactive PDF processor shall support the state Embedded".
    Embedded,
    /// `Windowed`, in "the floating window that an interactive PDF processor may present"
    /// (§13.7.2.2.6), as Tables 339 and 340 describe it.
    Windowed(RichMediaWindow),
}

/// Table 338's `RichMediaPresentation` dictionary, read for a `Sound` or `Video` configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct Presentation {
    /// `/Style`.
    pub style: Style,
    /// `/Transparent`: "whether the page content shall be displayed through the transparent areas
    /// of the rich media content"; `false`, the default.
    pub transparent: bool,
    /// `/Toolbar`: "If true, a toolbar should be displayed when the annotation is activated and
    /// given focus"; "false otherwise" than for `3D` content, the default here.
    pub toolbar: bool,
    /// `/PassContextClick`: "whether a contextual click on the rich media annotation is passed to
    /// the media player run time or shall be handled natively"; `false`, the default.
    pub pass_context_click: bool,
    /// What is read in place of what the dictionary states, or set aside, each named.
    pub notes: Vec<String>,
}

/// Table 339's `/Width` or `/Height`: "A dictionary with keys Default, Max, and Min", in default
/// user space units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Extent {
    /// `/Default`, held between [`Self::min`] and [`Self::max`].
    pub default: f32,
    /// `/Max`.
    pub max: f32,
    /// `/Min`.
    pub min: f32,
}

/// Table 340's `/HAlign` or `/VAlign`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    /// `Near`: the left edge — the right in a right-to-left reading language — or the top.
    Near,
    /// `Center`.
    Center,
    /// `Far`: the right edge — the left in a right-to-left reading language — or the bottom.
    Far,
}

/// §13.7.2.2.6's `RichMediaWindow`, with Table 340's position, in default user space units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RichMediaWindow {
    /// Table 339's `/Width`; "Default shall be 288, Max shall be 576, and Min shall be 72".
    pub width: Extent,
    /// Table 339's `/Height`; "Default shall be 216, Max shall be 432, and Min shall be 72".
    pub height: Extent,
    /// Table 340's `/HAlign`, `Far` by default.
    pub h_align: Align,
    /// Table 340's `/VAlign`, `Near` by default.
    pub v_align: Align,
    /// Table 340's `/HOffset`, 18 by default.
    pub h_offset: f32,
    /// Table 340's `/VOffset`, 18 by default.
    pub v_offset: f32,
}

impl RichMediaWindow {
    /// The floating window this program opens for it: `/Default`'s size, at the alignment and
    /// offsets Table 340 states, relative to the document window and held within it.
    ///
    /// **One default user space unit is one pixel** of the window, the scale at which this
    /// program shows a page at magnification 1, because "do not rotate or scale the window to
    /// match the orientation and magnification of the page" — §12.5.3's `NoZoom` — leaves the
    /// size the page has unmagnified (ADR 3865). A choice, not a reading: the clause fixes the
    /// unit and not the pixels a unit takes. The window keeps its `/Default` size: a person is not
    /// offered resizing, so `/Min` and `/Max` bound nothing they could do.
    ///
    /// Table 340 reverses `HAlign`'s descriptions for right-to-left reading languages:
    /// `right_to_left`, from the catalog's `/Lang`, turns `Near` to the right edge.
    #[must_use]
    pub fn floating(&self, right_to_left: bool) -> Floating {
        let pixels = |units: f32| {
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "held between 1 and 65 536 units before the cast, so it is a whole \
                          positive pixel count a u32 holds"
            )]
            // not a repair: a window this program opens is at least one pixel and at most its own
            //   bound; Table 339's sides are positive numbers read before this.
            let pixels = units.round().clamp(1.0, 65_536.0) as u32;
            pixels
        };
        let offset = |units: f32| {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "held within 65 536 units either way before the cast"
            )]
            // not a repair: an offset past any desktop is this program's bound on where it puts a
            //   window, which the hold within the document window then answers anyway.
            let offset = units.round().clamp(-65_536.0, 65_536.0) as i32;
            offset
        };
        // Table 340: a positive offset from `Near` or `Center` goes towards `Far`, from `Far`
        // towards `Near`. Horizontally `Far` is the right edge, the left for a right-to-left
        // language; vertically it is the bottom.
        // `true` where a positive offset moves the window leftward or upward.
        let (column, h_back) = match self.h_align {
            Align::Near => (if right_to_left { 2 } else { 0 }, right_to_left),
            Align::Center => (1, right_to_left),
            Align::Far => (if right_to_left { 0 } else { 2 }, !right_to_left),
        };
        let (row, v_back) = match self.v_align {
            Align::Near => (0, false),
            Align::Center => (1, false),
            Align::Far => (2, true),
        };
        let signed = |units: f32, back: bool| {
            let offset = offset(units);
            if back {
                offset.saturating_neg()
            } else {
                offset
            }
        };
        // Table 295's nine positions, row by row.
        let position = [[0, 1, 2], [3, 4, 5], [6, 7, 8]][row][column];
        Floating {
            size: (pixels(self.width.default), pixels(self.height.default)),
            relative: Relative::Document,
            // Nine positions, row by row: `row * 3 + column` is at most 8.
            // not a repair: the table above holds only 0 to 8, every one a position.
            position: Position::of_value(position).unwrap_or_default(),
            offscreen: Offscreen::OnScreen,
            title_bar: true,
            user_close: true,
            resize: Resize::Fixed,
            title: None,
            held_within: Some((signed(self.h_offset, h_back), signed(self.v_offset, v_back))),
        }
    }
}

/// Table 338's presentation the activation states, with Tables 339 and 340's defaults where it
/// states none: `Embedded`, opaque, no toolbar, the context click this program's.
///
/// A value of a type or name the tables do not list is said and read as the default; a `/Min`
/// past `/Max` is read swapped and a `/Default` outside them held to the nearer, each said, so
/// that the window this program opens is one the dictionary allows. `/PassContextClick` true is
/// said: "the interactive PDF processor shall allow the media runtime to provide any user
/// interface it wishes", and this program's player — the run time here — provides none, so a
/// context click is this program's as for false (ADR 3866). `/NavigationPane` true is said: the
/// pane shows "the hierarchical relationship of entities within the artwork", which a sound or a
/// video has none of.
#[must_use]
pub fn presentation(document: &Document, activation: Option<&Dictionary>) -> Presentation {
    let mut out = Presentation {
        style: Style::Embedded,
        transparent: false,
        toolbar: false,
        pass_context_click: false,
        notes: Vec::new(),
    };
    let dict = activation.map(|activation| document.get_key(activation, "Presentation"));
    let Some(dict) = dict.as_ref().and_then(Object::as_dict) else {
        return out;
    };
    let mut flag = |key: &str, default: bool| match document.get_key(dict, key) {
        Object::Null => default,
        Object::Boolean(value) => value,
        _ => {
            out.notes.push(format!(
                "Table 338's /{key} is not a boolean, so its default, {default}, is read"
            ));
            default
        }
    };
    let transparent = flag("Transparent", false);
    let toolbar = flag("Toolbar", false);
    let pass_context_click = flag("PassContextClick", false);
    let navigation_pane = flag("NavigationPane", false);
    out.transparent = transparent;
    out.toolbar = toolbar;
    out.pass_context_click = pass_context_click;
    if pass_context_click {
        out.notes.push(
            "Table 338's /PassContextClick true would let the media run time provide a context \
             click's interface; this program's player is that run time and provides none, so a \
             context click is handled by this program, as for false"
                .to_owned(),
        );
    }
    if navigation_pane {
        out.notes.push(
            "Table 338's /NavigationPane true is set aside: a sound or a video has no hierarchy \
             of entities for a navigation pane to show"
                .to_owned(),
        );
    }
    match document.get_key(dict, "Style") {
        Object::Null => {}
        Object::Name(name) if name.as_bytes() == b"Embedded" => {}
        Object::Name(name) if name.as_bytes() == b"Windowed" => {
            let window = document.get_key(dict, "Window");
            out.style = Style::Windowed(window_of(document, window.as_dict(), &mut out.notes));
        }
        _ => out.notes.push(
            "Table 338's /Style is neither Embedded nor Windowed, so the default, Embedded, is \
             read"
                .to_owned(),
        ),
    }
    out
}

/// Table 339's window and Table 340's position, their defaults where `window` states none.
fn window_of(
    document: &Document,
    window: Option<&Dictionary>,
    notes: &mut Vec<String>,
) -> RichMediaWindow {
    let sub =
        |key: &str| window.and_then(|window| document.get_key(window, key).as_dict().cloned());
    let width = extent(
        document,
        sub("Width").as_ref(),
        "Width",
        (288.0, 576.0, 72.0),
        notes,
    );
    let height = extent(
        document,
        sub("Height").as_ref(),
        "Height",
        (216.0, 432.0, 72.0),
        notes,
    );
    let position = sub("Position");
    let align = |key: &str, default: Align, notes: &mut Vec<String>| {
        let Some(position) = &position else {
            return default;
        };
        match document.get_key(position, key) {
            Object::Null => default,
            Object::Name(name) if name.as_bytes() == b"Near" => Align::Near,
            Object::Name(name) if name.as_bytes() == b"Center" => Align::Center,
            Object::Name(name) if name.as_bytes() == b"Far" => Align::Far,
            _ => {
                notes.push(format!(
                    "Table 340's /{key} is none of Near, Center and Far, so its default is read"
                ));
                default
            }
        }
    };
    let h_align = align("HAlign", Align::Far, notes);
    let v_align = align("VAlign", Align::Near, notes);
    let offset = |key: &str, notes: &mut Vec<String>| {
        let Some(position) = &position else {
            return 18.0;
        };
        match document.get_key(position, key) {
            Object::Null => 18.0,
            // not a repair: Table 340's own default, said in `notes`.
            object => number(&object).unwrap_or_else(|| {
                notes.push(format!(
                    "Table 340's /{key} is not a number, so its default, 18, is read"
                ));
                18.0
            }),
        }
    };
    let h_offset = offset("HOffset", notes);
    let v_offset = offset("VOffset", notes);
    RichMediaWindow {
        width,
        height,
        h_align,
        v_align,
        h_offset,
        v_offset,
    }
}

/// One of Table 339's `/Width` and `/Height`, `(default, max, min)` its defaults.
fn extent(
    document: &Document,
    dict: Option<&Dictionary>,
    side: &str,
    defaults: (f32, f32, f32),
    notes: &mut Vec<String>,
) -> Extent {
    let mut read = |key: &str, default: f32| {
        let Some(dict) = dict else {
            return default;
        };
        match document.get_key(dict, key) {
            Object::Null => default,
            object => match number(&object) {
                Some(value) if value > 0.0 => value,
                _ => {
                    notes.push(format!(
                        "Table 339's /{side} /{key} is not a positive number, so its default, \
                         {default}, is read"
                    ));
                    default
                }
            },
        }
    };
    let default = read("Default", defaults.0);
    let mut max = read("Max", defaults.1);
    let mut min = read("Min", defaults.2);
    if min > max {
        notes.push(format!(
            "Table 339's /{side} states a /Min of {min} past its /Max of {max}, so the two are \
             read the other way round"
        ));
        std::mem::swap(&mut min, &mut max);
    }
    // not a repair: a /Default outside /Min and /Max is held to them, said below, so that the
    //   window is one the dictionary allows.
    let held = default.clamp(min, max);
    if (held - default).abs() > f32::EPSILON {
        notes.push(format!(
            "Table 339's /{side} /Default of {default} is outside its /Min and /Max, so the \
             window is {held} units"
        ));
    }
    Extent {
        default: held,
        max,
        min,
    }
}

/// A finite number, integer or real.
fn number(object: &Object) -> Option<f32> {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        reason = "a window side or an offset in units, far inside what an f32 holds exactly enough \
                  to place a pixel"
    )]
    let value = match object {
        Object::Integer(value) => *value as f32,
        Object::Real(value) => *value as f32,
        _ => return None,
    };
    value.is_finite().then_some(value)
}

/// Whether the language `tag` names is read right to left, for Table 340's reversal of `HAlign`
/// for right-to-left reading languages: its BCP 47 script subtag
/// where it states one, otherwise its primary language subtag (ADR 3865).
///
/// The lists are a choice, not a reading — ISO 32000-2 defines no reading direction for a language
/// — of the scripts Unicode's bidirectional algorithm gives a right-to-left strong class and the
/// languages whose usual script is one of them.
fn reads_right_to_left(tag: &str) -> bool {
    const SCRIPTS: [&str; 8] = [
        "arab", "hebr", "syrc", "thaa", "nkoo", "adlm", "samr", "mand",
    ];
    const LANGUAGES: [&str; 14] = [
        "ar", "arc", "ckb", "dv", "fa", "he", "iw", "ji", "ps", "sd", "syr", "ug", "ur", "yi",
    ];
    let lower = tag.to_ascii_lowercase();
    let mut subtags = lower.split(['-', '_']);
    // not a repair: `split` yields at least one piece, so the empty tag is unreachable.
    let primary = subtags.next().unwrap_or_default();
    if let Some(script) = subtags.find(|subtag| subtag.len() == 4) {
        return SCRIPTS.contains(&script);
    }
    LANGUAGES.contains(&primary)
}

/// Table 335's `/RichMediaActivation`, where the settings state one, and the configuration it
/// loads: its `/Configuration`, or "the first element within the Configurations array".
fn configuration(
    document: &Document,
    annotation: &Dictionary,
) -> Result<(Option<Dictionary>, Dictionary), String> {
    if !is_rich_media(document, annotation) {
        return Err("the annotation is not a rich media annotation".to_owned());
    }
    let content = document.get_key(annotation, "RichMediaContent");
    let Some(content) = content.as_dict() else {
        return Err(
            "Table 333 makes /RichMediaContent required and this annotation states none".to_owned(),
        );
    };
    let activation = document
        .get_key(annotation, "RichMediaSettings")
        .as_dict()
        .map(|settings| document.get_key(settings, "Activation"))
        .and_then(|activation| activation.as_dict().cloned());
    let named = activation
        .as_ref()
        .map(|activation| document.get_key(activation, "Configuration"))
        .and_then(|configuration| configuration.as_dict().cloned());
    let configuration = named.or_else(|| {
        document
            .get_key(content, "Configurations")
            .as_array()
            .and_then(|configurations| configurations.first())
            .and_then(|first| document.resolve(first).as_dict().cloned())
    });
    match configuration {
        Some(configuration) => Ok((activation, configuration)),
        None => Err(
            "neither Table 335's /Configuration nor Table 341's /Configurations names a \
             configuration, so there is nothing to load"
                .to_owned(),
        ),
    }
}

/// The configuration's kind — `true` for `Video`, `false` for `Sound` — its instances of that kind
/// in the order Table 342's `/Instances` gives them, and how many instances are of another kind;
/// the `3D` kind and every other refused by name.
fn instances(
    document: &Document,
    configuration: &Dictionary,
) -> Result<(bool, Vec<Dictionary>, usize), String> {
    let instances: Vec<Dictionary> = document
        .get_key(configuration, "Instances")
        .as_array()
        .map_or_else(Vec::new, |instances| {
            instances
                .iter()
                .filter_map(|instance| document.resolve(instance).as_dict().cloned())
                .collect()
        });
    let subtype = |dict: &Dictionary| {
        document
            .get_key(dict, "Subtype")
            .as_name()
            .map(|name| name.as_bytes().to_vec())
    };
    let Some(kind) = subtype(configuration).or_else(|| instances.first().and_then(subtype)) else {
        return Err(
            "its configuration states no Table 342 /Subtype and has no instance whose Table 343 \
             /Subtype would decide one"
                .to_owned(),
        );
    };
    let video = match kind.as_slice() {
        b"Sound" => false,
        b"Video" => true,
        b"3D" => {
            return Err(
                "its configuration is 3D artwork, and rendering U3D and PRC artwork is excluded \
                 from this program (ISO 32000-2 clause 13)"
                    .to_owned(),
            );
        }
        other => {
            return Err(format!(
                "its configuration's kind /{} is none of the 3D, Sound and Video Table 342 \
                 defines",
                String::from_utf8_lossy(other)
            ));
        }
    };
    let kind_said = if video { "Video" } else { "Sound" };
    let total = instances.len();
    let of_kind: Vec<Dictionary> = instances
        .into_iter()
        .filter(|instance| subtype(instance).as_deref() == Some(kind.as_slice()))
        .collect();
    if of_kind.is_empty() {
        return Err(format!(
            "its {kind_said} configuration has no instance whose Table 343 /Subtype is {kind_said}"
        ));
    }
    let others = total.saturating_sub(of_kind.len());
    Ok((video, of_kind, others))
}

/// The asset of one `Sound` or `Video` instance, where the players take it: the embedded file, its
/// type, and the sentence saying the type was told from the file's name where it was.
///
/// **Matching an instance to its asset is a choice** (ADR 3867): §13.7 names no asset format for
/// either kind (RFC 0015 section 10.1), and Table 343's "The subtype shall match the asset file
/// type of the instance" binds the producer. The type is [`asset_type`]'s; a `Sound` instance plays
/// an asset this program's sound player takes, a `Video` instance one its video player takes, and
/// an asset of the other kind's type is the table's mismatch, said and not played.
fn asset(
    document: &Document,
    instance: &Dictionary,
    video: bool,
    processor: Processor<'_>,
) -> Result<(Data, String, Option<String>), String> {
    let kind_said = if video { "Video" } else { "Sound" };
    let Some(data) = file(document, &document.get_key(instance, "Asset"), "asset") else {
        return Err(format!(
            "its {kind_said} instance's Table 343 /Asset is not a file specification this reader \
             can read (§7.11)"
        ));
    };
    if !matches!(data, Data::Embedded(_)) {
        return Err(format!(
            "its {kind_said} instance's asset is not an embedded file, which §13.7.2.3.2's assets \
             name tree holds"
        ));
    }
    let Some((stated, guessed)) = asset_type(&data, video) else {
        return Err(format!(
            "its {kind_said} instance's asset states no Table 44 /Subtype and its name ends in no \
             extension this reader knows, so its type is not known and this program's players \
             do not take it"
        ));
    };
    let pictures = container::is_mp4_video(&stated);
    let sound = container::is_played_here(&stated);
    if video && sound || !video && pictures {
        return Err(format!(
            "its {kind_said} instance's asset is of type {stated}, a {}: Table 343's \"The subtype \
             shall match the asset file type of the instance\" is not met, so it is not played",
            if video { "sound" } else { "video" }
        ));
    }
    let takes = if video {
        processor.players == Players::AlsoVideoHere && pictures
    } else {
        processor.players != Players::HandOff && sound
    };
    if !takes {
        return Err(format!(
            "its {kind_said} instance's asset is of type {stated}, which this program's {} player, \
             where this window has one, does not play; it is handed over as a file to save",
            if video { "video" } else { "sound" }
        ));
    }
    let guessed = guessed.then(|| {
        format!(
            "its {kind_said} instance's asset states no Table 44 /Subtype, so its type, {stated}, \
             is read from its file name (ADR 3867)"
        )
    });
    Ok((data, stated, guessed))
}

/// An embedded asset's type, and whether it was told from the file's name rather than stated.
///
/// Table 44's `/Subtype` where the file states one — the reading a movie's file has. Where it
/// states none, the name §13.7.2.3.2 gives every asset — "The text string that represents the file
/// name in the name tree shall match the values stored for both the F and UF keys" — is read for
/// its extension: `.wav` a RIFF WAVE file, `.m4a` an MP4 file of sound, `.m4v` one of video, and
/// `.mp4`, which either may carry, the instance's kind's. That reading of a name is a choice, not
/// one §13.7 makes (ADR 3867).
fn asset_type(data: &Data, video: bool) -> Option<(String, bool)> {
    if let Some(stated) = content_type(data) {
        return Some((stated, false));
    }
    let Data::Embedded(file) = data else {
        return None;
    };
    // not a repair: §13.7.2.3.2 makes the name tree's key, /F and /UF one name, so where the
    //   specification states no /UF or /F the key is that name.
    let name = file.file_name.as_deref().unwrap_or(&file.name);
    let extension = name.rsplit_once('.')?.1.to_ascii_lowercase();
    let kind = match extension.as_str() {
        "wav" | "wave" => "audio/wav",
        "m4v" => "video/mp4",
        "mp4" if video => "video/mp4",
        "m4a" | "mp4" => "audio/mp4",
        _ => return None,
    };
    Some((kind.to_owned(), true))
}

/// Whether `annotation` is Table 333's `RichMedia` annotation.
/// The view a rich media annotation's activation invokes — Table 335's `/View`, else the first of
/// Table 341's `/Views` — read for the configuration activation loads (ADR 3881);
/// `None` for an annotation of another subtype, with no content, or stating no view.
#[must_use]
pub fn invoked_view(document: &Document, annotation: &Dictionary) -> Option<View> {
    // not a repair: an annotation whose activation loads no configuration invokes no view, and
    //   `activated` says why it loads none.
    let (activation, configuration) = configuration(document, annotation).ok()?;
    let content = document.get_key(annotation, "RichMediaContent");
    let three_d = document
        .get_key(&configuration, "Subtype")
        .as_name()
        .is_some_and(|kind| kind.as_bytes() == b"3D");
    views::view(document, activation.as_ref(), content.as_dict()?, three_d)
}

fn is_rich_media(document: &Document, annotation: &Dictionary) -> bool {
    document
        .get_key(annotation, "Subtype")
        .as_name()
        .is_some_and(|name| name.as_bytes() == b"RichMedia")
}

#[cfg(test)]
mod tests {
    use super::{Activation, Align, Deactivation, Style, activated, presentation, settings};
    use crate::media::{Players, Processor, Screens, WindowKind};
    use pdf_syntax::{Document, ObjectId};

    /// Builds a document from object bodies numbered from 1.
    fn document(objects: &[&str]) -> Document {
        use std::fmt::Write as _;
        let mut out = String::from("%PDF-2.0\n");
        let mut offsets = Vec::new();
        for (index, body) in objects.iter().enumerate() {
            offsets.push(out.len());
            let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
        }
        let xref = out.len();
        let _ = write!(
            out,
            "xref\n0 {}\n0000000000 65535 f \n",
            objects.len().saturating_add(1)
        );
        for offset in offsets {
            let _ = writeln!(out, "{offset:010} 00000 n ");
        }
        let _ = write!(
            out,
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len().saturating_add(1)
        );
        Document::open(out.into_bytes()).expect("a valid file")
    }

    fn annotation(doc: &Document) -> pdf_syntax::Dictionary {
        doc.get(ObjectId::new(2, 0))
            .as_dict()
            .cloned()
            .expect("the annotation is a dictionary")
    }

    fn players(players: Players) -> Processor<'static> {
        Processor {
            players,
            ..Processor::default()
        }
    }

    /// A rich media annotation with `settings` as its `/RichMediaSettings` and one configuration
    /// of `kind` whose one instance's asset is an embedded file of `media_type`.
    fn rich_media(settings: &str, kind: &str, media_type: &str) -> Document {
        document(&[
            "<< /Type /Catalog >>",
            &format!(
                "<< /Type /Annot /Subtype /RichMedia /Rect [0 0 10 10] /RichMediaSettings \
                 {settings} /RichMediaContent << /Assets << /Names [(clip) 5 0 R] >> \
                 /Configurations [3 0 R] >> >>"
            ),
            &format!("<< /Type /RichMediaConfiguration /Subtype /{kind} /Instances [4 0 R] >>"),
            &format!("<< /Type /RichMediaInstance /Subtype /{kind} /Asset 5 0 R >>"),
            "<< /Type /Filespec /F (clip) /UF (clip) /EF << /F 6 0 R >> >>",
            &format!(
                "<< /Type /EmbeddedFile /Subtype /{media_type} /Length 4 >>\nstream\nmoov\n\
                 endstream"
            ),
        ])
    }

    /// Table 335's three conditions and Table 336's three, each read as its name; absent, the
    /// defaults `XA` and `XD`; a name neither table lists is said and read as the default.
    #[test]
    fn each_condition_is_read_and_the_defaults_stand_where_none_is_stated() {
        let read = |settings_text: &str| {
            let doc = rich_media(settings_text, "Video", "video#2Fmp4");
            settings(&doc, &annotation(&doc)).expect("a rich media annotation")
        };
        let none = read("<< >>");
        assert_eq!(
            (none.activation, none.deactivation),
            (Activation::Explicit, Deactivation::Explicit)
        );
        for (condition, activation) in [
            ("XA", Activation::Explicit),
            ("PO", Activation::PageOpen),
            ("PV", Activation::PageVisible),
        ] {
            let read = read(&format!("<< /Activation << /Condition /{condition} >> >>"));
            assert_eq!(read.activation, activation, "{condition}");
            assert!(read.notes.is_empty(), "{:?}", read.notes);
        }
        for (condition, deactivation) in [
            ("XD", Deactivation::Explicit),
            ("PC", Deactivation::PageClose),
            ("PI", Deactivation::PageInvisible),
        ] {
            let read = read(&format!(
                "<< /Deactivation << /Condition /{condition} >> >>"
            ));
            assert_eq!(read.deactivation, deactivation, "{condition}");
        }
        let odd = read("<< /Activation << /Condition /PX >> /Deactivation << /Condition 3 >> >>");
        assert_eq!(
            (odd.activation, odd.deactivation),
            (Activation::Explicit, Deactivation::Explicit)
        );
        assert_eq!(odd.notes.len(), 2, "{:?}", odd.notes);
    }

    /// A `Video` configuration whose asset is an MP4 file of `video/` type is played here, in the
    /// annotation's rectangle, for a host that shows video, and refused by name for one that plays
    /// only sound; a `Sound` configuration's WAVE asset is played by either. `/Scripts` is said.
    #[test]
    fn a_sound_or_video_configuration_the_players_take_is_played_here() {
        let doc = rich_media(
            "<< /Activation << /Scripts [5 0 R] >> >>",
            "Video",
            "video#2Fmp4",
        );
        let clip = activated(&doc, &annotation(&doc), players(Players::AlsoVideoHere))
            .expect("a video the host shows")
            .clip;
        assert!(clip.played_here);
        assert!(clip.background.is_some(), "Table 338's Embedded rectangle");
        assert_eq!(clip.content_type.as_deref(), Some("video/mp4"));
        assert!(
            clip.unhonoured[0].starts_with("Table 335's /Scripts"),
            "{:?}",
            clip.unhonoured
        );
        let refused = activated(&doc, &annotation(&doc), players(Players::AlsoHere))
            .expect_err("no video player");
        assert!(refused.contains("video/mp4"), "{refused}");
        let doc = rich_media("<< >>", "Sound", "audio#2Fwav");
        let clip = activated(&doc, &annotation(&doc), players(Players::AlsoHere))
            .expect("a sound the host plays")
            .clip;
        assert!(clip.played_here && clip.background.is_none());
        assert!(clip.unhonoured.is_empty(), "{:?}", clip.unhonoured);
        assert!(activated(&doc, &annotation(&doc), players(Players::HandOff)).is_err());
    }

    /// The `3D` configuration is refused by name, a kind Table 342 does not define is said, and a
    /// `/Configuration` the activation names is loaded in place of the first.
    #[test]
    fn three_d_and_unknown_configurations_are_refused_and_the_named_one_is_loaded() {
        let doc = rich_media("<< >>", "3D", "model#2Fu3d");
        let refused = activated(&doc, &annotation(&doc), players(Players::AlsoVideoHere))
            .expect_err("3D is excluded");
        assert!(refused.contains("U3D and PRC"), "{refused}");
        let doc = rich_media("<< >>", "Flash", "application#2Fx-shockwave-flash");
        let refused = activated(&doc, &annotation(&doc), players(Players::AlsoVideoHere))
            .expect_err("no such kind");
        assert!(refused.contains("/Flash"), "{refused}");
        // The first configuration is 3D; the activation names the second, a video.
        let doc = document(&[
            "<< /Type /Catalog >>",
            "<< /Type /Annot /Subtype /RichMedia /Rect [0 0 10 10] /RichMediaSettings \
             << /Activation << /Configuration 7 0 R >> >> /RichMediaContent \
             << /Configurations [3 0 R 7 0 R] >> >>",
            "<< /Subtype /3D /Instances [4 0 R] >>",
            "<< /Subtype /3D /Asset 5 0 R >>",
            "<< /Type /Filespec /F (clip) /UF (clip) /EF << /F 6 0 R >> >>",
            "<< /Type /EmbeddedFile /Subtype /video#2Fmp4 /Length 4 >>\nstream\nmoov\nendstream",
            "<< /Instances [8 0 R] >>",
            "<< /Subtype /Video /Asset 5 0 R >>",
        ]);
        let clip = activated(&doc, &annotation(&doc), players(Players::AlsoVideoHere))
            .expect("the named configuration, whose kind its first instance decides")
            .clip;
        assert!(clip.played_here);
    }

    /// A host that opens and places windows, as the winit window does.
    fn windowing() -> Processor<'static> {
        Processor {
            players: Players::AlsoVideoHere,
            screens: Screens {
                windows: true,
                placed: true,
                monitors: [true; 7],
            },
            ..Processor::default()
        }
    }

    /// Table 338's default `Embedded` plays a video in the annotation's rectangle over an opaque
    /// background; `/Transparent` true paints that background at no opacity; `/Toolbar` true is the
    /// controller bar; `/PassContextClick` true is said, the context click staying this program's.
    #[test]
    fn embedded_is_the_rectangle_opaque_unless_transparent_and_the_toolbar_is_the_bar() {
        let doc = rich_media("<< >>", "Video", "video#2Fmp4");
        let played = activated(&doc, &annotation(&doc), windowing()).expect("a video");
        let background = played.clip.background.expect("Embedded, the default");
        assert!((background.opacity - 1.0).abs() < f32::EPSILON);
        assert!(played.clip.window.is_none() && !played.clip.playing.controls);
        assert!(played.notes.is_empty(), "{:?}", played.notes);
        let doc = rich_media(
            "<< /Activation << /Presentation << /Style /Embedded /Transparent true /Toolbar true \
             /PassContextClick true >> >> >>",
            "Video",
            "video#2Fmp4",
        );
        let played = activated(&doc, &annotation(&doc), windowing()).expect("a video");
        let background = played.clip.background.expect("Embedded");
        assert!(
            background.opacity.abs() < f32::EPSILON,
            "the page shows through"
        );
        assert!(played.clip.playing.controls, "Table 338's /Toolbar");
        assert!(
            played
                .notes
                .iter()
                .any(|note| note.contains("/PassContextClick")),
            "{:?}",
            played.notes
        );
        assert!(
            played.clip.unhonoured.is_empty(),
            "{:?}",
            played.clip.unhonoured
        );
    }

    /// `Windowed` with Table 339's and Table 340's defaults is a 288 by 216 window at the top
    /// right of the document window, 18 in from each edge and held within it; a host that cannot
    /// place a window presents it `Embedded`, said; a right-to-left `/Lang` reverses `HAlign`.
    #[test]
    fn windowed_opens_a_window_held_within_the_document_window_where_the_host_places_one() {
        let windowed = "<< /Activation << /Presentation << /Style /Windowed >> >> >>";
        let doc = rich_media(windowed, "Video", "video#2Fmp4");
        let played = activated(&doc, &annotation(&doc), windowing()).expect("a video");
        assert!(
            played.clip.background.is_none(),
            "nothing painted on the page"
        );
        let window = played.clip.window.expect("Windowed");
        let WindowKind::Floating(floating) = window.kind else {
            panic!("a floating window");
        };
        assert_eq!(floating.size, (288, 216));
        assert_eq!(
            (floating.position.row(), floating.position.column()),
            (0, 2)
        );
        assert_eq!(floating.held_within, Some((-18, 18)));

        let gtk = Processor {
            screens: Screens {
                windows: true,
                placed: false,
                monitors: [false; 7],
            },
            ..windowing()
        };
        let played = activated(&doc, &annotation(&doc), gtk).expect("a video");
        assert!(played.clip.window.is_none() && played.clip.background.is_some());
        assert!(
            played.notes[0].contains("presented Embedded"),
            "{:?}",
            played.notes
        );

        let stated = presentation(
            &doc,
            document(&[
                "<< /Type /Catalog >>",
                "<< /Presentation << /Style /Windowed /Window << /Width << /Default 900 /Max 100 \
                 /Min 400 >> /Position << /HAlign /Near /VAlign /Far /HOffset 5 /VOffset 7 >> >> \
                 >> >>",
            ])
            .get(ObjectId::new(2, 0))
            .as_dict(),
        );
        let Style::Windowed(window) = stated.style else {
            panic!("Windowed");
        };
        assert!(
            (window.width.default - 400.0).abs() < f32::EPSILON,
            "held to the swapped /Max"
        );
        assert_eq!((window.h_align, window.v_align), (Align::Near, Align::Far));
        assert_eq!(stated.notes.len(), 2, "{:?}", stated.notes);
        let ltr = window.floating(false);
        assert_eq!((ltr.position.value(), ltr.held_within), (6, Some((5, -7))));
        let rtl = window.floating(true);
        assert_eq!((rtl.position.value(), rtl.held_within), (8, Some((-5, -7))));
        assert!(super::reads_right_to_left("ar-EG") && super::reads_right_to_left("ku-Arab"));
        assert!(!super::reads_right_to_left("en-US") && !super::reads_right_to_left("az-Latn"));
    }

    /// Table 343: the first instance of the configuration's kind whose asset the players take is
    /// played, an asset of the other kind's type is the table's mismatch and passed over by name,
    /// an asset stating no type is read from its file name, and other kinds' instances are said.
    #[test]
    fn the_first_instance_whose_asset_matches_its_kind_is_played() {
        let doc = document(&[
            "<< /Type /Catalog >>",
            "<< /Type /Annot /Subtype /RichMedia /Rect [0 0 10 10] /RichMediaContent << /Assets \
             << /Names [(a.wav) 5 0 R (b.mp4) 7 0 R] >> /Configurations [3 0 R] >> >>",
            "<< /Subtype /Video /Instances [4 0 R 9 0 R 10 0 R] >>",
            "<< /Subtype /Video /Asset 5 0 R >>",
            "<< /Type /Filespec /F (a.wav) /UF (a.wav) /EF << /F 6 0 R >> >>",
            "<< /Type /EmbeddedFile /Subtype /audio#2Fwav /Length 4 >>\nstream\nRIFF\nendstream",
            "<< /Type /Filespec /F (b.mp4) /UF (b.mp4) /EF << /F 8 0 R >> >>",
            "<< /Type /EmbeddedFile /Length 4 >>\nstream\nmoov\nendstream",
            "<< /Subtype /Video /Asset 7 0 R >>",
            "<< /Subtype /Sound /Asset 5 0 R >>",
        ]);
        let played = activated(&doc, &annotation(&doc), windowing()).expect("the second instance");
        assert_eq!(played.clip.content_type.as_deref(), Some("video/mp4"));
        let said = played.notes.join(" | ");
        assert!(
            said.contains("1 instance(s) of a kind other than Video"),
            "{said}"
        );
        assert!(
            said.contains("instance 1") && said.contains("shall match"),
            "{said}"
        );
        assert!(said.contains("read from its file name"), "{said}");
        // A Sound instance whose asset is a video is the mismatch, and with no other it is the
        // reason the press is answered with.
        let doc = rich_media("<< >>", "Sound", "video#2Fmp4");
        let refused = activated(&doc, &annotation(&doc), windowing()).expect_err("a mismatch");
        assert!(refused.contains("shall match"), "{refused}");
    }

    /// Table 335's `/Animation` and `/View` reach the clip played: a `Linear` animation is set
    /// aside by name, since a clip carries no keyframe animation (ADR 3882), and the view's
    /// reading — here a direct view whose snapshot is no image — is said beside it (ADR 3881).
    #[test]
    fn the_activations_animation_and_view_are_said_beside_the_clip() {
        let doc = rich_media(
            "<< /Activation << /Animation << /Subtype /Linear /PlayCount 2 >> /View << /XN (v) \
             /Snapshot 5 0 R >> >> >>",
            "Video",
            "video#2Fmp4",
        );
        let played = activated(&doc, &annotation(&doc), players(Players::AlsoVideoHere))
            .expect("a video the host shows");
        assert!(
            played.clip.unhonoured.iter().any(|entry| entry
                .starts_with("Table 337's /Subtype Linear, played 2 time(s) at speed 1")),
            "{:?}",
            played.clip.unhonoured
        );
        assert!(
            played
                .notes
                .iter()
                .any(|note| note.contains("/View is a direct object")),
            "{:?}",
            played.notes
        );
        assert!(
            played
                .notes
                .iter()
                .any(|note| note.contains("not a stream holding an image XObject")),
            "{:?}",
            played.notes
        );
        let view = super::invoked_view(&doc, &annotation(&doc)).expect("a view is stated");
        assert_eq!((view.snapshot, view.params), (None, 0));
    }
}
