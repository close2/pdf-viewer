//! `[profile] base`: an operator's file that states only the sites it answers differently.
//!
//! `doc/rfc/0014` section 6, adopted by `doc/questions/A706` item 5. An operator who wants a
//! shipped profile with two answers changed writes those two and names the profile under them,
//! and every other site is the named profile's — so the file says what the operator chose and
//! nothing they copied. **The merge is of the two files' tables, before either is validated**:
//! what [`super::Configuration::read`] then holds to the target is one configuration, read by the
//! one reader, so a base adds no second path through the checks.
//!
//! # What the operator's file takes over, and what it cannot
//!
//! - **A site the file names in any table** — `[site."<id>"]`, or a target or shape row under it —
//!   is the file's at every target: none of the base's tables for that site is kept. A row-by-row
//!   merge would let the base's target-qualified row win over the operator's unqualified one at
//!   that target, and the answer would then be found in neither file alone.
//! - **A departure** (`[depart."<id>"]`) and **a tool** (`[tool.<name>]`) are taken over the same
//!   way, by their identifier.
//! - **`[profile]`'s keys** are the file's where it states them and the base's where it does not,
//!   except the three that say which file this is — `name`, `sentence` and `base` — which are never
//!   inherited: a file announcing itself under its base's name would be the confusion the name
//!   exists to prevent.
//!
//! **One level only**: a base that itself names a base is an error, so what a site does is always
//! found in at most two files (`doc/rfc/0014` section 6). The base is a profile this program ships
//! ([`crate::profiles::SHIPPED`]), compiled in, so naming one opens no path.

use std::collections::BTreeSet;

use super::config::ConfigError;
use super::toml::{self, Table};

/// Where a site's answer was read from, as `--remedy-sites` lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadFrom {
    /// The configuration's own file states the site.
    ThisFile,
    /// The file names no table for the site, and its `[profile] base` does.
    Base(String),
}

/// The base a configuration was read over, and the sites taken from it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct Inherited {
    /// The shipped profile `[profile] base` names.
    pub(super) base: String,
    /// The sites whose tables came from the base, by identifier.
    pub(super) sites: BTreeSet<String>,
}

/// The `[profile]` keys that say which file this is, which a base never supplies.
const OWN_KEYS: [&str; 3] = ["name", "sentence", "base"];

/// The three table kinds an identifier takes over, each by its second path segment.
const TAKEN_OVER: [&str; 3] = ["site", "depart", "tool"];

/// The operator's tables with the base's folded in, and what was inherited, where the file's
/// `[profile]` names a base; the tables unchanged where it names none.
///
/// # Errors
///
/// [`ConfigError::WrongValue`] where `base` is not a string, [`ConfigError::UnknownBase`] where it
/// names no shipped profile, and [`ConfigError::BaseOfABase`] where the profile it names states a
/// base of its own.
pub(super) fn inherit(tables: Vec<Table>) -> Result<(Vec<Table>, Option<Inherited>), ConfigError> {
    inherit_from(tables, &|name| {
        crate::profiles::named(name).map(|shipped| shipped.text)
    })
}

