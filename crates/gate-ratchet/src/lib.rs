//! Every hand-written bound a gate holds, printed beside the population it bounds.
//!
//! A ratchet is a number written by hand into a gate: a **ceiling** a population may only fall
//! below, or a **floor** it may only rise above. The population comes off the run; the bound comes
//! off the source. This crate is the one place the two meet, so that the distance between them —
//! the *slack* — is on every run's output instead of being a thing somebody could go and work out.
//!
//! # The defect it exists for
//!
//! A ceiling far above its population is not a ratchet. It is a gate that cannot fire, and it
//! looks exactly like one that can. Twice in one run of sessions a bound was found sitting well
//! clear of what it bounds: `MAX_INCOMPLETE` at 91 against a population of 61, and `MAX_PAGELESS`
//! at 6 against 5 — thirty documents' worth of silent regression admitted by the first. Neither
//! was hidden. Both gates print their population on every run; neither printed its bound, so the
//! two numbers never appeared on one line and putting them there was nobody's job.
//!
//! The second half of that is why this is a crate rather than a note. The check was then done by
//! hand over one file's remaining four bounds, and a check done by hand is done once. Every call
//! here does it on every run.
//!
//! # What it asserts, in order
//!
//! 1. The line is **printed first**, so the population and the bound are on the run's output even
//!    when what follows fails.
//! 2. The **bound holds** — the population is under a ceiling, or over a floor. This is the
//!    assertion the gate already made, and nothing here weakens it.
//! 3. The **slack is within what the bound's own comment allows**. [`ceiling`] and [`floor`] allow
//!    none: the bound is the population, and a round that moves the population moves the bound in
//!    the same commit with its reason. [`ceiling_with_headroom`] and [`floor_with_headroom`] take
//!    an argued distance and the argument for it, and print both.
//!
//! # When headroom is the right answer
//!
//! Zero slack is right for a population this tree alone decides — how many corpus documents draw
//! incompletely is a fact about this program and a pinned submodule, and it moves only when
//! somebody changes one of them. It is wrong for a population another program's version decides: a
//! floor on how many documents a *reference* renderer agreed about will move when that renderer is
//! upgraded, and a gate that fails on an upgrade is a gate that gets loosened in a hurry. Those
//! take headroom, and the reason goes beside the bound where the next round reads it.
//!
//! Headroom is never the answer to "the run is inconvenient". A bound whose reason is that nobody
//! re-measured is the defect above, wearing a justification.

#![forbid(unsafe_code)]

/// Which way a bound may move, and therefore which side of it the slack is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Direction {
    /// The population may only fall. Slack is the room above it.
    Ceiling,
    /// The population may only rise. Slack is the room below it.
    Floor,
}

impl Direction {
    /// The word printed between the population and the bound.
    fn word(self) -> &'static str {
        match self {
            Self::Ceiling => "ceiling",
            Self::Floor => "floor",
        }
    }

    /// Whether `population` is on the side of `bound` this direction requires.
    fn holds(self, population: usize, bound: usize) -> bool {
        match self {
            Self::Ceiling => population <= bound,
            Self::Floor => population >= bound,
        }
    }

    /// How far the population sits inside the bound, or zero when the bound does not hold.
    ///
    /// Saturating rather than checked because the breach is reported by [`Direction::holds`] and
    /// a negative distance is not a thing to print.
    fn slack(self, population: usize, bound: usize) -> usize {
        match self {
            Self::Ceiling => bound.saturating_sub(population),
            Self::Floor => population.saturating_sub(bound),
        }
    }

    /// What a breach of this direction reads as.
    fn breach(self) -> &'static str {
        match self {
            Self::Ceiling => "above the ceiling",
            Self::Floor => "below the floor",
        }
    }
}

/// Prints the line, then holds the bound, then holds the slack.
///
/// One body for all four entry points so that the printed shape and the order of the two
/// assertions are stated once.
fn hold(
    direction: Direction,
    what: &str,
    population: usize,
    bound: usize,
    headroom: usize,
    why: Option<&str>,
) {
    let slack = direction.slack(population, bound);
    match why {
        None => println!(
            "ratchet: {what}: {population}, {} {bound}, slack {slack}",
            direction.word()
        ),
        Some(why) => println!(
            "ratchet: {what}: {population}, {} {bound}, slack {slack} of {headroom} allowed — \
             {why}",
            direction.word()
        ),
    }
    assert!(
        direction.holds(population, bound),
        "{what}: {population}, {} of {bound} this gate holds",
        direction.breach()
    );
    assert!(
        slack <= headroom,
        "{what}: the bound is {bound} and the population is {population}, so this ratchet carries \
         {slack} of slack where {headroom} is allowed — which is that much regression it would \
         admit without speaking. Move the bound to {population}, with the reason above the \
         constant; or, if the distance is argued, say so beside the bound and pass it here."
    );
}

