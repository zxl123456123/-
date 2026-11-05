//! 随手记的数据层。界面在二进制里，待办规则和存盘可以不打开窗口就测。

mod font;
mod model;
mod store;

pub use font::{load_system_font, SystemFont};
pub use model::AppState;
pub use store::{load, load_from, notes_path, save, save_to};
