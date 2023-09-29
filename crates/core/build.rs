use std::{
    env,
    fs::{self, File},
    io::Write,
    path::PathBuf,
};

use inflector::Inflector;

fn main() {
    uniffi::generate_scaffolding("src/lib.udl").unwrap();
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/interner/strings.txt");

    let contents = fs::read_to_string("src/interner/strings.txt").unwrap();

    let mut symbols = Vec::with_capacity(100);
    symbols.push(("Empty".to_string(), "".to_string()));
    for line in contents.lines() {
        let line = line.trim();
        // Skip blank and comment lines
        if line.is_empty() || line.starts_with("#") || line.is_empty() {
            continue;
        }
        match line.split_once(':') {
            None => {
                let name = derive_name(None, line);
                let value = line.to_string();
                symbols.push((name, value));
            }
            Some((name, value)) => {
                let name = derive_name(Some(name), value);
                symbols.push((name, value.to_string()));
            }
        }
    }

    let out = PathBuf::from(env::var("OUT_DIR").unwrap()).join("strings.rs");
    let mut file = File::create(&out).unwrap();
    file.write_all(b"use super::Symbol;\n\n").unwrap();

    // Declare symbols
    for (i, (name, _)) in symbols.iter().enumerate() {
        write!(
            &mut file,
            r#"
#[allow(non_upper_case_globals)]
pub const {}: Symbol = Symbol::new({});
"#,
            &name, i
        )
        .unwrap()
    }

    // Symbol strings
    file.write_all(b"\n\npub(crate) const __SYMBOLS: &'static [(Symbol, &'static str)] = &[\n")
        .unwrap();
    for (name, value) in symbols.iter() {
        writeln!(&mut file, " ({}, \"{}\"),", name.as_str(), value.as_str()).unwrap();
    }
    file.write_all(b"];\n\n").unwrap();
    file.sync_data().unwrap();
}

fn derive_name(name: Option<&str>, value: &str) -> String {
    // Use explicitly provided name
    if let Some(name) = name {
        return name.to_string();
    }

    // If the value is screaming snake case, e.g. FOO_BAR, keep it that way as the casing is intentional
    if value.is_screaming_snake_case() {
        value.to_string()
    } else {
        value.to_pascal_case()
    }
}
