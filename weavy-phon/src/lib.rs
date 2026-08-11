//! PHON-backed durable module codec for Weavy.

use core::fmt;

use facet_value::{VArray, VObject, VString, Value};
use phon_schema::{
    Field, Primitive, Schema, SchemaId, SchemaKind, SchemaRef, primitive_id, resolve_ids,
    schema_from_bytes, schema_to_bytes,
};
use phon_storage::compact::{self, Registry};
use weavy::ir::{AggregateOp, ControlOp, InitOp, MemoryOp, WeavyOp};
use weavy::mem::{Layout, ScalarSegment};
use weavy::module::{Constant, ConstantPool, DialectRequirement, ModuleManifest, WeavyModule};
use weavy::{BlockRef, DenseLowered};

const MAGIC: [u8; 8] = *b"WEAVY\0\0\0";
const HEADER_SIZE: usize = 64;
const FORMAT_MAJOR: u16 = 1;
const FORMAT_MINOR: u16 = 0;
const DIRECTORY_ALIGNMENT: u32 = 8;
const DIRECTORY_SECTION_KIND: u32 = 1;
const SECTION_MANIFEST: u32 = 2;
const SECTION_SCHEMAS: u32 = 3;
const SECTION_PROGRAM: u32 = 4;
const SECTION_CONSTANTS: u32 = 5;
const FLAG_REQUIRED: u32 = 1;
const PROGRAM_SCHEMA_ID: u64 = 0x0bcb_92f4_3d1a_308a;
const CONSTANT_DIRECTORY_SCHEMA_ID: u64 = 0xd87c_d9d9_3b41_e5aa;

