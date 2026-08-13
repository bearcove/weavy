//! PHON-backed durable module codec for Weavy.

use core::fmt;

use facet_value::Value;
use phon_schema::{
    Field, Primitive, Schema, SchemaId, SchemaKind, SchemaRef, primitive_id, resolve_ids,
    schema_from_bytes, schema_matches_bytes, schema_to_bytes,
};
use phon_storage::compact::Registry;
use phon_storage::{AlignedDocument, AlignedRegistry, DenseRange, compact};
use weavy::ir::{AggregateOp, ControlOp, InitOp, MemoryOp, WeavyOp};
use weavy::mem::{Layout, ScalarSegment};
use weavy::module::{
    AdmissionError, Constant, ConstantPool, ConstantRange, ConstantRangeError,
    ConstantRangeMetadata, DialectRequirement, ModuleManifest, ModuleVerifier, StorageProfile,
    WeavyModule,
};
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
const SECTION_CONSTANT_RANGE_BASE: u32 = 0x1000;
const FLAG_REQUIRED: u32 = 1;
const PROGRAM_SCHEMA_ID: u64 = 0x0bcb_92f4_3d1a_308a;
const CONSTANT_DIRECTORY_SCHEMA_ID: u64 = 0xd87c_d9d9_3b41_e5aa;
const DIRECTORY_SECTION_SCHEMA_ID: u64 = 0x27e8_8229_2860_ab54;
const DIRECTORY_SCHEMA_ID: u64 = 0xcf54_d756_8f59_3290;
const DEFAULT_MAX_IMAGE_BYTES: usize = 1 << 30;
const DEFAULT_MAX_DIRECTORY_BYTES: usize = 16 * 1024 * 1024;
const DEFAULT_MAX_SECTIONS: usize = 1 << 16;
const DEFAULT_MAX_SCHEMAS: usize = 1 << 16;
const DEFAULT_MAX_MANIFEST_DIALECTS: usize = 1 << 16;
const DEFAULT_MAX_MANIFEST_ROOTS: usize = 1 << 20;
const DEFAULT_MAX_PROGRAM_BLOCKS: usize = 1 << 20;
const DEFAULT_MAX_PROGRAM_OPS: usize = 1 << 24;
const DEFAULT_MAX_SCALAR_SEGMENTS: usize = 1 << 20;
const DEFAULT_MAX_CONSTANTS: usize = 1 << 20;
const DEFAULT_MAX_DECODED_BYTES: usize = 256 * 1024 * 1024;
const DEFAULT_MAX_RETAINED_BYTES: usize = 256 * 1024 * 1024;
const MIN_DIRECTORY_ENTRY_BYTES: usize = 73;

/// Caller-selected ceilings for phase-0 container parsing and byte-derived collections.
///
/// These limits bound physical decoding only. They are not the semantic admission policy
/// described by Weavy's broader `AdmissionLimits` design.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContainerLimits {
    max_image_bytes: usize,
    max_directory_bytes: usize,
    max_sections: usize,
    max_schemas: usize,
    max_manifest_dialects: usize,
    max_manifest_roots: usize,
    max_program_blocks: usize,
    max_program_ops: usize,
    max_scalar_segments: usize,
    max_constants: usize,
    max_decoded_bytes: usize,
    max_retained_bytes: usize,
}

impl ContainerLimits {
    /// Default phase-0 ceilings for callers that do not need tighter bounds.
    pub const DEFAULT: Self = Self {
        max_image_bytes: DEFAULT_MAX_IMAGE_BYTES,
        max_directory_bytes: DEFAULT_MAX_DIRECTORY_BYTES,
        max_sections: DEFAULT_MAX_SECTIONS,
        max_schemas: DEFAULT_MAX_SCHEMAS,
        max_manifest_dialects: DEFAULT_MAX_MANIFEST_DIALECTS,
        max_manifest_roots: DEFAULT_MAX_MANIFEST_ROOTS,
        max_program_blocks: DEFAULT_MAX_PROGRAM_BLOCKS,
        max_program_ops: DEFAULT_MAX_PROGRAM_OPS,
        max_scalar_segments: DEFAULT_MAX_SCALAR_SEGMENTS,
        max_constants: DEFAULT_MAX_CONSTANTS,
        max_decoded_bytes: DEFAULT_MAX_DECODED_BYTES,
        max_retained_bytes: DEFAULT_MAX_RETAINED_BYTES,
    };

    #[must_use]
    pub const fn with_max_image_bytes(mut self, max_image_bytes: usize) -> Self {
        self.max_image_bytes = max_image_bytes;
        self
    }

    #[must_use]
    pub const fn with_max_directory_bytes(mut self, max_directory_bytes: usize) -> Self {
        self.max_directory_bytes = max_directory_bytes;
        self
    }

    #[must_use]
    pub const fn with_max_sections(mut self, max_sections: usize) -> Self {
        self.max_sections = max_sections;
        self
    }

    #[must_use]
    pub const fn with_max_schemas(mut self, max_schemas: usize) -> Self {
        self.max_schemas = max_schemas;
        self
    }

    #[must_use]
    pub const fn with_max_manifest_dialects(mut self, max_manifest_dialects: usize) -> Self {
        self.max_manifest_dialects = max_manifest_dialects;
        self
    }

    #[must_use]
    pub const fn with_max_manifest_roots(mut self, max_manifest_roots: usize) -> Self {
        self.max_manifest_roots = max_manifest_roots;
        self
    }

    #[must_use]
    pub const fn with_max_program_blocks(mut self, max_program_blocks: usize) -> Self {
        self.max_program_blocks = max_program_blocks;
        self
    }

    #[must_use]
    pub const fn with_max_program_ops(mut self, max_program_ops: usize) -> Self {
        self.max_program_ops = max_program_ops;
        self
    }

    #[must_use]
    pub const fn with_max_scalar_segments(mut self, max_scalar_segments: usize) -> Self {
        self.max_scalar_segments = max_scalar_segments;
        self
    }

    #[must_use]
    pub const fn with_max_constants(mut self, max_constants: usize) -> Self {
        self.max_constants = max_constants;
        self
    }

    #[must_use]
    pub const fn with_max_decoded_bytes(mut self, max_decoded_bytes: usize) -> Self {
        self.max_decoded_bytes = max_decoded_bytes;
        self
    }

    #[must_use]
    pub const fn with_max_retained_bytes(mut self, max_retained_bytes: usize) -> Self {
        self.max_retained_bytes = max_retained_bytes;
        self
    }
}

impl Default for ContainerLimits {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// The specific phase-0 ceiling exceeded while parsing an image.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum ContainerLimitKind {
    ImageBytes,
    DirectoryBytes,
    Sections,
    Schemas,
    ManifestDialects,
    ManifestRoots,
    ProgramBlocks,
    ProgramOps,
    ScalarSegments,
    Constants,
    DecodedBytes,
    RetainedBytes,
}

fn enforce_limit(
    kind: ContainerLimitKind,
    configured: usize,
    actual: usize,
) -> Result<(), CodecError> {
    if actual > configured {
        Err(CodecError::ContainerLimitExceeded {
            kind,
            configured,
            actual,
        })
    } else {
        Ok(())
    }
}

