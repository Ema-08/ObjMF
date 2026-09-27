use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, ErrorKind, Write};
use std::path::Path;

fn open_for_write(path: &Path, fail_if_exists: bool) -> Result<File, String> {
    let mut opts = OpenOptions::new();
    opts.write(true);
    if fail_if_exists {
        opts.create_new(true);
    } else {
        opts.create(true).truncate(true);
    }

    opts.open(path).map_err(|e| match e.kind() {
        ErrorKind::AlreadyExists => format!("File already exists: {}", path.display()),
        _ => format!("Cannot create {}: {e}", path.display()),
    })
}

pub fn read_textual(path: &Path) -> Result<Vec<String>, String> {
    let bytes = fs::read(path)
        .map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
    let decoded = String::from_utf8_lossy(&bytes);
    let text: &str = decoded.strip_prefix('\u{FEFF}').unwrap_or(&decoded);
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\r' | '\n' => {
                let opposite = if c == '\r' { '\n' } else { '\r' };
                if chars.peek() == Some(&opposite) {
                    chars.next();
                }
                lines.push(std::mem::take(&mut current));
            }
            _ => current.push(c),
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    Ok(lines)
}

pub fn write_textual(
    path: &Path,
    content: Vec<String>,
    fail_if_exists: bool,
) -> Result<(), String> {
    let file = open_for_write(path, fail_if_exists)?;
    let mut writer = BufWriter::new(file);
    let result = (|| -> Result<(), String> {
        for line in &content {
            writer
                .write_all(line.as_bytes())
                .and_then(|_| writer.write_all(b"\n"))
                .map_err(|e| format!("Cannot write {}: {e}", path.display()))?;
        }
        writer
            .flush()
            .map_err(|e| format!("Cannot finish writing {}: {e}", path.display()))
    })();
    if result.is_err() && fail_if_exists {
        drop(writer);
        let _ = fs::remove_file(path);
    }
    result
}

pub fn read_bin(path: &Path) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|e| format!("Cannot read {}: {e}", path.display()))
}

pub fn write_bin(
    path: &Path,
    data: Vec<u8>,
    fail_if_exists: bool,
) -> Result<(), String> {
    let mut file = open_for_write(path, fail_if_exists)?;

    let result = file
        .write_all(&data)
        .map_err(|e| format!("Cannot write {}: {e}", path.display()))
        .and_then(|_| {
            file.flush()
                .map_err(|e| format!("Cannot finish writing {}: {e}", path.display()))
        });

    if result.is_err() && fail_if_exists {
        drop(file);
        let _ = fs::remove_file(path);
    }
    result
}