use weavy::{
    FeatureDescriptor, FeatureId, FeatureNamespace, FeatureRequirement, FeatureSupport,
    FeatureSupportError,
};

fn feature_id(byte: u8) -> FeatureId {
    FeatureId::from_bytes([byte; 16])
}

#[test]
fn feature_support_sorts_descriptors_by_the_canonical_profile_key() {
    let support = FeatureSupport::new([
        FeatureDescriptor::new(
            FeatureNamespace::Helper,
            "core.helper.decode",
            1,
            2,
            [1; 32],
        )
        .expect("valid descriptor"),
        FeatureDescriptor::new(
            FeatureNamespace::Opcode,
            "core.opcode.second",
            1,
            0,
            [2; 32],
        )
        .expect("valid descriptor"),
        FeatureDescriptor::new(
            FeatureNamespace::Relation,
            "core.relation.sorted",
            1,
            1,
            [3; 32],
        )
        .expect("valid descriptor"),
        FeatureDescriptor::new(FeatureNamespace::Opcode, "core.opcode.first", 2, 3, [4; 32])
            .expect("valid descriptor"),
        FeatureDescriptor::new(FeatureNamespace::Opcode, "core.opcode.first", 1, 7, [5; 32])
            .expect("valid descriptor"),
    ])
    .expect("valid feature support");

    let actual = support
        .descriptors()
        .iter()
        .map(|descriptor| {
            (
                descriptor.namespace(),
                descriptor.stable_id(),
                descriptor.major(),
                descriptor.max_minor(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        actual,
        vec![
            (
                FeatureNamespace::Opcode,
                FeatureId::new(FeatureNamespace::Opcode, "core.opcode.first")
                    .expect("valid identity"),
                1,
                7,
            ),
            (
                FeatureNamespace::Opcode,
                FeatureId::new(FeatureNamespace::Opcode, "core.opcode.first")
                    .expect("valid identity"),
                2,
                3,
            ),
            (
                FeatureNamespace::Opcode,
                FeatureId::new(FeatureNamespace::Opcode, "core.opcode.second")
                    .expect("valid identity"),
                1,
                0,
            ),
            (
                FeatureNamespace::Helper,
                FeatureId::new(FeatureNamespace::Helper, "core.helper.decode")
                    .expect("valid identity"),
                1,
                2,
            ),
            (
                FeatureNamespace::Relation,
                FeatureId::new(FeatureNamespace::Relation, "core.relation.sorted")
                    .expect("valid identity"),
                1,
                1,
            ),
        ]
    );
}

#[test]
fn feature_support_requires_an_exact_major_and_at_least_the_minimum_minor() {
    let descriptor = FeatureDescriptor::new(
        FeatureNamespace::Helper,
        "core.helper.decode",
        2,
        4,
        [7; 32],
    )
    .expect("valid descriptor");
    let stable_id = descriptor.stable_id();
    let support = FeatureSupport::new([descriptor]).expect("valid feature support");

    assert!(support.supports(&FeatureRequirement::new(
        FeatureNamespace::Helper,
        stable_id,
        2,
        0,
    )));
    assert!(support.supports(&FeatureRequirement::new(
        FeatureNamespace::Helper,
        stable_id,
        2,
        4,
    )));
    assert!(!support.supports(&FeatureRequirement::new(
        FeatureNamespace::Helper,
        stable_id,
        2,
        5,
    )));
    assert!(!support.supports(&FeatureRequirement::new(
        FeatureNamespace::Helper,
        stable_id,
        1,
        0,
    )));
    assert!(!support.supports(&FeatureRequirement::new(
        FeatureNamespace::Opcode,
        stable_id,
        2,
        0,
    )));
}

#[test]
fn feature_support_rejects_an_absent_feature() {
    let descriptor = FeatureDescriptor::new(
        FeatureNamespace::Opcode,
        "core.opcode.present",
        1,
        3,
        [8; 32],
    )
    .expect("valid descriptor");
    let support = FeatureSupport::new([descriptor]).expect("valid feature support");

    assert!(!support.supports(&FeatureRequirement::new(
        FeatureNamespace::Opcode,
        feature_id(99),
        1,
        0,
    )));
}

#[test]
fn feature_support_rejects_duplicate_identity_major_records() {
    let first = FeatureDescriptor::new(
        FeatureNamespace::Relation,
        "core.relation.sorted",
        3,
        1,
        [1; 32],
    )
    .expect("valid descriptor");
    let stable_id = first.stable_id();
    let second = FeatureDescriptor::new(
        FeatureNamespace::Relation,
        "core.relation.sorted",
        3,
        4,
        [2; 32],
    )
    .expect("valid descriptor");

    assert!(matches!(
        FeatureSupport::new([first, second]),
        Err(FeatureSupportError::DuplicateIdentityMajor {
            namespace: FeatureNamespace::Relation,
            stable_id: actual,
            major: 3,
            ..
        }) if actual == stable_id
    ));
}

#[test]
fn feature_descriptors_and_requirements_preserve_semantic_identity_authority() {
    let namespace = FeatureNamespace::Capability;
    let canonical_name = "core.capability.host_clock";
    let digest = [0xa5; 32];
    let derived_id = FeatureId::new(namespace, canonical_name).expect("valid feature identity");
    let descriptor =
        FeatureDescriptor::new(namespace, canonical_name, 5, 8, digest).expect("valid descriptor");
    let requirement = FeatureRequirement::new(namespace, derived_id, 5, 6);
    let support = FeatureSupport::new([descriptor]).expect("valid feature support");
    let retained = &support.descriptors()[0];

    assert_eq!(retained.namespace(), namespace);
    assert_eq!(retained.canonical_name(), canonical_name);
    assert_eq!(retained.stable_id(), derived_id);
    assert_eq!(retained.major(), 5);
    assert_eq!(retained.max_minor(), 8);
    assert_eq!(retained.semantic_descriptor_digest(), &digest);
    assert_eq!(requirement.namespace(), namespace);
    assert_eq!(requirement.stable_id(), derived_id);
    assert_eq!(requirement.major(), 5);
    assert_eq!(requirement.min_minor(), 6);
}
