use std::sync::Arc;

use weavy::{
    FeatureId, PolicyDecision, PolicyDecisionRecord, PolicyDecisionRecordError, PolicyDecisionRow,
    PolicyDescriptor, PolicyKey, PolicyResolution, PolicyVersion,
};

const ZERO_DIGEST: [u8; 32] = [0; 32];
const ONE_DIGEST: [u8; 32] = [1; 32];

fn descriptor(key: &PolicyKey, minor: u16, digest: [u8; 32]) -> PolicyDescriptor {
    PolicyDescriptor::new_with_approved_digest(key.clone(), 1, minor, vec![minor as u8], digest)
}

fn feature(byte: u8) -> FeatureId {
    FeatureId::from_bytes([byte; 16])
}

fn row(
    key: &PolicyKey,
    max_minor: u16,
    descriptors: Vec<PolicyDescriptor>,
    digests: Vec<[u8; 32]>,
    decision: PolicyDecision,
    reference: &str,
) -> PolicyDecisionRow {
    PolicyDecisionRow {
        policy_key: key.clone(),
        major: 1,
        max_minor,
        descriptors,
        compatible_minor_digests: digests,
        affected_profiles: Vec::new(),
        affected_features: Vec::new(),
        decision,
        approval_reference: Arc::from(reference),
    }
}

#[test]
fn approved_decision_emits_the_identical_policy_version() {
    let key = PolicyKey::new("core.integer.width").expect("valid policy key");
    let descriptors = vec![
        descriptor(&key, 0, ZERO_DIGEST),
        descriptor(&key, 1, ONE_DIGEST),
    ];
    let mut source = row(
        &key,
        1,
        descriptors.clone(),
        vec![ZERO_DIGEST, ONE_DIGEST],
        PolicyDecision::Approved,
        "owner:gate0-policy-1",
    );
    source.affected_profiles = vec!["portable".into(), "native".into()];
    source.affected_features = vec![feature(2), feature(1)];
    let record = PolicyDecisionRecord::new(source).expect("valid approved decision");

    let PolicyResolution::Approved(version) = record.resolution() else {
        panic!("approved decision did not emit a policy version")
    };
    assert_eq!(
        version,
        &PolicyVersion::new(key, 1, 1, vec![ZERO_DIGEST, ONE_DIGEST])
            .expect("valid expected history")
    );
    assert_eq!(record.affected_profiles(), &["native", "portable"]);
    assert_eq!(record.affected_features(), &[feature(1), feature(2)]);
    assert_eq!(record.descriptors(), descriptors);
    assert_eq!(
        record.compatible_minor_digests(),
        &[ZERO_DIGEST, ONE_DIGEST]
    );
    assert_eq!(record.decision(), PolicyDecision::Approved);
}

#[test]
fn deferred_and_rejected_decisions_are_distinct_typed_blockers() {
    let key = PolicyKey::new("core.unicode.version").expect("valid policy key");
    let deferred = PolicyDecisionRecord::new(row(
        &key,
        0,
        vec![descriptor(&key, 0, ZERO_DIGEST)],
        vec![ZERO_DIGEST],
        PolicyDecision::Deferred,
        "owner:defer",
    ))
    .expect("valid deferred decision");
    let rejected = PolicyDecisionRecord::new(row(
        &key,
        0,
        vec![descriptor(&key, 0, ZERO_DIGEST)],
        vec![ZERO_DIGEST],
        PolicyDecision::Rejected,
        "owner:reject",
    ))
    .expect("valid rejected decision");

    assert_eq!(
        deferred.resolution(),
        &PolicyResolution::Deferred {
            policy_key: key.clone(),
            major: 1,
        }
    );
    assert_eq!(
        rejected.resolution(),
        &PolicyResolution::Rejected {
            policy_key: key,
            major: 1,
        }
    );
}

#[test]
fn policy_decision_authority_rejects_duplicate_key_major_rows() {
    let key = PolicyKey::new("core.integer.width").expect("valid policy key");
    let first = PolicyDecisionRecord::new(row(
        &key,
        0,
        vec![descriptor(&key, 0, ZERO_DIGEST)],
        vec![ZERO_DIGEST],
        PolicyDecision::Approved,
        "owner:first",
    ))
    .expect("valid first decision");
    let second = PolicyDecisionRecord::new(row(
        &key,
        1,
        vec![
            descriptor(&key, 0, ZERO_DIGEST),
            descriptor(&key, 1, ONE_DIGEST),
        ],
        vec![ZERO_DIGEST, ONE_DIGEST],
        PolicyDecision::Approved,
        "owner:second",
    ))
    .expect("valid second decision");

    let error = weavy::PolicyDecisionAuthority::new([second, first])
        .expect_err("duplicate policy authority must reject");
    assert_eq!(error.policy_key(), &key);
    assert_eq!(error.major(), 1);
    assert_eq!((error.first_index(), error.second_index()), (0, 1));
}

#[test]
fn policy_decision_rows_cross_validate_declared_history() {
    let key = PolicyKey::new("core.integer.width").expect("valid policy key");
    let descriptors = vec![
        descriptor(&key, 0, ZERO_DIGEST),
        descriptor(&key, 1, ONE_DIGEST),
    ];

    let wrong_max_minor = PolicyDecisionRecord::new(row(
        &key,
        0,
        descriptors.clone(),
        vec![ZERO_DIGEST, ONE_DIGEST],
        PolicyDecision::Approved,
        "owner:wrong-max",
    ));
    assert!(matches!(
        wrong_max_minor,
        Err(PolicyDecisionRecordError::DeclaredMaxMinorMismatch {
            declared: 0,
            derived: 1,
        })
    ));

    let wrong_digests = PolicyDecisionRecord::new(row(
        &key,
        1,
        descriptors,
        vec![ZERO_DIGEST, ZERO_DIGEST],
        PolicyDecision::Approved,
        "owner:wrong-digests",
    ));
    assert!(matches!(
        wrong_digests,
        Err(PolicyDecisionRecordError::CompatibleMinorDigestMismatch { minor: 1 })
    ));
}

#[test]
fn policy_decision_rows_reject_duplicate_source_entries_and_empty_approval() {
    let key = PolicyKey::new("core.integer.width").expect("valid policy key");
    let base_descriptors = vec![descriptor(&key, 0, ZERO_DIGEST)];

    let mut duplicate_profile = row(
        &key,
        0,
        base_descriptors.clone(),
        vec![ZERO_DIGEST],
        PolicyDecision::Approved,
        "owner:duplicate-profile",
    );
    duplicate_profile.affected_profiles = vec!["portable".into(), "portable".into()];
    assert!(matches!(
        PolicyDecisionRecord::new(duplicate_profile),
        Err(PolicyDecisionRecordError::DuplicateAffectedProfile { index: 1 })
    ));

    let mut duplicate_feature = row(
        &key,
        0,
        base_descriptors.clone(),
        vec![ZERO_DIGEST],
        PolicyDecision::Approved,
        "owner:duplicate-feature",
    );
    duplicate_feature.affected_features = vec![feature(1), feature(1)];
    assert!(matches!(
        PolicyDecisionRecord::new(duplicate_feature),
        Err(PolicyDecisionRecordError::DuplicateAffectedFeature { index: 1 })
    ));

    assert!(matches!(
        PolicyDecisionRecord::new(row(
            &key,
            0,
            base_descriptors,
            vec![ZERO_DIGEST],
            PolicyDecision::Approved,
            "",
        )),
        Err(PolicyDecisionRecordError::EmptyApprovalReference)
    ));
}
