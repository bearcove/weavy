use phon_schema::{Primitive, Schema, SchemaId, SchemaKind, primitive_id};
use weavy::ir::{ControlOp, WeavyOp};
use weavy::module::{
    AdmissionError, Constant, ConstantId, ConstantPool, ConstantRange, ConstantRangeError,
    ConstantRangeId, ConstantRangeReference, ConstantReference, DialectRequirement,
    IntrinsicContract, ModuleManifest, ModuleVerifier, StorageProfile, WeavyModule,
};
use weavy::{BlockRef, DenseLowered};

#[derive(Clone, Debug, PartialEq, Eq)]
struct TestIntrinsic {
    constant: ConstantId,
    range: ConstantRangeId,
    expected_range_schema: SchemaId,
}

impl IntrinsicContract for TestIntrinsic {
    fn constant_references(&self, visit: &mut dyn FnMut(ConstantReference)) {
        visit(ConstantReference::new(self.constant, 0x42));
    }

    fn constant_range_references(&self, visit: &mut dyn FnMut(ConstantRangeReference)) {
        visit(ConstantRangeReference::new(
            self.range,
            self.expected_range_schema,
            StorageProfile::Aligned,
        ));
    }
}

fn manifest() -> ModuleManifest {
    ModuleManifest::new("test.module", [DialectRequirement::new("test", 1, 0)], [0])
}

fn range(profile: StorageProfile) -> ConstantRange {
    ConstantRange::new(
        vec![Schema {
            id: SchemaId::from_raw(1),
            type_params: Vec::new(),
            kind: SchemaKind::Primitive(Primitive::U32),
        }],
        0,
        profile,
        3,
        32,
        vec![0; 96],
    )
    .expect("range")
}

fn module(constant: ConstantId) -> WeavyModule<TestIntrinsic> {
    WeavyModule::new(
        manifest(),
        DenseLowered::new(
            vec![
                WeavyOp::Intrinsic(TestIntrinsic {
                    constant,
                    range: ConstantRangeId::new(0),
                    expected_range_schema: primitive_id(Primitive::U32),
                }),
                WeavyOp::Control(ControlOp::CallBlock {
                    block: BlockRef::new(0),
                    base_offset: 0,
                }),
            ],
            vec![vec![WeavyOp::Control(ControlOp::Return)]],
        ),
        ConstantPool::new(vec![Constant::new(0x42, vec![1, 2, 3])]),
    )
    .with_constant_ranges(vec![range(StorageProfile::Aligned)])
}

#[test]
fn range_schema_identity_is_content_derived() {
    let derived = range(StorageProfile::Aligned);
    assert_eq!(derived.schema_id(), primitive_id(Primitive::U32));
    assert!(matches!(
        ConstantRange::new(Vec::new(), 0, StorageProfile::Aligned, 1, 32, Vec::new()),
        Err(ConstantRangeError::InvalidRootIndex { .. })
    ));
}

#[test]
fn admission_validates_blocks_constants_ranges_schemas_profiles_and_dialects() {
    let admitted = ModuleVerifier::new([DialectRequirement::new("test", 1, 7)])
        .admit(module(ConstantId::new(0)))
        .expect("admit");
    assert_eq!(admitted.module().constants().len(), 1);
    assert_eq!(admitted.module().constant_ranges().len(), 1);

    assert!(matches!(
        ModuleVerifier::new([DialectRequirement::new("test", 1, 7)])
            .admit(module(ConstantId::new(9))),
        Err(AdmissionError::InvalidConstantId { .. })
    ));

    let mut wrong_schema = module(ConstantId::new(0));
    wrong_schema.constants_mut()[0] = Constant::new(0x99, vec![1]);
    assert!(matches!(
        ModuleVerifier::new([DialectRequirement::new("test", 1, 7)]).admit(wrong_schema),
        Err(AdmissionError::WrongConstantSchema { .. })
    ));

    let mut wrong_range_schema = module(ConstantId::new(0));
    wrong_range_schema.program_mut().program[0] = WeavyOp::Intrinsic(TestIntrinsic {
        constant: ConstantId::new(0),
        range: ConstantRangeId::new(0),
        expected_range_schema: SchemaId::from_raw(0x88),
    });
    assert!(matches!(
        ModuleVerifier::new([DialectRequirement::new("test", 1, 7)]).admit(wrong_range_schema),
        Err(AdmissionError::WrongConstantRangeSchema { .. })
    ));

    let mut wrong_profile = module(ConstantId::new(0));
    wrong_profile.constant_ranges_mut()[0] = range(StorageProfile::Compact);
    assert!(matches!(
        ModuleVerifier::new([DialectRequirement::new("test", 1, 7)]).admit(wrong_profile),
        Err(AdmissionError::WrongConstantRangeProfile { .. })
    ));

    let mut invalid_range = module(ConstantId::new(0));
    invalid_range.program_mut().program[0] = WeavyOp::Intrinsic(TestIntrinsic {
        constant: ConstantId::new(0),
        range: ConstantRangeId::new(9),
        expected_range_schema: primitive_id(Primitive::U32),
    });
    assert!(matches!(
        ModuleVerifier::new([DialectRequirement::new("test", 1, 7)]).admit(invalid_range),
        Err(AdmissionError::InvalidConstantRangeId { .. })
    ));
}

#[test]
fn admission_rejects_invalid_block_references() {
    let mut invalid = module(ConstantId::new(0));
    invalid.program_mut().program[1] = WeavyOp::Control(ControlOp::CallBlock {
        block: BlockRef::new(9),
        base_offset: 0,
    });
    assert!(matches!(
        ModuleVerifier::new([DialectRequirement::new("test", 1, 0)]).admit(invalid),
        Err(AdmissionError::InvalidBlockRef { .. })
    ));
}

#[test]
fn legacy_constant_ranges_share_the_module_local_id_space() {
    let pool = ConstantPool::new(vec![
        Constant::new(7, vec![1]),
        Constant::new(7, vec![2]),
        Constant::new(7, vec![3]),
    ]);
    let range = pool.range(ConstantId::new(1), 2).expect("range");
    assert_eq!(range[0].bytes(), &[2]);
    assert_eq!(range[1].bytes(), &[3]);
    assert!(pool.range(ConstantId::new(2), 2).is_none());
}
