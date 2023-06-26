use std::env;
use std::fs;
use std::io::Read;
use std::path::Path;

fn main() {
    let manifest_dir = env::var_os("CARGO_MANIFEST_DIR").unwrap();

    let compiler = shaderc::Compiler::new().unwrap();
    let cs = compiler
        .compile_into_spirv(
            "shader.comp",
            shaderc::ShaderKind::Compute,
            "shader.comp",
            "main",
            None,
        )
        .unwrap();

    println!("cargo:rerun-if-changed=build.rs");
}