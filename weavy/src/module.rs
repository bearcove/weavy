//! Semantic model and admission checks for durable Weavy modules.
//!
//! Physical encodings live in sibling codec crates. This module owns the
//! process-independent address space and the checks required before execution.

use core::fmt;
use std::collections::BTreeMap;
use std::sync::Arc;

use crate::BlockRef;
use crate::ir::{AggregateOp, ControlOp, DenseWeavyLowered, WeavyOp};
use phon_schema::{Schema, SchemaId};

const FEATURE_ID_DOMAIN: &[u8] = b"weavy.feature.v1\0";
const FUNCTION_KEY_DOMAIN: &[u8] = b"weavy.function-key.v1\0";
const GROUP_KEY_DOMAIN: &[u8] = b"weavy.group-key.v1\0";
const MEMBER_KEY_DOMAIN: &[u8] = b"weavy.member-key.v1\0";
const BLOCK_KEY_DOMAIN: &[u8] = b"weavy.block-key.v1\0";

/// Versioned nominal-key projection used by canonical semantic modules.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum TypeKeyScheme {
    NominalNameHashV1 = 0,
}

impl TypeKeyScheme {
    #[must_use]
    pub const fn tag(self) -> u8 {
        self as u8
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NominalNameHashV1 => "nominal_name_hash_v1",
        }
    }
}

/// Stable 128-bit nominal identity for one canonical function declaration.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FunctionKey([u8; 16]);

impl FunctionKey {
    pub fn new(canonical_name: impl AsRef<str>) -> Result<Self, CanonicalNameError> {
        global_key(FUNCTION_KEY_DOMAIN, canonical_name.as_ref()).map(Self)
    }

    #[must_use]
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }

    #[must_use]
    pub const fn into_bytes(self) -> [u8; 16] {
        self.0
    }
}

/// Stable 128-bit nominal identity for one recursive declared type group.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GroupKey([u8; 16]);

impl GroupKey {
    pub fn new(canonical_name: impl AsRef<str>) -> Result<Self, CanonicalNameError> {
        global_key(GROUP_KEY_DOMAIN, canonical_name.as_ref()).map(Self)
    }

    #[must_use]
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }

    #[must_use]
    pub const fn into_bytes(self) -> [u8; 16] {
        self.0
    }
}

/// Stable owner-scoped 32-bit identity for one member of a recursive type group.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MemberKey(u32);

impl MemberKey {
    pub fn new(
        owner: GroupKey,
        canonical_name: impl AsRef<str>,
    ) -> Result<Self, CanonicalNameError> {
        local_key(MEMBER_KEY_DOMAIN, owner.as_bytes(), canonical_name.as_ref()).map(Self)
    }

    #[must_use]
    pub const fn from_u32(value: u32) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn as_u32(self) -> u32 {
        self.0
    }

    #[must_use]
    pub const fn from_le_bytes(bytes: [u8; 4]) -> Self {
        Self(u32::from_le_bytes(bytes))
    }

    #[must_use]
    pub const fn into_le_bytes(self) -> [u8; 4] {
        self.0.to_le_bytes()
    }
}

/// Stable owner-scoped 32-bit identity for one block in a canonical function.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BlockKey(u32);

impl BlockKey {
    pub fn new(
        owner: FunctionKey,
        canonical_name: impl AsRef<str>,
    ) -> Result<Self, CanonicalNameError> {
        local_key(BLOCK_KEY_DOMAIN, owner.as_bytes(), canonical_name.as_ref()).map(Self)
    }

    #[must_use]
    pub const fn from_u32(value: u32) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn as_u32(self) -> u32 {
        self.0
    }

    #[must_use]
    pub const fn from_le_bytes(bytes: [u8; 4]) -> Self {
        Self(u32::from_le_bytes(bytes))
    }

    #[must_use]
    pub const fn into_le_bytes(self) -> [u8; 4] {
        self.0.to_le_bytes()
    }
}

/// Canonical semantic instruction identity, independent of physical byte offset.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InstId {
    function: FunctionKey,
    ordinal: u32,
}

impl InstId {
    #[must_use]
    pub const fn new(function: FunctionKey, ordinal: u32) -> Self {
        Self { function, ordinal }
    }

    #[must_use]
    pub const fn function(self) -> FunctionKey {
        self.function
    }

    #[must_use]
    pub const fn ordinal(self) -> u32 {
        self.ordinal
    }
}

/// Full cryptographic digest of one canonical recursive-type descriptor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypeDigest([u8; 32]);

impl TypeDigest {
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    #[must_use]
    pub const fn into_bytes(self) -> [u8; 32] {
        self.0
    }
}

fn global_key(domain: &[u8], canonical_name: &str) -> Result<[u8; 16], CanonicalNameError> {
    validate_canonical_name(canonical_name)?;
    let mut hasher = blake3::Hasher::new();
    hasher.update(domain);
    hasher.update(canonical_name.as_bytes());
    let mut key = [0; 16];
    key.copy_from_slice(&hasher.finalize().as_bytes()[..16]);
    Ok(key)
}

fn local_key(
    domain: &[u8],
    owner: &[u8; 16],
    canonical_name: &str,
) -> Result<u32, CanonicalNameError> {
    validate_canonical_name(canonical_name)?;
    let mut hasher = blake3::Hasher::new();
    hasher.update(domain);
    hasher.update(owner);
    hasher.update(canonical_name.as_bytes());
    Ok(u32::from_le_bytes(
        hasher.finalize().as_bytes()[..4]
            .try_into()
            .expect("digest prefix length"),
    ))
}

/// Closed semantic-feature namespaces with their frozen profile sort tags.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum FeatureNamespace {
    Opcode = 0,
    Helper = 1,
    Relation = 2,
    Capability = 3,
}

impl FeatureNamespace {
    #[must_use]
    pub const fn tag(self) -> u8 {
        self as u8
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Opcode => "opcode",
            Self::Helper => "helper",
            Self::Relation => "relation",
            Self::Capability => "capability",
        }
    }
}

/// A process-independent, namespace-separated semantic feature identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FeatureId([u8; 16]);

