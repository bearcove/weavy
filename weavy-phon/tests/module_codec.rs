use weavy::ir::{ControlOp, WeavyOp};
use weavy::module::{
    Constant, ConstantId, ConstantPool, ConstantReference, DialectRequirement, IntrinsicContract,
    ModuleManifest, WeavyModule,
};
use weavy::{BlockRef, DenseLowered};
use weavy_phon::{CodecError, IntrinsicCodec, inspect, load, save};

#[derive(Clone, Debug, PartialEq, Eq)]
struct TestIntrinsic {
    constant: ConstantId,
}
impl IntrinsicContract for TestIntrinsic {
    fn constant_references(&self, visit: &mut dyn FnMut(ConstantReference)) {
        visit(ConstantReference::new(self.constant, 0x42));
    }
}
struct TestCodec;
impl IntrinsicCodec for TestCodec {
    type Intrinsic = TestIntrinsic;
    const DIALECT: &'static str = "test";
    const SCHEMA_ID: u64 = 0x7711;
    fn encode(intrinsic: &Self::Intrinsic, out: &mut Vec<u8>) {
        out.extend_from_slice(&intrinsic.constant.index().to_le_bytes());
    }
    fn decode(bytes: &[u8]) -> Result<Self::Intrinsic, CodecError> {
        if bytes.len() != 4 {
            return Err(CodecError::MalformedIntrinsic);
        }
        Ok(TestIntrinsic {
            constant: ConstantId::new(u32::from_le_bytes(bytes.try_into().expect("length"))),
        })
    }
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
}
#[test]
fn weavy_bytes_round_trip_deterministically() {
    let module = fixture();
    let first = save::<TestCodec>(&module).expect("save");
    let loaded = load::<TestCodec>(&first).expect("load");
    assert_eq!(loaded, module);
    assert_eq!(save::<TestCodec>(&loaded).expect("save again"), first);
}
#[test]
fn inspect_reports_discoverable_module_facts() {
    let bytes = save::<TestCodec>(&fixture()).expect("save");
    let report = inspect(&bytes).expect("inspect");
    assert_eq!(report.module_name, "codec.fixture");
    assert_eq!(report.program_op_count, 3);
    assert_eq!(report.block_count, 1);
    assert_eq!(report.constant_count, 1);
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
            .any(|section| section.name == "constants")
    );
}
#[test]
fn malformed_modules_are_rejected() {
    let bytes = save::<TestCodec>(&fixture()).expect("save");
    let mut corrupted = bytes.clone();
    let last = corrupted.len() - 1;
    corrupted[last] ^= 0x80;
    assert!(matches!(
        load::<TestCodec>(&corrupted),
        Err(CodecError::IntegrityMismatch { .. })
    ));
    assert!(matches!(
        load::<TestCodec>(&bytes[..bytes.len() - 1]),
        Err(CodecError::Truncated { .. })
    ));
    let mut bad_offset = bytes.clone();
    bad_offset[24..32].copy_from_slice(&u64::MAX.to_le_bytes());
    assert!(matches!(
        load::<TestCodec>(&bad_offset),
        Err(CodecError::SectionOutOfBounds { .. }) | Err(CodecError::IntegrityMismatch { .. })
    ));
    let mut bad_alignment = bytes.clone();
    bad_alignment[40..44].copy_from_slice(&3u32.to_le_bytes());
    assert!(matches!(
        load::<TestCodec>(&bad_alignment),
        Err(CodecError::InvalidAlignment { .. }) | Err(CodecError::IntegrityMismatch { .. })
    ));
    let mut unknown_required = bytes;
    unknown_required[44..48].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(matches!(
        load::<TestCodec>(&unknown_required),
        Err(CodecError::UnknownRequiredSection { .. }) | Err(CodecError::IntegrityMismatch { .. })
    ));
}
