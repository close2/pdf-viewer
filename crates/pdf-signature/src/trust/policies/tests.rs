//! The policy tree's bound, and the mapping step's two branches, beside PKITS's sections.

use super::*;

fn policy(last: u8) -> Oid {
    vec![0x2A, 0x03, last]
}

/// Section 6.1.3 (d) creates one node per expected policy per parent, and mappings multiply the
/// expected sets: thirty-two policies each mapped to thirty-two make the next certificate's every
/// policy a child of every node, past [`MAX_POLICY_NODES`], and the tree refuses to grow.
#[test]
fn a_tree_grown_by_mappings_past_the_bound_is_refused() {
    let mut tree = PolicyTree::new();
    let issued: Vec<Oid> = (0..32).map(policy).collect();
    tree.grow(1, &issued, false)
        .expect("thirty-two nodes are within the bound");
    let subject: Vec<Oid> = (100..132).map(policy).collect();
    let mappings: Vec<(Oid, Oid)> = issued
        .iter()
        .flat_map(|from| subject.iter().map(move |to| (from.clone(), to.clone())))
        .collect();
    tree.map(1, &mappings, true)
        .expect("mapping changes expected sets only");
    assert_eq!(tree.grow(2, &subject, false), Err(TooLarge));
}

/// Section 6.1.4 (b)(1) sets the expected set where mapping is allowed; (b)(2) deletes the node
/// where it is not, and the tree prunes to NULL.
#[test]
fn a_mapping_is_applied_where_allowed_and_deletes_where_inhibited() {
    let mappings = vec![(policy(1), policy(2))];
    let mut allowed = PolicyTree::new();
    allowed
        .grow(1, &[policy(1)], false)
        .expect("within the bound");
    allowed.map(1, &mappings, true).expect("within the bound");
    allowed
        .grow(2, &[policy(2)], false)
        .expect("within the bound");
    assert_eq!(
        allowed.admitted(2),
        vec![policy(1)],
        "reported in the anchor's domain"
    );
    let mut inhibited = PolicyTree::new();
    inhibited
        .grow(1, &[policy(1)], false)
        .expect("within the bound");
    inhibited
        .map(1, &mappings, false)
        .expect("within the bound");
    assert!(inhibited.is_null());
}

/// `SkipCerts` in either tagging, and the empty sequence section 4.2.1.11 forbids.
#[test]
fn policy_constraints_read_in_both_taggings() {
    let implicit = [0x30, 0x03, 0x80, 0x01, 0x02];
    let explicit = [0x30, 0x05, 0xA1, 0x03, 0x02, 0x01, 0x00];
    assert_eq!(
        read_policy_constraints(&implicit, Tagging::Implicit),
        Ok(Constraints {
            require_explicit_policy: Some(2),
            inhibit_policy_mapping: None
        })
    );
    assert_eq!(
        read_policy_constraints(&explicit, Tagging::Explicit),
        Ok(Constraints {
            require_explicit_policy: None,
            inhibit_policy_mapping: Some(0)
        })
    );
    assert_eq!(
        read_policy_constraints(&[0x30, 0x00], Tagging::Implicit),
        Err(Unreadable::Malformed)
    );
}