impl FeatureId {
    pub fn new(
        namespace: FeatureNamespace,
        canonical_name: impl AsRef<str>,
    ) -> Result<Self, CanonicalNameError> {
        let canonical_name = canonical_name.as_ref();
        validate_canonical_name(canonical_name)?;

        let mut hasher = blake3::Hasher::new();
        hasher.update(FEATURE_ID_DOMAIN);
        hasher.update(namespace.as_str().as_bytes());
        hasher.update(&[0]);
        hasher.update(canonical_name.as_bytes());
        let mut bytes = [0; 16];
        bytes.copy_from_slice(&hasher.finalize().as_bytes()[..16]);
        Ok(Self(bytes))
    }

    #[must_use]
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }

    #[must_use]
    pub const fn into_bytes(self) -> [u8; 16] {
        self.0
    }
}

/// Minimum semantic feature version required by a module.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FeatureRequirement {
    namespace: FeatureNamespace,
    stable_id: FeatureId,
    major: u16,
    min_minor: u16,
}

impl FeatureRequirement {
    #[must_use]
    pub const fn new(
        namespace: FeatureNamespace,
        stable_id: FeatureId,
        major: u16,
        min_minor: u16,
    ) -> Self {
        Self {
            namespace,
            stable_id,
            major,
            min_minor,
        }
    }

    #[must_use]
    pub const fn namespace(&self) -> FeatureNamespace {
        self.namespace
    }

    #[must_use]
    pub const fn stable_id(&self) -> FeatureId {
        self.stable_id
    }

    #[must_use]
    pub const fn major(&self) -> u16 {
        self.major
    }

    #[must_use]
    pub const fn min_minor(&self) -> u16 {
        self.min_minor
    }
}

/// Authority-preserving semantic feature descriptor for one compatible major line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeatureDescriptor {
    namespace: FeatureNamespace,
    canonical_name: Arc<str>,
    stable_id: FeatureId,
    major: u16,
    max_minor: u16,
    semantic_descriptor_digest: [u8; 32],
}

impl FeatureDescriptor {
    pub fn new(
        namespace: FeatureNamespace,
        canonical_name: impl AsRef<str>,
        major: u16,
        max_minor: u16,
        semantic_descriptor_digest: [u8; 32],
    ) -> Result<Self, CanonicalNameError> {
        let canonical_name = canonical_name.as_ref();
        let stable_id = FeatureId::new(namespace, canonical_name)?;
        Ok(Self {
            namespace,
            canonical_name: Arc::from(canonical_name),
            stable_id,
            major,
            max_minor,
            semantic_descriptor_digest,
        })
    }

    #[must_use]
    pub const fn namespace(&self) -> FeatureNamespace {
        self.namespace
    }

    #[must_use]
    pub fn canonical_name(&self) -> &str {
        &self.canonical_name
    }

    #[must_use]
    pub const fn stable_id(&self) -> FeatureId {
        self.stable_id
    }

    #[must_use]
    pub const fn major(&self) -> u16 {
        self.major
    }

    #[must_use]
    pub const fn max_minor(&self) -> u16 {
        self.max_minor
    }

    #[must_use]
    pub const fn semantic_descriptor_digest(&self) -> &[u8; 32] {
        &self.semantic_descriptor_digest
    }

    #[cfg(test)]
    fn with_stable_id_for_test(mut self, stable_id: FeatureId) -> Self {
        self.stable_id = stable_id;
        self
    }
}

/// One immutable runtime profile's supported semantic feature versions.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FeatureSupport {
    descriptors: Vec<FeatureDescriptor>,
}

impl FeatureSupport {
    pub fn new(
        descriptors: impl IntoIterator<Item = FeatureDescriptor>,
    ) -> Result<Self, FeatureSupportError> {
        let mut descriptors = descriptors.into_iter().collect::<Vec<_>>();
        descriptors.sort_unstable_by_key(|descriptor| {
            (
                descriptor.namespace.tag(),
                descriptor.stable_id,
                descriptor.major,
                descriptor.max_minor,
            )
        });

        let mut identities = BTreeMap::<FeatureId, (FeatureNamespace, Arc<str>)>::new();
        let mut identity_majors = BTreeMap::<(FeatureNamespace, FeatureId, u16), usize>::new();
        for (index, descriptor) in descriptors.iter().enumerate() {
            if let Some((namespace, canonical_name)) = identities.get(&descriptor.stable_id) {
                if *namespace != descriptor.namespace
                    || canonical_name.as_ref() != descriptor.canonical_name.as_ref()
                {
                    return Err(FeatureSupportError::FeatureIdCollision {
                        stable_id: descriptor.stable_id,
                        first_namespace: *namespace,
                        first_canonical_name: canonical_name.clone(),
                        second_namespace: descriptor.namespace,
                        second_canonical_name: descriptor.canonical_name.clone(),
                    });
                }
            } else {
                identities.insert(
                    descriptor.stable_id,
                    (descriptor.namespace, descriptor.canonical_name.clone()),
                );
            }

            let identity_major = (descriptor.namespace, descriptor.stable_id, descriptor.major);
            if let Some(&first_index) = identity_majors.get(&identity_major) {
                return Err(FeatureSupportError::DuplicateIdentityMajor {
                    namespace: descriptor.namespace,
                    stable_id: descriptor.stable_id,
                    canonical_name: descriptor.canonical_name.clone(),
                    major: descriptor.major,
                    first_index,
                    second_index: index,
                });
            }
            identity_majors.insert(identity_major, index);
        }

        Ok(Self { descriptors })
    }

    #[must_use]
    pub fn descriptors(&self) -> &[FeatureDescriptor] {
        &self.descriptors
    }

    #[must_use]
    pub fn supports(&self, required: &FeatureRequirement) -> bool {
        self.descriptors
            .binary_search_by_key(
                &(required.namespace.tag(), required.stable_id, required.major),
                |descriptor| {
                    (
                        descriptor.namespace.tag(),
                        descriptor.stable_id,
                        descriptor.major,
                    )
                },
            )
            .is_ok_and(|index| self.descriptors[index].max_minor >= required.min_minor)
    }
}

/// Why an immutable semantic feature-support collection was rejected.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FeatureSupportError {
    FeatureIdCollision {
        stable_id: FeatureId,
        first_namespace: FeatureNamespace,
        first_canonical_name: Arc<str>,
        second_namespace: FeatureNamespace,
        second_canonical_name: Arc<str>,
    },
    DuplicateIdentityMajor {
        namespace: FeatureNamespace,
        stable_id: FeatureId,
        canonical_name: Arc<str>,
        major: u16,
        first_index: usize,
        second_index: usize,
    },
}

impl fmt::Display for FeatureSupportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid semantic feature support: {self:?}")
    }
}