/// A population that may only fall, whose bound is the population.
///
/// # Panics
///
/// When the population is above the bound, and when the bound is above the population at all —
/// see the crate comment for why the second is a defect and not tidiness.
pub fn ceiling(what: &str, population: usize, bound: usize) {
    hold(Direction::Ceiling, what, population, bound, 0, None);
}

/// A population that may only fall, whose bound is deliberately above it by `headroom`.
///
/// `why` is the argument for that distance and is printed on every run beside it. It belongs in
/// the source next to the bound as well; this argument is what puts it in front of a reader who
/// only has the run's output.
///
/// # Panics
///
/// When the population is above the bound, or further below it than `headroom`.
pub fn ceiling_with_headroom(
    what: &str,
    population: usize,
    bound: usize,
    headroom: usize,
    why: &str,
) {
    hold(
        Direction::Ceiling,
        what,
        population,
        bound,
        headroom,
        Some(why),
    );
}

/// A population that may only rise, whose bound is the population.
///
/// # Panics
///
/// When the population is below the bound, and when the bound is below the population at all.
pub fn floor(what: &str, population: usize, bound: usize) {
    hold(Direction::Floor, what, population, bound, 0, None);
}

/// A population that may only rise, whose bound is deliberately below it by `headroom`.
///
/// # Panics
///
/// When the population is below the bound, or further above it than `headroom`.
pub fn floor_with_headroom(
    what: &str,
    population: usize,
    bound: usize,
    headroom: usize,
    why: &str,
) {
    hold(
        Direction::Floor,
        what,
        population,
        bound,
        headroom,
        Some(why),
    );
}

#[cfg(test)]
mod tests {
    use super::{ceiling, ceiling_with_headroom, floor, floor_with_headroom};

    /// A bound that sits on its population is what a ratchet is, in both directions.
    #[test]
    fn a_bound_on_its_population_holds() {
        ceiling("documents that draw incompletely", 61, 61);
        floor("date strings that conform", 1514, 1514);
    }

    /// Trap 13: the assertion above has to be shown failing, or it is a sentence about nothing.
    ///
    /// This is the defect planted back — the exact shape `MAX_INCOMPLETE` had, a ceiling thirty
    /// above the population it bounds — and the instrument must name it.
    #[test]
    #[should_panic(expected = "30 of slack where 0 is allowed")]
    fn a_ceiling_above_its_population_is_named() {
        ceiling("documents that draw incompletely", 61, 91);
    }

    /// And the same the other way: a floor below its population admits the difference.
    #[test]
    #[should_panic(expected = "5 of slack where 0 is allowed")]
    fn a_floor_below_its_population_is_named() {
        floor("date strings that conform", 1519, 1514);
    }

    /// The bound itself still fails first, which is the assertion that was already there.
    #[test]
    #[should_panic(expected = "above the ceiling of 61")]
    fn a_population_over_its_ceiling_still_fails() {
        ceiling("documents that draw incompletely", 62, 61);
    }

    /// A breach of a floor reads as itself rather than as slack.
    #[test]
    #[should_panic(expected = "below the floor of 1514")]
    fn a_population_under_its_floor_still_fails() {
        floor("date strings that conform", 1513, 1514);
    }

    /// Argued headroom admits exactly what it says and not one more.
    #[test]
    fn headroom_admits_what_it_states() {
        ceiling_with_headroom(
            "pages a reference declines",
            8,
            10,
            2,
            "a reference upgrade",
        );
        floor_with_headroom(
            "pages a reference agreed on",
            10,
            8,
            2,
            "a reference upgrade",
        );
    }

    /// And a distance past the headroom is still named, so the escape is not a door.
    #[test]
    #[should_panic(expected = "3 of slack where 2 is allowed")]
    fn headroom_does_not_admit_more_than_it_states() {
        ceiling_with_headroom(
            "pages a reference declines",
            7,
            10,
            2,
            "a reference upgrade",
        );
    }
}
