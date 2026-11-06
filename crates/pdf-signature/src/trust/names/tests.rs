//! Section 4.2.1.10's rule for each name form, and section 7.1's comparison, one test per rule.

use super::*;

fn dns(text: &str) -> Name {
    Name::Dns(text.to_owned())
}

fn mailbox(text: &str) -> Name {
    Name::Rfc822(text.to_owned())
}

fn uri(text: &str) -> Name {
    Name::Uri(text.to_owned())
}

/// "Any DNS name that can be constructed by simply adding zero or more labels to the left-hand
/// side of the name satisfies the name constraint" — and nothing else does.
#[test]
fn a_dns_name_is_within_its_domain_by_whole_labels_only() {
    let base = dns("example.com");
    assert_eq!(within(&dns("example.com"), &base), Match::Yes);
    assert_eq!(within(&dns("www.Example.COM"), &base), Match::Yes);
    assert_eq!(within(&dns("notexample.com"), &base), Match::No);
    assert_eq!(within(&dns("example.com.evil"), &base), Match::No);
}

/// "a particular mailbox, all addresses on a particular host, or all mailboxes in a domain".
#[test]
fn an_rfc822_constraint_is_a_mailbox_a_host_or_a_domain() {
    assert_eq!(
        within(&mailbox("a@host.example"), &mailbox("a@HOST.example")),
        Match::Yes
    );
    assert_eq!(
        within(&mailbox("A@host.example"), &mailbox("a@host.example")),
        Match::No,
        "section 7.5: the local part compares exactly"
    );
    assert_eq!(
        within(&mailbox("b@host.example"), &mailbox("host.example")),
        Match::Yes
    );
    assert_eq!(
        within(&mailbox("b@sub.host.example"), &mailbox("host.example")),
        Match::No
    );
    assert_eq!(
        within(&mailbox("b@sub.host.example"), &mailbox(".host.example")),
        Match::Yes
    );
    assert_eq!(
        within(&mailbox("b@host.example"), &mailbox(".host.example")),
        Match::No
    );
}

/// "The constraint MUST be applied to the host part of the name"; a leading period admits hosts
/// below the domain only.
#[test]
fn a_uri_constraint_reads_the_host_and_a_uri_without_one_is_refused() {
    assert_eq!(
        within(&uri("https://host.example:8443/a"), &uri("host.example")),
        Match::Yes
    );
    assert_eq!(
        within(&uri("https://a.host.example/"), &uri("host.example")),
        Match::No
    );
    assert_eq!(
        within(&uri("https://a.host.example/"), &uri(".host.example")),
        Match::Yes
    );
    let mut state = NameState::default();
    state
        .admit(NameConstraints {
            permitted: Some(vec![Subtree {
                base: uri("host.example"),
            }]),
            excluded: Vec::new(),
        })
        .expect("within the bound");
    assert_eq!(
        state.check(&[uri("urn:isbn:0451450523")]),
        Err(NameRefusal::UriWithoutDomainHost),
        "section 4.2.1.10: a URI with no authority under a URI constraint MUST be rejected"
    );
}

/// An address and a mask of twice the name's length; the other family is outside.
#[test]
fn an_ip_address_is_within_its_masked_range() {
    let base = Name::IpAddress(vec![192, 0, 2, 0, 255, 255, 255, 0]);
    assert_eq!(
        within(&Name::IpAddress(vec![192, 0, 2, 7]), &base),
        Match::Yes
    );
    assert_eq!(
        within(&Name::IpAddress(vec![192, 0, 3, 7]), &base),
        Match::No
    );
    assert_eq!(within(&Name::IpAddress(vec![0; 16]), &base), Match::No);
}

/// Section 7.1: RFC 4518's preparation with case folding and insignificant space handling, exact
/// on ASCII; outside ASCII two values that differ are not decided.
#[test]
fn attribute_values_compare_after_preparation_and_undecided_beyond_ascii() {
    let value = |text: Option<&str>, encoding: &[u8]| Attribute {
        kind: vec![0x55, 0x04, 0x03],
        encoding: encoding.to_vec(),
        text: text.map(str::to_owned),
    };
    assert_eq!(
        attribute_matches(
            &value(Some("  Test   CA "), &[1]),
            &value(Some("test ca"), &[2])
        ),
        Match::Yes
    );
    assert_eq!(
        attribute_matches(&value(Some("Test CA"), &[1]), &value(Some("Test CB"), &[2])),
        Match::No
    );
    assert_eq!(
        attribute_matches(
            &value(Some("Stra\u{df}e"), &[1]),
            &value(Some("strasse"), &[2])
        ),
        Match::Undecided
    );
    assert_eq!(
        attribute_matches(&value(None, &[1]), &value(None, &[1])),
        Match::Yes,
        "identical encodings match whatever they hold"
    );
}

/// Each permitted set constrains only the forms it names, and a name must lie in one subtree of
/// every set that names its form — section 6.1.4 (g)(1)'s intersection.
#[test]
fn permitted_sets_intersect_and_excluded_sets_unite() {
    let mut state = NameState::default();
    for (permitted, excluded) in [
        (vec![dns("example.com")], vec![]),
        (
            vec![dns("a.example.com"), mailbox("example.org")],
            vec![dns("b.a.example.com")],
        ),
    ] {
        state
            .admit(NameConstraints {
                permitted: Some(permitted.into_iter().map(|base| Subtree { base }).collect()),
                excluded: excluded.into_iter().map(|base| Subtree { base }).collect(),
            })
            .expect("within the bound");
    }
    assert_eq!(state.check(&[dns("x.a.example.com")]), Ok(()));
    assert_eq!(
        state.check(&[dns("x.example.com")]),
        Err(NameRefusal::NotPermitted(2))
    );
    assert_eq!(
        state.check(&[dns("c.b.a.example.com")]),
        Err(NameRefusal::Excluded(2))
    );
    assert_eq!(state.check(&[mailbox("m@example.org")]), Ok(()));
    assert_eq!(
        state.check(&[Name::IpAddress(vec![10, 0, 0, 1])]),
        Ok(()),
        "no set names iPAddress, so none constrains it"
    );
}

/// A form this program does not compare is refused only where a constraint names that form.
#[test]
fn an_unread_form_is_refused_only_under_a_constraint_on_it() {
    let mut state = NameState::default();
    state
        .admit(NameConstraints {
            permitted: None,
            excluded: vec![Subtree {
                base: Name::Unread(0),
            }],
        })
        .expect("within the bound");
    assert_eq!(
        state.check(&[Name::Unread(0)]),
        Err(NameRefusal::FormNotProcessed(0))
    );
    assert_eq!(state.check(&[Name::Unread(8)]), Ok(()));
}

/// The bound on subtrees held for one path.
#[test]
fn subtrees_past_the_bound_are_refused() {
    let mut state = NameState::default();
    let many = (0..=MAX_SUBTREES)
        .map(|_| Subtree {
            base: dns("example.com"),
        })
        .collect();
    assert_eq!(
        state.admit(NameConstraints {
            permitted: Some(many),
            excluded: Vec::new(),
        }),
        Err(Unreadable::PastBound)
    );
}
