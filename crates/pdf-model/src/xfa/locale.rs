//! The locales a form's picture clauses are read in: the `localeSet` packet, and which locale
//! prevails for a field.
//!
//! XFA 3.3 chapter 19, *Locale Set Specification* (pages 943–964), is the packet's grammar:
//! each `locale` carries its month, day, meridiem and era names, its date, time and number
//! patterns, and its number and currency symbols. Chapter 4 says where they come from inside a
//! PDF file (page 158): the locale definitions a form uses are to be resolved in full and included
//! in the file, so this module reads the packet and carries no locale database of its own. A
//! property the packet does not state is inherited from the progressively more generic locale —
//! `language_country_modifier`, `language_country`, `language` — and past those page 158 names
//! a root locale it likens to US English and whose values it does not print; such a property is
//! said to be missing rather than guessed (trap 5).
//!
//! The prevailing locale is page 1153's: a picture clause's own `(locale)`, then the `locale`
//! of the field or the nearest container above it, then the configuration's `common` `locale`
//! (page 153's third source, the `locale` element of the Config specification), then the
//! ambient locale, which this program does not supply, so XFA 3.3's own fallback of `en_US`
//! stands.

use super::xml::Tree;

/// One `locale` element of the packet, as far as reading a picture clause needs.
#[derive(Debug, Clone, Default)]
pub(crate) struct Locale {
    /// Its `name`, `-` read as `_` and compared without regard to case (page 155).
    name: String,
    /// `monthNames` and `dayNames`, full and `abbr="1"`; `meridiemNames`; `eraNames`.
    months: Option<Vec<String>>,
    months_abbr: Option<Vec<String>>,
    days: Option<Vec<String>>,
    days_abbr: Option<Vec<String>>,
    meridiems: Option<Vec<String>>,
    eras: Option<Vec<String>>,
    /// `datePattern`, `timePattern` and `numberPattern`, by their `name`.
    date_patterns: Vec<(String, String)>,
    time_patterns: Vec<(String, String)>,
    number_patterns: Vec<(String, String)>,
    /// `numberSymbol` and `currencySymbol`, by their `name`.
    number_symbols: Vec<(String, String)>,
    currency_symbols: Vec<(String, String)>,
}

/// The `localeSet` packet's locales, and the configuration's default locale.
#[derive(Debug, Clone, Default)]
pub struct LocaleSet {
    locales: Vec<Locale>,
    /// The `config` packet's `common` `locale`, where it states one.
    configured: Option<String>,
    /// Why the packet was present and could not be read, for the sentence a field it leaves
    /// without a locale is reported with.
    trouble: Option<String>,
}

/// Which list of names a picture symbol reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Names {
    Months,
    MonthsAbbr,
    Days,
    DaysAbbr,
    Meridiems,
    Eras,
}

impl Names {
    /// The packet's element for the list, for a sentence.
    pub(crate) const fn element(self) -> &'static str {
        match self {
            Self::Months => "monthNames",
            Self::MonthsAbbr => "monthNames abbr=\"1\"",
            Self::Days => "dayNames",
            Self::DaysAbbr => "dayNames abbr=\"1\"",
            Self::Meridiems => "meridiemNames",
            Self::Eras => "eraNames",
        }
    }
}

/// Which pattern list a predefined picture or Rule 2A's default reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Patterns {
    Date,
    Time,
    Number,
}

/// Which of a locale's pattern lists [`LocaleSet::pattern`] reads.
type PatternList = fn(&Locale) -> &[(String, String)];

fn normalised(name: &str) -> String {
    name.trim().replace('-', "_").to_ascii_lowercase()
}

impl LocaleSet {
    /// Reads `localeSet` under `root` in `tree`, and the default locale from `config`.
    pub(crate) fn read(locale_set: Option<(&Tree, usize)>, config: Option<&Tree>) -> Self {
        let mut set = Self::default();
        if let Some((tree, root)) = locale_set
            && let Some(element) = tree.get(root)
        {
            for &child in &element.children {
                if let Some(locale) = tree.get(child).filter(|e| e.local == "locale") {
                    set.locales
                        .push(read_locale(tree, child, locale.attribute("name")));
                }
            }
        }
        if let Some(config) = config {
            set.configured = config
                .elements
                .iter()
                .enumerate()
                .filter(|(_, element)| element.local == "common")
                .find_map(|(at, _)| config.child(at, "locale"))
                .and_then(|at| config.get(at))
                .map(|element| element.text.trim().to_owned())
                .filter(|name| !name.is_empty());
        }
        set
    }

