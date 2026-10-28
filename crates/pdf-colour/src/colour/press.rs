//! §11.4.7 and §11.7.2: the four-component press a page or group composites in, and the table of presses one interpretation names.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

use super::ColourSpace;
use super::device_cmyk::{CMYK_CORNERS, channel};
use super::rgb_route::RgbRoute;
use crate::icc::Rendering;

/// Which press a colour's four components belong to, for the things keyed on one.
///
/// A press is recognised by the profile it was sampled from **and by the parameters it was
/// sampled under**, never by where it is held: `SAMPLED` evicts, so the same profile can be
/// sampled into two different [`Press`] values over a process's life and they are the same
/// press. `crate::shading`'s cache is keyed on [`super::compositing::Compositing`], which carries this.
///
/// The second half of that is §11.7.5.3's second bullet: "the rendering intent used shall be
/// the current rendering intent in effect at the time the `Do` operator is applied to the
/// group". A profile has three "to CIE" transforms and a black point that may be compensated
/// or not, so one profile under two renderings is two conversions out of the group's four
/// components — two presses, not one, and a key that could not tell them apart is what made the
/// parameter unreadable (ADR 1054).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PressIdentity {
    /// ADR 0263's assumed process inks — `CMYK_CORNERS` — which no document names.
    ///
    /// No rendering beside it: the grid is `CMYK_CORNERS`, a compile-time constant with no
    /// profile behind it, so there is no transform for an intent to select.
    Assumed,
    /// The profile of this [`crate::icc::Profile::identity`], sampled under this rendering.
    Profile(u128, Rendering),
    /// `DeviceCMYK`'s own four components inside a `/Luminosity` mask group whose content blends
    /// — [`device_ink_press`]. Its conversion in is §10.4.2.4's with the nominal black generation
    /// and undercolour removal, and its `Y` is §11.5.3's EXAMPLE 2; it converts nothing out.
    DeviceInk,
}

/// What one admitted press is charged: every byte a press can come to hold, whether or not this
/// page ever asks for it.
///
/// A press is the grid `sample_press` builds — `PRESS_SIDE⁴` device colours, 1.0 MB at the side
/// below — and, built on first use, §11.5.3's `Y` over the same four components
/// ([`Press::luminance`], 0.33 MB) and the `INK_TABLE_SIDE`³ separations a colour converted *into*
/// the press is searched in (`Press::table`, 79 KB). The two lazy parts are charged at
/// admission rather than when they are built, so that a page's refusal is decided before any
/// sampling is paid for and is a function of the file alone (ADR 1254's "a refusal that costs the
/// work it refused is not one"). The profile the press keeps to convert in is charged beside
/// this, at [`crate::icc::Profile::owned_bytes`].
pub const PRESS_BYTES: usize = PRESS_SIDE
    .pow(4)
    .saturating_mul(size_of::<[f32; 3]>() + size_of::<f32>())
    .saturating_add(
        super::ink_search::INK_TABLE_SIDE
            .pow(3)
            .saturating_mul(size_of::<[f32; 4]>()),
    );

/// How many sampled presses this process keeps, so that a second page need not sample again.
///
/// A **cache** bound rather than a budget, and the difference is the whole of ADR 0417: past
/// this the least recently used press is dropped and a document naming it again pays for the
/// sampling a second time, which is slower and is the same picture. So it may be sized against
/// what the population measures — the web names 28 distinct presses over 65 703 documents
/// (`examples/press_census`) — where a page's budget ([`Presses::within`]) may not.
///
/// Eight is what this process has always spent on presses, so a run holding more is holding
/// them in the interpretations that named them. It is deliberately **not** tied to the page's
/// budget: one is what a page may name, which is its need, and the other what a process keeps
/// warm.
const MAX_CACHED_PRESSES: usize = 8;

/// The presses this process has sampled, most recently used first.
///
/// The lock is taken once per press an interpretation names — never per colour, which is what
/// [`super::compositing::Compositing`] carrying the press itself buys — and it is **never** held across
/// [`sample_press`], because a lock held across work rayon can steal hangs the process (ADR
/// 0269).
static SAMPLED: Mutex<Vec<Arc<Press>>> = Mutex::new(Vec::new());

/// How many times this process has built a press out of a profile.
///
/// Together with [`presses_cached`] this is what an instrument needs to say whether the cache
/// was worth its size: a run whose samplings exceed what it holds paid [`sample_press`] more
/// than once for the same profile. Neither number says anything about a verdict — since ADR
/// 0417 the budget is the interpretation's — which is exactly what makes them measurements of
/// speed rather than of correctness.
static SAMPLINGS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// How many times this process has built a press out of a profile. See `SAMPLINGS`.
#[must_use]
pub fn presses_sampled() -> usize {
    SAMPLINGS.load(std::sync::atomic::Ordering::Relaxed)
}

/// How many sampled presses this process still holds, at most `MAX_CACHED_PRESSES`.
///
/// # Errors
///
/// A panicking cache owner leaves the count unknown: [`NoPress::Poisoned`].
pub fn presses_cached() -> Result<usize, NoPress> {
    Ok(guard(&SAMPLED)?.len())
}

/// [`PressIdentity::Assumed`]'s press, built on first use like any other.
///
/// Outside [`SAMPLED`] and outside every page's budget, because it is a pure function of
/// [`CMYK_CORNERS`] — a compile-time constant — rather than anything a document names. No
/// budget applies to a press no file asked for.
static ASSUMED: OnceLock<Arc<Press>> = OnceLock::new();

/// [`PressIdentity::DeviceInk`]'s press, built on first use and outside every budget for
/// [`ASSUMED`]'s reason: nothing a document names goes into it.
static DEVICE_INK: OnceLock<Arc<Press>> = OnceLock::new();

