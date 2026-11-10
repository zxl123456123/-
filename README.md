# 随手记

一块停在桌面上的待办窗口。无边框、可拖动、可调透明度，液态玻璃质感，文字保持清晰，数据只写在本机。

## 做什么

- 输入后回车，或点左边的圆圈，新的在最上面
- 这是今天的纸：没做完的会留到第二天，昨天已完成的不再占地方
- 网页预览是四层液态玻璃（折射、着色、高光、文字），桌面程序透过真桌面看过去
- 纸条跟着待办收紧，底下不留一块空
- 点圆圈完成，双击文字修改，拖左边的点排序
- 透明度、深色在标题栏「···」里。透的是纸的底色，字不会一起淡掉
- 置顶是标题栏上的图钉
- 已完成沉到横线下面，可以收起或清除
- 拖边缘改变大小，拖标题栏「随手记」移动
- 关掉再打开，列表、透明度、位置都还在

## 参考了什么

窗口行为对齐 [BUG-gao/floating-todo](https://github.com/BUG-gao/floating-todo)：无边框透明窗、始终置顶、透明度只作用在面板上、新事项插到列表开头、本地保存。

标题栏和圆角对齐 [egui 的 custom_window_frame](https://github.com/emilk/egui/blob/0.31.1/examples/custom_window_frame/src/main.rs)：清屏色全透明，自己画一圈圆角面板。

版式参考的是产品，不是再铺一层控件：

- 便笺和记事本：窗口贴着内容，不是一块固定高度的空卡片
- 液态玻璃：四层叠加（backdrop 折射、着色、边缘高光、清晰文字），不是普通毛玻璃
- [Things 3](https://culturedcode.com/things/)：纸面上几乎没有工具栏，待办是平的一行，完成的事项沉到列表底部
- Windows 便笺，以及 [Glass Notes](https://github.com/psipher/GlassNotes)：颜色和透明度放在标题栏菜单里，不铺在正文下面
- [HushNotes](https://hushnotes.app/)：淡下去的是纸，字保持清楚

相对这两边做的取舍：

- 不用 Tauri / WebView。界面就是 Rust + [egui](https://github.com/emilk/egui) / eframe，一个二进制
- 不做三天视图、习惯打卡、提醒、托盘和鼠标穿透。穿透打开后又点不到窗口，这块小工具没有托盘可以把窗口找回来
- 渲染固定 Glow。Windows 上 wgpu 的透明窗口经常变成黑底，或把控件也一起变淡
- 待办规则和 JSON 读写跟界面分开，不打开窗口也能测

## 网页预览

想先看版式、先试记一条，打开 `web/index.html`，或把 `web` 文件夹丢进浏览器。网页用花墙纸把玻璃折射做出来；桌面版仍是独立透明窗口。

## 下载到 Windows

仓库地址：[github.com/zxl123456123/-](https://github.com/zxl123456123/-)。用分支 `cursor/note-surface-37dc`，不要下 `main`。

已经编好 Windows 程序，拉下来双击即可，不必装 Rust。

已经装了 Git：

```powershell
git clone -b cursor/note-surface-37dc https://github.com/zxl123456123/-.git suishouji
cd suishouji
.\dist\suishouji.exe
```

没有 Git 就下载这份压缩包，解压后双击 `dist\suishouji.exe`：  
[cursor/note-surface-37dc.zip](https://github.com/zxl123456123/-/archive/refs/heads/cursor/note-surface-37dc.zip)

也可以把 `dist\suishouji.exe` 拷到桌面，单独运行。发布版没有黑色控制台窗口。

自己从源码编译的话，需要 Rust 1.88 或更新：

```powershell
cargo run --release
```

生成的程序在 `target\release\suishouji.exe`。

数据文件：

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
| 输入后回车，或点输入行左边的圆圈 | 记下一条 |
| 输入框里按 Esc | 清空还没提交的草稿 |
| 点击圆圈 | 完成 / 取消完成 |
| 双击文字 | 修改，回车或点到别处保存，Esc 取消 |
| 拖左边三个点 | 调整顺序 |
| 悬停一行，点「×」 | 删除 |
| 列表末尾「已完成 / 已收起」 | 收起或展开已完成 |
| 「清除」 | 删掉已完成 |
| 标题栏图钉 | 置顶 / 取消置顶 |
| 标题栏「···」或底栏「外观」 | 调透明度 |
| 底栏浅色 / 深色 | 切换玻璃着色 |
| 拖标题栏日期 | 移动窗口 |
| 拖窗口边缘 | 改变大小 |
