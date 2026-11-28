//! Where a clip's floating or full-screen window goes, from a toolkit's own list of monitors:
//! ISO 32000-2 §13.2.7.5's monitor specifier and Table 295's `/D`, `/RT`, `/P` and `/O` (RFC 0015
//! section 6, rounds 10 and 11; ADRs 3586, 3587).
//!
//! Pure arithmetic over rectangles in the desktop's pixels, so that each toolkit window maps its
//! monitors and its document window onto [`Monitor`] and [`Area`] and asks the same questions; a
//! second physical monitor is then a list of two, which a test can build where no machine has one.

use pdf_model::media::{Floating, MonitorSpecifier, Offscreen, Relative};

/// A rectangle on the virtual desktop, in physical pixels, its origin at the top left.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Area {
    /// The left edge.
    pub x: i32,
    /// The top edge.
    pub y: i32,
    /// The width.
    pub width: u32,
    /// The height.
    pub height: u32,
}

impl Area {
    /// The right edge, one past the last column, widened so that no sum overflows.
    fn right(self) -> i64 {
        i64::from(self.x).saturating_add(i64::from(self.width))
    }

    /// The bottom edge, one past the last row.
    fn bottom(self) -> i64 {
        i64::from(self.y).saturating_add(i64::from(self.height))
    }

    /// The pixels `self` and `other` share.
    #[must_use]
    pub fn overlap(self, other: Self) -> u64 {
        let width = self
            .right()
            .min(other.right())
            .saturating_sub(i64::from(self.x.max(other.x)));
        let height = self
            .bottom()
            .min(other.bottom())
            .saturating_sub(i64::from(self.y.max(other.y)));
        u64::try_from(width.max(0).saturating_mul(height.max(0))).unwrap_or(0)
    }

    /// Its pixels — Table 304's "area (in pixels squared)".
    #[must_use]
    pub fn pixels(self) -> u64 {
        u64::from(self.width).saturating_mul(u64::from(self.height))
    }
}

/// One physical monitor as a toolkit reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Monitor {
    /// What the toolkit calls it, for the sentence that says which was chosen.
    pub name: String,
    /// Where it stands on the virtual desktop.
    pub area: Area,
    /// Its colour depth in bits, where the toolkit reports one; `None` where it does not, which is
    /// never guessed.
    pub colour_depth: Option<u16>,
    /// Whether the system considers it the primary monitor; no monitor is, where the toolkit has
    /// no such notion.
    pub primary: bool,
}

/// The document window as a toolkit can describe it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentWindow {
    /// Where it stands on the virtual desktop.
    At(Area),
    /// Only the monitor holding its largest part, which a toolkit that cannot say where a window
    /// stands may still answer (GDK's `monitor_at_surface`, winit's `current_monitor` on Wayland).
    On(usize),
    /// Nothing: no monitor can be found from it.
    Unknown,
}

