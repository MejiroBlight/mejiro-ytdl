# Mejiro YTDL

Tauri と SvelteKit で作成した動画ダウンロードアプリです。URLから動画情報を取得し、画質を選んで動画をダウンロードできます。音声のみのダウンロードにも対応しています。

## 主な機能

- URLから動画情報と選択可能な画質を取得
- 画質を指定した動画のダウンロード
- 音声のみのダウンロード
- 保存ファイル名の指定（拡張子は形式に合わせて自動付与）
- ダウンロード進捗とエラー内容の表示
- 保存先フォルダーをアプリから開く
- 起動時に `yt-dlp` の更新を確認
- FFmpegとffprobeがCLIから使えない場合、初回起動時に自動導入

## 使い方

1. アプリを起動し、動画のURLを入力します。
2. **画質を取得**を選択します。
3. 動画の画質または**音声のみ**を選びます。必要に応じて保存ファイル名を入力します。
4. **ダウンロード開始**を選択します。

ダウンロードしたファイルは、アプリの実行ファイルと同じ場所にある `result` フォルダーに保存されます。**resultフォルダーを開く**から保存先を開けます。

## 外部ツールのセットアップ

アプリには `yt-dlp.exe` が同梱されます。起動時に `yt-dlp.exe -U` を実行して更新を確認します。更新に失敗した場合も、現在のバージョンでアプリを利用できます。

FFmpegとffprobeの両方がコマンドラインから利用できる場合は、その環境を使います。利用できない場合は、[`ffmpeg-sidecar`](https://crates.io/crates/ffmpeg-sidecar) を使って初回起動時にダウンロードし、アプリの `lib` フォルダーへインストールします。この初回インストールにはインターネット接続が必要です。

## 開発

### 必要なもの

- Node.js と npm
- Rust ツールチェーン
- Tauriアプリの開発に必要な環境

### 起動

```powershell
npm install
npm run tauri dev
```

### ビルド

```powershell
npm run tauri build
```

### チェック

```powershell
npm run check
cargo check --manifest-path src-tauri\Cargo.toml
```

## 使用技術

- [Tauri 2](https://tauri.app/)
- [SvelteKit](https://svelte.dev/)
- [yt-dlp](https://github.com/yt-dlp/yt-dlp)
- [ffmpeg-sidecar](https://crates.io/crates/ffmpeg-sidecar)
