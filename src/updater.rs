// 自动更新模块
// 启动时检查 GitHub Releases，有新版本自动下载替换

use serde::Deserialize;
use std::io;
use std::path::PathBuf;
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

/// 检查是否有新版本，有则下载更新
pub fn check_update() {
    println!("[更新] 检查新版本... (当前 {})", CURRENT_VERSION);

    // 查询最新 release
    let resp = match ureq::get(GITHUB_API)
        .set("User-Agent", "faster-chant-300-hero-rust")
        .set("Accept", "application/vnd.github.v3+json")
        .call()
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[更新] 检查失败: {}", e);
            return;
        }
    };

    let release: GitHubRelease = match resp.into_json() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[更新] 解析失败: {}", e);
            return;
        }
    };

    // 去掉版本号前缀 v
    let latest = release.tag_name.trim_start_matches('v');
    if latest == CURRENT_VERSION {
        println!("[更新] 已是最新版本");
        return;
    }

    println!("[更新] 发现新版本: {} -> {}", CURRENT_VERSION, latest);

    // 找到 exe 资产
    let exe = match release.assets.iter().find(|a| a.name.ends_with(".exe")) {
        Some(a) => a,
        None => {
            eprintln!("[更新] 未找到 exe 下载链接");
            return;
        }
    };

    println!("[更新] 下载中: {} ...", exe.name);
    let resp = match ureq::get(&exe.browser_download_url)
        .set("User-Agent", "faster-chant-300-hero-rust")
        .call()
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[更新] 下载失败: {}", e);
            return;
        }
    };

    let mut data = Vec::new();
    if let Err(e) = resp.into_reader().read_to_end(&mut data) {
        eprintln!("[更新] 读取失败: {}", e);
        return;
    }

    // 保存到临时目录
    let tmp_dir = match std::env::temp_dir().join("faster-chant-update").as_path().to_owned() {
        p => {
            let _ = std::fs::create_dir_all(&p);
            p
        }
    };
    let new_exe = tmp_dir.join("faster-chant-300-hero-rust.exe");
    if let Err(e) = std::fs::write(&new_exe, &data) {
        eprintln!("[更新] 写入失败: {}", e);
        return;
    }

    println!("[更新] 下载完成，准备替换...");

    // 获取当前 exe 路径
    let current_exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(_) => {
            eprintln!("[更新] 无法获取当前 exe 路径");
            return;
        }
    };

    // 创建替换脚本 (bat)
    let script = tmp_dir.join("update.bat");
    let script_content = format!(
        "@echo off\r\n\
         echo 正在更新 faster-chant-300-hero-rust...\r\n\
         timeout /t 2 /nobreak > nul\r\n\
         move /Y \"{}\" \"{}\"\r\n\
         if %errorlevel% equ 0 (\r\n\
             echo 更新完成，正在启动...\r\n\
             start \"\" \"{}\"\r\n\
         ) else (\r\n\
             echo 更新失败，请手动替换\r\n\
             pause\r\n\
         )\r\n\
         del \"%~f0\"\r\n",
        new_exe.display(), current_exe.display(), current_exe.display()
    );

    let script_path = tmp_dir.join("update.bat");
    if let Err(e) = std::fs::write(&script_path, &script_content) {
        eprintln!("[更新] 创建更新脚本失败: {}", e);
        return;
    }

    // 执行替换脚本
    match Command::new("cmd")
        .args(["/C", &script_path.to_string_lossy()])
        .spawn()
    {
        Ok(_) => {
            println!("[更新] 更新脚本已启动，程序即将退出...");
            thread::sleep(Duration::from_millis(500));
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("[更新] 启动更新脚本失败: {}", e);
        }
    }
}