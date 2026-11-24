//! ISO 32000-2 §13.2.6.1's floating and full-screen windows — Table 294's `/W` 0 and 1, Table
//! 295's floating window parameters and §13.2.7.5's monitor specifier — read for a host whose own
//! player shows a clip's pictures in a window it opens (RFC 0015 section 6, rounds 10 and 11;
//! ADRs 3585, 3586).
//!
//! What a window can honour is the host's to say ([`Screens`]); this module reads what the
//! parameters ask and judges each entry against it by §13.2.2's rule — an `MH` entry that cannot
//! be honoured makes the rendition non-viable, a `BE` entry is set aside and said. Where the
//! window stands, which monitor it is on and how it is drawn are the host's, from its toolkit's
//! own list of monitors (`viewer_host::video::screens`).
//!
//! Table 295's entries "shall be treated as if they were present in the MH or BE dictionaries that
//! they are referenced from. That is, the contained entries shall be individually evaluated for
//! viability rather than the dictionary being evaluated as a whole", and "if a given entry is
//! present in both floating window parameters dictionaries, the one in the MH dictionary shall
//! take precedence", so each entry below is looked up in `MH`'s `/F` first and `BE`'s second.

use pdf_syntax::{Dictionary, Document, Object};

use super::Background;

/// What a host's windows can do for Table 294's window types, which §13.2.2's viability is judged
/// against (ADR 3585). Every field `false` — what a host that says nothing has — opens no window,
/// so `/W` 0 and 1 are judged as before: non-viable in `MH`, set aside in `BE`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Screens {
    /// Whether the host opens a window of its own for a clip's pictures: Table 294's `/W` 0, "\[a]
    /// floating window", and 1, "\[a] full-screen window that obscures all other windows".
    pub windows: bool,
    /// Whether it can put a floating window where Table 295's `/RT` and `/P` say, and on the
    /// monitor `/M` names. A toolkit that leaves every window's place to the window manager —
    /// GTK 4, or a Wayland session — cannot.
    pub placed: bool,
    /// Which of Table 304's seven monitor specifiers it can find, by value: a toolkit that reports
    /// no colour depth cannot find 3, one that cannot say where the document window stands cannot
    /// find 1.
    pub monitors: [bool; 7],
}

impl Screens {
    /// Whether the host can find the monitor `specifier` names.
    #[must_use]
    pub fn finds(&self, specifier: MonitorSpecifier) -> bool {
        self.monitors[usize::from(specifier.value())]
    }
}

/// ISO 32000-2 §13.2.7.5's monitor specifier, Table 304's seven values.
///
/// > A monitor specifier is an integer that shall identify a physical monitor attached to a system.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MonitorSpecifier {
    /// 0, "The monitor containing the largest section of the document window", and Table 294's
    /// default for `/M`.
    #[default]
    Largest,
    /// 1, "The monitor containing the smallest section of the document window".
    Smallest,
    /// 2, "Primary monitor. If no monitor is considered primary, shall treat as case 0".
    Primary,
    /// 3, "Monitor with the greatest colour depth".
    ColourDepth,
    /// 4, "Monitor with the greatest area (in pixels squared)".
    Area,
    /// 5, "Monitor with the greatest height (in pixels)".
    Height,
    /// 6, "Monitor with the greatest width (in pixels)".
    Width,
}

impl MonitorSpecifier {
    /// Table 304's value, or `None` for one the table does not list.
    #[must_use]
    pub fn of_value(value: i64) -> Option<Self> {
        Some(match value {
            0 => Self::Largest,
            1 => Self::Smallest,
            2 => Self::Primary,
            3 => Self::ColourDepth,
            4 => Self::Area,
            5 => Self::Height,
            6 => Self::Width,
            _ => return None,
        })
    }

    /// The value Table 304 gives it.
    #[must_use]
    pub fn value(self) -> u8 {
        match self {
            Self::Largest => 0,
            Self::Smallest => 1,
            Self::Primary => 2,
            Self::ColourDepth => 3,
            Self::Area => 4,
            Self::Height => 5,
            Self::Width => 6,
        }
    }
}

/// Table 295's `/RT`: "The window relative to which the floating window shall be positioned".
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Relative {
    /// 0, "The document window", the default.
    #[default]
    Document,
    /// 1, "The application window".
    ApplicationWindow,
    /// 2, "The full virtual desktop".
    Desktop,
    /// 3, "The monitor specified by M".
    Monitor,
}

