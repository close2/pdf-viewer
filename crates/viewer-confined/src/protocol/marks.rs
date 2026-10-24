//! What one page crosses as, and the table that remembers it between a render and a frame.
//!
//! The bytes are unchanged by the split that made this module (ADR 2256).

use pdf_render::Raster;
use viewer_core::DocumentId;

use super::error::ProtocolError;
use super::wire::{Reader, Writer};
use super::{display_list, windowed};

/// One page's encoded marks, kept until the viewer is asked for a frame.
///
/// **ADR 0607's payload choice needs both sizes in one place and they are known at two different
/// moments.** The list exists at [`viewer_core::Event::NeedsRender`], where the confined process
/// is handed one; the raster is what the viewer holds when a host asks [`Query::Frame`](viewer_core::Query::Frame). This is
/// what carries the first moment's answer to the second.
///
/// The *size* comparison itself happens at the first moment and costs no rasteriser: a target's
/// pixel count times four is exactly what its RGBA raster comes to, which is the same arithmetic
/// [`decode_answer`](super::decode_answer) checks a crossing raster against.
#[derive(Debug)]
pub(super) struct Crossed {
    /// Which request produced it. See [`Marks::record`].
    token: viewer_core::RenderToken,
    /// Which document, so a window the host asks for of it can say a refusal about it.
    document: DocumentId,
    /// The marks that were encoded, which a window the host asks for is drawn from (ADR 2017):
    /// the viewer holds the same list, so this is a reference and not a copy.
    list: std::sync::Arc<pdf_render::DisplayList>,
    /// What [`display_list::encode`] wrote.
    pub(super) bytes: Vec<u8>,
    /// The target the list was measured against, which the host draws it at.
    pub(super) target: pdf_render::TargetSpec,
    /// Where the viewer places this page in the viewport, in device pixels.
    ///
    /// **Kept here since ADR 0640 because the viewer no longer holds anything for this page.**
    /// A page whose host took the display list answers [`viewer_core::Rendered::Listed`], so it
    /// is absent from [`Answer::Frame`](viewer_core::Answer::Frame) — which is where the origin used to come from — and this
    /// store is the only thing that can place it.
    ///
    /// Written by [`Marks::decide`] when the page is recorded and again by [`Marks::place`] on
    /// every command, because a scroll moves a page without redrawing it: an origin remembered
    /// from the render would place the page where it used to be.
    pub(super) origin: (f32, f32),
}

/// What one page's frame carries, which [`Marks::decide`] settles.
///
/// Named rather than returned as a `bool` because the two arms are two different pieces of work
/// — one rasterises the page and one deliberately does not — and a call site reading
/// `if marks.decide(&request)` would say nothing about which.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Carries {
    /// The marks, which this store now holds. **No raster is drawn for the page at all**, which
    /// is what [`viewer_core::Rendered::Listed`] made expressible.
    Marks,
    /// The pixels, so the page is rasterised and the viewer holds what came back.
    Pixels,
    /// The windows of a page past the bound whose marks would cross, which this store now holds
    /// because the host asked for them in place of the marks ([`Route::Window`], ADR 2017). As
    /// with [`Self::Marks`] no raster of the page is drawn.
    ///
    /// [`Route::Window`]: crate::Route::Window
    Window,
}

/// One page of [`Answer::Frame`](viewer_core::Answer::Frame), from whichever of the two places holds it.
///
/// The two are disjoint by construction — a page crossing as marks answers
/// [`viewer_core::Rendered::Listed`] and leaves the viewer holding nothing, and a page crossing
/// as pixels is forgotten by [`Marks::forget`] — and [`encode_answer`](super::encode_answer) states the tie-break
/// anyway, because "by construction" is a claim about two crates at once.
#[derive(Debug)]
pub(super) enum Placed<'a> {
    /// The pixels the viewer is holding.
    Pixels(&'a viewer_core::FrameView<'a>),
    /// The marks this store is holding, and where the viewer puts the page they are of.
    Marks(&'a Crossed),
    /// The window this store drew of a page whose marks cannot cross (ADRs 1986, 2016).
    Window(&'a windowed::Windowed, windowed::Window, &'a Raster),
}

