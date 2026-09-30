fn main() {
  // intel_tex_2's bundled Linux objects use the C++ standard library.
  if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("linux") {
    println!("cargo:rustc-link-lib=stdc++");
  }
}
