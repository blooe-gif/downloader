use anyhow::Context;
use std::fs::{File, OpenOptions};
use std::path::Path;

#[cfg(unix)]
use std::os::unix::fs::FileExt;
#[cfg(windows)]
use std::os::windows::fs::FileExt;

pub fn create_preallocated(path: &Path, size: u64) -> anyhow::Result<File> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .read(true)
        .open(path)
        .with_context(|| format!("failed to create {}", path.display()))?;
    file.set_len(size)?;
    Ok(file)
}

pub fn write_all_at(file: &File, mut offset: u64, mut buf: &[u8]) -> anyhow::Result<()> {
    while !buf.is_empty() {
        let written = file.write_at(buf, offset)?;
        if written == 0 {
            anyhow::bail!("write_at returned 0");
        }
        offset += written as u64;
        buf = &buf[written..];
    }
    Ok(())
}
