//! "A folder" storage: any directory another tool keeps in sync (Syncthing, iCloud Drive, a cloud
//! provider's desktop client). Paths are confined to the chosen root.

use std::fs;
use std::path::{Component, Path, PathBuf};

use serde_json::{Value, json};

type Res<T> = Result<T, String>;

fn resolve(root: &str, rel: &str) -> Res<PathBuf> {
    let rel = Path::new(rel);
    if rel.components().any(|c| !matches!(c, Component::Normal(_))) {
        return Err("invalid path".into());
    }
    Ok(Path::new(root).join(rel))
}

/// Size and modification time: changes whenever the file does.
fn etag(path: &Path) -> Res<String> {
    let m = fs::metadata(path).map_err(|e| e.to_string())?;
    let mtime = m.modified().map_err(|e| e.to_string())?.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    Ok(format!("{}-{}", m.len(), mtime.as_nanos()))
}

#[tauri::command]
pub fn folder_list(root: String, dir: String) -> Res<Vec<Value>> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<Value>) -> Res<()> {
        let Ok(entries) = fs::read_dir(dir) else { return Ok(()) };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(root, &path, out)?;
            } else if path.extension().is_some_and(|e| e == "scyh" || e == "scyv") {
                let rel = path.strip_prefix(root).map_err(|e| e.to_string())?;
                let rel = rel.components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/");
                out.push(json!({ "path": rel, "etag": etag(&path)? }));
            }
        }
        Ok(())
    }
    let mut out = Vec::new();
    walk(Path::new(&root), &resolve(&root, &dir)?, &mut out)?;
    Ok(out)
}

#[tauri::command]
pub fn folder_get(root: String, path: String) -> Res<Option<Vec<u8>>> {
    let p = resolve(&root, &path)?;
    if !p.exists() {
        return Ok(None);
    }
    Ok(Some(fs::read(&p).map_err(|e| e.to_string())?))
}

/// `if_match`: None overwrites, Some("") only if absent, Some(etag) only if unchanged. Writes to a
/// temporary file and renames, so a sync tool never picks up a half-written file.
#[tauri::command]
pub fn folder_put(root: String, path: String, bytes: Vec<u8>, if_match: Option<String>) -> Res<String> {
    let p = resolve(&root, &path)?;
    match if_match.as_deref() {
        Some("") if p.exists() => return Err("conflict".into()),
        Some(tag) if !tag.is_empty() && etag(&p).ok().as_deref() != Some(tag) => return Err("conflict".into()),
        _ => {}
    }
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp = p.with_extension("partial");
    fs::write(&tmp, &bytes).map_err(|e| e.to_string())?;
    fs::rename(&tmp, &p).map_err(|e| e.to_string())?;
    etag(&p)
}
