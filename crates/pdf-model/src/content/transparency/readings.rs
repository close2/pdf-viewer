//! §11.6.4.3: the alpha source parameter `/AIS`. The readings the content being run painted under,
//! the seals that make an element state its shape under the other reading, and the scope that
//! records both (ADR 2080).

use pdf_render::Command;

use crate::image::ShapeMasks;

use crate::content::{GraphicsState, Interpreter};

use super::shape::{element_alpha_is_shape, stated_shape};

/// What §11.6.4.3's alpha source parameter makes of a soft mask and the two alpha constants.
///
/// Table 57's `/AIS` is one boolean and it decides which of §11.3.7.2's two products a value
/// enters. §11.6.4.3's NOTE 1 states it of the mask:
///
/// > This is a boolean flag, set with the AIS ("alpha is shape") entry in a graphics state
/// > parameter dictionary (8.4.5, "Graphics state parameter dictionaries"): true if the soft
/// > mask contains shape values, false for opacity.
///
/// and §11.6.4.4 states it again of the constants:
///
/// > As described previously for the soft mask, the AIS ('alpha is shape') entry in a
/// > graphics state parameter dictionary shall determine whether the alpha constants are
/// > interpreted as shape values ( true ) or opacity values ( false ).
///
/// Alpha is their product either way (§11.3.7.1), so the flag changes no pixel outside a
/// knockout group — and inside one it changes which quantity §11.4.6's weighted average is
/// taken with. The two readings therefore build a knockout element's shape differently, and
/// the difference is the whole of what this type is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::content) enum AlphaSource {
    /// `/AIS false`, Table 57's default: the mask and both constants are opacity.
    Opacity,
    /// `/AIS true`: they are shape, so §11.3.7.2's source opacity is 1.0 everywhere.
    Shape,
}

/// Which readings of §11.6.4.3's alpha source parameter the content being run painted under.
///
/// `/AIS` is a graphics state parameter, so one content stream may paint some of its marks
/// under each reading. [`Readings`] keeps a scope's record to one of them by sealing what was
/// painted under the other, so [`Mixed`] is left for the element whose shape cannot be stated
/// at all. What follows about a *group* is [`Self::settled_over`]'s rather than this type's:
/// the flag reinterprets two inputs and nothing else, so a group stating neither is described
/// by both readings.
///
/// [`Mixed`]: Self::Mixed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::content) enum AlphaSourcesSeen {
    /// Only §11.6.4.3's `false`.
    Opacity,
    /// Only its `true`.
    Shape,
    /// Both, with an element under the other reading whose shape could not be stated beside
    /// it (`seal`), so no single reading describes what the content painted.
    Mixed,
}

impl AlphaSourcesSeen {
    /// What a graphics state whose `/AIS` is `alpha_is_shape` paints under.
    pub(in crate::content) fn of(alpha_is_shape: bool) -> Self {
        if alpha_is_shape {
            Self::Shape
        } else {
            Self::Opacity
        }
    }

    /// The one reading that describes all of it, or `None` where there is no such reading.
    fn settled(self) -> Option<AlphaSource> {
        match self {
            Self::Opacity => Some(AlphaSource::Opacity),
            Self::Shape => Some(AlphaSource::Shape),
            Self::Mixed => None,
        }
    }

