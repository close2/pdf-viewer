//! What each operator of ISO 32000-2 Table 50 takes, for a walk that reads a content stream's
//! tokens without interpreting them (ADR 3541).
//!
//! # Why a second reader of the operands exists, and why it is this one
//!
//! The interpreter repairs operands as it runs them — an operator short of its operands is run
//! with what is there, a `Q` with nothing saved is ignored, three `sc` operands under
//! `DeviceGray` are read in `DeviceRGB` — and logs each repair where it makes it. A validator
//! does not interpret: running the interpreter over every page and form to learn five facts
//! about the token sequence would decode every image and load every font a second time, beside
//! the walk the validator already makes. Every one of them is a function of the tokens, the
//! `q`/`Q` and `BT`/`ET` nesting, and the number of components of the space a `cs` selected, so
//! a walk that already reads the tokens can ask them here, of the same table the interpreter's
//! operand counts are held to (`operand_counts_are_the_grammars`).
//!
//! They are, each under its clause:
//!
//! - §7.8.2: "all of the operands needed by an operator shall immediately precede that
//!   operator" — an operator short of its table's count, or given an operand of a type its table
//!   does not state; and its "operands shall not be left over when an operator finishes
//!   execution" — operands in front of the operator's own, which the interpreter drops;
//! - §8.4.2: "Occurrences of the q and Q operators shall be balanced within a given content
//!   stream" — a `Q` with nothing saved;
//! - §9.4.1, Figure 9: text-showing and text-positioning operators appear inside a text object;
//! - §8.6.8, Table 73: `sc` takes as many operands as the current colour space has components;
//! - §14.6.1, Table 352: `BMC` and `BDC` each begin a sequence "terminated by a balancing EMC
//!   operator" — an `EMC` with nothing open, or a stream ending inside a sequence;
//! - §7.8.2: `BX` and `EX` "shall occur in pairs" — an `EX` with nothing open, or a stream ending
//!   inside a section;
//! - §14.6.2: a `BDC`'s property list named by its operand "shall be defined as a named resource
//!   in the Properties subdictionary" — answered by the walk's lookup ([`Grammar::property_list`]).
//!
//! [`COVERAGE`] says, for every class of repair logged inside content, whether this grammar, the
//! reader the validator's walk shares with the interpreter, or the interpreter alone reads it.
//!
//! The last two are what a stream leaves open as much as what one operator does, so
//! [`Grammar::end`] answers them at the stream's end, where the interpreter makes the same repair.

use pdf_syntax::{Document, Object, RepairClass};

use crate::repair;

/// What one operand token is, as far as an operator's table can ask.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operand {
    /// An integer or a real (§7.3.3).
    Number,
    /// A name (§7.3.5).
    Name,
    /// A literal or hexadecimal string (§7.3.4).
    Text,
    /// A dictionary written in the stream (§14.6.2's property list).
    Dictionary,
    /// An array's `[`: §7.3.6's array is one object, so a keyword between its brackets is no
    /// operator, and the interpreter steps over it (ADR 3629).
    ArrayOpen,
    /// An array's `]`.
    ArrayClose,
    /// Anything else: a boolean, a null, or an array a walk hands over as one operand.
    Other,
}

/// What one operand place of an operator's table admits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Wants {
    /// A number.
    Number,
    /// A name.
    Name,
    /// A string.
    Text,
    /// §14.6.2's property list: a name of a `/Properties` entry, or a dictionary.
    Properties,
}

impl Wants {
    const fn admits(self, operand: Operand) -> bool {
        matches!(
            (self, operand),
            (Self::Number, Operand::Number)
                | (Self::Name | Self::Properties, Operand::Name)
                | (Self::Text, Operand::Text)
                | (Self::Properties, Operand::Dictionary)
        )
    }
}

/// What an operator does to the state the five checks read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    /// `q`.
    Save,
    /// `Q`.
    Restore,
    /// `BT`.
    BeginText,
    /// `ET`.
    EndText,
    /// A text-showing or text-positioning operator (Tables 106 and 107).
    InText,
    /// `CS` or `cs`: the space's components come from the walk ([`Grammar::select_space`]).
    SelectSpace { stroking: bool },
    /// `G`, `g`, `RG`, `rg`, `K` or `k`: a device space of this many components.
    DeviceColour { stroking: bool, components: usize },
    /// `SC`, `sc`, `SCN` or `scn`.
    Colour { stroking: bool },
    /// `BMC` or `BDC`.
    BeginMarked,
    /// `EMC`.
    EndMarked,
    /// `BX`.
    BeginCompatibility,
    /// `EX`.
    EndCompatibility,
    /// `ri` or `sh`: no colour the grammar counts, but one of §8.6.8's colour operators.
    OtherColour,
    /// `d1`: the colour operators after it in its glyph description are ignored (§8.6.8).
    ShapeOnlyGlyph,
    /// Everything else.
    Plain,
}

impl Role {
    /// Whether the operator is one of the colour operators §8.6.8 has a `d1` glyph description
    /// and an uncoloured tiling pattern ignore: Table 73's twelve, `ri` and `sh`.
    const fn is_colour(self) -> bool {
        matches!(
            self,
            Self::SelectSpace { .. }
                | Self::DeviceColour { .. }
                | Self::Colour { .. }
                | Self::OtherColour
        )
    }
}

