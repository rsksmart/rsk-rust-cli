use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=methods/guest");
    // Disable Docker-based proving; we use the local native prover.
    std::env::set_var("RISC0_USE_DOCKER", "0");

    // Check that the risc0 Rust toolchain is installed before attempting to build.
    // This gives contributors a clear, actionable error instead of a cryptic
    // "rustup run risc0 cargo build" failure.
    check_risc0_toolchain();

    // Build the guest for the RISC-V zkvm target using the risc0 toolchain.
    // Clear toolchain environment variables to prevent the host toolchain from leaking in.
    let status = Command::new("rustup")
        .env_remove("RUSTC")
        .env_remove("RUSTDOC")
        .env_remove("RUSTUP_TOOLCHAIN")
        .env_remove("CARGO")
        .current_dir("methods/guest")
        .args([
            "run",
            "risc0",
            "cargo",
            "build",
            "--target",
            "riscv32im-risc0-zkvm-elf",
            "--release",
        ])
        .status()
        .expect("Failed to spawn `rustup run risc0 cargo build`. Is rustup in PATH?");

    if !status.success() {
        panic!(
            "Guest circuit build failed.\n\
             \n\
             Ensure the RISC Zero toolchain is installed:\n\
             \n\
             cargo install rzup          # install the risc0 toolchain manager\n\
             rzup install                # install the risc0 Rust toolchain\n\
             \n\
             After installation, re-run `cargo build --features zk`."
        );
    }

    // Locate compiled ELF binaries and generate the methods.rs constants file.
    let target = "riscv32im-risc0-zkvm-elf";
    let project_dir = env::current_dir().unwrap();
    let target_dir = project_dir.join("target").join(target).join("release");

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set by Cargo");
    let out_path = Path::new(&out_dir);
    let methods_file_path = out_path.join("methods.rs");

    let mut content = String::new();
    let binaries = ["transfer", "vote"];

    for bin in binaries {
        let elf_path = target_dir.join(bin);
        if !elf_path.exists() {
            panic!(
                "Guest ELF binary not found at {:?}.\n\
                 \n\
                 This usually means the guest build succeeded but produced no output.\n\
                 Try running the guest build manually:\n\
                 \n\
                 cd zk-circuits/methods/guest\n\
                 rustup run risc0 cargo build --target riscv32im-risc0-zkvm-elf --release",
                elf_path
            );
        }

        let elf_bytes = fs::read(&elf_path)
            .unwrap_or_else(|e| panic!("Failed to read ELF at {:?}: {}", elf_path, e));

        let dest_elf_path = out_path.join(bin);
        fs::write(&dest_elf_path, &elf_bytes)
            .unwrap_or_else(|e| panic!("Failed to copy ELF to {:?}: {}", dest_elf_path, e));

        let upper = bin.to_uppercase();
        content.push_str(&format!(
            "pub const {}_ELF: &[u8] = include_bytes!({:?});\n",
            upper, dest_elf_path
        ));

        let image_id = risc0_zkvm::compute_image_id(&elf_bytes)
            .unwrap_or_else(|e| panic!("Failed to compute image ID for {}: {}", bin, e));
        content.push_str(&format!(
            "pub const {}_ID: [u32; 8] = {:?};\n",
            upper,
            image_id.as_words()
        ));
    }

    fs::write(&methods_file_path, content)
        .unwrap_or_else(|e| panic!("Failed to write {:?}: {}", methods_file_path, e));
}

/// Verify that the `risc0` rustup toolchain is listed before trying to use it.
fn check_risc0_toolchain() {
    let result = Command::new("rustup")
        .args(["toolchain", "list"])
        .output();

    match result {
        Err(e) => {
            panic!(
                "Could not run `rustup toolchain list`: {}\n\
                 \n\
                 Ensure rustup is installed and in PATH:\n\
                 https://rustup.rs",
                e
            );
        }
        Ok(out) => {
            let list = String::from_utf8_lossy(&out.stdout);
            if !list.contains("risc0") {
                panic!(
                    "The 'risc0' rustup toolchain is not installed.\n\
                     \n\
                     Install it with:\n\
                     \n\
                     cargo install rzup          # install the risc0 toolchain manager\n\
                     rzup install                # install the risc0 Rust toolchain\n\
                     \n\
                     Detected toolchains:\n\
                     {}\n\
                     \n\
                     After installation, re-run `cargo build --features zk`.",
                    list
                );
            }
        }
    }
}