    /// The reading that describes `commands`, which [`Self::Mixed`] does not by itself
    /// rule out (ISO 32000-2 §11.6.4.2, §11.6.4.3, §11.6.4.4).
    ///
    /// §11.6.4.3 and §11.6.4.4 give the flag exactly two things to reinterpret — the soft
    /// mask, and the two alpha constants — and §11.6.4.2 fixes everything else:
    ///
    /// > All elementary objects shall have an intrinsic opacity q j of 1.0 everywhere. Any
    /// > desired opacity less than 1.0 shall be applied by means of an opacity mask or
    /// > constant
    ///
    /// So where nothing in `commands` states either of those two, the flag reinterprets
    /// nothing: every element's source shape is its object shape and its source opacity is
    /// 1.0 under **both** readings, and [`Self::Mixed`] content is described by both rather
    /// than by neither. [`flag_reinterprets_nothing`] is that question, element by element
    /// and at every depth.
    ///
    /// [`AlphaSource::Opacity`] is the answer it then gives because Table 57 makes it the
    /// default and because it is the cheaper of two equal display lists: under it an opaque
    /// element's shape is the coverage a rasteriser already draws it with
    /// ([`super::shape::element_shape_is_coverage`]), where the other reading states the same shape as a
    /// second command.
    pub(in crate::content) fn settled_over(self, commands: &[Command]) -> Option<AlphaSource> {
        self.settled().or_else(|| {
            commands
                .iter()
                .all(flag_reinterprets_nothing)
                .then_some(AlphaSource::Opacity)
        })
    }
}

/// The record of §11.6.4.3's readings for the scope being run — which reading its elements
/// were painted under, kept to one by sealing (ISO 32000-2 §11.6.4.3, §11.6.4.4, §11.4.6).
///
/// `/AIS` is a graphics state parameter, so one scope may paint some elements under each
/// reading, and §11.4.6 then needs each element's shape built from the quantity *its* reading
/// makes shape. The record carries that without a field on every command: where a scope turns
/// from one reading to the other, every element painted so far states its shape beside itself
/// ([`seal`]) and is described by both readings from then on, so the record names one reading
/// for everything still unsealed. The statement itself decides nothing — only an element the
/// flag reinterprets, painted under the reading in force, reaches the record — which is what
/// keeps a `q`…`Q` that states the entry and paints nothing from costing a seal (ADR 1319).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::content) struct Readings {
    /// Where the scope's unsealed commands begin in the display list: the scope's first command,
    /// until a seal moves it to the boundary it sealed up to.
    start: usize,
    /// The reading every unsealed element before [`Self::since`] that the flag reinterprets
    /// was painted under: `None` while there is none, [`AlphaSourcesSeen::Mixed`] only where
    /// an element could not be sealed.
    painted: Option<AlphaSourcesSeen>,
    /// The reading in force, which the elements painted since [`Self::since`] are under.
    in_force: bool,
    /// The list's length when the reading in force took force, or when the record was last
    /// brought up to date.
    since: usize,
    /// Whether this scope, or one closed inside it, sealed an element — which the finished
    /// list's walk ([`unseal_list`]) is asked for on no other page.
    sealed: bool,
    /// Whether the scope's elements may become a knockout group's: a text object under §9.3.8's
    /// `Tk`, or a fill and stroke §11.7.4.4 may make one object. A tiling painted as one of them
    /// has to arrive as *one* element (`Interpreter::wrap_tiling`).
    knockout_elements: bool,
}

impl Readings {
    /// Whether a piece of a divided read left this record as one the join carries (ADR 1880):
    /// one reading in force throughout, nothing sealed, and whatever was painted under it.
    ///
    /// With one reading in force the record's only outcome is whether anything the flag
    /// reinterprets was painted — which reading it was painted under is the one in force — so the
    /// pieces' records join by that one fact, and the page's last settling walks every command
    /// after the first piece's last one again.
    pub(in crate::content) fn carried_by_join(self, began: Self) -> bool {
        self.in_force == began.in_force
            && !self.sealed
            && self.start == began.start
            && self.knockout_elements == began.knockout_elements
            && self
                .painted
                .is_none_or(|painted| painted == AlphaSourcesSeen::of(began.in_force))
    }

    /// Takes in what a later piece of a divided read painted (ADR 1880).
    pub(in crate::content) fn absorb_piece(&mut self, piece: Self) {
        if self.painted.is_none() {
            self.painted = piece.painted;
        }
    }

    /// A scope beginning at command `at` under the reading `alpha_is_shape`.
    pub(in crate::content) fn open(alpha_is_shape: bool, at: usize) -> Self {
        Self {
            start: at,
            painted: None,
            in_force: alpha_is_shape,
            since: at,
            sealed: false,
            knockout_elements: false,
        }
    }

