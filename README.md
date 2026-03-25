# Deepwrite

在 AI 開發的時代，程式碼可以交給 AI 寫，但 spec、文件、技術決策還是得你自己掌握。Deepwrite 讓你在終端機裡專注地閱讀和編輯 Markdown 文件 — 開發的同時，隨時切過來審核、修改你的文件。

結合 **Yazi 風格的檔案瀏覽** 和 **iA Writer 風格的專注寫作**，給 vibe coders 和開發者打造。

## 功能

- **雙模式介面** — 左側瀏覽檔案，右側編輯
- **專注模式（Focus Mode）** — 句子、段落、打字機三種淡化模式，幫助你集中注意力
- **非模態編輯** — 打開就能打字，方向鍵移動，Emacs 風格快捷鍵（Ctrl+A/E 跳到行首/行尾）
- **Markdown 語法高亮** — 標題、粗體、斜體、程式碼區塊、連結
- **格式化快捷鍵** — Ctrl+B 粗體、Ctrl+I 斜體、Ctrl+1/2/3 標題
- **中日韓字數統計** — 精確的 CJK 字元計數
- **自動儲存** — 2 秒延遲寫入，透過暫存檔 + 原子重新命名確保安全
- **外部變更偵測** — 自動偵測其他編輯器的檔案修改
- **淺色/深色主題** — 自動偵測系統偏好
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
| `y` | 複製檔案路徑 |
| `?` | 說明 |
| `q` | 離開 |

### 編輯模式

| 按鍵 | 動作 |
|------|------|
| `Esc` | 回到瀏覽模式 |
| `Ctrl+B` | 粗體 |
| `Ctrl+I` | 斜體 |
| `Ctrl+1/2/3` | 標題 1/2/3 |
| `Ctrl+A` | 跳到行首 |
| `Ctrl+E` | 跳到行尾 |
| `Ctrl+C` | 複製 |
| `Ctrl+V` | 貼上 |
| `Ctrl+Z` | 復原 |

## 設定

設定檔位於 `~/.config/deepwrite/config.toml`：

```toml
[editor]
tab_width = 4

[focus]
mode = "sentence"     # "none", "sentence", "paragraph", "typewriter"

[theme]
mode = "auto"         # "auto", "light", "dark"

[browser]
show_hidden = false
```

## 授權

[MIT](LICENSE)

## 貢獻

請參閱 [CONTRIBUTING.md](CONTRIBUTING.md)。