struct ByteBudgets {
    limits: ContainerLimits,
    decoded: usize,
    retained: usize,
}

impl ByteBudgets {
    const fn new(limits: ContainerLimits) -> Self {
        Self {
            limits,
            decoded: 0,
            retained: 0,
        }
    }

    fn decoded(&mut self, bytes: usize) -> Result<(), CodecError> {
        self.decoded = self
            .decoded
            .checked_add(bytes)
            .ok_or(CodecError::SizeOverflow)?;
        enforce_limit(
            ContainerLimitKind::DecodedBytes,
            self.limits.max_decoded_bytes,
            self.decoded,
        )
    }

    fn retained(&mut self, bytes: usize) -> Result<(), CodecError> {
        self.retained = self
            .retained
            .checked_add(bytes)
            .ok_or(CodecError::SizeOverflow)?;
        enforce_limit(
            ContainerLimitKind::RetainedBytes,
            self.limits.max_retained_bytes,
            self.retained,
        )
    }

    fn reserve<T>(&mut self, count: usize) -> Result<Vec<T>, CodecError> {
        self.retained(
            count
                .checked_mul(core::mem::size_of::<T>())
                .ok_or(CodecError::SizeOverflow)?,
        )?;
        let mut values = Vec::new();
        values
            .try_reserve_exact(count)
            .map_err(|_| CodecError::AdmissionLimitExceeded)?;
        Ok(values)
    }

    fn copy_bytes(&mut self, bytes: &[u8]) -> Result<Vec<u8>, CodecError> {
        self.retained(bytes.len())?;
        let mut owned = Vec::new();
        owned
            .try_reserve_exact(bytes.len())
            .map_err(|_| CodecError::AdmissionLimitExceeded)?;
        owned.extend_from_slice(bytes);
        Ok(owned)
    }

    fn copy_string(&mut self, value: &str) -> Result<String, CodecError> {
        self.retained(value.len())?;
        let mut owned = String::new();
        owned
            .try_reserve_exact(value.len())
            .map_err(|_| CodecError::AdmissionLimitExceeded)?;
        owned.push_str(value);
        Ok(owned)
    }
}
/// Truncated payload-integrity digest stored in the format-1.0 header.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PayloadIntegrityTag([u8; 16]);

impl PayloadIntegrityTag {
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

/// Full BLAKE3 digest of the exact complete received module image.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ImageId([u8; 32]);

impl ImageId {
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    #[must_use]
    pub const fn into_bytes(self) -> [u8; 32] {
        self.0
    }
}

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
    pub profile: Option<StorageProfile>,
    pub count: u32,
    pub stride: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConstantRangeReport {
    pub id: u32,
    pub schema_id: SchemaId,
    pub profile: StorageProfile,
    pub count: u32,
    pub stride: u32,
    pub encoded_len: u64,
}

/// Semantic inspection facts decoded from a structurally valid image.
///
/// This report is descriptive only; it does not confer admission authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InspectionReport {
    pub module_name: String,
    pub payload_integrity_tag: PayloadIntegrityTag,
    pub image_id: ImageId,
    pub dialects: Vec<DialectRequirement>,
    pub sections: Vec<SectionReport>,
    pub program_op_count: usize,
    pub block_count: usize,
    pub constant_count: usize,
    pub constant_ranges: Vec<ConstantRangeReport>,
}

/// Bounded physical facts from the module header and canonical directory.
///
/// Structural inspection verifies framing and payload integrity but deliberately
/// does not decode or validate semantic section payloads. Its result carries no
/// semantic or admission authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StructuralInspection {
    pub payload_integrity_tag: PayloadIntegrityTag,
    pub image_id: ImageId,
    pub sections: Vec<SectionReport>,
}

/// One validated typed range borrowed directly from the module image.
pub struct BorrowedConstantRange<'a> {
    report: ConstantRangeReport,
    bytes: &'a [u8],
}

impl<'a> BorrowedConstantRange<'a> {
    pub const fn report(&self) -> &ConstantRangeReport {
        &self.report
    }

    pub const fn bytes(&self) -> &'a [u8] {
        self.bytes
    }
}

/// Decoded small module metadata plus large typed ranges borrowed from input bytes.
pub struct BorrowedModule<'a, Intrinsic> {
    manifest: ModuleManifest,
    program: DenseLowered<WeavyOp<BlockRef, Intrinsic>>,
    constants: ConstantPool,
    compact_registry: Registry,
    aligned_registry: AlignedRegistry,
    ranges: Vec<BorrowedConstantRange<'a>>,
}

impl<'a, Intrinsic: weavy::module::IntrinsicContract> BorrowedModule<'a, Intrinsic> {
    pub const fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }
    pub const fn program(&self) -> &DenseLowered<WeavyOp<BlockRef, Intrinsic>> {
        &self.program
    }
    pub const fn constants(&self) -> &ConstantPool {
        &self.constants
    }
    pub fn constant_ranges(&self) -> &[BorrowedConstantRange<'a>] {
        &self.ranges
    }

    pub fn aligned_document(&self, index: usize) -> Result<AlignedDocument<'_>, CodecError> {
        let range = self
            .ranges
            .get(index)
            .ok_or(CodecError::MissingConstantRange {
                id: u32::try_from(index).unwrap_or(u32::MAX),
            })?;
        if range.report.profile != StorageProfile::Aligned {
            return Err(CodecError::WrongStorageProfile);
        }
        AlignedDocument::parse(range.bytes, range.report.schema_id, &self.aligned_registry)
            .map_err(CodecError::Aligned)
    }

    pub fn dense_range(&self, index: usize) -> Result<DenseRange<'a>, CodecError> {
        let range = self
            .ranges
            .get(index)
            .ok_or(CodecError::MissingConstantRange {
                id: u32::try_from(index).unwrap_or(u32::MAX),
            })?;
        if range.report.profile != StorageProfile::DenseAligned {
            return Err(CodecError::WrongStorageProfile);
        }
        DenseRange::parse(range.bytes, range.report.schema_id, &self.aligned_registry)
            .map_err(CodecError::Aligned)
    }

    pub fn compact_value(&self, index: usize) -> Result<Value, CodecError> {
        let range = self
            .ranges
            .get(index)
            .ok_or(CodecError::MissingConstantRange {
                id: u32::try_from(index).unwrap_or(u32::MAX),
            })?;
        if range.report.profile != StorageProfile::Compact {
            return Err(CodecError::WrongStorageProfile);
        }
        compact::from_bytes(range.bytes, range.report.schema_id, &self.compact_registry)
            .map_err(CodecError::Phon)
    }

    pub fn admit(&self, verifier: &ModuleVerifier) -> Result<(), AdmissionError> {
        let ranges = self
            .ranges
            .iter()
            .map(|range| ConstantRangeMetadata {
                schema_id: range.report.schema_id,
                profile: range.report.profile,
            })
            .collect::<Vec<_>>();
        verifier.verify_parts(&self.manifest, &self.program, &self.constants, &ranges)
    }
}

