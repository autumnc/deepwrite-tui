
# Deepwrite

在 AI 開發的時代，程式碼可以交給 AI 寫，但 spec、文件、技術決策還是得你自己掌握。Deepwrite 讓你在終端機裡專注地閱讀和編輯 Markdown 文件 — 開發的同時，隨時切過來審核、修改你的文件。

結合 **Yazi 風格的檔案瀏覽** 和 **iA Writer 風格的專注寫作**，給 vibe coders 和開發者打造。

<video src="https://github.com/user-attachments/assets/869fe3d4-203d-4023-a4a5-ebe1fed5b9f5" width="100%" autoplay loop muted playsinline></video>

## 功能

- **雙模式介面** — 左側瀏覽檔案，右側編輯
- **大綱面板** — Ctrl+O 開啟標題大綱，j/k 導航，Enter 跳轉
- **專注模式（Focus Mode）** — 句子、段落、打字機、當前行四種淡化模式，幫助集中注意力
- **非模態編輯** — 打開就能打字，方向鍵移動，Emacs 風格快捷鍵（Ctrl+A/E 跳到行首/行尾）
- **Markdown 語法高亮** — 標題階層顏色（H1-H6 暖冷漸層）、粗體、斜體、刪除線、程式碼區塊、連結、引用區塊斜體
- **格式化快捷鍵** — Ctrl+B 粗體、Ctrl+I 斜體、F1-F6 / Ctrl+1-6 標題、Ctrl+H 高亮、Ctrl+D 刪除線、Ctrl+U 底線、Ctrl+K 連結
- **`==highlight==` 語法** — 反轉背景色的螢光筆效果
- **`<u>underline</u>` 語法** — HTML 標籤風格的底線文字
- **高對比主題** — `theme.mode = "high_contrast"`，針對 `#101010` 背景最佳化
- **中日韓字數統計** — 精確的 CJK 字元計數
- **自動儲存** — 2 秒延遲寫入，透過暫存檔 + 原子重新命名確保安全
- **外部變更偵測** — 自動偵測其他編輯器的檔案修改
- **淺色/深色/高對比主題** — 自動偵測系統偏好，按 `t` 循環切換
- **可自訂** — `~/.config/deepwrite/config.toml`

## 安裝

### Homebrew（macOS / Linux）

```bash
brew install tomdhyang/tap/deepwrite
```

### 預編譯 Binary

從 [GitHub Releases](https://github.com/tomdhyang/deepwrite-tui/releases) 下載。

支援 macOS（Intel + Apple Silicon）、Linux（x64 + ARM64）、Windows（x64）。

### 從原始碼編譯

```bash
cargo install --git https://github.com/tomdhyang/deepwrite-tui.git
```

需要 [Rust](https://rustup.rs/)（建議使用最新穩定版）。

## 使用方式

```bash
# 開啟目前目錄
deepwrite

# 開啟指定目錄
deepwrite ~/Documents/notes

# 開啟指定檔案
deepwrite README.md
```

### 瀏覽模式

| 按鍵 | 動作 |
|------|------|
| `j` / `Down` | 向下移動 |
| `k` / `Up` | 向上移動 |
| `Enter` / `l` | 開啟檔案 / 進入目錄 |
| `h` / `Backspace` | 回到上層目錄 |
| `a` | 新增檔案或目錄 |
| `r` | 重新命名 |
| `d` | 刪除 |
| `/` | 搜尋 / 過濾 |
| `.` | 切換隱藏檔案 |
| `t` | 循環主題 |
| `cc` | 複製檔案路徑 |
| `?` | 說明 |
| `q` | 離開 |

### 編輯模式

| 按鍵 | 動作 |
|------|------|
| `Esc` | 回到瀏覽模式 |
| `Ctrl+O` | 開啟 / 切換大綱面板 |
| `Ctrl+E` | 切換檔案瀏覽面板 |
| `Ctrl+F` | 循環專注模式 |
| `Ctrl+B` | 粗體 |
| `Ctrl+I` / `Ctrl+T` | 斜體 |
| `F1-F6` / `Ctrl+1-6` | 標題層級 1-6 |
| `Ctrl+H` | ==高亮== |
| `Ctrl+D` | ~~刪除線~~ |
| `Ctrl+U` | 底線 |
| `Ctrl+K` | 插入連結 |
| `Ctrl+A` | 全選 |
| `Ctrl+C` | 複製 |
| `Ctrl+V` | 貼上 |
| `Ctrl+X` | 剪下 |
| `Ctrl+Z` | 復原 |
| `Ctrl+Y` | 重做 |
| `Ctrl+S` | 手動儲存 |

## 設定

設定檔位於 `~/.config/deepwrite/config.toml`：

```toml
[editor]
tab_width = 4

[focus]
mode = "sentence"     # "off", "sentence", "paragraph", "typewriter", "line"

[theme]
mode = "system"       # "system", "light", "dark", "high_contrast"

[browser]
show_hidden = false
```

## 授權

[MIT](LICENSE)

## 貢獻

請參閱 [CONTRIBUTING.md](CONTRIBUTING.md)。