    /// Whether the scope's elements may become a knockout group's.
    pub(in crate::content) fn knockout_elements(self) -> bool {
        self.knockout_elements
    }

    /// What the scope's elements were painted under, once [`Interpreter::settle_readings_until`]
    /// has brought the record to its end: the reading in force where nothing the flag
    /// reinterprets was painted, since then any reading describes the scope.
    pub(super) fn seen(self) -> AlphaSourcesSeen {
        self.painted.unwrap_or(AlphaSourcesSeen::of(self.in_force))
    }

    /// Whether a seal was made in this scope or one closed inside it.
    pub(in crate::content) fn sealed(self) -> bool {
        self.sealed
    }
}

/// Makes every element of `commands` that §11.6.4.3's flag reinterprets state its shape under
/// `alpha` beside itself, and answers what it did (ISO 32000-2 §11.6.4.2).
///
/// A sealed element is a [`Command::Shaped`] whose object is the element unchanged, so a
/// knockout group built from it reads the stated shape rather than re-deriving one under
/// whatever reading the group asks with — the one question the record could not otherwise
/// answer for an element painted under the other reading. Outside a knockout group the shape is
/// unused, and [`unseal_list`] takes it off before any backend sees the list, which is what
/// keeps `Command::Shaped`'s guarantee.
///
/// A non-isolated group is sealed like any other element: §11.4.6's NOTE 6 decides what such an
/// element composites with, and the constructions that ask look through a stated pair to its
/// object (`is_a_non_isolated_group`), the way the own-backdrop construction already states
/// one (ADR 1256).
fn seal(commands: &mut [Command], alpha: AlphaSource, shape_masks: &ShapeMasks) -> Sealed {
    let mut sealed = Sealed {
        every: true,
        any: false,
    };
    for command in commands {
        if flag_reinterprets_nothing(command) {
            continue;
        }
        match stated_shape(command, alpha, shape_masks) {
            Some(shape) => {
                *command = Command::Shaped {
                    object: Box::new(command.clone()),
                    shape: Box::new(shape),
                };
                sealed.any = true;
            }
            None => sealed.every = false,
        }
    }
    sealed
}

/// What [`seal`] did to a list of elements.
#[derive(Debug, Clone, Copy)]
struct Sealed {
    /// Whether every element that needed a seal got one.
    pub(super) every: bool,
    /// Whether any element was sealed, which is what the finished list's walk is owed for.
    pub(super) any: bool,
}

/// Takes the seals off a finished list: every [`Command::Shaped`] that is not a direct element
/// of a knockout group becomes its object again (ADR 1319).
///
/// `Command::Shaped` is guaranteed to appear only as a direct element of a knockout group,
/// because a backend draws its two stages there and nowhere else. A seal is made before anybody
/// knows whether the element will be one — a group refused for another reason, or content no
/// knockout group encloses, carries it out — so the guarantee is met here, once, on the list a
/// backend receives. A stated pair a knockout construction made is already where the guarantee
/// puts it and is kept.
pub(in crate::content) fn unseal_list(list: &mut pdf_render::DisplayList) {
    let mut commands = list.split_off_commands(0);
    unseal(&mut commands, false);
    for command in commands {
        list.push(command);
    }
}

/// [`unseal_list`]'s walk over one list of elements, `in_knockout` saying whether they are the
/// direct elements of a knockout group.
pub(super) fn unseal(commands: &mut [Command], in_knockout: bool) {
    for command in commands {
        match command {
            Command::Shaped { object, shape } => {
                unseal(std::slice::from_mut(&mut **object), false);
                unseal(std::slice::from_mut(&mut **shape), false);
                if !in_knockout {
                    *command = (**object).clone();
                }
            }
            Command::Group {
                commands,
                knockout,
                blending,
                ..
            } => {
                unseal(commands, *knockout);
                if let Some(pdf_render::GroupBlending::FourComponents { black, .. }) =
                    blending.as_deref_mut()
                {
                    unseal(black, *knockout);
                }
            }
            _ => {}
        }
    }
}