/// The four components a `DeviceCMYK` `/Luminosity` mask group is composited in where its content
/// blends — ISO 32000-2 §11.5.3, §11.3.5, §11.6.6 (ADR 1342).
///
/// Such a group is otherwise painted in one channel, `1 − ink ÷ 2`, which §11.3.5's blend
/// functions cannot be applied through: they act on each component "expressed in additive form",
/// and a weighted average of four complements is not four. This press carries the four as
/// §11.4.7's pair of rasters instead, and it is chosen so that nothing but the blend changes:
///
/// - **In**, a colour of another space goes through `super::ink_search::rgb_to_cmyk` — §10.4.2.4 with
///   `BG(k) = k` and `UCR(k) = k` — whose weighted ink `0.3 c + 0.59 m + 0.11 y + k` is
///   `1 − (0.3 R + 0.59 G + 0.11 B)`, because the black it adds is the black it removes from
///   weights summing to one. That is the grey the one-channel route paints the same colour in, so
///   under `Normal` the two routes composite to one mask value; a grey comes in as black alone,
///   which is §10.4.2.2's conversion. A `DeviceCMYK` colour keeps its numbers.
/// - **Out**, §11.5.3's `Y` is EXAMPLE 2's, [`pdf_render::Luminance::device_ink`], read off the
///   composited four. The grid it carries is the assumed press's, for a caller that asks; a mask
///   converts nothing to the device.
#[must_use]
pub fn device_ink_press() -> Arc<Press> {
    Arc::clone(DEVICE_INK.get_or_init(|| {
        let mut press = Press::assumed();
        press.identity = PressIdentity::DeviceInk;
        Arc::new(press)
    }))
}

/// The press this tree assumes when a document says nothing: `CMYK_CORNERS`.
#[must_use]
pub fn assumed_press() -> Arc<Press> {
    Arc::clone(ASSUMED.get_or_init(|| Arc::new(Press::assumed())))
}

/// The four components a page composites in, in both directions.
///
/// §11.4.7 gives a page a blending colour space and §11.7.2 requires colours to be converted
/// *into* it, so a press is two conversions that have to be inverses of each other — ADR 0263
/// is where that became the rule, and this is that rule with the press no longer assumed.
/// `Press::space` is the conversion out, sampled on a grid because that is what a backend
/// can be handed; the conversion in is `Press::profile`'s own `B2A` where the profile
/// carries one (ADR 0796), and otherwise `Press::table`, searched against the same grid.
#[derive(Debug)]
pub struct Press {
    /// §11.4.7's conversion out of the space, as the grid a backend interpolates.
    pub(super) space: pdf_render::BlendingSpace,
    /// Which press this is: the profile it was sampled from, or the assumed inks.
    pub(super) identity: PressIdentity,
    /// The profile, where it states the conversion *in* itself.
    ///
    /// §8.6.5.5 requires a profile used as a blending colour space to carry "from CIE"
    /// information "because the group colour space shall be used as both the destination for
    /// objects being painted within the group and the source for the group's results", and
    /// this is that destination route: [`crate::icc::Profile::to_device`]. Held only for a
    /// profile [`crate::icc::Profile::is_bidirectional`] answers for, so that a press without
    /// one — the assumed inks, or a profile a file states in breach of that clause — is the
    /// right inverse it always was.
    pub(super) profile: Option<Box<crate::icc::Profile>>,
    /// The parameters the grid was sampled under, so the conversion *in* can mirror them.
    ///
    /// §11.7.5.3's second bullet fixes them at the group's `Do`; [`super::ink_search::xyz_to_ink`] asks this
    /// rather than assuming compensation, because the two directions are inverses of each
    /// other and an inverse taken under other parameters is not one.
    pub(super) rendering: Rendering,
    /// [`super::ink_search::search_ink`] over a grid of sRGB, built on first use. See [`Press::table`].
    pub(super) table: OnceLock<Vec<[f32; 4]>>,
    /// §11.5.3's `Y` over the four components, built on first use. See [`Press::luminance`].
    luminance: OnceLock<Option<pdf_render::Luminance>>,
}

impl Press {
    /// Which press this is.
    #[must_use]
    pub fn identity(&self) -> PressIdentity {
        self.identity
    }

    /// Whether this press was sampled from the profile of this identity, under any rendering.
    ///
    /// §11.7.2 redefines `DeviceCMYK` inside a group whose blending space is that profile, and
    /// a colour already stated in the profile is already in the group's components — which is
    /// true whichever of the profile's transforms the conversion *out* was sampled through, so
    /// the rendering in [`PressIdentity::Profile`] is deliberately not compared here (ADR 1054).
    #[must_use]
    pub fn is_of_profile(&self, profile: u128) -> bool {
        matches!(self.identity, PressIdentity::Profile(held, _) if held == profile)
    }

    /// The parameters this press's grid was sampled under. See [`PressIdentity`].
    #[must_use]
    pub fn rendering(&self) -> Rendering {
        self.rendering
    }

    /// The press [`CMYK_CORNERS`] describes, whose grid is those sixteen corners.
    fn assumed() -> Self {
        let mut corners = [[0.0f32; 3]; 16];
        for (target, source) in corners.iter_mut().zip(CMYK_CORNERS) {
            for (component, value) in target.iter_mut().zip(source) {
                *component = f32::from(value) / 255.0;
            }
        }
        Self {
            space: pdf_render::BlendingSpace::new(2, corners.into())
                .unwrap_or_else(|| unreachable!("sixteen samples is a grid of side two")),
            identity: PressIdentity::Assumed,
            profile: None,
            // The assumed inks are interpolated between sixteen constants, so no transform of
            // any profile is selected and this is what [`super::ink_search::xyz_to_ink`]'s search reads.
            rendering: Rendering::compensating(),
            table: OnceLock::new(),
            luminance: OnceLock::new(),
        }
    }