/// Table 295's `/P`: where the floating window, "including such items as title bar and resizing
/// handles", stands in the window `/RT` names — 0 "Upper-left corner" to 8 "Lower-right corner",
/// row by row, and 4, "Centre", the default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position(u8);

impl Position {
    /// Table 295's default, 4, "Centre".
    pub const CENTRE: Self = Self(4);

    /// The position Table 295 numbers `value`, or `None` past 8.
    #[must_use]
    pub fn of_value(value: i64) -> Option<Self> {
        u8::try_from(value)
            // not a repair: a value outside Table 295's nine is unrecognised, which the caller
            // judges by the table's own rule for MH and BE.
            .ok()
            .filter(|value| *value <= 8)
            .map(Self)
    }

    /// The value Table 295 gives it.
    #[must_use]
    pub fn value(self) -> u8 {
        self.0
    }

    /// The column, 0 at the left edge, 1 centred and 2 at the right edge.
    #[must_use]
    pub fn column(self) -> u8 {
        self.0 % 3
    }

    /// The row, 0 at the top edge, 1 centred and 2 at the bottom edge.
    #[must_use]
    pub fn row(self) -> u8 {
        self.0 / 3
    }
}

impl Default for Position {
    fn default() -> Self {
        Self::CENTRE
    }
}

/// Table 295's `/O`: "what shall occur if the floating window is positioned totally or partially
/// offscreen (that is, not visible on any physical monitor)".
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Offscreen {
    /// 0, "Take no special action".
    Ignore,
    /// 1, "Move and/or resize the window so that it is on-screen", the default.
    #[default]
    OnScreen,
    /// 2, "Consider the object to be non-viable".
    NonViable,
}

/// Table 295's `/R`: "whether the floating window may be resized by a user".
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Resize {
    /// 0, "May not be resized" — ISO 32000-1's stated default, which ISO 32000-2's table row
    /// carries no default for.
    #[default]
    Fixed,
    /// 1, "May be resized only if aspect ratio is preserved".
    KeepingAspect,
    /// 2, "May be resized without preserving aspect ratio".
    Free,
}

/// Table 295's floating window, settled from `MH`'s `/F` and `BE`'s.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Floating {
    /// `/D`: "the dimensions of the rectangle in which the media shall play, not including such
    /// items as title bar and resizing handles", in pixels, width then height.
    pub size: (u32, u32),
    /// `/RT`.
    pub relative: Relative,
    /// `/P`.
    pub position: Position,
    /// `/O`.
    pub offscreen: Offscreen,
    /// `/T`: "If true, the floating window shall have a title bar".
    pub title_bar: bool,
    /// `/UC`: "If true, the floating window shall include user interface elements that allow a
    /// user to close a floating window".
    pub user_close: bool,
    /// `/R`.
    pub resize: Resize,
    /// `/TT`, chosen for the reader's language by §14.9.2.4, where it states text for one.
    pub title: Option<String>,
}

/// A window of the host's own a clip's pictures are shown in — Table 294's `/W` 0 or 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowKind {
    /// 0, "\[a] floating window".
    Floating(Floating),
    /// 1, "\[a] full-screen window that obscures all other windows".
    FullScreen,
}

/// Where a clip's pictures are shown when not over its screen annotation: the window, the monitor
/// it appears on and the colour behind the picture (ADR 3585).
#[derive(Debug, Clone, PartialEq)]
pub struct Window {
    /// The kind, and a floating window's parameters.
    pub kind: WindowKind,
    /// Table 294's `/M`, "which monitor in a multi-monitor system, a floating or full-screen
    /// window shall appear on".
    pub monitor: MonitorSpecifier,
    /// Table 294's `/B` — "the background colour for the rectangle in which the media is being
    /// played" — black where none is stated, the same stated choice as the annotation's rectangle
    /// ([`Background::DEFAULT_COLOUR`]). Its `/O` is 1: "It shall be ignored for full-screen and
    /// hidden windows", and a floating window this program opens is opaque.
    pub colour: [f32; 3],
}

/// One entry of the screen parameters, from `MH` or `BE`, as the clause's rules settle it.
struct Entry<'a> {
    object: Object,
    must: bool,
    name: &'a str,
}

