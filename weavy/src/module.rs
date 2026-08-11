//! Semantic model and admission checks for durable Weavy modules.
//!
//! Physical encodings live in sibling codec crates. This module owns the
//! process-independent address space and the checks required before execution.

use core::fmt;
use std::collections::BTreeMap;

use crate::BlockRef;
use crate::ir::{AggregateOp, ControlOp, DenseWeavyLowered, WeavyOp};
use phon_schema::{Schema, SchemaId};

/// Stable index in a module-local constant address space.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConstantId(u32);

impl ConstantId {
    #[must_use]
    pub const fn new(index: u32) -> Self {
        Self(index)
    }

    #[must_use]
    pub const fn index(self) -> u32 {
        self.0
    }
}

/// Stable index in a module-local typed constant-range address space.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConstantRangeId(u32);

impl ConstantRangeId {
    #[must_use]
    pub const fn new(index: u32) -> Self {
        Self(index)
    }

    #[must_use]
    pub const fn index(self) -> u32 {
        self.0
    }
}

/// PHON physical profile used by one typed constant range.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StorageProfile {
    Compact,
    Aligned,
}

/// One homogeneous typed constant range.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConstantRange {
    schemas: Vec<Schema>,
    schema_id: SchemaId,
    profile: StorageProfile,
    count: u32,
    stride: u32,
    bytes: Vec<u8>,
}

impl ConstantRange {
    pub fn new(
        schemas: Vec<Schema>,
        root_index: usize,
        profile: StorageProfile,
        count: u32,
        stride: u32,
        bytes: Vec<u8>,
    ) -> Result<Self, ConstantRangeError> {
        if root_index >= schemas.len() {
            return Err(ConstantRangeError::InvalidRootIndex {
                index: root_index,
                schema_count: schemas.len(),
            });
        }
        let resolved = phon_schema::resolve_ids(schemas);
        let schema_id = resolved[root_index].id;
        Ok(Self {
            schemas: resolved,
            schema_id,
            profile,
            count,
            stride,
            bytes,
        })
    }

    #[must_use]
    pub fn schemas(&self) -> &[Schema] {
        &self.schemas
    }

    #[must_use]
    pub const fn schema_id(&self) -> SchemaId {
        self.schema_id
    }

    #[must_use]
    pub const fn profile(&self) -> StorageProfile {
        self.profile
    }

    #[must_use]
    pub const fn count(&self) -> u32 {
        self.count
    }

    #[must_use]
    pub const fn stride(&self) -> u32 {
        self.stride
    }

    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConstantRangeError {
    InvalidRootIndex { index: usize, schema_count: usize },
}

impl fmt::Display for ConstantRangeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid typed constant range: {self:?}")
    }
}

impl std::error::Error for ConstantRangeError {}

/// Metadata required to admit a borrowed constant range without owning its bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConstantRangeMetadata {
    pub schema_id: SchemaId,
    pub profile: StorageProfile,
}

/// One typed constant's schema identity and encoded PHON payload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Constant {
    schema_id: u64,
    bytes: Vec<u8>,
}

impl Constant {
    #[must_use]
    pub fn new(schema_id: u64, bytes: Vec<u8>) -> Self {
        Self { schema_id, bytes }
    }

    #[must_use]
    pub const fn schema_id(&self) -> u64 {
        self.schema_id
    }

    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Module-local typed constants addressed by [`ConstantId`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConstantPool {
    constants: Vec<Constant>,
}

impl ConstantPool {
    #[must_use]
    pub fn new(constants: Vec<Constant>) -> Self {
        Self { constants }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.constants.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.constants.is_empty()
    }

    #[must_use]
    pub fn get(&self, id: ConstantId) -> Option<&Constant> {
        self.constants.get(id.index() as usize)
    }

    #[must_use]
    pub fn range(&self, first: ConstantId, count: u32) -> Option<&[Constant]> {
        let start = first.index() as usize;
        let end = start.checked_add(count as usize)?;
        self.constants.get(start..end)
    }
}

impl core::ops::Index<usize> for ConstantPool {
    type Output = Constant;

    fn index(&self, index: usize) -> &Self::Output {
        &self.constants[index]
    }
}

impl core::ops::IndexMut<usize> for ConstantPool {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.constants[index]
    }
}

/// Required dialect name and compatible intrinsic-set version.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DialectRequirement {
    name: String,
    major: u16,
    minor: u16,
}