    /// Whether the conversion into this press is the profile's own `B2A` table.
    ///
    /// `false` for the assumed inks and for a profile without one, whose conversion in is a
    /// right inverse of the conversion out (`super::ink_search::rgb_to_ink`).
    #[must_use]
    pub fn converts_in_by_profile(&self) -> bool {
        self.profile.is_some()
    }

    /// The conversion out of this press's four components, as the table a backend is handed.
    ///
    /// §11.4.7 converts a page composited in its blending colour space to the device's *once*,
    /// at the end, and that happens in a backend where no colour space exists (see
    /// [`pdf_render::Color`]). So the table goes down with the display list, and this is the
    /// one place it is built: `a_backends_table_is_this_crates_own_conversion` checks that
    /// interpolating the assumed press's is `super::device_cmyk::cmyk` to the last bit, which is what keeps the
    /// "one route from a colour to the screen" rule in this module's header true of it.
    #[must_use]
    pub fn blending_space(&self) -> pdf_render::BlendingSpace {
        self.space.clone()
    }

    /// ISO 32000-2 §11.5.3's `Y` over this press's four components, or `None` where the
    /// press is not a profile's.
    ///
    /// §11.3.4 lists an `ICCBased` bi-directional 'CMYK' space among the blending colour
    /// spaces and §8.6.5.1 makes it CIE-based, so a `/Luminosity` mask group naming one takes
    /// §11.5.3's colorimetric branch:
    ///
    /// > For CIE-based spaces, convert to the CIE 1931 XYZ space and use the Y component as
    /// > the luminosity. This produces a colorimetrically correct luminosity.
    ///
    /// That `Y` is one function of all four components at once, so it is sampled — **at the
    /// very grid points `sample_press` already samples for the conversion out**, which is
    /// what makes the fidelity argument the same one `PRESS_SIDE` carries rather than a new
    /// one: the two grids agree point for point on where they are evaluated, and a
    /// luminosity read between them departs by the profile's own curvature exactly as the
    /// device colour does. Interpolated over sixteen corners by `pdf_render::Luminance`.
    ///
    /// **Without §8.6.5.9's black point compensation**, as [`RgbRoute::luminance`] is one
    /// axis down: the clause asks for the colour's XYZ, and the compensation is a step
    /// toward a destination rather than part of it.
    ///
    /// # What it costs, and why it is behind a lock rather than in `sample_press`
    ///
    /// `PRESS_SIDE`⁴ profile evaluations — 83 521 of them — and 334 KB, which is a third of
    /// the grid beside it. Almost every press a document names is a *page*'s or an output
    /// intent's and never carries a mask, so paying this inside `sample_press` would put it
    /// on the launch path of every four-component page for a number no such page reads. Built
    /// on first use for the same reason `Press::table` is, and once per press rather than
    /// once per soft-mask dictionary — `6081357.pdf` states 912 masks on one page (ADR 0851).
    #[must_use]
    pub fn luminance(&self) -> Option<pdf_render::Luminance> {
        self.luminance
            .get_or_init(|| {
                if self.identity == PressIdentity::DeviceInk {
                    return Some(pdf_render::Luminance::device_ink());
                }
                let profile = self.profile.as_ref()?;
                let side = PRESS_SIDE;
                let last = side.saturating_sub(1);
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "a grid index below 17 and the side itself, both exact in f32"
                )]
                let at = |index: usize| index as f32 / last as f32;
                let mut samples = Vec::with_capacity(side.checked_pow(4)?);
                for black in 0..side {
                    for yellow in 0..side {
                        for magenta in 0..side {
                            for cyan in 0..side {
                                let xyz = profile.to_xyz_with(
                                    &[at(cyan), at(magenta), at(yellow), at(black)],
                                    Rendering::without_black_point(),
                                );
                                samples.push(channel(xyz[1]));
                            }
                        }
                    }
                }
                pdf_render::Luminance::ink_grid(side, Arc::from(samples))
            })
            .clone()
    }
}