/// Intrinsic-specific durable byte encoding.
pub trait IntrinsicCodec {
    type Intrinsic;
    const DIALECT: &'static str;
    const SCHEMA_ID: u64;
    fn encode(intrinsic: &Self::Intrinsic, out: &mut Vec<u8>);
    fn decode(bytes: &[u8]) -> Result<Self::Intrinsic, CodecError>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SectionReport {
    pub name: String,
    pub kind: u32,
    pub offset: u64,
    pub encoded_len: u64,
    pub decoded_len: u64,
    pub alignment: u32,
    pub schema_id: u64,
    pub flags: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InspectionReport {
    pub module_name: String,
    pub executable_identity: [u8; 16],
    pub dialects: Vec<DialectRequirement>,
    pub sections: Vec<SectionReport>,
    pub program_op_count: usize,
    pub block_count: usize,
    pub constant_count: usize,
}

pub fn save<C: IntrinsicCodec>(module: &WeavyModule<C::Intrinsic>) -> Result<Vec<u8>, CodecError> {
    let manifest = encode_manifest(module.manifest());
    let schemas = encode_schema_bundle();
    let program = encode_program::<C>(module.program())?;
    let constants = encode_constants(module.constants())?;

    let payloads = [
        ("manifest", SECTION_MANIFEST, 1u64, manifest),
        ("schemas", SECTION_SCHEMAS, 1u64, schemas),
        ("program", SECTION_PROGRAM, PROGRAM_SCHEMA_ID, program),
        (
            "constants",
            SECTION_CONSTANTS,
            CONSTANT_DIRECTORY_SCHEMA_ID,
            constants,
        ),
    ];
    let directory_placeholder = encode_directory(&[])?;
    let mut cursor = align_up(
        HEADER_SIZE + directory_placeholder.len(),
        DIRECTORY_ALIGNMENT as usize,
    )?;
    let mut sections = Vec::with_capacity(payloads.len());
    for (name, kind, schema_id, payload) in &payloads {
        cursor = align_up(cursor, 8)?;
        sections.push(SectionReport {
            name: (*name).to_owned(),
            kind: *kind,
            offset: cursor as u64,
            encoded_len: payload.len() as u64,
            decoded_len: payload.len() as u64,
            alignment: 8,
            schema_id: *schema_id,
            flags: FLAG_REQUIRED,
        });
        cursor = cursor
            .checked_add(payload.len())
            .ok_or(CodecError::SizeOverflow)?;
    }
    let directory = encode_directory(&sections)?;
    let first_payload = align_up(HEADER_SIZE + directory.len(), DIRECTORY_ALIGNMENT as usize)?;
    let delta = first_payload as i128 - sections[0].offset as i128;
    if delta != 0 {
        for section in &mut sections {
            section.offset = u64::try_from(section.offset as i128 + delta)
                .map_err(|_| CodecError::SizeOverflow)?;
        }
    }
    let directory = encode_directory(&sections)?;
    let file_len = sections.iter().zip(payloads.iter()).try_fold(
        0usize,
        |_, (section, (_, _, _, payload))| {
            usize::try_from(section.offset)
                .ok()
                .and_then(|offset| offset.checked_add(payload.len()))
                .ok_or(CodecError::SizeOverflow)
        },
    )?;
    let mut bytes = vec![0; file_len];
    bytes[..8].copy_from_slice(&MAGIC);
    bytes[8..10].copy_from_slice(&FORMAT_MAJOR.to_le_bytes());
    bytes[10..12].copy_from_slice(&FORMAT_MINOR.to_le_bytes());
    bytes[12] = 1;
    bytes[16..24].copy_from_slice(&(file_len as u64).to_le_bytes());
    bytes[24..32].copy_from_slice(&(HEADER_SIZE as u64).to_le_bytes());
    bytes[32..40].copy_from_slice(&(directory.len() as u64).to_le_bytes());
    bytes[40..44].copy_from_slice(&DIRECTORY_ALIGNMENT.to_le_bytes());
    bytes[44..48].copy_from_slice(&DIRECTORY_SECTION_KIND.to_le_bytes());
    bytes[HEADER_SIZE..HEADER_SIZE + directory.len()].copy_from_slice(&directory);
    for (section, (_, _, _, payload)) in sections.iter().zip(payloads) {
        let offset = section.offset as usize;
        bytes[offset..offset + payload.len()].copy_from_slice(&payload);
    }
    let identity = executable_identity(&bytes[HEADER_SIZE..]);
    bytes[48..64].copy_from_slice(&identity);
    Ok(bytes)
}

pub fn load<C: IntrinsicCodec>(bytes: &[u8]) -> Result<WeavyModule<C::Intrinsic>, CodecError> {
    let parsed = parse_container(bytes)?;
    validate_schema_bundle(parsed.section(SECTION_SCHEMAS)?)?;
    let manifest = decode_manifest(parsed.section(SECTION_MANIFEST)?)?;
    let program = decode_program::<C>(parsed.section(SECTION_PROGRAM)?)?;
    let constants = decode_constants(parsed.section(SECTION_CONSTANTS)?)?;
    Ok(WeavyModule::new(manifest, program, constants))
}

pub fn inspect(bytes: &[u8]) -> Result<InspectionReport, CodecError> {
    let parsed = parse_container(bytes)?;
    let manifest = decode_manifest(parsed.section(SECTION_MANIFEST)?)?;
    let (program_op_count, block_count) = inspect_program(parsed.section(SECTION_PROGRAM)?)?;
    let constant_count = inspect_constants(parsed.section(SECTION_CONSTANTS)?)?;
    Ok(InspectionReport {
        module_name: manifest.name().to_owned(),
        executable_identity: parsed.identity,
        dialects: manifest.dialects().to_vec(),
        sections: parsed.sections,
        program_op_count,
        block_count,
        constant_count,
    })
}

struct Parsed<'a> {
    bytes: &'a [u8],
    sections: Vec<SectionReport>,
    identity: [u8; 16],
}
impl<'a> Parsed<'a> {
    fn section(&self, kind: u32) -> Result<&'a [u8], CodecError> {
        let section = self
            .sections
            .iter()
            .find(|section| section.kind == kind)
            .ok_or(CodecError::MissingSection { kind })?;
        let start = usize::try_from(section.offset).map_err(|_| CodecError::SizeOverflow)?;
        let len = usize::try_from(section.encoded_len).map_err(|_| CodecError::SizeOverflow)?;
        Ok(&self.bytes[start..start + len])
    }
}