/// One operator of Table 50, as the grammar reads it.
#[derive(Debug, Clone, Copy)]
pub struct Signature {
    /// The operand places its table states, where it states a fixed number.
    wants: Option<&'static [Wants]>,
    /// What it does to the nesting and the colour space.
    role: Role,
}

impl Signature {
    /// How many operands its table states, where it states a fixed number.
    #[must_use]
    pub fn operand_count(&self) -> Option<usize> {
        self.wants.map(<[Wants]>::len)
    }
}

const N: Wants = Wants::Number;
const NAME: &[Wants] = &[Wants::Name];
const ONE_NUMBER: &[Wants] = &[N];
const TWO_NUMBERS: &[Wants] = &[N, N];
const THREE_NUMBERS: &[Wants] = &[N, N, N];
const FOUR_NUMBERS: &[Wants] = &[N, N, N, N];
const SIX_NUMBERS: &[Wants] = &[N, N, N, N, N, N];
const NONE: &[Wants] = &[];

/// The operator `word` names, or `None` for a keyword Table 50 does not list.
///
/// Each list is the operand list of the table Annex A's summary points at for the operator — the
/// same tables `Operator::operand_count` cites, and a test holds the two to one another.
#[must_use]
pub fn signature(word: &[u8]) -> Option<Signature> {
    let of = |wants: &'static [Wants], role| Signature {
        wants: Some(wants),
        role,
    };
    let plain = |wants: &'static [Wants]| of(wants, Role::Plain);
    let text = |wants: &'static [Wants]| of(wants, Role::InText);
    let device = |stroking, wants: &'static [Wants]| {
        of(
            wants,
            Role::DeviceColour {
                stroking,
                components: wants.len(),
            },
        )
    };
    Some(match word {
        b"q" => of(NONE, Role::Save),
        b"Q" => of(NONE, Role::Restore),
        b"BT" => of(NONE, Role::BeginText),
        b"ET" => of(NONE, Role::EndText),
        b"h" | b"n" | b"f" | b"F" | b"f*" | b"S" | b"s" | b"B" | b"B*" | b"b" | b"b*" | b"W"
        | b"W*" | b"BI" => plain(NONE),
        b"EMC" => of(NONE, Role::EndMarked),
        b"BX" => of(NONE, Role::BeginCompatibility),
        b"EX" => of(NONE, Role::EndCompatibility),
        b"T*" => text(NONE),
        b"w" | b"J" | b"j" | b"M" | b"i" | b"Tc" | b"Tw" | b"Tz" | b"TL" | b"Ts" | b"Tr" => {
            plain(ONE_NUMBER)
        }
        b"gs" | b"Do" | b"MP" => plain(NAME),
        b"ri" | b"sh" => of(NAME, Role::OtherColour),
        b"BMC" => of(NAME, Role::BeginMarked),
        b"Tj" | b"'" => text(&[Wants::Text]),
        b"\"" => text(&[N, N, Wants::Text]),
        b"Td" | b"TD" => text(TWO_NUMBERS),
        b"Tm" => text(SIX_NUMBERS),
        b"TJ" => Signature {
            wants: None,
            role: Role::InText,
        },
        b"m" | b"l" | b"d0" => plain(TWO_NUMBERS),
        b"Tf" => plain(&[Wants::Name, N]),
        b"DP" => plain(&[Wants::Name, Wants::Properties]),
        b"BDC" => of(&[Wants::Name, Wants::Properties], Role::BeginMarked),
        b"v" | b"y" | b"re" => plain(FOUR_NUMBERS),
        b"cm" | b"c" => plain(SIX_NUMBERS),
        b"d1" => of(SIX_NUMBERS, Role::ShapeOnlyGlyph),
        b"CS" | b"cs" => of(
            NAME,
            Role::SelectSpace {
                stroking: word == b"CS",
            },
        ),
        b"G" => device(true, ONE_NUMBER),
        b"g" => device(false, ONE_NUMBER),
        b"RG" => device(true, THREE_NUMBERS),
        b"rg" => device(false, THREE_NUMBERS),
        b"K" => device(true, FOUR_NUMBERS),
        b"k" => device(false, FOUR_NUMBERS),
        b"SC" | b"SCN" => Signature {
            wants: None,
            role: Role::Colour { stroking: true },
        },
        b"sc" | b"scn" => Signature {
            wants: None,
            role: Role::Colour { stroking: false },
        },
        b"d" | b"ID" | b"EI" => Signature {
            wants: None,
            role: Role::Plain,
        },
        _ => return None,
    })
}

/// How many operands of one stretch the grammar keeps: the most any fixed list states.
const KEPT: usize = 6;

