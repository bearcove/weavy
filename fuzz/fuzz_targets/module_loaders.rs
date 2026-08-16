#![no_main]

use libfuzzer_sys::fuzz_target;
use weavy::module::{DialectRequirement, IntrinsicContract, ModuleVerifier};
use weavy_phon::{CodecError, IntrinsicCodec};

struct OpaqueIntrinsic(Vec<u8>);

impl IntrinsicContract for OpaqueIntrinsic {
    fn constant_references(&self, _: &mut dyn FnMut(weavy::module::ConstantReference)) {}
}

struct OpaqueCodec;

impl IntrinsicCodec for OpaqueCodec {
    type Intrinsic = OpaqueIntrinsic;
    const DIALECT: &'static str = "fuzz";
    const SCHEMA_ID: u64 = 0;

    fn encode(intrinsic: &Self::Intrinsic, out: &mut Vec<u8>) {
        out.extend_from_slice(&intrinsic.0);
    }

    fn decode(bytes: &[u8]) -> Result<Self::Intrinsic, CodecError> {
        Ok(OpaqueIntrinsic(bytes.to_vec()))
    }
}

fuzz_target!(|bytes: &[u8]| {
    let verifier = ModuleVerifier::new([DialectRequirement::new("fuzz", 1, 0)]);
    let _ = weavy_phon::inspect(bytes);
    let _ = weavy_phon::load::<OpaqueCodec>(bytes);
    let _ = weavy_phon::load_borrowed::<OpaqueCodec>(bytes);
    let _ = weavy_phon::load_admitted::<OpaqueCodec>(bytes, &verifier);
    let _ = weavy_phon::load_borrowed_admitted::<OpaqueCodec>(bytes, &verifier);
});