fn parse_container(bytes: &[u8]) -> Result<Parsed<'_>, CodecError> {
    if bytes.len() < HEADER_SIZE {
        return Err(CodecError::Truncated {
            needed: HEADER_SIZE,
            actual: bytes.len(),
        });
    }
    if bytes[..8] != MAGIC {
        return Err(CodecError::BadMagic);
    }
    let major = read_u16(bytes, 8)?;
    let minor = read_u16(bytes, 10)?;
    if major != FORMAT_MAJOR {
        return Err(CodecError::UnsupportedFormat { major, minor });
    }
    let file_len = usize_from_u64(read_u64(bytes, 16)?)?;
    if file_len != bytes.len() {
        return Err(CodecError::Truncated {
            needed: file_len,
            actual: bytes.len(),
        });
    }
    let directory_offset = usize_from_u64(read_u64(bytes, 24)?)?;
    let directory_len = usize_from_u64(read_u64(bytes, 32)?)?;
    let alignment = read_u32(bytes, 40)?;
    if alignment == 0 || !alignment.is_power_of_two() {
        return Err(CodecError::InvalidAlignment { alignment });
    }
    let directory_kind = read_u32(bytes, 44)?;
    if directory_kind != DIRECTORY_SECTION_KIND {
        return Err(CodecError::UnknownRequiredSection {
            kind: directory_kind,
        });
    }
    let directory_end =
        directory_offset
            .checked_add(directory_len)
            .ok_or(CodecError::SectionOutOfBounds {
                offset: directory_offset as u64,
                len: directory_len as u64,
                file_len: bytes.len(),
            })?;
    if directory_offset < HEADER_SIZE || directory_end > bytes.len() {
        return Err(CodecError::SectionOutOfBounds {
            offset: directory_offset as u64,
            len: directory_len as u64,
            file_len: bytes.len(),
        });
    }
    let expected: [u8; 16] = bytes[48..64].try_into().expect("header length");
    let actual = executable_identity(&bytes[HEADER_SIZE..]);
    if expected != actual {
        return Err(CodecError::IntegrityMismatch { expected, actual });
    }
    let sections = decode_directory(&bytes[directory_offset..directory_end])?;
    for section in &sections {
        if section.alignment == 0 || !section.alignment.is_power_of_two() {
            return Err(CodecError::InvalidAlignment {
                alignment: section.alignment,
            });
        }
        let start = usize_from_u64(section.offset)?;
        let len = usize_from_u64(section.encoded_len)?;
        let end = start.checked_add(len).ok_or(CodecError::SizeOverflow)?;
        if !start.is_multiple_of(section.alignment as usize) || end > bytes.len() {
            return Err(CodecError::SectionOutOfBounds {
                offset: section.offset,
                len: section.encoded_len,
                file_len: bytes.len(),
            });
        }
        if !matches!(
            section.kind,
            SECTION_MANIFEST | SECTION_SCHEMAS | SECTION_PROGRAM | SECTION_CONSTANTS
        ) && section.flags & FLAG_REQUIRED != 0
        {
            return Err(CodecError::UnknownRequiredSection { kind: section.kind });
        }
    }
    Ok(Parsed {
        bytes,
        sections,
        identity: expected,
    })
}