/// The press a four-component ICC profile describes, or `None` if this process holds too many.
///
/// # What this is, and which clause asks for it
///
/// A document that states a four-component `ICCBased` blending colour space (§11.7.2), or that
/// names what its `DeviceCMYK` *is* through §8.6.5.6's `/DefaultCMYK` or §14.11.5's output
/// intent, is naming a press whose four components are not this tree's assumed inks. §11.3.4
/// composites per component, so compositing in ours is a different picture.
///
/// # Why sampling the profile is the conversion out, and `B2A` is the conversion in
///
/// The conversion **out** is the profile's own `A2B`, which this crate has evaluated since ADR
/// 0009, sampled here onto a grid because a backend interpolates a table rather than running a
/// colour management engine per pixel. §14.11.5's Table 401 names that direction for exactly
/// this use:
///
/// > The output transformation uses the profile's "from CIE" information (BToA in ICC
/// > terminology); the "to CIE" (AToB) information may optionally be used to remap source
/// > colour values to some other destination colour space, such as for screen preview or
/// > hardcopy proofing.
///
/// A screen is what this processor has, so the optional clause is the one in force.
///
/// The conversion **in** is the profile's `B2A` where it carries one, since ADR 0796, and
/// §8.6.5.5 is the clause that says so — a blending-space profile "shall have both 'to CIE'
/// ( AToB ) and 'from CIE' ( BToA ) information. This is because the group colour space shall
/// be used as both the destination for objects being painted within the group and the source
/// for the group's results." §10.3.1 hands a CIE-to-CIE conversion to the ICC specification
/// and §10.4.2.1 ranks that route above §10.4.2.4's classic one for an ICC-enabled processor,
/// so the profile's own table is the standard's answer and not a choice. Two maps are built on
/// one page, and the clause makes them the file's two: what an opaque mark loses on
/// the round trip through them is the profile's own residue, which is the picture the producer
/// specified rather than one this tree tuned to come back exactly. A profile *without* one
/// keeps the right inverse of its grid — the construction ADR 0263 built for the assumed inks,
/// with the press no longer assumed — which is the file's own breach of the clause drawn as
/// well as it can be.
///
/// A profile this crate cannot parse never reaches here, and §8.6.5.5 answers that case
/// itself:
///
/// > If this entry is omitted and the PDF reader does not understand the ICC profile data, the
/// > colour space that shall be used is DeviceGray , DeviceRGB , or DeviceCMYK , depending on
/// > whether the value of N is 1 , 3 , or 4 , respectively.
///
/// For four components that is `DeviceCMYK`, which [`assumed_press`] already composites in.
///
/// # The budget this does *not* spend
///
/// This is the sampling and the cache behind it, and it refuses nothing a document states: a
/// profile of four components always comes back with a press. What is bounded is how many
/// distinct presses one **interpretation** may name, and that is [`Presses`], which is where
/// §11.7.2's refusal is decided. ADR 0417.
///
/// # Errors
///
/// [`NoPress::Unsampled`] for a profile of other than four components, and
/// [`NoPress::Poisoned`] where the process's cache of presses was poisoned (ADR 2170).
pub fn press_for_profile(
    profile: &crate::icc::Profile,
    rendering: Rendering,
) -> Result<Arc<Press>, NoPress> {
    if profile.channels() != 4 {
        return Err(NoPress::Unsampled);
    }
    let identity = PressIdentity::Profile(profile.identity(), rendering);
    if let Some(found) = cached(identity)? {
        return Ok(found);
    }
    // Sampled with no lock held. That is not an optimisation: a `Mutex` held across work rayon can
    // steal hangs the process (ADR 0269), and it is why the entry is looked up again below rather
    // than assumed absent.
    let space = sample_press(profile, rendering).ok_or(NoPress::Unsampled)?;
    SAMPLINGS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let sampled = Arc::new(Press {
        space,
        identity,
        // Cloned rather than borrowed because a press outlives the colour space that named it
        // — it is cached across interpretations — and held only where it will be asked.
        profile: profile
            .is_bidirectional()
            .then(|| Box::new(profile.clone())),
        rendering,
        table: OnceLock::new(),
        luminance: OnceLock::new(),
    });
    let mut held = guard(&SAMPLED)?;
    if let Some(found) = held
        .iter()
        .find(|press| press.identity == identity)
        .map(Arc::clone)
    {
        // Another thread sampled the same profile while this one was working. Its copy wins,
        // so that one identity is one press and `crate::shading`'s cache cannot hold two
        // entries meaning the same thing; this one is dropped.
        return Ok(found);
    }
    held.insert(0, Arc::clone(&sampled));
    held.truncate(MAX_CACHED_PRESSES);
    Ok(sampled)
}

/// The press already sampled from a profile of this identity, if this process still holds it.
///
/// Moves the entry to the front, which is the whole of the eviction policy: [`SAMPLED`] is
/// most-recently-used first and [`press_for_profile`] truncates the tail.
fn cached(identity: PressIdentity) -> Result<Option<Arc<Press>>, NoPress> {
    let mut held = guard(&SAMPLED)?;
    let Some(at) = held.iter().position(|press| press.identity == identity) else {
        return Ok(None);
    };
    let found = held.remove(at);
    held.insert(0, Arc::clone(&found));
    Ok(Some(found))
}

/// Why no press came back for a profile of four components.
///
/// **Three answers, because they are three different sentences to a reader** (principle 1, ADR
/// 2170). [`NoPress::Spent`] is §11.7.2's budget and a fact about the file: the same page is
/// refused it on every run. [`NoPress::Poisoned`] is a fact about this process — a thread
/// panicked while it held one of the tables a press is kept in — and reporting it as the budget
/// would tell a reader their file named too many presses when it named one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoPress {
    /// The press does not fit in what is left of the memory the page may keep (ISO 32000-2
    /// §11.7.2): [`Presses::within`]'s bound, charged at [`PRESS_BYTES`] and the profile's.
    Spent,
    /// The profile is not four components a press's grid can be sampled from.
    Unsampled,
    /// A lock over a table of presses was poisoned: a thread panicked while it held it, so what
    /// the table holds is not read.
    Poisoned,
}

impl NoPress {
    /// The reason, in the words a report carries after a colon.
    #[must_use]
    pub const fn why(self) -> &'static str {
        match self {
            Self::Spent => {
                "the presses this page names have spent the memory it may keep (§11.7.2)"
            }
            Self::Unsampled => {
                "its profile is not four components a press's grid can be sampled from"
            }
            Self::Poisoned => {
                "a thread of this process panicked while it held the table of presses, so no press \
                 is read from it — a fault of this program, not of the file"
            }
        }
    }
}

/// The guard over one of this module's tables, or [`NoPress::Poisoned`] where a thread panicked
/// holding it — never a poisoned table read as an empty or a spent one.
fn guard<T>(lock: &Mutex<T>) -> Result<MutexGuard<'_, T>, NoPress> {
    lock.lock().map_err(|_| NoPress::Poisoned)
}

