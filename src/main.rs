//! 随手记：一张可以调透明度的桌面纸条。
//!
//! 窗口行为参考 floating-todo（无边框、透明、置顶、透明度只铺在纸上）。
//! 版式参考 Things 的纸面，以及 Windows 便笺 / Glass Notes：
//! 透明度、深浅色收在标题栏菜单里，不铺在待办下面。
//! 渲染固定用 Glow：Windows 上 wgpu 经常把透明窗口画成黑底，或者连文字一起变淡。

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;

use eframe::egui::{self, Align, Color32, CursorIcon, Id, Layout, RichText, Stroke, Vec2};
use suishouji::AppState;

const MENU_WIDTH: f32 = 220.0;

fn main() -> eframe::Result {
    let state = suishouji::load();
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("随手记")
        .with_inner_size([
            state.window.w.unwrap_or(360.0),
            state.window.h.unwrap_or(480.0),
        ])
        .with_min_inner_size([280.0, 220.0])
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
        .insert(egui::TextStyle::Body, egui::FontId::proportional(16.0));
    style
        .text_styles
        .insert(egui::TextStyle::Button, egui::FontId::proportional(14.0));
    style
        .text_styles
        .insert(egui::TextStyle::Small, egui::FontId::proportional(12.0));
    style
        .text_styles
        .insert(egui::TextStyle::Heading, egui::FontId::proportional(16.0));
    style.spacing.item_spacing = Vec2::new(8.0, 4.0);
    style.spacing.button_padding = Vec2::new(8.0, 4.0);
    style.spacing.interact_size.y = 20.0;
    style.spacing.scroll.floating = true;
    style.spacing.scroll.bar_width = 8.0;
    style.spacing.scroll.floating_width = 3.0;
    style.spacing.scroll.foreground_color = false;
    style.interaction.selectable_labels = false;
    ctx.set_style(style);
}

#[derive(Clone, Copy)]
struct Look {
    panel: Color32,
    text: Color32,
    faint: Color32,
    accent: Color32,
    line: Color32,
    hover: Color32,
    danger: Color32,
    dark: bool,
}

fn look_of(state: &AppState) -> Look {
    let alpha = (state.opacity.clamp(0.25, 0.96) * 255.0).round() as u8;
    if state.dark {
        Look {
            panel: Color32::from_rgba_unmultiplied(28, 28, 30, alpha),
            text: Color32::from_rgb(245, 245, 247),
            faint: Color32::from_rgb(152, 152, 158),
            accent: Color32::from_rgb(110, 176, 255),
            line: Color32::from_rgba_unmultiplied(255, 255, 255, 24),
            hover: Color32::from_rgba_unmultiplied(255, 255, 255, 16),
            danger: Color32::from_rgb(255, 114, 105),
            dark: true,
        }
    } else {
        Look {
            panel: Color32::from_rgba_unmultiplied(250, 249, 246, alpha),
            text: Color32::from_rgb(28, 28, 30),
            faint: Color32::from_rgb(142, 142, 147),
            accent: Color32::from_rgb(47, 128, 237),
            line: Color32::from_rgba_unmultiplied(28, 28, 30, 20),
            hover: Color32::from_rgba_unmultiplied(28, 28, 30, 12),
            danger: Color32::from_rgb(214, 64, 64),
            dark: false,
        }
    }
}

