//! 随手记：一块可以调透明度的桌面待办窗。
//!
//! 窗口形态参考 floating-todo（无边框、透明、置顶、透明度只铺在底色上）。
//! 自绘标题栏参考 egui 的 custom_window_frame。渲染固定用 Glow：
//! Windows 上 wgpu 经常把透明窗口画成黑底，或者连文字一起变淡。

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;

use eframe::egui::{self, Align, Color32, CursorIcon, Id, Layout, RichText, Stroke, Vec2};
use suishouji::AppState;

fn main() -> eframe::Result {
    let state = suishouji::load();
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("随手记")
        .with_inner_size([
            state.window.w.unwrap_or(340.0),
            state.window.h.unwrap_or(520.0),
        ])
        .with_min_inner_size([280.0, 240.0])
        .with_decorations(false)
        .with_transparent(true)
        .with_resizable(true)
        .with_window_level(if state.always_on_top {
            egui::WindowLevel::AlwaysOnTop
        } else {
            egui::WindowLevel::Normal
        });
    if let (Some(x), Some(y)) = (state.window.x, state.window.y) {
        viewport = viewport.with_position(egui::pos2(x, y));
    }

    let options = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };

    eframe::run_native(
        "随手记",
        options,
        Box::new(move |cc| {
            install_cjk(&cc.egui_ctx);
            configure_style(&cc.egui_ctx);
            Ok(Box::new(SuishoujiApp::new(state)))
        }),
    )
}

fn install_cjk(ctx: &egui::Context) {
    let Some(font) = suishouji::load_system_font() else {
        eprintln!(
            "随手记：没有找到中文字体，界面文字可能显示为方框。可以设置 SUISHOUJI_FONT 指向 .ttf 或 .ttc。"
        );
        return;
    };
    let mut data = egui::FontData::from_owned(font.bytes);
    data.index = font.index;
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert("cjk".to_owned(), Arc::new(data));
    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        fonts
            .families
            .entry(family)
            .or_default()
            .insert(0, "cjk".to_owned());
    }
    ctx.set_fonts(fonts);
}

fn configure_style(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style
        .text_styles
        .insert(egui::TextStyle::Body, egui::FontId::proportional(15.0));
    style
        .text_styles
        .insert(egui::TextStyle::Button, egui::FontId::proportional(14.0));
    style
        .text_styles
        .insert(egui::TextStyle::Small, egui::FontId::proportional(12.0));
    style
        .text_styles
        .insert(egui::TextStyle::Heading, egui::FontId::proportional(16.0));
    style.spacing.item_spacing = Vec2::new(8.0, 6.0);
    style.spacing.button_padding = Vec2::new(8.0, 4.0);
    ctx.set_style(style);
}

struct Look {
    panel: Color32,
    text: Color32,
    weak: Color32,
    accent: Color32,
    border: Color32,
    input: Color32,
}

fn look_of(state: &AppState) -> Look {
    let alpha = (state.opacity.clamp(0.25, 0.96) * 255.0).round() as u8;
    if state.dark {
        Look {
            panel: Color32::from_rgba_unmultiplied(24, 26, 31, alpha),
            text: Color32::from_rgb(236, 237, 241),
            weak: Color32::from_rgb(154, 160, 170),
            accent: Color32::from_rgb(110, 168, 254),
            border: Color32::from_rgba_unmultiplied(255, 255, 255, 36),
            input: Color32::from_rgba_unmultiplied(255, 255, 255, 18),
        }
    } else {
        Look {
            panel: Color32::from_rgba_unmultiplied(255, 252, 246, alpha),
            text: Color32::from_rgb(28, 30, 34),
            weak: Color32::from_rgb(110, 112, 120),
            accent: Color32::from_rgb(37, 99, 235),
            border: Color32::from_rgba_unmultiplied(40, 30, 10, 40),
            input: Color32::from_rgba_unmultiplied(0, 0, 0, 16),
        }
    }
}

struct SuishoujiApp {
    state: AppState,
    look: Look,
    editing: Option<u64>,
    edit_text: String,
    focus_edit: bool,
    scroll_to_top: bool,
    apply_level: bool,
    geometry_dirty: bool,
    save_error: Option<String>,
}

