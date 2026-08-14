use facet_value::{VArray, VObject, VString, Value};
use phon_schema::{
    Field, Primitive, Schema, SchemaId, SchemaKind, SchemaRef, primitive_id, resolve_ids,
};
use phon_storage::{AlignedRegistry, AlignedWriter};
use weavy::ir::{ControlOp, WeavyOp};
use weavy::module::{
    Constant, ConstantId, ConstantPool, ConstantRange, ConstantRangeId, ConstantRangeReference,
    ConstantReference, DialectRequirement, IntrinsicContract, ModuleManifest, StorageProfile,
    WeavyModule,
};
use weavy::{BlockRef, DenseLowered};
use weavy_phon::{CodecError, IntrinsicCodec};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TestIntrinsic {
    constant: ConstantId,
    range: ConstantRangeId,
}

impl TestIntrinsic {
    pub(crate) const fn new(constant: ConstantId, range: ConstantRangeId) -> Self {
        Self { constant, range }
    }
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

pub(crate) struct TestCodec;

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

pub(crate) fn range_schema() -> (Vec<Schema>, SchemaId) {
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

pub(crate) fn logical_rows() -> Value {
    let mut rows = VArray::new();
    for value in [1u32, 2, 3] {
        let mut row = VObject::new();
        row.insert(VString::new("value"), Value::from(value));
        rows.push(row);
    }
    rows.into()
}

pub(crate) fn aligned_rows() -> Vec<u8> {
    let (schemas, root) = range_schema();
    let registry = AlignedRegistry::new(schemas);
    AlignedWriter::encode(&logical_rows(), root, &registry).expect("aligned rows")
}

pub(crate) fn fixture() -> WeavyModule<TestIntrinsic> {
    WeavyModule::new(
        ModuleManifest::new(
            "codec.fixture",
            [DialectRequirement::new("test", 1, 0)],
            [0],
        ),
        DenseLowered::new(
            vec![
                WeavyOp::Intrinsic(TestIntrinsic::new(
                    ConstantId::new(0),
                    ConstantRangeId::new(0),
                )),
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
