use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    if target_os == "macos" {
        println!("cargo:rerun-if-changed=src/translate/apple_bridge.swift");

        let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
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

        println!("cargo:rustc-link-search={}", out_dir.display());
        // Link the object file directly is tricky in Rust without a library archive,
        // but we can use `cc` to bundle it or just tell rustc to link the object.
        // Actually, creating a static lib is safer.

        // Let's try creating a static library from the object file
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

        println!("cargo:rustc-link-lib=static=apple_bridge");
        println!("cargo:rustc-link-lib=framework=Translation");
        println!("cargo:rustc-link-lib=framework=Foundation");
        println!("cargo:rustc-link-lib=framework=NaturalLanguage");
        // Swift runtime libraries might be needed
        println!("cargo:rustc-link-lib=swiftCore");
        println!("cargo:rustc-link-lib=swiftFoundation");
        println!("cargo:rustc-link-lib=swift_Concurrency");

        println!("cargo:rustc-link-lib=swift_Concurrency");

        // Add search path for swift libs
        println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");
        println!(
            "cargo:rustc-link-arg=-Wl,-rpath,/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/lib/swift/macosx"
        );
    }
}