impl std::error::Error for FeatureSupportError {}

#[cfg(test)]
mod feature_support_tests {
    use super::*;

    #[test]
    fn rejects_stable_id_collisions_between_unequal_identity_descriptors() {
        let stable_id = FeatureId::from_bytes([42; 16]);
        let first =
            FeatureDescriptor::new(FeatureNamespace::Opcode, "core.opcode.first", 1, 0, [1; 32])
                .expect("valid descriptor")
                .with_stable_id_for_test(stable_id);
        let second = FeatureDescriptor::new(
            FeatureNamespace::Helper,
            "core.helper.second",
            2,
            0,
            [2; 32],
        )
        .expect("valid descriptor")
        .with_stable_id_for_test(stable_id);

        assert!(matches!(
            FeatureSupport::new([first, second]),
            Err(FeatureSupportError::FeatureIdCollision {
                stable_id: actual,
                first_namespace: FeatureNamespace::Opcode,
                second_namespace: FeatureNamespace::Helper,
                ..
            }) if actual == stable_id
        ));
    }

    #[test]
    fn rejects_stable_id_collisions_between_unequal_canonical_names() {
        let stable_id = FeatureId::from_bytes([43; 16]);
        let first =
            FeatureDescriptor::new(FeatureNamespace::Opcode, "core.opcode.first", 1, 0, [1; 32])
                .expect("valid descriptor")
                .with_stable_id_for_test(stable_id);
        let second = FeatureDescriptor::new(
            FeatureNamespace::Opcode,
            "core.opcode.second",
            2,
            0,
            [2; 32],
        )
        .expect("valid descriptor")
        .with_stable_id_for_test(stable_id);

        assert!(matches!(
            FeatureSupport::new([first, second]),
            Err(FeatureSupportError::FeatureIdCollision {
                stable_id: actual,
                first_namespace: FeatureNamespace::Opcode,
                second_namespace: FeatureNamespace::Opcode,
                ..
            }) if actual == stable_id
        ));
    }
}

/// A validated canonical policy identity.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PolicyKey(Arc<str>);

impl PolicyKey {
    pub fn new(key: impl AsRef<str>) -> Result<Self, CanonicalNameError> {
        let key = key.as_ref();
        validate_canonical_name(key)?;
        Ok(Self(Arc::from(key)))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for PolicyKey {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

/// One semantic policy descriptor paired with its authority-approved digest.
///
/// Digest computation stays outside core until Gate 0 freezes the exact
/// canonical PHON root and framing. Construction does not recompute or verify
/// the supplied digest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolicyDescriptor {
    policy_key: PolicyKey,
    major: u16,
    minor: u16,
    canonical_semantics: Vec<u8>,
    minor_digest: [u8; 32],
}

impl PolicyDescriptor {
    #[must_use]
    pub fn new_with_approved_digest(
        policy_key: PolicyKey,
        major: u16,
        minor: u16,
        canonical_semantics: Vec<u8>,
        minor_digest: [u8; 32],
    ) -> Self {
        Self {
            policy_key,
            major,
            minor,
            canonical_semantics,
            minor_digest,
        }
    }

    #[must_use]
    pub const fn policy_key(&self) -> &PolicyKey {
        &self.policy_key
    }

    #[must_use]
    pub const fn major(&self) -> u16 {
        self.major
    }

    #[must_use]
    pub const fn minor(&self) -> u16 {
        self.minor
    }

    #[must_use]
    pub fn canonical_semantics(&self) -> &[u8] {
        &self.canonical_semantics
    }

    #[must_use]
    pub const fn minor_digest(&self) -> &[u8; 32] {
        &self.minor_digest
    }
}

/// Minimum compatible policy semantics required by a module.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PolicyRequirement {
    policy_key: PolicyKey,
    major: u16,
    min_minor: u16,
    required_minor_digest: [u8; 32],
}

impl PolicyRequirement {
    #[must_use]
    pub const fn new(
        policy_key: PolicyKey,
        major: u16,
        min_minor: u16,
        required_minor_digest: [u8; 32],
    ) -> Self {
        Self {
            policy_key,
            major,
            min_minor,
            required_minor_digest,
        }
    }

    #[must_use]
    pub const fn policy_key(&self) -> &PolicyKey {
        &self.policy_key
    }

    #[must_use]
    pub const fn major(&self) -> u16 {
        self.major
    }

    #[must_use]
    pub const fn min_minor(&self) -> u16 {
        self.min_minor
    }

    #[must_use]
    pub const fn required_minor_digest(&self) -> &[u8; 32] {
        &self.required_minor_digest
    }
}

/// One runtime's complete append-only compatible policy-minor history.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolicyVersion {
    policy_key: PolicyKey,
    major: u16,
    max_minor: u16,
    compatible_minor_digests: Vec<[u8; 32]>,
}

impl PolicyVersion {
    pub fn new(
        policy_key: PolicyKey,
        major: u16,
        max_minor: u16,
        compatible_minor_digests: Vec<[u8; 32]>,
    ) -> Result<Self, PolicyHistoryError> {
        let expected = usize::from(max_minor) + 1;
        if compatible_minor_digests.len() != expected {
            return Err(PolicyHistoryError::HistoryLengthMismatch {
                max_minor,
                actual: compatible_minor_digests.len(),
            });
        }
        Ok(Self {
            policy_key,
            major,
            max_minor,
            compatible_minor_digests,
        })
    }

    pub fn from_descriptors(
        descriptors: impl IntoIterator<Item = PolicyDescriptor>,
    ) -> Result<Self, PolicyHistoryError> {
        let mut descriptors = descriptors.into_iter();
        let first = descriptors
            .next()
            .ok_or(PolicyHistoryError::EmptyDescriptorHistory)?;
        if first.minor != 0 {
            return Err(PolicyHistoryError::DescriptorMinorMismatch {
                index: 0,
                actual: first.minor,
            });
        }

        let policy_key = first.policy_key.clone();
        let major = first.major;
        let mut digests = vec![first.minor_digest];
        for (offset, descriptor) in descriptors.enumerate() {
            let index = offset + 1;
            if descriptor.policy_key != policy_key {
                return Err(PolicyHistoryError::DescriptorKeyMismatch {
                    index,
                    expected: policy_key,
                    actual: descriptor.policy_key,
                });
            }
            if descriptor.major != major {
                return Err(PolicyHistoryError::DescriptorMajorMismatch {
                    index,
                    expected: major,
                    actual: descriptor.major,
                });
            }
            let expected_minor = u16::try_from(index)
                .map_err(|_| PolicyHistoryError::DescriptorHistoryTooLong { actual: index + 1 })?;
            if descriptor.minor != expected_minor {
                return Err(PolicyHistoryError::DescriptorMinorMismatch {
                    index,
                    actual: descriptor.minor,
                });
            }
            digests.push(descriptor.minor_digest);
        }
        let max_minor = u16::try_from(digests.len() - 1).map_err(|_| {
            PolicyHistoryError::DescriptorHistoryTooLong {
                actual: digests.len(),
            }
        })?;
        Self::new(policy_key, major, max_minor, digests)
    }

