//! §11.4.7 and §11.7.2: the four-component press a page or group composites in, and the table of presses one interpretation names.

use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

use super::ColourSpace;
use super::device_cmyk::{CMYK_CORNERS, channel};
use super::rgb_route::RgbRoute;
use crate::icc::Rendering;

/// Which press a colour's four components belong to, for the things keyed on one.
///
/// A press is recognised by the profile it was sampled from **and by the parameters it was
/// sampled under**, never by where it is held: [`SAMPLED`] evicts, so the same profile can be
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
    /// ADR 0263's assumed process inks — [`CMYK_CORNERS`] — which no document names.
    ///
    /// No rendering beside it: the grid is [`CMYK_CORNERS`], a compile-time constant with no
    /// profile behind it, so there is no transform for an intent to select.
    Assumed,
    /// The profile of this [`crate::icc::Profile::identity`], sampled under this rendering.
    Profile(u128, Rendering),
    /// `DeviceCMYK`'s own four components inside a `/Luminosity` mask group whose content blends
    /// — [`device_ink_press`]. Its conversion in is §10.4.2.4's with the nominal black generation
    /// and undercolour removal, and its `Y` is §11.5.3's EXAMPLE 2; it converts nothing out.
    DeviceInk,
}

/// How many distinct conversions out of a group the standard defines for **one** profile.
///
/// Trap 38's question, answered: the standard does state a number this budget has to clear.
/// §11.7.5.3's second bullet makes the conversion out of a group a function of the graphics
/// state at the `Do`:
///
/// > the rendering intent used shall be the current rendering intent in effect at the time the
/// > Do operator is applied to the group
///
/// Two entries of that state select it. Table 69 names four rendering intents, §8.6.5.9's
/// `/UseBlackPtComp` is a second parameter of the same conversion, and that clause pins one of
/// the eight pairs:
///
/// > If the current render intent of an object is AbsColorimetric then the value of
/// > UseBlackPtComp shall be treated as OFF .
///
/// Four intents times two settings less the pair the clause removes is **seven**, and
/// [`PressIdentity::Profile`] is keyed on exactly that pair.
///
/// So a page that embeds one profile and paints it under every intent the standard defines
/// names seven presses, and a budget below seven would refuse a conformant page that named a
/// single profile. Nothing in the standard bounds how many *profiles* a page may name —
/// Annex C is informative and states no number for this at all.
const RENDERINGS_OF_ONE_PROFILE: usize = 7;

/// How many *distinct* presses one interpretation may name.
///
/// A press costs its grid — `PRESS_SIDE⁴` device colours — plus, if a page converts a colour
/// *into* it, `INK_TABLE_SIDE`³ separations. At the sizes below that is 1.0 MB and 78 KB
/// apiece.
///
/// **The number is the standard's floor with a profile's worth of room above it**, which is
/// what trap 38 asks a bound to say by name. [`RENDERINGS_OF_ONE_PROFILE`] is what one profile
/// can be, so no conformant page naming one profile can meet this refusal; the second multiple
/// is margin over everything measured. What was measured is
/// `pdf-model --example press_depth`: over the 974-document corpus **no page names a press at
/// all**, and over a 5 977-document even sample of the crawl — 13 188 pages — the deepest page
/// names **one**. So the budget is two orders of magnitude above the population and one
/// profile above the clause, and an interpretation that spent the whole of it would hold
/// 15 MB of press. ADR 1254.
///
/// **It is a budget on the interpretation rather than on the process, and that is the thing
/// about it worth saying out loud** (ADR 0417). A `static` table filled from the front and never
/// evicted would refuse the ninth distinct press a *process* met, and which document that fell
/// on would be decided by the order the scheduler ran the eight before it in. Every other
/// budget in this tree — `MAX_LIST_BYTES`, `MAX_FORM_DEPTH`, `MAX_STATE_DEPTH` — is spent by the
/// document that reaches it, and this one is too: a page naming more than this many presses
/// is refused the next one on every run and on every machine, and a page naming one is never
/// refused whatever else the process has open. ADR 0416 is the diagnosis and `doc/todo/49`'s
/// third-bound section the three roads.
///
/// What is *not* per interpretation is the sampling, which is [`SAMPLED`]: a cache changes how
/// fast an answer is reached and never what it is, so it may be shared where a budget may not.
pub const MAX_PRESSES: usize = RENDERINGS_OF_ONE_PROFILE * 2;

/// How many sampled presses this process keeps, so that a second page need not sample again.
///
/// A **cache** bound rather than a budget, and the difference is the whole of ADR 0417: past
/// this the least recently used press is dropped and a document naming it again pays for the
/// sampling a second time, which is slower and is the same picture. So it may be sized against
/// what the population measures — the web names 28 distinct presses over 65 703 documents
/// (`examples/press_census`) — where [`MAX_PRESSES`] may not.
///
/// Eight is what this process has always spent on presses, so a run holding more is holding
/// them in the interpretations that named them. It is deliberately **not** tied to
/// [`MAX_PRESSES`]: one is what a page may name and the other what a process keeps warm, and
/// the web names 28 distinct presses over 65 703 documents either way.
const MAX_CACHED_PRESSES: usize = 8;

