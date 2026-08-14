#[path = "../fixture-support/mod.rs"]
mod support;

use facet_value::{VArray, VObject, VString, Value};
use phon_storage::{AlignedRegistry, AlignedWriter, DenseRangeWriter, compact};
use weavy::DenseLowered;
use weavy::ir::{ControlOp, WeavyOp};
use weavy::module::{
    Constant, ConstantId, ConstantPool, ConstantRange, ConstantRangeId, DialectRequirement,
    ModuleManifest, ModuleVerifier, StorageProfile, WeavyModule,
};
use weavy_phon::{
    CodecError, ContainerLimits, ImageId, PayloadIntegrityTag, inspect, inspect_structure, load,
    load_borrowed, save,
};

use support::{TestCodec, TestIntrinsic, aligned_rows, fixture, logical_rows, range_schema};

fn module_with_range(
    profile: StorageProfile,
    bytes: Vec<u8>,
    stride: u32,
) -> WeavyModule<TestIntrinsic> {
    WeavyModule::new(
        ModuleManifest::new(
            "profile.oracle",
            [DialectRequirement::new("test", 1, 0)],
            [0],
        ),
        DenseLowered::new(vec![WeavyOp::Control(ControlOp::Return)], Vec::new()),
        ConstantPool::new(Vec::new()),
    )
    .with_constant_ranges(vec![
        ConstantRange::new(range_schema().0, 1, profile, 3, stride, bytes).expect("range"),
    ])
}

#[test]
fn table_profiles_preserve_canonical_logical_rows() {
    let logical = logical_rows();
    let (schemas, root) = range_schema();
    let compact_registry = phon_storage::compact::Registry::new(schemas.clone());
    let aligned_registry = AlignedRegistry::new(schemas);

    let compact_bytes = compact::to_bytes(&logical, root, &compact_registry).expect("compact rows");
    let aligned_bytes =
        AlignedWriter::encode(&logical, root, &aligned_registry).expect("aligned rows");
    let dense_bytes =
        DenseRangeWriter::encode(&logical, root, &aligned_registry).expect("dense rows");

    let compact_image = save::<TestCodec>(&module_with_range(
        StorageProfile::Compact,
        compact_bytes,
        4,
    ))
    .expect("compact image");
    let aligned_image = save::<TestCodec>(&module_with_range(
        StorageProfile::Aligned,
        aligned_bytes,
        32,
    ))
    .expect("aligned image");
    let dense_image = save::<TestCodec>(&module_with_range(
        StorageProfile::DenseAligned,
        dense_bytes,
        4,
    ))
    .expect("dense image");
    let compact_id = inspect_structure(&compact_image, ContainerLimits::DEFAULT)
        .expect("compact structure")
        .image_id;
    let aligned_id = inspect_structure(&aligned_image, ContainerLimits::DEFAULT)
        .expect("aligned structure")
        .image_id;
    let dense_id = inspect_structure(&dense_image, ContainerLimits::DEFAULT)
        .expect("dense structure")
        .image_id;
    assert_ne!(compact_id, aligned_id);
    assert_ne!(compact_id, dense_id);
    assert_ne!(aligned_id, dense_id);

    let compact = load_borrowed::<TestCodec>(&compact_image, ContainerLimits::DEFAULT)
        .expect("compact load")
        .compact_value(0)
        .expect("compact value");
    let aligned = load_borrowed::<TestCodec>(&aligned_image, ContainerLimits::DEFAULT)
        .expect("aligned load")
        .aligned_document(0)
        .expect("aligned document")
        .to_value()
        .expect("aligned value");
    let dense_module =
        load_borrowed::<TestCodec>(&dense_image, ContainerLimits::DEFAULT).expect("dense load");
    let dense = dense_module.dense_range(0).expect("dense range");
    let mut dense_rows = VArray::new();
    for index in 0..dense.count() {
        let mut row = VObject::new();
        row.insert(
            VString::new("value"),
            Value::from(
                dense
                    .typed_row(index)
                    .expect("dense row")
                    .u32("value")
                    .expect("value"),
            ),
        );
        dense_rows.push(row);
    }
    let dense: Value = dense_rows.into();

    assert_eq!(compact, logical);
    assert_eq!(aligned, logical);
    assert_eq!(dense, logical);
    let canonical =
        compact::to_bytes(&logical, root, &compact_registry).expect("canonical logical bytes");
    for decoded in [&compact, &aligned, &dense] {
        assert_eq!(
            compact::to_bytes(decoded, root, &compact_registry).expect("canonical decoded rows"),
            canonical,
        );
    }
}