/// The checks over one content stream's tokens, in the order the stream states them.
///
/// Fed by [`Self::operand`] for every operand token and [`Self::operator`] for every operator, it
/// answers each operator with the class of repair an interpreter would have made there, or none,
/// and [`Self::end`] answers what the stream left open when it ends.
/// A page's stream starts in §8.4.1's initial state — `DeviceGray`, one component, for both
/// colours ([`Spaces::INITIAL`]) — and its own `q` and `Q` decide what a `Q` restores. A form
/// `XObject` starts in the spaces in force at the `Do` that paints it ([`Self::starting_in`]), as
/// ISO 32000-2 §8.10.1 says after the steps `Do` performs, none of which touches a colour space:
///
/// > Except as described above, the initial graphics state for the form shall be inherited from
/// > the graphics state that is in effect at the time Do is invoked.
#[derive(Debug, Clone)]
pub struct Grammar {
    /// The last [`KEPT`] operands since the last operator, oldest first, and how many there were.
    operands: [Operand; KEPT],
    given: usize,
    /// How many numbers since the last operator, for the colour operators, which state no list.
    numbers: usize,
    /// The colour spaces' components saved by each open `q`: stroking, then non-stroking.
    saved: Vec<(Option<usize>, Option<usize>)>,
    /// The current spaces' components, `None` where the walk could not count them (a pattern, a
    /// name no resource defines).
    stroke: Option<usize>,
    fill: Option<usize>,
    /// Whether a `BT` is open.
    in_text: bool,
    /// How many `BMC` and `BDC` sections are open, and how many `BX` sections.
    marked: usize,
    compatibility: usize,
    /// How deep in arrays the operands are, counted as the interpreter counts them: a keyword
    /// inside an array is stepped over, and the operands around it stay the next operator's.
    array_depth: usize,
    /// Whether §8.6.8's colour operators are ignored here: after a `d1`, or in a stream the walk
    /// says an uncoloured tiling pattern or such a glyph invokes ([`Self::ignore_colour`]).
    uncoloured: bool,
    /// Whether the operator just answered was a `BDC` whose property list is a name, which only
    /// the resources can resolve ([`Self::property_list`]).
    names_a_property_list: bool,
}

/// The repairs the interpreter makes at one operator, as the grammar reads them off the tokens: at
/// most the one that decides whether it runs, and §7.8.2's operands left over in front of its own.
#[derive(Debug, Clone, Copy, Default)]
pub struct Found {
    /// A shortfall, an operand of the wrong type, or the state's class (§8.4.2, §9.4.1, §8.6.8,
    /// §14.6.1, §7.8.2's pairs).
    decides: Option<&'static RepairClass>,
    /// Whether operands stood in front of the ones the operator's table states.
    left_over: bool,
}

impl Found {
    /// The classes, the deciding one first.
    pub fn classes(self) -> impl Iterator<Item = &'static RepairClass> {
        [
            self.decides,
            self.left_over.then_some(&repair::OPERANDS_LEFT_OVER),
        ]
        .into_iter()
        .flatten()
    }

    /// Whether the operator needed no repair.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.decides.is_none() && !self.left_over
    }
}

/// How many components each current colour space has, as a walk counts them: `None` where it could
/// not (a pattern, a name no resource defines), and then nothing is judged against it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spaces {
    /// The stroking colour space's components.
    pub stroke: Option<usize>,
    /// The non-stroking colour space's components.
    pub fill: Option<usize>,
}

impl Spaces {
    /// §8.4.1's initial state: `DeviceGray` for both, one component each.
    pub const INITIAL: Self = Self {
        stroke: Some(1),
        fill: Some(1),
    };

    /// Spaces no walk could count, where a stream may start in more than one state: no colour
    /// operator is judged until the stream selects a space of its own.
    pub const UNCOUNTED: Self = Self {
        stroke: None,
        fill: None,
    };
}

impl Default for Spaces {
    fn default() -> Self {
        Self::INITIAL
    }
}

impl Default for Grammar {
    fn default() -> Self {
        Self::starting_in(Spaces::INITIAL)
    }
}

impl Grammar {
    /// A stream that starts in `spaces`: a page's in [`Spaces::INITIAL`], a form `XObject`'s in
    /// the spaces in force at its `Do` (ISO 32000-2 §8.10.1), a Type 3 glyph description's in
    /// those at the text-showing operator that invokes it (§9.6.4), and a tiling pattern's cell in
    /// those its parent stream began in (§8.7.3.1's step b).
    #[must_use]
    pub const fn starting_in(spaces: Spaces) -> Self {
        Self {
            operands: [Operand::Other; KEPT],
            given: 0,
            numbers: 0,
            saved: Vec::new(),
            stroke: spaces.stroke,
            fill: spaces.fill,
            in_text: false,
            marked: 0,
            compatibility: 0,
            array_depth: 0,
            uncoloured: false,
            names_a_property_list: false,
        }
    }
}

/// How deep the saved spaces may grow; past it a `q` saves nothing more and a `Q` is not judged,
/// so a hostile stream of `q`s costs a bound rather than memory.
const MAX_SAVED: usize = 1024;

impl Grammar {
    /// One operand token: an array's brackets and each element are one each, as the interpreter
    /// keeps them.
    pub fn operand(&mut self, operand: Operand) {
        match operand {
            Operand::ArrayOpen => self.array_depth = self.array_depth.saturating_add(1),
            Operand::ArrayClose => self.array_depth = self.array_depth.saturating_sub(1),
            _ => {}
        }
        // Past the interpreter's bound on one operator's operands it keeps no more and stops
        // believing the array it was in, so an unclosed `[` does not swallow the stream.
        if self.given >= super::MAX_OPERANDS {
            self.array_depth = 0;
        }
        if self.given >= KEPT {
            self.operands.rotate_left(1);
        }
        let at = self.given.min(KEPT - 1);
        self.operands[at] = operand;
        self.given = self.given.saturating_add(1);
        if operand == Operand::Number {
            self.numbers = self.numbers.saturating_add(1);
        }
    }