impl DialectRequirement {
    #[must_use]
    pub fn new(name: impl Into<String>, major: u16, minor: u16) -> Self {
        Self {
            name: name.into(),
            major,
            minor,
        }
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn major(&self) -> u16 {
        self.major
    }

    #[must_use]
    pub const fn minor(&self) -> u16 {
        self.minor
    }

    fn supports(&self, required: &Self) -> bool {
        self.name == required.name && self.major == required.major && self.minor >= required.minor
    }
}

/// Durable module metadata independent of its producing frontend.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleManifest {
    name: String,
    format_major: u16,
    format_minor: u16,
    dialects: Vec<DialectRequirement>,
    root_entries: Vec<u32>,
}

impl ModuleManifest {
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        dialects: impl IntoIterator<Item = DialectRequirement>,
        root_entries: impl IntoIterator<Item = u32>,
    ) -> Self {
        Self {
            name: name.into(),
            format_major: 1,
            format_minor: 0,
            dialects: dialects.into_iter().collect(),
            root_entries: root_entries.into_iter().collect(),
        }
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn format_major(&self) -> u16 {
        self.format_major
    }

    #[must_use]
    pub const fn format_minor(&self) -> u16 {
        self.format_minor
    }

    #[must_use]
    pub fn dialects(&self) -> &[DialectRequirement] {
        &self.dialects
    }

    #[must_use]
    pub fn root_entries(&self) -> &[u32] {
        &self.root_entries
    }
}

/// A self-contained semantic Weavy module.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WeavyModule<Intrinsic> {
    manifest: ModuleManifest,
    program: DenseWeavyLowered<Intrinsic>,
    constants: ConstantPool,
    constant_ranges: Vec<ConstantRange>,
}

impl<Intrinsic> WeavyModule<Intrinsic> {
    #[must_use]
    pub fn new(
        manifest: ModuleManifest,
        program: DenseWeavyLowered<Intrinsic>,
        constants: ConstantPool,
    ) -> Self {
        Self {
            manifest,
            program,
            constant_ranges: Vec::new(),
            constants,
        }
    }

    #[must_use]
    pub const fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    #[must_use]
    pub const fn program(&self) -> &DenseWeavyLowered<Intrinsic> {
        &self.program
    }

    #[must_use]
    pub fn program_mut(&mut self) -> &mut DenseWeavyLowered<Intrinsic> {
        &mut self.program
    }

    #[must_use]
    pub const fn constants(&self) -> &ConstantPool {
        &self.constants
    }

    #[must_use]
    pub fn constants_mut(&mut self) -> &mut ConstantPool {
        &mut self.constants
    }

    #[must_use]
    pub fn with_constant_ranges(mut self, constant_ranges: Vec<ConstantRange>) -> Self {
        self.constant_ranges = constant_ranges;
        self
    }

    #[must_use]
    pub fn constant_ranges(&self) -> &[ConstantRange] {
        &self.constant_ranges
    }

    #[must_use]
    pub fn constant_ranges_mut(&mut self) -> &mut [ConstantRange] {
        &mut self.constant_ranges
    }
}

/// One intrinsic reference to a typed constant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConstantReference {
    id: ConstantId,
    expected_schema: u64,
}

impl ConstantReference {
    #[must_use]
    pub const fn new(id: ConstantId, expected_schema: u64) -> Self {
        Self {
            id,
            expected_schema,
        }
    }
}

/// One intrinsic reference to a homogeneous typed constant range.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConstantRangeReference {
    id: ConstantRangeId,
    expected_schema: SchemaId,
    expected_profile: StorageProfile,
}

impl ConstantRangeReference {
    #[must_use]
    pub const fn new(
        id: ConstantRangeId,
        expected_schema: SchemaId,
        expected_profile: StorageProfile,
    ) -> Self {
        Self {
            id,
            expected_schema,
            expected_profile,
        }
    }
}