/// The pages whose marks are what crosses, one entry apiece.
///
/// **Bounded by the pages on the screen and not by the pages ever drawn**, which is
/// [`Self::place`]'s whole subject: a confined process runs under an address-space ceiling, and
/// a store that grew by one encoded list per page a reader scrolled past would be a slow leak
/// inside the one process that must not have one.
#[derive(Debug, Default)]
pub(crate) struct Marks {
    pub(super) by_page: std::collections::BTreeMap<usize, Crossed>,
    /// The pages whose marks cannot cross drawn here as the window shows them: past the bound,
    /// and under it where the toolkit windows' rule holds (`windowed`'s module comment, ADRs 1986,
    /// 2016). A page is in one of the two maps or in neither.
    pub(super) windows: std::collections::BTreeMap<usize, windowed::Windowed>,
    /// The pages past the bound whose marks cross that the host asked to cross as windows, and
    /// the document each is of (ADR 2017). Forgotten with the page when it leaves the
    /// arrangement, so a page that comes back is priced again.
    pub(super) routed: std::collections::BTreeMap<usize, DocumentId>,
    /// The pages a draw here left marks of to the library's converter, which their frames say so
    /// the host can name them (ADR 2316). Forgotten with the page when it leaves the arrangement;
    /// a page drawn again when it comes back declines again.
    pub(super) declined: std::collections::BTreeSet<usize>,
}

impl Marks {
    /// Settles ADR 0607's payload choice for one render request.
    ///
    /// Both sides of the comparison are known here and neither needs a rasteriser: the list's
    /// price is what [`display_list::encode`] writes, and the raster's is the target's pixel
    /// count times four, which is exactly what an RGBA raster of that target comes to — the same
    /// arithmetic [`decode_raster_payload`](super::payload::decode_raster_payload) checks a crossing raster against.
    ///
    /// A refusal is not a failure here and never reaches a host as one: ADR 0607's two deferred
    /// producers, a list nested past what a backend composites, and a variant added to one of
    /// `pdf-render`'s open enumerations all mean *this page crosses as pixels*, which is the arm
    /// that was already there for the scans.
    ///
    /// `origin` is where the viewer places the page, which the caller asks it for: since ADR
    /// 0640 a page crossing as marks is drawn by nobody on this side, so this store is what has
    /// to be able to place it.
    pub(crate) fn decide(
        &mut self,
        request: &viewer_core::RenderRequest,
        origin: Option<(f32, f32)>,
    ) -> Carries {
        // A page the viewer will not place is one this store could not describe to a host, and a
        // frame it could not describe would be a page missing from the screen. Drawing it is the
        // answer that cannot be wrong. It is an arm rather than an assertion because the
        // alternative inside a confinement is a panic: as `viewer-core` stands a render is asked
        // for only where the arrangement has just placed the page and interpreted it, which is
        // exactly the condition `Query::PageGeometry` answers under.
        let Some(origin) = origin else {
            self.forget(request.page, request.token);
            return Carries::Pixels;
        };
        // A page the host asked for as windows is held as windows, and nothing is encoded; asked
        // for under the bound, it is forgotten, as the host forgets a page it no longer prices.
        if self.routed.get(&request.page) == Some(&request.document) {
            if past_the_bound(request.target) {
                self.hold_window(request, origin);
                return Carries::Window;
            }
            self.routed.remove(&request.page);
        }
        // **What a page may claim in this process is a page's pixels at the bound, whichever form
        // it crosses in**: a page past `MAX_PIXELS` is asked for only by a window that draws it from
        // its marks (ADR 1944), and its list crosses where it is no larger than the raster
        // `message_budget` already sets aside, so the ceiling's arithmetic holds for it unchanged.
        let raster = u64::from(request.target.width)
            .saturating_mul(u64::from(request.target.height))
            .min(viewer_core::MAX_PIXELS)
            .saturating_mul(4);
        match display_list::crossing(&request.list, raster) {
            display_list::Crossing::List(bytes) => {
                self.record(
                    request.page,
                    Crossed {
                        token: request.token,
                        document: request.document,
                        list: std::sync::Arc::clone(&request.list),
                        bytes,
                        target: request.target,
                        origin,
                    },
                );
                Carries::Marks
            }
            display_list::Crossing::Raster(_) => {
                self.forget(request.page, request.token);
                Carries::Pixels
            }
        }
    }

