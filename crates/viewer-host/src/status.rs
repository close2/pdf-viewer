//! What a status bar says about the pages on the screen, which is a sentence rather than a list.
//!
//! `viewer_core::Query::Reports` answers with one entry per page Table 29's arrangement is
//! showing, because a column shows several and a refusal belongs to one of them. What a host does
//! with that is put it somewhere a person reads, and the three hosts do it in a `GtkLabel`, a
//! `QStatusBar` and a line of chrome — three widgets and **one wording**, which is what this crate
//! is for: the third copy of a sentence is where two hosts stop agreeing about what they are
//! saying (ADR 0246).
//!
//! The page is named because it has to be. Under `SinglePage` a note needs no attribution — there
//! is one page and it is the one being looked at — and under a column a bare "could not draw the
//! shading" is a sentence about one of four pages with nothing saying which.

use viewer_core::PageReports;

/// What the pages on the screen could not draw, worded for a status bar.
///
/// The empty string where nothing was reported, which is most documents' every page: a host says
/// nothing rather than saying that there is nothing to say. Pages are separated by ` · ` and a
/// page's own notes by `; `, so that the boundary between two pages is visible in one line.
#[must_use]
pub fn on_screen(pages: &[PageReports<'_>]) -> String {
    pages
        .iter()
        .filter(|page| !page.notes.is_empty())
        .map(|page| {
            format!(
                "page {}: {}",
                page.page.saturating_add(1),
                page.notes.join("; ")
            )
        })
        .collect::<Vec<String>>()
        .join(" · ")
}

/// What a window says when [`viewer_core::Event::OpenFailed`] arrives.
///
/// **One sentence, three windows, and it replaced an `exit(1)` in the third.** `viewer-ui` printed
/// this on `stderr` and left the process; the two native hosts each built their own version of the
/// line and stayed up. That is the shape ADR 0545 corrected for §7.6.4.1's password one round
/// earlier and deliberately did not widen — a window refusing the *environment* rather than
/// answering the document — and the argument is the same: a person who launched a viewer from a
/// desktop is told nothing at all by a process that is no longer there.
///
/// `named` is what the person asked for, not a path this crate went looking for: [`viewer_core`]'s
/// rule 2 is that the crate has no filesystem, so which file it was is the host's to say.
#[must_use]
pub fn cannot_open(named: &str, reason: &str) -> String {
    format!("{named} could not be opened: {reason}")
}

/// What a window says about a document that opened and has no pages in it.
///
/// **Not an error, and that is why it is a sentence rather than an exit.** §7.7.3.2 makes a page
/// tree's `/Count` "the number of leaf nodes (page objects) that are descendants of this node", and
/// states no floor on it; §7.7.2's `/Pages` is required and a tree with no leaves is a document
/// with nothing to show. So this program has read the file correctly and there is nothing on the
/// screen, which is exactly the case a reader most needs told — a blank window and a window with a
/// broken file behind it look identical.
#[must_use]
pub fn no_pages(named: &str) -> String {
    format!("{named} states no pages (§7.7.3.2's /Count is zero), so there is nothing to show")
}

#[cfg(test)]
mod tests {
    use viewer_core::PageReports;

    use super::on_screen;

    /// Nothing to report is nothing said, which is what a status bar wants.
    #[test]
    fn a_screen_with_nothing_to_report_is_silent() {
        assert_eq!(on_screen(&[]), "");
        let quiet = [PageReports {
            page: 0,
            notes: &[],
        }];
        assert_eq!(on_screen(&quiet), "");
    }

    /// Every page that reported is named, and a page that did not is not mentioned.
    ///
    /// The case this exists for is the middle one: under a column, page 3 reporting while pages 2
    /// and 4 do not must read as a sentence about page 3 — which is exactly what a host showing
    /// the *current* page's reports for the whole screen could not say.
    #[test]
    fn each_reporting_page_is_named_and_the_quiet_ones_are_not() {
        let second: Vec<String> = vec!["a shading this reader could not draw".to_owned()];
        let third: Vec<String> = vec!["one font".to_owned(), "one image".to_owned()];
        let pages = [
            PageReports {
                page: 1,
                notes: &second,
            },
            PageReports {
                page: 2,
                notes: &third,
            },
            PageReports {
                page: 3,
                notes: &[],
            },
        ];
        assert_eq!(
            on_screen(&pages),
            "page 2: a shading this reader could not draw · page 3: one font; one image"
        );
    }
}
