//! Opt-in refresh-local provenance for the settled whole-page display experiment.
//! No identity here survives a refresh or controls rendering.
use pdf_render::Raster;
use viewer_core::{FrameView, PageGeometry};
use viewer_host::stand_in::Placed;

/// Completed rasters borrowed before partials, retained pages or oversized tiles are composed.
pub(super) struct Source<'a> {
    page: usize,
    raster: &'a Raster,
    geometry: Option<PageGeometry>,
}

pub(super) fn sources<'a>(
    frames: &[FrameView<'a>],
    geometry: impl Fn(usize) -> Option<PageGeometry>,
) -> Vec<Source<'a>> {
    frames
        .iter()
        .map(|frame| Source {
            page: frame.page,
            raster: frame.raster,
            geometry: geometry(frame.page),
        })
        .collect()
}

/// An observation of one final placement; `source_index` is local to this refresh only.
#[derive(Clone, Debug)]
pub(super) struct Observation {
    pub(super) page: usize,
    pub(super) source_index: Option<usize>,
    pub(super) native_completed: bool,
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) origin: (f32, f32),
    pub(super) extent: (u32, u32),
}

pub(super) fn observe(placed: &Placed<'_>, sources: &[Source<'_>]) -> Observation {
    let source_index = sources.iter().position(|source| {
        source.page == placed.frame.page && std::ptr::eq(source.raster, placed.frame.raster)
    });
    let size = (placed.frame.raster.width, placed.frame.raster.height);
    let native_completed = source_index
        .and_then(|index| sources.get(index))
        .and_then(|source| source.geometry)
        .is_some_and(|geometry| {
            size == (geometry.width, geometry.height)
                && placed.extent == size
                && placed.frame.origin == geometry.origin
                && !placed.retained
                && placed.frame.origin.0.is_finite()
                && placed.frame.origin.1.is_finite()
        });
    Observation {
        page: placed.frame.page,
        source_index,
        native_completed,
        width: size.0,
        height: size.1,
        origin: placed.shown_at(),
        extent: placed.extent,
    }
}

/// The first experiment accepts one completed page only. This stronger restriction avoids
/// attributing covered pixels to a source; broader arrangements require an overlap census.
/// Alpha is checked on the requested interior ROI, not inferred from a rectangle.
pub(super) fn opaque_roi(raster: &Raster, roi: (u32, u32, u32, u32)) -> bool {
    let (x, y, width, height) = roi;
    let Some(right) = x.checked_add(width) else {
        return false;
    };
    let Some(bottom) = y.checked_add(height) else {
        return false;
    };
    if x == 0
        || y == 0
        || width == 0
        || height == 0
        || right >= raster.width
        || bottom >= raster.height
    {
        return false;
    }
    let Ok(stride) = usize::try_from(raster.width) else {
        return false;
    };
    let Some(stride) = stride.checked_mul(4) else {
        return false;
    };
    (y..bottom).all(|row| {
        let begin = usize::try_from(row)
            .ok()
            .and_then(|row| row.checked_mul(stride))
            .and_then(|at| {
                usize::try_from(x)
                    .ok()
                    .and_then(|x| x.checked_mul(4))
                    .and_then(|x| at.checked_add(x))
            });
        let length = usize::try_from(width)
            .ok()
            .and_then(|width| width.checked_mul(4));
        begin
            .zip(length)
            .and_then(|(begin, length)| {
                begin
                    .checked_add(length)
                    .and_then(|end| raster.data.get(begin..end))
            })
            .is_some_and(|bytes| {
                bytes
                    .chunks_exact(4)
                    .all(|pixel| pixel.last() == Some(&255))
            })
    })
}

