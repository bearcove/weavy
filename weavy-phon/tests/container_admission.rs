use facet_value::{VArray, Value};
use phon_schema::{Primitive, Schema, SchemaId, SchemaKind, SchemaRef, primitive_id, resolve_ids};
use phon_storage::compact::Registry;
use weavy::ir::{ControlOp, WeavyOp};
use weavy::module::{
    ConstantPool, ConstantRange, ConstantReference, DialectRequirement, IntrinsicContract,
    ModuleManifest, StorageProfile, WeavyModule,
};
use weavy::{BlockRef, DenseLowered};
use weavy_phon::{
    CodecError, ContainerLimitKind, ContainerLimits, IntrinsicCodec, inspect, inspect_structure,
    load, load_borrowed, save,
};

const SECTION_MANIFEST: u32 = 2;
const SECTION_SCHEMAS: u32 = 3;
const SECTION_PROGRAM: u32 = 4;
const SECTION_CONSTANTS: u32 = 5;
const SECTION_CONSTANT_RANGE_BASE: u32 = 0x1000;

#[derive(Clone, Debug, PartialEq, Eq)]
struct TestIntrinsic;

impl IntrinsicContract for TestIntrinsic {
    fn constant_references(&self, _visit: &mut dyn FnMut(ConstantReference)) {}
}

struct TestCodec;

impl IntrinsicCodec for TestCodec {
    type Intrinsic = TestIntrinsic;
    const DIALECT: &'static str = "container-admission";
    const SCHEMA_ID: u64 = 0x7711;

    fn encode(_intrinsic: &Self::Intrinsic, _out: &mut Vec<u8>) {
        unreachable!("the fixture contains no intrinsic operations")
    }

    fn decode(_bytes: &[u8]) -> Result<Self::Intrinsic, CodecError> {
        Err(CodecError::MalformedIntrinsic)
    }
}

fn compact_range() -> ConstantRange {
    let schemas = resolve_ids(vec![Schema {
        id: SchemaId::from_raw(1),
        type_params: Vec::new(),
        kind: SchemaKind::List {
            element: SchemaRef::concrete(primitive_id(Primitive::U32)),
        },
    }]);
    let root = schemas[0].id;
    let registry = Registry::new(schemas.clone());
    let mut rows = VArray::new();
    rows.push(Value::from(11u32));
    rows.push(Value::from(22u32));
    let bytes = phon_storage::compact::to_bytes(&rows.into(), root, &registry)
        .expect("encode compact range");
    ConstantRange::new(schemas, 0, StorageProfile::Compact, 2, 4, bytes).expect("compact range")
}

fn valid_bytes() -> Vec<u8> {
    let program: DenseLowered<WeavyOp<BlockRef, TestIntrinsic>> =
        DenseLowered::new(vec![WeavyOp::Control(ControlOp::Return)], Vec::new());
    let module = WeavyModule::new(
        ModuleManifest::new(
            "container.admission",
            [DialectRequirement::new("container-admission", 1, 0)],
            [0],
        ),
        program,
        ConstantPool::new(Vec::new()),
    )
    .with_constant_ranges(vec![compact_range(), compact_range()]);
    save::<TestCodec>(&module).expect("save fixture")
}

#[derive(Clone, Copy)]
struct EntryOffsets {
    kind: u32,
    kind_at: usize,
    offset_at: usize,
    encoded_len_at: usize,
    decoded_len_at: usize,
    alignment_at: usize,
    schema_id_at: usize,
    flags_at: usize,
    profile_at: usize,
    count_at: usize,
    stride_at: usize,
}