    /// A keyword that is no operator: the operands before it belong to nothing, so the next
    /// operator's are counted afresh.
    ///
    /// Inside an array it is stepped over, as §7.3.6 admits nothing but objects into one and the
    /// interpreter keeps the array's operands for the operator after it.
    pub fn boundary(&mut self) {
        if self.array_depth > 0 {
            return;
        }
        self.given = 0;
        self.numbers = 0;
    }

    /// A stream whose colour operators the reader ignores, §8.6.8's two cases: an uncoloured
    /// tiling pattern's cell and a `d1` glyph description, and every stream either invokes. The
    /// interpreter drops such an operator with its operands before asking anything of them, so
    /// the grammar answers nothing at one.
    pub const fn ignore_colour(&mut self) {
        self.uncoloured = true;
    }

    /// Whether this stream's colour operators are ignored, so that a walk passes it on to the
    /// streams this one invokes after a `d1`.
    #[must_use]
    pub const fn ignores_colour(&self) -> bool {
        self.uncoloured
    }

    /// The spaces in force after the last operator, which a stream this one invokes starts in
    /// ([`Self::starting_in`]).
    #[must_use]
    pub const fn spaces(&self) -> Spaces {
        Spaces {
            stroke: self.stroke,
            fill: self.fill,
        }
    }

    /// The operator `signature` names, after its operands: the repairs it needs. Where it needs
    /// two of the deciding kind, the operand's is answered, since that is the one that decides
    /// whether the operator runs at all; §7.8.2's leftovers are answered beside it.
    pub fn operator(&mut self, signature: Signature) -> Found {
        self.names_a_property_list = false;
        // §7.3.6: a keyword between an array's brackets is neither an element nor an operator,
        // and the interpreter steps over it with the array's operands kept.
        if self.array_depth > 0 {
            return Found::default();
        }
        // §8.6.8: "all of the following operators shall be ignored", operands and all, before
        // the interpreter asks anything of them.
        if self.uncoloured && signature.role.is_colour() {
            self.boundary();
            return Found::default();
        }
        // A section is opened whatever its operands are: the interpreter runs `BMC` and `BDC`
        // with what it was given, so the `EMC` after one balances it either way.
        match signature.role {
            Role::BeginMarked => self.marked = self.marked.saturating_add(1),
            Role::BeginCompatibility => self.compatibility = self.compatibility.saturating_add(1),
            Role::ShapeOnlyGlyph => self.uncoloured = true,
            _ => {}
        }
        // §7.8.2: "operands shall not be left over when an operator finishes execution" — the
        // interpreter gives the operator the last of them and drops the rest.
        let left_over = signature
            .wants
            .is_some_and(|wants| self.given > wants.len());
        // §14.6.2's second form names its property list in the last of two operands; whether the
        // resources hold it is the walk's to answer.
        self.names_a_property_list = signature.role == Role::BeginMarked
            && signature.operand_count() == Some(2)
            && self.given >= 2
            && self
                .given
                .min(KEPT)
                .checked_sub(1)
                .and_then(|last| self.operands.get(last))
                == Some(&Operand::Name);
        let decides = self
            .operands_found(signature)
            .or_else(|| self.state_found(signature));
        self.boundary();
        Found { decides, left_over }
    }

    /// Whether the operator just answered was a `BDC` naming its property list, which the walk
    /// resolves in the resources' `/Properties` (§14.6.2) and answers with [`Self::property_list`].
    #[must_use]
    pub const fn names_a_property_list(&self) -> bool {
        self.names_a_property_list
    }

