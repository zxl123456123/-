//! 待办本身：新增、勾选、改字、排序、清除。
//!
//! 纸条是今天的桌面：没做完的会留到第二天，昨天已完成的不再占地方。
//! 透明度只描述面板底色，文字始终不透明，避免整窗一起变淡。

use chrono::{Datelike, TimeZone};
use serde::{Deserialize, Serialize};

const NOTE_CHARS: usize = 2_000;
const MIN_OPACITY: f32 = 0.25;
const MAX_OPACITY: f32 = 0.96;

fn default_opacity() -> f32 {
    0.58
}

fn default_true() -> bool {
    true
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

fn trim_note(text: &str) -> &str {
    let text = text.trim();
    let mut end = text.len();
    for (count, (index, _)) in text.char_indices().enumerate() {
        if count == NOTE_CHARS {
            end = index;
            break;
        }
    }
    &text[..end]
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Todo {
    pub id: u64,
    pub text: String,
    #[serde(default)]
    pub done: bool,
    #[serde(default)]
    pub created_ms: i64,
    #[serde(default)]
    pub done_ms: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WindowGeom {
    #[serde(default)]
    pub x: Option<f32>,
    #[serde(default)]
    pub y: Option<f32>,
    #[serde(default)]
    pub w: Option<f32>,
    #[serde(default)]
    pub h: Option<f32>,
}

impl Default for WindowGeom {
    fn default() -> Self {
        Self {
            x: None,
            y: None,
            w: None,
            h: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AppState {
    #[serde(default = "default_opacity")]
    pub opacity: f32,
    #[serde(default = "default_true")]
    pub always_on_top: bool,
    #[serde(default)]
    pub dark: bool,
    #[serde(default)]
    pub hide_done: bool,
    #[serde(default)]
    pub draft: String,
    #[serde(default)]
    pub window: WindowGeom,
    #[serde(default)]
    pub next_id: u64,
    #[serde(default)]
    pub todos: Vec<Todo>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            opacity: default_opacity(),
            always_on_top: true,
            dark: false,
            hide_done: false,
            draft: String::new(),
            window: WindowGeom::default(),
            next_id: 1,
            todos: Vec::new(),
        }
    }
}

impl AppState {
    pub fn normalize(&mut self) {
        self.opacity = if self.opacity.is_finite() {
            self.opacity.clamp(MIN_OPACITY, MAX_OPACITY)
        } else {
            default_opacity()
        };
        self.draft = trim_note(&self.draft).to_string();
        self.window.x = self.window.x.filter(|value| plausible_pos(*value));
        self.window.y = self.window.y.filter(|value| plausible_pos(*value));
        self.window.w = self.window.w.filter(|value| plausible_size(*value));
        self.window.h = self.window.h.filter(|value| plausible_size(*value));

        let mut seen = std::collections::BTreeSet::new();
        let mut max_id = 0u64;
        for todo in &mut self.todos {
            todo.text = trim_note(&todo.text).to_string();
            if todo.id == 0 || !seen.insert(todo.id) {
                todo.id = 0;
            } else {
                max_id = max_id.max(todo.id);
            }
        }
        self.todos.retain(|todo| !todo.text.is_empty());
        for todo in &mut self.todos {
            if todo.id == 0 {
                max_id = max_id.saturating_add(1);
                todo.id = max_id;
            }
        }
        if self.next_id <= max_id {
            self.next_id = max_id.saturating_add(1);
        }
        if self.next_id == 0 {
            self.next_id = 1;
        }
        let now = now_ms();
        for todo in &mut self.todos {
            if todo.created_ms == 0 {
                todo.created_ms = now;
            }
            if todo.done && todo.done_ms == 0 {
                todo.done_ms = now;
            }
        }
        self.sweep_done_before(today_day_key());
    }

    pub fn add(&mut self, text: &str) -> Option<u64> {
        let text = trim_note(text);
        if text.is_empty() {
            return None;
        }
        let id = self.next_id.max(1);
        self.next_id = id.saturating_add(1);
        self.todos.insert(
            0,
            Todo {
                id,
                text: text.to_string(),
                done: false,
                created_ms: now_ms(),
                done_ms: 0,
            },
        );
        Some(id)
    }

    pub fn toggle(&mut self, id: u64) -> bool {
        let Some(todo) = self.todos.iter_mut().find(|todo| todo.id == id) else {
            return false;
        };
        todo.done = !todo.done;
        todo.done_ms = if todo.done { now_ms() } else { 0 };
        true
    }

    pub fn rename(&mut self, id: u64, text: &str) -> bool {
        let text = trim_note(text);
        if text.is_empty() {
            return false;
        }
        let Some(todo) = self.todos.iter_mut().find(|todo| todo.id == id) else {
            return false;
        };
        if todo.text == text {
            return false;
        }
        todo.text = text.to_string();
        true
    }

    pub fn delete(&mut self, id: u64) -> bool {
        let before = self.todos.len();
        self.todos.retain(|todo| todo.id != id);
        self.todos.len() != before
    }

    pub fn clear_done(&mut self) -> usize {
        let before = self.todos.len();
        self.todos.retain(|todo| !todo.done);
        before - self.todos.len()
    }

    /// 把 `from_id` 挪到 `to_id` 原先占据的那一格。
    pub fn reorder(&mut self, from_id: u64, to_id: u64) -> bool {
        if from_id == to_id {
            return false;
        }
        let Some(from) = self.todos.iter().position(|todo| todo.id == from_id) else {
            return false;
        };
        let Some(to) = self.todos.iter().position(|todo| todo.id == to_id) else {
            return false;
        };
        let item = self.todos.remove(from);
        self.todos.insert(to, item);
        true
    }

    pub fn set_opacity(&mut self, opacity: f32) {
        self.opacity = if opacity.is_finite() {
            opacity.clamp(MIN_OPACITY, MAX_OPACITY)
        } else {
            default_opacity()
        };
    }

    pub fn open_count(&self) -> usize {
        self.todos.iter().filter(|todo| !todo.done).count()
    }

    pub fn done_count(&self) -> usize {
        self.todos.len() - self.open_count()
    }

    pub fn leftover_count(&self) -> usize {
        let today = today_day_key();
        self.todos
            .iter()
            .filter(|todo| !todo.done && day_key(todo.created_ms) < today)
            .count()
    }

    /// 昨天及更早勾掉的事项不再占今天的纸。没做完的留下。
    pub fn sweep_done_before(&mut self, today: i32) -> usize {
        let before = self.todos.len();
        self.todos.retain(|todo| {
            if !todo.done {
                return true;
            }
            day_key(todo.done_ms) >= today
        });
        before - self.todos.len()
    }
}

pub fn today_day_key() -> i32 {
    chrono::Local::now().date_naive().num_days_from_ce()
}

pub fn today_label() -> String {
    let date = chrono::Local::now().date_naive();
    format!("{}月{}日", date.month(), date.day())
}

fn day_key(ms: i64) -> i32 {
    chrono::DateTime::from_timestamp_millis(ms)
        .map(|utc| {
            chrono::Local
                .from_utc_datetime(&utc.naive_utc())
                .date_naive()
                .num_days_from_ce()
        })
        .unwrap_or(0)
}

fn plausible_pos(value: f32) -> bool {
    value.is_finite() && (-10_000.0..20_000.0).contains(&value)
}

fn plausible_size(value: f32) -> bool {
    value.is_finite() && (160.0..4_000.0).contains(&value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_trims_and_puts_newest_first() {
        let mut state = AppState::default();
        assert!(state.add("   ").is_none());
        let first = state.add(" 买牛奶 ").unwrap();
        let second = state.add("回邮件").unwrap();
        assert_eq!(state.todos[0].id, second);
        assert_eq!(state.todos[0].text, "回邮件");
        assert_eq!(state.todos[1].id, first);
        assert_eq!(state.todos[1].text, "买牛奶");
        assert!(!state.todos[0].done);
    }

    #[test]
    fn toggle_rename_delete_and_clear() {
        let mut state = AppState::default();
        let id = state.add("写周报").unwrap();
        assert!(state.toggle(id));
        assert!(state.todos[0].done);
        assert!(!state.rename(id, "   "));
        assert!(state.rename(id, "写周报草稿"));
        assert_eq!(state.todos[0].text, "写周报草稿");
        state.add("还在做");
        assert_eq!(state.clear_done(), 1);
        assert_eq!(state.todos.len(), 1);
        assert!(state.delete(state.todos[0].id));
        assert!(state.todos.is_empty());
    }

    #[test]
    fn reorder_moves_item_into_the_target_slot() {
        let mut state = AppState::default();
        let a = state.add("A").unwrap();
        let b = state.add("B").unwrap();
        let c = state.add("C").unwrap();
        // 当前顺序是 C B A。把 C 放到 A 的位置。
        assert!(state.reorder(c, a));
        assert_eq!(
            state.todos.iter().map(|todo| todo.id).collect::<Vec<_>>(),
            vec![b, a, c]
        );
        assert!(!state.reorder(a, a));
        assert!(!state.reorder(a, 999));
    }

    #[test]
    fn opacity_and_note_length_are_clamped() {
        let mut state = AppState::default();
        state.set_opacity(0.01);
        assert_eq!(state.opacity, MIN_OPACITY);
        state.set_opacity(2.0);
        assert_eq!(state.opacity, MAX_OPACITY);
        state.set_opacity(f32::NAN);
        assert_eq!(state.opacity, default_opacity());

        let long = "字".repeat(NOTE_CHARS + 10);
        state.add(&long);
        assert_eq!(state.todos[0].text.chars().count(), NOTE_CHARS);
    }

    #[test]
    fn normalize_repairs_missing_and_duplicate_ids() {
        let mut state = AppState {
            opacity: 4.0,
            next_id: 0,
            window: WindowGeom {
                x: Some(f32::INFINITY),
                y: Some(12.0),
                w: Some(10.0),
                h: Some(480.0),
            },
            todos: vec![
                Todo {
                    id: 3,
                    text: "  留下  ".into(),
                    done: false,
                    created_ms: 0,
                    done_ms: 0,
                },
                Todo {
                    id: 3,
                    text: "重复".into(),
                    done: true,
                    created_ms: 0,
                    done_ms: now_ms(),
                },
                Todo {
                    id: 0,
                    text: "   ".into(),
                    done: false,
                    created_ms: 0,
                    done_ms: 0,
                },
            ],
            ..AppState::default()
        };
        state.normalize();
        assert_eq!(state.opacity, MAX_OPACITY);
        assert_eq!(state.window.x, None);
        assert_eq!(state.window.y, Some(12.0));
        assert_eq!(state.window.w, None);
        assert_eq!(state.todos.len(), 2);
        assert_eq!(state.todos[0].text, "留下");
        assert_ne!(state.todos[0].id, state.todos[1].id);
        assert!(state.next_id > state.todos.iter().map(|todo| todo.id).max().unwrap());
    }

    #[test]
    fn yesterday_done_is_swept_open_leftovers_stay() {
        let today = today_day_key();
        let mut state = AppState::default();
        let open = state.add("以前留下的").unwrap();
        let done = state.add("昨天勾掉的").unwrap();
        state.todos[0].created_ms = 1;
        state.todos[1].created_ms = 1;
        assert!(state.toggle(done));
        state
            .todos
            .iter_mut()
            .find(|todo| todo.id == done)
            .unwrap()
            .done_ms = 1;
        assert_eq!(state.sweep_done_before(today), 1);
        assert_eq!(state.todos.len(), 1);
        assert_eq!(state.todos[0].id, open);
        assert_eq!(state.leftover_count(), 1);
    }

    #[test]
    fn normalize_stamps_missing_created_ms_as_today() {
        let mut state = AppState {
            todos: vec![Todo {
                id: 1,
                text: "旧数据".into(),
                done: false,
                created_ms: 0,
                done_ms: 0,
            }],
            next_id: 2,
            ..AppState::default()
        };
        state.normalize();
        assert!(state.todos[0].created_ms > 0);
        assert_eq!(state.leftover_count(), 0);
    }

    #[test]
    fn done_today_stays_on_the_paper() {
        let mut state = AppState::default();
        let id = state.add("今天勾掉").unwrap();
        assert!(state.toggle(id));
        assert_eq!(state.sweep_done_before(today_day_key()), 0);
        assert_eq!(state.todos.len(), 1);
        assert!(state.todos[0].done);
    }
}
