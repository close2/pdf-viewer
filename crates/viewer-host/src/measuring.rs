//! ISO 32000-2 §12.9's measuring, as a gesture: the points a person puts down and what they read.
//!
//! # Why a host needs anything at all here
//!
//! §12.9 is arithmetic and formatting, and `pdf_model::measurement` is both of them:
//! `Viewports::traced` takes a path in default user space and answers the strings Table 267's
//! number format arrays produce. `viewer_core::Query::Measure` is the way across — a host hands
//! over viewport points and is handed back the sentence's parts.
//!
//! What is left over is a *mode* and a *wording*, and neither is a message. The clause states no
//! state for a viewer to be in: it says a measurement is what "users of interactive PDF
//! processors" perform and leaves every part of performing one to the processor. So the points
//! are the host's, the way they are drawn is the host's — a rubber band is chrome, drawn where a
//! selection highlight is drawn — and the two things three windows must not disagree about are
//! *when* a press is a point and *what* the answer says. Those are here, for [`crate::status`]'s
//! reason: the third copy of a sentence is where two hosts stop agreeing about what they are
//! saying.
//!
//! # What the mode does to the pointer, which is the one thing it takes away
//!
//! While measuring is on, a press on the page is a point rather than the start of a selection.
//! That is deliberate and it is why the mode is a mode: §12.4.2's drag and §12.9's drag are the
//! same gesture, and a window cannot tell them apart from the pointer alone. Nothing else
//! changes — no pixel of the page is different, every key still means what
//! [`crate::keys::meaning`] says, and turning it off puts the selection back.
//!
//! ADR 1191.

use pdf_model::measurement::{Geographic, Traced};

/// How many points one measurement may hold.
///
/// A path a person clicks out, not a shape a file states: a hundred presses is already a gesture
/// nobody made on purpose, and the bound keeps a mode left on from growing without end.
const MAX_POINTS: usize = 100;

/// The measuring mode: whether it is on, and the points put down so far.
///
/// Viewport points in device pixels, which is the space every other shape this program passes to
/// `viewer_core` is in (ADR 0118), so a host hands them straight to
/// `viewer_core::Query::Measure` and draws them with no conversion of its own.
#[derive(Debug, Clone, Default)]
pub struct Measuring {
    on: bool,
    points: Vec<[f32; 2]>,
}

impl Measuring {
    /// Turns measuring on, or off and empty.
    ///
    /// Answers what it turned it to, so that a host can say so in the same statement. Stopping
    /// forgets the points: a path a person has finished with is not one they want back when they
    /// press the key again half a document later.
    pub fn toggle(&mut self) -> bool {
        self.on = !self.on;
        if !self.on {
            self.points.clear();
        }
        self.on
    }

    /// Whether a press on the page is a point rather than the start of a selection.
    #[must_use]
    pub const fn is_on(&self) -> bool {
        self.on
    }

    /// Puts a point down, where the mode is on and the path has room.
    ///
    /// Answers whether anything changed, which is what tells a host whether to repaint.
    pub fn point(&mut self, at: (f32, f32)) -> bool {
        if !self.on || self.points.len() >= MAX_POINTS {
            return false;
        }
        self.points.push([at.0, at.1]);
        true
    }

    /// Empties the path without leaving the mode.
    pub fn restart(&mut self) {
        self.points.clear();
    }

    /// The points put down, for the query and for whatever the host draws over them.
    #[must_use]
    pub fn points(&self) -> &[[f32; 2]] {
        &self.points
    }
}

/// What a window says when measuring is turned on or off.
///
/// The sentence names the key, for [`crate::status::still_drawing`]'s reason: which key that is
/// is [`crate::keys`]'s decision for all three windows at once, and a window offering a different
/// one would be a window whose chrome had come apart from its table.
#[must_use]
pub fn switched(on: bool) -> String {
    if on {
        "measuring (§12.9): click points on the page, m again to stop".to_owned()
    } else {
        "measuring off".to_owned()
    }
}

/// What a window says about the path put down so far.
///
/// `traced` is `viewer_core::Answer::Measured`'s payload, or `None` where that answered
/// `Answer::None` — which §12.9.1 makes a statement rather than a failure: the viewport chosen is
/// "the viewport of the first point", so no viewport there means the page has said nothing about
/// what a unit is worth where the person is pointing.
///
/// The empty string for a path of no points, so that a window with the mode on and nothing
/// clicked says only what [`switched`] said.
#[must_use]
pub fn said(points: usize, traced: Option<&Traced>) -> String {
    if points == 0 {
        return String::new();
    }
    let Some(traced) = traced else {
        return "no measuring system here — this page states no §12.9 viewport at that point"
            .to_owned();
    };
    let mut parts: Vec<String> = Vec::new();
    if let Some(name) = &traced.viewport {
        parts.push(name.clone());
    }
    if let Some(ratio) = &traced.ratio {
        parts.push(ratio.clone());
    }
    for (label, value) in [
        ("length", &traced.length),
        ("area", &traced.area),
        ("angle", &traced.angle),
        ("slope", &traced.slope),
    ] {
        if let Some(value) = value {
            parts.push(format!("{label} {value}"));
        }
    }
    if let Some(geospatial) = &traced.geospatial {
        parts.push(geospatial_sentence(geospatial));
    }
    if parts.is_empty() {
        // A viewport with no `/Measure`, or one whose measuring system describes none of the
        // quantities these points support — Table 267 refuses a distance for a `/Y` with no
        // `/CYX` by name. Said rather than left blank: a person who clicked twice and saw
        // nothing has been told nothing at all.
        return format!("{points} point(s): this viewport states no units for them");
    }
    parts.join(" · ")
}

