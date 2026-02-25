use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let cmap_dir = Path::new("src/data/cmap");
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_path = Path::new(&out_dir).join("cmap_data.rs");

    let mut entries = Vec::new();

    if cmap_dir.exists() {
        let mut files: Vec<_> = fs::read_dir(cmap_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        files.sort();

        for name in &files {
            let path = format!("src/data/cmap/{}", name);
            entries.push(format!(
                "    (\"{name}\", include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/{path}\"))),",
                name = name,
                path = path,
            ));
        }
    }

    let code = format!(
        "static PREDEFINED_CMAPS: &[(&str, &str)] = &[\n{}\n];\n",
        entries.join("\n")
    );

    fs::write(out_path, code).unwrap();

    println!("cargo:rerun-if-changed=src/data/cmap");
}