/// Whether §11.6.4.3's flag reinterprets nothing in this element, so that both readings
/// describe it and [`AlphaSourcesSeen::Mixed`] content holding it can still be drawn.
///
/// §11.6.4.3 and §11.6.4.4 give the flag exactly two things to reinterpret — the soft mask and
/// the two alpha constants — and §11.6.4.2 fixes every other opacity input at 1.0. So the
/// question is [`element_alpha_is_shape`]'s under [`AlphaSource::Opacity`]: does this element
/// state a mask or a constant *as opacity*.
///
/// **A [`Command::Shaped`] answers yes here and no there, and the two answers are to different
/// questions.** [`super::shape::group_alpha_is_shape`] declines a stated pair because proving that the alpha
/// its two composites leave is §8.5.4's shape is a separate argument, which costs an exact
/// intersection and never correctness. This question is the clause's: a pair arrives with its
/// shape *already stated*, under the reading its own group's content ran under, and no flag in
/// an enclosing state can reinterpret a quantity that is no longer an input. Borrowing the
/// other predicate whole made a group holding an inner knockout group's element refuse for a
/// reason that belongs to the other question (ADR 1205).
///
/// **Cost, `callgrind_interpret`, 50 interpretations of ISO 32000-2 page 101, one sitting**:
/// 1 252 899 002 → 1 252 899 128, +126 instructions, **+0.00001%**. A page whose record is
/// already settled cannot pay at all — `Option::or_else` is not evaluated there — so what the
/// number bounds is the `matches!` this adds per element of a group that painted under both
/// readings.
fn flag_reinterprets_nothing(command: &Command) -> bool {
    matches!(command, Command::Shaped { .. })
        || element_alpha_is_shape(command, AlphaSource::Opacity)
}

