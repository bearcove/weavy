use core::mem::size_of;

use weavy::{
    BlockKey, CanonicalNameError, FunctionKey, GroupKey, InstId, MemberKey, TypeDigest,
    TypeKeyScheme,
};

#[test]
fn canonical_key_widths_and_scheme_tag_are_frozen() {
    assert_eq!(TypeKeyScheme::NominalNameHashV1.tag(), 0);
    assert_eq!(
        TypeKeyScheme::NominalNameHashV1.as_str(),
        "nominal_name_hash_v1"
    );
    assert_eq!(size_of::<FunctionKey>(), 16);
    assert_eq!(size_of::<GroupKey>(), 16);
    assert_eq!(size_of::<MemberKey>(), 4);
    assert_eq!(size_of::<BlockKey>(), 4);
    assert_eq!(size_of::<TypeDigest>(), 32);
}

#[test]
fn global_and_local_keys_are_domain_and_owner_separated() {
    let name = "core.parser.entry";
    let function = FunctionKey::new(name).expect("function key");
    let group = GroupKey::new(name).expect("group key");
    assert_ne!(function.as_bytes(), group.as_bytes());
    let other_function = FunctionKey::new("core.parser.other").expect("other function");
    let other_group = GroupKey::new("core.parser.other").expect("other group");
    assert_ne!(
        BlockKey::new(function, "block.loop").expect("block key"),
        BlockKey::new(other_function, "block.loop").expect("other block key")
    );
    assert_ne!(
        MemberKey::new(group, "member.node").expect("member key"),
        MemberKey::new(other_group, "member.node").expect("other member key")
    );
}

#[test]
fn canonical_keys_match_independently_derived_vectors() {
    let function = FunctionKey::new("core.parser.entry").expect("function key");
    assert_eq!(
        function.as_bytes(),
        &[
            196, 73, 195, 29, 35, 105, 213, 207, 91, 251, 237, 149, 119, 62, 72, 84,
        ]
    );

    let group = GroupKey::new("core.syntax.node").expect("group key");
    assert_eq!(
        group.as_bytes(),
        &[
            104, 163, 134, 168, 63, 76, 153, 192, 115, 189, 159, 250, 108, 237, 160, 100,
        ]
    );
    assert_eq!(
        MemberKey::new(group, "member.expression")
            .expect("member key")
            .as_u32(),
        673_377_427
    );
    assert_eq!(
        BlockKey::new(function, "block.loop")
            .expect("block key")
            .as_u32(),
        3_322_455_788
    );
}

#[test]
fn canonical_key_order_is_raw_global_bytes_then_numeric_local_values() {
    let low = FunctionKey::from_bytes([0; 16]);
    let high = FunctionKey::from_bytes([0xff; 16]);
    assert!(low < high);

    assert!(MemberKey::from_u32(1) < MemberKey::from_u32(256));
    assert!(BlockKey::from_u32(1) < BlockKey::from_u32(256));

    let first = InstId::new(low, 9);
    let later_function = InstId::new(high, 0);
    let later_ordinal = InstId::new(low, 10);
    assert!(first < later_function);
    assert!(first < later_ordinal);
    assert_eq!(first.function(), low);
    assert_eq!(first.ordinal(), 9);
}

#[test]
fn every_canonical_key_name_uses_the_frozen_name_grammar() {
    let expected = CanonicalNameError::InvalidByte {
        index: 4,
        byte: b'-',
    };
    assert_eq!(FunctionKey::new("core-entry"), Err(expected.clone()));
    assert_eq!(GroupKey::new("core-entry"), Err(expected.clone()));
    assert_eq!(
        MemberKey::new(GroupKey::from_bytes([0; 16]), "core-entry"),
        Err(expected.clone())
    );
    assert_eq!(
        BlockKey::new(FunctionKey::from_bytes([0; 16]), "core-entry"),
        Err(expected)
    );
}

#[test]
fn key_and_digest_raw_forms_round_trip_without_host_layout() {
    let function_bytes = [1; 16];
    let group_bytes = [2; 16];
    let digest_bytes = [3; 32];
    assert_eq!(
        FunctionKey::from_bytes(function_bytes).into_bytes(),
        function_bytes
    );
    assert_eq!(GroupKey::from_bytes(group_bytes).into_bytes(), group_bytes);
    let member = MemberKey::from_u32(0x1234_5678);
    assert_eq!(member.as_u32(), 0x1234_5678);
    assert_eq!(member.into_le_bytes(), [0x78, 0x56, 0x34, 0x12]);
    assert_eq!(MemberKey::from_le_bytes(member.into_le_bytes()), member);
    let block = BlockKey::from_u32(0x9abc_def0);
    assert_eq!(block.as_u32(), 0x9abc_def0);
    assert_eq!(block.into_le_bytes(), [0xf0, 0xde, 0xbc, 0x9a]);
    assert_eq!(BlockKey::from_le_bytes(block.into_le_bytes()), block);
    assert_eq!(
        TypeDigest::from_bytes(digest_bytes).into_bytes(),
        digest_bytes
    );
}