/// What a window says about a geospatial viewport, which is what the file states and no more.
///
/// **No latitude, and the reason is §12.10 rather than the work.** That clause states the
/// correspondence between the object's unit square and the earth — `/GPTS` against `/LPTS`, point
/// for point — and states no function between the registration points; where `/GCS` is projected
/// it names the EPSG registry and ISO 19162's grammar, both texts outside this standard. So a
/// coordinate this program cannot derive is absent, and what a person is told instead is which
/// system the map is in, how many points register it, and whether §12.10.2's neatline covers the
/// place they are pointing at.
fn geospatial_sentence(geospatial: &Geographic) -> String {
    use std::fmt::Write as _;

    let mut said = String::from("geospatial");
    if let Some(system) = &geospatial.system {
        if let Some(epsg) = system.epsg {
            // Writing into a `String` cannot fail, and the result is discarded rather than
            // unwrapped for the reason `CLAUDE.md` gives: an `unwrap` outside a test needs a
            // comment naming what cannot happen, and there is nothing here that can.
            let _ = write!(said, " EPSG {epsg}");
        } else if system.wkt.is_some() {
            said.push_str(" (system stated as Well Known Text)");
        }
        if system.projected {
            said.push_str(", projected");
        }
    }
    let _ = write!(said, ", {} registration point(s)", geospatial.registration);
    if !geospatial.within_bounds {
        said.push_str(", outside the neatline");
    }
    said.push_str(" — §12.10 states no position between them");
    said
}

#[cfg(test)]
mod tests {
    use super::{Measuring, said, switched};
    use pdf_model::measurement::{CoordinateSystem, Geographic, Traced};

    /// The mode is what decides whether a press is a point, and nothing else it does is stateful.
    #[test]
    fn the_mode_takes_points_only_while_it_is_on() {
        let mut measuring = Measuring::default();
        assert!(!measuring.is_on());
        assert!(
            !measuring.point((1.0, 2.0)),
            "a press before the key does nothing"
        );

        assert!(measuring.toggle());
        assert!(measuring.point((1.0, 2.0)));
        assert!(measuring.point((3.0, 4.0)));
        assert_eq!(measuring.points(), [[1.0, 2.0], [3.0, 4.0]]);

        measuring.restart();
        assert!(measuring.points().is_empty(), "and the mode is still on");
        assert!(measuring.is_on());

        assert!(measuring.point((5.0, 6.0)));
        assert!(!measuring.toggle());
        assert!(
            measuring.points().is_empty(),
            "stopping forgets the path, so the key is not a way to resume an old one"
        );
    }

    /// A path with no units is said rather than left blank, which is trap 5.
    #[test]
    fn a_viewport_with_no_units_says_so_rather_than_nothing() {
        assert_eq!(said(0, None), "");
        assert!(said(2, None).contains("no §12.9 viewport"));
        let bare = Traced {
            viewport: None,
            ..Traced::default()
        };
        assert_eq!(
            said(2, Some(&bare)),
            "2 point(s): this viewport states no units for them"
        );
    }

    /// Every quantity Table 267 formats reaches the sentence, in the order the table states them.
    #[test]
    fn each_of_table_267s_arrays_reaches_the_sentence() {
        let traced = Traced {
            viewport: Some("Plan".to_owned()),
            ratio: Some("1/4 in = 1 ft".to_owned()),
            length: Some("12 ft".to_owned()),
            area: Some("30 sqft".to_owned()),
            angle: Some("90 deg".to_owned()),
            slope: Some("0.5".to_owned()),
            geospatial: None,
        };
        assert_eq!(
            said(3, Some(&traced)),
            "Plan · 1/4 in = 1 ft · length 12 ft · area 30 sqft · angle 90 deg · slope 0.5"
        );
    }

    /// §12.10's viewport reports its system and says outright that it states no position.
    #[test]
    fn a_geospatial_viewport_is_named_and_its_refusal_is_stated() {
        let traced = Traced {
            geospatial: Some(Geographic {
                system: Some(CoordinateSystem {
                    projected: true,
                    epsg: Some(32631),
                    wkt: None,
                }),
                registration: 4,
                within_bounds: false,
                ..Geographic::default()
            }),
            ..Traced::default()
        };
        let sentence = said(2, Some(&traced));
        assert!(sentence.contains("EPSG 32631"), "{sentence}");
        assert!(sentence.contains("projected"), "{sentence}");
        assert!(sentence.contains("4 registration point(s)"), "{sentence}");
        assert!(sentence.contains("outside the neatline"), "{sentence}");
        assert!(sentence.contains("§12.10 states no position"), "{sentence}");
    }

    /// The sentence that turns the mode on names the key that turns it off.
    #[test]
    fn the_wording_names_the_key_the_table_states() {
        assert!(switched(true).contains("m again"));
        assert_eq!(switched(false), "measuring off");
    }
}
