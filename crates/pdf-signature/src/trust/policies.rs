//! RFC 5280 section 6.1's `valid_policy_tree`, and the four extensions that drive it.
//!
//! Section 6.1.2 (a) starts the tree with one `anyPolicy` node; section 6.1.3 (d) to (f) grows it
//! by each certificate's `certificatePolicies` and prunes it; section 6.1.4 (a) and (b) apply an
//! intermediate's `policyMappings`; section 6.1.4 (h) to (j) and section 6.1.5 (a), (b) and (g)
//! count down `explicit_policy`, `policy_mapping` and `inhibit_anyPolicy` and intersect the tree with
//! `user-initial-policy-set`. [`PolicyTree`] is the tree and its steps; [`crate::trust`]'s walk
//! holds the three counters and calls the steps in the section's order.
//!
//! The tree is kept as one vector of nodes per depth, each node naming its parent by index into the
//! depth above, and deletion marks a node rather than moving the others — so an index stays an
//! index for the life of the walk. Policy qualifiers are not kept: section 6.1.3 (d) carries them
//! into the tree for the application's use, and nothing in this program displays one.
//!
//! # Bounds
//!
//! Section 6.1.3 (d) creates one node per expected policy per parent, and a chain of policy
//! mappings can make that grow with every certificate — the shape of a known denial of service
//! against validators of this algorithm. At most [`MAX_POLICY_NODES`] nodes are created for one
//! path and at most [`MAX_POLICIES`] policies or mappings read from one extension; past either the
//! path is refused as this program's limit.

use crate::der::{INTEGER, OBJECT_IDENTIFIER, Reader, SEQUENCE, Value};

use super::names::Tagging;

/// `anyPolicy`, RFC 5280 section 4.2.1.4's `{ id-ce-certificatePolicies 0 }`, `2.5.29.32.0`.
pub const ANY_POLICY: &[u8] = &[0x55, 0x1D, 0x20, 0x00];

/// The most nodes the tree may grow to over one path.
///
/// Eight certificates of four policies each is thirty-two nodes; a thousand is far past any
/// hierarchy's and small enough that a hostile chain costs nothing.
pub const MAX_POLICY_NODES: usize = 1024;

/// The most policies one `certificatePolicies` or one `user-initial-policy-set` states, and the
/// most mappings one `policyMappings` does.
pub const MAX_POLICIES: usize = 32;

/// An object identifier's contents, owned.
pub type Oid = Vec<u8>;

/// Why one of the four extensions could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unreadable {
    /// The encoding is not the type's.
    Malformed,
    /// More entries than [`MAX_POLICIES`].
    PastBound,
}

/// `certificatePolicies`: the policy identifiers, qualifiers set aside — section 4.2.1.4.
///
/// # Errors
///
/// [`Unreadable`] where it is not a non-empty `SEQUENCE OF PolicyInformation`, states a policy
/// twice ("A certificate policy OID MUST NOT appear more than once"), or is past the bound.
pub fn read_certificate_policies(encoded: &[u8]) -> Result<Vec<Oid>, Unreadable> {
    let mut policies: Vec<Oid> = Vec::new();
    for information in sequence_of(encoded)? {
        let mut parts = children(&information)?;
        let policy = next(&mut parts)?
            .and_then(|value| value.object_identifier())
            .ok_or(Unreadable::Malformed)?;
        if policies.iter().any(|held| held == policy) {
            return Err(Unreadable::Malformed);
        }
        policies.push(policy.to_vec());
    }
    Ok(policies)
}

/// `policyMappings`: `(issuerDomainPolicy, subjectDomainPolicy)` pairs — section 4.2.1.5.
///
/// # Errors
///
/// [`Unreadable`] where it is not a non-empty `SEQUENCE OF` pairs of identifiers, or past the bound.
pub fn read_policy_mappings(encoded: &[u8]) -> Result<Vec<(Oid, Oid)>, Unreadable> {
    let mut mappings = Vec::new();
    for pair in sequence_of(encoded)? {
        let mut parts = children(&pair)?;
        let mut oid = || {
            next(&mut parts)?
                .and_then(|value| value.object_identifier())
                .map(<[u8]>::to_vec)
                .ok_or(Unreadable::Malformed)
        };
        let issuer = oid()?;
        let subject = oid()?;
        mappings.push((issuer, subject));
    }
    Ok(mappings)
}