/// Admission contract implemented by each durable intrinsic vocabulary.
pub trait IntrinsicContract {
    fn constant_references(&self, visit: &mut dyn FnMut(ConstantReference));

    fn constant_range_references(&self, _visit: &mut dyn FnMut(ConstantRangeReference)) {}
}

/// Admission context for one runtime's supported dialect set.
pub struct ModuleVerifier {
    dialects: BTreeMap<String, DialectRequirement>,
}

impl ModuleVerifier {
    #[must_use]
    pub fn new(dialects: impl IntoIterator<Item = DialectRequirement>) -> Self {
        Self {
            dialects: dialects
                .into_iter()
                .map(|dialect| (dialect.name.clone(), dialect))
                .collect(),
        }
    }

    pub fn admit<Intrinsic: IntrinsicContract>(
        &self,
        module: WeavyModule<Intrinsic>,
    ) -> Result<AdmittedModule<Intrinsic>, AdmissionError> {
        self.verify_manifest(module.manifest())?;
        verify_program(
            module.program(),
            module.constants(),
            module.constant_ranges(),
        )?;
        Ok(AdmittedModule { module })
    }
    pub fn verify_parts<Intrinsic: IntrinsicContract>(
        &self,
        manifest: &ModuleManifest,
        program: &DenseWeavyLowered<Intrinsic>,
        constants: &ConstantPool,
        constant_ranges: &[ConstantRangeMetadata],
    ) -> Result<(), AdmissionError> {
        self.verify_manifest(manifest)?;
        verify_program_metadata(program, constants, constant_ranges)
    }

    fn verify_manifest(&self, manifest: &ModuleManifest) -> Result<(), AdmissionError> {
        if manifest.format_major != 1 {
            return Err(AdmissionError::UnsupportedFormat {
                major: manifest.format_major,
                minor: manifest.format_minor,
            });
        }
        for required in &manifest.dialects {
            let Some(available) = self.dialects.get(&required.name) else {
                return Err(AdmissionError::MissingDialect {
                    name: required.name.clone(),
                });
            };
            if !available.supports(required) {
                return Err(AdmissionError::IncompatibleDialect {
                    name: required.name.clone(),
                    required_major: required.major,
                    required_minor: required.minor,
                    available_major: available.major,
                    available_minor: available.minor,
                });
            }
        }
        Ok(())
    }
}

fn verify_program<Intrinsic: IntrinsicContract>(
    program: &DenseWeavyLowered<Intrinsic>,
    constants: &ConstantPool,
    constant_ranges: &[ConstantRange],
) -> Result<(), AdmissionError> {
    verify_ops(
        &program.program,
        program.blocks.len(),
        constants,
        constant_ranges,
    )?;
    for block in &program.blocks {
        verify_ops(block, program.blocks.len(), constants, constant_ranges)?;
    }
    Ok(())
}

fn verify_ops<Intrinsic: IntrinsicContract>(
    ops: &[WeavyOp<BlockRef, Intrinsic>],
    block_count: usize,
    constants: &ConstantPool,
    constant_ranges: &[ConstantRange],
) -> Result<(), AdmissionError> {
    let metadata = constant_ranges
        .iter()
        .map(|range| ConstantRangeMetadata {
            schema_id: range.schema_id,
            profile: range.profile,
        })
        .collect::<Vec<_>>();
    verify_ops_metadata(ops, block_count, constants, &metadata)
}

fn verify_program_metadata<Intrinsic: IntrinsicContract>(
    program: &DenseWeavyLowered<Intrinsic>,
    constants: &ConstantPool,
    constant_ranges: &[ConstantRangeMetadata],
) -> Result<(), AdmissionError> {
    verify_ops_metadata(
        &program.program,
        program.blocks.len(),
        constants,
        constant_ranges,
    )?;
    for block in &program.blocks {
        verify_ops_metadata(block, program.blocks.len(), constants, constant_ranges)?;
    }
    Ok(())
}

