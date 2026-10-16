use tauri::Emitter;
use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;
use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader, Read};
use std::collections::BTreeSet;

#[derive(serde::Serialize)]
struct VideoInfo {
    title: String,
    qualities: Vec<u32>,
}

fn get_exe_dir() -> Result<std::path::PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    exe.parent()
        .map(|p| p.to_path_buf())
        .ok_or_else(|| "アプリの実行ファイルの場所を取得できません".to_string())
}

fn get_result_dir() -> Result<std::path::PathBuf, String> {
    let exe_dir = get_exe_dir()?;
    let result_dir = exe_dir.join("result");
    std::fs::create_dir_all(&result_dir).map_err(|e| e.to_string())?;
    Ok(result_dir)
}

#[tauri::command]
fn open_result_dir(app_handle: AppHandle) -> Result<(), String> {
    let result_dir = get_result_dir()?;
    let result_path = result_dir.to_string_lossy().into_owned();
    app_handle
        .opener()
        .open_path(result_path, None::<&str>)
        .map_err(|error| format!("resultフォルダーを開けませんでした: {error}"))
}

fn get_lib_dir() -> Result<std::path::PathBuf, String> {
    let exe_dir = get_exe_dir()?;
    let lib_dir = exe_dir.join("lib");
    Ok(lib_dir)
}

fn validate_file_name(file_name: Option<String>) -> Result<Option<String>, String> {
    let Some(file_name) = file_name else {
        return Ok(None);
    };
    let file_name = file_name.trim();
    if file_name.is_empty() {
        return Ok(None);
    }

    if file_name.encode_utf16().count() > 200
        || file_name == "."
        || file_name == ".."
        || file_name.ends_with([' ', '.'])
        || file_name
            .chars()
            .any(|character| character.is_control() || "<>:\"/\\|?*".contains(character))
    {
        return Err("ファイル名に使用できない文字が含まれているか、長すぎます".into());
    }

    let device_name = file_name
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    let reserved_device = matches!(
        device_name.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) || (device_name.len() == 4
        && (device_name.starts_with("COM") || device_name.starts_with("LPT"))
        && matches!(device_name.as_bytes()[3], b'1'..=b'9'));
    if reserved_device {
        return Err("この名前はファイル名として使用できません".into());
    }

    Ok(Some(file_name.to_string()))
}

#[tauri::command]
async fn download_video(
    app_handle: AppHandle,
    url: String,
    quality: Option<u32>,
    audio_only: bool,
    file_name: Option<String>,
) -> Result<String, String> {
    let file_name = validate_file_name(file_name)?;
    let result_dir = get_result_dir()?;
    let lib_dir = get_lib_dir()?;
    let ytdlp_path = lib_dir.join("yt-dlp.exe");
    let ffmpeg_dir = lib_dir; // yt-dlpにはffmpeg.exeの「フォルダ」を渡すのが確実

    if !audio_only && quality == Some(0) {
        return Err("画質の指定が正しくありません".into());
    }

    let app_handle_clone = app_handle.clone();

    // 重いプロセス処理は spawn_blocking の中で回す
    tauri::async_runtime::spawn_blocking(move || {
        let mut command = Command::new(&ytdlp_path);
        command
            .arg(&url)
            .arg("--ffmpeg-location")
            .arg(&ffmpeg_dir)
            .arg("--paths")
            .arg(&result_dir);

        if audio_only {
            command.arg("--extract-audio");
        } else if let Some(quality) = quality {
            command
                .arg("--format")
                .arg(format!(
                    "bestvideo[height<={quality}]+bestaudio/best[height<={quality}]"
                ));
        }

        if let Some(file_name) = file_name {
                command
                    .arg("--output")
                    .arg(format!("{}.%(ext)s", file_name.replace('%', "%%")));
        }

        let mut child = command
            .arg("--newline")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())?;

        let stderr = child.stderr.take().expect("stderr is piped");
        let stderr_reader = std::thread::spawn(move || {
            let mut output = String::new();
            BufReader::new(stderr)
                .read_to_string(&mut output)
                .map(|_| output)
        });

        // 標準出力を1行ずつリアルタイムで読み取る
        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                if let Ok(line_str) = line {
                    // Tauriのイベント機能でフロントエンドにテキストを送信
                    let _ = app_handle_clone.emit("download-progress", &line_str);
                }
            }
        }

        let status = child.wait().map_err(|e| e.to_string())?;
        let stderr = stderr_reader
            .join()
            .map_err(|_| "標準エラー出力の読み取りに失敗しました".to_string())?
            .map_err(|e| e.to_string())?;

        if status.success() {
            Ok("ダウンロードが完了しました！".into())
        } else {
            let error = stderr.trim();
            if error.is_empty() {
                Err(format!("ダウンロードに失敗しました (終了コード: {status})"))
            } else {
                Err(format!("ダウンロードに失敗しました: {error}"))
            }
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn update_ytdlp() -> Result<String, String> {
    let ytdlp_path = get_lib_dir()?.join("yt-dlp.exe");
    let output = tauri::async_runtime::spawn_blocking(move || {
        Command::new(&ytdlp_path).arg("-U").output()
    })
    .await
    .map_err(|error| error.to_string())?
    .map_err(|error| format!("yt-dlpの更新を起動できませんでした: {error}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let details = [stdout, stderr]
        .into_iter()
        .filter(|output| !output.is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    if output.status.success() {
        if details.is_empty() {
            Ok("yt-dlpの更新確認が完了しました。".into())
        } else {
            Ok(format!("yt-dlpの更新確認が完了しました。\n{details}"))
        }
    } else if details.is_empty() {
        Err(format!(
            "yt-dlpの自動更新に失敗しました (終了コード: {})",
            output.status
        ))
    } else {
        Err(format!("yt-dlpの自動更新に失敗しました: {details}"))
    }
}

#[tauri::command]
async fn get_video_info(url: String) -> Result<VideoInfo, String> {
    let lib_dir = get_lib_dir()?;
    let ytdlp_path = lib_dir.join("yt-dlp.exe");

    let output = tauri::async_runtime::spawn_blocking(move || {
        Command::new(&ytdlp_path)
            .arg("--dump-single-json")
            .arg("--skip-download")
            .arg("--no-warnings")
            .arg(&url)
            .output()
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    if output.status.success() {
        let metadata: serde_json::Value = serde_json::from_slice(&output.stdout)
            .map_err(|e| format!("動画情報を読み取れませんでした: {e}"))?;
        let title = metadata
            .get("title")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("動画")
            .to_string();
        let qualities = metadata
            .get("formats")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .filter(|format| {
                format
                    .get("vcodec")
                    .and_then(serde_json::Value::as_str)
                    .map_or(true, |codec| codec != "none")
            })
            .filter_map(|format| {
                format
                    .get("height")
                    .and_then(serde_json::Value::as_u64)
                    .and_then(|height| u32::try_from(height).ok())
            })
            .collect::<BTreeSet<_>>()
            .into_iter()
            .rev()
            .collect();

        Ok(VideoInfo { title, qualities })
    } else {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        Err(format!("情報取得に失敗しました: {}", err_msg.trim()))
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            download_video,
            update_ytdlp,
            get_video_info,
            open_result_dir
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