    /// Records what one page crosses as, unless a later render has already answered for it.
    ///
    /// **The token comparison is not belt and braces.** A `viewer-core` that asks for one page
    /// twice before either answer arrives keeps the *later* answer and drops the earlier one, and
    /// the worker performs the two in whichever order its own queue pops them — so a store that
    /// took the last write could hold the marks of the render the viewer threw away, and the host
    /// would draw an interpretation nothing on the screen was of. Keeping the greatest token is
    /// exactly the rule `Viewer::rendered` applies.
    fn record(&mut self, page: usize, crossed: Crossed) {
        if self.newer(page, crossed.token) {
            return;
        }
        self.windows.remove(&page);
        self.by_page.insert(page, crossed);
    }

    /// Whether either map holds an answer for `page` from a render later than `token`'s.
    fn newer(&self, page: usize, token: viewer_core::RenderToken) -> bool {
        self.by_page
            .get(&page)
            .is_some_and(|held| held.token > token)
            || self
                .windows
                .get(&page)
                .is_some_and(|held| held.token > token)
    }

    /// Records that a page whose marks cannot cross is drawn here as the window shows it, from
    /// `request`'s list, with no window drawn yet — [`Self::windows_owed`] names the first (ADRs
    /// 1986, 2016).
    pub(crate) fn hold_window(&mut self, request: &viewer_core::RenderRequest, origin: (f32, f32)) {
        if self.newer(request.page, request.token) {
            return;
        }
        self.by_page.remove(&request.page);
        self.windows.insert(
            request.page,
            windowed::Windowed {
                token: request.token,
                document: request.document,
                list: std::sync::Arc::clone(&request.list),
                target: request.target,
                origin,
                drawn: None,
            },
        );
    }

    /// Every window owed where the viewport `viewport` shows a part of a page drawn as windows
    /// that the last window drawn of it does not hold: the page, its marks, its target and the window.
    pub(crate) fn windows_owed(
        &self,
        viewport: (u32, u32),
    ) -> Vec<(
        usize,
        std::sync::Arc<pdf_render::DisplayList>,
        pdf_render::TargetSpec,
        windowed::Window,
    )> {
        self.windows
            .iter()
            .filter_map(|(page, held)| {
                held.owed(viewport).map(|window| {
                    (
                        *page,
                        std::sync::Arc::clone(&held.list),
                        held.target,
                        window,
                    )
                })
            })
            .collect()
    }

    /// Takes in how the host asked `page` to cross (ADR 2017): as windows, from the next command
    /// on, or as its marks again from its next render.
    pub(crate) fn route(&mut self, page: usize, route: crate::Route) {
        match route {
            crate::Route::Window => {
                let document = self
                    .by_page
                    .get(&page)
                    .map(|crossed| crossed.document)
                    .or_else(|| self.windows.get(&page).map(|held| held.document));
                if let Some(document) = document {
                    self.routed.insert(page, document);
                }
            }
            crate::Route::Marks => {
                self.routed.remove(&page);
            }
        }
    }

    /// Moves every page the host asked for as windows out of the marks and into the windows,
    /// with no window drawn yet: the caller draws [`Self::windows_owed`] next, in the same command,
    /// so the frame after it holds the window where the one before held the marks.
    pub(crate) fn take_routed(&mut self) {
        let asked: Vec<usize> = self
            .by_page
            .iter()
            .filter(|(page, crossed)| {
                self.routed.get(page) == Some(&crossed.document) && past_the_bound(crossed.target)
            })
            .map(|(page, _)| *page)
            .collect();
        for page in asked {
            if let Some(crossed) = self.by_page.remove(&page) {
                self.windows.insert(
                    page,
                    windowed::Windowed {
                        token: crossed.token,
                        document: crossed.document,
                        list: crossed.list,
                        target: crossed.target,
                        origin: crossed.origin,
                        drawn: None,
                    },
                );
            }
        }
    }

