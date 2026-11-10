//! 本地 JSON。不走浏览器存储，关掉窗口也不会丢。
//!
//! 先写临时文件再替换，避免写到一半断电留下半截 JSON。

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::model::AppState;

pub fn notes_path() -> PathBuf {
    dirs::data_local_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("suishouji")
        .join("notes.json")
}

pub fn load() -> AppState {
    let path = notes_path();
    match load_from(&path) {
        Ok(state) => state,
        Err(err) if err.kind() == io::ErrorKind::NotFound => AppState::default(),
        Err(err) => {
            let backup = path.with_extension("json.bak");
            let copied = fs::copy(&path, &backup);
            eprintln!(
                "随手记：读取 {} 失败（{err}）。{}",
                path.display(),
                match copied {
                    Ok(_) => format!("原文件已复制到 {}。", backup.display()),
                    Err(copy_err) => format!("原文件未改动（备份失败：{copy_err}）。"),
                }
            );
            AppState::default()
        }
    }
}

pub fn save(state: &AppState) -> io::Result<()> {
    save_to(&notes_path(), state)
}

pub fn load_from(path: &Path) -> io::Result<AppState> {
    let text = fs::read_to_string(path)?;
    let mut state: AppState = serde_json::from_str(&text)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    state.normalize();
    Ok(state)
}

pub fn save_to(path: &Path, state: &AppState) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut json = serde_json::to_string_pretty(state)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    json.push('\n');
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, json)?;
    replace_file(&tmp, path)
}

fn replace_file(tmp: &Path, path: &Path) -> io::Result<()> {
    match fs::rename(tmp, path) {
        Ok(()) => Ok(()),
        Err(err) => {
            if path.exists() {
                fs::remove_file(path)?;
                fs::rename(tmp, path)
            } else {
                let _ = fs::remove_file(tmp);
                Err(err)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "suishouji-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn roundtrip_keeps_todos_and_opacity() {
        let dir = scratch_dir();
        let path = dir.join("notes.json");
        let mut state = AppState::default();
        state.set_opacity(0.4);
        state.always_on_top = false;
        state.add("买牛奶");
        let id = state.todos[0].id;
        state.toggle(id);
        save_to(&path, &state).unwrap();
        assert!(!path.with_extension("json.tmp").exists());

        let loaded = load_from(&path).unwrap();
        assert!((loaded.opacity - 0.4).abs() < f32::EPSILON);
        assert!(!loaded.always_on_top);
        assert_eq!(loaded.todos.len(), 1);
        assert!(loaded.todos[0].done);
        assert_eq!(loaded.todos[0].text, "买牛奶");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn old_file_without_new_fields_still_loads() {
        let dir = scratch_dir();
        let path = dir.join("notes.json");
        fs::write(&path, r#"{"todos":[{"id":7,"text":"旧数据"}]}"#).unwrap();
        let state = load_from(&path).unwrap();
        assert_eq!(state.todos[0].text, "旧数据");
        assert!((state.opacity - 0.58).abs() < f32::EPSILON);
        assert!(state.always_on_top);
        assert!(!state.dark);
        assert!(state.next_id > 7);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn corrupt_file_is_an_invalid_data_error() {
        let dir = scratch_dir();
        let path = dir.join("notes.json");
        fs::write(&path, "{").unwrap();
        let err = load_from(&path).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        let _ = fs::remove_dir_all(&dir);
    }
}