struct SuishoujiApp {
    state: AppState,
    look: Look,
    editing: Option<u64>,
    edit_text: String,
    focus_edit: bool,
    focus_composer: bool,
    scroll_to_top: bool,
    apply_level: bool,
    geometry_dirty: bool,
    save_error: Option<String>,
    menu_open: bool,
    menu_button: egui::Rect,
    menu_rect: egui::Rect,
    ignore_menu_click: bool,
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
            focus_composer: true,
            scroll_to_top: false,
            apply_level: true,
            geometry_dirty: false,
            save_error: None,
            menu_open: false,
            menu_button: egui::Rect::NOTHING,
            menu_rect: egui::Rect::NOTHING,
            ignore_menu_click: false,
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
        visuals.extreme_bg_color = Color32::TRANSPARENT;
        visuals.faint_bg_color = Color32::TRANSPARENT;
        visuals.code_bg_color = Color32::TRANSPARENT;
        visuals.selection.bg_fill = self.look.accent;
        visuals.selection.stroke.color = Color32::WHITE;
        visuals.hyperlink_color = self.look.accent;
        visuals.window_stroke = Stroke::NONE;
        visuals.slider_trailing_fill = true;
        let radius = egui::CornerRadius::same(8);
        for widget in [
            &mut visuals.widgets.noninteractive,
            &mut visuals.widgets.inactive,
            &mut visuals.widgets.hovered,
            &mut visuals.widgets.active,
            &mut visuals.widgets.open,
        ] {
            widget.corner_radius = radius;
            widget.bg_stroke = Stroke::NONE;
            widget.fg_stroke.color = self.look.text;
            widget.expansion = 0.0;
        }
        visuals.widgets.noninteractive.bg_fill = Color32::TRANSPARENT;
        visuals.widgets.inactive.bg_fill = Color32::TRANSPARENT;
        visuals.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
        visuals.widgets.hovered.bg_fill = self.look.hover;
        visuals.widgets.hovered.weak_bg_fill = self.look.hover;
        visuals.widgets.hovered.fg_stroke.color = self.look.text;
        visuals.widgets.active.bg_fill = self.look.accent;
        visuals.widgets.active.fg_stroke.color = Color32::WHITE;
        visuals.widgets.open.bg_fill = Color32::TRANSPARENT;
        ctx.set_visuals(visuals);
    }

    fn title_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if icon_button(ui, "×", self.look.faint, Some(self.look.danger))
                    .on_hover_text("关闭")
                    .clicked()
                {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
                if icon_button(ui, "–", self.look.faint, None)
                    .on_hover_text("最小化")
                    .clicked()
                {
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                }
                let menu = more_button(ui, self.menu_open, &self.look);
                self.menu_button = menu.rect;
                if menu.clicked() {
                    self.menu_open = !self.menu_open;
                    self.ignore_menu_click = true;
                }
                if pin_button(ui, self.state.always_on_top, &self.look).clicked() {
                    self.state.always_on_top = !self.state.always_on_top;
                    self.apply_level = true;
                    self.persist_now();
                }

                let mut drag_size = ui.available_size_before_wrap();
                drag_size.y = drag_size.y.max(26.0);
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
                    egui::FontId::proportional(13.0),
                    self.look.faint,
                );
            });
        });
    }

    fn composer(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            // 和列表左侧的拖动点对齐，圆圈才能落在同一列。
            let _grip_column = ui.allocate_exact_size(Vec2::new(10.0, 18.0), egui::Sense::hover());
            let ready = !trim_view(&self.state.draft).is_empty();
            let add = add_mark(ui, ready, &self.look);
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.state.draft)
                    .hint_text("记一件事，回车")
                    .desired_width(ui.available_width())
                    .frame(false)
                    .id(Id::new("composer")),
            );
            if self.focus_composer {
                response.request_focus();
                self.focus_composer = false;
            }
            let enter = ui.input(|input| input.key_pressed(egui::Key::Enter));
            let escape = ui.input(|input| input.key_pressed(egui::Key::Escape));
            if escape && response.has_focus() && !self.menu_open {
                self.state.draft.clear();
                self.persist_now();
            } else if response.changed() {
                self.persist_now();
            }
            let submitted =
                add.clicked() || (enter && (response.lost_focus() || response.has_focus()));
            let draft = self.state.draft.clone();
            if submitted && self.state.add(&draft).is_some() {
                self.state.draft.clear();
                self.scroll_to_top = true;
                self.focus_composer = true;
                self.persist_now();
            }
        });
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
        let done = self.state.done_count();
        let look = self.look;
        let editing = self.editing;
        let mut edit_text = std::mem::take(&mut self.edit_text);
        let mut toggle = None;
        let mut delete = None;
        let mut start_edit = None;
        let mut commit = false;
        let mut cancel = false;
        let mut reorder = None;
        let mut toggle_hide = false;
        let mut clear_done = false;

        let list_height = ui.available_height().max(48.0);
        egui::ScrollArea::vertical()
            .max_height(list_height)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if self.scroll_to_top {
                    ui.scroll_to_cursor(Some(Align::TOP));
                    self.scroll_to_top = false;
                }
                if rows.is_empty() && done == 0 {
                    ui.add_space(36.0);
                    ui.label(RichText::new("记下一件就好").color(look.faint).size(15.0));
                    return;
                }

                for todo in &rows {
                    let row_id = Id::new(("todo-row-rect", todo.id));
                    let row_hot = ui
                        .data(|data| data.get_temp::<egui::Rect>(row_id))
                        .is_some_and(|rect| ui.rect_contains_pointer(rect));
                    let row = egui::Frame::new()
                        .fill(if row_hot {
                            look.hover
                        } else {
                            Color32::TRANSPARENT
                        })
                        .corner_radius(8)
                        .inner_margin(egui::Margin::symmetric(2, 6))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 8.0;
                                let grip_color = if row_hot { look.faint } else { look.line };
                                let _drag = ui
                                    .dnd_drag_source(
                                        Id::new(("todo-drag", todo.id)),
                                        todo.id,
                                        |ui| grip_dots(ui, grip_color),
                                    )
                                    .response
                                    .on_hover_text("拖动排序");
                                if circle_check(ui, todo.done, &look).clicked() {
                                    toggle = Some(todo.id);
                                }
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    if row_delete(ui, row_hot, &look).clicked() {
                                        delete = Some(todo.id);
                                    }
                                    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                                        if editing == Some(todo.id) {
                                            let edit_id = Id::new(("todo-edit", todo.id));
                                            let response = ui.add(
                                                egui::TextEdit::singleline(&mut edit_text)
                                                    .id(edit_id)
                                                    .frame(false)
                                                    .desired_width(ui.available_width()),
                                            );
                                            let just_opened = self.focus_edit;
                                            if just_opened {
                                                ui.memory_mut(|mem| mem.request_focus(edit_id));
                                                self.focus_edit = false;
                                            }
                                            let escape = ui.input(|input| {
                                                input.key_pressed(egui::Key::Escape)
                                            });
                                            if escape {
                                                cancel = true;
                                            } else if response.lost_focus() && !just_opened {
                                                commit = true;
                                            }
                                        } else {
                                            let text = if todo.done {
                                                RichText::new(&todo.text)
                                                    .strikethrough()
                                                    .color(look.faint)
                                            } else {
                                                RichText::new(&todo.text).color(look.text)
                                            };
                                            let label = ui
                                                .add(
                                                    egui::Label::new(text)
                                                        .wrap()
                                                        .sense(egui::Sense::click()),
                                                )
                                                .on_hover_text("双击修改");
                                            if label.double_clicked() {
                                                start_edit = Some(todo.id);
                                            }
                                        }
                                    });
                                });
                            });
                        });
                    ui.data_mut(|data| data.insert_temp(row_id, row.response.rect));

                    let drop = ui.interact(
                        row.response.rect,
                        Id::new(("todo-drop", todo.id)),
                        egui::Sense::hover(),
                    );
                    if drop.dnd_hover_payload::<u64>().is_some() {
                        ui.painter().hline(
                            drop.rect.x_range(),
                            drop.rect.top(),
                            Stroke::new(2.0_f32, look.accent),
                        );
                    }
                    if let Some(payload) = drop.dnd_release_payload::<u64>() {
                        reorder = Some((*payload, todo.id));
                    }
                }

                if done > 0 {
                    ui.add_space(14.0);
                    let caption = if hide_done {
                        format!("已收起 {done}")
                    } else {
                        format!("已完成 {done}")
                    };
                    ui.horizontal(|ui| {
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if !hide_done {
                                let clear =
                                    text_action(ui, "清除", look.faint).on_hover_text("清除已完成");
                                if clear.clicked() {
                                    clear_done = true;
                                }
                            }
                            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                                let hint = if hide_done {
                                    "点击展开"
                                } else {
                                    "点击收起"
                                };
                                let toggle_done =
                                    text_action(ui, &caption, look.faint).on_hover_text(hint);
                                if toggle_done.clicked() {
                                    toggle_hide = true;
                                }
                            });
                        });
                    });
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
        if toggle_hide {
            self.state.hide_done = !self.state.hide_done;
            self.persist_now();
        }
        if clear_done && self.state.clear_done() > 0 {
            self.editing = None;
            self.edit_text.clear();
            self.persist_now();
        }
    }

    fn appearance_menu(&mut self, ui: &mut egui::Ui) -> egui::Response {
        let screen = ui.ctx().screen_rect();
        let x = (self.menu_button.right() - MENU_WIDTH).max(screen.left() + 8.0);
        let y = self.menu_button.bottom() + 6.0;
        // 跟待办画在同一层，后画的才能接到点击。单独的 Area 在这个透明窗口里点不到。
        let menu_rect = egui::Rect::from_min_size(egui::pos2(x, y), Vec2::new(MENU_WIDTH, 176.0));
        self.menu_rect = menu_rect;
        let mut host = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(menu_rect)
                .layout(Layout::top_down(Align::Min)),
        );
        let blocker = host.interact(
            menu_rect,
            Id::new("appearance-menu-block"),
            egui::Sense::click(),
        );
        let sheet = if self.look.dark {
            Color32::from_rgb(44, 44, 48)
        } else {
            Color32::from_rgb(255, 255, 255)
        };
        let inner = egui::Frame::new()
            .fill(sheet)
            .stroke(Stroke::new(1.0_f32, self.look.line))
            .corner_radius(14)
            .inner_margin(egui::Margin::same(14))
            .shadow(egui::Shadow {
                offset: [0, 8],
                blur: 18,
                spread: 0,
                color: Color32::from_black_alpha(56),
            })
            .show(&mut host, |ui| {
                ui.set_width(MENU_WIDTH - 28.0);
                ui.label(
                    RichText::new(format!("透明度  {:.0}%", self.state.opacity * 100.0))
                        .size(12.0)
                        .color(self.look.faint),
                );
                ui.add_space(6.0);
                ui.spacing_mut().slider_width = MENU_WIDTH - 36.0;
                let mut opacity = self.state.opacity;
                let slider = ui.add(
                    egui::Slider::new(&mut opacity, 0.25..=0.96)
                        .show_value(false)
                        .trailing_fill(true),
                );
                if slider.changed() {
                    self.state.set_opacity(opacity);
                }
                if slider.drag_stopped() || (slider.changed() && !slider.dragged()) {
                    self.persist_now();
                }
                ui.add_space(12.0);
                ui.label(RichText::new("外观").size(12.0).color(self.look.faint));
                ui.add_space(6.0);
                if let Some(dark) = theme_choice(ui, self.state.dark) {
                    self.state.dark = dark;
                    self.persist_now();
                }
            })
            .response;
        blocker.union(inner)
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