#[test]
fn writer_matches_known_good_format_1_image() {
    let generated = save::<TestCodec>(&fixture()).expect("save known-good fixture");
    let checked_in = include_bytes!("fixtures/format-1.0-known-good.weavy");
    let report = inspect_structure(checked_in, ContainerLimits::DEFAULT)
        .expect("known-good structural inspection");
    assert_eq!(
        generated, checked_in,
        "regenerate with the dedicated fixture generator after an approved format change"
    );
    assert_eq!(&checked_in[..8], b"WEAVY\0\0\0");
    assert_eq!(
        u16::from_le_bytes(checked_in[8..10].try_into().expect("major")),
        1
    );
    assert_eq!(
        u16::from_le_bytes(checked_in[10..12].try_into().expect("minor")),
        0
    );
    assert_eq!(checked_in[12], 1);
    assert_eq!(&checked_in[13..16], &[0; 3]);
    assert_eq!(
        u64::from_le_bytes(checked_in[16..24].try_into().expect("length")) as usize,
        checked_in.len()
    );
    assert_eq!(
        u64::from_le_bytes(checked_in[24..32].try_into().expect("directory offset")),
        64
    );
    assert_eq!(
        u32::from_le_bytes(checked_in[40..44].try_into().expect("directory alignment")),
        8
    );
    assert_eq!(
        u32::from_le_bytes(checked_in[44..48].try_into().expect("directory kind")),
        1
    );
    assert_eq!(
        report
            .sections
            .iter()
            .map(|section| section.kind)
            .collect::<Vec<_>>(),
        [2, 3, 4, 5, 0x1000],
    );
    assert_eq!(
        report.payload_integrity_tag,
        PayloadIntegrityTag::from_bytes([
            56, 180, 132, 25, 27, 182, 111, 117, 36, 142, 182, 144, 170, 158, 121, 159,
        ]),
    );
    assert_eq!(
        report.image_id,
        ImageId::from_bytes([
            211, 49, 174, 171, 242, 56, 93, 119, 172, 41, 24, 204, 95, 182, 181, 108, 58, 152, 211,
            69, 157, 114, 148, 127, 180, 89, 166, 195, 166, 196, 149, 117,
        ]),
    );
}
#[test]
fn weavy_bytes_round_trip_deterministically() {
    let module = fixture();
    let first = save::<TestCodec>(&module).expect("save");
    let loaded = load::<TestCodec>(&first, ContainerLimits::default()).expect("load");
    assert_eq!(loaded, module);
    assert_eq!(loaded.constant_ranges(), module.constant_ranges());
    assert_eq!(save::<TestCodec>(&loaded).expect("save again"), first);
}

#[test]
fn large_constant_round_trips_deterministically() {
    let payload = (0..32 * 1024 * 1024)
        .map(|index| (index % 251) as u8)
        .collect::<Vec<_>>();
    let module = WeavyModule::new(
        ModuleManifest::new(
            "large.constant",
            [DialectRequirement::new("test", 1, 0)],
            [0],
        ),
        DenseLowered::new(
            vec![WeavyOp::Intrinsic(TestIntrinsic::new(
                ConstantId::new(0),
                ConstantRangeId::new(0),
            ))],
            Vec::new(),
        ),
        ConstantPool::new(vec![Constant::new(0x42, payload)]),
    )
    .with_constant_ranges(vec![
        ConstantRange::new(
            range_schema().0,
            1,
            StorageProfile::Aligned,
            3,
            32,
            aligned_rows(),
        )
        .expect("range"),
    ]);

    let first = save::<TestCodec>(&module).expect("save");
    let borrowed =
        load_borrowed::<TestCodec>(&first, ContainerLimits::default()).expect("borrowed load");
    assert_eq!(
        borrowed.constants()[0].bytes(),
        module.constants()[0].bytes()
    );
    let owned = load::<TestCodec>(&first, ContainerLimits::default()).expect("owned load");
    assert_eq!(save::<TestCodec>(&owned).expect("save again"), first);
}