    #[must_use]
    pub const fn policy_key(&self) -> &PolicyKey {
        &self.policy_key
    }

    #[must_use]
    pub const fn major(&self) -> u16 {
        self.major
    }

    #[must_use]
    pub const fn max_minor(&self) -> u16 {
        self.max_minor
    }

    #[must_use]
    pub fn compatible_minor_digests(&self) -> &[[u8; 32]] {
        &self.compatible_minor_digests
    }

    #[must_use]
    pub fn is_compatible_with(&self, required: &PolicyRequirement) -> bool {
        self.policy_key == required.policy_key
            && self.major == required.major
            && self.max_minor >= required.min_minor
            && self.compatible_minor_digests[usize::from(required.min_minor)]
                == required.required_minor_digest
    }

    pub fn extends(&self, previous: &Self) -> Result<(), PolicyHistoryError> {
        if self.policy_key != previous.policy_key {
            return Err(PolicyHistoryError::HistoryKeyMismatch {
                expected: previous.policy_key.clone(),
                actual: self.policy_key.clone(),
            });
        }
        if self.major != previous.major {
            return Err(PolicyHistoryError::HistoryMajorMismatch {
                expected: previous.major,
                actual: self.major,
            });
        }
        if self.max_minor < previous.max_minor {
            return Err(PolicyHistoryError::HistoryTruncated {
                previous_max_minor: previous.max_minor,
                max_minor: self.max_minor,
            });
        }
        for (minor, (actual, expected)) in self
            .compatible_minor_digests
            .iter()
            .zip(&previous.compatible_minor_digests)
            .enumerate()
        {
            if actual != expected {
                return Err(PolicyHistoryError::EarlierDigestChanged {
                    minor: u16::try_from(minor).expect("validated policy history length"),
                });
            }
        }
        Ok(())
    }
}

/// Owner disposition for one complete policy compatibility history.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PolicyDecision {
    Approved,
    Rejected,
    Deferred,
}

/// Typed result of resolving one owner policy decision.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PolicyResolution {
    Approved(PolicyVersion),
    Rejected { policy_key: PolicyKey, major: u16 },
    Deferred { policy_key: PolicyKey, major: u16 },
}

/// Source row mirrored by `owner-policy-decisions.styx`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolicyDecisionRow {
    pub policy_key: PolicyKey,
    pub major: u16,
    pub max_minor: u16,
    pub descriptors: Vec<PolicyDescriptor>,
    pub compatible_minor_digests: Vec<[u8; 32]>,
    pub affected_profiles: Vec<String>,
    pub affected_features: Vec<FeatureId>,
    pub decision: PolicyDecision,
    pub approval_reference: Arc<str>,
}

/// One immutable owner decision over a complete policy history.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolicyDecisionRecord {
    policy_key: PolicyKey,
    major: u16,
    max_minor: u16,
    descriptors: Vec<PolicyDescriptor>,
    compatible_minor_digests: Vec<[u8; 32]>,
    affected_profiles: Vec<String>,
    affected_features: Vec<FeatureId>,
    decision: PolicyDecision,
    approval_reference: Arc<str>,
    resolution: PolicyResolution,
}

impl PolicyDecisionRecord {
    pub fn new(mut row: PolicyDecisionRow) -> Result<Self, PolicyDecisionRecordError> {
        let version = PolicyVersion::from_descriptors(row.descriptors.clone())?;
        if version.policy_key() != &row.policy_key {
            return Err(PolicyDecisionRecordError::DeclaredPolicyKeyMismatch);
        }
        if version.major() != row.major {
            return Err(PolicyDecisionRecordError::DeclaredMajorMismatch {
                declared: row.major,
                derived: version.major(),
            });
        }
        if version.max_minor() != row.max_minor {
            return Err(PolicyDecisionRecordError::DeclaredMaxMinorMismatch {
                declared: row.max_minor,
                derived: version.max_minor(),
            });
        }
        if row.compatible_minor_digests.len() != version.compatible_minor_digests().len() {
            return Err(
                PolicyDecisionRecordError::CompatibleMinorDigestCountMismatch {
                    declared: row.compatible_minor_digests.len(),
                    derived: version.compatible_minor_digests().len(),
                },
            );
        }
        for (minor, (declared, derived)) in row
            .compatible_minor_digests
            .iter()
            .zip(version.compatible_minor_digests())
            .enumerate()
        {
            if declared != derived {
                return Err(PolicyDecisionRecordError::CompatibleMinorDigestMismatch {
                    minor: u16::try_from(minor).expect("validated policy history length"),
                });
            }
        }
        row.affected_profiles.sort_unstable();
        if let Some(index) = first_duplicate_index(&row.affected_profiles) {
            return Err(PolicyDecisionRecordError::DuplicateAffectedProfile { index });
        }
        row.affected_features.sort_unstable();
        if let Some(index) = first_duplicate_index(&row.affected_features) {
            return Err(PolicyDecisionRecordError::DuplicateAffectedFeature { index });
        }
        if row.approval_reference.is_empty() {
            return Err(PolicyDecisionRecordError::EmptyApprovalReference);
        }
        let resolution = match row.decision {
            PolicyDecision::Approved => PolicyResolution::Approved(version),
            PolicyDecision::Rejected => PolicyResolution::Rejected {
                policy_key: row.policy_key.clone(),
                major: row.major,
            },
            PolicyDecision::Deferred => PolicyResolution::Deferred {
                policy_key: row.policy_key.clone(),
                major: row.major,
            },
        };
        Ok(Self {
            policy_key: row.policy_key,
            major: row.major,
            max_minor: row.max_minor,
            descriptors: row.descriptors,
            compatible_minor_digests: row.compatible_minor_digests,
            affected_profiles: row.affected_profiles,
            affected_features: row.affected_features,
            decision: row.decision,
            approval_reference: row.approval_reference,
            resolution,
        })
    }