/// A `PolicyConstraints`: `requireExplicitPolicy` and `inhibitPolicyMapping` — section 4.2.1.11.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Constraints {
    /// `requireExplicitPolicy`'s `SkipCerts`.
    pub require_explicit_policy: Option<usize>,
    /// `inhibitPolicyMapping`'s `SkipCerts`.
    pub inhibit_policy_mapping: Option<usize>,
}

/// Reads a `PolicyConstraints` under `tagging`.
///
/// # Errors
///
/// [`Unreadable::Malformed`] where it is not one, or states neither field, which section 4.2.1.11
/// forbids: "Conforming CAs MUST NOT issue certificates where policy constraints is an empty
/// sequence."
pub fn read_policy_constraints(
    encoded: &[u8],
    tagging: Tagging,
) -> Result<Constraints, Unreadable> {
    let value = single(encoded)?;
    if value.identifier != SEQUENCE {
        return Err(Unreadable::Malformed);
    }
    let mut read = Constraints::default();
    let mut members = children(&value)?;
    while let Some(member) = next(&mut members)? {
        let integer = match tagging {
            Tagging::Implicit if !member.is_constructed() => member.contents,
            Tagging::Explicit if member.is_constructed() => {
                let mut inner = children(&member)?;
                let value = next(&mut inner)?.ok_or(Unreadable::Malformed)?;
                if value.identifier != INTEGER || next(&mut inner)?.is_some() {
                    return Err(Unreadable::Malformed);
                }
                value.contents
            }
            _ => return Err(Unreadable::Malformed),
        };
        let count = skip_certs(integer)?;
        match member.tag_number() {
            0 if member.is_context(0) && read.require_explicit_policy.is_none() => {
                read.require_explicit_policy = Some(count);
            }
            1 if member.is_context(1) && read.inhibit_policy_mapping.is_none() => {
                read.inhibit_policy_mapping = Some(count);
            }
            _ => return Err(Unreadable::Malformed),
        }
    }
    if read == Constraints::default() {
        return Err(Unreadable::Malformed);
    }
    Ok(read)
}

/// `inhibitAnyPolicy`'s `SkipCerts` — section 4.2.1.14.
///
/// # Errors
///
/// [`Unreadable::Malformed`] where it is not a non-negative `INTEGER`.
pub fn read_inhibit_any_policy(encoded: &[u8]) -> Result<usize, Unreadable> {
    let value = single(encoded)?;
    if value.identifier != INTEGER {
        return Err(Unreadable::Malformed);
    }
    skip_certs(value.contents)
}

/// `SkipCerts ::= INTEGER (0..MAX)`, saturating where it is past what a path could count.
fn skip_certs(contents: &[u8]) -> Result<usize, Unreadable> {
    match contents.first() {
        None => Err(Unreadable::Malformed),
        Some(&first) if first & 0x80 != 0 => Err(Unreadable::Malformed),
        Some(_) => Ok(contents
            .iter()
            .try_fold(0_usize, |held, &octet| {
                held.checked_mul(256)
                    .and_then(|held| held.checked_add(usize::from(octet)))
            })
            .unwrap_or(usize::MAX)),
    }
}

/// One node of the tree — section 6.1.2 (a)'s `valid_policy` and `expected_policy_set`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Node {
    policy: Oid,
    expected: Vec<Oid>,
    /// The parent's index in the depth above; the root's is never read.
    parent: usize,
    alive: bool,
}

/// The tree grew past [`MAX_POLICY_NODES`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TooLarge;

/// Section 6.1.2 (a)'s `valid_policy_tree`, never NULL: a NULL tree is `None` in the caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyTree {
    levels: Vec<Vec<Node>>,
    created: usize,
}

impl Default for PolicyTree {
    fn default() -> Self {
        Self::new()
    }
}

