use std::{fs::File, io::Read, path::PathBuf};

pub fn read_wasm_bytes(path: &PathBuf) -> anyhow::Result<Vec<u8>> {
    let mut file = File::open(path)?;
    let mut wasm = Vec::new();
    file.read_to_end(&mut wasm)?;
    Ok(wasm)
}