pub fn save<C: IntrinsicCodec>(module: &WeavyModule<C::Intrinsic>) -> Result<Vec<u8>, CodecError> {
    let manifest = encode_manifest(module.manifest());
    let schemas = encode_schema_bundle(module.constant_ranges());
    let program = encode_program::<C>(module.program())?;
    let constants = encode_constants(module.constants())?;
    for range in module.constant_ranges() {
        if range.profile() == StorageProfile::DenseAligned {
            let registry =
                AlignedRegistry::try_new(range.schemas().to_vec()).map_err(CodecError::Phon)?;
            let dense = DenseRange::parse(range.bytes(), range.schema_id(), &registry)
                .map_err(CodecError::Aligned)?;
            if dense.count() != range.count() as usize || dense.stride() != range.stride() as usize
            {
                return Err(CodecError::MalformedConstantRange);
            }
        }
    }

    let mut payloads = vec![
        (
            "manifest".to_owned(),
            SECTION_MANIFEST,
            1u64,
            manifest,
            None,
            0,
            0,
        ),
        (
            "schemas".to_owned(),
            SECTION_SCHEMAS,
            1u64,
            schemas,
            None,
            0,
            0,
        ),
        (
            "program".to_owned(),
            SECTION_PROGRAM,
            PROGRAM_SCHEMA_ID,
            program,
            None,
            0,
            0,
        ),
        (
            "constants".to_owned(),
            SECTION_CONSTANTS,
            CONSTANT_DIRECTORY_SCHEMA_ID,
            constants,
            None,
            0,
            0,
        ),
    ];
    for (index, range) in module.constant_ranges().iter().enumerate() {
        let kind = SECTION_CONSTANT_RANGE_BASE
            .checked_add(u32::try_from(index).map_err(|_| CodecError::SizeOverflow)?)
            .ok_or(CodecError::SizeOverflow)?;
        payloads.push((
            format!("constant_range.{index}"),
            kind,
            range.schema_id().as_u64(),
            range.bytes().to_vec(),
            Some(range.profile()),
            range.count(),
            range.stride(),
        ));
    }
    let directory_placeholder = encode_directory(&[])?;
    let mut cursor = align_up(
        HEADER_SIZE + directory_placeholder.len(),
        DIRECTORY_ALIGNMENT as usize,
    )?;
    let mut sections = Vec::with_capacity(payloads.len());
    for (name, kind, schema_id, payload, profile, count, stride) in &payloads {
        let payload_alignment = if matches!(
            profile,
            Some(StorageProfile::Aligned | StorageProfile::DenseAligned)
        ) {
            64
        } else {
            8
        };
        cursor = align_up(cursor, payload_alignment)?;
        sections.push(SectionReport {
            name: name.clone(),
            kind: *kind,
            offset: cursor as u64,
            encoded_len: payload.len() as u64,
            decoded_len: payload.len() as u64,
            alignment: payload_alignment as u32,
            schema_id: *schema_id,
            flags: FLAG_REQUIRED,
            profile: *profile,
            count: *count,
            stride: *stride,
        });
        cursor = cursor
            .checked_add(payload.len())
            .ok_or(CodecError::SizeOverflow)?;
    }
    let mut directory = encode_directory(&sections)?;
    loop {
        let mut next = align_up(HEADER_SIZE + directory.len(), DIRECTORY_ALIGNMENT as usize)?;
        for (section, (_, _, _, payload, profile, _, _)) in sections.iter_mut().zip(&payloads) {
            let alignment = if matches!(
                profile,
                Some(StorageProfile::Aligned | StorageProfile::DenseAligned)
            ) {
                64
            } else {
                8
            };
            next = align_up(next, alignment)?;
            section.offset = next as u64;
            next = next
                .checked_add(payload.len())
                .ok_or(CodecError::SizeOverflow)?;
        }
        let updated = encode_directory(&sections)?;
        if updated.len() == directory.len() {
            directory = updated;
            break;
        }
        directory = updated;
    }
    let file_len = sections.iter().zip(payloads.iter()).try_fold(
        0usize,
        |_, (section, (_, _, _, payload, _, _, _))| {
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
    for (section, (_, _, _, payload, _, _, _)) in sections.iter().zip(payloads) {
        let offset = section.offset as usize;
        bytes[offset..offset + payload.len()].copy_from_slice(&payload);
    }
    let payload_integrity_tag = payload_integrity_tag(&bytes[HEADER_SIZE..]);
    bytes[48..64].copy_from_slice(payload_integrity_tag.as_bytes());
    Ok(bytes)
}

/// Load an owned module with caller-selected phase-0 container ceilings.
pub fn load<C: IntrinsicCodec>(
    bytes: &[u8],
    limits: ContainerLimits,
) -> Result<WeavyModule<C::Intrinsic>, CodecError> {
    let mut budgets = ByteBudgets::new(limits);
    let parsed = parse_container(bytes, limits, &mut budgets)?;
    let schemas = decode_schema_bundle(parsed.section(SECTION_SCHEMAS)?, limits, &mut budgets)?;
    let compact_registry = Registry::try_new(schemas.clone()).map_err(CodecError::Phon)?;
    let aligned_registry = AlignedRegistry::try_new(schemas).map_err(CodecError::Phon)?;
    validate_constant_range_sections(&parsed, &compact_registry, &aligned_registry, &mut budgets)?;
    let manifest = decode_manifest(parsed.section(SECTION_MANIFEST)?, limits, &mut budgets)?;
    let program = decode_program::<C>(parsed.section(SECTION_PROGRAM)?, limits, &mut budgets)?;
    let constants = decode_constants(parsed.section(SECTION_CONSTANTS)?, limits, &mut budgets)?;
    let ranges = decode_constant_ranges(&parsed, &compact_registry, &mut budgets)?;
    Ok(WeavyModule::new(manifest, program, constants).with_constant_ranges(ranges))
}

/// Load a borrowing module with caller-selected phase-0 container ceilings.
pub fn load_borrowed<C: IntrinsicCodec>(
    bytes: &[u8],
    limits: ContainerLimits,
) -> Result<BorrowedModule<'_, C::Intrinsic>, CodecError> {
    let mut budgets = ByteBudgets::new(limits);
    let parsed = parse_container(bytes, limits, &mut budgets)?;
    let schemas = decode_schema_bundle(parsed.section(SECTION_SCHEMAS)?, limits, &mut budgets)?;
    let compact_registry = Registry::try_new(schemas.clone()).map_err(CodecError::Phon)?;
    let aligned_registry = AlignedRegistry::try_new(schemas).map_err(CodecError::Phon)?;
    validate_constant_range_sections(&parsed, &compact_registry, &aligned_registry, &mut budgets)?;
    let manifest = decode_manifest(parsed.section(SECTION_MANIFEST)?, limits, &mut budgets)?;
    let program = decode_program::<C>(parsed.section(SECTION_PROGRAM)?, limits, &mut budgets)?;
    let constants = decode_constants(parsed.section(SECTION_CONSTANTS)?, limits, &mut budgets)?;
    let range_count = parsed
        .sections
        .iter()
        .filter(|section| section.kind >= SECTION_CONSTANT_RANGE_BASE)
        .count();
    let mut ranges = budgets.reserve::<BorrowedConstantRange<'_>>(range_count)?;
    for section in parsed
        .sections
        .iter()
        .filter(|section| section.kind >= SECTION_CONSTANT_RANGE_BASE)
    {
        ranges.push(BorrowedConstantRange {
            report: ConstantRangeReport {
                id: section.kind - SECTION_CONSTANT_RANGE_BASE,
                schema_id: SchemaId::from_raw(section.schema_id),
                profile: section.profile.ok_or(CodecError::MalformedDirectory)?,
                count: section.count,
                stride: section.stride,
                encoded_len: section.encoded_len,
            },
            bytes: parsed.section(section.kind)?,
        });
    }
    Ok(BorrowedModule {
        manifest,
        program,
        constants,
        compact_registry,
        aligned_registry,
        ranges,
    })
}

/// Inspect decoded module facts with caller-selected phase-0 container ceilings.
///
/// This report is descriptive only; it does not confer admission authority.
pub fn inspect(bytes: &[u8], limits: ContainerLimits) -> Result<InspectionReport, CodecError> {
    let mut budgets = ByteBudgets::new(limits);
    let parsed = parse_container(bytes, limits, &mut budgets)?;
    let schemas = decode_schema_bundle(parsed.section(SECTION_SCHEMAS)?, limits, &mut budgets)?;
    let compact_registry = Registry::try_new(schemas.clone()).map_err(CodecError::Phon)?;
    let aligned_registry = AlignedRegistry::try_new(schemas).map_err(CodecError::Phon)?;
    validate_constant_range_sections(&parsed, &compact_registry, &aligned_registry, &mut budgets)?;
    let manifest = decode_manifest(parsed.section(SECTION_MANIFEST)?, limits, &mut budgets)?;
    let (program_op_count, block_count) =
        inspect_program(parsed.section(SECTION_PROGRAM)?, limits, &mut budgets)?;
    let constant_count =
        inspect_constants(parsed.section(SECTION_CONSTANTS)?, limits, &mut budgets)?;
    let range_count = parsed
        .sections
        .iter()
        .filter(|section| section.kind >= SECTION_CONSTANT_RANGE_BASE)
        .count();
    let mut constant_ranges = budgets.reserve::<ConstantRangeReport>(range_count)?;
    constant_ranges.extend(
        parsed
            .sections
            .iter()
            .filter(|section| section.kind >= SECTION_CONSTANT_RANGE_BASE)
            .map(|section| ConstantRangeReport {
                id: section.kind - SECTION_CONSTANT_RANGE_BASE,
                schema_id: SchemaId::from_raw(section.schema_id),
                profile: section.profile.expect("range profile validated"),
                count: section.count,
                stride: section.stride,
                encoded_len: section.encoded_len,
            }),
    );
    let module_name = budgets.copy_string(manifest.name())?;
    let mut dialects = budgets.reserve::<DialectRequirement>(manifest.dialects().len())?;
    dialects.extend_from_slice(manifest.dialects());
    Ok(InspectionReport {
        module_name,
        payload_integrity_tag: parsed.payload_integrity_tag,
        image_id: parsed.image_id,
        dialects,
        sections: parsed.sections,
        program_op_count,
        block_count,
        constant_count,
        constant_ranges,
    })
}

/// Inspect physical image structure with caller-selected phase-0 container ceilings.
///
/// Structural inspection verifies framing and payload integrity but deliberately does not
/// decode semantic section payloads. Its result carries no semantic or admission authority.
pub fn inspect_structure(
    bytes: &[u8],
    limits: ContainerLimits,
) -> Result<StructuralInspection, CodecError> {
    let mut budgets = ByteBudgets::new(limits);
    let parsed = parse_container(bytes, limits, &mut budgets)?;
    Ok(StructuralInspection {
        payload_integrity_tag: parsed.payload_integrity_tag,
        image_id: parsed.image_id,
        sections: parsed.sections,
    })
}

struct Parsed<'a> {
    bytes: &'a [u8],
    sections: Vec<SectionReport>,
    payload_integrity_tag: PayloadIntegrityTag,
    image_id: ImageId,
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

fn parse_container<'a>(
    bytes: &'a [u8],
    limits: ContainerLimits,
    budgets: &mut ByteBudgets,
) -> Result<Parsed<'a>, CodecError> {
    enforce_limit(
        ContainerLimitKind::ImageBytes,
        limits.max_image_bytes,
        bytes.len(),
    )?;
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
    if major != FORMAT_MAJOR || minor != FORMAT_MINOR {
        return Err(CodecError::UnsupportedFormat { major, minor });
    }
    if bytes[12] != 1 || bytes[13..16] != [0; 3] {
        return Err(CodecError::MalformedHeader);
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
    let directory_kind = read_u32(bytes, 44)?;
    if directory_offset != HEADER_SIZE
        || alignment != DIRECTORY_ALIGNMENT
        || directory_kind != DIRECTORY_SECTION_KIND
    {
        return Err(CodecError::MalformedHeader);
    }
    enforce_limit(
        ContainerLimitKind::DirectoryBytes,
        limits.max_directory_bytes,
        directory_len,
    )?;
    let directory_end =
        directory_offset
            .checked_add(directory_len)
            .ok_or(CodecError::SectionOutOfBounds {
                offset: directory_offset as u64,
                len: directory_len as u64,
                file_len: bytes.len(),
            })?;
    if directory_end > bytes.len() {
        return Err(CodecError::SectionOutOfBounds {
            offset: directory_offset as u64,
            len: directory_len as u64,
            file_len: bytes.len(),
        });
    }
    let expected =
        PayloadIntegrityTag::from_bytes(bytes[48..64].try_into().expect("header length"));
    let actual = payload_integrity_tag(&bytes[HEADER_SIZE..]);
    if expected != actual {
        return Err(CodecError::IntegrityMismatch { expected, actual });
    }
    let image_id = image_id(bytes);
    let directory_bytes = &bytes[directory_offset..directory_end];
    let sections = decode_directory(directory_bytes, limits, budgets)?;
    if encode_directory(&sections)? != directory_bytes {
        return Err(CodecError::MalformedDirectory);
    }
    validate_directory_entries(bytes, directory_end, &sections)?;
    Ok(Parsed {
        bytes,
        sections,
        payload_integrity_tag: expected,
        image_id,
    })
}