/// The monitor §13.2.7.5's `specifier` names among `monitors`, by its index, or why none can be
/// named.
///
/// Table 304's values are read as the table states them; value 2's "If no monitor is considered
/// primary, shall treat as case 0" is followed. "Ties may be broken in an implementation-dependent
/// manner", and this one breaks a tie towards the monitor holding the largest part of the document
/// window and then towards the earlier in the toolkit's list (ADR 3586) — a choice, not a reading.
///
/// # Errors
///
/// A sentence where the toolkit did not report what the value needs: a colour depth, or where the
/// document window stands. A host that cannot find a value says so before the clip is judged
/// (`pdf_model::media::Screens::monitors`), so this answers an error only where the facts changed.
pub fn monitor(
    specifier: MonitorSpecifier,
    monitors: &[Monitor],
    document: DocumentWindow,
) -> Result<usize, String> {
    if monitors.is_empty() {
        return Err("the toolkit reports no monitor".to_owned());
    }
    let largest = || -> Result<usize, String> {
        match document {
            DocumentWindow::At(area) => Ok(best(monitors, None, |monitor| {
                Some(monitor.area.overlap(area))
            })),
            DocumentWindow::On(index) if index < monitors.len() => Ok(index),
            _ => Err("the toolkit does not say where the document window stands".to_owned()),
        }
    };
    let preferred = largest().ok();
    match specifier {
        MonitorSpecifier::Largest => largest(),
        MonitorSpecifier::Smallest => {
            let DocumentWindow::At(area) = document else {
                return Err(
                    "the toolkit does not say how much of the document window each monitor holds"
                        .to_owned(),
                );
            };
            // "containing the smallest section": a monitor holding none of the window contains
            // no section of it.
            let holding: Vec<(usize, u64)> = monitors
                .iter()
                .enumerate()
                .map(|(index, monitor)| (index, monitor.area.overlap(area)))
                .filter(|(_, shared)| *shared > 0)
                .collect();
            let least = holding.iter().map(|(_, shared)| *shared).min();
            let tied: Vec<usize> = holding
                .iter()
                .filter(|(_, shared)| Some(*shared) == least)
                .map(|(index, _)| *index)
                .collect();
            tied.first()
                .copied()
                .ok_or_else(|| "no monitor holds any of the document window".to_owned())
        }
        MonitorSpecifier::Primary => match monitors.iter().position(|monitor| monitor.primary) {
            Some(index) => Ok(index),
            None => largest(),
        },
        MonitorSpecifier::ColourDepth => {
            if monitors
                .iter()
                .any(|monitor| monitor.colour_depth.is_none())
            {
                return Err("the toolkit does not report each monitor's colour depth".to_owned());
            }
            Ok(best(monitors, preferred, |monitor| {
                monitor.colour_depth.map(u64::from)
            }))
        }
        MonitorSpecifier::Area => Ok(best(monitors, preferred, |monitor| {
            Some(monitor.area.pixels())
        })),
        MonitorSpecifier::Height => Ok(best(monitors, preferred, |monitor| {
            Some(u64::from(monitor.area.height))
        })),
        MonitorSpecifier::Width => Ok(best(monitors, preferred, |monitor| {
            Some(u64::from(monitor.area.width))
        })),
    }
}

/// The index of the monitor with the greatest `measure`, a tie broken towards `preferred`, then
/// towards the earlier.
fn best(
    monitors: &[Monitor],
    preferred: Option<usize>,
    measure: impl Fn(&Monitor) -> Option<u64>,
) -> usize {
    let measures: Vec<u64> = monitors
        .iter()
        .map(|monitor| measure(monitor).unwrap_or(0))
        .collect();
    let greatest = measures.iter().copied().max().unwrap_or(0);
    if let Some(index) = preferred
        && measures.get(index) == Some(&greatest)
    {
        return index;
    }
    measures
        .iter()
        .position(|value| *value == greatest)
        .unwrap_or(0)
}

/// A window's rectangle of `inner` pixels as the quadrilateral [`super::placed`] fits a picture
/// into: corners clockwise from the top left, y downwards.
#[must_use]
pub fn whole(inner: (u32, u32)) -> [f32; 8] {
    #[expect(
        clippy::cast_precision_loss,
        reason = "a window's pixels are far below f32's exact integer range"
    )]
    let (width, height) = (inner.0 as f32, inner.1 as f32);
    [0.0, 0.0, width, 0.0, width, height, 0.0, height]
}

/// The smallest rectangle holding every monitor: Table 295's "full virtual desktop".
#[must_use]
pub fn desktop(monitors: &[Monitor]) -> Option<Area> {
    let left = monitors
        .iter()
        .map(|monitor| i64::from(monitor.area.x))
        .min()?;
    let top = monitors
        .iter()
        .map(|monitor| i64::from(monitor.area.y))
        .min()?;
    let right = monitors.iter().map(|monitor| monitor.area.right()).max()?;
    let bottom = monitors.iter().map(|monitor| monitor.area.bottom()).max()?;
    Some(Area {
        x: i32::try_from(left).ok()?,
        y: i32::try_from(top).ok()?,
        width: u32::try_from(right.saturating_sub(left)).ok()?,
        height: u32::try_from(bottom.saturating_sub(top)).ok()?,
    })
}