/// [`inherit`], with the profiles a base may name given by `shipped`.
fn inherit_from(
    mut tables: Vec<Table>,
    shipped: &dyn Fn(&str) -> Option<&'static str>,
) -> Result<(Vec<Table>, Option<Inherited>), ConfigError> {
    let Some(own) = tables
        .iter()
        .position(|tbl| tbl.path.len() == 1 && tbl.path[0] == "profile")
    else {
        return Ok((tables, None));
    };
    let Some(stated) = tables[own].get("base") else {
        return Ok((tables, None));
    };
    let line = tables[own].line;
    let Some(name) = stated.as_text().map(str::to_owned) else {
        return Err(ConfigError::WrongValue {
            line,
            site: "profile".to_owned(),
            key: "base".to_owned(),
            wanted: "the name of a profile this program ships",
            found: stated.kind(),
        });
    };
    let text = shipped(&name).ok_or_else(|| ConfigError::UnknownBase {
        line,
        base: name.clone(),
        offered: crate::profiles::names(),
    })?;
    let base_tables = toml::parse(text).map_err(|error| ConfigError::Toml(error.to_string()))?;
    if base_tables
        .iter()
        .any(|tbl| tbl.path.len() == 1 && tbl.path[0] == "profile" && tbl.get("base").is_some())
    {
        return Err(ConfigError::BaseOfABase { line, base: name });
    }
    let stated_here: BTreeSet<(String, String)> = tables.iter().filter_map(identifier).collect();
    let mut inherited = Inherited {
        base: name,
        sites: BTreeSet::new(),
    };
    for tbl in base_tables {
        if tbl.path.len() == 1 && tbl.path[0] == "profile" {
            for entry in tbl.entries {
                if !OWN_KEYS.contains(&entry.key.as_str()) && tables[own].get(&entry.key).is_none()
                {
                    tables[own].entries.push(entry);
                }
            }
            continue;
        }
        let Some(id) = identifier(&tbl) else {
            continue;
        };
        if stated_here.contains(&id) {
            continue;
        }
        if id.0 == "site" {
            inherited.sites.insert(id.1);
        }
        tables.push(tbl);
    }
    Ok((tables, Some(inherited)))
}

/// The kind and the identifier a table is taken over by, where it is one of [`TAKEN_OVER`].
fn identifier(tbl: &Table) -> Option<(String, String)> {
    let kind = tbl.path.first()?;
    let id = tbl.path.get(1)?;
    TAKEN_OVER
        .contains(&kind.as_str())
        .then(|| (kind.clone(), id.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const OPERATOR: &str = "[profile]\nname = \"mine\"\nbase = \"theirs\"\n\n\
                            [site.\"a\"]\nremedy = \"stop\"\n";

    /// A base two levels deep is refused, whatever the second level would have said.
    #[test]
    fn a_base_that_names_a_base_is_refused() {
        let tables = toml::parse(OPERATOR).expect("parses");
        let deeper: &dyn Fn(&str) -> Option<&'static str> =
            &|_| Some("[profile]\nname = \"theirs\"\nbase = \"as-shown\"\n");
        assert_eq!(
            inherit_from(tables, deeper).map(|(_, inherited)| inherited),
            Err(ConfigError::BaseOfABase {
                line: 1,
                base: "theirs".to_owned()
            })
        );
    }

    /// The file's own site replaces every table the base has for it, a target row included;
    /// the base's other sites, its tools and its unstated `[profile]` keys are kept, and its
    /// name is not.
    #[test]
    fn a_site_the_file_names_is_the_files_at_every_target() {
        let base: &dyn Fn(&str) -> Option<&'static str> = &|_| {
            Some(
                "[profile]\nname = \"theirs\"\nrecovered-structure = \"discard\"\n\n\
                 [site.\"a\"]\nremedy = \"discard\"\n\n\
                 [site.\"a\".target.\"4\"]\nremedy = \"discard\"\n\n\
                 [site.\"b\"]\nremedy = \"discard\"\n",
            )
        };
        let (tables, inherited) =
            inherit_from(toml::parse(OPERATOR).expect("parses"), base).expect("merges");
        let inherited = inherited.expect("a base was named");
        assert_eq!(inherited.base, "theirs");
        assert_eq!(inherited.sites, BTreeSet::from(["b".to_owned()]));
        let rows_for_a = tables
            .iter()
            .filter(|tbl| tbl.path.get(1).map(String::as_str) == Some("a"))
            .count();
        assert_eq!(
            rows_for_a, 1,
            "only the file's own row for the site it names"
        );
        let profile = &tables[0];
        assert_eq!(
            profile.get("name").and_then(toml::Value::as_text),
            Some("mine")
        );
        assert_eq!(
            profile
                .get("recovered-structure")
                .and_then(toml::Value::as_text),
            Some("discard")
        );
    }
}