/// The distinct presses one interpretation has named, and the memory it has spent on them.
///
/// # Why this is a table and not a counter
///
/// §11.4.7 gives the page one press and §11.7.2 lets a transparency group inside it name
/// another, so an interpretation can name several — and the same one twice, which must cost
/// the budget once. Holding what was named answers both questions with one structure, and it
/// makes the refusal a function of the file: a page whose presses do not fit in the bytes it may
/// keep is refused the next one on every run, independently of other pages and scheduling
/// (ADR 0417).
///
/// # Why the budget is bytes, and is the page's
///
/// Every distinct four-component profile a page names gets its press. What a page can exhaust
/// by naming profiles is memory — a press is [`PRESS_BYTES`] and the profile it keeps, at [`crate::icc::Profile::owned_bytes`] — so the
/// bound is in that unit, and it is the page's own memory bound handed in by the caller
/// ([`Presses::within`]) rather than a count chosen here: a count of presses says nothing about
/// what they hold, and the standard states none (Annex C is informative and names no number
/// for this). ADR 2532.
///
/// The lock is taken once per press a page names and never while a colour is being converted,
/// because [`super::compositing::Compositing`] carries the press itself rather than a handle
/// into here.
#[derive(Debug)]
pub struct Presses {
    /// The presses named so far, in the order they were named, and what they are charged.
    named: Mutex<Named>,
    /// The three-component routes built so far, in the order they were built.
    routes: Mutex<Vec<Arc<RgbRoute>>>,
    /// The most bytes the named presses may be charged.
    bound: usize,
}

/// The presses an interpretation has named and the bytes they are charged.
#[derive(Debug, Default)]
struct Named {
    /// By identity, because a page can name as many presses as its memory admits and every group
    /// it paints asks which one it is: a lookup that walked them would be a quadratic page.
    presses: HashMap<PressIdentity, Arc<Press>>,
    /// The sum of what each was charged at admission: [`press_charge`].
    bytes: usize,
}

/// What admitting a press sampled from `profile` is charged: the press's own grids and the
/// tables of the profile it keeps to convert *in* (§8.6.5.5; [`Press::profile`] is `Some` only
/// for a profile that is bidirectional).
fn press_charge(profile: &crate::icc::Profile) -> usize {
    let kept = if profile.is_bidirectional() {
        profile.owned_bytes()
    } else {
        0
    };
    PRESS_BYTES.saturating_add(kept)
}

/// How many [`RgbRoute`]s an interpretation keeps.
///
/// A route into a table profile is 36 000 profile evaluations — the same shape of cost as
/// sampling a press, which `examples/press_cost` puts at 17 to 46 ms against a 14 to 18 ms
/// interpretation of the page (ADR 0417) — and a group names its space at every `Do`, so
/// a page of such groups would sample the same profile once per group. A route beyond the cap
/// is built and not kept, which is slower and not wrong: this is a cache's bound, not a
/// budget, and no refusal rests on it.
///
/// Fourteen because a route holds about half a megabyte (36 000 samples of three `f32`
/// components) and fourteen is then about six megabytes, a small share of the page's bound
/// beside a press's. The value is not derived from a measurement of what pages name.
const MAX_RGB_ROUTES: usize = 14;

impl Presses {
    /// A table whose presses may be charged `bound` bytes between them.
    ///
    /// `bound` is the memory the page may keep for presses; a caller passes the page's own bound
    /// so that the presses and the page's other charges are one budget (ADR 2532).
    #[must_use]
    pub fn within(bound: usize) -> Self {
        Self {
            named: Mutex::new(Named::default()),
            routes: Mutex::new(Vec::new()),
            bound,
        }
    }

    /// The bytes the presses named so far are charged: what a caller adds to the page's other
    /// charges to ask whether the page as a whole has passed its bound.
    ///
    /// # Errors
    ///
    /// [`NoPress::Poisoned`] leaves the charge unknown, never zero.
    pub fn bytes_held(&self) -> Result<usize, NoPress> {
        Ok(guard(&self.named)?.bytes)
    }

    /// The route into and out of `space`, built once per interpretation for each space.
    ///
    /// `None` where `space` is not one [`RgbRoute::of`] answers for. Keyed on
    /// [`ColourSpace::rgb_identity`], so that two dictionaries stating one `CalRGB`, or two
    /// streams carrying one profile, are one route.
    ///
    /// # Errors
    ///
    /// [`NoPress::Poisoned`] means a thread panicked holding the route table, rather than
    /// that the colour space has no route. The caller must report its fallback.
    pub fn rgb_route(&self, space: &ColourSpace) -> Result<Option<Arc<RgbRoute>>, NoPress> {
        let Some(identity) = space.rgb_identity() else {
            return Ok(None);
        };
        if let Some(found) = guard(&self.routes)?
            .iter()
            .find(|route| route.identity == identity)
        {
            return Ok(Some(Arc::clone(found)));
        }
        // Built with no lock held, for the reason `press_for_profile` gives.
        let Some(route) = RgbRoute::of(space) else {
            return Ok(None);
        };
        let built = Arc::new(route);
        let mut routes = guard(&self.routes)?;
        if let Some(found) = routes
            .iter()
            .find(|route| route.identity == identity)
            .map(Arc::clone)
        {
            return Ok(Some(found));
        }
        if routes.len() < MAX_RGB_ROUTES {
            routes.push(Arc::clone(&built));
        }
        Ok(Some(built))
    }

    /// The press this profile describes, under this interpretation's budget.
    ///
    /// # Errors
    ///
    /// [`NoPress::Spent`] is the press not fitting in the bytes [`Presses::within`] allows for
    /// the presses already named, which `pdf_model::content::transparency` turns into §11.7.2's
    /// report. A profile of other than four components is not a press at all and answers
    /// [`NoPress::Unsampled`]; the caller asks that question first. [`NoPress::Poisoned`] is this
    /// table or the process's cache poisoned, which is said as itself (ADR 2170).
    pub fn press_for_profile(
        &self,
        profile: &crate::icc::Profile,
        rendering: Rendering,
    ) -> Result<Arc<Press>, NoPress> {
        if profile.channels() != 4 {
            return Err(NoPress::Unsampled);
        }
        let identity = PressIdentity::Profile(profile.identity(), rendering);
        let charge = {
            let named = guard(&self.named)?;
            if let Some(found) = named.presses.get(&identity) {
                return Ok(Arc::clone(found));
            }
            let charge = press_charge(profile);
            // Asked before the sampling rather than after it, so that a page naming a thousand
            // presses pays only for the admitted presses: the budget is a refusal and a
            // refusal that costs the work it refused is not one.
            if named.bytes.saturating_add(charge) > self.bound {
                return Err(NoPress::Spent);
            }
            charge
        };
        // Sampled with no lock of this table held, for ADR 0269's reason: `sample_press` is
        // 83 521 profile evaluations and a lock held across work is a lock rayon can deadlock.
        let sampled = press_for_profile(profile, rendering)?;
        let mut named = guard(&self.named)?;
        if let Some(found) = named.presses.get(&identity) {
            return Ok(Arc::clone(found));
        }
        if named.bytes.saturating_add(charge) > self.bound {
            return Err(NoPress::Spent);
        }
        named.bytes = named.bytes.saturating_add(charge);
        named.presses.insert(identity, Arc::clone(&sampled));
        Ok(sampled)
    }