#[test]
fn borrowed_load_keeps_aligned_range_in_module_bytes() {
    let first = save::<TestCodec>(&fixture()).expect("save");
    let report = inspect(&first, ContainerLimits::default()).expect("inspect");
    let section = report
        .sections
        .iter()
        .find(|section| section.name == "constant_range.0")
        .expect("range section");
    let borrowed =
        load_borrowed::<TestCodec>(&first, ContainerLimits::default()).expect("borrowed load");
    borrowed
        .admit(&ModuleVerifier::new([DialectRequirement::new(
            "test", 1, 0,
        )]))
        .expect("borrowed admission");
    let range = &borrowed.constant_ranges()[0];
    assert_eq!(
        range.bytes().as_ptr(),
        first[section.offset as usize..].as_ptr(),
    );
    let document = borrowed.aligned_document(0).expect("aligned document");
    assert_eq!(document.root().len().expect("range length"), 3);
    assert_eq!(
        document
            .root()
            .index(1)
            .expect("second row")
            .field("value")
            .expect("value field")
            .as_u32()
            .expect("u32"),
        2,
    );
}

#[test]
fn inspect_reports_discoverable_module_facts() {
    let bytes = save::<TestCodec>(&fixture()).expect("save");
    let report = inspect(&bytes, ContainerLimits::default()).expect("inspect");
    assert_eq!(report.module_name, "codec.fixture");
    assert_eq!(report.program_op_count, 3);
    assert_eq!(report.block_count, 1);
    assert_eq!(report.constant_count, 1);
    assert_eq!(report.constant_ranges.len(), 1);
    assert_eq!(report.constant_ranges[0].count, 3);
    assert_eq!(report.constant_ranges[0].profile, StorageProfile::Aligned);
    assert!(
        report
            .sections
            .iter()
            .any(|section| section.name == "program")
    );
    assert!(
        report
            .sections
            .iter()
            .any(|section| section.name == "constant_range.0")
    );
}

#[test]
fn saved_image_reports_distinct_physical_identities() {
    let bytes = save::<TestCodec>(&fixture()).expect("save");
    let expected_payload_tag = blake3::hash(&bytes[64..]);
    let expected_image_id = blake3::hash(&bytes);

    let report = inspect(&bytes, ContainerLimits::default()).expect("inspect");
    assert_eq!(
        report.payload_integrity_tag,
        PayloadIntegrityTag::from_bytes(
            expected_payload_tag.as_bytes()[..16]
                .try_into()
                .expect("tag length"),
        )
    );
    assert_eq!(
        report.image_id,
        ImageId::from_bytes(*expected_image_id.as_bytes())
    );
    assert_eq!(report.payload_integrity_tag.as_bytes(), &bytes[48..64]);

    let mut with_trailing_byte = bytes.clone();
    with_trailing_byte.push(0);
    assert!(matches!(
        inspect_structure(&with_trailing_byte, ContainerLimits::default()),
        Err(CodecError::Truncated { needed, actual })
            if needed == bytes.len() && actual == with_trailing_byte.len()
    ));
}

#[test]
fn image_id_covers_header_while_payload_tag_does_not() {
    let bytes = save::<TestCodec>(&fixture()).expect("save");
    let original =
        inspect_structure(&bytes, ContainerLimits::default()).expect("structural inspection");

    let mut changed_header = bytes.clone();
    changed_header[13] = 1;

    let changed_image_id = ImageId::from_bytes(*blake3::hash(&changed_header).as_bytes());
    let changed_payload_tag = PayloadIntegrityTag::from_bytes(
        blake3::hash(&changed_header[64..]).as_bytes()[..16]
            .try_into()
            .expect("tag length"),
    );
    assert_eq!(changed_payload_tag, original.payload_integrity_tag);
    assert_ne!(changed_image_id, original.image_id);
    assert!(matches!(
        inspect_structure(&changed_header, ContainerLimits::default()),
        Err(CodecError::MalformedHeader)
    ));
}

