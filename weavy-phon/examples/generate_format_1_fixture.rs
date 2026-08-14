#[path = "../fixture-support/mod.rs"]
mod support;

use std::path::PathBuf;

use weavy_phon::save;

fn main() {
    let mut args = std::env::args_os();
    let _program = args.next();
    let output = PathBuf::from(
        args.next()
            .expect("usage: generate_format_1_fixture <output.weavy>"),
    );
    assert!(args.next().is_none(), "expected exactly one output path");

    let bytes = save::<support::TestCodec>(&support::fixture()).expect("generate fixture");
    std::fs::write(output, bytes).expect("write fixture");
}
