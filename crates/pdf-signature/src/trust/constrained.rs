//! RFC 5280 section 6.1's policy and name state over one prospective path, step by lettered step.
//!
//! [`super::Search::walk`] calls [`State::certificate`] where section 6.1.3 processes certificate
//! `i`, [`State::prepare`] where section 6.1.4 prepares for certificate `i + 1`, and
//! [`State::wrap_up`] for section 6.1.5. Each method names the steps it performs; the key, name
//! and length steps the same sections letter stay in [`super`].

use super::names::{self, NameRefusal, NameState, Tagging};
use super::policies::{self, Oid, PolicyTree};
use super::{PathInputs, PathRefusal};
use crate::x509::Certificate;

/// What section 6.1.6 outputs of the policy tree, as two sets of policies.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Policies {
    /// The policies the authorities on the path admit, before section 6.1.5 (g)'s intersection, in
    /// the trust anchor's domain ([`PolicyTree::admitted`]). `anyPolicy` among them admits every
    /// policy.
    pub authorities_constrained: Vec<Oid>,
    /// The same after the intersection with `user-initial-policy-set` — the policies the path is
    /// valid for under the caller's inputs. Empty where the tree is NULL.
    pub user_constrained: Vec<Oid>,
}

/// Section 6.1.2's `valid_policy_tree`, `permitted_subtrees`, `excluded_subtrees`,
/// `explicit_policy`, `inhibit_anyPolicy` and `policy_mapping`.
#[derive(Debug, Clone)]
pub(super) struct State {
    tree: Option<PolicyTree>,
    names: NameState,
    explicit_policy: usize,
    policy_mapping: usize,
    inhibit_any_policy: usize,
}

impl State {
    /// Section 6.1.2 (a) to (e), (j) and the initial subtrees (b) and (c), for a path of `n`.
    ///
    /// Each counter starts at `n + 1`, or at the input's `SkipCerts` where that is smaller — so an
    /// input of zero is the section's boolean set, and a trust point's `PolicyConstraints` applies
    /// as though the trust point were certificate zero (ETSI TS 119 172-2 clause 4.22.4.1).
    pub(super) fn new(inputs: &PathInputs, n: usize) -> Result<Self, PathRefusal> {
        let start = |skip: Option<usize>| {
            skip.map_or(n.saturating_add(1), |skip| skip.min(n.saturating_add(1)))
        };
        let mut names = NameState::default();
        if let Some(constraints) = &inputs.name_constraints {
            names
                .admit(constraints.clone())
                .map_err(|_| PathRefusal::ExtensionPastBound("nameConstraints"))?;
        }
        Ok(Self {
            tree: Some(PolicyTree::new()),
            names,
            explicit_policy: start(inputs.require_explicit_policy),
            policy_mapping: start(inputs.inhibit_policy_mapping),
            inhibit_any_policy: start(inputs.inhibit_any_policy),
        })
    }

    /// Section 6.1.3 (b) to (f) for certificate `i` of `n`.
    pub(super) fn certificate(
        &mut self,
        certificate: &Certificate<'_>,
        i: usize,
        n: usize,
    ) -> Result<(), PathRefusal> {
        let self_issued = certificate.subject == certificate.issuer;
        // (b) and (c), skipped for a self-issued certificate that is not the last.
        if (!self_issued || i >= n) && !self.names.is_empty() {
            let names = names::names_of(
                certificate.subject_encoding,
                certificate.extensions.subject_alt_name,
            )
            .map_err(|why| unreadable_name("subjectAltName", why))?;
            self.names.check(&names).map_err(name_refusal)?;
        }
        // (d) and (e).
        match certificate.extensions.certificate_policies {
            Some(encoded) => {
                let stated = policies::read_certificate_policies(encoded)
                    .map_err(|why| unreadable_policy("certificatePolicies", why))?;
                if let Some(tree) = &mut self.tree {
                    let any_allowed = self.inhibit_any_policy > 0 || (i < n && self_issued);
                    tree.grow(i, &stated, any_allowed)
                        .map_err(|_| PathRefusal::PolicyTreeTooLarge)?;
                    if tree.is_null() {
                        self.tree = None;
                    }
                }
            }
            None => self.tree = None,
        }
        // (f).
        if self.explicit_policy == 0 && self.tree.is_none() {
            return Err(PathRefusal::NoAcceptablePolicy);
        }
        Ok(())
    }