impl SuishoujiApp {
    fn new(state: AppState) -> Self {
        let look = look_of(&state);
        Self {
            state,
            look,
            editing: None,
            edit_text: String::new(),
            focus_edit: false,
            scroll_to_top: false,
            apply_level: true,
            geometry_dirty: false,
            save_error: None,
        }
    }

    fn persist_now(&mut self) {
        self.geometry_dirty = false;
        match suishouji::save(&self.state) {
            Ok(()) => self.save_error = None,
            Err(err) => self.save_error = Some(err.to_string()),
        }
    }

    fn apply_visuals(&self, ctx: &egui::Context) {
        let mut visuals = if self.state.dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        visuals.override_text_color = Some(self.look.text);
        visuals.window_fill = Color32::TRANSPARENT;
        visuals.panel_fill = Color32::TRANSPARENT;
        visuals.extreme_bg_color = self.look.input;
        visuals.faint_bg_color = self.look.input;
        visuals.code_bg_color = self.look.input;
        visuals.selection.bg_fill = self.look.accent;
        visuals.selection.stroke.color = Color32::WHITE;
        visuals.hyperlink_color = self.look.accent;
        visuals.window_stroke = Stroke::new(1.0_f32, self.look.border);
        for widget in [
            &mut visuals.widgets.noninteractive,
            &mut visuals.widgets.inactive,
        ] {
            widget.bg_fill = self.look.input;
            widget.fg_stroke.color = self.look.text;
            widget.bg_stroke = Stroke::new(1.0_f32, self.look.border);
        }
        visuals.widgets.hovered.bg_fill = if self.state.dark {
            Color32::from_rgba_unmultiplied(255, 255, 255, 28)
        } else {
            Color32::from_rgba_unmultiplied(0, 0, 0, 18)
        };
        visuals.widgets.hovered.fg_stroke.color = self.look.text;
        visuals.widgets.active.bg_fill = self.look.accent;
        visuals.widgets.active.fg_stroke.color = Color32::WHITE;
        visuals.widgets.open.bg_fill = self.look.input;
        visuals.widgets.open.fg_stroke.color = self.look.text;
        ctx.set_visuals(visuals);
    }

    fn title_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().button_padding = Vec2::new(8.0, 2.0);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let close = ui
                    .add(egui::Button::new(RichText::new("×").size(16.0)).frame(false))
                    .on_hover_text("关闭");
                if close.clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
                let minimize = ui
                    .add(egui::Button::new(RichText::new("—").size(16.0)).frame(false))
                    .on_hover_text("最小化");
                if minimize.clicked() {
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                }

