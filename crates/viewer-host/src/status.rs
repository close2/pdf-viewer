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