#[test]
fn payload_changes_invalidate_the_integrity_tag() {
    let mut bytes = save::<TestCodec>(&fixture()).expect("save");
    let original_tag = inspect_structure(&bytes, ContainerLimits::default())
        .expect("structural inspection")
        .payload_integrity_tag;
    let last = bytes.len() - 1;
    bytes[last] ^= 0x80;

    let actual = PayloadIntegrityTag::from_bytes(
        blake3::hash(&bytes[64..]).as_bytes()[..16]
            .try_into()
            .expect("tag length"),
    );
    assert_ne!(actual, original_tag);
    assert!(matches!(
        inspect_structure(&bytes, ContainerLimits::default()),
        Err(CodecError::IntegrityMismatch { .. })
    ));
}

#[test]
fn structural_inspection_does_not_decode_semantic_payloads() {
    let mut bytes = save::<TestCodec>(&fixture()).expect("save");
    let report = inspect(&bytes, ContainerLimits::default()).expect("semantic inspection");
    let manifest = report
        .sections
        .iter()
        .find(|section| section.name == "manifest")
        .expect("manifest section");
    bytes[manifest.offset as usize] ^= 0x80;
    rehash(&mut bytes);

    let structural =
        inspect_structure(&bytes, ContainerLimits::default()).expect("structural inspection");
    assert_eq!(structural.sections, report.sections);
    assert_eq!(
        structural.image_id,
        ImageId::from_bytes(*blake3::hash(&bytes).as_bytes())
    );
    assert!(
        inspect(&bytes, ContainerLimits::default()).is_err(),
        "semantic inspection accepted payload"
    );
    assert!(
        load::<TestCodec>(&bytes, ContainerLimits::default()).is_err(),
        "structural inspection conferred admission authority"
    );
}

#[test]
fn malformed_modules_are_rejected() {
    let bytes = save::<TestCodec>(&fixture()).expect("save");
    let mut corrupted = bytes.clone();
    let last = corrupted.len() - 1;
    corrupted[last] ^= 0x80;
    assert!(matches!(
        load::<TestCodec>(&corrupted, ContainerLimits::default()),
        Err(CodecError::IntegrityMismatch { .. })
    ));
    assert!(matches!(
        load::<TestCodec>(&bytes[..bytes.len() - 1], ContainerLimits::default()),
        Err(CodecError::Truncated { .. })
    ));
    let mut bad_offset = bytes.clone();
    bad_offset[24..32].copy_from_slice(&u64::MAX.to_le_bytes());
    assert!(matches!(
        load::<TestCodec>(&bad_offset, ContainerLimits::default()),
        Err(CodecError::MalformedHeader) | Err(CodecError::IntegrityMismatch { .. })
    ));
    let mut bad_alignment = bytes.clone();
    bad_alignment[40..44].copy_from_slice(&3u32.to_le_bytes());
    assert!(matches!(
        load::<TestCodec>(&bad_alignment, ContainerLimits::default()),
        Err(CodecError::MalformedHeader) | Err(CodecError::IntegrityMismatch { .. })
    ));
    let mut unknown_required = bytes;
    unknown_required[44..48].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(matches!(
        load::<TestCodec>(&unknown_required, ContainerLimits::default()),
        Err(CodecError::MalformedHeader) | Err(CodecError::IntegrityMismatch { .. })
    ));
}

fn rehash(bytes: &mut [u8]) {
    let hash = blake3::hash(&bytes[64..]);
    bytes[48..64].copy_from_slice(&hash.as_bytes()[..16]);
}

