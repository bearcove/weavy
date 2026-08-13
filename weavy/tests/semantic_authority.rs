use weavy::{
    CanonicalNameError, FeatureId, FeatureNamespace, PolicyDescriptor, PolicyHistoryError,
    PolicyKey, PolicyRequirement, PolicyVersion,
};

const ZERO_DIGEST: [u8; 32] = [0; 32];
const ONE_DIGEST: [u8; 32] = [1; 32];
const TWO_DIGEST: [u8; 32] = [2; 32];

#[test]
fn feature_ids_match_frozen_vectors() {
    let cases = [
        (
            FeatureNamespace::Opcode,
            "core.builder.commit",
            [
                0x13, 0xb2, 0xd2, 0x57, 0x1e, 0x26, 0x29, 0xf9, 0xb6, 0xe1, 0xfe, 0x0b, 0x76, 0x04,
                0xc8, 0x5a,
            ],
        ),
        (
            FeatureNamespace::Relation,
            "core.relation.sorted.unique",
            [
                0xb1, 0x36, 0x9a, 0x64, 0x59, 0x52, 0x12, 0xc8, 0x40, 0x2e, 0x35, 0x9b, 0x8c, 0x11,
                0x58, 0xb3,
            ],
        ),
    ];

    for (namespace, name, expected) in cases {
        let id = FeatureId::new(namespace, name).expect("valid canonical name");
        assert_eq!(id.as_bytes(), &expected);
        assert_eq!(FeatureId::from_bytes(expected), id);
    }
}

#[test]
fn feature_ids_are_namespace_separated_and_tags_are_frozen() {
    assert_eq!(FeatureNamespace::Opcode.tag(), 0);
    assert_eq!(FeatureNamespace::Helper.tag(), 1);
    assert_eq!(FeatureNamespace::Relation.tag(), 2);
    assert_eq!(FeatureNamespace::Capability.tag(), 3);
    assert_eq!(FeatureNamespace::Opcode.as_str(), "opcode");
    assert_eq!(FeatureNamespace::Helper.as_str(), "helper");
    assert_eq!(FeatureNamespace::Relation.as_str(), "relation");
    assert_eq!(FeatureNamespace::Capability.as_str(), "capability");

    let name = "core.builder.commit";
    let opcode = FeatureId::new(FeatureNamespace::Opcode, name).expect("valid canonical name");
    for namespace in [
        FeatureNamespace::Helper,
        FeatureNamespace::Relation,
        FeatureNamespace::Capability,
    ] {
        assert_ne!(
            FeatureId::new(namespace, name).expect("valid canonical name"),
            opcode
        );
    }
}

#[test]
fn canonical_names_accept_normative_underscored_segments() {
    FeatureId::new(FeatureNamespace::Opcode, "core.builder.init_field")
        .expect("normative opcode name");
    PolicyKey::new("core.integer.fixed_width").expect("normative policy key");
}

#[test]
fn canonical_names_reject_non_lowercase_ascii_dotted_forms() {
    let invalid = [
        ("", CanonicalNameError::Empty),
        (
            "Core.builder",
            CanonicalNameError::InvalidByte {
                index: 0,
                byte: b'C',
            },
        ),
        (
            "core-builder",
            CanonicalNameError::InvalidByte {
                index: 4,
                byte: b'-',
            },
        ),
        (
            "core..builder",
            CanonicalNameError::EmptySegment { dot_index: 5 },
        ),
        (".core", CanonicalNameError::EmptySegment { dot_index: 0 }),
        ("core.", CanonicalNameError::EmptySegment { dot_index: 4 }),
        (
            "core.café",
            CanonicalNameError::InvalidByte {
                index: 8,
                byte: 0xc3,
            },
        ),
    ];

    for (name, expected) in invalid {
        assert_eq!(
            FeatureId::new(FeatureNamespace::Opcode, name),
            Err(expected.clone())
        );
        assert_eq!(PolicyKey::new(name), Err(expected));
    }
}