fn validate_directory_entries(
    bytes: &[u8],
    directory_end: usize,
    sections: &[SectionReport],
) -> Result<(), CodecError> {
    let mut singleton_counts = [0u8; 4];
    let mut expected_range_kind = SECTION_CONSTANT_RANGE_BASE;
    let mut previous_end = directory_end;

    for (index, section) in sections.iter().enumerate() {
        if sections[..index]
            .iter()
            .any(|candidate| candidate.name == section.name)
        {
            return Err(CodecError::MalformedDirectory);
        }
        if section.flags & !FLAG_REQUIRED != 0 {
            return Err(CodecError::MalformedDirectory);
        }
        let start = usize_from_u64(section.offset)?;
        let len = usize_from_u64(section.encoded_len)?;
        let end = start.checked_add(len).ok_or(CodecError::SizeOverflow)?;
        if section.alignment == 0
            || !section.alignment.is_power_of_two()
            || !start.is_multiple_of(section.alignment as usize)
            || start < directory_end
            || end > bytes.len()
        {
            return Err(CodecError::SectionOutOfBounds {
                offset: section.offset,
                len: section.encoded_len,
                file_len: bytes.len(),
            });
        }
        let expected_start = align_up(previous_end, section.alignment as usize)?;
        if start != expected_start || bytes[previous_end..start].iter().any(|byte| *byte != 0) {
            return Err(CodecError::MalformedDirectory);
        }
        previous_end = end;

        match section.kind {
            0 | DIRECTORY_SECTION_KIND => return Err(CodecError::MalformedDirectory),
            SECTION_MANIFEST => {
                singleton_counts[0] = singleton_counts[0]
                    .checked_add(1)
                    .ok_or(CodecError::MalformedDirectory)?;
                validate_legacy_section(section, "manifest", 1)?;
            }
            SECTION_SCHEMAS => {
                singleton_counts[1] = singleton_counts[1]
                    .checked_add(1)
                    .ok_or(CodecError::MalformedDirectory)?;
                validate_legacy_section(section, "schemas", 1)?;
            }
            SECTION_PROGRAM => {
                singleton_counts[2] = singleton_counts[2]
                    .checked_add(1)
                    .ok_or(CodecError::MalformedDirectory)?;
                validate_legacy_section(section, "program", PROGRAM_SCHEMA_ID)?;
            }
            SECTION_CONSTANTS => {
                singleton_counts[3] = singleton_counts[3]
                    .checked_add(1)
                    .ok_or(CodecError::MalformedDirectory)?;
                validate_legacy_section(section, "constants", CONSTANT_DIRECTORY_SCHEMA_ID)?;
            }
            SECTION_CONSTANT_RANGE_BASE.. => {
                if section.kind != expected_range_kind {
                    return Err(CodecError::MalformedDirectory);
                }
                expected_range_kind = expected_range_kind
                    .checked_add(1)
                    .ok_or(CodecError::SizeOverflow)?;
                validate_range_section(section)?;
            }
            _ if section.flags & FLAG_REQUIRED != 0 => {
                return Err(CodecError::UnknownRequiredSection { kind: section.kind });
            }
            _ => {}
        }
    }

    if bytes[previous_end..].iter().any(|byte| *byte != 0) {
        return Err(CodecError::MalformedDirectory);
    }
    if singleton_counts != [1; 4] {
        return Err(CodecError::MalformedDirectory);
    }
    Ok(())
}

