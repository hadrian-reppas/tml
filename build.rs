const USE_COMPUTED_GOTO: bool = true;

fn main() {
    println!("cargo:rerun-if-changed=src/vm.c");
    let profile = std::env::var("PROFILE").unwrap();
    match profile.as_str() {
        "debug" => cc::Build::new()
            .file("src/vm.c")
            .define("DEBUG", "1")
            .compile("vm"),
        "release" => {
            if USE_COMPUTED_GOTO {
                cc::Build::new()
                    .file("src/vm.c")
                    .opt_level(3)
                    .define("USE_COMPUTED_GOTO", "1")
                    .compile("vm")
            } else {
                cc::Build::new().file("src/vm.c").opt_level(3).compile("vm")
            }
        }
        _ => unreachable!(),
    }
}