    /// How many distinct presses this interpretation has named, for a test that asks.
    ///
    /// # Errors
    ///
    /// [`NoPress::Poisoned`] leaves the count unknown, never zero.
    pub fn named(&self) -> Result<usize, NoPress> {
        Ok(guard(&self.named)?.presses.len())
    }
}

/// How many samples per axis a press's grid holds.
///
/// A real press is not multilinear between the corners of its ink cube — that is what makes
/// [`CMYK_CORNERS`] an assumption rather than a measurement — so the grid has to be fine
/// enough that interpolating it is the profile.
///
/// # The side is measured, and so is what it still costs
///
/// `examples/press_census.rs --sample` builds the grid at several sides over **the 287 presses
/// the whole crawl names** and compares it against evaluating the profile directly, in levels
/// of 255:
///
/// | side | median worst gap | p90 | largest |
/// |---|---|---|---|
/// | 9 | 15.06 | — | 21.36 |
/// | **17** | **5.91** | **12.06** | **15.90** |
/// | 33, on a sample of six | 1.80–4.80 | — | — |
///
/// **No feasible side reaches half a level**, and that is a property of the profiles rather
/// than of the arithmetic: a v2 CMYK profile puts a steep sampled curve on each ink *before*
/// its own table, so a grid uniform in ink is misaligned with the shape it is sampling.
/// Sampling in linear light instead is **worse** — 22.88 at its largest against 15.90 —
/// because it moves the error into the bright end where a level of 255 is a smaller step.
///
/// So seventeen is where the curve flattens against what a finer grid costs: 33 is 1.19 million
/// evaluations and 14 MB a press, for about half the remaining gap. The figures above moved
/// with ADR 1253's compensation — the largest was 14.52 while the black point was a per-axis
/// stretch and is 15.90 under ISO 18619's single scale, because the steeper map is the harder
/// one to interpolate — which is the price of that reading, measured rather than assumed. What the residue is
/// measured *against* is the alternative it replaces — compositing a page in somebody else's
/// four components, which ADR 0251 measured at **48 to 51 of 255**. ADR 0272 records both
/// numbers and the construction that would close the rest: per-axis input curves beside the
/// grid, which is what an ICC `A2B` tag *is*, and which the backend would have to be taught.
///
/// The cost is 83 521 profile evaluations and 1.0 MB, paid once per press and only where a
/// page asks for one — `CLAUDE.md`'s launch rule, the same argument [`Press::table`] makes.
const PRESS_SIDE: usize = 17;

/// A profile's `A2B` sampled onto a [`PRESS_SIDE`] grid of its four components.
///
/// `rendering` is §11.7.5.3's second bullet — the intent and black point compensation in force
/// at the `Do` that paints the group — and it selects which of the profile's transforms the
/// grid is taken through. The whole grid is taken under one rendering because the grid *is* the
/// group's conversion out, and a conversion out is one function (ADR 1054).
fn sample_press(
    profile: &crate::icc::Profile,
    rendering: Rendering,
) -> Option<pdf_render::BlendingSpace> {
    let side = PRESS_SIDE;
    let last = side.saturating_sub(1);
    #[expect(
        clippy::cast_precision_loss,
        reason = "a grid index below 17 and the side itself, both exact in f32"
    )]
    let at = |index: usize| index as f32 / last as f32;
    let mut grid = Vec::with_capacity(side.checked_pow(4)?);
    for black in 0..side {
        for yellow in 0..side {
            for magenta in 0..side {
                for cyan in 0..side {
                    // Under the `Do`'s parameters, which is §11.7.5.3's second bullet; where
                    // nothing states any, `Rendering::compensating()` is Table 51's initial
                    // intent with §8.6.5.9's `Default` compensating, the answer ADR 0009 chose
                    // for every other colour that reaches a pixel through a profile. The
                    // conversion *in* is searched against this same grid and asks
                    // [`Press::rendering`] for the black point, so the two stay inverses
                    // whichever way it is set.
                    let colour = profile
                        .to_rgb_with(&[at(cyan), at(magenta), at(yellow), at(black)], rendering);
                    grid.push([colour.r, colour.g, colour.b]);
                }
            }
        }
    }
    pdf_render::BlendingSpace::new(side, grid.into())
}

/// The conversion out of the assumed press, for a caller that has no [`Press`] to hand.
#[must_use]
pub fn device_cmyk_blending_space() -> pdf_render::BlendingSpace {
    assumed_press().blending_space()
}

#[cfg(test)]
mod tests {
    use super::{NoPress, PRESS_BYTES, Presses, press_charge};
    use crate::icc::{A2b, Profile, Rendering};