    /// A set whose packet was present and unreadable, with the reason.
    pub(crate) fn unreadable(reason: String) -> Self {
        Self {
            trouble: Some(reason),
            ..Self::default()
        }
    }

    /// The locale that prevails where the template's own chain states `stated` (page 1153's
    /// second source) — the configuration's, else `en_US`.
    pub(crate) fn prevailing(&self, stated: Option<&str>) -> String {
        stated
            .filter(|name| !name.trim().is_empty() && name.trim() != "ambient")
            .or(self.configured.as_deref())
            .unwrap_or("en_US")
            .to_owned()
    }

    /// The locales `name` inherits from, most specific first (page 158's progression).
    fn chain(&self, name: &str) -> Vec<&Locale> {
        let mut wanted = normalised(name);
        let mut chain = Vec::new();
        loop {
            if let Some(locale) = self.locales.iter().find(|l| l.name == wanted) {
                chain.push(locale);
            }
            match wanted.rfind('_') {
                Some(at) => wanted.truncate(at),
                None => return chain,
            }
        }
    }

    fn missing(&self, locale: &str, what: &str) -> String {
        match &self.trouble {
            Some(trouble) => format!(
                "its locale {locale} needs the form's localeSet packet for its {what}, and {trouble}"
            ),
            None => format!(
                "the form's localeSet packet states no {what} for its locale {locale} or a locale \
                 it inherits from, and XFA 3.3 page 158 has a form inside a PDF file carry its \
                 locales resolved"
            ),
        }
    }

    /// A list of names of `locale`, inherited where the locale does not state it.
    pub(crate) fn names(&self, locale: &str, which: Names) -> Result<&[String], String> {
        self.chain(locale)
            .into_iter()
            .find_map(|l| {
                match which {
                    Names::Months => &l.months,
                    Names::MonthsAbbr => &l.months_abbr,
                    Names::Days => &l.days,
                    Names::DaysAbbr => &l.days_abbr,
                    Names::Meridiems => &l.meridiems,
                    Names::Eras => &l.eras,
                }
                .as_deref()
            })
            .ok_or_else(|| self.missing(locale, which.element()))
    }

    /// A named pattern of `locale` — `short`, `med`, `long`, `full` for dates and times;
    /// `numeric`, `currency`, `percent` for numbers (pages 1155–1156).
    pub(crate) fn pattern(
        &self,
        locale: &str,
        which: Patterns,
        name: &str,
    ) -> Result<&str, String> {
        let (list, element): (PatternList, _) = match which {
            Patterns::Date => (|l| &l.date_patterns, "datePattern"),
            Patterns::Time => (|l| &l.time_patterns, "timePattern"),
            Patterns::Number => (|l| &l.number_patterns, "numberPattern"),
        };
        self.chain(locale)
            .into_iter()
            .find_map(|l| {
                list(l)
                    .iter()
                    .find(|(n, _)| n == name)
                    .map(|(_, p)| p.as_str())
            })
            .ok_or_else(|| self.missing(locale, &format!("{element} named {name}")))
    }

    /// A `numberSymbol` of `locale` — `decimal`, `grouping`, `percent`, `minus`, `zero` (page
    /// 963) — or, with `currency`, a `currencySymbol` — `symbol`, `isoname`, `decimal` (page 945).
    pub(crate) fn symbol(&self, locale: &str, currency: bool, name: &str) -> Result<&str, String> {
        self.chain(locale)
            .into_iter()
            .find_map(|l| {
                if currency {
                    &l.currency_symbols
                } else {
                    &l.number_symbols
                }
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, s)| s.as_str())
            })
            .ok_or_else(|| {
                let element = if currency {
                    "currencySymbol"
                } else {
                    "numberSymbol"
                };
                self.missing(locale, &format!("{element} named {name}"))
            })
    }
}