/// The window Table 295's `/RT` names, as an area: the document window, which in a program of one
/// window is the application window too, the virtual desktop, or the monitor `/M` chose.
///
/// # Errors
///
/// A sentence where the document window's place is unknown, or no monitor is reported.
pub fn relative_to(
    relative: Relative,
    monitors: &[Monitor],
    document: Option<Area>,
    chosen: usize,
) -> Result<Area, String> {
    match relative {
        Relative::Document | Relative::ApplicationWindow => document
            .ok_or_else(|| "the toolkit does not say where the document window stands".to_owned()),
        Relative::Desktop => {
            desktop(monitors).ok_or_else(|| "the toolkit reports no monitor".to_owned())
        }
        Relative::Monitor => monitors
            .get(chosen)
            .map(|monitor| monitor.area)
            .ok_or_else(|| "the toolkit reports no monitor".to_owned()),
    }
}

/// Where a floating window stands, its frame included, and the size of its picture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placement {
    /// The window, title bar and resizing handles included — what Table 295's `/P` positions.
    pub outer: Area,
    /// The rectangle the media plays in — `/D`, or less where `/O` 1 shrank it to fit.
    pub inner: (u32, u32),
}

/// Table 295's floating window placed in `reference` — the window `/RT` names — at `/P`, with a
/// frame of `frame` pixels around `/D` (left plus right, top plus bottom: "including such items as
/// title bar and resizing handles"), and `/O`'s rule applied against `monitors`.
///
/// `/P` puts the window's left edge at the reference's left, centred, or at its right in its
/// column, and its top likewise in its row. `/O`'s "positioned totally or partially offscreen
/// (that is, not visible on any physical monitor)" is a window any of whose pixels no monitor
/// shows: 0 leaves it; 1 moves it, and shrinks it if it is larger than the monitor that holds most
/// of it, onto that monitor; 2 answers that the clip is non-viable.
///
/// A rich media window (§13.7.2.2.6) is then moved by its [`Floating::held_within`] offset —
/// Table 340's `/HOffset` and `/VOffset` from the edge its alignment names — and held inside
/// `reference`, the document window that is this program's presentation area, before `/O`'s
/// rule is asked (ADR 3865).
///
/// # Errors
///
/// `/O` 2's "Consider the object to be non-viable", as a sentence.
pub fn place(
    floating: &Floating,
    reference: Area,
    frame: (u32, u32),
    monitors: &[Monitor],
) -> Result<Placement, String> {
    let (width, height) = floating.size;
    let outer_width = width.saturating_add(frame.0);
    let outer_height = height.saturating_add(frame.1);
    let along = |start: i32, room: u32, size: u32, step: u8| -> i32 {
        let spare = i64::from(room).saturating_sub(i64::from(size));
        let offset = match step {
            0 => 0,
            1 => spare.checked_div(2).unwrap_or(0),
            _ => spare,
        };
        i32::try_from(i64::from(start).saturating_add(offset)).unwrap_or(start)
    };
    let moved = |at: i32, by: i32| at.saturating_add(by);
    let outer = Area {
        x: moved(
            along(
                reference.x,
                reference.width,
                outer_width,
                floating.position.column(),
            ),
            floating.held_within.map_or(0, |offset| offset.0),
        ),
        y: moved(
            along(
                reference.y,
                reference.height,
                outer_height,
                floating.position.row(),
            ),
            floating.held_within.map_or(0, |offset| offset.1),
        ),
        width: outer_width,
        height: outer_height,
    };
    let placed = if floating.held_within.is_some() {
        held_within(outer, (width, height), frame, reference)
    } else {
        Placement {
            outer,
            inner: (width, height),
        }
    };
    let outer = placed.outer;
    let (width, height) = placed.inner;
    if on_screen(outer, monitors) {
        return Ok(placed);
    }
    match floating.offscreen {
        Offscreen::Ignore => Ok(placed),
        Offscreen::NonViable => Err(format!(
            "its floating window would stand partly off every monitor at {}, {} and Table 295's \
             /O 2 says to consider it non-viable",
            outer.x, outer.y
        )),
        Offscreen::OnScreen => {
            let Some(home) = monitors
                .iter()
                .map(|monitor| monitor.area)
                .max_by_key(|area| area.overlap(outer))
            else {
                return Ok(placed);
            };
            let inner_width = width.min(home.width.saturating_sub(frame.0)).max(1);
            let inner_height = height.min(home.height.saturating_sub(frame.1)).max(1);
            let fitted_width = inner_width.saturating_add(frame.0);
            let fitted_height = inner_height.saturating_add(frame.1);
            let clamp = |at: i32, size: u32, start: i32, room: u32| -> i32 {
                let last = i64::from(start)
                    .saturating_add(i64::from(room))
                    .saturating_sub(i64::from(size));
                let at = i64::from(at).min(last).max(i64::from(start));
                i32::try_from(at).unwrap_or(start)
            };
            Ok(Placement {
                outer: Area {
                    x: clamp(outer.x, fitted_width, home.x, home.width),
                    y: clamp(outer.y, fitted_height, home.y, home.height),
                    width: fitted_width,
                    height: fitted_height,
                },
                inner: (inner_width, inner_height),
            })
        }
    }
}

