use weavy::{
    FeatureDescriptor, FeatureId, FeatureNamespace, FeatureRequirement, FeatureSupport, PolicyKey,
    PolicyRequirement, PolicyVersion, RuntimeSemanticSupport, RuntimeSemanticSupportError,
};

const ZERO_DIGEST: [u8; 32] = [0; 32];
const ONE_DIGEST: [u8; 32] = [1; 32];
const TWO_DIGEST: [u8; 32] = [2; 32];

fn feature_support() -> (FeatureSupport, FeatureId) {
    let descriptor = FeatureDescriptor::new(
        FeatureNamespace::Helper,
        "core.helper.decode",
        2,
        4,
        [9; 32],
    )
    .expect("valid feature descriptor");
    let stable_id = descriptor.stable_id();
    let support = FeatureSupport::new([descriptor]).expect("valid feature support");
    (support, stable_id)
}

fn policy_version(key: &str, major: u16, compatible_minor_digests: Vec<[u8; 32]>) -> PolicyVersion {
    let max_minor =
        u16::try_from(compatible_minor_digests.len() - 1).expect("test policy history fits in u16");
    PolicyVersion::new(
        PolicyKey::new(key).expect("valid policy key"),
        major,
        max_minor,
        compatible_minor_digests,
    )
    .expect("valid policy history")
}

#[test]
fn runtime_support_answers_exact_feature_requirements() {
    let (features, stable_id) = feature_support();
    let support = RuntimeSemanticSupport::new(features, []).expect("valid runtime support");

    assert!(support.supports_feature(&FeatureRequirement::new(
        FeatureNamespace::Helper,
        stable_id,
        2,
        4,
    )));
    assert!(!support.supports_feature(&FeatureRequirement::new(
        FeatureNamespace::Helper,
        FeatureId::from_bytes([0xff; 16]),
        2,
        4,
    )));
    assert!(!support.supports_feature(&FeatureRequirement::new(
        FeatureNamespace::Helper,
        stable_id,
        1,
        4,
    )));
    assert!(!support.supports_feature(&FeatureRequirement::new(
        FeatureNamespace::Opcode,
        stable_id,
        2,
        4,
    )));
    assert!(!support.supports_feature(&FeatureRequirement::new(
        FeatureNamespace::Helper,
        stable_id,
        2,
        5,
    )));
}

#[test]
fn runtime_support_answers_compatible_policy_requirements() {
    let key = PolicyKey::new("core.integer.width").expect("valid policy key");
    let support = RuntimeSemanticSupport::new(
        FeatureSupport::default(),
        [policy_version(
            key.as_str(),
            3,
            vec![ZERO_DIGEST, ONE_DIGEST, TWO_DIGEST],
        )],
    )
    .expect("valid runtime support");

    assert!(support.supports_policy(&PolicyRequirement::new(key.clone(), 3, 1, ONE_DIGEST,)));
    assert!(!support.supports_policy(&PolicyRequirement::new(key.clone(), 3, 1, TWO_DIGEST,)));
    assert!(!support.supports_policy(&PolicyRequirement::new(key, 2, 1, ONE_DIGEST,)));
}

#[test]
fn runtime_support_rejects_duplicate_policy_authority() {
    let key = PolicyKey::new("core.integer.width").expect("valid policy key");
    let lower_minor = policy_version(key.as_str(), 3, vec![ZERO_DIGEST]);
    let higher_minor = policy_version(key.as_str(), 3, vec![ZERO_DIGEST, ONE_DIGEST]);

    assert!(matches!(
        RuntimeSemanticSupport::new(FeatureSupport::default(), [higher_minor, lower_minor]),
        Err(RuntimeSemanticSupportError::DuplicatePolicyMajor {
            policy_key,
            major: 3,
            first_index: 0,
            second_index: 1,
        }) if policy_key == key
    ));
}

#[test]
fn runtime_support_canonically_orders_policy_versions_and_exposes_immutable_authority() {
    let (features, stable_id) = feature_support();
    let support = RuntimeSemanticSupport::new(
        features.clone(),
        [
            policy_version("core.unicode.version", 1, vec![ZERO_DIGEST]),
            policy_version("core.integer.width", 2, vec![ZERO_DIGEST]),
            policy_version("core.integer.width", 1, vec![ZERO_DIGEST, ONE_DIGEST]),
        ],
    )
    .expect("valid runtime support");

    let retained_features: &FeatureSupport = support.feature_support();
    let retained_policies: &[PolicyVersion] = support.policy_versions();

    assert_eq!(retained_features, &features);
    assert!(retained_features.supports(&FeatureRequirement::new(
        FeatureNamespace::Helper,
        stable_id,
        2,
        0,
    )));
    assert_eq!(
        retained_policies
            .iter()
            .map(|version| (version.policy_key().as_str(), version.major()))
            .collect::<Vec<_>>(),
        vec![
            ("core.integer.width", 1),
            ("core.integer.width", 2),
            ("core.unicode.version", 1),
        ]
    );
}
