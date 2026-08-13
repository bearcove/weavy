use facet_value::{VArray, VObject, VString, Value};
use phon_schema::{
    Field, Primitive, Schema, SchemaId, SchemaKind, SchemaRef, primitive_id, resolve_ids,
};
use phon_storage::{AlignedRegistry, AlignedWriter};
use weavy::ir::{ControlOp, WeavyOp};
use weavy::module::{
    Constant, ConstantId, ConstantPool, ConstantRange, ConstantRangeId, ConstantRangeReference,
    ConstantReference, DialectRequirement, IntrinsicContract, ModuleManifest, ModuleVerifier,
    StorageProfile, WeavyModule,
};
use weavy::{BlockRef, DenseLowered};
use weavy_phon::{
    CodecError, ContainerLimits, ImageId, IntrinsicCodec, PayloadIntegrityTag, inspect,
    inspect_structure, load, load_borrowed, save,
};

#[derive(Clone, Debug, PartialEq, Eq)]
struct TestIntrinsic {
    constant: ConstantId,
    range: ConstantRangeId,
}

impl IntrinsicContract for TestIntrinsic {
    fn constant_references(&self, visit: &mut dyn FnMut(ConstantReference)) {
        visit(ConstantReference::new(self.constant, 0x42));
    }

    fn constant_range_references(&self, visit: &mut dyn FnMut(ConstantRangeReference)) {
        visit(ConstantRangeReference::new(
            self.range,
            range_schema().1,
            StorageProfile::Aligned,
        ));
    }
}

struct TestCodec;

impl IntrinsicCodec for TestCodec {
    type Intrinsic = TestIntrinsic;
    const DIALECT: &'static str = "test";
    const SCHEMA_ID: u64 = 0x7711;

    fn encode(intrinsic: &Self::Intrinsic, out: &mut Vec<u8>) {
        out.extend_from_slice(&intrinsic.constant.index().to_le_bytes());
        out.extend_from_slice(&intrinsic.range.index().to_le_bytes());
    }

    fn decode(bytes: &[u8]) -> Result<Self::Intrinsic, CodecError> {
        if bytes.len() != 8 {
            return Err(CodecError::MalformedIntrinsic);
        }
        Ok(TestIntrinsic {
            constant: ConstantId::new(u32::from_le_bytes(bytes[..4].try_into().expect("length"))),
            range: ConstantRangeId::new(u32::from_le_bytes(bytes[4..].try_into().expect("length"))),
        })
    }
}

fn range_schema() -> (Vec<Schema>, SchemaId) {
    let row = Schema {
        id: SchemaId::from_raw(1),
        type_params: Vec::new(),
        kind: SchemaKind::Struct {
            name: "TestRow".into(),
            fields: vec![Field {
                name: "value".into(),
                schema: SchemaRef::concrete(primitive_id(Primitive::U32)),
                required: true,
            }],
        },
    };
    let list = Schema {
        id: SchemaId::from_raw(2),
        type_params: Vec::new(),
        kind: SchemaKind::List {
            element: SchemaRef::concrete(row.id),
        },
    };
    let schemas = resolve_ids(vec![row, list]);
    let root = schemas[1].id;
    (schemas, root)
}

fn aligned_rows() -> Vec<u8> {
    let (schemas, root) = range_schema();
    let registry = AlignedRegistry::new(schemas);
    let mut rows = VArray::new();
    for value in [1u32, 2, 3] {
        let mut row = VObject::new();
        row.insert(VString::new("value"), Value::from(value));
        rows.push(row);
    }
    AlignedWriter::encode(&Value::from(rows), root, &registry).expect("aligned rows")
}

fn fixture() -> WeavyModule<TestIntrinsic> {
    WeavyModule::new(
        ModuleManifest::new(
            "codec.fixture",
            [DialectRequirement::new("test", 1, 0)],
            [0],
        ),
        DenseLowered::new(
            vec![
                WeavyOp::Intrinsic(TestIntrinsic {
                    constant: ConstantId::new(0),
                    range: ConstantRangeId::new(0),
                }),
                WeavyOp::Control(ControlOp::CallBlock {
                    block: BlockRef::new(0),
                    base_offset: 12,
                }),
            ],
            vec![vec![WeavyOp::Control(ControlOp::Return)]],
        ),
        ConstantPool::new(vec![Constant::new(0x42, vec![1, 2, 3, 4])]),
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
    ])
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
            vec![WeavyOp::Intrinsic(TestIntrinsic {
                constant: ConstantId::new(0),
                range: ConstantRangeId::new(0),
            })],
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
            vec![WeavyOp::Intrinsic(TestIntrinsic {
                constant: ConstantId::new(0),
                range: ConstantRangeId::new(0),
            })],
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
            vec![WeavyOp::Intrinsic(TestIntrinsic {
                constant: ConstantId::new(0),
                range: ConstantRangeId::new(0),
            })],
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