/// The presses this process has sampled, most recently used first.
///
/// The lock is taken once per press an interpretation names — never per colour, which is what
/// [`Compositing`] carrying the press itself buys — and it is **never** held across
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

/// How many times this process has built a press out of a profile. See [`SAMPLINGS`].
#[must_use]
pub fn presses_sampled() -> usize {
    SAMPLINGS.load(std::sync::atomic::Ordering::Relaxed)
}

/// How many sampled presses this process still holds, at most [`MAX_CACHED_PRESSES`].
#[must_use]
pub fn presses_cached() -> usize {
    SAMPLED.lock().map_or(0, |sampled| sampled.len())
}

/// [`PressIdentity::Assumed`]'s press, built on first use like any other.
///
/// Outside [`SAMPLED`] and outside [`MAX_PRESSES`], because it is a pure function of
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
/// - **In**, a colour of another space goes through [`super::ink_search::rgb_to_cmyk`] — §10.4.2.4 with
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

/// The press this tree assumes when a document says nothing: [`CMYK_CORNERS`].
#[must_use]
pub fn assumed_press() -> Arc<Press> {
    Arc::clone(ASSUMED.get_or_init(|| Arc::new(Press::assumed())))
}

/// The four components a page composites in, in both directions.
///
/// §11.4.7 gives a page a blending colour space and §11.7.2 requires colours to be converted
/// *into* it, so a press is two conversions that have to be inverses of each other — ADR 0263
/// is where that became the rule, and this is that rule with the press no longer assumed.
/// [`Press::space`] is the conversion out, sampled on a grid because that is what a backend
/// can be handed; the conversion in is [`Press::profile`]'s own `B2A` where the profile
/// carries one (ADR 0796), and otherwise [`Press::table`], searched against the same grid.
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
    /// §11.7.5.3's second bullet fixes them at the group's `Do`; [`xyz_to_ink`] asks this
    /// rather than assuming compensation, because the two directions are inverses of each
    /// other and an inverse taken under other parameters is not one.
    pub(super) rendering: Rendering,
    /// [`search_ink`] over a grid of sRGB, built on first use. See [`Press::table`].
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
            // any profile is selected and this is what [`xyz_to_ink`]'s search reads.
            rendering: Rendering::compensating(),
            table: OnceLock::new(),
            luminance: OnceLock::new(),
        }
    }

    /// Whether the conversion into this press is the profile's own `B2A` table.
    ///
    /// `false` for the assumed inks and for a profile without one, whose conversion in is a
    /// right inverse of the conversion out ([`super::ink_search::rgb_to_ink`]).
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
    /// interpolating the assumed press's is [`super::device_cmyk::cmyk`] to the last bit, which is what keeps the
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
    /// very grid points [`sample_press`] already samples for the conversion out**, which is
    /// what makes the fidelity argument the same one [`PRESS_SIDE`] carries rather than a new
    /// one: the two grids agree point for point on where they are evaluated, and a
    /// luminosity read between them departs by the profile's own curvature exactly as the
    /// device colour does. Interpolated over sixteen corners by `pdf_render::Luminance`.
    ///
    /// **Without §8.6.5.9's black point compensation**, as [`RgbRoute::luminance`] is one
    /// axis down: the clause asks for the colour's XYZ, and the compensation is a step
    /// toward a destination rather than part of it.
    ///
    /// # What it costs, and why it is behind a lock rather than in [`sample_press`]
    ///
    /// [`PRESS_SIDE`]⁴ profile evaluations — 83 521 of them — and 334 KB, which is a third of
    /// the grid beside it. Almost every press a document names is a *page*'s or an output
    /// intent's and never carries a mask, so paying this inside [`sample_press`] would put it
    /// on the launch path of every four-component page for a number no such page reads. Built
    /// on first use for the same reason [`Press::table`] is, and once per press rather than
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
/// so the profile's own table is the standard's answer and not a choice. **This paragraph used
/// to argue the opposite** — that a `B2A` was not read "even where §8.6.5.5 requires the file
/// to carry one", because two separately-built maps on one page would be two colour models.
/// They are two maps, and the clause makes them the file's two: what an opaque mark loses on
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
    /// This interpretation has already named [`MAX_PRESSES`] distinct presses.
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
                "it is the ninth distinct press this page names and eight is the budget (§11.7.2)"
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

/// The distinct presses one interpretation has named, and the budget it spends on them.
///
/// # Why this is a table and not a counter
///
/// §11.4.7 gives the page one press and §11.7.2 lets a transparency group inside it name
/// another, so an interpretation can name several — and the same one twice, which must cost
/// the budget once. Holding what was named answers both questions with one structure, and it
/// is what makes the refusal a function of the file: the ninth *distinct* press of one page is
/// refused on every run and on every machine, where until ADR 0417 the ninth press of the
/// *process* was, and which page that fell on depended on the scheduler.
///
/// The lock is taken once per press a page names — a handful of times at most — and never
/// while a colour is being converted, because [`super::compositing::Compositing`] carries the press itself rather
/// than a handle into here.
#[derive(Debug, Default)]
pub struct Presses {
    /// The presses named so far, in the order they were named.
    named: Mutex<Vec<Arc<Press>>>,
    /// The three-component routes built so far, in the order they were built.
    routes: Mutex<Vec<Arc<RgbRoute>>>,
}

