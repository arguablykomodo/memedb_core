use std::env::var_os;
use std::fs::{read_dir, write};
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let cargo_manifest_dir = var_os("CARGO_MANIFEST_DIR").unwrap();
    let src_path = Path::new(&cargo_manifest_dir).join("src").join("formats");

    let mut data = Vec::new();

    for file in read_dir(src_path).unwrap() {
        let path = file.unwrap().path();
        println!("cargo:rerun-if-changed={}", path.display());

        let module = path.file_stem().unwrap().to_str().unwrap();

        if var_os(format!("CARGO_FEATURE_{}", module.to_uppercase())).is_none() {
            continue;
        }

        let upper = {
            let mut c = module.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        };

        data.push(format!("{} => {}", module, upper));
    }

    let out_dir = var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("format_macro.rs");

    write(dest_path, format!("generate_formats!({});", data.join(","))).unwrap();
}