fn directory_entries(bytes: &[u8]) -> Vec<EntryOffsets> {
    let directory_offset = read_u64(bytes, 24) as usize;
    let directory_len = read_u64(bytes, 32) as usize;
    let directory_end = directory_offset + directory_len;
    let mut cursor = directory_offset;
    let count = read_compact_u32(bytes, directory_offset, &mut cursor) as usize;
    let mut entries = Vec::with_capacity(count);

    for _ in 0..count {
        let name_len = read_compact_u32(bytes, directory_offset, &mut cursor) as usize;
        cursor += name_len;
        align_cursor(directory_offset, &mut cursor, 4);
        let kind_at = cursor;
        let kind = read_u32(bytes, cursor);
        cursor += 4;
        align_cursor(directory_offset, &mut cursor, 8);
        let offset_at = cursor;
        cursor += 8;
        align_cursor(directory_offset, &mut cursor, 8);
        let encoded_len_at = cursor;
        cursor += 8;
        align_cursor(directory_offset, &mut cursor, 8);
        let decoded_len_at = cursor;
        cursor += 8;
        align_cursor(directory_offset, &mut cursor, 4);
        let alignment_at = cursor;
        cursor += 4;
        align_cursor(directory_offset, &mut cursor, 8);
        let schema_id_at = cursor;
        cursor += 8;
        align_cursor(directory_offset, &mut cursor, 4);
        let flags_at = cursor;
        cursor += 4;
        let profile_at = cursor;
        cursor += 1;
        align_cursor(directory_offset, &mut cursor, 4);
        let count_at = cursor;
        cursor += 4;
        align_cursor(directory_offset, &mut cursor, 4);
        let stride_at = cursor;
        cursor += 4;
        entries.push(EntryOffsets {
            kind,
            kind_at,
            offset_at,
            encoded_len_at,
            decoded_len_at,
            alignment_at,
            schema_id_at,
            flags_at,
            profile_at,
            count_at,
            stride_at,
        });
    }

    assert_eq!(cursor, directory_end, "directory framing changed");
    entries
}

fn read_compact_u32(bytes: &[u8], base: usize, cursor: &mut usize) -> u32 {
    align_cursor(base, cursor, 4);
    let value = read_u32(bytes, *cursor);
    *cursor += 4;
    value
}

fn align_cursor(base: usize, cursor: &mut usize, alignment: usize) {
    while !(*cursor - base).is_multiple_of(alignment) {
        *cursor += 1;
    }
}

fn entry(bytes: &[u8], kind: u32) -> EntryOffsets {
    directory_entries(bytes)
        .into_iter()
        .find(|entry| entry.kind == kind)
        .expect("directory entry")
}

fn assert_all_entry_points_reject(bytes: &[u8]) {
    let limits = ContainerLimits::default();
    assert!(
        load::<TestCodec>(bytes, limits).is_err(),
        "owned load accepted image"
    );
    assert!(
        load_borrowed::<TestCodec>(bytes, limits).is_err(),
        "borrowed load accepted image"
    );
    assert!(inspect(bytes, limits).is_err(), "inspection accepted image");
}

fn assert_limit_error(
    result: Result<(), CodecError>,
    kind: ContainerLimitKind,
    configured: usize,
    actual: usize,
) {
    assert!(matches!(
        result,
        Err(CodecError::ContainerLimitExceeded {
            kind: actual_kind,
            configured: actual_configured,
            actual: actual_value,
        }) if actual_kind == kind
            && actual_configured == configured
            && actual_value == actual
    ));
}

fn assert_all_entry_points_reject_with_limits(
    bytes: &[u8],
    limits: ContainerLimits,
    kind: ContainerLimitKind,
    configured: usize,
    actual: usize,
) {
    assert_limit_error(
        load::<TestCodec>(bytes, limits).map(|_| ()),
        kind,
        configured,
        actual,
    );
    assert_limit_error(
        load_borrowed::<TestCodec>(bytes, limits).map(|_| ()),
        kind,
        configured,
        actual,
    );
    assert_limit_error(inspect(bytes, limits).map(|_| ()), kind, configured, actual);
    assert_limit_error(
        inspect_structure(bytes, limits).map(|_| ()),
        kind,
        configured,
        actual,
    );
}

type LimitedEntryPoint = fn(&[u8], ContainerLimits) -> Result<(), CodecError>;

fn owned_entry(bytes: &[u8], limits: ContainerLimits) -> Result<(), CodecError> {
    load::<TestCodec>(bytes, limits).map(|_| ())
}

fn borrowed_entry(bytes: &[u8], limits: ContainerLimits) -> Result<(), CodecError> {
    load_borrowed::<TestCodec>(bytes, limits).map(|_| ())
}

fn semantic_entry(bytes: &[u8], limits: ContainerLimits) -> Result<(), CodecError> {
    inspect(bytes, limits).map(|_| ())
}

fn structural_entry(bytes: &[u8], limits: ContainerLimits) -> Result<(), CodecError> {
    inspect_structure(bytes, limits).map(|_| ())
}