    /// The class of a `BDC` whose named property list the walk found (`resolved`) or did not:
    /// §14.6.2's list "shall be defined as a named resource in the Properties subdictionary", and
    /// the interpreter reads one it cannot find as stating nothing (ADR 3629).
    #[must_use]
    pub fn property_list(&self, resolved: bool) -> Option<&'static RepairClass> {
        (self.names_a_property_list && !resolved).then_some(&repair::PROPERTY_LIST_UNRESOLVED)
    }

    /// A `CS` or `cs` the walk resolved: the space it selected has `components`, or `None` where
    /// the walk could not count them.
    pub const fn select_space(&mut self, stroking: bool, components: Option<usize>) {
        if stroking {
            self.stroke = components;
        } else {
            self.fill = components;
        }
    }

    /// §7.8.2's shortfall and type, against the operator's own list.
    fn operands_found(&self, signature: Signature) -> Option<&'static RepairClass> {
        let wants = signature.wants?;
        if self.given < wants.len() {
            return Some(&repair::OPERATOR_SHORT_OF_OPERANDS);
        }
        // The last `wants.len()` operands, which is what §7.8.2's "immediately precede" makes
        // the operator's own; leftovers in front of them are another question (ADR 3541).
        let kept = self.given.min(KEPT);
        let own = self.operands.get(kept.saturating_sub(wants.len())..kept)?;
        let fits = own
            .iter()
            .zip(wants)
            .all(|(operand, wants)| wants.admits(*operand));
        (!fits).then_some(&repair::OPERAND_OF_THE_WRONG_TYPE)
    }

    /// §8.4.2's balance, §9.4.1's text object, and §8.6.8's component count.
    fn state_found(&mut self, signature: Signature) -> Option<&'static RepairClass> {
        match signature.role {
            Role::Save => {
                if self.saved.len() < MAX_SAVED {
                    self.saved.push((self.stroke, self.fill));
                }
                None
            }
            Role::Restore => match self.saved.pop() {
                Some((stroke, fill)) => {
                    (self.stroke, self.fill) = (stroke, fill);
                    None
                }
                None => Some(&repair::RESTORE_WITH_NOTHING_SAVED),
            },
            Role::BeginText => {
                self.in_text = true;
                None
            }
            Role::EndText => {
                self.in_text = false;
                None
            }
            Role::InText => (!self.in_text).then_some(&repair::TEXT_OPERATOR_OUTSIDE_A_TEXT_OBJECT),
            Role::SelectSpace { stroking } => {
                // The walk counts the space it resolves; until it does, nothing is judged.
                self.select_space(stroking, None);
                None
            }
            Role::DeviceColour {
                stroking,
                components,
            } => {
                self.select_space(stroking, Some(components));
                None
            }
            Role::Colour { stroking } => {
                let current = if stroking { self.stroke } else { self.fill };
                current
                    .is_some_and(|components| components != self.numbers)
                    .then_some(&repair::COLOUR_OPERANDS_DO_NOT_FIT_THE_SPACE)
            }
            // Table 352's `BMC` and `BDC` are each "terminated by a balancing EMC operator", and
            // §7.8.2's `BX` and `EX` "shall occur in pairs": a closing operator with nothing open
            // balances nothing.
            Role::EndMarked => match self.marked.checked_sub(1) {
                Some(open) => {
                    self.marked = open;
                    None
                }
                None => Some(&repair::MARKED_CONTENT_UNBALANCED),
            },
            Role::EndCompatibility => match self.compatibility.checked_sub(1) {
                Some(open) => {
                    self.compatibility = open;
                    None
                }
                None => Some(&repair::COMPATIBILITY_SECTIONS_UNBALANCED),
            },
            Role::BeginMarked
            | Role::BeginCompatibility
            | Role::OtherColour
            | Role::ShapeOnlyGlyph
            | Role::Plain => None,
        }
    }

    /// What the stream left open at its end: a marked-content section and a compatibility
    /// section each read as if closed there, which the interpreter repairs at the end of the same
    /// stream (`doc/adr/3614`).
    pub fn end(&self) -> impl Iterator<Item = &'static RepairClass> {
        [
            (self.marked > 0).then_some(&repair::MARKED_CONTENT_UNBALANCED),
            (self.compatibility > 0).then_some(&repair::COMPATIBILITY_SECTIONS_UNBALANCED),
        ]
        .into_iter()
        .flatten()
    }
}

/// Who reads a class of repair made inside content, so that a validator's walk, which does not
/// interpret, knows every class before it decides (ADR 3630).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reading {
    /// This grammar, off the tokens, with the walk's resolution of a name where the class turns
    /// on one: a `cs`'s space ([`Grammar::select_space`]) or a `BDC`'s property list
    /// ([`Grammar::property_list`]).
    Grammar,
    /// The content reader, its inline-object reader or §8.9.7's inline-image scan, or the lexer
    /// under them: code the validator's walk runs on the same bytes as the interpreter does.
    Reader,
    /// Only the interpreter, for the reason given: the class is a fact about an object a name
    /// resolves to, read where the operator applies it, and not about the tokens.
    Interpreter(&'static str),
}

/// Why the classes of a graphics state parameter dictionary are the interpreter's.
const A_GRAPHICS_STATE_DICTIONARY: &str = "an entry of the dictionary a gs names (Table 57), read \
     where the interpreter applies it: its type, its range, and whether a function or halftone it \
     names reads are facts about that object, which no token states";

/// Why the classes of a form `XObject`'s dictionary are the interpreter's.
const A_FORM_DICTIONARY: &str = "an entry of the form XObject a Do names (Table 93), read where \
     the interpreter invokes it: its Matrix, BBox and Resources are facts about that object, which \
     no token states";