/// Dump only caller-requested diagnostic data; no document controls the directory or name.
/// Timing comparisons must exclude this census and IO.
pub(super) fn dump(
    directory: &std::path::Path,
    generation: u64,
    document: u64,
    observation: &Observation,
    raster: &Raster,
) -> std::io::Result<Option<u32>> {
    use std::io::{Read as _, Write as _};
    let request_path = directory.join("request");
    let file = match std::fs::File::open(&request_path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    if file.metadata()?.len() > 32 {
        return Err(std::io::Error::other("capture request exceeds bound"));
    }
    let mut request = String::new();
    file.take(32).read_to_string(&mut request)?;
    let id = request
        .trim()
        .parse::<u32>()
        .ok()
        .filter(|id| (1..=16).contains(id))
        .ok_or_else(|| std::io::Error::other("invalid bounded capture request"))?;
    let roi = (
        raster.width / 4,
        raster.height / 4,
        raster.width / 2,
        raster.height / 2,
    );
    let expected = u64::from(raster.width)
        .checked_mul(u64::from(raster.height))
        .and_then(|pixels| pixels.checked_mul(4));
    if expected != u64::try_from(raster.data.len()).ok() || raster.data.len() > 64 * 1024 * 1024 {
        return Err(std::io::Error::other(
            "raster exceeds capture bound or byte layout",
        ));
    }
    if !observation.native_completed || !opaque_roi(raster, roi) {
        return Ok(None);
    }
    // Claim the request before IO. A failed write remains evidence, never a successful receipt.
    std::fs::hard_link(
        &request_path,
        directory.join(format!("request-{id}.claimed")),
    )?;
    std::fs::remove_file(&request_path)?;
    let mut pixels = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(directory.join(format!("capture-{id}.rgba")))?;
    pixels.write_all(&raster.data)?;
    pixels.flush()?;
    let mut metadata = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(directory.join(format!("capture-{id}.json")))?;
    let source_index = observation
        .source_index
        .map_or_else(|| "null".to_owned(), |index| index.to_string());
    writeln!(
        metadata,
        "{{\"id\":{id},\"generation\":{generation},\"document\":{document},\"page\":{},\"width\":{},\"height\":{},\"origin\":[{},{}],\"roi\":[{},{},{},{}],\"source_index\":{source_index},\"format\":\"RGBA8-straight\"}}",
        observation.page,
        raster.width,
        raster.height,
        observation.origin.0,
        observation.origin.1,
        roi.0,
        roi.1,
        roi.2,
        roi.3
    )?;
    metadata.flush()?;
    Ok(Some(id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdf_render::{RasterFormat, Size};

    fn raster() -> Raster {
        Raster {
            width: 8,
            height: 8,
            format: RasterFormat::Rgba8,
            data: vec![255; 256],
        }
    }
    fn geometry() -> PageGeometry {
        PageGeometry {
            page: Size {
                width: 8.0,
                height: 8.0,
            },
            scale: 1.0,
            width: 8,
            height: 8,
            origin: (2.25, 3.75),
        }
    }
    fn placement(raster: &Raster) -> Placed<'_> {
        Placed {
            frame: FrameView {
                page: 19,
                raster,
                origin: geometry().origin,
            },
            extent: (8, 8),
            retained: false,
            held: false,
        }
    }

    #[test]
    fn current_completed_source_qualifies_but_same_size_partial_does_not() {
        let completed = raster();
        let partial = raster();
        let original = [FrameView {
            page: 19,
            raster: &completed,
            origin: geometry().origin,
        }];
        let sources = sources(&original, |_| Some(geometry()));
        let accepted = observe(&placement(&completed), &sources);
        assert!(accepted.native_completed);
        assert_eq!(accepted.source_index, Some(0));
        assert_eq!(accepted.origin, (2.0, 4.0));
        let rejected = observe(&placement(&partial), &sources);
        assert!(!rejected.native_completed);
        assert_eq!(rejected.source_index, None);
    }

    #[test]
    fn changed_target_or_retained_identity_never_qualifies() {
        let completed = raster();
        let original = [FrameView {
            page: 19,
            raster: &completed,
            origin: geometry().origin,
        }];
        let mut current = geometry();
        current.width = 9;
        let changed = sources(&original, |_| Some(current));
        assert!(!observe(&placement(&completed), &changed).native_completed);
        let missing = sources(&original, |_| None);
        assert!(!observe(&placement(&completed), &missing).native_completed);
        let sources = sources(&original, |_| Some(geometry()));
        let mut retained = placement(&completed);
        retained.retained = true;
        assert!(!observe(&retained, &sources).native_completed);
        retained.retained = false;
        retained.frame.page = 20;
        assert!(!observe(&retained, &sources).native_completed);
    }

    #[test]
    fn opaque_roi_rejects_transparency_short_data_and_noninterior_bounds() {
        let mut raster = raster();
        assert!(opaque_roi(&raster, (2, 2, 4, 4)));
        assert!(!opaque_roi(&raster, (0, 2, 4, 4)));
        assert!(!opaque_roi(&raster, (2, 2, 6, 4)));
        assert!(!opaque_roi(&raster, (u32::MAX, 2, 4, 4)));
        assert!(!opaque_roi(&raster, (2, 2, 0, 4)));
        if let Some(alpha) = raster.data.get_mut((3 * 8 + 3) * 4 + 3) {
            *alpha = 254;
        }
        assert!(!opaque_roi(&raster, (2, 2, 4, 4)));
        raster.data.truncate(20);
        assert!(!opaque_roi(&raster, (2, 2, 4, 4)));
    }
}

#[cfg(test)]
mod generation_tests {
    use super::{Snapshot, State};
    #[test]
    fn every_refresh_invalidates_the_prior_observation() {
        let state = State::default();
        let first = state.begin();
        state.applied(Some(Snapshot {
            generation: first,
            document: 1,
            placements: Vec::new(),
        }));
        assert!(state.snapshot().is_some());
        let second = state.begin();
        assert!(second > first);
        assert!(state.snapshot().is_none());
        state.applied(Some(Snapshot {
            generation: first,
            document: 1,
            placements: Vec::new(),
        }));
        assert!(
            state.snapshot().is_none(),
            "stale generation must never be reapplied"
        );
        state.applied(Some(Snapshot {
            generation: second,
            document: 1,
            placements: Vec::new(),
        }));
        assert!(state.snapshot().is_some());
        state.applied(None);
        assert!(
            state.snapshot().is_none(),
            "empty or failed conversion clears the receipt"
        );
    }
}

/// A refresh's successfully applied source descriptors, never a persistent raster identity.
#[derive(Clone, Debug)]
pub(super) struct Snapshot {
    pub(super) generation: u64,
    pub(super) document: u64,
    pub(super) placements: Vec<Observation>,
}

/// Observation state only; clearing it never changes the displayed frame.
#[derive(Debug, Default)]
pub(super) struct State {
    generation: std::cell::Cell<u64>,
    capture_failed: std::cell::Cell<bool>,
    applied: std::cell::RefCell<Option<Snapshot>>,
    directory: std::cell::OnceCell<Option<std::path::PathBuf>>,
}
impl State {
    pub(super) fn begin(&self) -> u64 {
        self.applied.replace(None);
        let generation = self.generation.get().saturating_add(1);
        self.generation.set(generation);
        generation
    }
    pub(super) fn applied(&self, snapshot: Option<Snapshot>) {
        self.applied
            .replace(snapshot.filter(|value| value.generation == self.generation.get()));
    }
    pub(super) fn snapshot(&self) -> std::cell::Ref<'_, Option<Snapshot>> {
        self.applied.borrow()
    }
    pub(super) fn disable_capture(&self) {
        self.capture_failed.set(true);
    }
    pub(super) fn directory(&self) -> Option<&std::path::Path> {
        if self.capture_failed.get() {
            return None;
        }
        self.directory
            .get_or_init(|| {
                std::env::var_os("PDFVIEWER_ZOOM_CAPTURE_DIR").map(std::path::PathBuf::from)
            })
            .as_deref()
    }
}