fn validate_legacy_section(
    section: &SectionReport,
    name: &str,
    schema_id: u64,
) -> Result<(), CodecError> {
    if section.name != name
        || section.schema_id != schema_id
        || section.flags != FLAG_REQUIRED
        || section.profile.is_some()
        || section.count != 0
        || section.stride != 0
        || section.alignment != 8
        || section.decoded_len != section.encoded_len
    {
        return Err(CodecError::MalformedDirectory);
    }
    Ok(())
}

fn validate_range_section(section: &SectionReport) -> Result<(), CodecError> {
    let range_id = section.kind - SECTION_CONSTANT_RANGE_BASE;
    if section.name != format!("constant_range.{range_id}")
        || section.flags != FLAG_REQUIRED
        || section.stride == 0
        || section.decoded_len != section.encoded_len
    {
        return Err(CodecError::MalformedDirectory);
    }
    let expected_alignment = match section.profile {
        Some(StorageProfile::Compact) => 8,
        Some(StorageProfile::Aligned | StorageProfile::DenseAligned) => 64,
        None => return Err(CodecError::MalformedDirectory),
    };
    if section.alignment != expected_alignment {
        return Err(CodecError::InvalidAlignment {
            alignment: section.alignment,
        });
    }
    Ok(())
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
                field("profile", Primitive::U8),
                field("count", Primitive::U32),
                field("stride", Primitive::U32),
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
    assert_eq!(schemas[0].id.as_u64(), DIRECTORY_SECTION_SCHEMA_ID);
    assert_eq!(schemas[1].id.as_u64(), DIRECTORY_SCHEMA_ID);
    (schemas, SchemaId::from_raw(DIRECTORY_SCHEMA_ID))
}
fn field(name: &str, primitive: Primitive) -> Field {
    Field {
        name: name.into(),
        schema: SchemaRef::concrete(primitive_id(primitive)),
        required: true,
    }
}

fn encode_directory(sections: &[SectionReport]) -> Result<Vec<u8>, CodecError> {
    let minimum = 4usize
        .checked_add(
            sections
                .len()
                .checked_mul(MIN_DIRECTORY_ENTRY_BYTES)
                .ok_or(CodecError::SizeOverflow)?,
        )
        .ok_or(CodecError::SizeOverflow)?;
    let names = sections.iter().try_fold(0usize, |total, section| {
        total
            .checked_add(section.name.len())
            .ok_or(CodecError::SizeOverflow)
    })?;
    let mut out = Vec::new();
    out.try_reserve(
        minimum
            .checked_add(names)
            .and_then(|bytes| bytes.checked_add(sections.len() * 32))
            .ok_or(CodecError::SizeOverflow)?,
    )
    .map_err(|_| CodecError::AdmissionLimitExceeded)?;
    put_u32(
        &mut out,
        u32::try_from(sections.len()).map_err(|_| CodecError::SizeOverflow)?,
    );
    for section in sections {
        put_directory_string(&mut out, &section.name)?;
        put_directory_u32(&mut out, section.kind);
        put_directory_u64(&mut out, section.offset);
        put_directory_u64(&mut out, section.encoded_len);
        put_directory_u64(&mut out, section.decoded_len);
        put_directory_u32(&mut out, section.alignment);
        put_directory_u64(&mut out, section.schema_id);
        put_directory_u32(&mut out, section.flags);
        out.push(match section.profile {
            None => 0,
            Some(StorageProfile::Compact) => 1,
            Some(StorageProfile::Aligned) => 2,
            Some(StorageProfile::DenseAligned) => 3,
        });
        put_directory_u32(&mut out, section.count);
        put_directory_u32(&mut out, section.stride);
    }
    Ok(out)
}

fn pad_directory(out: &mut Vec<u8>, alignment: usize) {
    while !out.len().is_multiple_of(alignment) {
        out.push(0);
    }
}

fn put_directory_u32(out: &mut Vec<u8>, value: u32) {
    pad_directory(out, 4);
    out.extend_from_slice(&value.to_le_bytes());
}

fn put_directory_u64(out: &mut Vec<u8>, value: u64) {
    pad_directory(out, 8);
    out.extend_from_slice(&value.to_le_bytes());
}

fn put_directory_string(out: &mut Vec<u8>, value: &str) -> Result<(), CodecError> {
    put_directory_u32(
        out,
        u32::try_from(value.len()).map_err(|_| CodecError::SizeOverflow)?,
    );
    out.extend_from_slice(value.as_bytes());
    Ok(())
}