impl PolicyTree {
    /// Section 6.1.2 (a): one node of depth zero, `anyPolicy` expecting `anyPolicy`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            levels: vec![vec![Node {
                policy: ANY_POLICY.to_vec(),
                expected: vec![ANY_POLICY.to_vec()],
                parent: 0,
                alive: true,
            }]],
            created: 1,
        }
    }

    /// Whether the root survived, which is whether the tree is not NULL.
    #[must_use]
    pub fn is_null(&self) -> bool {
        !self
            .levels
            .first()
            .and_then(|level| level.first())
            .is_some_and(|root| root.alive)
    }

    fn add(&mut self, depth: usize, node: Node) -> Result<(), TooLarge> {
        self.created = self.created.saturating_add(1);
        if self.created > MAX_POLICY_NODES {
            return Err(TooLarge);
        }
        while self.levels.len() <= depth {
            self.levels.push(Vec::new());
        }
        if let Some(level) = self.levels.get_mut(depth) {
            level.push(node);
        }
        Ok(())
    }

    fn alive(&self, depth: usize) -> impl Iterator<Item = (usize, &Node)> {
        self.levels
            .get(depth)
            .into_iter()
            .flatten()
            .enumerate()
            .filter(|(_, node)| node.alive)
    }

    fn has_child(&self, depth: usize, parent: usize, policy: Option<&[u8]>) -> bool {
        self.alive(depth.saturating_add(1)).any(|(_, child)| {
            child.parent == parent && policy.is_none_or(|policy| child.policy == policy)
        })
    }

    /// Section 6.1.3 (d)(1) to (3) for certificate `depth`: grows the tree by `policies` and prunes
    /// it. `any_allowed` is (d)(2)'s condition — `inhibit_anyPolicy` above zero, or a self-issued
    /// certificate that is not the last.
    ///
    /// # Errors
    ///
    /// [`TooLarge`] past [`MAX_POLICY_NODES`].
    pub fn grow(
        &mut self,
        depth: usize,
        policies: &[Oid],
        any_allowed: bool,
    ) -> Result<(), TooLarge> {
        let above = depth.saturating_sub(1);
        let mut created = Vec::new();
        // (d)(1): each policy other than anyPolicy, under every node expecting it, else under every
        // anyPolicy node.
        for policy in policies
            .iter()
            .filter(|policy| policy.as_slice() != ANY_POLICY)
        {
            let expecting: Vec<usize> = self
                .alive(above)
                .filter(|(_, node)| node.expected.contains(policy))
                .map(|(index, _)| index)
                .collect();
            let parents = if expecting.is_empty() {
                self.alive(above)
                    .filter(|(_, node)| node.policy == ANY_POLICY)
                    .map(|(index, _)| index)
                    .collect()
            } else {
                expecting
            };
            for parent in parents {
                created.push((parent, policy.clone()));
            }
        }
        for (parent, policy) in created {
            self.add(
                depth,
                Node {
                    expected: vec![policy.clone()],
                    policy,
                    parent,
                    alive: true,
                },
            )?;
        }
        // (d)(2): anyPolicy, where allowed, adds every expected policy a parent has no child for.
        if any_allowed && policies.iter().any(|policy| policy == ANY_POLICY) {
            let mut wanted = Vec::new();
            for (parent, node) in self.alive(above) {
                for expected in &node.expected {
                    if !self.has_child(above, parent, Some(expected))
                        && !wanted
                            .iter()
                            .any(|(p, e): &(usize, Oid)| *p == parent && e == expected)
                    {
                        wanted.push((parent, expected.clone()));
                    }
                }
            }
            for (parent, policy) in wanted {
                self.add(
                    depth,
                    Node {
                        expected: vec![policy.clone()],
                        policy,
                        parent,
                        alive: true,
                    },
                )?;
            }
        }
        // Make sure the depth exists even where nothing was created, so pruning sees it empty.
        while self.levels.len() <= depth {
            self.levels.push(Vec::new());
        }
        // (d)(3).
        self.prune(depth);
        Ok(())
    }

    /// Deletes every node above `depth` that has no child, depth by depth upwards — section
    /// 6.1.3 (d)(3)'s "Repeat this step until there are no nodes of depth i-1 or less without
    /// children".
    fn prune(&mut self, depth: usize) {
        for above in (0..depth).rev() {
            let childless: Vec<usize> = self
                .alive(above)
                .filter(|(index, _)| !self.has_child(above, *index, None))
                .map(|(index, _)| index)
                .collect();
            if let Some(level) = self.levels.get_mut(above) {
                for index in childless {
                    if let Some(node) = level.get_mut(index) {
                        node.alive = false;
                    }
                }
            }
        }
    }

    /// Marks every node whose parent is dead as dead too, from `depth` down.
    fn cascade(&mut self, depth: usize) {
        for below in depth.saturating_add(1)..self.levels.len() {
            let dead: Vec<usize> = self
                .levels
                .get(below.saturating_sub(1))
                .map(|level| {
                    level
                        .iter()
                        .enumerate()
                        .filter(|(_, node)| !node.alive)
                        .map(|(index, _)| index)
                        .collect()
                })
                .unwrap_or_default();
            if let Some(level) = self.levels.get_mut(below) {
                for node in level.iter_mut().filter(|node| dead.contains(&node.parent)) {
                    node.alive = false;
                }
            }
        }
    }

    /// Section 6.1.4 (b) for certificate `depth`: applies `mappings` where `mapping_allowed`
    /// (`policy_mapping` above zero), else deletes the issuer-domain policies they name.
    ///
    /// # Errors
    ///
    /// [`TooLarge`] past [`MAX_POLICY_NODES`].
    pub fn map(
        &mut self,
        depth: usize,
        mappings: &[(Oid, Oid)],
        mapping_allowed: bool,
    ) -> Result<(), TooLarge> {
        let mut issuers: Vec<&Oid> = Vec::new();
        for (issuer, _) in mappings {
            if !issuers.contains(&issuer) {
                issuers.push(issuer);
            }
        }
        for issuer in issuers {
            let mut mapped: Vec<Oid> = Vec::new();
            for (_, subject) in mappings.iter().filter(|(from, _)| from == issuer) {
                if !mapped.contains(subject) {
                    mapped.push(subject.clone());
                }
            }
            if mapping_allowed {
                // (b)(1).
                let mut found = false;
                if let Some(level) = self.levels.get_mut(depth) {
                    for node in level
                        .iter_mut()
                        .filter(|node| node.alive && node.policy == *issuer)
                    {
                        node.expected.clone_from(&mapped);
                        found = true;
                    }
                }
                let any_parent = if found {
                    None
                } else {
                    self.alive(depth)
                        .find(|(_, node)| node.policy == ANY_POLICY)
                        .map(|(_, node)| node.parent)
                };
                if let Some(parent) = any_parent {
                    self.add(
                        depth,
                        Node {
                            policy: issuer.clone(),
                            expected: mapped,
                            parent,
                            alive: true,
                        },
                    )?;
                }
            } else {
                // (b)(2).
                if let Some(level) = self.levels.get_mut(depth) {
                    for node in level.iter_mut().filter(|node| node.policy == *issuer) {
                        node.alive = false;
                    }
                }
                self.prune(depth);
            }
        }
        Ok(())
    }

    /// The policies the path is valid for, in the trust anchor's domain: for each node of depth
    /// `depth`, the `valid_policy` of its ancestor whose parent is `anyPolicy` — a member of section
    /// 6.1.5 (g)(iii)'s `valid_policy_node_set`, which is the set that step intersects with the
    /// caller's. Below a mapping a leaf's own `valid_policy` is a subject-domain policy, which the
    /// caller never named; its ancestor is the policy it was mapped from.
    #[must_use]
    pub fn admitted(&self, depth: usize) -> Vec<Oid> {
        let mut admitted: Vec<Oid> = Vec::new();
        for (index, _) in self.alive(depth) {
            let (mut at, mut index) = (depth, index);
            let policy = loop {
                let Some(node) = self.levels.get(at).and_then(|level| level.get(index)) else {
                    break None;
                };
                let parent_any = at == 0
                    || self
                        .levels
                        .get(at.saturating_sub(1))
                        .and_then(|level| level.get(node.parent))
                        .is_some_and(|parent| parent.policy == ANY_POLICY);
                if parent_any {
                    break Some(node.policy.clone());
                }
                at = at.saturating_sub(1);
                index = node.parent;
            };
            if let Some(policy) = policy
                && !admitted.contains(&policy)
            {
                admitted.push(policy);
            }
        }
        admitted
    }

    /// Section 6.1.5 (g)(iii): the intersection with a `user-initial-policy-set` that is not
    /// `any-policy`, for a path of `n` certificates.
    ///
    /// # Errors
    ///
    /// [`TooLarge`] past [`MAX_POLICY_NODES`].
    pub fn intersect(&mut self, n: usize, user: &[Oid]) -> Result<(), TooLarge> {
        // (1) the valid_policy_node_set: nodes whose parent's valid_policy is anyPolicy.
        let mut node_set: Vec<(usize, usize)> = Vec::new();
        for depth in 1..self.levels.len() {
            for (index, node) in self.alive(depth) {
                let parent_any = self
                    .levels
                    .get(depth.saturating_sub(1))
                    .and_then(|level| level.get(node.parent))
                    .is_some_and(|parent| parent.policy == ANY_POLICY);
                if parent_any {
                    node_set.push((depth, index));
                }
            }
        }
        // (2) delete those not in the user's set and not anyPolicy, with their children.
        for &(depth, index) in &node_set {
            if let Some(node) = self
                .levels
                .get_mut(depth)
                .and_then(|level| level.get_mut(index))
                && node.policy != ANY_POLICY
                && !user.contains(&node.policy)
            {
                node.alive = false;
            }
        }
        self.cascade(0);
        // (3) a depth-n anyPolicy node is replaced by the user's policies it does not already
        // reach, each a child of its parent.
        let any_leaf = self
            .alive(n)
            .find(|(_, node)| node.policy == ANY_POLICY)
            .map(|(index, node)| (index, node.parent));
        if let Some((index, parent)) = any_leaf {
            let reached: Vec<Oid> = node_set
                .iter()
                .filter_map(|&(depth, index)| {
                    self.levels.get(depth).and_then(|level| level.get(index))
                })
                .filter(|node| node.alive)
                .map(|node| node.policy.clone())
                .collect();
            for policy in user.iter().filter(|policy| !reached.contains(policy)) {
                self.add(
                    n,
                    Node {
                        policy: policy.clone(),
                        expected: vec![policy.clone()],
                        parent,
                        alive: true,
                    },
                )?;
            }
            if let Some(node) = self
                .levels
                .get_mut(n)
                .and_then(|level| level.get_mut(index))
            {
                node.alive = false;
            }
        }
        // (4).
        self.prune(n);
        Ok(())
    }
}

