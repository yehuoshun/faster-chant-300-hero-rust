// 自动更新模块
// 启动时检查 GitHub Releases，有新版本自动下载替换

use crate::logger;
use serde::Deserialize;
use std::io::Read;
use std::os::windows::process::CommandExt;
use std::process::Command;
use std::thread;
use std::time::Duration;

const GITHUB_API: &str =
    "https://api.github.com/repos/yehuoshun/faster-chant-300-hero-rust/releases/latest";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    assets: Vec<GitHubAsset>,
}

#[derive(Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

pub fn check_update() {
    logger::info(&format!("检查更新，当前版本: v{}", CURRENT_VERSION));

    let resp = match ureq::get(GITHUB_API)
        .set("User-Agent", "faster-chant-300-hero-rust")
        .set("Accept", "application/vnd.github.v3+json")
        .call()
    {
        Ok(r) => r,
        Err(e) => {
            logger::warn(&format!("更新检查失败(网络): {}", e));
            return;
        }
    };

    let release: GitHubRelease = match resp.into_json() {
        Ok(r) => r,
        Err(e) => {
            logger::warn(&format!("更新检查失败(解析): {}", e));
            return;
        }
    };

    let latest = release.tag_name.trim_start_matches('v');
    if latest == CURRENT_VERSION {
        logger::info("已是最新版本");
        return;
    }

    logger::info(&format!("发现新版本: v{}", latest));

    let exe = match release.assets.iter().find(|a| a.name.ends_with(".exe")) {
        Some(a) => a,
        None => {
            logger::error("未找到 exe 下载链接");
            return;
        }
    };

    logger::info(&format!("下载中: {} ({} bytes)", exe.name, exe.browser_download_url));

    let resp = match ureq::get(&exe.browser_download_url)
        .set("User-Agent", "faster-chant-300-hero-rust")
        .call()
    {
        Ok(r) => r,
        Err(e) => {
            logger::error(&format!("下载失败: {}", e));
            return;
        }
    };

    let mut data = Vec::new();
    if let Err(e) = resp.into_reader().read_to_end(&mut data) {
        logger::error(&format!("读取下载数据失败: {}", e));
        return;
    }

    logger::info(&format!("下载完成: {} bytes", data.len()));

    let tmp_dir = std::env::temp_dir().join("faster-chant-update");
    let _ = std::fs::create_dir_all(&tmp_dir);

    let new_exe = tmp_dir.join("faster-chant-300-hero-rust.exe");
    if let Err(e) = std::fs::write(&new_exe, &data) {
        logger::error(&format!("写入新版本失败: {}", e));
        return;
    }

    let current_exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            logger::error(&format!("无法获取当前 exe 路径: {}", e));
            return;
        }
    };

    logger::info(&format!("准备替换: {:?} -> {:?}", current_exe, new_exe));

    let script = tmp_dir.join("update.bat");
    let script_content = format!(
        "@echo off\r\n\
         echo Updating faster-chant-300-hero-rust...\r\n\
         :loop\r\n\
         timeout /t 1 /nobreak > nul\r\n\
         if exist \"{0}\" goto loop\r\n\
         move /Y \"{1}\" \"{0}\"\r\n\
         if %errorlevel% equ 0 (\r\n\
             start \"\" \"{0}\"\r\n\
         ) else (\r\n\
             echo Update failed, please replace manually\r\n\
             pause\r\n\
         )\r\n\
         del \"%~f0\"\r\n",
        current_exe.display(),
        new_exe.display()
    );

    if let Err(e) = std::fs::write(&script, &script_content) {
        logger::error(&format!("创建更新脚本失败: {}", e));
        return;
    }

    match Command::new("cmd")
        .args(["/C", &script.to_string_lossy()])
        .creation_flags(0x08000000)
        .spawn()
    {
        Ok(_) => {
            logger::info("更新脚本已启动，程序即将退出");
            thread::sleep(Duration::from_millis(500));
            std::process::exit(0);
        }
        Err(e) => {
            logger::error(&format!("启动更新脚本失败: {}", e));
        }
    }
}