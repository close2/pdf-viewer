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
//! # And where the members have names, the names are the bound
//!
//! [`population`] is the fifth entry point and the strongest of them. A count of ten cannot tell a
//! document that *started* needing a password from one that *stopped*, and both are findings; a
//! list of ten names tells them apart and says which. It prints the same line — the count beside
//! the length of the list — so the table stays whole, and then holds the two sets equal in both
//! directions. Use it wherever the gate already knows its members by name.
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

use std::collections::BTreeSet;

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

/// Prints the one line every entry point here puts on the run's output.
///
/// Separated from [`hold`] so that [`population`] prints the same shape before making an
/// assertion of its own: the printing comes first everywhere, so the two numbers are on the
/// output even when what follows fails.
fn line(
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
    line(direction, what, population, bound, headroom, why);
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

/// A population held as the **names** of its members, with the count printed beside them.
///
/// The strongest shape a ratchet has, and the one a count cannot reach. A ceiling of ten on the
/// documents that need a password says nothing when one document starts needing one in the same
/// run as another stops: the count is ten both times, and both are findings. So the bound is the
/// list, how many names it holds is what goes on the table beside the population, and the
/// assertion is set equality in both directions — a name that joined, and a name that left, each reported as
/// itself. `crates/pdf-model/tests/save_round_trip.rs` and
/// `crates/viewer-core/tests/accessibility_census.rs` are where the shape came from; this is it
/// with the printed line the other four entry points put on every run.
///
/// The list carries the reason each member is in it, beside its name in the source. That is not
/// bookkeeping: a document leaves this population when somebody fixes something, and the round
/// that deletes the name is the round that has to read what the name was for.
///
/// # Panics
///
/// When the found set and the named set differ, naming the difference in both directions.
pub fn population(what: &str, found: impl IntoIterator<Item = String>, named: &[&str]) {
    let found: BTreeSet<String> = found.into_iter().collect();
    let expected: BTreeSet<&str> = named.iter().copied().collect();
    line(
        Direction::Ceiling,
        what,
        found.len(),
        expected.len(),
        0,
        None,
    );
    let joined: Vec<&str> = found
        .iter()
        .map(String::as_str)
        .filter(|name| !expected.contains(name))
        .collect();
    let left: Vec<&str> = expected
        .iter()
        .copied()
        .filter(|name| !found.contains(*name))
        .collect();
    assert!(
        joined.is_empty() && left.is_empty(),
        "{what}: the population moved — joined {joined:?}, left {left:?}. Both directions are \
         findings: a name that joined is a document that started needing this, and a name that \
         left is one that stopped. Read each, then edit the list with the reason beside the name, \
         in that order."
    );
}

/// How far `value` sits from the nearer edge of `low .. high`, in the band's own unit and as a
/// share of its width.
///
/// The share is what makes two figures in different units comparable at a glance: 50% is the
/// middle of the band, 0% is the edge, and a figure creeping toward one is a falling percentage
/// over consecutive runs.
fn margin(value: f64, low: f64, high: f64) -> String {
    let (edge, distance) = if value - low <= high - value {
        ("low", value - low)
    } else {
        ("high", high - value)
    };
    if distance < 0.0 {
        return format!("{:.3} past the {edge} edge", -distance);
    }
    let width = high - low;
    if width <= 0.0 {
        return format!("{distance:.3} from the {edge} edge");
    }
    format!(
        "{distance:.3} from the {edge} edge ({:.0}% of the band)",
        distance / width * 100.0
    )
}

/// A figure printed beside the two-sided band it is held to, and how far it is from the nearer
/// edge.
///
/// **A band is a bound, and until this existed it was printed only when a figure had already
/// crossed it.** `doc/checks/launch-path.toml`'s clock and memory bands and
/// `doc/checks/fixed-documents.toml`'s ink bands are both of that shape, so a figure creeping
/// toward an edge over ten rounds was invisible for all ten — the defect ADR 1075 was written
/// about, one shape along.
///
/// **It prints and does not assert**, which is the difference between a band and the four
/// one-sided bounds above, and it is deliberate rather than a gap. A band's verdict has conditions
/// this crate has no business knowing: the launch path declines a figure whose child's probe says
/// the machine was busy, and the fixed-documents check pins a *refusal* on some rows and nothing
/// at all on others. Those verdicts stay in the gates, where the reasons are; what belongs here is
/// the one line that puts the figure, the band and the distance on the run's output.
///
/// **A share of 0% is not always a warning, and trap 39 is why that is said here rather than
/// discovered.** Where an edge is a wall the quantity cannot cross — eight rows of
/// `doc/checks/fixed-documents.toml` pin a page that is honestly blank as `ink = 0.0 .. 1.0`, and
/// ink is a mean of `255 - luma` with nowhere below zero to go — the figure sits *on* the low edge
/// on every run, by design, and the bound that can actually fire is the other one. A falling share
/// is the signal; a constant one is a fact about the row.
pub fn band(what: &str, value: f64, low: f64, high: f64) {
    println!(
        "ratchet: {what}: {value:.3}, band {low:.3} .. {high:.3}, {}",
        margin(value, low, high)
    );
}

#[cfg(test)]
mod tests {
    use super::{
        band, ceiling, ceiling_with_headroom, floor, floor_with_headroom, margin, population,
    };

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

    /// Trap 13: what [`margin`] says has to be shown saying each of the three things it says.
    #[test]
    fn a_margin_names_the_nearer_edge_and_how_far_it_is() {
        assert!(
            margin(11.0, 10.0, 20.0).starts_with("1.000 from the low edge (10%"),
            "a figure a tenth of the way in is a tenth of the band from the low edge: {}",
            margin(11.0, 10.0, 20.0)
        );
        assert!(
            margin(19.5, 10.0, 20.0).starts_with("0.500 from the high edge (5%"),
            "the nearer edge is the one reported: {}",
            margin(19.5, 10.0, 20.0)
        );
        assert_eq!(
            margin(9.0, 10.0, 20.0),
            "1.000 past the low edge",
            "a figure outside its band says so, and by how much"
        );
        assert_eq!(
            margin(22.0, 10.0, 20.0),
            "2.000 past the high edge",
            "and the same above the band"
        );
    }

    /// A band prints and does not assert, which is the one thing about it a reader must not
    /// have to take on trust: a figure outside its band returns from this call.
    #[test]
    fn a_band_prints_a_figure_outside_it_rather_than_failing() {
        band("the cold open", 0.90, 0.49, 0.80);
    }

    /// The three documents a gate would name, found and named alike, is what holding looks like.
    #[test]
    fn a_named_population_that_matches_holds() {
        population(
            "documents that need a password",
            ["a.pdf", "b.pdf"].map(str::to_owned),
            &["a.pdf", "b.pdf"],
        );
    }

    /// Trap 13, one direction: a document the list does not name is named as having joined.
    #[test]
    #[should_panic(expected = "joined [\"c.pdf\"], left []")]
    fn a_document_that_joined_is_named() {
        population(
            "documents that need a password",
            ["a.pdf", "b.pdf", "c.pdf"].map(str::to_owned),
            &["a.pdf", "b.pdf"],
        );
    }

    /// Trap 13, the other direction: a name the run no longer finds is named as having left.
    #[test]
    #[should_panic(expected = "joined [], left [\"b.pdf\"]")]
    fn a_document_that_left_is_named() {
        population(
            "documents that need a password",
            ["a.pdf"].map(str::to_owned),
            &["a.pdf", "b.pdf"],
        );
    }

    /// And the case that is the whole reason a name beats a count: one joined, one left, and the
    /// count identical on both sides of the swap.
    #[test]
    #[should_panic(expected = "joined [\"c.pdf\"], left [\"b.pdf\"]")]
    fn a_swap_a_count_cannot_see_is_named() {
        population(
            "documents that need a password",
            ["a.pdf", "c.pdf"].map(str::to_owned),
            &["a.pdf", "b.pdf"],
        );
    }
}