fn directory_schemas() -> (Vec<Schema>, SchemaId) {
    let section = Schema {
        id: SchemaId::from_raw(1),
        type_params: Vec::new(),
        kind: SchemaKind::Struct {
            name: "WeavySection".into(),
            fields: vec![
                field("name", Primitive::String),
                field("kind", Primitive::U32),
                field("offset", Primitive::U64),
                field("encoded_len", Primitive::U64),
                field("decoded_len", Primitive::U64),
                field("alignment", Primitive::U32),
                field("schema_id", Primitive::U64),
                field("flags", Primitive::U32),
            ],
        },
    };
    let directory = Schema {
        id: SchemaId::from_raw(2),
        type_params: Vec::new(),
        kind: SchemaKind::List {
            element: SchemaRef::concrete(section.id),
        },
    };
    let schemas = resolve_ids(vec![section, directory]);
    let root = schemas[1].id;
    (schemas, root)
}
fn field(name: &str, primitive: Primitive) -> Field {
    Field {
        name: name.into(),
        schema: SchemaRef::concrete(primitive_id(primitive)),
        required: true,
    }
}
fn encode_directory(sections: &[SectionReport]) -> Result<Vec<u8>, CodecError> {
    let (schemas, root) = directory_schemas();
    let registry = Registry::new(schemas);
    let mut array = VArray::new();
    for section in sections {
        let mut object = VObject::new();
        object.insert(VString::new("name"), Value::from(section.name.as_str()));
        object.insert(VString::new("kind"), Value::from(section.kind));
        object.insert(VString::new("offset"), Value::from(section.offset));
        object.insert(
            VString::new("encoded_len"),
            Value::from(section.encoded_len),
        );
        object.insert(
            VString::new("decoded_len"),
            Value::from(section.decoded_len),
        );
        object.insert(VString::new("alignment"), Value::from(section.alignment));
        object.insert(VString::new("schema_id"), Value::from(section.schema_id));
        object.insert(VString::new("flags"), Value::from(section.flags));
        array.push(object);
    }
    compact::to_bytes(&array.into(), root, &registry).map_err(CodecError::Phon)
}
fn decode_directory(bytes: &[u8]) -> Result<Vec<SectionReport>, CodecError> {
    let (schemas, root) = directory_schemas();
    let registry = Registry::new(schemas);
    let value = compact::from_bytes(bytes, root, &registry).map_err(CodecError::Phon)?;
    let array = value.as_array().ok_or(CodecError::MalformedDirectory)?;
    (0..array.len())
        .map(|index| {
            let object = array
                .get(index)
                .and_then(Value::as_object)
                .ok_or(CodecError::MalformedDirectory)?;
            Ok(SectionReport {
                name: object_string(object, "name")?,
                kind: object_u32(object, "kind")?,
                offset: object_u64(object, "offset")?,
                encoded_len: object_u64(object, "encoded_len")?,
                decoded_len: object_u64(object, "decoded_len")?,
                alignment: object_u32(object, "alignment")?,
                schema_id: object_u64(object, "schema_id")?,
                flags: object_u32(object, "flags")?,
            })
        })
        .collect()
}
fn object_value<'a>(object: &'a VObject, name: &str) -> Result<&'a Value, CodecError> {
    object
        .get(&VString::new(name))
        .ok_or(CodecError::MalformedDirectory)
}
fn object_string(object: &VObject, name: &str) -> Result<String, CodecError> {
    object_value(object, name)?
        .as_string()
        .map(|value| value.as_str().to_owned())
        .ok_or(CodecError::MalformedDirectory)
}
fn object_u64(object: &VObject, name: &str) -> Result<u64, CodecError> {
    object_value(object, name)?
        .as_number()
        .and_then(|value| value.to_u64())
        .ok_or(CodecError::MalformedDirectory)
}
fn object_u32(object: &VObject, name: &str) -> Result<u32, CodecError> {
    u32::try_from(object_u64(object, name)?).map_err(|_| CodecError::MalformedDirectory)
}

fn encode_manifest(manifest: &ModuleManifest) -> Vec<u8> {
    let mut out = Vec::new();
    put_string(&mut out, manifest.name());
    put_u16(&mut out, manifest.format_major());
    put_u16(&mut out, manifest.format_minor());
    put_u32(&mut out, manifest.dialects().len() as u32);
    for dialect in manifest.dialects() {
        put_string(&mut out, dialect.name());
        put_u16(&mut out, dialect.major());
        put_u16(&mut out, dialect.minor());
    }
    put_u32(&mut out, manifest.root_entries().len() as u32);
    for root in manifest.root_entries() {
        put_u32(&mut out, *root);
    }
    out
}
fn decode_manifest(bytes: &[u8]) -> Result<ModuleManifest, CodecError> {
    let mut r = Reader::new(bytes);
    let name = r.string()?;
    let major = r.u16()?;
    let minor = r.u16()?;
    if major != 1 || minor != 0 {
        return Err(CodecError::UnsupportedFormat { major, minor });
    }
    let dialect_count = r.u32()?;
    let mut dialects = Vec::with_capacity(dialect_count as usize);
    for _ in 0..dialect_count {
        dialects.push(DialectRequirement::new(r.string()?, r.u16()?, r.u16()?));
    }
    let root_count = r.u32()?;
    let mut roots = Vec::with_capacity(root_count as usize);
    for _ in 0..root_count {
        roots.push(r.u32()?);
    }
    r.finish()?;
    Ok(ModuleManifest::new(name, dialects, roots))
}
fn encode_schema_bundle() -> Vec<u8> {
    let (schemas, _) = directory_schemas();
    let mut out = Vec::new();
    put_u32(&mut out, schemas.len() as u32);
    for schema in &schemas {
        let bytes = schema_to_bytes(schema);
        put_bytes(&mut out, &bytes);
    }
    out
}

fn validate_schema_bundle(bytes: &[u8]) -> Result<(), CodecError> {
    let mut r = Reader::new(bytes);
    let count = r.u32()?;
    for _ in 0..count {
        schema_from_bytes(r.bytes()?).map_err(|_| CodecError::MalformedSchemas)?;
    }
    r.finish()
}

