use weavy::ir::{ControlOp, WeavyOp};
use weavy::module::{
    AdmissionError, Constant, ConstantId, ConstantPool, ConstantReference, DialectRequirement,
    IntrinsicContract, ModuleManifest, ModuleVerifier, WeavyModule,
};
use weavy::{BlockRef, DenseLowered};

#[derive(Clone, Debug, PartialEq, Eq)]
struct TestIntrinsic {
    constant: ConstantId,
}

impl IntrinsicContract for TestIntrinsic {
    fn constant_references(&self, visit: &mut dyn FnMut(ConstantReference)) {
        visit(ConstantReference::new(self.constant, 0x42));
    }
}

fn manifest() -> ModuleManifest {
    ModuleManifest::new("test.module", [DialectRequirement::new("test", 1, 0)], [0])
}

fn module(constant: ConstantId) -> WeavyModule<TestIntrinsic> {
    WeavyModule::new(
        manifest(),
        DenseLowered::new(
            vec![
                WeavyOp::Intrinsic(TestIntrinsic { constant }),
                WeavyOp::Control(ControlOp::CallBlock {
                    block: BlockRef::new(0),
                    base_offset: 0,
                }),
            ],
            vec![vec![WeavyOp::Control(ControlOp::Return)]],
        ),
        ConstantPool::new(vec![Constant::new(0x42, vec![1, 2, 3])]),
    )
}

#[test]
fn admission_validates_blocks_constants_schemas_and_dialects() {
    let admitted = ModuleVerifier::new([DialectRequirement::new("test", 1, 7)])
        .admit(module(ConstantId::new(0)))
        .expect("admit");
    assert_eq!(admitted.module().constants().len(), 1);
    assert_eq!(admitted.module().manifest().name(), "test.module");

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

    assert!(matches!(
        ModuleVerifier::new([DialectRequirement::new("other", 1, 0)])
            .admit(module(ConstantId::new(0))),
        Err(AdmissionError::MissingDialect { .. })
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
fn constant_ranges_share_the_module_local_id_space() {
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