/// Looks `name` up in `MH`'s dictionary then `BE`'s — inside each one's `/F` where `floating` —
/// and answers the first present, with which it came from.
fn entry<'a>(
    document: &Document,
    screen: &Dictionary,
    name: &'a str,
    floating: bool,
) -> Option<Entry<'a>> {
    for (key, must) in [("MH", true), ("BE", false)] {
        let options = document.get_key(screen, key);
        let Some(options) = options.as_dict() else {
            continue;
        };
        let inner;
        let holder = if floating {
            inner = document.get_key(options, "F");
            let Some(inner) = inner.as_dict() else {
                continue;
            };
            inner
        } else {
            options
        };
        if holder.get(name).is_some() {
            return Some(Entry {
                object: document.get_key(holder, name),
                must,
                name,
            });
        }
    }
    None
}

/// The judgement of one entry's value: `read` recognised it, or `MH` makes the rendition
/// non-viable and `BE` gives it its default — Table 294's and Table 295's own rule, "if they are
/// nested within an MH dictionary, the floating window parameters object (and hence the media
/// screen parameters object) shall be considered non-viable; if they are nested within a BE
/// dictionary, they shall be considered to have their default values".
fn recognised<T>(
    entry: Option<Entry<'_>>,
    table: &str,
    read: impl Fn(&Object) -> Option<T>,
    default: T,
    reasons: &mut Vec<String>,
) -> (T, Option<bool>) {
    let Some(entry) = entry else {
        return (default, None);
    };
    if let Some(value) = read(&entry.object) {
        return (value, Some(entry.must));
    }
    if entry.must {
        reasons.push(format!(
            "its media screen parameters' MH states a {table} /{} this program does not recognise",
            entry.name
        ));
    }
    (default, None)
}

/// An entry this host's windows cannot honour: in `MH` a reason the rendition is non-viable, in
/// `BE` a sentence saying it is set aside (§13.2.2).
fn settable(must: bool, what: &str, reasons: &mut Vec<String>, set_aside: &mut Vec<String>) {
    if must {
        reasons.push(format!("its media screen parameters' MH asks {what}"));
    } else {
        set_aside.push(format!(
            "the best-effort screen parameter {what}; it is set aside"
        ));
    }
}

/// The window `/W` 0 or 1 asks for, for a host whose windows `screens` describes — or `None`
/// where an `MH` entry cannot be honoured (every reason pushed to `reasons`), or where a floating
/// window's `/D` came from `BE` alone and is unusable, which leaves the clip to its annotation's
/// rectangle (said in `set_aside`).
///
/// `language` chooses `/TT`'s text by §14.9.2.4.
pub(super) fn window(
    document: &Document,
    screen: &Dictionary,
    floating: bool,
    screens: Screens,
    language: Option<&str>,
    reasons: &mut Vec<String>,
    set_aside: &mut Vec<String>,
) -> Option<Window> {
    let before = reasons.len();
    let integer = |object: &Object| object.as_integer();
    let (monitor, from) = recognised(
        entry(document, screen, "M", false),
        "Table 294",
        |object| integer(object).and_then(MonitorSpecifier::of_value),
        MonitorSpecifier::Largest,
        reasons,
    );
    let mut monitor = monitor;
    if let Some(must) = from {
        let unfound = !screens.finds(monitor);
        // A floating window's monitor is a place, so a host that places nothing cannot honour it.
        let unplaced = floating && !screens.placed;
        if unfound || unplaced {
            let why = if unplaced {
                "this window cannot choose where a floating window appears".to_owned()
            } else {
                format!(
                    "this window's toolkit does not report what Table 304's {} needs",
                    monitor.value()
                )
            };
            settable(
                must,
                &format!(
                    "for Table 294's /M {} (§13.2.7.5), and {why}",
                    monitor.value()
                ),
                reasons,
                set_aside,
            );
            monitor = MonitorSpecifier::Largest;
        }
    }
    let painted: Background = super::background(document, screen, reasons, set_aside);
    if floating
        && painted.opacity < 1.0
        && let Some(opacity) = entry(document, screen, "O", false)
    {
        settable(
            opacity.must,
            "for Table 294's /O below 1.0, a transparent floating window, and a window this \
             program opens is opaque",
            reasons,
            set_aside,
        );
    }
    let kind = if floating {
        WindowKind::Floating(floating_window(
            document, screen, screens, language, reasons, set_aside,
        )?)
    } else {
        WindowKind::FullScreen
    };
    (reasons.len() == before).then_some(Window {
        kind,
        monitor,
        colour: painted.colour,
    })
}