fn encode_program<C: IntrinsicCodec>(
    program: &DenseLowered<WeavyOp<BlockRef, C::Intrinsic>>,
) -> Result<Vec<u8>, CodecError> {
    let mut out = Vec::new();
    encode_ops::<C>(&program.program, &mut out)?;
    put_u32(
        &mut out,
        u32::try_from(program.blocks.len()).map_err(|_| CodecError::SizeOverflow)?,
    );
    for block in &program.blocks {
        encode_ops::<C>(block, &mut out)?;
    }
    Ok(out)
}
fn encode_ops<C: IntrinsicCodec>(
    ops: &[WeavyOp<BlockRef, C::Intrinsic>],
    out: &mut Vec<u8>,
) -> Result<(), CodecError> {
    put_u32(
        out,
        u32::try_from(ops.len()).map_err(|_| CodecError::SizeOverflow)?,
    );
    for op in ops {
        encode_op::<C>(op, out)?;
    }
    Ok(())
}
fn encode_op<C: IntrinsicCodec>(
    op: &WeavyOp<BlockRef, C::Intrinsic>,
    out: &mut Vec<u8>,
) -> Result<(), CodecError> {
    match op {
        WeavyOp::Control(ControlOp::CallBlock { block, base_offset }) => {
            out.push(1);
            put_block(out, *block)?;
            put_usize(out, *base_offset)?;
        }
        WeavyOp::Control(ControlOp::CallBlockThen {
            block,
            then,
            base_offset,
        }) => {
            out.push(2);
            put_block(out, *block)?;
            put_block(out, *then)?;
            put_usize(out, *base_offset)?;
        }
        WeavyOp::Control(ControlOp::Return) => out.push(3),
        WeavyOp::Memory(MemoryOp::ScalarCopy {
            offset,
            size,
            align,
        }) => {
            out.push(10);
            put_usize(out, *offset)?;
            put_usize(out, *size)?;
            put_usize(out, *align)?;
        }
        WeavyOp::Memory(MemoryOp::ScalarRun { segments }) => {
            out.push(11);
            put_u32(out, segments.len() as u32);
            for segment in segments {
                put_usize(out, segment.offset)?;
                put_usize(out, segment.size)?;
                put_usize(out, segment.align)?;
            }
        }
        WeavyOp::Memory(MemoryOp::Zero { offset, size }) => {
            out.push(12);
            put_usize(out, *offset)?;
            put_usize(out, *size)?;
        }
        WeavyOp::Memory(MemoryOp::Move {
            src_offset,
            dst_offset,
            size,
            align,
        }) => {
            out.push(13);
            for value in [*src_offset, *dst_offset, *size, *align] {
                put_usize(out, value)?;
            }
        }
        WeavyOp::Memory(MemoryOp::Drop { offset, layout }) => {
            out.push(14);
            put_usize(out, *offset)?;
            put_layout(out, *layout)?;
        }
        WeavyOp::Init(InitOp::Default { offset }) => {
            out.push(20);
            put_usize(out, *offset)?;
        }
        WeavyOp::Init(InitOp::OptionNone { offset }) => {
            out.push(21);
            put_usize(out, *offset)?;
        }
        WeavyOp::Init(InitOp::OptionSome { offset, inner }) => {
            out.push(22);
            put_usize(out, *offset)?;
            put_layout(out, *inner)?;
        }
        WeavyOp::Init(InitOp::ListFromRawParts {
            offset,
            element,
            len,
            cap,
        }) => {
            out.push(23);
            put_usize(out, *offset)?;
            put_layout(out, *element)?;
            put_usize(out, *len)?;
            put_usize(out, *cap)?;
        }
        WeavyOp::Init(InitOp::PointerFromScratch { offset, pointee }) => {
            out.push(24);
            put_usize(out, *offset)?;
            put_layout(out, *pointee)?;
        }
        WeavyOp::Aggregate(AggregateOp::BeginRecord { field_count }) => {
            out.push(30);
            put_usize(out, *field_count)?;
        }
        WeavyOp::Aggregate(AggregateOp::RecordField { index, offset }) => {
            out.push(31);
            put_usize(out, *index)?;
            put_usize(out, *offset)?;
        }
        WeavyOp::Aggregate(AggregateOp::FinishRecord) => out.push(32),
        WeavyOp::Aggregate(AggregateOp::BeginList {
            offset,
            element,
            loop_block,
        }) => {
            out.push(33);
            put_usize(out, *offset)?;
            put_layout(out, *element)?;
            put_block(out, *loop_block)?;
        }
        WeavyOp::Aggregate(AggregateOp::FinishList) => out.push(34),
        WeavyOp::Intrinsic(intrinsic) => {
            out.push(40);
            let mut bytes = Vec::new();
            C::encode(intrinsic, &mut bytes);
            put_bytes(out, &bytes);
        }
        _ => return Err(CodecError::UnsupportedOperation),
    }
    Ok(())
}
fn decode_program<C: IntrinsicCodec>(
    bytes: &[u8],
) -> Result<DenseLowered<WeavyOp<BlockRef, C::Intrinsic>>, CodecError> {
    let mut r = Reader::new(bytes);
    let program = decode_ops::<C>(&mut r)?;
    let count = r.u32()?;
    let mut blocks = Vec::with_capacity(count as usize);
    for _ in 0..count {
        blocks.push(decode_ops::<C>(&mut r)?);
    }
    r.finish()?;
    Ok(DenseLowered::new(program, blocks))
}
fn decode_ops<C: IntrinsicCodec>(
    r: &mut Reader<'_>,
) -> Result<Vec<WeavyOp<BlockRef, C::Intrinsic>>, CodecError> {
    let count = r.u32()?;
    let mut ops = Vec::with_capacity(count as usize);
    for _ in 0..count {
        ops.push(decode_op::<C>(r)?);
    }
    Ok(ops)
}
fn decode_op<C: IntrinsicCodec>(
    r: &mut Reader<'_>,
) -> Result<WeavyOp<BlockRef, C::Intrinsic>, CodecError> {
    Ok(match r.u8()? {
        1 => WeavyOp::Control(ControlOp::CallBlock {
            block: r.block()?,
            base_offset: r.usize()?,
        }),
        2 => WeavyOp::Control(ControlOp::CallBlockThen {
            block: r.block()?,
            then: r.block()?,
            base_offset: r.usize()?,
        }),
        3 => WeavyOp::Control(ControlOp::Return),
        10 => WeavyOp::Memory(MemoryOp::ScalarCopy {
            offset: r.usize()?,
            size: r.usize()?,
            align: r.usize()?,
        }),
        11 => {
            let n = r.u32()?;
            let mut segments = Vec::with_capacity(n as usize);
            for _ in 0..n {
                segments.push(ScalarSegment {
                    offset: r.usize()?,
                    size: r.usize()?,
                    align: r.usize()?,
                });
            }
            WeavyOp::Memory(MemoryOp::ScalarRun { segments })
        }
        12 => WeavyOp::Memory(MemoryOp::Zero {
            offset: r.usize()?,
            size: r.usize()?,
        }),
        13 => WeavyOp::Memory(MemoryOp::Move {
            src_offset: r.usize()?,
            dst_offset: r.usize()?,
            size: r.usize()?,
            align: r.usize()?,
        }),
        14 => WeavyOp::Memory(MemoryOp::Drop {
            offset: r.usize()?,
            layout: r.layout()?,
        }),
        20 => WeavyOp::Init(InitOp::Default { offset: r.usize()? }),
        21 => WeavyOp::Init(InitOp::OptionNone { offset: r.usize()? }),
        22 => WeavyOp::Init(InitOp::OptionSome {
            offset: r.usize()?,
            inner: r.layout()?,
        }),
        23 => WeavyOp::Init(InitOp::ListFromRawParts {
            offset: r.usize()?,
            element: r.layout()?,
            len: r.usize()?,
            cap: r.usize()?,
        }),
        24 => WeavyOp::Init(InitOp::PointerFromScratch {
            offset: r.usize()?,
            pointee: r.layout()?,
        }),
        30 => WeavyOp::Aggregate(AggregateOp::BeginRecord {
            field_count: r.usize()?,
        }),
        31 => WeavyOp::Aggregate(AggregateOp::RecordField {
            index: r.usize()?,
            offset: r.usize()?,
        }),
        32 => WeavyOp::Aggregate(AggregateOp::FinishRecord),
        33 => WeavyOp::Aggregate(AggregateOp::BeginList {
            offset: r.usize()?,
            element: r.layout()?,
            loop_block: r.block()?,
        }),
        34 => WeavyOp::Aggregate(AggregateOp::FinishList),
        40 => WeavyOp::Intrinsic(C::decode(r.bytes()?)?),
        _ => return Err(CodecError::MalformedProgram),
    })
}
fn inspect_program(bytes: &[u8]) -> Result<(usize, usize), CodecError> {
    let mut r = Reader::new(bytes);
    let roots = skip_ops(&mut r)?;
    let blocks = r.u32()? as usize;
    let mut total = roots;
    for _ in 0..blocks {
        total += skip_ops(&mut r)?;
    }
    r.finish()?;
    Ok((total, blocks))
}
fn skip_ops(r: &mut Reader<'_>) -> Result<usize, CodecError> {
    let count = r.u32()? as usize;
    for _ in 0..count {
        skip_op(r)?;
    }
    Ok(count)
}
fn skip_op(r: &mut Reader<'_>) -> Result<(), CodecError> {
    match r.u8()? {
        1 => {
            r.u32()?;
            r.u64()?;
        }
        2 => {
            r.u32()?;
            r.u32()?;
            r.u64()?;
        }
        3 | 32 | 34 => {}
        10 => {
            for _ in 0..3 {
                r.u64()?;
            }
        }
        11 => {
            let n = r.u32()?;
            for _ in 0..n * 3 {
                r.u64()?;
            }
        }
        12 | 20 | 21 | 30 => {
            r.u64()?;
        }
        13 => {
            for _ in 0..4 {
                r.u64()?;
            }
        }
        14 | 22 | 24 => {
            r.u64()?;
            r.u64()?;
            r.u64()?;
        }
        23 => {
            for _ in 0..5 {
                r.u64()?;
            }
        }
        31 => {
            r.u64()?;
            r.u64()?;
        }
        33 => {
            r.u64()?;
            r.u64()?;
            r.u64()?;
            r.u32()?;
        }
        40 => {
            r.bytes()?;
        }
        _ => return Err(CodecError::MalformedProgram),
    }
    Ok(())
}

