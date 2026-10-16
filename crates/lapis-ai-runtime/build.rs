fn main() {
    println!(
        "cargo:rustc-env=LAPIS_AI_TARGET={}",
        std::env::var("TARGET").unwrap()
    );
}
