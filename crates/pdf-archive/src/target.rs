//! Which part, level and flavour of ISO 19005 a document is being held to.
//!
//! Two owned parts, six targets, and the shape of that set is the reason this crate has one
//! requirement table rather than six: **a level is an applicability column, not an
//! implementation.** ISO 19005-2 section 5.3 and section 5.4 say so themselves — Level B may ignore
//! the Unicode subclause and Level A's logical structure, Level U may ignore the latter — and ISO
//! 19005-4's Annexes A and B are modifications to clause 6 that mostly *relax* it. So the
//! difference between `2b` and `2a` is which requirements apply, and every predicate is shared.
//!
//! # Why parts 1 and 3 are not here
//!
//! `CLAUDE.md` principle 5: a requirement may not be implemented from somebody else's reading
//! of a text this project does not have. ISO 19005-1 and -3 are not in `doc/pdfa/`
//! (`doc/questions/A16`, `A17`: part 1 never, part 3 not bought), so they are not targets and
//! adding them is a purchase rather than a patch.

use core::fmt;

/// A part of ISO 19005 this crate can hold a document to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Part {
    /// ISO 19005-2:2011, PDF/A-2, defined on ISO 32000-1.
    ///
    /// **Its base document is a different edition from the one this tree is written against**,
    /// which is a fact about every PDF/A-2 verdict rather than about any one requirement: section
    /// 5.1 makes a conforming file one that adheres to all of ISO 32000-1 as modified by part 2,
    /// while `doc/md/`, the conformance ledger and every doc comment in `crates/` are ISO
    /// 32000-2's.
    ///
    /// **The edition itself is now readable.** The owner obtained ISO 32000-1:2008 on
    /// 2026-09-07 — Adobe publishes it without charge — and it is `doc/PDF32000_2008.pdf`,
    /// which this tree's own reader opens. So a rule that turns on a difference between the
    /// editions can now be *written* against the right one; what remains is the work of going
    /// through the part 2 rows and saying which have been. Until a row says so, reading its
    /// requirement in the later edition is what this crate did, and where the editions agree —
    /// most of the file format — that gives the same answer. `doc/questions/Q49` tracks what is
    /// left of it, which is no longer a purchase.
    Two,
    /// ISO 19005-4:2020, PDF/A-4, defined on ISO 32000-2.
    ///
    /// The part whose base document *is* the edition this tree owns, cites and implements,
    /// which is why it is the one finished first.
    Four,
}

/// ISO 19005-2 section 5's conformance levels.
///
/// The order is the standard's own containment: every Level A file is a Level U file and every
/// Level U file is a Level B file, because section 5.2 and section 5.4 define A and U by *adding*
/// to B rather than by replacing it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Level {
    /// Section 5.3's Level B: every requirement except section 6.2.11.7's Unicode maps and section
    /// 6.7's structure.
    B,
    /// Section 5.4's Level U: every requirement except section 6.7's structure.
    U,
    /// Section 5.2's Level A: every requirement of the part.
    A,
}

/// ISO 19005-4's conformance flavours, which its Annexes A and B define.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Flavour {
    /// Clause 6 as written: no embedded files of arbitrary type, no 3D.
    Plain,
    /// Annex A's PDF/A-4f, where an embedded file may be of any type.
    F,
    /// Annex B's PDF/A-4e, the engineering profile.
    ///
    /// **Held but never fully judged.** Annex B admits 3D and `RichMedia`, which is
    /// `CLAUDE.md`'s clause-13 exclusion by name — so a `4e` verdict reports every requirement
    /// outside Annex B's 3D subclauses and reports those as not checked, which is
    /// `doc/questions/Q20`'s discipline rather than an exception to it.
    E,
}

/// One conformance target: a part, and the level or flavour that narrows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Target {
    /// PDF/A-2 at one of §5's three levels.
    Two(Level),
    /// PDF/A-4 in one of its three flavours.
    Four(Flavour),
}