    #[must_use]
    pub const fn resolution(&self) -> &PolicyResolution {
        &self.resolution
    }
    #[must_use]
    pub const fn policy_key(&self) -> &PolicyKey {
        &self.policy_key
    }
    #[must_use]
    pub const fn major(&self) -> u16 {
        self.major
    }
    #[must_use]
    pub const fn max_minor(&self) -> u16 {
        self.max_minor
    }
    #[must_use]
    pub fn descriptors(&self) -> &[PolicyDescriptor] {
        &self.descriptors
    }
    #[must_use]
    pub fn compatible_minor_digests(&self) -> &[[u8; 32]] {
        &self.compatible_minor_digests
    }
    #[must_use]
    pub fn affected_profiles(&self) -> &[String] {
        &self.affected_profiles
    }
    #[must_use]
    pub fn affected_features(&self) -> &[FeatureId] {
        &self.affected_features
    }
    #[must_use]
    pub const fn decision(&self) -> PolicyDecision {
        self.decision
    }
    #[must_use]
    pub fn approval_reference(&self) -> &str {
        &self.approval_reference
    }
}

/// Why a source owner-policy row disagreed with its descriptor authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PolicyDecisionRecordError {
    History(PolicyHistoryError),
    DeclaredPolicyKeyMismatch,
    DeclaredMajorMismatch { declared: u16, derived: u16 },
    DeclaredMaxMinorMismatch { declared: u16, derived: u16 },
    CompatibleMinorDigestCountMismatch { declared: usize, derived: usize },
    CompatibleMinorDigestMismatch { minor: u16 },
    DuplicateAffectedProfile { index: usize },
    DuplicateAffectedFeature { index: usize },
    EmptyApprovalReference,
}

fn first_duplicate_index<T: PartialEq>(values: &[T]) -> Option<usize> {
    values
        .windows(2)
        .position(|pair| pair[0] == pair[1])
        .map(|index| index + 1)
}

impl From<PolicyHistoryError> for PolicyDecisionRecordError {
    fn from(error: PolicyHistoryError) -> Self {
        Self::History(error)
    }
}

impl fmt::Display for PolicyDecisionRecordError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid owner policy decision row: {self:?}")
    }
}

impl std::error::Error for PolicyDecisionRecordError {}

/// Canonically ordered owner authority with exactly one row per policy major.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PolicyDecisionAuthority {
    records: Vec<PolicyDecisionRecord>,
}

impl PolicyDecisionAuthority {
    pub fn new(
        records: impl IntoIterator<Item = PolicyDecisionRecord>,
    ) -> Result<Self, PolicyDecisionAuthorityError> {
        let mut records = records.into_iter().collect::<Vec<_>>();
        records.sort_unstable_by(|left, right| {
            left.policy_key()
                .cmp(right.policy_key())
                .then_with(|| left.major().cmp(&right.major()))
        });
        for (first_index, adjacent) in records.windows(2).enumerate() {
            if adjacent[0].policy_key() == adjacent[1].policy_key()
                && adjacent[0].major() == adjacent[1].major()
            {
                return Err(PolicyDecisionAuthorityError {
                    policy_key: adjacent[0].policy_key().clone(),
                    major: adjacent[0].major(),
                    first_index,
                    second_index: first_index + 1,
                });
            }
        }
        Ok(Self { records })
    }

    #[must_use]
    pub fn records(&self) -> &[PolicyDecisionRecord] {
        &self.records
    }
}

/// Duplicate owner authority for one canonical policy key and major.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolicyDecisionAuthorityError {
    policy_key: PolicyKey,
    major: u16,
    first_index: usize,
    second_index: usize,
}

impl PolicyDecisionAuthorityError {
    #[must_use]
    pub const fn policy_key(&self) -> &PolicyKey {
        &self.policy_key
    }

    #[must_use]
    pub const fn major(&self) -> u16 {
        self.major
    }

    #[must_use]
    pub const fn first_index(&self) -> usize {
        self.first_index
    }

    #[must_use]
    pub const fn second_index(&self) -> usize {
        self.second_index
    }
}

impl fmt::Display for PolicyDecisionAuthorityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "duplicate owner policy authority: {self:?}")
    }
}

impl std::error::Error for PolicyDecisionAuthorityError {}

/// Immutable semantic authority available to one runtime.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeSemanticSupport {
    feature_support: FeatureSupport,
    policy_versions: Vec<PolicyVersion>,
}

impl RuntimeSemanticSupport {
    pub fn new(
        feature_support: FeatureSupport,
        policy_versions: impl IntoIterator<Item = PolicyVersion>,
    ) -> Result<Self, RuntimeSemanticSupportError> {
        let mut policy_versions = policy_versions.into_iter().collect::<Vec<_>>();
        policy_versions.sort_unstable_by(|left, right| {
            left.policy_key
                .cmp(&right.policy_key)
                .then_with(|| left.major.cmp(&right.major))
                .then_with(|| left.max_minor.cmp(&right.max_minor))
                .then_with(|| {
                    left.compatible_minor_digests
                        .cmp(&right.compatible_minor_digests)
                })
        });

        for (first_index, adjacent) in policy_versions.windows(2).enumerate() {
            let first = &adjacent[0];
            let second = &adjacent[1];
            if first.policy_key == second.policy_key && first.major == second.major {
                return Err(RuntimeSemanticSupportError::DuplicatePolicyMajor {
                    policy_key: first.policy_key.clone(),
                    major: first.major,
                    first_index,
                    second_index: first_index + 1,
                });
            }
        }

        Ok(Self {
            feature_support,
            policy_versions,
        })
    }

    #[must_use]
    pub const fn feature_support(&self) -> &FeatureSupport {
        &self.feature_support
    }

    #[must_use]
    pub fn policy_versions(&self) -> &[PolicyVersion] {
        &self.policy_versions
    }

    #[must_use]
    pub fn supports_feature(&self, required: &FeatureRequirement) -> bool {
        self.feature_support.supports(required)
    }

    #[must_use]
    pub fn supports_policy(&self, required: &PolicyRequirement) -> bool {
        self.policy_versions
            .binary_search_by(|version| {
                version
                    .policy_key
                    .cmp(&required.policy_key)
                    .then_with(|| version.major.cmp(&required.major))
            })
            .is_ok_and(|index| self.policy_versions[index].is_compatible_with(required))
    }
}