impl Interpreter<'_> {
    /// Records that the content being run now paints under §11.6.4.3's reading
    /// `alpha_is_shape`, which a `gs` stating `/AIS`, a `Q` restoring a different one and the
    /// end of a nested content stream all change: the entry is a graphics state parameter, so
    /// `Q` puts the reading back as it puts back every other one, and §8.10.1's form ends with
    /// the `Q` its `Do` began with the `q` of.
    ///
    /// Nothing is decided at the statement. What was painted under the reading being left is
    /// joined to the record first ([`Self::settle_readings_until`]), and the new one is only
    /// *in force*: a reading nothing is painted under never reaches the record, so a `q`…`Q`
    /// that states it and paints nothing leaves the record as it found it (ADR 1319).
    pub(in crate::content) fn note_alpha_source(&mut self, alpha_is_shape: bool) {
        let now = self.list.command_count();
        self.settle_readings_until(now);
        self.readings.in_force = alpha_is_shape;
        self.readings.since = now;
    }

    /// Joins what was painted since the reading in force took force, up to `end`, to the
    /// record under that reading.
    ///
    /// Only an element §11.6.4.3's flag reinterprets is a reading's to answer for
    /// ([`flag_reinterprets_nothing`]): an opaque mark, or one that states its own shape, is
    /// described by both, and a scope of such marks stated under both readings has painted
    /// under neither.
    ///
    /// A record that already names the reading in force has nothing to learn from the range,
    /// so it is not walked — which is every page that never states `/AIS`, once its first
    /// translucent mark is joined.
    fn settle_readings_until(&mut self, end: usize) {
        let since = self.readings.since;
        let reading = AlphaSourcesSeen::of(self.readings.in_force);
        let reinterpreted = self.readings.painted != Some(reading)
            && end > since
            && self.list.commands().get(since..end).is_some_and(|painted| {
                painted
                    .iter()
                    .any(|command| !flag_reinterprets_nothing(command))
            });
        if reinterpreted {
            self.join_reading(reading, since);
        }
        self.readings.since = end;
    }

    /// Joins elements painted under `reading`, the first of them at `boundary`, to the record
    /// of the scope being run — sealing the elements before them where they were painted
    /// under the other reading (ISO 32000-2 §11.6.4.3, §11.6.4.4, §11.4.6).
    ///
    /// This is where a scope that paints under both readings stays describable. Everything
    /// the record holds before `boundary` was painted under the one reading it names, so each
    /// such element can state its shape under that reading beside itself —
    /// [`Command::Shaped`], §11.6.4.2's shape built from the quantity the element's own
    /// reading makes shape — and is then described by both readings, since no flag can
    /// reinterpret a shape that is no longer an input. The record then names the new reading
    /// alone, and a knockout group built from the scope reads every element's shape under the
    /// reading it was painted under. An element whose shape cannot be stated leaves the record
    /// [`AlphaSourcesSeen::Mixed`], which is refused by that name where it is read.
    fn join_reading(&mut self, reading: AlphaSourcesSeen, boundary: usize) {
        let joined = match self.readings.painted {
            None => reading,
            Some(painted) if painted == reading => reading,
            Some(AlphaSourcesSeen::Mixed) => AlphaSourcesSeen::Mixed,
            Some(_) if reading == AlphaSourcesSeen::Mixed => AlphaSourcesSeen::Mixed,
            Some(painted) => {
                if self.seal_elements(self.readings.start, boundary, painted) {
                    // Everything before the boundary is sealed or needs no seal, so the next
                    // turn seals from here: each element is sealed once and moved once.
                    self.readings.start = boundary;
                    reading
                } else {
                    AlphaSourcesSeen::Mixed
                }
            }
        };
        self.readings.painted = Some(joined);
    }

    /// Seals the list's commands from `from` to `until` under `reading` ([`seal`]), and answers
    /// whether every one that needed it could be.
    pub(in crate::content) fn seal_elements(
        &mut self,
        from: usize,
        until: usize,
        reading: AlphaSourcesSeen,
    ) -> bool {
        let Some(alpha) = reading.settled() else {
            return false;
        };
        let mut tail = self.list.split_off_commands(from);
        let count = until.saturating_sub(from).min(tail.len());
        let sealed = tail
            .get_mut(..count)
            .map(|elements| seal(elements, alpha, self.image_masks.shape_masks()));
        // Put back one for one, so that every mark a caller holds into the list still names
        // the command it named.
        for command in tail {
            self.list.push(command);
        }
        self.readings.sealed |= sealed.is_some_and(|sealed| sealed.any);
        sealed.is_none_or(|sealed| sealed.every)
    }

    /// Joins a nested scope's elements, which begin at `from` and were painted under `inside`,
    /// to the enclosing record — for a scope whose commands are the enclosing content's own
    /// elements rather than one group that encloses them: a flattened group, a text object's
    /// glyphs, a combined fill and stroke (ADR 1319).
    pub(in crate::content) fn absorb_reading(&mut self, inside: AlphaSourcesSeen, from: usize) {
        self.settle_readings_until(from);
        let now = self.list.command_count();
        let reinterpreted = self.list.commands().get(from..now).is_some_and(|painted| {
            painted
                .iter()
                .any(|command| !flag_reinterprets_nothing(command))
        });
        if reinterpreted {
            self.join_reading(inside, from);
        }
        self.readings.since = now;
    }

    /// The record's answer for the scope being run, with everything painted so far joined.
    pub(in crate::content) fn settled_readings(&mut self) -> AlphaSourcesSeen {
        let now = self.list.command_count();
        self.settle_readings_until(now);
        self.readings.seen()
    }

    /// Records that what the scope being run painted cannot be described by one reading.
    pub(in crate::content) fn poison_readings(&mut self) {
        self.readings.painted = Some(AlphaSourcesSeen::Mixed);
    }

    /// Opens a record of §11.6.4.3's readings for a scope whose content starts under
    /// `alpha_is_shape` at the current command count, and answers the enclosing record for
    /// [`Self::close_reading_scope`].
    ///
    /// A tiling cell, a text object, a group's content and a combined fill and stroke are such
    /// scopes: each paints under readings of its own — a cell's is the one §8.7.3.1 installs
    /// rather than the painting mark's, a text object's are those in force at each of its
    /// glyphs — and which of them reach the enclosing record is decided when the scope's
    /// commands are placed (ADR 1306, ADR 1319).
    pub(in crate::content) fn open_reading_scope(&mut self, alpha_is_shape: bool) -> Readings {
        let at = self.list.command_count();
        std::mem::replace(&mut self.readings, Readings::open(alpha_is_shape, at))
    }

    /// [`Self::open_reading_scope`], for a scope whose elements may become a knockout group's
    /// — a text object under §9.3.8's `Tk`, or a fill and stroke under §11.7.4.4.
    pub(in crate::content) fn open_knockout_reading_scope(
        &mut self,
        alpha_is_shape: bool,
    ) -> Readings {
        let outer = self.open_reading_scope(alpha_is_shape);
        self.readings.knockout_elements = true;
        outer
    }

    /// Closes a scope opened by [`Self::open_reading_scope`]: the enclosing record comes back
    /// untouched but for whether a seal was made, and the scope's own is the answer, with the
    /// reading in force at its end — which a text object hands on, since §9.3.8 makes
    /// "\[c]hanges made to graphics state parameters within the text object ... persist beyond
    /// the end of the text object".
    pub(in crate::content) fn close_reading_scope(
        &mut self,
        outer: Readings,
    ) -> (AlphaSourcesSeen, bool) {
        let inside = self.settled_readings();
        let in_force = self.readings.in_force;
        let sealed = self.readings.sealed;
        self.readings = outer;
        self.readings.sealed |= sealed;
        (inside, in_force)
    }

    /// Seals a finished group's elements under the reading `inside` their content ran under,
    /// the black half of a pair beside them, and answers whether every one could be ([`seal`]).
    ///
    /// The two halves are two runs of one content stream and `paired` has already held them to
    /// one structure, so both are sealed or neither is described.
    fn seal_group_content(
        &mut self,
        inside: AlphaSourcesSeen,
        commands: &mut [Command],
        pair: &mut Option<pdf_render::GroupBlending>,
    ) -> bool {
        let Some(alpha) = inside.settled() else {
            return false;
        };
        let shape_masks = self.image_masks.shape_masks();
        let chromatic = seal(commands, alpha, shape_masks);
        let black = match pair.as_mut() {
            Some(pdf_render::GroupBlending::FourComponents { black, .. }) => {
                seal(black, alpha, shape_masks)
            }
            _ => Sealed {
                every: true,
                any: false,
            },
        };
        self.readings.sealed |= chromatic.any || black.any;
        chromatic.every && black.every
    }

    /// Makes a group whose raster is not its own shape an element the reading at its `Do`
    /// describes (ISO 32000-2 §11.6.4.3, §11.6.4.4).
    ///
    /// Its own constant and mask are §11.6.4.4's and §11.6.4.3's inputs at the `Do`, read under
    /// the reading in force there; its content is reached by whatever the enclosing content
    /// asks of it, element by element, so content that ran under the other reading states its
    /// shapes beside itself first. Where one cannot, the enclosing record says so (ADR 1319).
    pub(super) fn describe_group_by_its_do(
        &mut self,
        inside: AlphaSourcesSeen,
        outer: &GraphicsState,
        commands: &mut [Command],
        pair: &mut Option<pdf_render::GroupBlending>,
    ) {
        if inside != AlphaSourcesSeen::of(outer.alpha_is_shape)
            && !self.seal_group_content(inside, commands, pair)
        {
            self.poison_readings();
        }
    }

    /// Starts the scope being run again from `at`, for a second run of the same content whose
    /// first run's commands were taken off the list: the second has to make the same seals the
    /// first did, which it can only do from the same start.
    pub(super) fn restart_reading_scope(&mut self, alpha_is_shape: bool, at: usize) {
        let sealed = self.readings.sealed;
        self.readings = Readings::open(alpha_is_shape, at);
        self.readings.sealed = sealed;
    }
}