fn trim_view(text: &str) -> &str {
    text.trim()
}

fn icon_button(
    ui: &mut egui::Ui,
    label: &str,
    color: Color32,
    hover_fill: Option<Color32>,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(26.0), egui::Sense::click());
    if response.hovered() {
        let fill = hover_fill.unwrap_or(Color32::from_rgba_unmultiplied(128, 128, 128, 28));
        ui.painter().circle_filled(rect.center(), 11.0, fill);
    }
    let text_color = if response.hovered() && hover_fill.is_some() {
        Color32::WHITE
    } else if response.hovered() {
        color
    } else {
        color
    };
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(14.0),
        text_color,
    );
    response.on_hover_cursor(CursorIcon::PointingHand)
}

fn pin_button(ui: &mut egui::Ui, on: bool, look: &Look) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(26.0), egui::Sense::click());
    let color = if on || response.hovered() {
        look.accent
    } else {
        look.faint
    };
    if response.hovered() {
        ui.painter().circle_filled(rect.center(), 11.0, look.hover);
    }
    let head = rect.center() + Vec2::new(0.0, -4.2);
    if on {
        ui.painter().circle_filled(head, 3.1, color);
    } else {
        ui.painter()
            .circle_stroke(head, 3.1, Stroke::new(1.3_f32, color));
    }
    ui.painter().line_segment(
        [
            rect.center() + Vec2::new(0.0, -1.2),
            rect.center() + Vec2::new(0.0, 6.2),
        ],
        Stroke::new(1.4_f32, color),
    );
    response
        .on_hover_text(if on { "取消置顶" } else { "置顶" })
        .on_hover_cursor(CursorIcon::PointingHand)
}