/// Why immutable runtime semantic authority was rejected after canonical sorting.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeSemanticSupportError {
    /// Two sorted records claimed authority for the same policy key and major.
    /// Indices address the canonical [`RuntimeSemanticSupport::policy_versions`] order.
    DuplicatePolicyMajor {
        policy_key: PolicyKey,
        major: u16,
        first_index: usize,
        second_index: usize,
    },
}

impl fmt::Display for RuntimeSemanticSupportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid runtime semantic support: {self:?}")
    }
}

impl std::error::Error for RuntimeSemanticSupportError {}

/// Why a lowercase ASCII dotted identifier was rejected.
///
/// Segments use lowercase letters, digits, and underscores, matching the
/// normative catalog's dotted names such as `core.builder.init_field`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CanonicalNameError {
    Empty,
    EmptySegment { dot_index: usize },
    InvalidByte { index: usize, byte: u8 },
}

impl fmt::Display for CanonicalNameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid lowercase ASCII dotted canonical name: {self:?}")
    }
}

impl std::error::Error for CanonicalNameError {}

/// Why a policy descriptor or compatible-minor history was rejected.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PolicyHistoryError {
    EmptyDescriptorHistory,
    DescriptorHistoryTooLong {
        actual: usize,
    },
    DescriptorKeyMismatch {
        index: usize,
        expected: PolicyKey,
        actual: PolicyKey,
    },
    DescriptorMajorMismatch {
        index: usize,
        expected: u16,
        actual: u16,
    },
    DescriptorMinorMismatch {
        index: usize,
        actual: u16,
    },
    HistoryLengthMismatch {
        max_minor: u16,
        actual: usize,
    },
    HistoryKeyMismatch {
        expected: PolicyKey,
        actual: PolicyKey,
    },
    HistoryMajorMismatch {
        expected: u16,
        actual: u16,
    },
    HistoryTruncated {
        previous_max_minor: u16,
        max_minor: u16,
    },
    EarlierDigestChanged {
        minor: u16,
    },
}

impl fmt::Display for PolicyHistoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid policy compatibility history: {self:?}")
    }
}

impl std::error::Error for PolicyHistoryError {}

fn validate_canonical_name(name: &str) -> Result<(), CanonicalNameError> {
    if name.is_empty() {
        return Err(CanonicalNameError::Empty);
    }
    let bytes = name.as_bytes();
    for (index, &byte) in bytes.iter().enumerate() {
        match byte {
            b'.' if index == 0 || bytes[index - 1] == b'.' => {
                return Err(CanonicalNameError::EmptySegment { dot_index: index });
            }
            b'a'..=b'z' | b'0'..=b'9' | b'_' | b'.' => {}
            _ => return Err(CanonicalNameError::InvalidByte { index, byte }),
        }
    }
    if bytes.last() == Some(&b'.') {
        return Err(CanonicalNameError::EmptySegment {
            dot_index: bytes.len() - 1,
        });
    }
    Ok(())
}

/// Stable index in a module-local constant address space.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConstantId(u32);

impl ConstantId {
    #[must_use]
    pub const fn new(index: u32) -> Self {
        Self(index)
    }

    #[must_use]
    pub const fn index(self) -> u32 {
        self.0
    }
}

/// Stable index in a module-local typed constant-range address space.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConstantRangeId(u32);

impl ConstantRangeId {
    #[must_use]
    pub const fn new(index: u32) -> Self {
        Self(index)
    }

    #[must_use]
    pub const fn index(self) -> u32 {
        self.0
    }
}

/// PHON physical profile used by one typed constant range.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StorageProfile {
    Compact,
    Aligned,
    DenseAligned,
}

/// One homogeneous typed constant range.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConstantRange {
    schemas: Vec<Schema>,
    schema_id: SchemaId,
    profile: StorageProfile,
    count: u32,
    stride: u32,
    bytes: Vec<u8>,
}

impl ConstantRange {
    pub fn new(
        schemas: Vec<Schema>,
        root_index: usize,
        profile: StorageProfile,
        count: u32,
        stride: u32,
        bytes: Vec<u8>,
    ) -> Result<Self, ConstantRangeError> {
        if root_index >= schemas.len() {
            return Err(ConstantRangeError::InvalidRootIndex {
                index: root_index,
                schema_count: schemas.len(),
            });
        }
        let resolved = phon_schema::resolve_ids(schemas);
        let schema_id = resolved[root_index].id;
        Ok(Self {
            schemas: resolved,
            schema_id,
            profile,
            count,
            stride,
            bytes,
        })
    }

    #[must_use]
    pub fn schemas(&self) -> &[Schema] {
        &self.schemas
    }

    #[must_use]
    pub const fn schema_id(&self) -> SchemaId {
        self.schema_id
    }

    #[must_use]
    pub const fn profile(&self) -> StorageProfile {
        self.profile
    }

    #[must_use]
    pub const fn count(&self) -> u32 {
        self.count
    }

    #[must_use]
    pub const fn stride(&self) -> u32 {
        self.stride
    }

    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConstantRangeError {
    InvalidRootIndex { index: usize, schema_count: usize },
}

impl fmt::Display for ConstantRangeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid typed constant range: {self:?}")
    }
}

impl std::error::Error for ConstantRangeError {}

/// Metadata required to admit a borrowed constant range without owning its bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConstantRangeMetadata {
    pub schema_id: SchemaId,
    pub profile: StorageProfile,
}

/// One typed constant's schema identity and encoded PHON payload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Constant {
    schema_id: u64,
    bytes: Vec<u8>,
}

impl Constant {
    #[must_use]
    pub fn new(schema_id: u64, bytes: Vec<u8>) -> Self {
        Self { schema_id, bytes }
    }