fn decode_directory(
    bytes: &[u8],
    limits: ContainerLimits,
    budgets: &mut ByteBudgets,
) -> Result<Vec<SectionReport>, CodecError> {
    budgets.decoded(bytes.len())?;
    let mut reader = DirectoryReader::new(bytes);
    let encoded_count = reader.u32()? as usize;
    let byte_derived_max = bytes.len().saturating_sub(4) / MIN_DIRECTORY_ENTRY_BYTES;
    enforce_limit(
        ContainerLimitKind::Sections,
        limits.max_sections,
        encoded_count,
    )?;
    if encoded_count > byte_derived_max {
        return Err(CodecError::AdmissionLimitExceeded);
    }
    let mut sections = budgets.reserve::<SectionReport>(encoded_count)?;
    for _ in 0..encoded_count {
        let name = reader.string(budgets)?;
        let kind = reader.u32()?;
        let offset = reader.u64()?;
        let encoded_len = reader.u64()?;
        let decoded_len = reader.u64()?;
        let alignment = reader.u32()?;
        let schema_id = reader.u64()?;
        let flags = reader.u32()?;
        let profile = match reader.u8()? {
            0 => None,
            1 => Some(StorageProfile::Compact),
            2 => Some(StorageProfile::Aligned),
            3 => Some(StorageProfile::DenseAligned),
            _ => return Err(CodecError::MalformedDirectory),
        };
        let count = reader.u32()?;
        let stride = reader.u32()?;
        sections.push(SectionReport {
            name,
            kind,
            offset,
            encoded_len,
            decoded_len,
            alignment,
            schema_id,
            flags,
            profile,
            count,
            stride,
        });
    }
    reader.finish()?;
    Ok(sections)
}

struct DirectoryReader<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

impl<'a> DirectoryReader<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, cursor: 0 }
    }

    fn align(&mut self, alignment: usize) -> Result<(), CodecError> {
        let aligned = align_up(self.cursor, alignment)?;
        let padding = self
            .bytes
            .get(self.cursor..aligned)
            .ok_or(CodecError::MalformedDirectory)?;
        if padding.iter().any(|byte| *byte != 0) {
            return Err(CodecError::MalformedDirectory);
        }
        self.cursor = aligned;
        Ok(())
    }

    fn take(&mut self, len: usize) -> Result<&'a [u8], CodecError> {
        let end = self
            .cursor
            .checked_add(len)
            .ok_or(CodecError::SizeOverflow)?;
        let value = self
            .bytes
            .get(self.cursor..end)
            .ok_or(CodecError::MalformedDirectory)?;
        self.cursor = end;
        Ok(value)
    }

    fn u8(&mut self) -> Result<u8, CodecError> {
        Ok(self.take(1)?[0])
    }

    fn u32(&mut self) -> Result<u32, CodecError> {
        self.align(4)?;
        Ok(u32::from_le_bytes(
            self.take(4)?.try_into().expect("length"),
        ))
    }

    fn u64(&mut self) -> Result<u64, CodecError> {
        self.align(8)?;
        Ok(u64::from_le_bytes(
            self.take(8)?.try_into().expect("length"),
        ))
    }

    fn string(&mut self, budgets: &mut ByteBudgets) -> Result<String, CodecError> {
        let len = self.u32()? as usize;
        let bytes = self.take(len)?;
        let value = core::str::from_utf8(bytes).map_err(|_| CodecError::InvalidUtf8)?;
        budgets.copy_string(value)
    }

    fn finish(self) -> Result<(), CodecError> {
        if self.cursor == self.bytes.len() {
            Ok(())
        } else {
            Err(CodecError::MalformedDirectory)
        }
    }
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
fn decode_manifest(
    bytes: &[u8],
    limits: ContainerLimits,
    budgets: &mut ByteBudgets,
) -> Result<ModuleManifest, CodecError> {
    budgets.decoded(bytes.len())?;
    let mut r = Reader::new(bytes);
    let name = r.string_with_budget(budgets)?;
    let major = r.u16()?;
    let minor = r.u16()?;
    if major != 1 || minor != 0 {
        return Err(CodecError::UnsupportedFormat { major, minor });
    }
    let dialect_count = r.bounded_count(8)?;
    enforce_limit(
        ContainerLimitKind::ManifestDialects,
        limits.max_manifest_dialects,
        dialect_count,
    )?;
    let mut dialects = budgets.reserve::<DialectRequirement>(dialect_count)?;
    for _ in 0..dialect_count {
        dialects.push(DialectRequirement::new(
            r.string_with_budget(budgets)?,
            r.u16()?,
            r.u16()?,
        ));
    }
    let root_count = r.bounded_count(4)?;
    enforce_limit(
        ContainerLimitKind::ManifestRoots,
        limits.max_manifest_roots,
        root_count,
    )?;
    let mut roots = budgets.reserve::<u32>(root_count)?;
    for _ in 0..root_count {
        roots.push(r.u32()?);
    }
    r.finish()?;
    Ok(ModuleManifest::new(name, dialects, roots))
}

fn encode_schema_bundle(ranges: &[ConstantRange]) -> Vec<u8> {
    let mut schemas = directory_schemas().0;
    for range in ranges {
        for schema in range.schemas() {
            if !schemas.iter().any(|candidate| candidate.id == schema.id) {
                schemas.push(schema.clone());
            }
        }
    }
    let mut out = Vec::new();
    put_u32(&mut out, schemas.len() as u32);
    for schema in &schemas {
        put_bytes(&mut out, &schema_to_bytes(schema));
    }
    out
}

fn decode_schema_bundle(
    bytes: &[u8],
    limits: ContainerLimits,
    budgets: &mut ByteBudgets,
) -> Result<Vec<Schema>, CodecError> {
    budgets.decoded(bytes.len())?;
    let mut r = Reader::new(bytes);
    let count = r.bounded_count(4)?;
    enforce_limit(ContainerLimitKind::Schemas, limits.max_schemas, count)?;
    let mut schemas = budgets.reserve::<Schema>(count)?;
    let mut schema_ids = Vec::new();
    schema_ids
        .try_reserve_exact(count)
        .map_err(|_| CodecError::AdmissionLimitExceeded)?;
    for _ in 0..count {
        let encoded = r.bytes()?;
        let remaining = limits.max_retained_bytes.saturating_sub(budgets.retained);
        let (schema, owned_bytes) = schema_from_bytes(
            encoded,
            phon_schema::DecodeLimits::DEFAULT.with_max_owned_bytes(remaining),
        )
        .map_err(map_schema_decode_error)?;
        if !schema_matches_bytes(&schema, encoded) {
            return Err(CodecError::MalformedSchemas);
        }
        budgets.retained(owned_bytes)?;
        schema_ids.push(schema.id);
        schemas.push(schema);
    }
    r.finish()?;
    schema_ids.sort_unstable();
    if schema_ids.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(CodecError::MalformedSchemas);
    }
    Ok(schemas)
}

fn map_schema_decode_error(error: phon_schema::DecodeError) -> CodecError {
    match error {
        phon_schema::DecodeError::OwnedBytesLimitExceeded {
            configured,
            attempted,
        } => CodecError::ContainerLimitExceeded {
            kind: ContainerLimitKind::RetainedBytes,
            configured,
            actual: attempted,
        },
        phon_schema::DecodeError::AllocationFailed => CodecError::AdmissionLimitExceeded,
        other => CodecError::SchemaDecode(other),
    }
}

