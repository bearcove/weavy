use weavy::DenseLowered;
use weavy::ir::WeavyOp;
use weavy::module::{
    Constant, ConstantId, ConstantPool, ConstantReference, DialectRequirement, IntrinsicContract,
    ModuleManifest, ModuleVerifier, WeavyModule,
};
use weavy_phon::{
    AdmittedLoadError, CodecError, IntrinsicCodec, load_admitted, load_borrowed_admitted, save,
};

#[derive(Clone, Debug, PartialEq, Eq)]
struct Intrinsic(ConstantId);
impl IntrinsicContract for Intrinsic {
    fn constant_references(&self, visit: &mut dyn FnMut(ConstantReference)) {
        visit(ConstantReference::new(self.0, 7));
    }
}
struct Codec;
impl IntrinsicCodec for Codec {
    type Intrinsic = Intrinsic;
    const DIALECT: &'static str = "admission";
    const SCHEMA_ID: u64 = 9;
    fn encode(intrinsic: &Intrinsic, out: &mut Vec<u8>) {
        out.extend_from_slice(&intrinsic.0.index().to_le_bytes());
    }
    fn decode(bytes: &[u8]) -> Result<Intrinsic, CodecError> {
        Ok(Intrinsic(ConstantId::new(u32::from_le_bytes(
            bytes
                .try_into()
                .map_err(|_| CodecError::MalformedIntrinsic)?,
        ))))
    }
}

#[test]
fn bad_constant_id_is_rejected_before_execution() {
    let module = WeavyModule::new(
        ModuleManifest::new(
            "bad.constant",
            [DialectRequirement::new("admission", 1, 0)],
            [0],
        ),
        DenseLowered::new(
            vec![WeavyOp::Intrinsic(Intrinsic(ConstantId::new(4)))],
            vec![],
        ),
        ConstantPool::new(vec![Constant::new(7, vec![])]),
    );
    let bytes = save::<Codec>(&module).expect("save");
    assert!(matches!(
        load_admitted::<Codec>(
            &bytes,
            &ModuleVerifier::new([DialectRequirement::new("admission", 1, 0)]),
        ),
        Err(AdmittedLoadError::Admission(
            weavy::module::AdmissionError::InvalidConstantId { .. }
        ))
    ));
}

#[test]
fn admitted_load_returns_execution_ready_module() {
    let module = WeavyModule::new(
        ModuleManifest::new(
            "good.constant",
            [DialectRequirement::new("admission", 1, 0)],
            [0],
        ),
        DenseLowered::new(
            vec![WeavyOp::Intrinsic(Intrinsic(ConstantId::new(0)))],
            vec![],
        ),
        ConstantPool::new(vec![Constant::new(7, vec![])]),
    );
    let bytes = save::<Codec>(&module).expect("save");
    let admitted = load_admitted::<Codec>(
        &bytes,
        &ModuleVerifier::new([DialectRequirement::new("admission", 1, 0)]),
    )
    .expect("load and admit");
    assert_eq!(admitted.module(), &module);
}

#[test]
fn admitted_borrowed_load_preserves_module_borrowing() {
    let module = WeavyModule::new(
        ModuleManifest::new(
            "good.borrowed",
            [DialectRequirement::new("admission", 1, 0)],
            [0],
        ),
        DenseLowered::new(
            vec![WeavyOp::Intrinsic(Intrinsic(ConstantId::new(0)))],
            vec![],
        ),
        ConstantPool::new(vec![Constant::new(7, vec![])]),
    );
    let bytes = save::<Codec>(&module).expect("save");
    let admitted = load_borrowed_admitted::<Codec>(
        &bytes,
        &ModuleVerifier::new([DialectRequirement::new("admission", 1, 0)]),
    )
    .expect("borrowed load and admit");
    assert_eq!(admitted.module().manifest(), module.manifest());
    assert_eq!(admitted.module().program(), module.program());
    assert_eq!(admitted.module().constants(), module.constants());
}