fn more_button(ui: &mut egui::Ui, open: bool, look: &Look) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(26.0), egui::Sense::click());
    let color = if open || response.hovered() {
        look.text
    } else {
        look.faint
    };
    if open || response.hovered() {
        ui.painter().circle_filled(rect.center(), 11.0, look.hover);
    }
    for index in -1..=1 {
        ui.painter().circle_filled(
            rect.center() + Vec2::new(index as f32 * 4.2, 0.0),
            1.35,
            color,
        );
    }
    response
        .on_hover_text("透明度、外观")
        .on_hover_cursor(CursorIcon::PointingHand)
}

fn add_mark(ui: &mut egui::Ui, ready: bool, look: &Look) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(22.0), egui::Sense::click());
    let center = rect.center();
    let color = if ready { look.accent } else { look.faint };
    ui.painter()
        .circle_stroke(center, 8.0, Stroke::new(1.4_f32, color));
    if ready {
        let mark = Stroke::new(1.4_f32, color);
        ui.painter().line_segment(
            [center + Vec2::new(-3.2, 0.0), center + Vec2::new(3.2, 0.0)],
            mark,
        );
        ui.painter().line_segment(
            [center + Vec2::new(0.0, -3.2), center + Vec2::new(0.0, 3.2)],
            mark,
        );
    }
    response
        .on_hover_text("记下来")
        .on_hover_cursor(CursorIcon::PointingHand)
}

