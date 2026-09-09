//! One requirement of ISO 19005, as a row a person can review.
//!
//! # Why the requirements are data
//!
//! There are a few hundred of them across the two owned parts, most are shared between them,
//! and every one has to carry its clause so that a verdict is a citation rather than an
//! opinion. A table of rows gives three things a tree of `if`s does not: the shared ones are
//! written once and *cited* twice; the applicability of a level is a field rather than a branch
//! (`doc/questions/Q46`); and the set can be counted, listed and reviewed against the standard
//! by somebody who is not reading Rust.
//!
//! # The two states a row can be in, and why the second one is not a gap
//!
//! A row's [`Requirement::check`] is either a predicate or a named absence. `CLAUDE.md`
//! principle 5 and `doc/questions/Q20` both land on the same discipline: a requirement this
//! crate does not check is reported **by name**, in the verdict, with the reason — never
//! silently omitted, and never implemented from a secondary source. A validator that omits a
//! check silently is indistinguishable from a document that passes it.

use crate::finding::Findings;
use crate::target::{Flavour, Level, Part, Target};

/// Where a requirement is written, in each part that states it.
///
/// Two clause numbers rather than one, because the parts number the same rule differently — font
/// embedding is ISO 19005-2 section 6.2.11.4.1 and ISO 19005-4 section 6.2.10.4.1 — and a verdict
/// that cited the wrong part's numbering would be unusable to the person checking it against their
/// own copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Clauses {
    /// The clause in ISO 19005-2:2011, where that part states this requirement.
    pub two: Option<&'static str>,
    /// The clause in ISO 19005-4:2020, where that part states this requirement.
    pub four: Option<&'static str>,
}

impl Clauses {
    /// A requirement both parts state, at the clause numbers each uses.
    #[must_use]
    pub const fn both(two: &'static str, four: &'static str) -> Self {
        Self {
            two: Some(two),
            four: Some(four),
        }
    }

    /// A requirement only ISO 19005-2 states.
    #[must_use]
    pub const fn only_two(two: &'static str) -> Self {
        Self {
            two: Some(two),
            four: None,
        }
    }

    /// A requirement only ISO 19005-4 states.
    #[must_use]
    pub const fn only_four(four: &'static str) -> Self {
        Self {
            two: None,
            four: Some(four),
        }
    }

    /// How this requirement is cited for a given target, as `ISO 19005-2 section 6.1.6`.
    #[must_use]
    pub fn citation(self, target: Target) -> Option<String> {
        match target.part() {
            Part::Two => self
                .two
                .map(|clause| format!("ISO 19005-2 section {clause}")),
            Part::Four => self
                .four
                .map(|clause| format!("ISO 19005-4 section {clause}")),
        }
    }
}

/// Which targets a requirement applies to, beyond the parts that state it.
///
/// [`Clauses`] already says which *parts* state a rule; this says which levels and flavours
/// within them it reaches. The three variants are the only shapes the two standards use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Applies {
    /// To every target of every part that states it.
    Always,
    /// To ISO 19005-2 at this level and above, and to every ISO 19005-4 target.
    ///
    /// Section 5.3 lets a Level B file ignore section 6.2.11.7 and section 6.7; section 5.4 lets a
    /// Level U file ignore section 6.7. Both are stated as exemptions from the whole part, so a
    /// requirement inside those subclauses carries the level it starts applying at.
    FromLevel(Level),
    /// Only to the ISO 19005-4 flavours listed.
    ///
    /// Annexes A and B modify clause 6 for `4f` and `4e`, almost always by permitting
    /// something the plain profile forbids — so the *plain* rule is the one that carries this,
    /// naming the flavours it still binds.
    Flavours(&'static [Flavour]),
}

impl Applies {
    /// Whether the requirement binds this target.
    #[must_use]
    pub fn binds(self, target: Target) -> bool {
        match (self, target) {
            // `FromLevel` names a level of *part 2*: section 5.3 and section 5.4 are that part's
            // exemptions and ISO 19005-4 has no levels to be exempted at, so a part 4 target is
            // bound by such a requirement exactly as `Always` binds it.
            (Self::Always | Self::FromLevel(_), Target::Four(_))
            | (Self::Always, Target::Two(_)) => true,
            (Self::FromLevel(from), Target::Two(level)) => level >= from,
            (Self::Flavours(list), Target::Four(flavour)) => list.contains(&flavour),
            (Self::Flavours(_), Target::Two(_)) => false,
        }
    }
}

