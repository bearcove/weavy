use std::env;
use std::path::PathBuf;

use snark_dsl::typed_ast::{TypedAstConfig, generate_typed_ast};

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let grammar_js = manifest.join("grammar/grammar.js");
    let ann_js = manifest.join("fable_ast.snark.js");
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());

    println!("cargo:rerun-if-changed=build.rs");
    generate_typed_ast(&TypedAstConfig {
        grammar_js: &grammar_js,
        annotations_js: &ann_js,
        out_dir: &out,
        grammar_output: "fable_grammar.json",
        ast_output: "fable_ast.rs",
        annotation_source_name: "fable_ast.snark.js",
        generated_by: "fable/build.rs",
        language_name: "fable",
    })
    .expect("generate fable typed AST");
}
