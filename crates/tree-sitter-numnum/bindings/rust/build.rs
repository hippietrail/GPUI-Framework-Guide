fn main() {
    let src_dir = std::path::Path::new("../../src");

    let mut c_config = cc::Build::new();
    c_config.include(src_dir);
    c_config.file(src_dir.join("parser.c"));

    // Handle tree-sitter scanner if it exists
    let scanner_path = src_dir.join("scanner.c");
    if scanner_path.exists() {
        c_config.file(&scanner_path);
    }

    c_config.compile("tree-sitter-numnum");
}