/// `outer`, around a picture of `inner` pixels inside a frame of `frame`, moved onto `reference`
/// and shrunk where it is larger: §13.7.2.2.6's "constrain the window extent to be within the
/// presentation area" (ADR 3865). A side the reference cannot hold even without its picture keeps
/// one pixel of picture and stands at the reference's near edge.
fn held_within(outer: Area, inner: (u32, u32), frame: (u32, u32), reference: Area) -> Placement {
    let inner = (
        inner.0.min(reference.width.saturating_sub(frame.0)).max(1),
        inner.1.min(reference.height.saturating_sub(frame.1)).max(1),
    );
    let width = inner.0.saturating_add(frame.0);
    let height = inner.1.saturating_add(frame.1);
    let clamp = |at: i32, size: u32, start: i32, room: u32| -> i32 {
        let last = i64::from(start)
            .saturating_add(i64::from(room))
            .saturating_sub(i64::from(size));
        let at = i64::from(at).min(last).max(i64::from(start));
        i32::try_from(at).unwrap_or(start)
    };
    Placement {
        outer: Area {
            x: clamp(outer.x, width, reference.x, reference.width),
            y: clamp(outer.y, height, reference.y, reference.height),
            width,
            height,
        },
        inner,
    }
}

/// Whether every pixel of `area` is shown by some monitor: the area's pixels, counted cell by cell
/// over the grid every edge makes, each cell inside a monitor.
fn on_screen(area: Area, monitors: &[Monitor]) -> bool {
    let mut columns = vec![i64::from(area.x), area.right()];
    let mut rows = vec![i64::from(area.y), area.bottom()];
    for monitor in monitors {
        columns.extend([i64::from(monitor.area.x), monitor.area.right()]);
        rows.extend([i64::from(monitor.area.y), monitor.area.bottom()]);
    }
    let inside = |edges: &mut Vec<i64>, low: i64, high: i64| {
        edges.retain(|edge| (low..=high).contains(edge));
        edges.sort_unstable();
        edges.dedup();
    };
    inside(&mut columns, i64::from(area.x), area.right());
    inside(&mut rows, i64::from(area.y), area.bottom());
    columns.windows(2).all(|column| {
        rows.windows(2).all(|row| {
            monitors.iter().any(|monitor| {
                let shown = monitor.area;
                i64::from(shown.x) <= column[0]
                    && column[1] <= shown.right()
                    && i64::from(shown.y) <= row[0]
                    && row[1] <= shown.bottom()
            })
        })
    })
}

#[cfg(test)]
mod tests {
    use pdf_model::media::{Floating, MonitorSpecifier, Offscreen, Position, Relative, Resize};

    use super::{Area, DocumentWindow, Monitor, desktop, monitor, place, relative_to};

