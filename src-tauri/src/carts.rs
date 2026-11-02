//! Cart lookup ported from the old server.py; no Tauri types so it can be unit-tested alone.

use std::fs;
use std::path::{Path, PathBuf};

pub const EXT: &str = ".p8.png";

fn strip_suffix_ci<'a>(s: &'a str, suf: &str) -> Option<&'a str> {
    let cut = s.len().checked_sub(suf.len())?;
    if s.is_char_boundary(cut) && s[cut..].eq_ignore_ascii_case(suf) {
        Some(&s[..cut])
    } else {
        None
    }
}

pub fn list(dir: &Path) -> Vec<(String, u64)> {
    let mut out = Vec::new();
    if let Ok(rd) = fs::read_dir(dir) {
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if let Some(id) = strip_suffix_ci(&name, EXT) {
                if !id.is_empty() {
                    let size = e.metadata().map(|m| m.len()).unwrap_or(0);
                    out.push((id.to_string(), size));
                }
            }
        }
    }
    out.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
    out
}

// Multi-cart games call load("name") while the file is name-3.p8.png, hence the highest-revision fallback.
pub fn resolve(dir: &Path, lid: &str) -> Option<PathBuf> {
    let mut lid = lid.trim();
    for suf in [EXT, ".p8", ".png"] {
        if let Some(rest) = strip_suffix_ci(lid, suf) {
            lid = rest;
            break;
        }
    }
    if lid.is_empty() || lid.starts_with('.') || lid.contains(['/', '\\', '\0']) {
        return None;
    }
    let key = lid.to_lowercase();
    let files = list(dir);

    if let Some((id, _)) = files.iter().find(|(id, _)| id.to_lowercase() == key) {
        return Some(dir.join(format!("{id}{EXT}")));
    }
    let prefix = format!("{key}-");
    files
        .iter()
        .filter_map(|(id, _)| {
            let low = id.to_lowercase();
            let n = low.strip_prefix(&prefix)?;
            if !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()) {
                Some((n.parse::<u64>().ok()?, id))
            } else {
                None
            }
        })
        .max_by_key(|(n, _)| *n)
        .map(|(_, id)| dir.join(format!("{id}{EXT}")))
}

pub fn safe_id(name: &str) -> Option<String> {
    let base = name.rsplit(['/', '\\']).next()?.trim();
    let mut id = base;
    for suf in [EXT, ".png", ".p8"] {
        if let Some(rest) = strip_suffix_ci(id, suf) {
            id = rest;
            break;
        }
    }
    let id = id.trim();
    if id.is_empty() || id.starts_with('.') || id.contains('\0') {
        None
    } else {
        Some(id.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(tag: &str, files: &[&str]) -> PathBuf {
        let d = std::env::temp_dir().join(format!("p8test-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        for f in files {
            fs::write(d.join(f), b"x").unwrap();
        }
        d
    }

    #[test]
    fn exact_and_case_insensitive() {
        let d = tmp("a", &["PicoTetris-5.p8.png", "other.p8.png", "notes.txt"]);
        assert_eq!(list(&d).len(), 2);
        assert_eq!(resolve(&d, "picotetris-5"), Some(d.join("PicoTetris-5.p8.png")));
        assert_eq!(resolve(&d, "other.p8.png"), Some(d.join("other.p8.png")));
    }

    #[test]
    fn highest_revision_wins() {
        let d = tmp("b", &["game-2.p8.png", "game-10.p8.png", "game-3.p8.png", "gamer-99.p8.png"]);
        assert_eq!(resolve(&d, "game"), Some(d.join("game-10.p8.png")));
    }

    #[test]
    fn rejects_traversal_and_missing() {
        let d = tmp("c", &["a-1.p8.png"]);
        assert_eq!(resolve(&d, "../a-1"), None);
        assert_eq!(resolve(&d, "a\\b"), None);
        assert_eq!(resolve(&d, ".hidden"), None);
        assert_eq!(resolve(&d, ""), None);
        assert_eq!(resolve(&d, "nope"), None);
    }

    #[test]
    fn safe_ids() {
        assert_eq!(safe_id("C:\\x\\cab_ride-7.p8.png").as_deref(), Some("cab_ride-7"));
        assert_eq!(safe_id("../../evil.PNG").as_deref(), Some("evil"));
        assert_eq!(safe_id(".p8.png"), None);
        assert_eq!(safe_id("é-1.p8.png").as_deref(), Some("é-1"));
    }
}
