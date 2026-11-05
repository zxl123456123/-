# 随手记

一块停在桌面上的待办窗口。无边框、可拖动、可调透明度，文字保持清晰，数据只写在本机。

## 做什么

- 回车记下一条，新的在最上面
- 勾选完成，双击修改，拖左边的 `≡` 排序
- 底栏拖动透明度（25%–96%）。透的是面板底色，字不会一起淡掉
- 可置顶、切换深色/浅色、隐藏已完成
- 拖边缘改变大小，拖标题栏移动
- 关掉再打开，列表、透明度、位置都还在

## 参考了什么

窗口行为对齐 [BUG-gao/floating-todo](https://github.com/BUG-gao/floating-todo)：无边框透明窗、始终置顶、透明度只作用在面板上、新事项插到列表开头、本地保存。

标题栏和圆角对齐 [egui 的 custom_window_frame](https://github.com/emilk/egui/blob/0.31.1/examples/custom_window_frame/src/main.rs)：清屏色全透明，自己画一圈圆角面板。

相对这两边做的取舍：

- 不用 Tauri / WebView。界面就是 Rust + [egui](https://github.com/emilk/egui) / eframe，一个二进制
- 不做三天视图、习惯打卡、提醒、托盘和鼠标穿透。穿透打开后又点不到窗口，这块小工具没有托盘可以把窗口找回来
- 渲染固定 Glow。Windows 上 wgpu 的透明窗口经常变成黑底，或把控件也一起变淡
- 待办规则和 JSON 读写跟界面分开，不打开窗口也能测

## 运行

需要 Rust 1.88 或更新。

```bash
cargo run --release
```

Windows 上发布版不带控制台窗口。数据文件：

- Windows：`%LOCALAPPDATA%\suishouji\notes.json`
- Linux：`~/.local/share/suishouji/notes.json`
- macOS：`~/Library/Application Support/suishouji/notes.json`

中文使用系统字体（Windows 为微软雅黑）。也可以自己指定：

```bash
SUISHOUJI_FONT=/path/to/font.ttf cargo run --release
```

## 操作

| 操作 | 结果 |
| --- | --- |
| 输入后回车，或点「添加」 | 记下一条 |
| 输入框里按 Esc | 清空还没提交的草稿 |
| 点击圆圈 | 完成 / 取消完成 |
| 双击文字 | 修改，回车或点到别处保存，Esc 取消 |
| 拖 `≡` | 调整顺序 |
| 拖标题栏空白处 | 移动窗口 |
| 拖窗口边缘 | 改变大小 |
