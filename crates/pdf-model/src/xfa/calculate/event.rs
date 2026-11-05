//! The `$event` object a script reads while an event runs (XFA 3.3 pages 398–404; ADR 2646).
//!
//! A primary event — one a person's action raises — sets some of `$event`'s properties, and page
//! 398's table says which for each event of each kind of object; a secondary event, a calculation
//! or a validation the primary one woke, sees the primary event's values. Using a property the
//! current event did not set is an error (page 398), so a script that reads one is stopped with a
//! sentence and not handed a null. Page 399 lets a script write `change`, `selStart`, `selEnd`
//! and `cancelAction` and no other property; page 400 makes `newText` the text `change` puts
//! between `selStart` and `selEnd` of `prevText`, which is what is placed into the field once the
//! event's scripts have run.
//!
//! **A property is a node of its own.** A script reaches `$event` through the paths it already
//! names — `FormCalc`'s accessors and JavaScript's questions to its host — so each property is a
//! number past the template's containers, read and assigned as a container's value is, and
//! `$event.target` is the container itself. Nothing on the wire between a host and a script
//! worker changes.

use super::Scalar;

/// The properties of `$event` this reader carries, in the order page 402 lists them, with the
/// number past the template's containers each one is.
const CARRIED: [&str; 7] = [
    "cancelAction",
    "change",
    "name",
    "newText",
    "prevText",
    "selEnd",
    "selStart",
];

/// The properties page 398's table has an event set that this reader is never told of: the
/// keyboard's modifier keys, a choice list's re-entry, how an exit was committed, and what a
/// paste held past a field's limit. Reading one is refused by name rather than answered null.
const UNSUPPLIED: [&str; 8] = [
    "modifier",
    "shift",
    "reenter",
    "commitKey",
    "keyDown",
    "fullText",
    "newContentType",
    "prevContentType",
];

/// The other properties page 402–404 lists: a connection's fault and the start of a selection,
/// which no event this reader raises sets.
const ELSEWHERE: [&str; 3] = ["soapFaultCode", "soapFaultString", "start"];

/// What a person's entry replaced (page 400): the field's text before it, and the span of that
/// text the entry replaced, by characters.
#[derive(Debug, Clone, PartialEq)]
pub struct Replaced {
    /// `prevText`, where the host said what the field held; `None` makes `prevText`, `selStart`
    /// and `selEnd` unsupplied and the entry the replacement of the whole.
    previous: Option<String>,
    /// `change`.
    change: String,
    /// `selStart`.
    start: usize,
    /// `selEnd`.
    end: usize,
}

impl Replaced {
    /// A commit of `entered` over `previous`: a commit replaces the field's whole content, so the
    /// span is all of `previous` and `change` is all of `entered`.
    #[must_use]
    pub fn commit(previous: Option<&str>, entered: &str) -> Self {
        Self {
            previous: previous.map(str::to_owned),
            change: entered.to_owned(),
            start: 0,
            end: previous.map_or(0, |text| text.chars().count()),
        }
    }

    /// `newText` (page 400): `change` between `selStart` and `selEnd` of `prevText`.
    #[must_use]
    pub fn new_text(&self) -> String {
        let Some(previous) = &self.previous else {
            return self.change.clone();
        };
        let head: String = previous.chars().take(self.start).collect();
        let tail: String = previous.chars().skip(self.end).collect();
        format!("{head}{}{tail}", self.change)
    }
}

/// The primary event a script runs under: its name, the container it happened on, and for a
/// `change` what the entry replaced.
#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    /// The activity, which is `$event.name`.
    name: String,
    /// `$event.target`: the object whose change of state fired the event (page 404), which is
    /// the field a listening container's script hears it from.
    target: usize,
    /// Whether page 398's table has this event set `name` and `target`: it lists no property for
    /// an exclusion group's `enter` or `exit`.
    named: bool,
    /// What a `change` replaced.
    replaced: Option<Replaced>,
    /// `$event.cancelAction` while an action such as a save or a submit is under way, which the
    /// processor clears at the action's beginning (page 402); `None` for an event no action
    /// raised, which does not set it.
    cancel: Option<bool>,
}