/// Every class of repair `pdf_model`'s content reader and interpreter and `pdf_syntax`'s lexer log
/// inside content, and who reads it (ADR 3630).
///
/// A class the interpreter alone logs appears in the log only once a page is drawn, after a
/// validator's walk has decided — the trap ADR 3614 found with marked-content balance. So each
/// class a token decides is read here, and each that is left to the interpreter states why.
/// `tools/conformance/tests/content_classes.rs` holds the table to the sources: a class logged
/// in content and missing here fails it.
pub static COVERAGE: &[(&RepairClass, Reading)] = &[
    (&repair::OPERAND_OF_THE_WRONG_TYPE, Reading::Grammar),
    (&repair::OPERATOR_SHORT_OF_OPERANDS, Reading::Grammar),
    (&repair::OPERANDS_LEFT_OVER, Reading::Grammar),
    (&repair::RESTORE_WITH_NOTHING_SAVED, Reading::Grammar),
    (
        &repair::TEXT_OPERATOR_OUTSIDE_A_TEXT_OBJECT,
        Reading::Grammar,
    ),
    (
        &repair::COLOUR_OPERANDS_DO_NOT_FIT_THE_SPACE,
        Reading::Grammar,
    ),
    (&repair::COMPATIBILITY_SECTIONS_UNBALANCED, Reading::Grammar),
    (&repair::MARKED_CONTENT_UNBALANCED, Reading::Grammar),
    (&repair::PROPERTY_LIST_UNRESOLVED, Reading::Grammar),
    (&repair::CONTENTS_ENTRY_NOT_A_STREAM, Reading::Reader),
    (&repair::CONTENT_PAST_A_BOUND, Reading::Reader),
    (&repair::INLINE_OBJECT_MALFORMED, Reading::Reader),
    (&repair::INLINE_IMAGE_KEY_TWICE, Reading::Reader),
    (
        &repair::INLINE_IMAGE_COLOUR_SPACE_UNRESOLVED,
        Reading::Reader,
    ),
    (&repair::INLINE_IMAGE_DATA_EXTENT, Reading::Reader),
    (&pdf_syntax::repair::DELIMITER_UNMATCHED, Reading::Reader),
    (&pdf_syntax::repair::NAME_ESCAPE_INCOMPLETE, Reading::Reader),
    (&pdf_syntax::repair::STRING_UNTERMINATED, Reading::Reader),
    (
        &pdf_syntax::repair::HEXADECIMAL_STRING_UNTERMINATED,
        Reading::Reader,
    ),
    (
        &pdf_syntax::repair::HEXADECIMAL_STRING_STRAY_BYTE,
        Reading::Reader,
    ),
    (&pdf_syntax::repair::INTEGER_PAST_I64, Reading::Reader),
    (
        &pdf_syntax::repair::NUMBER_WITH_AN_EXPONENT,
        Reading::Reader,
    ),
    (&pdf_syntax::repair::NUMBER_SALVAGED, Reading::Reader),
    (
        &repair::GRAPHICS_STATE_ENTRY_OF_THE_WRONG_TYPE,
        Reading::Interpreter(A_GRAPHICS_STATE_DICTIONARY),
    ),
    (
        &repair::TRANSFER_FUNCTION_UNREADABLE,
        Reading::Interpreter(A_GRAPHICS_STATE_DICTIONARY),
    ),
    (
        &repair::ALPHA_CONSTANT_OUT_OF_RANGE,
        Reading::Interpreter(A_GRAPHICS_STATE_DICTIONARY),
    ),
    (
        &repair::HALFTONE_UNREADABLE,
        Reading::Interpreter(A_GRAPHICS_STATE_DICTIONARY),
    ),
    (
        &repair::HALFTONE_TRANSFER_FUNCTION_UNREADABLE,
        Reading::Interpreter(A_GRAPHICS_STATE_DICTIONARY),
    ),
    (
        &repair::BLEND_MODE_UNKNOWN,
        Reading::Interpreter(A_GRAPHICS_STATE_DICTIONARY),
    ),
    (
        &repair::FORM_MATRIX_MALFORMED,
        Reading::Interpreter(A_FORM_DICTIONARY),
    ),
    (
        &repair::FORM_BBOX_UNUSABLE,
        Reading::Interpreter(A_FORM_DICTIONARY),
    ),
    (
        &repair::FORM_RESOURCES_NOT_A_DICTIONARY,
        Reading::Interpreter(A_FORM_DICTIONARY),
    ),
];

/// Whether a keyword a validator's walk reads whole (`ContentReader::with_runs_as_written`) is a
/// run a reader that draws reads as the number off its front — `12pt`, `33600.272.58` — and so an
/// operand of the operator after it rather than a boundary before it.
///
/// The run itself is `syntax/number-salvaged`'s to log; the grammar asks only what the drawing
/// reader's operator was given, so that a salvaged run is not counted twice as a shortfall.
#[must_use]
pub fn drawn_as_a_number(word: &[u8]) -> bool {
    let mut lexer = pdf_syntax::Lexer::new(word);
    let number = matches!(
        lexer.next_token(),
        Some(pdf_syntax::Token::Integer(_) | pdf_syntax::Token::Real(_))
    );
    number
        && lexer
            .salvaged()
            .is_some_and(crate::content::reader::draws_the_salvage)
}

/// How many components the colour space `space` has, where a walk can count them from the
/// object alone (§8.6.4 to §8.6.6, Table 61): `None` for a pattern space, whose `scn` takes a
/// name, and for anything this cannot read.
#[must_use]
pub fn components(document: &Document, space: &Object) -> Option<usize> {
    let space = document.resolve(space);
    if let Some(name) = space.as_name() {
        return family_components(name.as_bytes());
    }
    let items = space.as_array()?;
    let family = document.resolve(items.first()?);
    match family.as_name()?.as_bytes() {
        // An ICC-based space's count is its profile's, which the interpreter reads and a stated
        // `/N` may contradict — a disagreement a row of its own judges (§8.6.5.5, Table 65) — so
        // the grammar, which reads no profile, does not count it.
        b"ICCBased" => None,
        b"DeviceN" => Some(document.resolve(items.get(1)?).as_array()?.len()),
        other => family_components(other),
    }
}