    /// How many distinct conversions out of a group the standard defines for **one** profile.
    ///
    /// §11.7.5.3's second bullet makes the conversion out of a group a function of the graphics
    /// state at the `Do`:
    ///
    /// > the rendering intent used shall be the current rendering intent in effect at the time the
    /// > Do operator is applied to the group
    ///
    /// Table 69 names four rendering intents, §8.6.5.9's `/UseBlackPtComp` is a second parameter
    /// of the same conversion, and that clause pins one of the eight pairs:
    ///
    /// > If the current render intent of an object is AbsColorimetric then the value of
    /// > UseBlackPtComp shall be treated as OFF .
    ///
    /// Four intents times two settings less that pair is seven, and
    /// [`super::PressIdentity::Profile`] is keyed on exactly that pair.
    const RENDERINGS_OF_ONE_PROFILE: usize = 7;

    /// The seven renderings of one profile, each a distinct press.
    fn renderings() -> Vec<Rendering> {
        let mut all = Vec::new();
        for transform in [
            A2b::Perceptual,
            A2b::Colorimetric,
            A2b::Absolute,
            A2b::Saturation,
        ] {
            for black_point in [false, true] {
                let rendering = Rendering::new(transform, black_point);
                if !all.contains(&rendering) {
                    all.push(rendering);
                }
            }
        }
        all
    }

    /// Three profiles whose bytes, and so whose identities, differ.
    fn three_profiles() -> Vec<Profile> {
        [
            crate::icc::fixtures::two_way_cmyk_profile(),
            crate::icc::fixtures::ink_limited_cmyk_profile(),
            crate::icc::fixtures::one_way_cmyk_profile(),
        ]
        .iter()
        .map(|bytes| Profile::parse(bytes).expect("the fixture parses"))
        .collect()
    }

    /// The clause's seven renderings of one profile are seven presses, and the fixtures make
    /// twenty-one.
    #[test]
    fn the_renderings_of_a_profile_are_the_standards_seven() {
        assert_eq!(renderings().len(), RENDERINGS_OF_ONE_PROFILE);
        assert_eq!(three_profiles().len() * renderings().len(), 21);
    }

    /// A page that names twenty-one presses is given every one of them, and is charged for each.
    #[test]
    fn a_page_naming_twenty_one_presses_gets_every_one() {
        let profiles = three_profiles();
        let wanted: usize = profiles
            .iter()
            .map(|profile| press_charge(profile) * renderings().len())
            .sum();
        let presses = Presses::within(wanted);
        for profile in &profiles {
            for rendering in renderings() {
                assert!(presses.press_for_profile(profile, rendering).is_ok());
            }
        }
        assert_eq!(presses.named(), Ok(21));
        assert_eq!(presses.bytes_held(), Ok(wanted));
        assert!(wanted > 21 * PRESS_BYTES - 1);
    }

    /// What the process holds resident, in bytes, from `/proc/self/statm`.
    fn resident_bytes() -> usize {
        let statm = std::fs::read_to_string("/proc/self/statm").expect("Linux");
        let pages: usize = statm
            .split_whitespace()
            .nth(1)
            .and_then(|field| field.parse().ok())
            .expect("statm's second field is the resident set in pages");
        pages.saturating_mul(4096)
    }

    /// A two-way profile with `A2B0`, `A2B1` and `B2A1` of 33 grid points a side, which the
    /// variant keeps from being another profile's bytes.
    fn large_two_way_profile(variant: u16) -> Profile {
        use crate::icc::fixtures::{complement_clut, mft2_grid_tag, mft2_tag, profile_of};
        let side = 33_usize;
        let mut samples = vec![0x4000u16; side.pow(4).saturating_mul(3)];
        samples[0] = variant;
        Profile::parse(&profile_of(
            *b"CMYK",
            *b"XYZ ",
            2,
            &[
                (*b"A2B1", mft2_grid_tag(4, 3, side, &samples)),
                (*b"A2B0", mft2_grid_tag(4, 3, side, &samples)),
                (*b"B2A1", mft2_tag(3, 4, &complement_clut())),
            ],
        ))
        .expect("the fixture parses")
    }

