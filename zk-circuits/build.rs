use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=methods/guest");
    std::env::set_var("RISC0_USE_DOCKER", "0");

    // 1. Build the guest for RISC-V ZKVM using the risc0 toolchain
    // We expect the 'risc0' toolchain to be installed via `rzup install` and linked to rustup.
    // Explicitly clean environment to avoid toolchain pollution.
    let status = Command::new("rustup")
        .env_remove("RUSTC")
        .env_remove("RUSTDOC")
        .env_remove("RUSTUP_TOOLCHAIN")
        .env_remove("CARGO")
        .current_dir("methods/guest")
        .args(&[
            "run",
            "risc0",
            "cargo",
            "build",
            "--target",
            "riscv32im-risc0-zkvm-elf",
            "--release",
        ])
        .status()
        .expect("Failed to run rustup run risc0 cargo build for guest");

    if !status.success() {
        panic!(
            "Guest build failed. Ensure you have the 'risc0' toolchain installed (rzup install)."
        );
    }

    // 2. Locate artifacts
    let target = "riscv32im-risc0-zkvm-elf";
    // Artifacts are in the workspace target directory, which is the current directory's target
    let project_dir = env::current_dir().unwrap();
    let target_dir = project_dir.join("target").join(target).join("release");

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
    let out_path = Path::new(&out_dir);
    let methods_file_path = out_path.join("methods.rs");

    let mut content = String::new();
    let binaries = ["transfer", "vote"];

    for bin in binaries {
        let elf_path = target_dir.join(bin);
        if !elf_path.exists() {
            panic!(
                "Guest binary not found at {:?}. Checked 'debug' dirs.",
                elf_path
            );
        }

        let elf_bytes = fs::read(&elf_path).expect("Failed to read ELF");

        let dest_elf_path = out_path.join(bin);
        fs::write(&dest_elf_path, &elf_bytes).expect("Failed to copy ELF");

        let upper = bin.to_uppercase();

        content.push_str(&format!(
            "pub const {}_ELF: &[u8] = include_bytes!({:?});\n",
            upper, dest_elf_path
        ));

        // Compute Image ID
        let image_id =
            risc0_zkvm::compute_image_id(&elf_bytes).expect("Failed to compute Image ID");
        content.push_str(&format!(
            "pub const {}_ID: [u32; 8] = {:?};\n",
            upper,
            image_id.as_words()
        ));
    }

    fs::write(&methods_file_path, content).expect("Failed to write methods.rs");
}
