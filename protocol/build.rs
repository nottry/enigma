use std::path::Path;

fn main() {
    println!("cargo::rustc-check-cfg=cfg(rust_analyzer)");

    let proto_files = ["sonetto.proto", "cmd_id.proto"];
    println!("cargo::rerun-if-env-changed=ENIGMA_REGEN_PROTO");

    // Opt-in: a stale local proto would otherwise silently rewrite the committed include/_.rs.
    if std::env::var_os("ENIGMA_REGEN_PROTO").is_some()
        && proto_files.iter().all(|f| Path::new(f).exists())
    {
        for proto in &proto_files {
            println!("cargo::rerun-if-changed={proto}");
        }
        prost_build::Config::new()
            .type_attribute(
                ".",
                "#[derive(serde::Serialize, serde::Deserialize)]\n#[serde(rename_all = \"camelCase\")]",
            )
            .message_attribute(".", r#"#[serde(default)]"#)
            .field_attribute("*.type", "#[serde(rename = \"type\")]")
            .out_dir("include/")
            .compile_protos(&proto_files, &["."])
            .expect("Failed to compile proto files");
    } else {
        // Without regeneration, watching a possibly missing proto file is always dirty.
        println!("cargo::rerun-if-changed=.");
    }
}