fn read_locale(tree: &Tree, at: usize, name: Option<&str>) -> Locale {
    let mut locale = Locale {
        name: normalised(name.unwrap_or("")),
        ..Locale::default()
    };
    let texts = |list: usize| -> Vec<String> {
        tree.get(list)
            .map(|e| {
                e.children
                    .iter()
                    .filter_map(|&c| tree.get(c))
                    .map(|c| c.text.trim().to_owned())
                    .collect()
            })
            .unwrap_or_default()
    };
    let named = |list: usize| -> Vec<(String, String)> {
        tree.get(list)
            .map(|e| {
                e.children
                    .iter()
                    .filter_map(|&c| tree.get(c))
                    .map(|c| (c.attribute("name").unwrap_or("").to_owned(), c.text.clone()))
                    .collect()
            })
            .unwrap_or_default()
    };
    let Some(element) = tree.get(at) else {
        return locale;
    };
    for &child in &element.children {
        let Some(property) = tree.get(child) else {
            continue;
        };
        match property.local.as_str() {
            "calendarSymbols" => {
                for &list in &property.children {
                    let Some(list_element) = tree.get(list) else {
                        continue;
                    };
                    let abbr = list_element.attribute("abbr") == Some("1");
                    let slot = match (list_element.local.as_str(), abbr) {
                        ("monthNames", false) => &mut locale.months,
                        ("monthNames", true) => &mut locale.months_abbr,
                        ("dayNames", false) => &mut locale.days,
                        ("dayNames", true) => &mut locale.days_abbr,
                        ("meridiemNames", _) => &mut locale.meridiems,
                        ("eraNames", _) => &mut locale.eras,
                        _ => continue,
                    };
                    *slot = Some(texts(list));
                }
            }
            "datePatterns" => locale.date_patterns = named(child),
            "timePatterns" => locale.time_patterns = named(child),
            "numberPatterns" => locale.number_patterns = named(child),
            // Page 963 makes `decimal` the default name of a `numberSymbol` and page 945
            // `symbol` that of a `currencySymbol`.
            "numberSymbols" => {
                locale.number_symbols = defaulted(named(child), "decimal");
            }
            "currencySymbols" => {
                locale.currency_symbols = defaulted(named(child), "symbol");
            }
            _ => {}
        }
    }
    locale
}

fn defaulted(list: Vec<(String, String)>, default: &str) -> Vec<(String, String)> {
    list.into_iter()
        .map(|(name, value)| {
            if name.is_empty() {
                (default.to_owned(), value)
            } else {
                (name, value)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{LocaleSet, Names, Patterns};
    use crate::xfa::xml::parse;

    /// Page 158's inheritance: `fr_BE` takes what it does not state from `fr`, and nothing past
    /// the packet is invented.
    #[test]
    fn a_locale_inherits_from_its_language_and_no_further() {
        let text = "<localeSet><locale name=\"fr\"><calendarSymbols><monthNames><month>janvier</month>\
            </monthNames></calendarSymbols><numberSymbols><numberSymbol name=\"decimal\">,</numberSymbol>\
            </numberSymbols></locale><locale name=\"fr-BE\"><datePatterns>\
            <datePattern name=\"med\">D/MM/YY</datePattern></datePatterns></locale></localeSet>";
        let tree = parse(text.as_bytes()).expect("well formed");
        let set = LocaleSet::read(Some((&tree, 0)), None);
        assert_eq!(
            set.names("fr_BE", Names::Months).map(<[String]>::len),
            Ok(1)
        );
        assert_eq!(set.pattern("FR_be", Patterns::Date, "med"), Ok("D/MM/YY"));
        assert_eq!(set.symbol("fr_BE", false, "decimal"), Ok(","));
        assert!(set.symbol("fr_BE", false, "grouping").is_err());
        assert!(set.names("de_DE", Names::Months).is_err());
    }

    /// Page 1153's order past the template: the configuration's locale, then `en_US`.
    #[test]
    fn the_configured_locale_then_en_us_prevail() {
        let config =
            parse(b"<config><acrobat><common><locale>de_CH</locale></common></acrobat></config>")
                .expect("well formed");
        let set = LocaleSet::read(None, Some(&config));
        assert_eq!(set.prevailing(None), "de_CH");
        assert_eq!(set.prevailing(Some("ambient")), "de_CH");
        assert_eq!(set.prevailing(Some("nl_NL")), "nl_NL");
        assert_eq!(LocaleSet::default().prevailing(None), "en_US");
    }
}