impl Target {
    /// Every target this crate knows, in the order a report lists them.
    pub const ALL: [Self; 6] = [
        Self::Two(Level::B),
        Self::Two(Level::U),
        Self::Two(Level::A),
        Self::Four(Flavour::Plain),
        Self::Four(Flavour::F),
        Self::Four(Flavour::E),
    ];

    /// Which part of ISO 19005 this target belongs to.
    #[must_use]
    pub const fn part(self) -> Part {
        match self {
            Self::Two(_) => Part::Two,
            Self::Four(_) => Part::Four,
        }
    }

    /// The level, for a part 2 target.
    #[must_use]
    pub const fn level(self) -> Option<Level> {
        match self {
            Self::Two(level) => Some(level),
            Self::Four(_) => None,
        }
    }

    /// The flavour, for a part 4 target.
    #[must_use]
    pub const fn flavour(self) -> Option<Flavour> {
        match self {
            Self::Two(_) => None,
            Self::Four(flavour) => Some(flavour),
        }
    }

    /// The target a name like `2b`, `2u`, `2a`, `4`, `4f` or `4e` asks for.
    ///
    /// Case-insensitive, and `pdfa-2b` and `pdf/a-2b` are accepted for the same reason a
    /// command line takes them: a user writing the name of a standard writes it the way the
    /// standard is spelled rather than the way an enum is.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        let name = name.trim().to_ascii_lowercase();
        let name = name
            .strip_prefix("pdf/a-")
            .or_else(|| name.strip_prefix("pdfa-"))
            .or_else(|| name.strip_prefix("pdf-a-"))
            .unwrap_or(&name);
        match name {
            "2b" => Some(Self::Two(Level::B)),
            "2u" => Some(Self::Two(Level::U)),
            "2a" => Some(Self::Two(Level::A)),
            "4" => Some(Self::Four(Flavour::Plain)),
            "4f" => Some(Self::Four(Flavour::F)),
            "4e" => Some(Self::Four(Flavour::E)),
            _ => None,
        }
    }

    /// The `pdfaid:part` value a conforming file states for this target.
    ///
    /// ISO 19005-2 section 6.6.4 and ISO 19005-4 section 6.7.3 both require it to be the part
    /// number.
    #[must_use]
    pub const fn identification_part(self) -> u8 {
        match self.part() {
            Part::Two => 2,
            Part::Four => 4,
        }
    }
}

impl fmt::Display for Target {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Two(Level::B) => out.write_str("PDF/A-2b"),
            Self::Two(Level::U) => out.write_str("PDF/A-2u"),
            Self::Two(Level::A) => out.write_str("PDF/A-2a"),
            Self::Four(Flavour::Plain) => out.write_str("PDF/A-4"),
            Self::Four(Flavour::F) => out.write_str("PDF/A-4f"),
            Self::Four(Flavour::E) => out.write_str("PDF/A-4e"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Flavour, Level, Part, Target};

    #[test]
    fn every_target_round_trips_through_its_own_name() {
        for target in Target::ALL {
            let name = target.to_string();
            assert_eq!(Target::parse(&name), Some(target), "{name}");
        }
    }

    #[test]
    fn a_name_is_taken_however_the_standard_is_spelled() {
        for spelling in ["2b", "2B", "PDF/A-2b", "pdfa-2b", " pdf-a-2b "] {
            assert_eq!(
                Target::parse(spelling),
                Some(Target::Two(Level::B)),
                "{spelling}"
            );
        }
        assert_eq!(
            Target::parse("3b"),
            None,
            "part 3 is not owned and is not a target"
        );
        assert_eq!(Target::parse("1b"), None, "part 1 never");
    }

    #[test]
    fn a_part_two_target_has_a_level_and_a_part_four_target_has_a_flavour() {
        assert_eq!(Target::Two(Level::A).part(), Part::Two);
        assert_eq!(Target::Two(Level::A).level(), Some(Level::A));
        assert_eq!(Target::Two(Level::A).flavour(), None);
        assert_eq!(Target::Four(Flavour::F).part(), Part::Four);
        assert_eq!(Target::Four(Flavour::F).flavour(), Some(Flavour::F));
        assert_eq!(Target::Four(Flavour::F).level(), None);
    }
}
