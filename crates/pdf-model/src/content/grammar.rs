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
//! the walk the validator already makes. Every one of the five is a function of the tokens, the
//! `q`/`Q` and `BT`/`ET` nesting, and the number of components of the space a `cs` selected, so
//! a walk that already reads the tokens can ask them here, of the same table the interpreter's
//! operand counts are held to (`operand_counts_are_the_grammars`).
//!
//! The five, each under its clause:
//!
//! - §7.8.2: "all of the operands needed by an operator shall immediately precede that
//!   operator" — an operator short of its table's count, or given an operand of a type its table
//!   does not state;
//! - §8.4.2: "Occurrences of the q and Q operators shall be balanced within a given content
//!   stream" — a `Q` with nothing saved;
//! - §9.4.1, Figure 9: text-showing and text-positioning operators appear inside a text object;
//! - §8.6.8, Table 73: `sc` takes as many operands as the current colour space has components.

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
    /// Anything else: an array bracket, a boolean, a null.
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
    /// Everything else.
    Plain,
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
    let plain = |wants: &'static [Wants]| Signature {
        wants: Some(wants),
        role: Role::Plain,
    };
    let text = |wants: &'static [Wants]| Signature {
        wants: Some(wants),
        role: Role::InText,
    };
    let device = |stroking, wants: &'static [Wants]| Signature {
        wants: Some(wants),
        role: Role::DeviceColour {
            stroking,
            components: wants.len(),
        },
    };
    Some(match word {
        b"q" => Signature {
            wants: Some(NONE),
            role: Role::Save,
        },
        b"Q" => Signature {
            wants: Some(NONE),
            role: Role::Restore,
        },
        b"BT" => Signature {
            wants: Some(NONE),
            role: Role::BeginText,
        },
        b"ET" => Signature {
            wants: Some(NONE),
            role: Role::EndText,
        },
        b"h" | b"n" | b"f" | b"F" | b"f*" | b"S" | b"s" | b"B" | b"B*" | b"b" | b"b*" | b"W"
        | b"W*" | b"EMC" | b"BX" | b"EX" | b"BI" => plain(NONE),
        b"T*" => text(NONE),
        b"w" | b"J" | b"j" | b"M" | b"i" | b"Tc" | b"Tw" | b"Tz" | b"TL" | b"Ts" | b"Tr" => {
            plain(ONE_NUMBER)
        }
        b"ri" | b"gs" | b"sh" | b"Do" | b"MP" | b"BMC" => plain(NAME),
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
        b"DP" | b"BDC" => plain(&[Wants::Name, Wants::Properties]),
        b"v" | b"y" | b"re" => plain(FOUR_NUMBERS),
        b"cm" | b"c" | b"d1" => plain(SIX_NUMBERS),
        b"CS" | b"cs" => Signature {
            wants: Some(NAME),
            role: Role::SelectSpace {
                stroking: word == b"CS",
            },
        },
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

/// The five checks over one content stream's tokens, in the order the stream states them.
///
/// Fed by [`Self::operand`] for every operand token and [`Self::operator`] for every operator, it
/// answers each operator with the class of repair an interpreter would have made there, or none.
/// A stream starts in §8.4.1's initial state — `DeviceGray`, one component, for both colours — and
/// its own `q` and `Q` decide what a `Q` restores; a form `XObject` is a stream of its own.
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
}

impl Default for Grammar {
    fn default() -> Self {
        Self {
            operands: [Operand::Other; KEPT],
            given: 0,
            numbers: 0,
            saved: Vec::new(),
            stroke: Some(1),
            fill: Some(1),
            in_text: false,
        }
    }
}

/// How deep the saved spaces may grow; past it a `q` saves nothing more and a `Q` is not judged,
/// so a hostile stream of `q`s costs a bound rather than memory.
const MAX_SAVED: usize = 1024;

impl Grammar {
    /// One operand token.
    pub fn operand(&mut self, operand: Operand) {
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
    pub fn boundary(&mut self) {
        self.given = 0;
        self.numbers = 0;
    }

    /// The operator `signature` names, after its operands: the class of the repair it needs, if
    /// any. Where it needs two, the operand's is answered, since that is the one that decides
    /// whether the operator runs at all.
    pub fn operator(&mut self, signature: Signature) -> Option<&'static RepairClass> {
        let found = self
            .operands_found(signature)
            .or_else(|| self.state_found(signature));
        self.boundary();
        found
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
            Role::Plain => None,
        }
    }
}

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
    use super::{Grammar, Operand, signature};
    use crate::repair;

    fn run(tokens: &[&str]) -> Vec<&'static str> {
        let mut grammar = Grammar::default();
        let mut found = Vec::new();
        for token in tokens {
            let operand = match token.as_bytes().first() {
                Some(b'0'..=b'9' | b'-' | b'.') => Some(Operand::Number),
                Some(b'/') => Some(Operand::Name),
                Some(b'(') => Some(Operand::Text),
                _ => None,
            };
            if let Some(operand) = operand {
                grammar.operand(operand);
            } else if let Some(signature) = signature(token.as_bytes()) {
                found.extend(grammar.operator(signature).map(|class| class.name));
            } else {
                grammar.boundary();
            }
        }
        found
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
}