                let mut drag_size = ui.available_size_before_wrap();
                drag_size.y = drag_size.y.max(24.0);
                let drag = ui.allocate_response(drag_size, egui::Sense::drag());
                if drag.hovered() || drag.dragged() {
                    ui.ctx().set_cursor_icon(CursorIcon::Grab);
                }
                if drag.drag_started() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
                ui.painter().text(
                    drag.rect.left_center() + Vec2::new(2.0, 0.0),
                    egui::Align2::LEFT_CENTER,
                    "随手记",
                    egui::FontId::proportional(16.0),
                    self.look.text,
                );
            });
        });
    }

    fn composer(&mut self, ui: &mut egui::Ui) {
        let mut submitted = false;
        // right_to_left 直接铺在竖向布局上时，会把整段剩余高度占满。
        // 先套一层 horizontal，输入行就只占一行。
        ui.horizontal(|ui| {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let add = ui
                    .add(
                        egui::Button::new(RichText::new("添加").color(Color32::WHITE))
                            .fill(self.look.accent),
                    )
                    .on_hover_text("记下来");
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.state.draft)
                        .hint_text("记一件事，回车添加")
                        .desired_width(ui.available_width())
                        .id(Id::new("composer")),
                );
                let enter = ui.input(|input| input.key_pressed(egui::Key::Enter));
                let escape = ui.input(|input| input.key_pressed(egui::Key::Escape));
                if escape && response.has_focus() {
                    self.state.draft.clear();
                    self.persist_now();
                } else if response.changed() {
                    self.persist_now();
                }
                submitted =
                    add.clicked() || (enter && (response.lost_focus() || response.has_focus()));
            });
        });
        let draft = self.state.draft.clone();
        if submitted && self.state.add(&draft).is_some() {
            self.state.draft.clear();
            self.scroll_to_top = true;
            self.persist_now();
        }
    }

    fn list(&mut self, ui: &mut egui::Ui) {
        let rows: Vec<_> = self
            .state
            .todos
            .iter()
            .filter(|todo| !(self.state.hide_done && todo.done))
            .cloned()
            .collect();
        let hide_done = self.state.hide_done;
        let has_hidden = hide_done && self.state.done_count() > 0;
        let editing = self.editing;
        let mut edit_text = std::mem::take(&mut self.edit_text);
        let mut toggle = None;
        let mut delete = None;
        let mut start_edit = None;
        let mut commit = false;
        let mut cancel = false;
        let mut reorder = None;

        let list_height = ui.available_height().max(48.0);
        egui::ScrollArea::vertical()
            .max_height(list_height)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if self.scroll_to_top {
                    ui.scroll_to_cursor(Some(Align::TOP));
                    self.scroll_to_top = false;
                }
                if rows.is_empty() {
                    ui.add_space(18.0);
                    let message = if has_hidden {
                        "已完成的都先收起来了"
                    } else {
                        "还没有待办。\n上面写一条，回车就会记下。"
                    };
                    ui.label(RichText::new(message).color(self.look.weak));
                    return;
                }

                for todo in &rows {
                    let row = ui.horizontal(|ui| {
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui
                                .add(
                                    egui::Button::new(RichText::new("×").color(self.look.weak))
                                        .frame(false),
                                )
                                .on_hover_text("删除")
                                .clicked()
                            {
                                delete = Some(todo.id);
                            }
                            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                                let drag = ui
                                    .dnd_drag_source(
                                        Id::new(("todo-drag", todo.id)),
                                        todo.id,
                                        |ui| {
                                            let (rect, _response) = ui.allocate_exact_size(
                                                Vec2::new(12.0, 16.0),
                                                egui::Sense::hover(),
                                            );
                                            let painter = ui.painter();
                                            for index in 0..3 {
                                                painter.circle_filled(
                                                    rect.center()
                                                        + Vec2::new(
                                                            0.0,
                                                            (index as f32 - 1.0) * 4.0,
                                                        ),
                                                    1.4_f32,
                                                    self.look.weak,
                                                );
                                            }
                                        },
                                    )
                                    .response
                                    .on_hover_text("拖动排序");
                                let _ = drag;
                                let mut done = todo.done;
                                if ui.checkbox(&mut done, "").changed() {
                                    toggle = Some(todo.id);
                                }
                                if editing == Some(todo.id) {
                                    let edit_id = Id::new(("todo-edit", todo.id));
                                    let response = ui.add(
                                        egui::TextEdit::singleline(&mut edit_text)
                                            .id(edit_id)
                                            .desired_width(ui.available_width()),
                                    );
                                    let just_opened = self.focus_edit;
                                    if just_opened {
                                        ui.memory_mut(|mem| mem.request_focus(edit_id));
                                        self.focus_edit = false;
                                    }
                                    let escape =
                                        ui.input(|input| input.key_pressed(egui::Key::Escape));
                                    if escape {
                                        cancel = true;
                                    } else if response.lost_focus() && !just_opened {
                                        commit = true;
                                    }
                                } else {
                                    let text = if todo.done {
                                        RichText::new(&todo.text)
                                            .strikethrough()
                                            .color(self.look.weak)
                                    } else {
                                        RichText::new(&todo.text).color(self.look.text)
                                    };
                                    let label = ui
                                        .add(egui::Label::new(text).sense(egui::Sense::click()))
                                        .on_hover_text("双击修改");
                                    if label.double_clicked() {
                                        start_edit = Some(todo.id);
                                    }
                                }
                            });
                        });
                    });

                    let drop = ui.interact(
                        row.response.rect,
                        Id::new(("todo-drop", todo.id)),
                        egui::Sense::hover(),
                    );
                    if drop.dnd_hover_payload::<u64>().is_some() {
                        ui.painter().hline(
                            drop.rect.x_range(),
                            drop.rect.top(),
                            Stroke::new(2.0_f32, self.look.accent),
                        );
                    }
                    if let Some(payload) = drop.dnd_release_payload::<u64>() {
                        reorder = Some((*payload, todo.id));
                    }
                    ui.add_space(2.0);
                }
            });

        self.edit_text = edit_text;
        if let Some(id) = toggle {
            if self.state.toggle(id) {
                self.persist_now();
            }
        }
        if let Some(id) = delete {
            if self.editing == Some(id) {
                self.editing = None;
                self.edit_text.clear();
            }
            if self.state.delete(id) {
                self.persist_now();
            }
        }
        if let Some((from, to)) = reorder {
            if self.state.reorder(from, to) {
                self.persist_now();
            }
        }
        if cancel {
            self.editing = None;
            self.edit_text.clear();
        } else if commit {
            if let Some(id) = self.editing.take() {
                let text = std::mem::take(&mut self.edit_text);
                if self.state.rename(id, &text) {
                    self.persist_now();
                }
            }
        } else if let Some(id) = start_edit {
            if let Some(todo) = self.state.todos.iter().find(|todo| todo.id == id) {
                self.edit_text = todo.text.clone();
                self.editing = Some(id);
                self.focus_edit = true;
            }
        }
    }

    fn footer(&mut self, ui: &mut egui::Ui) {
        ui.separator();
        ui.horizontal(|ui| {
            let mut on_top = self.state.always_on_top;
            if ui
                .checkbox(&mut on_top, "置顶")
                .on_hover_text("保持在其他窗口上面")
                .changed()
            {
                self.state.always_on_top = on_top;
                self.apply_level = true;
                self.persist_now();
            }
            let mut dark = self.state.dark;
            if ui.checkbox(&mut dark, "深色").changed() {
                self.state.dark = dark;
                self.persist_now();
            }
            let mut hide_done = self.state.hide_done;
            if ui.checkbox(&mut hide_done, "隐藏已完成").changed() {
                self.state.hide_done = hide_done;
                self.persist_now();
            }
        });
        ui.horizontal(|ui| {
            ui.label("透明度");
            ui.spacing_mut().slider_width = (ui.available_width() - 52.0).max(80.0);
            let mut opacity = self.state.opacity;
            let slider = ui.add(
                egui::Slider::new(&mut opacity, 0.25..=0.96)
                    .show_value(false)
                    .trailing_fill(true),
            );
            if slider.changed() {
                self.state.set_opacity(opacity);
            }
            // 拖动过程中先改画面，松手再写盘。延后一帧的重绘在这台事件循环里不可靠。
            if slider.drag_stopped() || (slider.changed() && !slider.dragged()) {
                self.persist_now();
            }
            ui.label(
                RichText::new(format!("{:.0}%", self.state.opacity * 100.0)).color(self.look.weak),
            );
        });
        ui.horizontal(|ui| {
            let open = self.state.open_count();
            let done = self.state.done_count();
            ui.label(format!("{open} 未完成"));
            if done > 0 {
                ui.label(RichText::new(format!("{done} 已完成")).color(self.look.weak));
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .add_enabled(done > 0, egui::Button::new("清除已完成"))
                    .clicked()
                {
                    if self.state.clear_done() > 0 {
                        self.editing = None;
                        self.persist_now();
                    }
                }
            });
        });
        if let Some(err) = &self.save_error {
            ui.label(
                RichText::new(format!("保存失败：{err}")).color(Color32::from_rgb(220, 90, 90)),
            );
        }
    }

    fn resize_handles(&self, ui: &mut egui::Ui) {
        let rect = ui.ctx().screen_rect();
        let edge = 6.0;
        let corner = 16.0;
        let handles = [
            (
                "n",
                egui::Rect::from_min_max(
                    rect.left_top() + Vec2::new(corner, 0.0),
                    egui::pos2(rect.right() - corner, rect.top() + edge),
                ),
                egui::ResizeDirection::North,
                CursorIcon::ResizeNorth,
            ),
            (
                "s",
                egui::Rect::from_min_max(
                    egui::pos2(rect.left() + corner, rect.bottom() - edge),
                    rect.right_bottom() - Vec2::new(corner, 0.0),
                ),
                egui::ResizeDirection::South,
                CursorIcon::ResizeSouth,
            ),
            (
                "w",
                egui::Rect::from_min_max(
                    rect.left_top() + Vec2::new(0.0, corner),
                    egui::pos2(rect.left() + edge, rect.bottom() - corner),
                ),
                egui::ResizeDirection::West,
                CursorIcon::ResizeWest,
            ),
            (
                "e",
                egui::Rect::from_min_max(
                    egui::pos2(rect.right() - edge, rect.top() + corner),
                    rect.right_bottom() - Vec2::new(0.0, corner),
                ),
                egui::ResizeDirection::East,
                CursorIcon::ResizeEast,
            ),
            (
                "nw",
                egui::Rect::from_min_size(rect.left_top(), Vec2::splat(corner)),
                egui::ResizeDirection::NorthWest,
                CursorIcon::ResizeNorthWest,
            ),
            (
                "ne",
                egui::Rect::from_min_size(
                    rect.right_top() - Vec2::new(corner, 0.0),
                    Vec2::splat(corner),
                ),
                egui::ResizeDirection::NorthEast,
                CursorIcon::ResizeNorthEast,
            ),
            (
                "sw",
                egui::Rect::from_min_size(
                    rect.left_bottom() - Vec2::new(0.0, corner),
                    Vec2::splat(corner),
                ),
                egui::ResizeDirection::SouthWest,
                CursorIcon::ResizeSouthWest,
            ),
            (
                "se",
                egui::Rect::from_min_size(
                    rect.right_bottom() - Vec2::splat(corner),
                    Vec2::splat(corner),
                ),
                egui::ResizeDirection::SouthEast,
                CursorIcon::ResizeSouthEast,
            ),
        ];

        let previous_clip = ui.clip_rect();
        ui.set_clip_rect(rect);
        for (name, handle, direction, cursor) in handles {
            if handle.width() <= 0.0 || handle.height() <= 0.0 {
                continue;
            }
            let response = ui.interact(handle, Id::new(("resize", name)), egui::Sense::drag());
            if response.hovered() || response.dragged() {
                ui.ctx().set_cursor_icon(cursor);
            }
            if response.drag_started() {
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::BeginResize(direction));
            }
        }
        ui.set_clip_rect(previous_clip);
    }

    fn sync_geometry(&mut self, ctx: &egui::Context) {
        let (outer, inner) =
            ctx.input(|input| (input.viewport().outer_rect, input.viewport().inner_rect));
        let mut changed = false;
        if let Some(rect) = outer {
            changed |= assign_if_far(&mut self.state.window.x, rect.min.x);
            changed |= assign_if_far(&mut self.state.window.y, rect.min.y);
        }
        if let Some(rect) = inner {
            changed |= assign_if_far(&mut self.state.window.w, rect.width());
            changed |= assign_if_far(&mut self.state.window.h, rect.height());
        }
        if changed {
            self.geometry_dirty = true;
        }
        let pointer_down = ctx.input(|input| input.pointer.any_down());
        if self.geometry_dirty && !pointer_down {
            self.persist_now();
        }
    }
}

