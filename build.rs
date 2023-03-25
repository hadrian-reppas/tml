fn main() {
    println!("cargo:rerun-if-changed=src/vm.c");
    cc::Build::new().file("src/vm.c").opt_level(3).compile("vm");
}