/// What this crate does about a requirement when it is asked to check one.
#[derive(Clone, Copy)]
pub enum Check {
    /// A predicate over the document, which records what it finds.
    ///
    /// It reports rather than returns: one requirement can fail in several places, and a
    /// verdict that said only "failed" would send the reader back to the document to find out
    /// where.
    Implemented(fn(&crate::Examination<'_>, &mut Findings)),
    /// The requirement binds a **conforming processor**, not a conforming file.
    ///
    /// ISO 19005 states both kinds in the same clauses: "[t]he Encrypt key shall not be present
    /// in the trailer dictionary" is about a document, and "[c]onforming readers shall ignore
    /// the BG, BG2, UCR and UCR2 functions" is about a program. **No document can fail the
    /// second kind**, so reporting it beside a file's failures — "your processor must ignore
    /// `/Dur`" — tells a reader nothing about the file in front of them.
    ///
    /// It is not `Unchecked` either, and the difference matters: `Unchecked` is a debt this crate
    /// owes, and these are not owed by a validator at all. They are owed by *this project*, if it
    /// claims to be a conforming processor, and `doc/PLAN.md` section 5a's conformance ledger is
    /// where a claim about this program's own behaviour belongs. Counting them among a document's
    /// unchecked requirements overstated the gap by thirteen rows on PDF/A-4.
    ///
    /// So they are carried, named and reported in their own section — visible, and not mistaken
    /// for either a pass or a debt.
    Processor(&'static str),
    /// Not checked, and the reason — `doc/questions/Q20`'s discipline.
    ///
    /// The reason is prose for a person, and it is a promise about *why*: that the requirement
    /// is unimplemented, or that its clause is in a base document this tree does not carry
    /// (`Part::Two`'s note), or that judging it is outside `CLAUDE.md`'s scope. It never says
    /// "not applicable" — that is [`Applies`]' answer, and confusing the two is how a validator
    /// comes to report a clean pass over a rule it never looked at.
    Unchecked(&'static str),
}

impl core::fmt::Debug for Check {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Implemented(_) => out.write_str("Implemented(..)"),
            Self::Processor(why) => out.debug_tuple("Processor").field(why).finish(),
            Self::Unchecked(why) => out.debug_tuple("Unchecked").field(why).finish(),
        }
    }
}

/// One requirement of ISO 19005, with its clause, its applicability and its check.
#[derive(Debug, Clone, Copy)]
pub struct Requirement {
    /// A stable identifier, unique across the table.
    ///
    /// The shape is `area/rule` — `file-structure/no-lzw-filter` — rather than a clause
    /// number, because the two parts number the same rule differently and an identifier that
    /// picked one part's numbering would read as a claim that the other part's rule is a
    /// different requirement. The clause numbers are [`Requirement::clauses`]'s job.
    pub id: &'static str,
    /// A single sentence saying what the requirement asks, in this crate's own words.
    ///
    /// **Deliberately not the standard's words.** ISO 19005-2 and -4 are licensed to a single
    /// reader (`doc/questions/A16`), so this tree cites their clauses and paraphrases their
    /// rules; `doc/pdf-a-conversion-limits.md` says the same and why. A reader who needs the
    /// sentence itself opens `doc/pdfa/` at the clause the row names.
    pub asks: &'static str,
    /// Where each part states it.
    pub clauses: Clauses,
    /// Which targets it binds, beyond the parts that state it.
    pub applies: Applies,
    /// The predicate, or the named absence of one.
    pub check: Check,
}

impl Requirement {
    /// Whether this requirement binds the given target.
    ///
    /// Both halves have to hold: the target's part must state the rule, and [`Applies`] must
    /// reach the level or flavour.
    #[must_use]
    pub fn binds(&self, target: Target) -> bool {
        let stated = match target.part() {
            Part::Two => self.clauses.two.is_some(),
            Part::Four => self.clauses.four.is_some(),
        };
        stated && self.applies.binds(target)
    }
}