    /// Section 6.1.4 (a), (b) and (g) to (j) for certificate `i`, which signs the next.
    pub(super) fn prepare(
        &mut self,
        certificate: &Certificate<'_>,
        i: usize,
    ) -> Result<(), PathRefusal> {
        let extensions = &certificate.extensions;
        // (a) and (b).
        if let Some(encoded) = extensions.policy_mappings {
            let mappings = policies::read_policy_mappings(encoded)
                .map_err(|why| unreadable_policy("policyMappings", why))?;
            if mappings.iter().any(|(issuer, subject)| {
                issuer == policies::ANY_POLICY || subject == policies::ANY_POLICY
            }) {
                return Err(PathRefusal::PolicyMappingOfAnyPolicy);
            }
            if let Some(tree) = &mut self.tree {
                tree.map(i, &mappings, self.policy_mapping > 0)
                    .map_err(|_| PathRefusal::PolicyTreeTooLarge)?;
                if tree.is_null() {
                    self.tree = None;
                }
            }
        }
        // (g).
        if let Some(encoded) = extensions.name_constraints {
            let constraints = names::read_constraints(encoded, Tagging::Implicit)
                .map_err(|why| unreadable_name("nameConstraints", why))?;
            self.names
                .admit(constraints)
                .map_err(|_| PathRefusal::ExtensionPastBound("nameConstraints"))?;
        }
        // (h).
        if certificate.subject != certificate.issuer {
            for counter in [
                &mut self.explicit_policy,
                &mut self.policy_mapping,
                &mut self.inhibit_any_policy,
            ] {
                *counter = counter.saturating_sub(1);
            }
        }
        // (i).
        if let Some(encoded) = extensions.policy_constraints {
            let stated = policies::read_policy_constraints(encoded, Tagging::Implicit)
                .map_err(|why| unreadable_policy("policyConstraints", why))?;
            if let Some(skip) = stated.require_explicit_policy {
                self.explicit_policy = self.explicit_policy.min(skip);
            }
            if let Some(skip) = stated.inhibit_policy_mapping {
                self.policy_mapping = self.policy_mapping.min(skip);
            }
        }
        // (j).
        if let Some(encoded) = extensions.inhibit_any_policy {
            let skip = policies::read_inhibit_any_policy(encoded)
                .map_err(|why| unreadable_policy("inhibitAnyPolicy", why))?;
            self.inhibit_any_policy = self.inhibit_any_policy.min(skip);
        }
        Ok(())
    }

    /// Section 6.1.5 (a), (b) and (g) after certificate `n`, and the success condition.
    pub(super) fn wrap_up(
        mut self,
        last: &Certificate<'_>,
        n: usize,
        inputs: &PathInputs,
    ) -> Result<Policies, PathRefusal> {
        // (a).
        self.explicit_policy = self.explicit_policy.saturating_sub(1);
        // (b).
        if let Some(encoded) = last.extensions.policy_constraints {
            let stated = policies::read_policy_constraints(encoded, Tagging::Implicit)
                .map_err(|why| unreadable_policy("policyConstraints", why))?;
            if stated.require_explicit_policy == Some(0) {
                self.explicit_policy = 0;
            }
        }
        // (g).
        let authorities_constrained = self
            .tree
            .as_ref()
            .map(|tree| tree.admitted(n))
            .unwrap_or_default();
        if let (Some(tree), Some(user)) = (&mut self.tree, &inputs.policy_set) {
            tree.intersect(n, user)
                .map_err(|_| PathRefusal::PolicyTreeTooLarge)?;
            if tree.is_null() {
                self.tree = None;
            }
        }
        let user_constrained = self
            .tree
            .as_ref()
            .map(|tree| tree.admitted(n))
            .unwrap_or_default();
        // "If either (1) the value of explicit_policy variable is greater than zero or (2) the
        // valid_policy_tree is not NULL, then path processing has succeeded."
        if self.explicit_policy == 0 && self.tree.is_none() {
            return Err(PathRefusal::NoAcceptablePolicy);
        }
        Ok(Policies {
            authorities_constrained,
            user_constrained,
        })
    }
}

fn unreadable_policy(extension: &'static str, why: policies::Unreadable) -> PathRefusal {
    match why {
        policies::Unreadable::Malformed => PathRefusal::ExtensionUnreadable(extension),
        policies::Unreadable::PastBound => PathRefusal::ExtensionPastBound(extension),
    }
}

fn unreadable_name(extension: &'static str, why: names::Unreadable) -> PathRefusal {
    match why {
        names::Unreadable::Malformed => PathRefusal::ExtensionUnreadable(extension),
        names::Unreadable::PastBound => PathRefusal::ExtensionPastBound(extension),
        names::Unreadable::DistanceStated => PathRefusal::SubtreeDistanceStated,
    }
}

fn name_refusal(why: NameRefusal) -> PathRefusal {
    match why {
        NameRefusal::NotPermitted(form) => PathRefusal::NameNotPermitted(names::form_word(form)),
        NameRefusal::Excluded(form) => PathRefusal::NameExcluded(names::form_word(form)),
        NameRefusal::UriWithoutDomainHost => PathRefusal::UriWithoutDomainHost,
        NameRefusal::FormNotProcessed(form) => {
            PathRefusal::NameFormNotProcessed(names::form_word(form))
        }
        NameRefusal::Undecided(form) => {
            PathRefusal::NameComparisonUndecided(names::form_word(form))
        }
    }
}
