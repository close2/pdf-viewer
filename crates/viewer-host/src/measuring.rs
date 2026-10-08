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

use pdf_model::geospatial::Displayed;
use pdf_model::measurement::{Geographic, Traced};
use viewer_core::Located;

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
/// `traced` and `located` are `viewer_core::Answer::Measured`'s payload, or `None` where that
/// answered `Answer::None` — which §12.9.1 makes a statement rather than a failure: the viewport
/// chosen is "the viewport of the first point", so no viewport there means the page has said
/// nothing about what a unit is worth where the person is pointing.
///
/// The empty string for a path of no points, so that a window with the mode on and nothing
/// clicked says only what [`switched`] said.
#[must_use]
pub fn said(points: usize, traced: Option<&Traced>, located: Option<&Located>) -> String {
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
        parts.push(geospatial_sentence(geospatial, located));
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

/// Smallest departure of a registration from its affine map that is worth saying: half the last
/// place [`degrees`] prints, below which the sentence would state a difference it cannot show.
const DEPARTURE_SAID: f64 = 0.000_000_5;

/// A latitude and a longitude as this program writes them (ADR 1593): decimal degrees to six
/// places, with the hemisphere's letter rather than a sign.
///
/// Table 269 leaves the form to the processor — "[f]ormatting the displayed representation of
/// these values is controlled by the interactive PDF processor" — and the clause's own example of
/// a display system is the one "corresponding to values reported by a GPS device", which reports
/// decimal degrees. Six places is a tenth of a metre on the ground, finer than any map's
/// registration is drawn, so the last place shown is never the reading's own error. The letter
/// is because a minus sign is the one character of a coordinate a person misreads.
#[must_use]
pub fn degrees(latitude: f64, longitude: f64) -> String {
    let north = if latitude < 0.0 { 'S' } else { 'N' };
    let east = if longitude < 0.0 { 'W' } else { 'E' };
    format!(
        "{:.6}° {north}, {:.6}° {east}",
        latitude.abs(),
        longitude.abs()
    )
}

/// An easting and a northing as this program writes them (ADR 1678): each named, to two places
/// of the display system's own unit, the unit as the system's string spells it.
///
/// Table 269 leaves the form to the processor, as [`degrees`] says. A grid position is named
/// rather than lettered because an `E` after a number is also a hemisphere, and the two axes of a
/// projected system are not the two of a geographic one. Two places is a centimetre of a metre,
/// the order of the forward projection's own budget of 1.5 cm on the ground (ADR 1672), so a third
/// place would print a digit the reading does not hold.
#[must_use]
pub fn grid(easting: f64, northing: f64, unit: &str) -> String {
    format!("easting {easting:.2} {unit}, northing {northing:.2} {unit}")
}

/// What a window says about a geospatial viewport: the system, the registration, the neatline,
/// and where the last point is on the earth.
///
/// The position is `viewer_core`'s reading (ADR 1593) and the wording is here: the file's own
/// system first, `/DCS`'s beside it where the file names one — in degrees, or as an easting and a
/// northing where it is projected (ADR 1678) — how far the registration departs
/// from the map the position was read through where it departs at all, and a refusal's sentence
/// where no position is given.
fn geospatial_sentence(geospatial: &Geographic, located: Option<&Located>) -> String {
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
    match located {
        Some(Located::At {
            latitude,
            longitude,
            display,
            departure,
        }) => {
            let _ = write!(said, " — {}", degrees(*latitude, *longitude));
            match display {
                Some(Ok(Displayed::Geographic(shown))) => {
                    let _ = write!(
                        said,
                        ", displayed in /DCS as {}",
                        degrees(shown.latitude, shown.longitude)
                    );
                }
                Some(Ok(Displayed::Projected {
                    easting,
                    northing,
                    unit,
                })) => {
                    let _ = write!(
                        said,
                        ", displayed in /DCS as {}",
                        grid(*easting, *northing, unit)
                    );
                }
                Some(Err(why)) => {
                    let _ = write!(said, "; not in /DCS: {why}");
                }
                None => {}
            }
            if *departure >= DEPARTURE_SAID {
                let _ = write!(
                    said,
                    " (read through one affine map, from which the file's registration points \
                     depart by up to {departure:.6}°)"
                );
            }
        }
        Some(Located::Refused(why)) => {
            let _ = write!(said, " — no position: {why}");
        }
        None => {}
    }
    said
}

#[cfg(test)]
mod tests {
    use super::{Measuring, said, switched};
    use pdf_model::measurement::{CoordinateSystem, Geographic, Traced};
    use viewer_core::Located;

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
        assert_eq!(said(0, None, None), "");
        assert!(said(2, None, None).contains("no §12.9 viewport"));
        let bare = Traced {
            viewport: None,
            ..Traced::default()
        };
        assert_eq!(
            said(2, Some(&bare), None),
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
            said(3, Some(&traced), None),
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
        let refused = Located::Refused("the file's points are shaped as degrees".to_owned());
        let sentence = said(2, Some(&traced), Some(&refused));
        assert!(sentence.contains("EPSG 32631"), "{sentence}");
        assert!(sentence.contains("projected"), "{sentence}");
        assert!(sentence.contains("4 registration point(s)"), "{sentence}");
        assert!(sentence.contains("outside the neatline"), "{sentence}");
        assert!(
            sentence.ends_with("— no position: the file's points are shaped as degrees"),
            "{sentence}"
        );
    }

    /// A position is written in decimal degrees to six places with its hemispheres, `/DCS`'s
    /// beside it, and a registration's departure from the affine map where there is one.
    #[test]
    fn a_position_is_written_in_degrees_with_its_hemispheres() {
        let traced = Traced {
            geospatial: Some(Geographic {
                registration: 4,
                within_bounds: true,
                ..Geographic::default()
            }),
            ..Traced::default()
        };
        let exact = Located::At {
            latitude: -13.574_12,
            longitude: 171.193_325,
            display: None,
            departure: 0.0,
        };
        assert_eq!(
            said(1, Some(&traced), Some(&exact)),
            "geospatial, 4 registration point(s) — 13.574120° S, 171.193325° E"
        );
        let skewed = Located::At {
            latitude: 51.5,
            longitude: -0.125,
            display: Some(Err("the display system's datum differs".to_owned())),
            departure: 0.05,
        };
        let sentence = said(1, Some(&traced), Some(&skewed));
        assert!(sentence.contains("51.500000° N, 0.125000° W"), "{sentence}");
        assert!(sentence.contains("; not in /DCS: the display system's datum differs"));
        assert!(sentence.contains("depart by up to 0.050000°"), "{sentence}");
    }

    /// Table 269's `/DCS` is the system "used for the display of position values": a geographic
    /// one is written in degrees beside the file's own, and a projected one as its easting and
    /// northing in its own unit, each named (ADR 1678).
    #[test]
    fn a_display_system_writes_its_own_coordinates() {
        use pdf_model::geospatial::{Displayed, GeographicPosition};
        let traced = Traced {
            geospatial: Some(Geographic {
                registration: 4,
                within_bounds: true,
                ..Geographic::default()
            }),
            ..Traced::default()
        };
        let located = |display| Located::At {
            latitude: 47.5,
            longitude: 9.0,
            display: Some(Ok(display)),
            departure: 0.0,
        };
        let geographic = located(Displayed::Geographic(GeographicPosition {
            latitude: 47.5,
            longitude: -0.5,
        }));
        assert_eq!(
            said(1, Some(&traced), Some(&geographic)),
            "geospatial, 4 registration point(s) — 47.500000° N, 9.000000° E, displayed in /DCS \
             as 47.500000° N, 0.500000° W"
        );
        let projected = located(Displayed::Projected {
            easting: 500_000.0,
            northing: 5_260_729.733,
            unit: "Meter".to_owned(),
        });
        assert_eq!(
            said(1, Some(&traced), Some(&projected)),
            "geospatial, 4 registration point(s) — 47.500000° N, 9.000000° E, displayed in /DCS \
             as easting 500000.00 Meter, northing 5260729.73 Meter"
        );
    }

    /// The sentence that turns the mode on names the key that turns it off.
    #[test]
    fn the_wording_names_the_key_the_table_states() {
        assert!(switched(true).contains("m again"));
        assert_eq!(switched(false), "measuring off");
    }
}