impl Event {
    /// The event `activity` on the container `target`; `named` as page 398's table says.
    #[must_use]
    pub fn new(activity: &str, target: usize, named: bool, replaced: Option<Replaced>) -> Self {
        Self {
            name: activity.to_owned(),
            target,
            named,
            replaced,
            cancel: None,
        }
    }

    /// The event the pre- and post-events of an action run under, `activity` being the one that
    /// fires first (pages 387–389 and 402): the action clears `cancelAction` at its beginning,
    /// and where a person's action raised it — a click on a field whose event submits — the
    /// primary properties are that action's, since page 398 has a secondary event leave the
    /// primary properties as the primary event set them. An action no person's action raised —
    /// a save — sets no primary property.
    #[must_use]
    pub fn for_action(primary: Option<Self>, activity: &str) -> Self {
        let mut event = primary.unwrap_or_else(|| Self::new(activity, 0, false, None));
        event.cancel = Some(false);
        event
    }

    /// Whether a script set `cancelAction` (page 402), which aborts the action it is under.
    #[must_use]
    pub fn cancelled(&self) -> bool {
        self.cancel == Some(true)
    }

    /// What the entry replaced, as the event's scripts have left it.
    #[must_use]
    pub fn replaced(&self) -> Option<&Replaced> {
        self.replaced.as_ref()
    }
}

/// What a path's step after `$event` names.
pub(super) enum Named {
    /// `target`: the container.
    Target(usize),
    /// Another property: the node it is.
    Property(usize),
}

/// The property `name` of `event`, as a node past `containers`.
///
/// # Errors
///
/// The sentence for a property no event is running for, one the event did not set, one this reader
/// is not told of, and a name that is not a property of `$event`.
pub(super) fn property(
    event: Option<&Event>,
    containers: usize,
    name: &str,
) -> Result<Named, String> {
    let known = CARRIED.contains(&name)
        || name == "target"
        || UNSUPPLIED.contains(&name)
        || ELSEWHERE.contains(&name);
    if !known {
        return Err(format!(
            "$event.{name}: not a property of $event (XFA 3.3 pages 402–404)"
        ));
    }
    let Some(event) = event else {
        return Err(format!(
            "$event.{name}: no person's action is being handled, so $event has no property set \
             (XFA 3.3 page 398)"
        ));
    };
    let unset = || {
        format!(
            "$event.{name}: the {} event does not set it, and a script may not use a property \
             the current event did not set (XFA 3.3 page 398)",
            event.name
        )
    };
    if UNSUPPLIED.contains(&name) {
        return Err(format!(
            "$event.{name}: this reader is not told it by the window, so it is not offered"
        ));
    }
    if name == "cancelAction" {
        return match event.cancel {
            Some(_) => Ok(Named::Property(containers)),
            None => Err(unset()),
        };
    }
    if !event.named || ELSEWHERE.contains(&name) {
        return Err(unset());
    }
    if name == "target" {
        return Ok(Named::Target(event.target));
    }
    if name != "name" {
        let Some(replaced) = &event.replaced else {
            return Err(unset());
        };
        if replaced.previous.is_none() && matches!(name, "prevText" | "selStart" | "selEnd") {
            return Err(format!(
                "$event.{name}: the window did not say what the field held before the entry, so \
                 it is not offered"
            ));
        }
    }
    let at = CARRIED
        .iter()
        .position(|carried| *carried == name)
        .unwrap_or(0);
    Ok(Named::Property(containers.saturating_add(at)))
}

