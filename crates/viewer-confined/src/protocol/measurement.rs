//! §12.9's measurement and a located point, as an answer carries them.
//!
//! The bytes are unchanged by the split that made this module (ADR 2256).

use super::error::ProtocolError;
use super::wire::{Reader, Writer};

/// §12.9's measurement, written out.
///
/// Every field is a string §12.9.2's algorithm produced or one Table 265 states for a person,
/// so the encoding is the six options and §12.10's reading — no numbers cross, because the
/// clause's answer is text.
pub(super) fn encode_traced(writer: &mut Writer, traced: &pdf_model::measurement::Traced) {
    writer
        .option_str(traced.viewport.as_deref())
        .option_str(traced.ratio.as_deref())
        .option_str(traced.length.as_deref())
        .option_str(traced.area.as_deref())
        .option_str(traced.angle.as_deref())
        .option_str(traced.slope.as_deref());
    match &traced.geospatial {
        Some(geospatial) => {
            writer.u8(1);
            encode_coordinate_system(writer, geospatial.system.as_ref());
            encode_coordinate_system(writer, geospatial.display_system.as_ref());
            match &geospatial.units {
                Some([linear, area, angular]) => {
                    writer.u8(1).str(linear).str(area).str(angular);
                }
                None => {
                    writer.u8(0);
                }
            }
            writer
                .usize(geospatial.registration)
                .bool(geospatial.within_bounds)
                .bool(geospatial.matrix_applies);
        }
        None => {
            writer.u8(0);
        }
    }
}

/// Tables 270 and 271's two entries, and which of the two tables this is.
fn encode_coordinate_system(
    writer: &mut Writer,
    system: Option<&pdf_model::measurement::CoordinateSystem>,
) {
    match system {
        Some(system) => {
            writer
                .u8(1)
                .bool(system.projected)
                .option_i64(system.epsg)
                .option_str(system.wkt.as_deref());
        }
        None => {
            writer.u8(0);
        }
    }
}

/// §12.10's position of a measured path's last point, as `viewer_core` read it (ADR 1593), with
/// Table 269's `/DCS` reading in either of its shapes (ADR 1678).
pub(super) fn encode_located(writer: &mut Writer, located: Option<&viewer_core::Located>) {
    use pdf_model::geospatial::Displayed;
    match located {
        None => {
            writer.u8(0);
        }
        Some(viewer_core::Located::At {
            latitude,
            longitude,
            display,
            departure,
        }) => {
            writer.u8(1).f64(*latitude).f64(*longitude);
            match display {
                None => {
                    writer.u8(0);
                }
                Some(Ok(Displayed::Geographic(shown))) => {
                    writer.u8(1).f64(shown.latitude).f64(shown.longitude);
                }
                Some(Err(why)) => {
                    writer.u8(2).str(why);
                }
                Some(Ok(Displayed::Projected {
                    easting,
                    northing,
                    unit,
                })) => {
                    writer.u8(3).f64(*easting).f64(*northing).str(unit);
                }
            }
            writer.f64(*departure);
        }
        Some(viewer_core::Located::Refused(why)) => {
            writer.u8(2).str(why);
        }
    }
}

/// [`encode_located`]'s inverse.
pub(super) fn decode_located(
    reader: &mut Reader<'_>,
) -> Result<Option<viewer_core::Located>, ProtocolError> {
    use pdf_model::geospatial::{Displayed, GeographicPosition};
    Ok(match reader.u8("whether a measured point is located")? {
        0 => None,
        1 => {
            let latitude = reader.f64("a latitude")?;
            let longitude = reader.f64("a longitude")?;
            let display = match reader.u8("whether a position has a display system")? {
                0 => None,
                1 => Some(Ok(Displayed::Geographic(GeographicPosition {
                    latitude: reader.f64("a displayed latitude")?,
                    longitude: reader.f64("a displayed longitude")?,
                }))),
                2 => Some(Err(reader.string("why a display position is refused")?)),
                3 => Some(Ok(Displayed::Projected {
                    easting: reader.f64("a displayed easting")?,
                    northing: reader.f64("a displayed northing")?,
                    unit: reader.string("a display system's linear unit")?,
                })),
                other => {
                    return Err(ProtocolError::Unrecognised {
                        what: "whether a position has a display system",
                        value: u32::from(other),
                    });
                }
            };
            Some(viewer_core::Located::At {
                latitude,
                longitude,
                display,
                departure: reader.f64("a registration's departure")?,
            })
        }
        2 => Some(viewer_core::Located::Refused(
            reader.string("why a point has no position")?,
        )),
        other => {
            return Err(ProtocolError::Unrecognised {
                what: "whether a measured point is located",
                value: u32::from(other),
            });
        }
    })
}

/// [`encode_traced`]'s inverse.
pub(super) fn decode_traced(
    reader: &mut Reader<'_>,
) -> Result<pdf_model::measurement::Traced, ProtocolError> {
    let viewport = reader.option_string("a measured viewport's name")?;
    let ratio = reader.option_string("a measured viewport's scale ratio")?;
    let length = reader.option_string("a measured length")?;
    let area = reader.option_string("a measured area")?;
    let angle = reader.option_string("a measured angle")?;
    let slope = reader.option_string("a measured slope")?;
    let geospatial = match reader.u8("whether a measurement is geospatial")? {
        0 => None,
        1 => {
            let system = decode_coordinate_system(reader)?;
            let display_system = decode_coordinate_system(reader)?;
            let units = match reader.u8("whether a geospatial measure states display units")? {
                0 => None,
                1 => Some([
                    reader.string("a linear display unit")?,
                    reader.string("an area display unit")?,
                    reader.string("an angular display unit")?,
                ]),
                other => {
                    return Err(ProtocolError::Unrecognised {
                        what: "a geospatial measure's display units",
                        value: u32::from(other),
                    });
                }
            };
            Some(pdf_model::measurement::Geographic {
                system,
                display_system,
                units,
                registration: reader.usize("how many points register a map")?,
                within_bounds: reader.bool("whether a path is inside the neatline")?,
                matrix_applies: reader.bool("whether /PCSM applies")?,
            })
        }
        other => {
            return Err(ProtocolError::Unrecognised {
                what: "whether a measurement is geospatial",
                value: u32::from(other),
            });
        }
    };
    Ok(pdf_model::measurement::Traced {
        viewport,
        ratio,
        length,
        area,
        angle,
        slope,
        geospatial,
    })
}

/// [`encode_coordinate_system`]'s inverse.
fn decode_coordinate_system(
    reader: &mut Reader<'_>,
) -> Result<Option<pdf_model::measurement::CoordinateSystem>, ProtocolError> {
    match reader.u8("whether a coordinate system is stated")? {
        0 => Ok(None),
        1 => Ok(Some(pdf_model::measurement::CoordinateSystem {
            projected: reader.bool("whether a coordinate system is projected")?,
            epsg: reader.option_i64("an EPSG reference code")?,
            wkt: reader.option_string("a Well Known Text description")?,
        })),
        other => Err(ProtocolError::Unrecognised {
            what: "whether a coordinate system is stated",
            value: u32::from(other),
        }),
    }
}