fn validate_constant_range_sections(
    parsed: &Parsed<'_>,
    compact_registry: &Registry,
    aligned_registry: &AlignedRegistry,
    budgets: &mut ByteBudgets,
) -> Result<(), CodecError> {
    for section in parsed
        .sections
        .iter()
        .filter(|section| section.kind >= SECTION_CONSTANT_RANGE_BASE)
    {
        let profile = section.profile.ok_or(CodecError::MalformedDirectory)?;
        if section.stride == 0 {
            return Err(CodecError::MalformedConstantRange);
        }
        let schema = SchemaId::from_raw(section.schema_id);
        let bytes = parsed.section(section.kind)?;
        budgets.decoded(bytes.len())?;
        match profile {
            StorageProfile::Aligned => {
                let document = AlignedDocument::parse(bytes, schema, aligned_registry)
                    .map_err(CodecError::Aligned)?;
                let count = document.root().len().map_err(CodecError::Aligned)?;
                if count != section.count as usize {
                    return Err(CodecError::MalformedConstantRange);
                }
            }
            StorageProfile::DenseAligned => {
                let range = DenseRange::parse(bytes, schema, aligned_registry)
                    .map_err(CodecError::Aligned)?;
                if range.count() != section.count as usize
                    || range.stride() != section.stride as usize
                {
                    return Err(CodecError::MalformedConstantRange);
                }
            }
            StorageProfile::Compact => {
                compact::from_bytes(bytes, schema, compact_registry).map_err(CodecError::Phon)?;
            }
        }
    }
    Ok(())
}

fn decode_constant_ranges(
    parsed: &Parsed<'_>,
    registry: &Registry,
    budgets: &mut ByteBudgets,
) -> Result<Vec<ConstantRange>, CodecError> {
    let range_count = parsed
        .sections
        .iter()
        .filter(|section| section.kind >= SECTION_CONSTANT_RANGE_BASE)
        .count();
    let mut ranges = budgets.reserve::<ConstantRange>(range_count)?;
    for section in parsed
        .sections
        .iter()
        .filter(|section| section.kind >= SECTION_CONSTANT_RANGE_BASE)
    {
        let root = SchemaId::from_raw(section.schema_id);
        let mut schemas = Vec::new();
        collect_schema_closure(root, registry, &mut schemas, budgets)?;
        let root_index = schemas
            .iter()
            .position(|schema| schema.id == root)
            .ok_or(CodecError::MalformedSchemas)?;
        ranges.push(
            ConstantRange::new(
                schemas,
                root_index,
                section.profile.ok_or(CodecError::MalformedDirectory)?,
                section.count,
                section.stride,
                budgets.copy_bytes(parsed.section(section.kind)?)?,
            )
            .map_err(CodecError::ConstantRange)?,
        );
    }
    Ok(ranges)
}
fn collect_schema_closure(
    id: SchemaId,
    registry: &Registry,
    out: &mut Vec<Schema>,
    budgets: &mut ByteBudgets,
) -> Result<(), CodecError> {
    if out.iter().any(|schema| schema.id == id) || registry.primitive(id).is_some() {
        return Ok(());
    }
    let schema = registry
        .composite(id)
        .ok_or(CodecError::MalformedSchemas)?
        .clone();
    let mut references = budgets.reserve::<SchemaId>(schema_reference_count(&schema.kind))?;
    visit_schema_refs(&schema.kind, &mut |reference| references.push(reference));
    for reference in references {
        collect_schema_closure(reference, registry, out, budgets)?;
    }
    budgets.retained(core::mem::size_of::<Schema>())?;
    out.try_reserve(1)
        .map_err(|_| CodecError::AdmissionLimitExceeded)?;
    out.push(schema);
    Ok(())
}

fn schema_reference_count(kind: &SchemaKind) -> usize {
    let mut count = 0;
    visit_schema_refs(kind, &mut |_| count += 1);
    count
}

fn visit_schema_refs(kind: &SchemaKind, visit: &mut dyn FnMut(SchemaId)) {
    match kind {
        SchemaKind::Struct { fields, .. } => {
            fields
                .iter()
                .for_each(|field| visit_ref(&field.schema, visit));
        }
        SchemaKind::Tuple { elements } => {
            elements.iter().for_each(|schema| visit_ref(schema, visit));
        }
        SchemaKind::List { element }
        | SchemaKind::Set { element }
        | SchemaKind::Option { element }
        | SchemaKind::Array { element, .. } => visit_ref(element, visit),
        _ => {}
    }
}