/// The values of a non-empty `SEQUENCE OF SEQUENCE`, bounded at [`MAX_POLICIES`].
fn sequence_of(encoded: &[u8]) -> Result<Vec<Value<'_>>, Unreadable> {
    let list = single(encoded)?;
    if list.identifier != SEQUENCE {
        return Err(Unreadable::Malformed);
    }
    let mut entries = children(&list)?;
    let mut values = Vec::new();
    while let Some(entry) = next(&mut entries)? {
        if entry.identifier != SEQUENCE {
            return Err(Unreadable::Malformed);
        }
        if values.len() >= MAX_POLICIES {
            return Err(Unreadable::PastBound);
        }
        values.push(entry);
    }
    if values.is_empty() {
        return Err(Unreadable::Malformed);
    }
    Ok(values)
}

/// Reads an `AcceptablePolicySet ::= SEQUENCE OF CertPolicyId` (RFC 3125, ETSI TS 119 172-3).
///
/// # Errors
///
/// [`Unreadable`] where it is not one, or past the bound.
pub fn read_policy_set(value: &Value<'_>) -> Result<Vec<Oid>, Unreadable> {
    if value.identifier != SEQUENCE {
        return Err(Unreadable::Malformed);
    }
    let mut entries = children(value)?;
    let mut policies: Vec<Oid> = Vec::new();
    while let Some(entry) = next(&mut entries)? {
        if entry.identifier != OBJECT_IDENTIFIER {
            return Err(Unreadable::Malformed);
        }
        if policies.len() >= MAX_POLICIES {
            return Err(Unreadable::PastBound);
        }
        if !policies.iter().any(|held| held == entry.contents) {
            policies.push(entry.contents.to_vec());
        }
    }
    Ok(policies)
}

fn single(encoded: &[u8]) -> Result<Value<'_>, Unreadable> {
    let mut reader = Reader::new(encoded).map_err(|_| Unreadable::Malformed)?;
    let value = next(&mut reader)?.ok_or(Unreadable::Malformed)?;
    if next(&mut reader)?.is_some() {
        return Err(Unreadable::Malformed);
    }
    Ok(value)
}

fn children<'a>(value: &Value<'a>) -> Result<Reader<'a>, Unreadable> {
    value.children().map_err(|_| Unreadable::Malformed)
}

fn next<'a>(reader: &mut Reader<'a>) -> Result<Option<Value<'a>>, Unreadable> {
    reader.next_value().map_err(|_| Unreadable::Malformed)
}

#[cfg(test)]
mod tests;