    #[must_use]
    pub const fn schema_id(&self) -> u64 {
        self.schema_id
    }

    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Module-local typed constants addressed by [`ConstantId`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConstantPool {
    constants: Vec<Constant>,
}

impl ConstantPool {
    #[must_use]
    pub fn new(constants: Vec<Constant>) -> Self {
        Self { constants }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.constants.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.constants.is_empty()
    }

    #[must_use]
    pub fn get(&self, id: ConstantId) -> Option<&Constant> {
        self.constants.get(id.index() as usize)
    }

    #[must_use]
    pub fn range(&self, first: ConstantId, count: u32) -> Option<&[Constant]> {
        let start = first.index() as usize;
        let end = start.checked_add(count as usize)?;
        self.constants.get(start..end)
    }
}

impl core::ops::Index<usize> for ConstantPool {
    type Output = Constant;

    fn index(&self, index: usize) -> &Self::Output {
        &self.constants[index]
    }
}

impl core::ops::IndexMut<usize> for ConstantPool {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.constants[index]
    }
}

/// Required dialect name and compatible intrinsic-set version.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DialectRequirement {
    name: String,
    major: u16,
    minor: u16,
}

impl DialectRequirement {
    #[must_use]
    pub fn new(name: impl Into<String>, major: u16, minor: u16) -> Self {
        Self {
            name: name.into(),
            major,
            minor,
        }
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn major(&self) -> u16 {
        self.major
    }

    #[must_use]
    pub const fn minor(&self) -> u16 {
        self.minor
    }

    fn supports(&self, required: &Self) -> bool {
        self.name == required.name && self.major == required.major && self.minor >= required.minor
    }
}

/// Durable module metadata independent of its producing frontend.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleManifest {
    name: String,
    format_major: u16,
    format_minor: u16,
    dialects: Vec<DialectRequirement>,
    root_entries: Vec<u32>,
}

impl ModuleManifest {
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        dialects: impl IntoIterator<Item = DialectRequirement>,
        root_entries: impl IntoIterator<Item = u32>,
    ) -> Self {
        Self {
            name: name.into(),
            format_major: 1,
            format_minor: 0,
            dialects: dialects.into_iter().collect(),
            root_entries: root_entries.into_iter().collect(),
        }
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn format_major(&self) -> u16 {
        self.format_major
    }

    #[must_use]
    pub const fn format_minor(&self) -> u16 {
        self.format_minor
    }

    #[must_use]
    pub fn dialects(&self) -> &[DialectRequirement] {
        &self.dialects
    }

    #[must_use]
    pub fn root_entries(&self) -> &[u32] {
        &self.root_entries
    }
}

/// A self-contained semantic Weavy module.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WeavyModule<Intrinsic> {
    manifest: ModuleManifest,
    program: DenseWeavyLowered<Intrinsic>,
    constants: ConstantPool,
    constant_ranges: Vec<ConstantRange>,
}

impl<Intrinsic> WeavyModule<Intrinsic> {
    #[must_use]
    pub fn new(
        manifest: ModuleManifest,
        program: DenseWeavyLowered<Intrinsic>,
        constants: ConstantPool,
    ) -> Self {
        Self {
            manifest,
            program,
            constant_ranges: Vec::new(),
            constants,
        }
    }

    #[must_use]
    pub const fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    #[must_use]
    pub const fn program(&self) -> &DenseWeavyLowered<Intrinsic> {
        &self.program
    }

    #[must_use]
    pub fn program_mut(&mut self) -> &mut DenseWeavyLowered<Intrinsic> {
        &mut self.program
    }

    #[must_use]
    pub const fn constants(&self) -> &ConstantPool {
        &self.constants
    }

    #[must_use]
    pub fn constants_mut(&mut self) -> &mut ConstantPool {
        &mut self.constants
    }

    #[must_use]
    pub fn with_constant_ranges(mut self, constant_ranges: Vec<ConstantRange>) -> Self {
        self.constant_ranges = constant_ranges;
        self
    }

    #[must_use]
    pub fn constant_ranges(&self) -> &[ConstantRange] {
        &self.constant_ranges
    }

    #[must_use]
    pub fn constant_ranges_mut(&mut self) -> &mut [ConstantRange] {
        &mut self.constant_ranges
    }
}

/// One intrinsic reference to a typed constant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConstantReference {
    id: ConstantId,
    expected_schema: u64,
}

impl ConstantReference {
    #[must_use]
    pub const fn new(id: ConstantId, expected_schema: u64) -> Self {
        Self {
            id,
            expected_schema,
        }
    }
}

/// One intrinsic reference to a homogeneous typed constant range.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConstantRangeReference {
    id: ConstantRangeId,
    expected_schema: SchemaId,
    expected_profile: StorageProfile,
}

impl ConstantRangeReference {
    #[must_use]
    pub const fn new(
        id: ConstantRangeId,
        expected_schema: SchemaId,
        expected_profile: StorageProfile,
    ) -> Self {
        Self {
            id,
            expected_schema,
            expected_profile,
        }
    }

    #[must_use]
    pub const fn id(self) -> ConstantRangeId {
        self.id
    }

    #[must_use]
    pub const fn expected_schema(self) -> SchemaId {
        self.expected_schema
    }

    #[must_use]
    pub const fn expected_profile(self) -> StorageProfile {
        self.expected_profile
    }
}

/// Admission contract implemented by each durable intrinsic vocabulary.
pub trait IntrinsicContract {
    fn constant_references(&self, visit: &mut dyn FnMut(ConstantReference));

    fn constant_range_references(&self, _visit: &mut dyn FnMut(ConstantRangeReference)) {}
}

/// Admission context for one runtime's supported dialect set.
pub struct ModuleVerifier {
    dialects: BTreeMap<String, DialectRequirement>,
}

impl ModuleVerifier {
    #[must_use]
    pub fn new(dialects: impl IntoIterator<Item = DialectRequirement>) -> Self {
        Self {
            dialects: dialects
                .into_iter()
                .map(|dialect| (dialect.name.clone(), dialect))
                .collect(),
        }
    }

    pub fn admit<Intrinsic: IntrinsicContract>(
        &self,
        module: WeavyModule<Intrinsic>,
    ) -> Result<AdmittedModule<Intrinsic>, AdmissionError> {
        self.verify_manifest(module.manifest())?;
        verify_program(
            module.program(),
            module.constants(),
            module.constant_ranges(),
        )?;
        Ok(AdmittedModule { module })
    }
    pub fn verify_parts<Intrinsic: IntrinsicContract>(
        &self,
        manifest: &ModuleManifest,
        program: &DenseWeavyLowered<Intrinsic>,
        constants: &ConstantPool,
        constant_ranges: &[ConstantRangeMetadata],
    ) -> Result<(), AdmissionError> {
        self.verify_manifest(manifest)?;
        verify_program_metadata(program, constants, constant_ranges)
    }

