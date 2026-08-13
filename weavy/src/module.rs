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
