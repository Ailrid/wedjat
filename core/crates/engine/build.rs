use std::env;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

fn main() {
    let project_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let ov_libs_path =
        PathBuf::from(&project_dir).join(".venv/lib/python3.11/site-packages/openvino/libs");

    // 检查路径...（保留你之前的逻辑）

    let libs_dir = ov_libs_path.to_str().expect("Path error");

    // --- 关键修正 1: 强制设置环境变量给依赖的 crate 看 ---
    println!("cargo:rustc-env=OPENVINO_INSTALL_DIR={}", libs_dir);
    // 有些 crate 依赖这个变量来寻找头文件和库
    unsafe {
        env::set_var("OPENVINO_INSTALL_DIR", libs_dir);
    }
    // --- 关键修正 2: 软链接处理 ---
    setup_openvino_symlinks(&ov_libs_path);

    // --- 关键修正 3: 链接参数 ---
    println!("cargo:rustc-link-search=native={}", libs_dir);
    // 注意：有些 openvino 绑定内部会自动发 link-lib 指令，
    // 手动发可能会导致冲突，但如果找不到库，手动发是安全的。
    println!("cargo:rustc-link-lib=dylib=openvino");
    println!("cargo:rustc-link-lib=dylib=openvino_c");

    // Rpath 确保运行时能找到 .so
    println!("cargo:rustc-link-arg=-Wl,-rpath,{}", libs_dir);

    println!("cargo:rerun-if-changed=build.rs");
}

fn setup_openvino_symlinks(dir: &Path) {
    // 定义我们需要生成的标准名称
    let targets = vec!["libopenvino.so", "libopenvino_c.so"];

    for target in targets {
        let dest_path = dir.join(target);

        // 核心修正：如果软链接已存在（哪怕是断开的），先删掉它
        if dest_path.exists() || dest_path.is_symlink() {
            let _ = fs::remove_file(&dest_path);
        }

        // 寻找带版本号的原文件，例如找到 libopenvino.so.2410
        let entries = fs::read_dir(dir).unwrap();
        let source_file = entries.flatten().find(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            name.starts_with(target) && name != target
        });

        if let Some(src) = source_file {
            println!("cargo:warning=Linking {} -> {:?}", target, src.file_name());
            if let Err(e) = symlink(src.path(), dest_path) {
                println!("cargo:warning=Symlink failed: {:?}", e);
            }
        }
    }
}