#[test]
fn borrowed_ranges_reject_bad_schema_bounds_alignment_and_count() {
    let bytes = save::<TestCodec>(&fixture()).expect("save");
    let report = inspect(&bytes, ContainerLimits::default()).expect("inspect");
    let section = report
        .sections
        .iter()
        .find(|section| section.name == "constant_range.0")
        .expect("range section");

    let mut corrupted = bytes.clone();
    corrupted[section.offset as usize + 16] ^= 1;
    rehash(&mut corrupted);
    assert!(matches!(
        load_borrowed::<TestCodec>(&corrupted, ContainerLimits::default()),
        Err(CodecError::Aligned(_))
    ));

    let mut truncated = bytes.clone();
    truncated.truncate(truncated.len() - 1);
    assert!(matches!(
        load_borrowed::<TestCodec>(&truncated, ContainerLimits::default()),
        Err(CodecError::Truncated { .. })
    ));

    let mut bad_alignment = bytes.clone();
    let offset = section.offset as usize;
    bad_alignment[offset + 24..offset + 32].copy_from_slice(&65u64.to_le_bytes());
    rehash(&mut bad_alignment);
    assert!(matches!(
        load_borrowed::<TestCodec>(&bad_alignment, ContainerLimits::default()),
        Err(CodecError::Aligned(_))
    ));
}

#[test]
fn borrowed_dense_range_keeps_fixed_rows_in_module_bytes() {
    let (schemas, root) = range_schema();
    let registry = AlignedRegistry::new(schemas.clone());
    let mut rows = VArray::new();
    for value in [1u32, 2] {
        let mut row = VObject::new();
        row.insert(VString::new("value"), Value::from(value));
        rows.push(row);
    }
    let payload =
        phon_storage::DenseRangeWriter::encode(&rows.into(), root, &registry).expect("dense rows");
    let module = WeavyModule::new(
        ModuleManifest::new("dense", [DialectRequirement::new("test", 1, 0)], [0]),
        DenseLowered::new(
            vec![WeavyOp::Intrinsic(TestIntrinsic::new(
                ConstantId::new(0),
                ConstantRangeId::new(0),
            ))],
            Vec::new(),
        ),
        ConstantPool::new(vec![Constant::new(0x42, vec![1])]),
    )
    .with_constant_ranges(vec![
        ConstantRange::new(schemas, 1, StorageProfile::DenseAligned, 2, 4, payload).expect("range"),
    ]);
    let bytes = save::<TestCodec>(&module).expect("save");
    let borrowed =
        load_borrowed::<TestCodec>(&bytes, ContainerLimits::default()).expect("borrowed load");
    let range = borrowed.dense_range(0).expect("dense range");
    assert_eq!(range.row(1).expect("second row"), &[2, 0, 0, 0]);
    assert_eq!(
        range.bytes().as_ptr(),
        borrowed.constant_ranges()[0].bytes().as_ptr()
    );
}

#[test]
fn borrowed_dense_range_rejects_declared_schema_layout_mismatch() {
    let (schemas, root) = range_schema();
    let registry = AlignedRegistry::new(schemas.clone());
    let mut rows = VArray::new();
    let mut row = VObject::new();
    row.insert(VString::new("value"), Value::from(1u32));
    rows.push(row);
    let payload =
        phon_storage::DenseRangeWriter::encode(&rows.into(), root, &registry).expect("dense rows");
    let module = WeavyModule::new(
        ModuleManifest::new("dense", [DialectRequirement::new("test", 1, 0)], [0]),
        DenseLowered::new(
            vec![WeavyOp::Intrinsic(TestIntrinsic::new(
                ConstantId::new(0),
                ConstantRangeId::new(0),
            ))],
            Vec::new(),
        ),
        ConstantPool::new(vec![Constant::new(0x42, vec![1])]),
    )
    .with_constant_ranges(vec![
        ConstantRange::new(schemas, 1, StorageProfile::DenseAligned, 1, 8, payload)
            .expect("semantic range"),
    ]);
    assert!(matches!(
        save::<TestCodec>(&module),
        Err(CodecError::MalformedConstantRange)
            | Err(CodecError::Aligned(
                phon_storage::AlignedError::WrongDenseLayout { .. }
            ))
    ));
}
