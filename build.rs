use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    if target_os == "macos" && env::var("CARGO_FEATURE_APPLE").is_ok() {
        println!("cargo:rerun-if-changed=src/translate/apple_bridge.swift");
        println!("cargo:rerun-if-changed=src/transcribe/swift/Sources/SoksakBridge/Bridge.swift");

        let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

        // --- Build apple_bridge.swift (Translate) ---
        let swift_file = "src/translate/apple_bridge.swift";
        let object_file = out_dir.join("apple_bridge.o");

        let status = Command::new("swiftc")
            .arg("-emit-object")
            .arg("-o")
            .arg(&object_file)
            .arg(swift_file)
            .arg("-parse-as-library")
            .status()
            .expect("Failed to run swiftc");

        if !status.success() {
            panic!("swiftc failed");
        }

        let lib_path = out_dir.join("libapple_bridge.a");
        let status = Command::new("ar")
            .arg("crus")
            .arg(&lib_path)
            .arg(&object_file)
            .status()
            .expect("Failed to run ar");

        if !status.success() {
            panic!("ar failed");
        }

        println!("cargo:rustc-link-search={}", out_dir.display());
        println!("cargo:rustc-link-lib=static=apple_bridge");

        // --- Build SoksakBridge (WhisperKit) ---
        let swift_package_dir = "src/transcribe/swift";
        let status = Command::new("swift")
            .arg("build")
            .arg("-c")
            .arg("release")
            .current_dir(swift_package_dir)
            .status()
            .expect("Failed to run swift build for SoksakBridge");

        if !status.success() {
            panic!("swift build failed for SoksakBridge");
        }

        // The build output is usually in .build/release
        // We need to find where the static lib is.
        // SwiftPM 5.9+ might put it in .build/release/libSoksakBridge.a or similar.
        // Let's assume standard layout.
        let swift_build_dir = PathBuf::from(swift_package_dir).join(".build/release");
        println!("cargo:rustc-link-search={}", swift_build_dir.display());
        println!("cargo:rustc-link-lib=static=SoksakBridge");
        // WhisperKit dependencies
        // Note: WhisperKit might be a dynamic lib or static depending on how it's built.
        // If it's a package dependency, it might be embedded or we might need to link its products.
        // Actually, `libSoksakBridge.a` should contain the bridge code, but we need to link WhisperKit.
        // Since we defined SoksakBridge as a static library depending on WhisperKit,
        // SwiftPM might bundle it or we might need to link object files.
        // A safer bet for SwiftPM integration in Rust is often to build a single static lib that includes dependencies
        // or rely on the fact that we are linking against the swift build directory.

        // Let's check if we need to link WhisperKit explicitly.
        // Usually `swift build` produces `libSoksakBridge.a` which might NOT include WhisperKit symbols if it's a separate product.
        // However, since we are linking a static library from SwiftPM, we might need to link all transitive dependencies.
        // This can be complex.
        // A common trick is to make a "fat" static lib or just link all .a files found in the build dir.
        // For now, let's try linking SoksakBridge and see if it works.
        // If symbols are missing, we might need to link `WhisperKit` explicitly if it exists as a .a

        // Actually, we should probably just link the frameworks it needs.
        println!("cargo:rustc-link-lib=framework=Translation");
        println!("cargo:rustc-link-lib=framework=Foundation");
        println!("cargo:rustc-link-lib=framework=NaturalLanguage");
        println!("cargo:rustc-link-lib=framework=CoreML");
        println!("cargo:rustc-link-lib=framework=Accelerate");
        println!("cargo:rustc-link-lib=framework=AVFoundation");

        // Swift runtime libraries might be needed
        println!("cargo:rustc-link-lib=swiftCore");
        println!("cargo:rustc-link-lib=swiftFoundation");
        println!("cargo:rustc-link-lib=swift_Concurrency");

        println!("cargo:rustc-link-lib=static=swiftCompatibility56");
        println!("cargo:rustc-link-lib=static=swiftCompatibilityConcurrency");

        // Add search path for swift libs
        println!("cargo:rustc-link-search=/usr/lib/swift");
        println!(
            "cargo:rustc-link-search=/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/lib/swift/macosx"
        );

        println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");
        println!(
            "cargo:rustc-link-arg=-Wl,-rpath,/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/lib/swift/macosx"
        );
    }
}