    /// Two monitors side by side, a smaller primary at the left with less colour depth.
    fn two() -> Vec<Monitor> {
        vec![
            Monitor {
                name: "left".to_owned(),
                area: Area {
                    x: 0,
                    y: 0,
                    width: 1280,
                    height: 1024,
                },
                colour_depth: Some(24),
                primary: true,
            },
            Monitor {
                name: "right".to_owned(),
                area: Area {
                    x: 1280,
                    y: 0,
                    width: 1920,
                    height: 1080,
                },
                colour_depth: Some(30),
                primary: false,
            },
        ]
    }

    /// A document window mostly on the left monitor and partly on the right.
    const DOCUMENT: Area = Area {
        x: 1000,
        y: 100,
        width: 400,
        height: 300,
    };

    /// Table 304's seven values over two monitors, and value 2's "If no monitor is considered
    /// primary, shall treat as case 0".
    #[test]
    fn each_monitor_specifier_names_the_monitor_table_304_describes() {
        let monitors = two();
        let at = DocumentWindow::At(DOCUMENT);
        let named = |value: i64| {
            monitor(
                MonitorSpecifier::of_value(value).expect("listed"),
                &monitors,
                at,
            )
        };
        assert_eq!(named(0), Ok(0), "280 columns on the left, 120 on the right");
        assert_eq!(named(1), Ok(1));
        assert_eq!(named(2), Ok(0), "the primary");
        assert_eq!(named(3), Ok(1), "30 bits over 24");
        assert_eq!(named(4), Ok(1));
        assert_eq!(named(5), Ok(1));
        assert_eq!(named(6), Ok(1));
        let mut no_primary = two();
        for monitor in &mut no_primary {
            monitor.primary = false;
        }
        let on_the_right = DocumentWindow::At(Area {
            x: 2000,
            ..DOCUMENT
        });
        assert_eq!(
            monitor(MonitorSpecifier::Primary, &no_primary, on_the_right),
            Ok(1),
            "case 0: the monitor holding the largest section"
        );
    }

    /// What a toolkit does not report is said, never guessed: a monitor of unknown colour depth,
    /// a document window whose place is unknown; and a tie goes to the document window's monitor.
    #[test]
    fn what_a_toolkit_does_not_report_is_said_and_a_tie_goes_to_the_document_window() {
        let mut monitors = two();
        monitors[1].colour_depth = None;
        assert!(
            monitor(
                MonitorSpecifier::ColourDepth,
                &monitors,
                DocumentWindow::Unknown
            )
            .is_err()
        );
        assert!(
            monitor(
                MonitorSpecifier::Largest,
                &monitors,
                DocumentWindow::Unknown
            )
            .is_err()
        );
        assert!(monitor(MonitorSpecifier::Smallest, &monitors, DocumentWindow::On(0)).is_err());
        assert_eq!(
            monitor(MonitorSpecifier::Largest, &monitors, DocumentWindow::On(1)),
            Ok(1)
        );
        let mut same = two();
        same[0].colour_depth = Some(24);
        same[1].colour_depth = Some(24);
        let on_the_right = DocumentWindow::At(Area {
            x: 2000,
            ..DOCUMENT
        });
        assert_eq!(
            monitor(MonitorSpecifier::ColourDepth, &same, on_the_right),
            Ok(1),
            "a tie broken towards the document window's monitor"
        );
        assert_eq!(
            monitor(
                MonitorSpecifier::ColourDepth,
                &same,
                DocumentWindow::At(DOCUMENT)
            ),
            Ok(0)
        );
    }

    fn floating(position: i64, offscreen: Offscreen, size: (u32, u32)) -> Floating {
        Floating {
            size,
            relative: Relative::Document,
            position: Position::of_value(position).expect("listed"),
            offscreen,
            title_bar: true,
            user_close: true,
            resize: Resize::Fixed,
            title: None,
            held_within: None,
        }
    }

