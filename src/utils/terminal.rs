use std::process::Command;
use std::io::{self, Write};

/// Clears the terminal screen in a cross-platform way
pub fn clear_screen() {
    if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(["/c", "cls"])
            .status()
            .unwrap();
    } else {
        // For Unix-like systems
        Command::new("clear")
            .status()
            .unwrap();
    }
    
    // Ensure the screen is cleared before continuing
    io::stdout().flush().unwrap();
}

/// Shows the current wallet version
pub fn show_version() {
    println!("Rootstock Wallet v{}", env!("CARGO_PKG_VERSION"));
}