    fn verify_manifest(&self, manifest: &ModuleManifest) -> Result<(), AdmissionError> {
        if manifest.format_major != 1 {
            return Err(AdmissionError::UnsupportedFormat {
                major: manifest.format_major,
                minor: manifest.format_minor,
            });
        }
        for required in &manifest.dialects {
            let Some(available) = self.dialects.get(&required.name) else {
                return Err(AdmissionError::MissingDialect {
                    name: required.name.clone(),
                });
            };
            if !available.supports(required) {
                return Err(AdmissionError::IncompatibleDialect {
                    name: required.name.clone(),
                    required_major: required.major,
                    required_minor: required.minor,
                    available_major: available.major,
                    available_minor: available.minor,
                });
            }
        }
        Ok(())
    }
}

fn verify_program<Intrinsic: IntrinsicContract>(
    program: &DenseWeavyLowered<Intrinsic>,
    constants: &ConstantPool,
    constant_ranges: &[ConstantRange],
) -> Result<(), AdmissionError> {
    verify_ops(
        &program.program,
        program.blocks.len(),
        constants,
        constant_ranges,
    )?;
    for block in &program.blocks {
        verify_ops(block, program.blocks.len(), constants, constant_ranges)?;
    }
    Ok(())
}

fn verify_ops<Intrinsic: IntrinsicContract>(
    ops: &[WeavyOp<BlockRef, Intrinsic>],
    block_count: usize,
    constants: &ConstantPool,
    constant_ranges: &[ConstantRange],
) -> Result<(), AdmissionError> {
    let metadata = constant_ranges
        .iter()
        .map(|range| ConstantRangeMetadata {
            schema_id: range.schema_id,
            profile: range.profile,
        })
        .collect::<Vec<_>>();
    verify_ops_metadata(ops, block_count, constants, &metadata)
}

fn verify_program_metadata<Intrinsic: IntrinsicContract>(
    program: &DenseWeavyLowered<Intrinsic>,
    constants: &ConstantPool,
    constant_ranges: &[ConstantRangeMetadata],
) -> Result<(), AdmissionError> {
    verify_ops_metadata(
        &program.program,
        program.blocks.len(),
        constants,
        constant_ranges,
    )?;
    for block in &program.blocks {
        verify_ops_metadata(block, program.blocks.len(), constants, constant_ranges)?;
    }
    Ok(())
}

fn verify_ops_metadata<Intrinsic: IntrinsicContract>(
    ops: &[WeavyOp<BlockRef, Intrinsic>],
    block_count: usize,
    constants: &ConstantPool,
    constant_ranges: &[ConstantRangeMetadata],
) -> Result<(), AdmissionError> {
    for op in ops {
        match op {
            WeavyOp::Control(ControlOp::CallBlock { block, .. }) => {
                verify_block(*block, block_count)?
            }
            WeavyOp::Control(ControlOp::CallBlockThen { block, then, .. }) => {
                verify_block(*block, block_count)?;
                verify_block(*then, block_count)?;
            }
            WeavyOp::Aggregate(AggregateOp::BeginList { loop_block, .. }) => {
                verify_block(*loop_block, block_count)?;
            }
            WeavyOp::Intrinsic(intrinsic) => {
                let mut error = None;
                intrinsic.constant_references(&mut |reference| {
                    if error.is_some() {
                        return;
                    }
                    let Some(constant) = constants.get(reference.id) else {
                        error = Some(AdmissionError::InvalidConstantId {
                            id: reference.id,
                            constant_count: constants.len(),
                        });
                        return;
                    };
                    if constant.schema_id != reference.expected_schema {
                        error = Some(AdmissionError::WrongConstantSchema {
                            id: reference.id,
                            expected: reference.expected_schema,
                            actual: constant.schema_id,
                        });
                    }
                });
                intrinsic.constant_range_references(&mut |reference| {
                    if error.is_some() {
                        return;
                    }
                    let Some(range) = constant_ranges.get(reference.id.index() as usize) else {
                        error = Some(AdmissionError::InvalidConstantRangeId {
                            id: reference.id,
                            range_count: constant_ranges.len(),
                        });
                        return;
                    };
                    if range.schema_id != reference.expected_schema {
                        error = Some(AdmissionError::WrongConstantRangeSchema {
                            id: reference.id,
                            expected: reference.expected_schema,
                            actual: range.schema_id,
                        });
                    } else if range.profile != reference.expected_profile {
                        error = Some(AdmissionError::WrongConstantRangeProfile {
                            id: reference.id,
                            expected: reference.expected_profile,
                            actual: range.profile,
                        });
                    }
                });
                if let Some(error) = error {
                    return Err(error);
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn verify_block(block: BlockRef, block_count: usize) -> Result<(), AdmissionError> {
    if block.index() >= block_count {
        Err(AdmissionError::InvalidBlockRef { block, block_count })
    } else {
        Ok(())
    }
}

/// A module that has passed all structural and intrinsic admission checks.
pub struct AdmittedModule<Intrinsic> {
    module: WeavyModule<Intrinsic>,
}

impl<Intrinsic> AdmittedModule<Intrinsic> {
    #[must_use]
    pub const fn module(&self) -> &WeavyModule<Intrinsic> {
        &self.module
    }

    #[must_use]
    pub fn into_module(self) -> WeavyModule<Intrinsic> {
        self.module
    }
}

/// Why a semantic module was rejected before execution.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum AdmissionError {
    UnsupportedFormat {
        major: u16,
        minor: u16,
    },
    MissingDialect {
        name: String,
    },
    IncompatibleDialect {
        name: String,
        required_major: u16,
        required_minor: u16,
        available_major: u16,
        available_minor: u16,
    },
    InvalidBlockRef {
        block: BlockRef,
        block_count: usize,
    },
    InvalidConstantId {
        id: ConstantId,
        constant_count: usize,
    },
    WrongConstantSchema {
        id: ConstantId,
        expected: u64,
        actual: u64,
    },
    InvalidConstantRangeId {
        id: ConstantRangeId,
        range_count: usize,
    },
    WrongConstantRangeSchema {
        id: ConstantRangeId,
        expected: SchemaId,
        actual: SchemaId,
    },
    WrongConstantRangeProfile {
        id: ConstantRangeId,
        expected: StorageProfile,
        actual: StorageProfile,
    },
}

impl fmt::Display for AdmissionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Weavy module admission failed: {self:?}")
    }
}

impl std::error::Error for AdmissionError {}