fn verify_ops_metadata<Intrinsic: IntrinsicContract>(
    ops: &[WeavyOp<BlockRef, Intrinsic>],
    block_count: usize,
    constants: &ConstantPool,
    constant_ranges: &[ConstantRangeMetadata],
) -> Result<(), AdmissionError> {
    for op in ops {
        match op {
            WeavyOp::Control(ControlOp::CallBlock { block, .. }) => {
                verify_block(*block, block_count)?
            }
            WeavyOp::Control(ControlOp::CallBlockThen { block, then, .. }) => {
                verify_block(*block, block_count)?;
                verify_block(*then, block_count)?;
            }
            WeavyOp::Aggregate(AggregateOp::BeginList { loop_block, .. }) => {
                verify_block(*loop_block, block_count)?;
            }
            WeavyOp::Intrinsic(intrinsic) => {
                let mut error = None;
                intrinsic.constant_references(&mut |reference| {
                    if error.is_some() {
                        return;
                    }
                    let Some(constant) = constants.get(reference.id) else {
                        error = Some(AdmissionError::InvalidConstantId {
                            id: reference.id,
                            constant_count: constants.len(),
                        });
                        return;
                    };
                    if constant.schema_id != reference.expected_schema {
                        error = Some(AdmissionError::WrongConstantSchema {
                            id: reference.id,
                            expected: reference.expected_schema,
                            actual: constant.schema_id,
                        });
                    }
                });
                intrinsic.constant_range_references(&mut |reference| {
                    if error.is_some() {
                        return;
                    }
                    let Some(range) = constant_ranges.get(reference.id.index() as usize) else {
                        error = Some(AdmissionError::InvalidConstantRangeId {
                            id: reference.id,
                            range_count: constant_ranges.len(),
                        });
                        return;
                    };
                    if range.schema_id != reference.expected_schema {
                        error = Some(AdmissionError::WrongConstantRangeSchema {
                            id: reference.id,
                            expected: reference.expected_schema,
                            actual: range.schema_id,
                        });
                    } else if range.profile != reference.expected_profile {
                        error = Some(AdmissionError::WrongConstantRangeProfile {
                            id: reference.id,
                            expected: reference.expected_profile,
                            actual: range.profile,
                        });
                    }
                });
                if let Some(error) = error {
                    return Err(error);
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn verify_block(block: BlockRef, block_count: usize) -> Result<(), AdmissionError> {
    if block.index() >= block_count {
        Err(AdmissionError::InvalidBlockRef { block, block_count })
    } else {
        Ok(())
    }
}

/// A module that has passed all structural and intrinsic admission checks.
pub struct AdmittedModule<Intrinsic> {
    module: WeavyModule<Intrinsic>,
}

impl<Intrinsic> AdmittedModule<Intrinsic> {
    #[must_use]
    pub const fn module(&self) -> &WeavyModule<Intrinsic> {
        &self.module
    }

    #[must_use]
    pub fn into_module(self) -> WeavyModule<Intrinsic> {
        self.module
    }
}

/// Why a semantic module was rejected before execution.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum AdmissionError {
    UnsupportedFormat {
        major: u16,
        minor: u16,
    },
    MissingDialect {
        name: String,
    },
    IncompatibleDialect {
        name: String,
        required_major: u16,
        required_minor: u16,
        available_major: u16,
        available_minor: u16,
    },
    InvalidBlockRef {
        block: BlockRef,
        block_count: usize,
    },
    InvalidConstantId {
        id: ConstantId,
        constant_count: usize,
    },
    WrongConstantSchema {
        id: ConstantId,
        expected: u64,
        actual: u64,
    },
    InvalidConstantRangeId {
        id: ConstantRangeId,
        range_count: usize,
    },
    WrongConstantRangeSchema {
        id: ConstantRangeId,
        expected: SchemaId,
        actual: SchemaId,
    },
    WrongConstantRangeProfile {
        id: ConstantRangeId,
        expected: StorageProfile,
        actual: StorageProfile,
    },
}

impl fmt::Display for AdmissionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Weavy module admission failed: {self:?}")
    }
}

impl std::error::Error for AdmissionError {}