/// Table 295's floating window parameters, each entry judged ([`window`]'s floating half): `None`
/// where no usable `/D` was found.
fn floating_window(
    document: &Document,
    screen: &Dictionary,
    screens: Screens,
    language: Option<&str>,
    reasons: &mut Vec<String>,
    set_aside: &mut Vec<String>,
) -> Option<Floating> {
    let integer = |object: &Object| object.as_integer();
    let size = floating_size(document, screen, reasons, set_aside)?;
    let (relative, relative_from) = recognised(
        entry(document, screen, "RT", true),
        "Table 295",
        |object| {
            Some(match integer(object)? {
                0 => Relative::Document,
                1 => Relative::ApplicationWindow,
                2 => Relative::Desktop,
                3 => Relative::Monitor,
                _ => return None,
            })
        },
        Relative::Document,
        reasons,
    );
    let (position, position_from) = recognised(
        entry(document, screen, "P", true),
        "Table 295",
        |object| integer(object).and_then(Position::of_value),
        Position::CENTRE,
        reasons,
    );
    if !screens.placed {
        for (name, from) in [("RT", relative_from), ("P", position_from)] {
            if let Some(must) = from {
                settable(
                    must,
                    &format!(
                        "for Table 295's /{name}, and this window cannot choose where a \
                         floating window appears"
                    ),
                    reasons,
                    set_aside,
                );
            }
        }
    }
    let (offscreen, _) = recognised(
        entry(document, screen, "O", true),
        "Table 295",
        |object| {
            Some(match integer(object)? {
                0 => Offscreen::Ignore,
                1 => Offscreen::OnScreen,
                2 => Offscreen::NonViable,
                _ => return None,
            })
        },
        Offscreen::OnScreen,
        reasons,
    );
    let (resize, _) = recognised(
        entry(document, screen, "R", true),
        "Table 295",
        |object| {
            Some(match integer(object)? {
                0 => Resize::Fixed,
                1 => Resize::KeepingAspect,
                2 => Resize::Free,
                _ => return None,
            })
        },
        Resize::Fixed,
        reasons,
    );
    let boolean = |object: &Object| match object {
        Object::Boolean(value) => Some(*value),
        _ => None,
    };
    let (title_bar, _) = recognised(
        entry(document, screen, "T", true),
        "Table 295",
        boolean,
        true,
        reasons,
    );
    let (user_close, _) = recognised(
        entry(document, screen, "UC", true),
        "Table 295",
        boolean,
        true,
        reasons,
    );
    let title = entry(document, screen, "TT", true).and_then(|entry| {
        let items = document.resolve(&entry.object);
        super::alternate(document, items.as_array()?, language)
    });
    Some(Floating {
        size,
        relative,
        position,
        offscreen,
        title_bar,
        user_close,
        resize,
        title,
    })
}

/// Table 295's `/D`, "(Required)": two non-negative integers, the first one found in `MH`'s `/F`
/// then `BE`'s. A window of no pixels shows no picture, so a zero is not honourable either. A `/D`
/// that is not usable in `MH` makes the rendition non-viable; in `BE` it is set aside, and with no
/// `/D` at all the window cannot be made, which `None` answers.
fn floating_size(
    document: &Document,
    screen: &Dictionary,
    reasons: &mut Vec<String>,
    set_aside: &mut Vec<String>,
) -> Option<(u32, u32)> {
    let read = |object: &Object| -> Option<(u32, u32)> {
        let array = document.resolve(object);
        let [width, height] = array.as_array()? else {
            return None;
        };
        let side = |object: &Object| {
            document
                .resolve(object)
                .as_integer()
                // not a repair: a negative side is not /D's "non-negative integers", which the
                // caller judges by §13.2.2's rule for MH and BE.
                .and_then(|value| u32::try_from(value).ok())
                .filter(|value| *value > 0)
        };
        Some((side(width)?, side(height)?))
    };
    let Some(found) = entry(document, screen, "D", true) else {
        set_aside.push(
            "the floating window's /D (Table 295), which is required and absent, so no floating \
             window can be made"
                .to_owned(),
        );
        return None;
    };
    let size = read(&found.object);
    if size.is_none() {
        if found.must {
            reasons.push(
                "its media screen parameters' MH states a Table 295 /D that is not two positive \
                 integers"
                    .to_owned(),
            );
        } else {
            set_aside.push(
                "the best-effort floating window's /D (Table 295), which is not two positive \
                 integers, so no floating window can be made"
                    .to_owned(),
            );
        }
    }
    size
}
