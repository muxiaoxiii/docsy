use std::path::PathBuf;

fn main() {
    println!("cargo:rustc-check-cfg=cfg(docsy_opencv)");
    println!("cargo:rerun-if-changed=src/ffmpeg/opencv_enhancer.cpp");
    configure_embedded_opencv();
    let windows_msvc = std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc");
    if windows_msvc {
        // Use the linker for the shared manifest instead of embedding the same
        // resource twice (winres on the application plus the linker on tests).
        tauri_build::try_build(
            tauri_build::Attributes::new()
                .windows_attributes(tauri_build::WindowsAttributes::new_without_app_manifest()),
        )
        .expect("Tauri build configuration failed");
        // Tauri embeds a v6 Common Controls manifest in the application, but
        // library test executables do not inherit that resource. Dialog/menu
        // dependencies import TaskDialogIndirect, which comctl32 v5 lacks.
        // Apply this to all final links so both unit and integration tests run.
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg=/MANIFESTDEPENDENCY:type='win32' name='Microsoft.Windows.Common-Controls' version='6.0.0.0' processorArchitecture='*' publicKeyToken='6595b64144ccf1df' language='*'");
    } else {
        tauri_build::build();
    }
}

fn configure_embedded_opencv() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let root = manifest_dir.join("vendor/opencv-mobile");
    let include_dir = root.join("include");
    if target_os == "macos" {
        if target_arch != "aarch64" {
            panic!("Docsy embedded OpenCV currently supports the macOS ARM64 release target");
        }
        let library_dir = root.join("macos-arm64/lib");
        if !library_dir.join("libopencv2.a").is_file() {
            panic!("embedded OpenCV static library is missing");
        }
        let mut build = cc::Build::new();
        build
            .cpp(true)
            .std("c++17")
            .include(&include_dir)
            .file(manifest_dir.join("src/ffmpeg/opencv_enhancer.cpp"))
            .flag("-fexceptions");
        build.compile("docsy_opencv_enhancer");
        println!("cargo:rustc-link-search=native={}", library_dir.display());
        println!("cargo:rustc-link-lib=static=opencv2");
        println!("cargo:rustc-link-lib=framework=Accelerate");
        println!("cargo:rustc-link-lib=framework=Foundation");
        println!("cargo:rustc-cfg=docsy_opencv");
        return;
    }
    if target_os == "windows" {
        if target_arch != "x86_64" {
            println!(
                "cargo:warning=embedded OpenCV is disabled for Windows {target_arch}; using the Rust frame-selection engine"
            );
            return;
        }
        let library_dir = root.join("windows-x64/lib");
        if !library_dir.join("opencv_imgproc4130.lib").is_file() {
            panic!("embedded OpenCV static libraries are incomplete");
        }
        cc::Build::new()
            .cpp(true)
            .std("c++17")
            .include(include_dir)
            .file(manifest_dir.join("src/ffmpeg/opencv_enhancer.cpp"))
            .compile("docsy_opencv_enhancer");
        println!("cargo:rustc-link-search=native={}", library_dir.display());
        println!("cargo:rustc-link-lib=static=opencv_imgproc4130");
        println!("cargo:rustc-link-lib=static=opencv_core4130");
        println!("cargo:rustc-link-lib=dylib=comctl32");
        println!("cargo:rustc-link-lib=dylib=gdi32");
        println!("cargo:rustc-link-lib=dylib=ole32");
        println!("cargo:rustc-link-lib=dylib=setupapi");
        println!("cargo:rustc-link-lib=dylib=ws2_32");
        println!("cargo:rustc-cfg=docsy_opencv");
        return;
    }
    println!("cargo:warning=embedded OpenCV is not configured for target OS {target_os}");
}
