use std::fs;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=assets/art");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let images: Vec<Vec<u8>> = ["icon16.png", "icon32.png", "icon64.png"]
        .iter()
        .map(|name| fs::read(format!("assets/art/{name}")).expect("missing icon png"))
        .collect();
    let mut ico = vec![0, 0, 1, 0];
    ico.extend_from_slice(&(images.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * images.len() as u32;
    for data in &images {
        let width = u32::from_be_bytes(data[16..20].try_into().unwrap());
        let height = u32::from_be_bytes(data[20..24].try_into().unwrap());
        ico.push(width as u8);
        ico.push(height as u8);
        ico.extend_from_slice(&[0, 0]);
        ico.extend_from_slice(&1u16.to_le_bytes());
        ico.extend_from_slice(&32u16.to_le_bytes());
        ico.extend_from_slice(&(data.len() as u32).to_le_bytes());
        ico.extend_from_slice(&offset.to_le_bytes());
        offset += data.len() as u32;
    }
    for data in &images {
        ico.extend_from_slice(data);
    }
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("app.ico");
    fs::write(&out, &ico).unwrap();
    winres::WindowsResource::new().set_icon(out.to_str().unwrap()).compile().unwrap();
}