    /// Table 295's `/P`, its nine positions, in the window `/RT` names, the frame included.
    #[test]
    fn a_floating_window_stands_where_its_position_says_in_its_reference() {
        let monitors = two();
        let reference = Area {
            x: 100,
            y: 50,
            width: 1000,
            height: 800,
        };
        let corner = |position: i64| {
            place(
                &floating(position, Offscreen::Ignore, (200, 100)),
                reference,
                (20, 40),
                &monitors,
            )
            .map(|placed| (placed.outer.x, placed.outer.y))
        };
        assert_eq!(corner(0), Ok((100, 50)), "upper-left");
        assert_eq!(corner(1), Ok((490, 50)), "upper centre");
        assert_eq!(corner(2), Ok((880, 50)), "upper-right");
        assert_eq!(corner(4), Ok((490, 380)), "centre");
        assert_eq!(corner(8), Ok((880, 710)), "lower-right");
        assert_eq!(
            desktop(&monitors),
            Some(Area {
                x: 0,
                y: 0,
                width: 3200,
                height: 1080
            })
        );
        assert_eq!(
            relative_to(Relative::Monitor, &monitors, None, 1).map(|area| area.x),
            Ok(1280)
        );
        assert!(relative_to(Relative::Document, &monitors, None, 0).is_err());
    }

    /// §13.7.2.2.6's rich media window: Table 340's offsets move it from the edge its alignment
    /// names — the example's 18 units from the top-right corner — and "constrain the window extent
    /// to be within the presentation area" moves a window pushed past the document window's edge
    /// back onto it and shrinks one larger than it (ADR 3865).
    #[test]
    fn a_rich_media_window_is_offset_from_its_corner_and_held_within_the_document_window() {
        let monitors = two();
        let reference = Area {
            x: 100,
            y: 50,
            width: 1000,
            height: 800,
        };
        let rich = |size: (u32, u32), offset: (i32, i32)| Floating {
            held_within: Some(offset),
            ..floating(2, Offscreen::OnScreen, size)
        };
        let at = |window: &Floating| {
            place(window, reference, (20, 40), &monitors)
                .map(|placed| (placed.outer.x, placed.outer.y, placed.inner))
        };
        // Far and Near, offset 18 towards Near horizontally and towards Far vertically.
        assert_eq!(at(&rich((288, 216), (-18, 18))), Ok((774, 68, (288, 216))));
        // Pushed 500 pixels past the right edge, it is moved back flush against it.
        assert_eq!(at(&rich((288, 216), (500, 18))), Ok((792, 68, (288, 216))));
        // Larger than the document window, it is shrunk to it, frame included.
        assert_eq!(at(&rich((1200, 900), (0, 0))), Ok((100, 50, (980, 760))));
        // Table 295's window is not held within: past the edge it stays where it was put.
        // Table 295's window is placed by `/P` alone, and past the edge it stays where it was put.
        let free = floating(2, Offscreen::Ignore, (1400, 216));
        assert_eq!(at(&free).map(|(x, _, _)| x), Ok(-320));
    }

    /// Table 295's `/O`: a window partly off every monitor is left by 0, moved and shrunk onto
    /// the monitor holding most of it by 1, and non-viable by 2. The gap below the left monitor
    /// (1024 rows against the right one's 1080) is off every monitor.
    #[test]
    fn a_window_offscreen_is_left_moved_or_non_viable_as_its_o_says() {
        let monitors = two();
        let low = Area {
            x: 0,
            y: 900,
            width: 1280,
            height: 100,
        };
        let size = (400, 300);
        let left = place(
            &floating(6, Offscreen::Ignore, size),
            low,
            (0, 0),
            &monitors,
        )
        .expect("left");
        assert_eq!((left.outer.x, left.outer.y), (0, 700));
        let below = Area { y: 1000, ..low };
        assert!(
            place(
                &floating(6, Offscreen::NonViable, size),
                below,
                (0, 0),
                &monitors
            )
            .is_err()
        );
        let moved = place(
            &floating(6, Offscreen::OnScreen, size),
            below,
            (0, 0),
            &monitors,
        )
        .expect("moved");
        assert_eq!(
            (moved.outer.x, moved.outer.y),
            (0, 724),
            "onto the left monitor"
        );
        let huge = place(
            &floating(0, Offscreen::OnScreen, (5000, 5000)),
            low,
            (10, 30),
            &monitors,
        )
        .expect("shrunk");
        assert_eq!(
            huge.inner,
            (1910, 1050),
            "the right monitor holds most of it"
        );
        assert_eq!(huge.outer.x, 1280);
    }
}