    /// The window drawn for `page`, kept until the next one replaces it — and the document the
    /// page is of, for what its draw declined (ADR 2290).
    pub(crate) fn drew(
        &mut self,
        page: usize,
        window: windowed::Window,
        raster: Raster,
    ) -> Option<DocumentId> {
        let held = self.windows.get_mut(&page)?;
        held.drawn = Some((window, raster));
        Some(held.document)
    }

    /// A draw here of `page` — whole, or a window of it — left marks to the library's converter,
    /// which the page's frames now say (ADR 2316).
    pub(crate) fn declined_here(&mut self, page: usize) {
        self.declined.insert(page);
    }

    /// A window of `page` the rasteriser refused: the page is let go, and the document it is of
    /// named for the sentence that says so.
    pub(crate) fn refused_window(&mut self, page: usize) -> Option<DocumentId> {
        self.windows.remove(&page).map(|held| held.document)
    }

    /// Records that a page crosses as pixels, forgetting any marks held for it.
    ///
    /// The same token rule as [`Self::record`], and it is needed in this direction too: a page
    /// whose newest interpretation cannot cross as a list — a scan, or one of ADR 0607's two
    /// deferred producers — must not be drawn from an older interpretation that could.
    fn forget(&mut self, page: usize, token: viewer_core::RenderToken) {
        if self.newer(page, token) {
            return;
        }
        self.by_page.remove(&page);
        self.windows.remove(&page);
    }

    /// Places every page this store holds where the viewer now places it, and forgets the rest.
    ///
    /// **One pass that does two jobs, because the viewer answers both with one question.** Where
    /// a page sits changes without the page being redrawn — a scroll is the ordinary case — so
    /// the origin has to be asked for rather than remembered. And a page the viewer will not
    /// place at all has left Table 29's arrangement, which is this store's eviction rule: a
    /// confined process runs under an address-space ceiling, and a store that kept one encoded
    /// list per page a reader scrolled past would be a slow leak in the one process that must
    /// not have one.
    ///
    /// **It used to be [`Answer::Frame`](viewer_core::Answer::Frame)'s own page list**, which stopped being the right
    /// population the moment a page crossing as marks stopped producing a raster: those pages
    /// are precisely the ones that answer is now silent about.
    pub(crate) fn place(&mut self, mut origin: impl FnMut(usize) -> Option<(f32, f32)>) {
        self.by_page.retain(|page, crossed| match origin(*page) {
            Some(at) => {
                crossed.origin = at;
                true
            }
            None => false,
        });
        self.windows.retain(|page, held| match origin(*page) {
            Some(at) => {
                held.origin = at;
                true
            }
            None => false,
        });
        self.routed.retain(|page, _| origin(*page).is_some());
        self.declined.retain(|page| origin(*page).is_some());
    }
}

/// Whether a target's whole raster would be past [`viewer_core::MAX_PIXELS`].
fn past_the_bound(target: pdf_render::TargetSpec) -> bool {
    u64::from(target.width).saturating_mul(u64::from(target.height)) > viewer_core::MAX_PIXELS
}

/// Encodes how the host asks a page to cross ([`FRAME_ROUTE`](super::FRAME_ROUTE), ADR 2017): its index, then `0`
/// for its marks and `1` for its windows.
pub(crate) fn encode_route(page: usize, route: crate::Route) -> Vec<u8> {
    let mut writer = Writer::new();
    writer.usize(page).u8(match route {
        crate::Route::Marks => 0,
        crate::Route::Window => 1,
    });
    writer.finish()
}

/// Reads how the host asks a page to cross.
pub(crate) fn decode_route(bytes: &[u8]) -> Result<(usize, crate::Route), ProtocolError> {
    let what = "a page's route";
    let mut reader = Reader::new(bytes);
    let page = reader.usize("a page index")?;
    let route = match reader.u8(what)? {
        0 => crate::Route::Marks,
        1 => crate::Route::Window,
        other => {
            return Err(ProtocolError::Unrecognised {
                what,
                value: other.into(),
            });
        }
    };
    reader.end(what)?;
    Ok((page, route))
}