fn encode_constants(pool: &ConstantPool) -> Result<Vec<u8>, CodecError> {
    let mut out = Vec::new();
    put_u32(
        &mut out,
        u32::try_from(pool.len()).map_err(|_| CodecError::SizeOverflow)?,
    );
    for index in 0..pool.len() {
        let constant = &pool[index];
        put_u64(&mut out, constant.schema_id());
        put_bytes(&mut out, constant.bytes());
    }
    Ok(out)
}
fn decode_constants(bytes: &[u8]) -> Result<ConstantPool, CodecError> {
    let mut r = Reader::new(bytes);
    let count = r.u32()?;
    let mut constants = Vec::with_capacity(count as usize);
    for _ in 0..count {
        constants.push(Constant::new(r.u64()?, r.bytes()?.to_vec()));
    }
    r.finish()?;
    Ok(ConstantPool::new(constants))
}
fn inspect_constants(bytes: &[u8]) -> Result<usize, CodecError> {
    let mut r = Reader::new(bytes);
    let count = r.u32()?;
    for _ in 0..count {
        r.u64()?;
        r.bytes()?;
    }
    r.finish()?;
    Ok(count as usize)
}

fn executable_identity(bytes: &[u8]) -> [u8; 16] {
    blake3::hash(bytes).as_bytes()[..16]
        .try_into()
        .expect("hash length")
}
fn align_up(value: usize, alignment: usize) -> Result<usize, CodecError> {
    value
        .checked_add(alignment - 1)
        .map(|v| v & !(alignment - 1))
        .ok_or(CodecError::SizeOverflow)
}
fn put_u16(out: &mut Vec<u8>, v: u16) {
    out.extend_from_slice(&v.to_le_bytes())
}
fn put_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes())
}
fn put_u64(out: &mut Vec<u8>, v: u64) {
    out.extend_from_slice(&v.to_le_bytes())
}
fn put_usize(out: &mut Vec<u8>, v: usize) -> Result<(), CodecError> {
    put_u64(out, u64::try_from(v).map_err(|_| CodecError::SizeOverflow)?);
    Ok(())
}
fn put_block(out: &mut Vec<u8>, v: BlockRef) -> Result<(), CodecError> {
    put_u32(
        out,
        u32::try_from(v.index()).map_err(|_| CodecError::SizeOverflow)?,
    );
    Ok(())
}
fn put_layout(out: &mut Vec<u8>, v: Layout) -> Result<(), CodecError> {
    put_usize(out, v.size)?;
    put_usize(out, v.align)
}
fn put_string(out: &mut Vec<u8>, v: &str) {
    put_bytes(out, v.as_bytes())
}
fn put_bytes(out: &mut Vec<u8>, v: &[u8]) {
    put_u32(out, v.len() as u32);
    out.extend_from_slice(v)
}
fn read_u16(b: &[u8], o: usize) -> Result<u16, CodecError> {
    Ok(u16::from_le_bytes(read_array(b, o)?))
}
fn read_u32(b: &[u8], o: usize) -> Result<u32, CodecError> {
    Ok(u32::from_le_bytes(read_array(b, o)?))
}
fn read_u64(b: &[u8], o: usize) -> Result<u64, CodecError> {
    Ok(u64::from_le_bytes(read_array(b, o)?))
}
fn read_array<const N: usize>(b: &[u8], o: usize) -> Result<[u8; N], CodecError> {
    let e = o.checked_add(N).ok_or(CodecError::SizeOverflow)?;
    b.get(o..e)
        .ok_or(CodecError::Truncated {
            needed: e,
            actual: b.len(),
        })?
        .try_into()
        .map_err(|_| CodecError::Truncated {
            needed: e,
            actual: b.len(),
        })
}
fn usize_from_u64(v: u64) -> Result<usize, CodecError> {
    usize::try_from(v).map_err(|_| CodecError::SizeOverflow)
}
struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}
impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }
    fn take(&mut self, n: usize) -> Result<&'a [u8], CodecError> {
        let end = self.pos.checked_add(n).ok_or(CodecError::SizeOverflow)?;
        let result = self.bytes.get(self.pos..end).ok_or(CodecError::Truncated {
            needed: end,
            actual: self.bytes.len(),
        })?;
        self.pos = end;
        Ok(result)
    }
    fn u8(&mut self) -> Result<u8, CodecError> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, CodecError> {
        Ok(u16::from_le_bytes(
            self.take(2)?.try_into().expect("length"),
        ))
    }
    fn u32(&mut self) -> Result<u32, CodecError> {
        Ok(u32::from_le_bytes(
            self.take(4)?.try_into().expect("length"),
        ))
    }
    fn u64(&mut self) -> Result<u64, CodecError> {
        Ok(u64::from_le_bytes(
            self.take(8)?.try_into().expect("length"),
        ))
    }
    fn usize(&mut self) -> Result<usize, CodecError> {
        usize_from_u64(self.u64()?)
    }
    fn block(&mut self) -> Result<BlockRef, CodecError> {
        Ok(BlockRef::new(self.u32()? as usize))
    }
    fn bytes(&mut self) -> Result<&'a [u8], CodecError> {
        let len = self.u32()? as usize;
        self.take(len)
    }
    fn string(&mut self) -> Result<String, CodecError> {
        String::from_utf8(self.bytes()?.to_vec()).map_err(|_| CodecError::InvalidUtf8)
    }
    fn layout(&mut self) -> Result<Layout, CodecError> {
        Ok(Layout {
            size: self.usize()?,
            align: self.usize()?,
        })
    }
    fn finish(&self) -> Result<(), CodecError> {
        if self.pos == self.bytes.len() {
            Ok(())
        } else {
            Err(CodecError::TrailingBytes {
                count: self.bytes.len() - self.pos,
            })
        }
    }
}

#[derive(Debug)]
#[non_exhaustive]
pub enum CodecError {
    BadMagic,
    UnsupportedFormat {
        major: u16,
        minor: u16,
    },
    Truncated {
        needed: usize,
        actual: usize,
    },
    SizeOverflow,
    InvalidAlignment {
        alignment: u32,
    },
    SectionOutOfBounds {
        offset: u64,
        len: u64,
        file_len: usize,
    },
    UnknownRequiredSection {
        kind: u32,
    },
    MissingSection {
        kind: u32,
    },
    IntegrityMismatch {
        expected: [u8; 16],
        actual: [u8; 16],
    },
    MalformedDirectory,
    MalformedSchemas,
    MalformedProgram,
    MalformedIntrinsic,
    UnsupportedOperation,
    InvalidUtf8,
    TrailingBytes {
        count: usize,
    },
    Phon(compact::CompactError),
}
impl fmt::Display for CodecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Weavy PHON codec error: {self:?}")
    }
}
impl std::error::Error for CodecError {}