fn circle_check(ui: &mut egui::Ui, checked: bool, look: &Look) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(22.0), egui::Sense::click());
    let center = rect.center();
    if checked {
        ui.painter().circle_filled(center, 8.0, look.accent);
        let mark = Stroke::new(1.7_f32, Color32::WHITE);
        ui.painter().line_segment(
            [center + Vec2::new(-3.4, 0.2), center + Vec2::new(-1.0, 2.7)],
            mark,
        );
        ui.painter().line_segment(
            [center + Vec2::new(-1.0, 2.7), center + Vec2::new(3.8, -2.8)],
            mark,
        );
    } else {
        ui.painter()
            .circle_stroke(center, 8.0, Stroke::new(1.4_f32, look.faint));
    }
    response.on_hover_cursor(CursorIcon::PointingHand)
}

fn grip_dots(ui: &mut egui::Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(10.0, 18.0), egui::Sense::hover());
    for index in 0..3 {
        ui.painter().circle_filled(
            rect.center() + Vec2::new(0.0, (index as f32 - 1.0) * 3.6),
            1.15,
            color,
        );
    }
}

fn row_delete(ui: &mut egui::Ui, shown: bool, look: &Look) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(22.0), egui::Sense::click());
    if shown || response.hovered() {
        if response.hovered() {
            ui.painter().circle_filled(rect.center(), 10.0, look.hover);
        }
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "×",
            egui::FontId::proportional(14.0),
            if response.hovered() {
                look.danger
            } else {
                look.faint
            },
        );
    }
    response
        .on_hover_text("删除")
        .on_hover_cursor(CursorIcon::PointingHand)
}

fn text_action(ui: &mut egui::Ui, text: &str, color: Color32) -> egui::Response {
    ui.add(
        egui::Label::new(RichText::new(text).size(12.5).color(color)).sense(egui::Sense::click()),
    )
    .on_hover_cursor(CursorIcon::PointingHand)
}