fn assign_if_far(slot: &mut Option<f32>, value: f32) -> bool {
    if !value.is_finite() {
        return false;
    }
    if slot.is_some_and(|old| (old - value).abs() <= 0.5) {
        return false;
    }
    *slot = Some(value);
    true
}

impl eframe::App for SuishoujiApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.look = look_of(&self.state);
        self.apply_visuals(ctx);
        if self.apply_level {
            ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
                if self.state.always_on_top {
                    egui::WindowLevel::AlwaysOnTop
                } else {
                    egui::WindowLevel::Normal
                },
            ));
            self.apply_level = false;
        }

        let frame = egui::Frame::new()
            .fill(self.look.panel)
            .stroke(Stroke::new(1.0_f32, self.look.border))
            .corner_radius(14)
            .inner_margin(egui::Margin::same(12));
        egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
            self.title_bar(ui);
            ui.add_space(4.0);
            let full = ui.available_rect_before_wrap();
            let footer_h = 108.0;
            let footer_top = (full.bottom() - footer_h).max(full.top());
            let footer_rect =
                egui::Rect::from_min_max(egui::pos2(full.left(), footer_top), full.right_bottom());
            let body_rect =
                egui::Rect::from_min_max(full.left_top(), egui::pos2(full.right(), footer_top));
            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(body_rect), |ui| {
                self.composer(ui);
                ui.add_space(6.0);
                self.list(ui);
            });
            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(footer_rect), |ui| {
                self.footer(ui);
            });
            self.resize_handles(ui);
        });

        self.sync_geometry(ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.persist_now();
    }
}
