// intel_tex_2 links C++ kernels (ispc_texcomp) that need the C++ runtime; Rust does not add it on its own.
fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("linux") {
        println!("cargo:rustc-link-lib=stdc++");
    }
}
