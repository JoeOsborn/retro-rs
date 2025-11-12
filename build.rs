fn main() {
    cc::Build::new().file("c-src/logging.c").compile("shims");
}