fn visit_ref(reference: &SchemaRef, visit: &mut dyn FnMut(SchemaId)) {
    if let SchemaRef::Concrete { id, args } = reference {
        visit(*id);
        args.iter().for_each(|argument| visit_ref(argument, visit));
    }
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
    limits: ContainerLimits,
    budgets: &mut ByteBudgets,
) -> Result<DenseLowered<WeavyOp<BlockRef, C::Intrinsic>>, CodecError> {
    budgets.decoded(bytes.len())?;
    let mut r = Reader::new(bytes);
    let mut total_ops = 0;
    let program = decode_ops::<C>(&mut r, limits, &mut total_ops, budgets)?;
    let count = r.bounded_count(4)?;
    enforce_limit(
        ContainerLimitKind::ProgramBlocks,
        limits.max_program_blocks,
        count,
    )?;
    let mut blocks = budgets.reserve::<Vec<WeavyOp<BlockRef, C::Intrinsic>>>(count)?;
    for _ in 0..count {
        blocks.push(decode_ops::<C>(&mut r, limits, &mut total_ops, budgets)?);
    }
    r.finish()?;
    Ok(DenseLowered::new(program, blocks))
}
fn decode_ops<C: IntrinsicCodec>(
    r: &mut Reader<'_>,
    limits: ContainerLimits,
    total_ops: &mut usize,
    budgets: &mut ByteBudgets,
) -> Result<Vec<WeavyOp<BlockRef, C::Intrinsic>>, CodecError> {
    let count = r.bounded_count(1)?;
    *total_ops = total_ops
        .checked_add(count)
        .ok_or(CodecError::SizeOverflow)?;
    enforce_limit(
        ContainerLimitKind::ProgramOps,
        limits.max_program_ops,
        *total_ops,
    )?;
    let mut ops = budgets.reserve::<WeavyOp<BlockRef, C::Intrinsic>>(count)?;
    for _ in 0..count {
        ops.push(decode_op::<C>(r, limits, budgets)?);
    }
    Ok(ops)
}
fn decode_op<C: IntrinsicCodec>(
    r: &mut Reader<'_>,
    limits: ContainerLimits,
    budgets: &mut ByteBudgets,
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
            let n = r.bounded_count(24)?;
            enforce_limit(
                ContainerLimitKind::ScalarSegments,
                limits.max_scalar_segments,
                n,
            )?;
            let mut segments = budgets.reserve::<ScalarSegment>(n)?;
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

fn inspect_program(
    bytes: &[u8],
    limits: ContainerLimits,
    budgets: &mut ByteBudgets,
) -> Result<(usize, usize), CodecError> {
    budgets.decoded(bytes.len())?;
    let mut r = Reader::new(bytes);
    let roots = skip_ops(&mut r, limits)?;
    let blocks = r.bounded_count(4)?;
    enforce_limit(
        ContainerLimitKind::ProgramBlocks,
        limits.max_program_blocks,
        blocks,
    )?;
    let mut total = roots;
    for _ in 0..blocks {
        total = total
            .checked_add(skip_ops(&mut r, limits)?)
            .ok_or(CodecError::SizeOverflow)?;
        enforce_limit(
            ContainerLimitKind::ProgramOps,
            limits.max_program_ops,
            total,
        )?;
    }
    r.finish()?;
    Ok((total, blocks))
}
fn skip_ops(r: &mut Reader<'_>, limits: ContainerLimits) -> Result<usize, CodecError> {
    let count = r.bounded_count(1)?;
    enforce_limit(
        ContainerLimitKind::ProgramOps,
        limits.max_program_ops,
        count,
    )?;
    for _ in 0..count {
        skip_op(r, limits)?;
    }
    Ok(count)
}
fn skip_op(r: &mut Reader<'_>, limits: ContainerLimits) -> Result<(), CodecError> {
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
            let n = r.bounded_count(24)?;
            enforce_limit(
                ContainerLimitKind::ScalarSegments,
                limits.max_scalar_segments,
                n,
            )?;
            for _ in 0..n {
                r.u64()?;
                r.u64()?;
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
fn decode_constants(
    bytes: &[u8],
    limits: ContainerLimits,
    budgets: &mut ByteBudgets,
) -> Result<ConstantPool, CodecError> {
    budgets.decoded(bytes.len())?;
    let mut r = Reader::new(bytes);
    let count = r.bounded_count(12)?;
    enforce_limit(ContainerLimitKind::Constants, limits.max_constants, count)?;
    let mut constants = budgets.reserve::<Constant>(count)?;
    for _ in 0..count {
        let schema_id = r.u64()?;
        let bytes = budgets.copy_bytes(r.bytes()?)?;
        constants.push(Constant::new(schema_id, bytes));
    }
    r.finish()?;
    Ok(ConstantPool::new(constants))
}

fn inspect_constants(
    bytes: &[u8],
    limits: ContainerLimits,
    budgets: &mut ByteBudgets,
) -> Result<usize, CodecError> {
    budgets.decoded(bytes.len())?;
    let mut r = Reader::new(bytes);
    let count = r.bounded_count(12)?;
    enforce_limit(ContainerLimitKind::Constants, limits.max_constants, count)?;
    for _ in 0..count {
        r.u64()?;
        r.bytes()?;
    }
    r.finish()?;
    Ok(count)
}

fn payload_integrity_tag(bytes: &[u8]) -> PayloadIntegrityTag {
    PayloadIntegrityTag::from_bytes(
        blake3::hash(bytes).as_bytes()[..16]
            .try_into()
            .expect("hash length"),
    )
}

fn image_id(bytes: &[u8]) -> ImageId {
    ImageId::from_bytes(*blake3::hash(bytes).as_bytes())
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
    fn bounded_count(&mut self, minimum_bytes_per_item: usize) -> Result<usize, CodecError> {
        let count = self.u32()? as usize;
        let max = self
            .bytes
            .len()
            .saturating_sub(self.pos)
            .checked_div(minimum_bytes_per_item)
            .ok_or(CodecError::SizeOverflow)?;
        if count > max {
            return Err(CodecError::AdmissionLimitExceeded);
        }
        Ok(count)
    }
    fn string_with_budget(&mut self, budgets: &mut ByteBudgets) -> Result<String, CodecError> {
        let bytes = self.bytes()?;
        let value = core::str::from_utf8(bytes).map_err(|_| CodecError::InvalidUtf8)?;
        budgets.copy_string(value)
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
    MalformedHeader,
    AdmissionLimitExceeded,
    ContainerLimitExceeded {
        kind: ContainerLimitKind,
        configured: usize,
        actual: usize,
    },
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
        expected: PayloadIntegrityTag,
        actual: PayloadIntegrityTag,
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
    SchemaDecode(phon_schema::DecodeError),
    Phon(compact::CompactError),
    MissingConstantRange {
        id: u32,
    },
    WrongStorageProfile,
    MalformedConstantRange,
    Aligned(phon_storage::AlignedError),
    ConstantRange(ConstantRangeError),
}
impl fmt::Display for CodecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Weavy PHON codec error: {self:?}")
    }
}
impl std::error::Error for CodecError {}

#[cfg(test)]
mod range_validation_tests {
    use super::*;
    use facet_value::{VArray, VObject, VString};

    #[test]
    fn aligned_range_count_must_match_directory() {
        let row = Schema {
            id: SchemaId::from_raw(1),
            type_params: Vec::new(),
            kind: SchemaKind::Struct {
                name: "CountRow".into(),
                fields: vec![field("value", Primitive::U32)],
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
        let compact_registry = Registry::try_new(schemas.clone()).expect("compact registry");
        let aligned_registry = AlignedRegistry::try_new(schemas).expect("aligned registry");
        let mut rows = VArray::new();
        for value in [1u32, 2, 3] {
            let mut object = VObject::new();
            object.insert(VString::new("value"), Value::from(value));
            rows.push(object);
        }
        let bytes =
            phon_storage::AlignedWriter::encode(&Value::from(rows), root, &aligned_registry)
                .expect("aligned rows");
        let parsed = Parsed {
            bytes: &bytes,
            sections: vec![SectionReport {
                name: "constant_range.0".into(),
                kind: SECTION_CONSTANT_RANGE_BASE,
                offset: 0,
                encoded_len: bytes.len() as u64,
                decoded_len: bytes.len() as u64,
                alignment: 64,
                schema_id: root.as_u64(),
                flags: FLAG_REQUIRED,
                profile: Some(StorageProfile::Aligned),
                count: 4,
                stride: 32,
            }],
            payload_integrity_tag: PayloadIntegrityTag::from_bytes([0; 16]),
            image_id: ImageId::from_bytes([0; 32]),
        };
        let mut budgets = ByteBudgets::new(ContainerLimits::DEFAULT);
        assert!(matches!(
            validate_constant_range_sections(
                &parsed,
                &compact_registry,
                &aligned_registry,
                &mut budgets,
            ),
            Err(CodecError::MalformedConstantRange)
        ));
    }

    #[test]
    fn schema_bundle_rejects_duplicate_ids_before_registry_construction() {
        let schema = directory_schemas().0[0].clone();
        let encoded = schema_to_bytes(&schema);
        let mut bytes = Vec::new();
        put_u32(&mut bytes, 2);
        put_bytes(&mut bytes, &encoded);
        put_bytes(&mut bytes, &encoded);
        let mut budgets = ByteBudgets::new(ContainerLimits::DEFAULT);

        assert!(matches!(
            decode_schema_bundle(&bytes, ContainerLimits::DEFAULT, &mut budgets),
            Err(CodecError::MalformedSchemas)
        ));
    }
}