#[test]
fn policy_minor_compatibility_uses_the_required_minor_digest() {
    let key = PolicyKey::new("core.integer.width").expect("valid policy key");
    let version = PolicyVersion::new(key.clone(), 1, 2, vec![ZERO_DIGEST, ONE_DIGEST, TWO_DIGEST])
        .expect("valid history");

    assert!(version.is_compatible_with(&PolicyRequirement::new(key.clone(), 1, 1, ONE_DIGEST,)));
    assert!(!version.is_compatible_with(&PolicyRequirement::new(key.clone(), 1, 1, TWO_DIGEST,)));
    assert!(!version.is_compatible_with(&PolicyRequirement::new(key.clone(), 1, 3, TWO_DIGEST,)));
    assert!(!version.is_compatible_with(&PolicyRequirement::new(key, 2, 1, ONE_DIGEST,)));
}

#[test]
fn policy_descriptor_history_rejects_key_major_and_minor_mismatches() {
    let key = PolicyKey::new("core.integer.width").expect("valid policy key");
    let other_key = PolicyKey::new("core.unicode.version").expect("valid policy key");
    let descriptor = |key: PolicyKey, major, minor, digest| {
        PolicyDescriptor::new_with_approved_digest(key, major, minor, vec![minor as u8], digest)
    };

    let accepted = PolicyVersion::from_descriptors([
        descriptor(key.clone(), 1, 0, ZERO_DIGEST),
        descriptor(key.clone(), 1, 1, ONE_DIGEST),
    ])
    .expect("ordered descriptors");
    assert_eq!(accepted.policy_key(), &key);
    assert_eq!(accepted.major(), 1);
    assert_eq!(accepted.max_minor(), 1);
    assert_eq!(
        accepted.compatible_minor_digests(),
        &[ZERO_DIGEST, ONE_DIGEST]
    );

    assert!(matches!(
        PolicyVersion::from_descriptors([
            descriptor(key.clone(), 1, 0, ZERO_DIGEST),
            descriptor(other_key, 1, 1, ONE_DIGEST),
        ]),
        Err(PolicyHistoryError::DescriptorKeyMismatch { index: 1, .. })
    ));
    assert!(matches!(
        PolicyVersion::from_descriptors([
            descriptor(key.clone(), 1, 0, ZERO_DIGEST),
            descriptor(key.clone(), 2, 1, ONE_DIGEST),
        ]),
        Err(PolicyHistoryError::DescriptorMajorMismatch {
            index: 1,
            expected: 1,
            actual: 2,
        })
    ));
    assert!(matches!(
        PolicyVersion::from_descriptors([
            descriptor(key.clone(), 1, 0, ZERO_DIGEST),
            descriptor(key, 1, 2, TWO_DIGEST),
        ]),
        Err(PolicyHistoryError::DescriptorMinorMismatch {
            index: 1,
            actual: 2,
        })
    ));
}

#[test]
fn policy_histories_are_complete_and_append_only() {
    let key = PolicyKey::new("core.integer.width").expect("valid policy key");
    let v1 = PolicyVersion::new(key.clone(), 1, 1, vec![ZERO_DIGEST, ONE_DIGEST])
        .expect("valid history");
    let v2 = PolicyVersion::new(key.clone(), 1, 2, vec![ZERO_DIGEST, ONE_DIGEST, TWO_DIGEST])
        .expect("valid extension");
    assert!(v2.extends(&v1).is_ok());

    assert!(matches!(
        PolicyVersion::new(key.clone(), 1, 2, vec![ZERO_DIGEST, ONE_DIGEST]),
        Err(PolicyHistoryError::HistoryLengthMismatch {
            max_minor: 2,
            actual: 2,
        })
    ));

    let rewritten =
        PolicyVersion::new(key.clone(), 1, 2, vec![ZERO_DIGEST, TWO_DIGEST, TWO_DIGEST])
            .expect("internally complete history");
    assert_eq!(
        rewritten.extends(&v1),
        Err(PolicyHistoryError::EarlierDigestChanged { minor: 1 })
    );

    let other = PolicyVersion::new(
        PolicyKey::new("core.unicode.version").expect("valid policy key"),
        1,
        2,
        vec![ZERO_DIGEST, ONE_DIGEST, TWO_DIGEST],
    )
    .expect("valid other history");
    assert!(matches!(
        other.extends(&v1),
        Err(PolicyHistoryError::HistoryKeyMismatch { .. })
    ));

    let shorter = PolicyVersion::new(key, 1, 0, vec![ZERO_DIGEST]).expect("valid shorter history");
    assert_eq!(
        shorter.extends(&v1),
        Err(PolicyHistoryError::HistoryTruncated {
            previous_max_minor: 1,
            max_minor: 0,
        })
    );
}