/// The components of a family whose count its name decides.
fn family_components(family: &[u8]) -> Option<usize> {
    match family {
        b"DeviceGray" | b"CalGray" | b"Indexed" | b"Separation" | b"G" | b"I" => Some(1),
        b"DeviceRGB" | b"CalRGB" | b"Lab" | b"RGB" => Some(3),
        b"DeviceCMYK" | b"CMYK" => Some(4),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{COVERAGE, Grammar, Operand, Reading, Spaces, signature};
    use crate::repair;

    fn run(tokens: &[&str]) -> Vec<&'static str> {
        run_in(Spaces::INITIAL, tokens)
    }

    fn run_in(spaces: Spaces, tokens: &[&str]) -> Vec<&'static str> {
        let mut grammar = Grammar::starting_in(spaces);
        let mut found = Vec::new();
        for token in tokens {
            let operand = match token.as_bytes().first() {
                Some(b'0'..=b'9' | b'-' | b'.') => Some(Operand::Number),
                Some(b'/') => Some(Operand::Name),
                Some(b'(') => Some(Operand::Text),
                Some(b'[') => Some(Operand::ArrayOpen),
                Some(b']') => Some(Operand::ArrayClose),
                _ => None,
            };
            if let Some(operand) = operand {
                grammar.operand(operand);
            } else if let Some(signature) = signature(token.as_bytes()) {
                found.extend(
                    grammar
                        .operator(signature)
                        .classes()
                        .map(|class| class.name),
                );
            } else {
                grammar.boundary();
            }
        }
        found
    }

    /// ISO 32000-2 §8.10.1: a form's "initial graphics state ... shall be inherited from the
    /// graphics state that is in effect at the time Do is invoked", so three `sc` operands fit a
    /// form painted where a three-component space is current, and one does not.
    #[test]
    fn a_form_is_judged_in_the_spaces_its_do_left_in_force() {
        let rgb = Spaces {
            stroke: Some(1),
            fill: Some(3),
        };
        assert_eq!(
            run_in(rgb, &["1", "0", "0", "sc", "0", "SC"]),
            [] as [&str; 0]
        );
        assert_eq!(
            run_in(rgb, &["0.5", "sc"]),
            [repair::COLOUR_OPERANDS_DO_NOT_FIT_THE_SPACE.name]
        );
        // A `q` in the form saves the inherited space and its `Q` restores it.
        assert_eq!(
            run_in(rgb, &["q", "0", "g", "Q", "1", "0", "0", "sc"]),
            [] as [&str; 0]
        );
        // Where the walk could not tell which space a form starts in, nothing is judged.
        assert_eq!(
            run_in(Spaces::UNCOUNTED, &["0.5", "sc", "1", "0", "0", "SC"]),
            [] as [&str; 0]
        );
        let mut grammar = Grammar::default();
        grammar.select_space(false, Some(4));
        assert_eq!(
            grammar.spaces(),
            Spaces {
                stroke: Some(1),
                fill: Some(4)
            }
        );
    }

    /// A conforming stream needs nothing.
    #[test]
    fn a_conforming_stream_needs_no_repair() {
        assert_eq!(
            run(&[
                "q",
                "1",
                "0",
                "0",
                "1",
                "0",
                "0",
                "cm",
                "0",
                "0",
                "m",
                "1",
                "1",
                "l",
                "S",
                "BT",
                "/F1",
                "12",
                "Tf",
                "(a)",
                "Tj",
                "ET",
                "/DeviceRGB",
                "cs",
                "1",
                "0",
                "0",
                "sc",
                "Q",
            ]),
            [""; 0]
        );
    }

    /// Each of the five, as §7.8.2, §8.4.2, §9.4.1 and §8.6.8 state them.
    #[test]
    fn each_departure_is_its_class() {
        assert_eq!(
            run(&["(a)", "0", "m"]),
            [repair::OPERAND_OF_THE_WRONG_TYPE.name]
        );
        assert_eq!(run(&["0", "m"]), [repair::OPERATOR_SHORT_OF_OPERANDS.name]);
        assert_eq!(run(&["Q"]), [repair::RESTORE_WITH_NOTHING_SAVED.name]);
        assert_eq!(
            run(&["(a)", "Tj"]),
            [repair::TEXT_OPERATOR_OUTSIDE_A_TEXT_OBJECT.name]
        );
        assert_eq!(
            run(&["0.5", "g", "1", "0", "0", "sc"]),
            [repair::COLOUR_OPERANDS_DO_NOT_FIT_THE_SPACE.name]
        );
    }

    /// A `Q` restores the space the `q` saved.
    #[test]
    fn a_restore_brings_back_the_saved_space() {
        assert_eq!(run(&["q", "1", "0", "0", "rg", "Q", "0.5", "sc"]), [""; 0]);
    }

    /// Table 352's balancing `EMC` and §7.8.2's paired `BX` and `EX`: a closing operator with
    /// nothing open is the class at once, and a stream that ends with a section open is the class
    /// at its end (`doc/adr/3614`). A `BDC` whose tag is not a name still opens a section, because
    /// the interpreter runs it with what it was given.
    #[test]
    fn a_marked_or_compatibility_section_left_open_or_closed_twice_is_the_class() {
        let ended = |tokens: &[&str]| {
            let mut grammar = Grammar::default();
            for token in tokens {
                let signature = signature(token.as_bytes()).expect("Table 50 operators");
                let _ = grammar.operator(signature);
            }
            grammar.end().map(|class| class.name).collect::<Vec<_>>()
        };
        assert_eq!(run(&["/Tag", "BMC", "EMC", "BX", "EX"]), Vec::<&str>::new());
        assert_eq!(run(&["EMC"]), [repair::MARKED_CONTENT_UNBALANCED.name]);
        assert_eq!(
            run(&["EX"]),
            [repair::COMPATIBILITY_SECTIONS_UNBALANCED.name]
        );
        assert_eq!(
            run(&["123", "/oc1", "BDC", "EMC"]),
            [repair::OPERAND_OF_THE_WRONG_TYPE.name],
            "the wrong tag is the BDC's class, and its EMC balances it"
        );
        assert_eq!(
            ended(&["BMC", "BX"]),
            [
                repair::MARKED_CONTENT_UNBALANCED.name,
                repair::COMPATIBILITY_SECTIONS_UNBALANCED.name
            ]
        );
        assert_eq!(ended(&["BMC", "EMC", "BX", "EX"]), Vec::<&str>::new());
    }

    /// §7.8.2's "operands shall not be left over": the interpreter gives the operator the last of
    /// them and drops the rest, and the grammar says so beside any other class (ADR 3629).
    #[test]
    fn operands_in_front_of_an_operators_own_are_left_over() {
        assert_eq!(
            run(&["1", "2", "3", "m"]),
            [repair::OPERANDS_LEFT_OVER.name]
        );
        assert_eq!(
            run(&["BT", "/F0", "36.", "(a)", "Tj", "ET"]),
            [repair::OPERANDS_LEFT_OVER.name],
            "T02-05-01_008's show, whose name and size stand before its string"
        );
        assert_eq!(
            run(&["(a)", "1", "2", "m"]),
            [repair::OPERANDS_LEFT_OVER.name],
            "the operator's own are the last two, and they are numbers"
        );
        assert_eq!(
            run(&["/a", "(b)", "2", "m"]),
            [
                repair::OPERAND_OF_THE_WRONG_TYPE.name,
                repair::OPERANDS_LEFT_OVER.name
            ]
        );
        assert_eq!(run(&["1", "2", "m", "3", "4", "l"]), [""; 0]);
    }

    /// §7.3.6: a keyword inside an array is no operator, and the interpreter keeps the array's
    /// operands for the operator after it (`operator-in-TJ-array.pdf`).
    #[test]
    fn a_keyword_inside_an_array_is_stepped_over() {
        assert_eq!(
            run(&["BT", "[", "(a)", "0", "Tc", "(b)", "]", "TJ", "ET"]),
            [""; 0]
        );
        assert_eq!(
            run(&["[", "1", "]", "0", "0", "m"]),
            [repair::OPERANDS_LEFT_OVER.name],
            "an array in front of a fixed list is left over, counted as the interpreter counts it"
        );
    }

    /// §8.6.8: after `d1`, and in a stream the walk marks uncoloured, a colour operator is ignored
    /// with its operands, so nothing is answered at it.
    #[test]
    fn colour_operators_are_ignored_where_the_glyph_or_pattern_is_uncoloured() {
        let glyph = [
            "0", "0", "0", "0", "1", "1", "d1", "1", "0", "g", "1", "0", "0", "sc",
        ];
        assert_eq!(run(&glyph), [""; 0]);
        let mut grammar = Grammar::default();
        grammar.ignore_colour();
        grammar.operand(Operand::Number);
        grammar.operand(Operand::Number);
        let rg = signature(b"rg").expect("Table 50 lists rg");
        assert!(grammar.operator(rg).is_empty());
        let shaded = signature(b"sh").expect("Table 50 lists sh");
        assert!(grammar.operator(shaded).is_empty());
        assert!(grammar.ignores_colour());
        assert_eq!(
            run(&["0", "0", "d0", "1", "0", "g"]),
            [repair::OPERANDS_LEFT_OVER.name],
            "a d0 glyph sets its own colour, and its operators are judged"
        );
    }

    /// §14.6.2: a `BDC` naming its property list is answered by the walk's lookup, and one
    /// written inline or a `BMC` is not asked (ADR 3629).
    #[test]
    fn a_bdc_naming_its_property_list_asks_the_walk() {
        let asked = |operands: &[Operand], word: &[u8]| {
            let mut grammar = Grammar::default();
            for operand in operands {
                grammar.operand(*operand);
            }
            let _ = grammar.operator(signature(word).expect("Table 50 lists it"));
            (
                grammar.names_a_property_list(),
                grammar.property_list(false).map(|class| class.name),
                grammar.property_list(true),
            )
        };
        let named = asked(&[Operand::Name, Operand::Name], b"BDC");
        assert!(named.0);
        assert_eq!(named.1, Some(repair::PROPERTY_LIST_UNRESOLVED.name));
        assert!(named.2.is_none(), "a list the resources hold is no repair");
        assert!(
            asked(&[Operand::Number, Operand::Name], b"BDC").0,
            "issue17679_2's `123 /oc1 BDC` names a list too"
        );
        assert!(!asked(&[Operand::Name, Operand::Dictionary], b"BDC").0);
        assert!(!asked(&[Operand::Name], b"BMC").0);
        assert!(!asked(&[Operand::Name, Operand::Name], b"DP").0);
    }

    /// Each class the table names once, each a declared class, and each interpreter's class with
    /// its reason (ADR 3630).
    #[test]
    fn the_coverage_names_each_declared_class_once() {
        let mut seen = std::collections::BTreeSet::new();
        for (class, reading) in COVERAGE {
            assert!(seen.insert(class.name), "{} twice", class.name);
            assert!(
                repair::CLASSES
                    .iter()
                    .any(|declared| declared.name == class.name)
                    || pdf_syntax::repair::CLASSES
                        .iter()
                        .any(|declared| declared.name == class.name),
                "{} is no declared class",
                class.name
            );
            if let Reading::Interpreter(reason) = reading {
                assert!(!reason.is_empty(), "{} states no reason", class.name);
            }
        }
    }
}
