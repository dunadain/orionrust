use std::{env, fs, path::Path, vec};

fn main() {
    let mut vec = vec![];
    find_proto_files("src/", &mut vec);
    prost_build::compile_protos(&vec, &["src/"]).unwrap();
    let content = r#"
pub mod protocol {
    include!(concat!(env!("OUT_DIR"), "/game.rs"));
}
pub mod rpc {
"#;
    let mut content = content.to_string();

    let out_dir = env::var("OUT_DIR").unwrap();
    for entry in fs::read_dir(out_dir).expect("Failed to read directory") {
        let entry = entry.expect("Failed to read entry");
        let file_name = entry.file_name().into_string().unwrap();
        if file_name == "game.rs" {
            continue;
        }
        let item = format!(
            "   include!(concat!(env!(\"OUT_DIR\"), \"/{}\"));",
            file_name
        );
        // println!("cargo:warning=Item {}", item);
        content.push_str(&item);
    }
    content.push_str("\n}");
    let dest_path = Path::new("src").join("lib.rs");
    fs::write(dest_path, content).unwrap();
}

fn find_proto_files(dir: &str, protos: &mut Vec<String>) {
    for entry in fs::read_dir(dir).expect("Failed to read directory") {
        let entry = entry.expect("Failed to read entry");
        let path = entry.path();
        if path.is_dir() {
            find_proto_files(path.to_str().unwrap(), protos);
        } else if path.extension().and_then(|s| s.to_str()) == Some("proto") {
            protos.push(path.to_str().unwrap().to_string());
        }
    }
}