/// 点到另一侧时返回新的深色开关。
fn theme_choice(ui: &mut egui::Ui, dark: bool) -> Option<bool> {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 32.0), egui::Sense::hover());
    let track = if dark {
        Color32::from_rgb(58, 58, 64)
    } else {
        Color32::from_rgb(236, 236, 239)
    };
    let knob = if dark {
        Color32::from_rgb(78, 78, 86)
    } else {
        Color32::WHITE
    };
    let ink = if dark {
        Color32::from_rgb(245, 245, 247)
    } else {
        Color32::from_rgb(28, 28, 30)
    };
    let mute = if dark {
        Color32::from_rgb(160, 160, 168)
    } else {
        Color32::from_rgb(120, 120, 128)
    };
    ui.painter()
        .rect_filled(rect, egui::CornerRadius::same(10), track);
    let half = rect.width() * 0.5;
    let knob_rect = if dark {
        egui::Rect::from_min_size(
            rect.left_top() + Vec2::new(half, 0.0),
            Vec2::new(half, rect.height()),
        )
    } else {
        egui::Rect::from_min_size(rect.left_top(), Vec2::new(half, rect.height()))
    };
    ui.painter()
        .rect_filled(knob_rect.shrink(3.0), egui::CornerRadius::same(8), knob);

    let left =
        egui::Rect::from_min_max(rect.left_top(), egui::pos2(rect.center().x, rect.bottom()));
    let right =
        egui::Rect::from_min_max(egui::pos2(rect.center().x, rect.top()), rect.right_bottom());
    let light = ui.interact(left, Id::new("theme-light"), egui::Sense::click());
    let dark_hit = ui.interact(right, Id::new("theme-dark"), egui::Sense::click());
    ui.painter().text(
        left.center(),
        egui::Align2::CENTER_CENTER,
        "浅色",
        egui::FontId::proportional(13.0),
        if dark { mute } else { ink },
    );
    ui.painter().text(
        right.center(),
        egui::Align2::CENTER_CENTER,
        "深色",
        egui::FontId::proportional(13.0),
        if dark { ink } else { mute },
    );
    if light.clicked() && dark {
        Some(false)
    } else if dark_hit.clicked() && !dark {
        Some(true)
    } else {
        None
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

        let was_editing = self.editing.is_some();
        let mut menu_hovered = false;
        let frame = egui::Frame::new()
            .fill(self.look.panel)
            .stroke(Stroke::new(1.0_f32, self.look.line))
            .corner_radius(18)
            .inner_margin(egui::Margin {
                left: 16,
                right: 14,
                top: 8,
                bottom: 12,
            });
        egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
            self.title_bar(ui);
            ui.add_space(6.0);
            self.composer(ui);
            if let Some(err) = &self.save_error {
                ui.add_space(4.0);
                ui.label(
                    RichText::new(format!("没能保存：{err}"))
                        .color(self.look.danger)
                        .size(12.0),
                );
            }
            ui.add_space(4.0);
            self.list(ui);
            self.resize_handles(ui);
            if self.menu_open {
                menu_hovered = self.appearance_menu(ui).hovered();
            }
        });
        let (click, escape, pointer) = ctx.input(|input| {
            (
                input.pointer.any_click(),
                input.key_pressed(egui::Key::Escape),
                input.pointer.interact_pos(),
            )
        });
        let composer_focused = ctx.memory(|mem| mem.has_focus(Id::new("composer")));
        if self.menu_open && !self.ignore_menu_click {
            let on_button = pointer.is_some_and(|pos| self.menu_button.contains(pos));
            let on_menu = pointer.is_some_and(|pos| self.menu_rect.contains(pos));
            if (click && !menu_hovered && !on_menu && !on_button)
                || (escape && !was_editing && !composer_focused)
            {
                self.menu_open = false;
            }
        }
        self.ignore_menu_click = false;

        self.sync_geometry(ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.persist_now();
    }
}
