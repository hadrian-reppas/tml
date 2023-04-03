fn main() {
    println!("cargo:rerun-if-changed=src/vm.c");
    let profile = std::env::var("PROFILE").unwrap();
    match profile.as_str() {
        "debug" => cc::Build::new()
            .file("src/vm.c")
            .define("DEBUG", "1")
            .compile("vm"),
        "release" => cc::Build::new().file("src/vm.c").opt_level(3).compile("vm"),
        _ => unreachable!(),
    }
}