fn limit_actual(
    result: Result<(), CodecError>,
    kind: ContainerLimitKind,
    configured: usize,
) -> usize {
    match result {
        Err(CodecError::ContainerLimitExceeded {
            kind: actual_kind,
            configured: actual_configured,
            actual,
        }) if actual_kind == kind && actual_configured == configured => actual,
        other => panic!("expected {kind:?} limit {configured}, got {other:?}"),
    }
}

fn assert_cumulative_byte_limit(
    bytes: &[u8],
    kind: ContainerLimitKind,
    set_limit: fn(ContainerLimits, usize) -> ContainerLimits,
) {
    for (name, entry) in [
        ("owned", owned_entry as LimitedEntryPoint),
        ("borrowed", borrowed_entry as LimitedEntryPoint),
        ("semantic", semantic_entry as LimitedEntryPoint),
        ("structural", structural_entry as LimitedEntryPoint),
    ] {
        let first_actual = limit_actual(
            entry(bytes, set_limit(ContainerLimits::DEFAULT, 0)),
            kind,
            0,
        );
        let later_actual = limit_actual(
            entry(bytes, set_limit(ContainerLimits::DEFAULT, first_actual)),
            kind,
            first_actual,
        );
        assert!(
            later_actual > first_actual,
            "{name} did not accumulate {kind:?} across allocations"
        );
    }
}

#[test]
fn decoded_byte_limit_is_cumulative_across_entry_points() {
    let bytes = valid_bytes();
    for (name, entry) in [
        ("owned", owned_entry as LimitedEntryPoint),
        ("borrowed", borrowed_entry as LimitedEntryPoint),
        ("semantic", semantic_entry as LimitedEntryPoint),
        ("structural", structural_entry as LimitedEntryPoint),
    ] {
        let mut configured = 0;
        let mut failures = 0;
        loop {
            match entry(
                &bytes,
                ContainerLimits::DEFAULT.with_max_decoded_bytes(configured),
            ) {
                Ok(()) => break,
                Err(CodecError::ContainerLimitExceeded {
                    kind: ContainerLimitKind::DecodedBytes,
                    configured: actual_configured,
                    actual,
                }) if actual_configured == configured && actual > configured => {
                    configured = actual;
                    failures += 1;
                }
                other => panic!("{name} returned unexpected decoded-byte result: {other:?}"),
            }
        }
        assert!(failures > 0, "{name} did not enforce decoded bytes");
    }
}

#[test]
fn retained_byte_limit_is_cumulative_across_entry_points() {
    assert_cumulative_byte_limit(
        &valid_bytes(),
        ContainerLimitKind::RetainedBytes,
        ContainerLimits::with_max_retained_bytes,
    );
}

#[test]
fn tiny_image_limit_rejects_all_entry_points() {
    let bytes = valid_bytes();
    let configured = bytes.len() - 1;
    assert_all_entry_points_reject_with_limits(
        &bytes,
        ContainerLimits::DEFAULT.with_max_image_bytes(configured),
        ContainerLimitKind::ImageBytes,
        configured,
        bytes.len(),
    );
}

#[test]
fn tiny_directory_limit_rejects_all_entry_points() {
    let bytes = valid_bytes();
    let actual = read_u64(&bytes, 32) as usize;
    let configured = actual - 1;
    assert_all_entry_points_reject_with_limits(
        &bytes,
        ContainerLimits::DEFAULT.with_max_directory_bytes(configured),
        ContainerLimitKind::DirectoryBytes,
        configured,
        actual,
    );
}

#[test]
fn tiny_section_limit_rejects_all_entry_points() {
    let bytes = valid_bytes();
    let actual = directory_entries(&bytes).len();
    let configured = actual - 1;
    assert_all_entry_points_reject_with_limits(
        &bytes,
        ContainerLimits::DEFAULT.with_max_sections(configured),
        ContainerLimitKind::Sections,
        configured,
        actual,
    );
}

#[test]
fn tiny_schema_limit_rejects_semantic_entry_points() {
    let bytes = valid_bytes();
    let schemas = entry(&bytes, SECTION_SCHEMAS);
    let schemas_offset = read_u64(&bytes, schemas.offset_at) as usize;
    let actual = read_u32(&bytes, schemas_offset) as usize;
    let configured = actual - 1;
    let limits = ContainerLimits::DEFAULT.with_max_schemas(configured);

    assert_limit_error(
        load::<TestCodec>(&bytes, limits).map(|_| ()),
        ContainerLimitKind::Schemas,
        configured,
        actual,
    );
    assert_limit_error(
        load_borrowed::<TestCodec>(&bytes, limits).map(|_| ()),
        ContainerLimitKind::Schemas,
        configured,
        actual,
    );
    assert_limit_error(
        inspect(&bytes, limits).map(|_| ()),
        ContainerLimitKind::Schemas,
        configured,
        actual,
    );
    inspect_structure(&bytes, limits).expect("structural inspect skips schema decode");
}