    /// What a press charges is at least what it holds resident, for the profile with the most to
    /// hold: two 33-point `A2B` tables and a `B2A`, drawn under the perceptual intent that parses
    /// the second (ISO 32000-2 §8.6.5.5; ADR 2532).
    ///
    /// Each profile is dropped once its press is built, so the growth is the press's own clone.
    /// Run alone, in release, so that no other test's allocations are in the figure:
    /// `cargo test --release -p pdf-colour --lib -- --ignored --test-threads 1 large_two_way`.
    #[test]
    // not a gate: a probe of resident memory, which a process shared with the other tests cannot
    // state; it holds the arithmetic `a_two_way_profile_is_charged_at_least_what_the_perceptual_table_adds_to_its_clone`
    // holds in the gate against a table's own sizes (ADR 2532)
    #[ignore = "a probe of resident memory: run alone, in release, by hand"]
    #[expect(
        clippy::print_stdout,
        reason = "a probe whose entire output is a measurement"
    )]
    fn large_two_way_profiles_are_charged_at_least_their_resident_size() {
        const NAMED: u16 = 24;
        let presses = Presses::within(usize::MAX);
        let rendering = Rendering::new(A2b::Perceptual, false);
        let before = resident_bytes();
        let mut kept = Vec::new();
        for variant in 0..NAMED {
            let profile = large_two_way_profile(variant);
            kept.push(
                presses
                    .press_for_profile(&profile, rendering)
                    .expect("admitted"),
            );
        }
        let grown = resident_bytes().saturating_sub(before);
        let charged = presses.bytes_held().expect("not poisoned");
        println!(
            "{NAMED} presses: {charged} bytes charged, {grown} bytes resident growth \
             ({} charged per press)",
            charged / usize::from(NAMED)
        );
        assert!(
            charged >= grown,
            "{charged} bytes charged against {grown} bytes resident"
        );
        drop(kept);
    }

    /// At the budget the press that fits is admitted and the next is refused by name, and a
    /// press already named stays reachable for no further charge.
    #[test]
    fn the_press_that_does_not_fit_is_refused_by_name_and_the_named_stay_reachable() {
        let profiles = three_profiles();
        let first = profiles.first().expect("a profile");
        let second = profiles.get(1).expect("a second profile");
        let exactly = press_charge(first) + press_charge(second);
        let presses = Presses::within(exactly);
        let rendering = Rendering::compensating();
        assert!(presses.press_for_profile(first, rendering).is_ok());
        assert!(presses.press_for_profile(second, rendering).is_ok());
        assert_eq!(presses.bytes_held(), Ok(exactly));
        let refused = presses.press_for_profile(first, Rendering::without_black_point());
        assert_eq!(refused.err(), Some(NoPress::Spent));
        assert!(presses.press_for_profile(first, rendering).is_ok());
        assert_eq!(presses.named(), Ok(2));
        assert_eq!(presses.bytes_held(), Ok(exactly));
    }

    /// One byte under the two presses' charge admits the first and refuses the second.
    #[test]
    fn a_bound_one_byte_short_refuses_the_press_that_would_cross_it() {
        let profiles = three_profiles();
        let first = profiles.first().expect("a profile");
        let second = profiles.get(1).expect("a second profile");
        let presses = Presses::within(press_charge(first) + press_charge(second) - 1);
        let rendering = Rendering::compensating();
        assert!(presses.press_for_profile(first, rendering).is_ok());
        assert_eq!(
            presses.press_for_profile(second, rendering).err(),
            Some(NoPress::Spent)
        );
        assert_eq!(presses.named(), Ok(1));
    }

    /// A bound of nothing admits no press at all, and says so as the budget.
    #[test]
    fn an_empty_budget_admits_nothing() {
        let profiles = three_profiles();
        let presses = Presses::within(0);
        assert_eq!(
            presses
                .press_for_profile(
                    profiles.first().expect("a profile"),
                    Rendering::compensating()
                )
                .err(),
            Some(NoPress::Spent)
        );
        assert_eq!(presses.bytes_held(), Ok(0));
    }

    /// A table of presses a panicking thread poisoned answers [`NoPress::Poisoned`], and never
    /// [`NoPress::Spent`]: the budget is §11.7.2's and a fact about the file, the poison a fact
    /// about this process (ADR 2170). Fails with `guard`'s error mapped to `Spent`.
    #[test]
    fn a_poisoned_table_of_presses_is_said_as_poisoned_and_not_as_a_spent_budget() {
        let profile = Profile::parse(&crate::icc::fixtures::two_way_cmyk_profile())
            .expect("the fixture parses");
        let presses = Presses::within(usize::MAX);
        let panicked = std::thread::scope(|scope| {
            scope
                .spawn(|| {
                    let _held = presses.named.lock();
                    panic!("a thread panics while it holds the table");
                })
                .join()
        });
        assert!(panicked.is_err(), "the thread panicked holding the lock");
        assert!(presses.named.is_poisoned());
        assert_eq!(
            presses
                .press_for_profile(&profile, Rendering::compensating())
                .err(),
            Some(NoPress::Poisoned)
        );
        assert!(NoPress::Poisoned.why().contains("not of the file"));
    }

    /// A poisoned count is unknown, not an empty interpretation.
    #[test]
    fn a_poisoned_named_counter_is_not_zero() {
        let presses = Presses::within(usize::MAX);
        assert!(
            std::thread::scope(|scope| scope
                .spawn(|| {
                    let _held = presses.named.lock();
                    panic!("poison the count");
                })
                .join())
            .is_err()
        );
        assert_eq!(presses.named(), Err(NoPress::Poisoned));
    }

    /// The same valid RGB space has a route before poisoning and an explicit fault after it.
    #[test]
    fn a_poisoned_rgb_route_is_not_an_absent_route() {
        let presses = Presses::within(usize::MAX);
        let space = crate::colour::ColourSpace::CalRgb {
            white: [0.9505, 1.0, 1.0890],
            black: [0.0; 3],
            gamma: [1.0; 3],
            matrix: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
        };
        assert!(presses.rgb_route(&space).expect("whole table").is_some());
        assert!(
            std::thread::scope(|scope| scope
                .spawn(|| {
                    let _held = presses.routes.lock();
                    panic!("poison the route table");
                })
                .join())
            .is_err()
        );
        assert_eq!(presses.rgb_route(&space).err(), Some(NoPress::Poisoned));
    }

    /// The process-wide cache is poisoned in a child process so neighbouring tests keep a
    /// whole cache. This exercises the public counter, not a substitute lock.
    #[test]
    fn a_poisoned_process_cache_counter_is_not_zero() {
        const CHILD: &str = "PDF_COLOUR_POISON_CACHE_CHILD";
        if std::env::var_os(CHILD).is_none() {
            let status =
                std::process::Command::new(std::env::current_exe().expect("test executable"))
                    .args([
                        "--exact",
                        "colour::press::tests::a_poisoned_process_cache_counter_is_not_zero",
                    ])
                    .env(CHILD, "1")
                    .status()
                    .expect("child test starts");
            assert!(status.success());
            return;
        }
        assert!(super::presses_cached().is_ok());
        assert!(
            std::thread::spawn(|| {
                let _held = super::SAMPLED.lock();
                panic!("poison the process cache");
            })
            .join()
            .is_err()
        );
        assert_eq!(super::presses_cached(), Err(NoPress::Poisoned));
    }

    /// The unpoisoned table answers a press, so the test above is about the poison alone.
    #[test]
    fn the_same_profile_on_a_whole_table_is_a_press() {
        let profile = Profile::parse(&crate::icc::fixtures::two_way_cmyk_profile())
            .expect("the fixture parses");
        let presses = Presses::within(usize::MAX);
        assert!(
            presses
                .press_for_profile(&profile, Rendering::compensating())
                .is_ok()
        );
        assert_eq!(presses.named(), Ok(1));
    }
}