/// How many [`RgbRoute`]s an interpretation keeps.
///
/// A route into a table profile is 36 000 profile evaluations — the same shape of cost as
/// sampling a press, which `examples/press_cost` puts at 17 to 46 ms against a 14 to 18 ms
/// interpretation of the page (ADR 0417) — and a group names its space at every `Do`, so
/// a page of such groups would sample the same profile once per group. The cap is the
/// press's; a route beyond it is built and not kept, which is slower and not wrong.
const MAX_RGB_ROUTES: usize = MAX_PRESSES;

impl Presses {
    /// The route into and out of `space`, built once per interpretation for each space.
    ///
    /// `None` where `space` is not one [`RgbRoute::of`] answers for. Keyed on
    /// [`ColourSpace::rgb_identity`], so that two dictionaries stating one `CalRGB`, or two
    /// streams carrying one profile, are one route.
    #[must_use]
    pub fn rgb_route(&self, space: &ColourSpace) -> Option<Arc<RgbRoute>> {
        let identity = space.rgb_identity()?;
        if let Some(found) = self
            .routes
            .lock()
            .ok()?
            .iter()
            .find(|route| route.identity == identity)
        {
            return Some(Arc::clone(found));
        }
        // Built with no lock held, for the reason `press_for_profile` gives.
        let built = Arc::new(RgbRoute::of(space)?);
        let mut routes = self.routes.lock().ok()?;
        if let Some(found) = routes
            .iter()
            .find(|route| route.identity == identity)
            .map(Arc::clone)
        {
            return Some(found);
        }
        if routes.len() < MAX_RGB_ROUTES {
            routes.push(Arc::clone(&built));
        }
        Some(built)
    }

    /// The press this profile describes, under this interpretation's budget.
    ///
    /// # Errors
    ///
    /// [`NoPress::Spent`] is [`MAX_PRESSES`] distinct presses already named, which
    /// `pdf_model::content::transparency` turns into §11.7.2's report. A profile of other than four
    /// components is not a press at all and answers [`NoPress::Unsampled`]; the caller asks that
    /// question first. [`NoPress::Poisoned`] is this table or the process's cache poisoned, which is
    /// said as itself (ADR 2170).
    pub fn press_for_profile(
        &self,
        profile: &crate::icc::Profile,
        rendering: Rendering,
    ) -> Result<Arc<Press>, NoPress> {
        let identity = PressIdentity::Profile(profile.identity(), rendering);
        {
            let named = guard(&self.named)?;
            if let Some(found) = named.iter().find(|press| press.identity == identity) {
                return Ok(Arc::clone(found));
            }
            // Asked before the sampling rather than after it, so that a page naming a hundred
            // presses pays for eight rather than for a hundred: the budget is a refusal and a
            // refusal that costs the work it refused is not one.
            if named.len() >= MAX_PRESSES {
                return Err(NoPress::Spent);
            }
        }
        // Sampled with no lock of this table held, for ADR 0269's reason: `sample_press` is
        // 83 521 profile evaluations and a lock held across work is a lock rayon can deadlock.
        let sampled = press_for_profile(profile, rendering)?;
        let mut named = guard(&self.named)?;
        if let Some(found) = named
            .iter()
            .find(|press| press.identity == identity)
            .map(Arc::clone)
        {
            return Ok(found);
        }
        if named.len() >= MAX_PRESSES {
            return Err(NoPress::Spent);
        }
        named.push(Arc::clone(&sampled));
        Ok(sampled)
    }

    /// How many distinct presses this interpretation has named, for a test that asks.
    #[must_use]
    pub fn named(&self) -> usize {
        self.named.lock().map_or(0, |named| named.len())
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
    use super::{NoPress, Presses};
    use crate::icc::Rendering;

    /// A table of presses a panicking thread poisoned answers [`NoPress::Poisoned`], and never
    /// [`NoPress::Spent`]: the budget is §11.7.2's and a fact about the file, the poison a fact
    /// about this process (ADR 2170). Fails with `guard`'s error mapped to `Spent`.
    #[test]
    fn a_poisoned_table_of_presses_is_said_as_poisoned_and_not_as_a_spent_budget() {
        let profile = crate::icc::Profile::parse(&crate::icc::fixtures::two_way_cmyk_profile())
            .expect("the fixture parses");
        let presses = Presses::default();
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

    /// The unpoisoned table answers a press, so the test above is about the poison alone.
    #[test]
    fn the_same_profile_on_a_whole_table_is_a_press() {
        let profile = crate::icc::Profile::parse(&crate::icc::fixtures::two_way_cmyk_profile())
            .expect("the fixture parses");
        let presses = Presses::default();
        assert!(
            presses
                .press_for_profile(&profile, Rendering::compensating())
                .is_ok()
        );
        assert_eq!(presses.named(), 1);
    }
}
