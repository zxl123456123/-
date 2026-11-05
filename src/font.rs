//! 系统里的中文字体。默认英文字体画不出「随手记」。
//!
//! Windows 优先微软雅黑，其它系统再找 Noto / 文泉驿 / 系统回退字体。
//! 不把字体打进二进制，安装包才能保持轻。

use std::path::{Path, PathBuf};
use std::process::Command;

pub struct SystemFont {
    pub bytes: Vec<u8>,
    pub index: u32,
    pub path: PathBuf,
}

pub fn load_system_font() -> Option<SystemFont> {
    let mut paths = candidate_paths();
    if let Some(path) = fontconfig_path() {
        if !paths.iter().any(|existing| existing == &path) {
            paths.push(path);
        }
    }
    paths.into_iter().find_map(|path| try_load(&path))
}

fn candidate_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(custom) = std::env::var_os("SUISHOUJI_FONT") {
        paths.push(PathBuf::from(custom));
    }
    paths.extend([
        PathBuf::from(r"C:\Windows\Fonts\msyh.ttc"),
        PathBuf::from(r"C:\Windows\Fonts\msyh.ttf"),
        PathBuf::from(r"C:\Windows\Fonts\msyhl.ttc"),
        PathBuf::from(r"C:\Windows\Fonts\simhei.ttf"),
        PathBuf::from(r"C:\Windows\Fonts\simsun.ttc"),
        PathBuf::from("/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf"),
        PathBuf::from("/usr/share/fonts/truetype/wqy/wqy-microhei.ttc"),
        PathBuf::from("/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc"),
        PathBuf::from("/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc"),
        PathBuf::from("/System/Library/Fonts/PingFang.ttc"),
        PathBuf::from("/System/Library/Fonts/Hiragino Sans GB.ttc"),
        PathBuf::from("/System/Library/Fonts/STHeiti Light.ttc"),
    ]);
    paths
}

fn try_load(path: &Path) -> Option<SystemFont> {
    let bytes = std::fs::read(path).ok()?;
    let index =
        (0..6).find(|index| ab_glyph::FontRef::try_from_slice_and_index(&bytes, *index).is_ok())?;
    Some(SystemFont {
        bytes,
        index,
        path: path.to_path_buf(),
    })
}

#[cfg(unix)]
fn fontconfig_path() -> Option<PathBuf> {
    let output = Command::new("fc-list")
        .args([":lang=zh", "file"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    for line in text.lines() {
        let path = line.split(':').next()?.trim();
        if path.ends_with(".ttf") || path.ends_with(".otf") || path.ends_with(".ttc") {
            return Some(PathBuf::from(path));
        }
    }
    None
}

#[cfg(not(unix))]
fn fontconfig_path() -> Option<PathBuf> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_cjk_font_on_this_machine_when_one_exists() {
        let Some(font) = load_system_font() else {
            return;
        };
        assert!(!font.bytes.is_empty());
        assert!(ab_glyph::FontRef::try_from_slice_and_index(&font.bytes, font.index).is_ok());
    }
}