#[test]
fn explicit_default_limits_load_current_fixture() {
    let bytes = valid_bytes();
    let limits = ContainerLimits::default();
    load::<TestCodec>(&bytes, limits).expect("owned load with default limits");
    load_borrowed::<TestCodec>(&bytes, limits).expect("borrowed load with default limits");
    inspect(&bytes, limits).expect("semantic inspect with default limits");
    inspect_structure(&bytes, limits).expect("structural inspect with default limits");
}

fn rehash(bytes: &mut [u8]) {
    let hash = blake3::hash(&bytes[64..]);
    bytes[48..64].copy_from_slice(&hash.as_bytes()[..16]);
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().expect("u32"))
}

fn read_u64(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(bytes[offset..offset + 8].try_into().expect("u64"))
}

fn write_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn write_u64(bytes: &mut [u8], offset: usize, value: u64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

#[test]
fn rejects_wrong_byte_order_marker() {
    let mut bytes = valid_bytes();
    bytes[12] = 2;
    assert_all_entry_points_reject(&bytes);
}

#[test]
fn rejects_nonzero_reserved_header_byte() {
    let mut bytes = valid_bytes();
    bytes[14] = 1;
    assert_all_entry_points_reject(&bytes);
}

#[test]
fn rejects_nonzero_format_minor() {
    let mut bytes = valid_bytes();
    bytes[10..12].copy_from_slice(&1u16.to_le_bytes());
    assert_all_entry_points_reject(&bytes);
}

#[test]
fn rejects_noncanonical_directory_header() {
    for mutate in [
        |bytes: &mut Vec<u8>| bytes[24..32].copy_from_slice(&72u64.to_le_bytes()),
        |bytes: &mut Vec<u8>| bytes[40..44].copy_from_slice(&16u32.to_le_bytes()),
        |bytes: &mut Vec<u8>| bytes[44..48].copy_from_slice(&2u32.to_le_bytes()),
    ] {
        let mut bytes = valid_bytes();
        mutate(&mut bytes);
        assert_all_entry_points_reject(&bytes);
    }
}

#[test]
fn rejects_wrong_legacy_schema_id() {
    let mut bytes = valid_bytes();
    let manifest = entry(&bytes, SECTION_MANIFEST);
    write_u64(&mut bytes, manifest.schema_id_at, 2);
    rehash(&mut bytes);
    assert_all_entry_points_reject(&bytes);
}

#[test]
fn rejects_duplicate_legacy_singleton() {
    let mut bytes = valid_bytes();
    let second_range = entry(&bytes, SECTION_CONSTANT_RANGE_BASE + 1);
    write_u32(&mut bytes, second_range.kind_at, SECTION_MANIFEST);
    rehash(&mut bytes);
    assert_all_entry_points_reject(&bytes);
}

#[test]
fn rejects_overlapping_sections() {
    let mut bytes = valid_bytes();
    let first_range = entry(&bytes, SECTION_CONSTANT_RANGE_BASE);
    let second_range = entry(&bytes, SECTION_CONSTANT_RANGE_BASE + 1);
    let first_offset = read_u64(&bytes, first_range.offset_at);
    write_u64(&mut bytes, second_range.offset_at, first_offset);
    rehash(&mut bytes);
    assert_all_entry_points_reject(&bytes);
}

#[test]
fn rejects_out_of_order_sections() {
    let mut bytes = valid_bytes();
    let first_range = entry(&bytes, SECTION_CONSTANT_RANGE_BASE);
    let second_range = entry(&bytes, SECTION_CONSTANT_RANGE_BASE + 1);
    let first_offset = read_u64(&bytes, first_range.offset_at);
    let second_offset = read_u64(&bytes, second_range.offset_at);
    write_u64(&mut bytes, first_range.offset_at, second_offset);
    write_u64(&mut bytes, second_range.offset_at, first_offset);
    rehash(&mut bytes);
    assert_all_entry_points_reject(&bytes);
}

#[test]
fn rejects_undefined_section_flag() {
    let mut bytes = valid_bytes();
    let program = entry(&bytes, SECTION_PROGRAM);
    write_u32(&mut bytes, program.flags_at, 3);
    rehash(&mut bytes);
    assert_all_entry_points_reject(&bytes);
}

#[test]
fn rejects_non_dense_constant_range_kinds() {
    let mut bytes = valid_bytes();
    let second_range = entry(&bytes, SECTION_CONSTANT_RANGE_BASE + 1);
    write_u32(
        &mut bytes,
        second_range.kind_at,
        SECTION_CONSTANT_RANGE_BASE + 2,
    );
    rehash(&mut bytes);
    assert_all_entry_points_reject(&bytes);
}

#[test]
fn rejects_directory_entry_kinds_zero_and_one_even_when_optional() {
    for kind in [0, 1] {
        let mut bytes = valid_bytes();
        let second_range = entry(&bytes, SECTION_CONSTANT_RANGE_BASE + 1);
        write_u32(&mut bytes, second_range.kind_at, kind);
        write_u32(&mut bytes, second_range.flags_at, 0);
        rehash(&mut bytes);
        assert_all_entry_points_reject(&bytes);
    }
}

type SectionMutation = fn(&mut [u8], EntryOffsets);

#[test]
fn rejects_wrong_legacy_singleton_metadata() {
    let mutations: &[(u32, SectionMutation)] = &[
        (SECTION_MANIFEST, |bytes, section| {
            write_u32(bytes, section.alignment_at, 1)
        }),
        (SECTION_SCHEMAS, |bytes, section| {
            write_u32(bytes, section.count_at, 1)
        }),
        (SECTION_PROGRAM, |bytes, section| {
            bytes[section.profile_at] = 1
        }),
        (SECTION_CONSTANTS, |bytes, section| {
            write_u32(bytes, section.stride_at, 1)
        }),
        (SECTION_MANIFEST, |bytes, section| {
            let encoded_len = read_u64(bytes, section.encoded_len_at);
            write_u64(bytes, section.decoded_len_at, encoded_len + 1)
        }),
    ];

    for &(kind, mutate) in mutations {
        let mut bytes = valid_bytes();
        let section = entry(&bytes, kind);
        mutate(&mut bytes, section);
        rehash(&mut bytes);
        assert_all_entry_points_reject(&bytes);
    }
}

#[test]
fn rejects_wrong_constant_range_metadata() {
    for mutate in [
        |bytes: &mut Vec<u8>, section: EntryOffsets| write_u32(bytes, section.flags_at, 0),
        |bytes: &mut Vec<u8>, section: EntryOffsets| write_u32(bytes, section.alignment_at, 1),
    ] {
        let mut bytes = valid_bytes();
        let section = entry(&bytes, SECTION_CONSTANT_RANGE_BASE);
        mutate(&mut bytes, section);
        rehash(&mut bytes);
        assert_all_entry_points_reject(&bytes);
    }
}

#[test]
fn rejects_nonzero_section_padding() {
    let mut bytes = valid_bytes();
    let schemas = entry(&bytes, SECTION_SCHEMAS);
    let schemas_offset = read_u64(&bytes, schemas.offset_at) as usize;
    let previous = entry(&bytes, SECTION_MANIFEST);
    let previous_end =
        (read_u64(&bytes, previous.offset_at) + read_u64(&bytes, previous.encoded_len_at)) as usize;
    assert!(
        previous_end < schemas_offset,
        "fixture needs section padding"
    );
    bytes[previous_end] = 1;
    rehash(&mut bytes);
    assert_all_entry_points_reject(&bytes);
}

#[test]
fn rejects_unbounded_legacy_counts_before_allocation() {
    for kind in [
        SECTION_MANIFEST,
        SECTION_SCHEMAS,
        SECTION_PROGRAM,
        SECTION_CONSTANTS,
    ] {
        let mut bytes = valid_bytes();
        let section = entry(&bytes, kind);
        let offset = read_u64(&bytes, section.offset_at) as usize;
        let count_at = match kind {
            SECTION_MANIFEST => {
                let name_len = read_u32(&bytes, offset) as usize;
                offset + 4 + name_len + 4
            }
            SECTION_SCHEMAS | SECTION_PROGRAM | SECTION_CONSTANTS => offset,
            _ => unreachable!(),
        };
        write_u32(&mut bytes, count_at, u32::MAX);
        rehash(&mut bytes);
        assert_all_entry_points_reject(&bytes);
    }
}