/// The value of the property node `node`, `None` where it is not one.
pub(super) fn value(event: Option<&Event>, containers: usize, node: usize) -> Option<Scalar> {
    let name = CARRIED.get(node.checked_sub(containers)?)?;
    let event = event?;
    let replaced = event.replaced.as_ref();
    let count = |n: usize| Scalar::Number(f64::from(u32::try_from(n).unwrap_or(u32::MAX)));
    Some(match *name {
        "cancelAction" => Scalar::Number(if event.cancel? { 1.0 } else { 0.0 }),
        "name" => Scalar::Text(event.name.clone()),
        "change" => Scalar::Text(replaced?.change.clone()),
        "newText" => Scalar::Text(replaced?.new_text()),
        "prevText" => Scalar::Text(replaced?.previous.clone()?),
        "selStart" => count(replaced?.start),
        _ => count(replaced?.end),
    })
}

/// Whether `node` is a property node past `containers`.
pub(super) fn is_property(containers: usize, node: usize) -> bool {
    node.checked_sub(containers)
        .is_some_and(|at| at < CARRIED.len())
}

/// Assigns `value` to the property node `node` of `event` (page 399), and answers the bytes of
/// text it stored.
///
/// # Errors
///
/// The sentence for a read-only property, a value of the wrong kind, and a span outside
/// `prevText` or reversed (page 403: `selStart` is never past `selEnd`).
pub(super) fn assign(
    event: Option<&mut Event>,
    containers: usize,
    node: usize,
    value: Scalar,
) -> Result<u64, String> {
    let name = node
        .checked_sub(containers)
        .and_then(|at| CARRIED.get(at))
        .copied()
        .unwrap_or("?");
    if name == "cancelAction" {
        let Some(flag) = event.and_then(|event| event.cancel.as_mut()) else {
            return Err(
                "$event.cancelAction: no action such as a save or a submit is under way, so the \
                 current event did not set it (XFA 3.3 pages 398 and 402)"
                    .to_owned(),
            );
        };
        if !super::activate::true_value(&value) {
            return Err(
                "$event.cancelAction: a script may set it and must not attempt to clear it (XFA \
                 3.3 page 402)"
                    .to_owned(),
            );
        }
        *flag = true;
        return Ok(0);
    }
    let Some(replaced) = event.and_then(|event| event.replaced.as_mut()) else {
        return Err(format!(
            "$event.{name}: read-only, since a script may write only change, selStart, selEnd and \
             cancelAction, and only where the event set them (XFA 3.3 page 399)"
        ));
    };
    let length = replaced
        .previous
        .as_deref()
        .map_or(0, |text| text.chars().count());
    let position = |value: &Scalar| match value {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "checked to lie in 0..=length first, and a length is a usize"
        )]
        Scalar::Number(n)
            if n.is_finite()
                && *n >= 0.0
                && *n <= f64::from(u32::try_from(length).unwrap_or(u32::MAX)) =>
        {
            Ok(n.trunc() as usize)
        }
        _ => Err(format!(
            "$event.{name}: a position in prevText's {length} characters (XFA 3.3 page 403)"
        )),
    };
    match name {
        "change" => {
            let text = match value {
                Scalar::Text(text) => text,
                Scalar::Number(n) => n.to_string(),
                Scalar::Null => String::new(),
            };
            let cost = u64::try_from(text.len()).unwrap_or(u64::MAX);
            replaced.change = text;
            Ok(cost)
        }
        "selStart" | "selEnd" if replaced.previous.is_none() => Err(format!(
            "$event.{name}: the window did not say what the field held before the entry, so \
             there is no span to move"
        )),
        "selStart" => {
            let at = position(&value)?;
            if at > replaced.end {
                return Err(format!(
                    "$event.selStart: {at} is past selEnd {} (XFA 3.3 page 403)",
                    replaced.end
                ));
            }
            replaced.start = at;
            Ok(0)
        }
        "selEnd" => {
            let at = position(&value)?;
            if at < replaced.start {
                return Err(format!(
                    "$event.selEnd: {at} is before selStart {} (XFA 3.3 page 403)",
                    replaced.start
                ));
            }
            replaced.end = at;
            Ok(0)
        }
        _ => Err(format!(
            "$event.{name}: read-only, since a script may write only change, selStart, selEnd and \
             cancelAction (XFA 3.3 page 399)"
        )),
    }
}
