use std::io::{self, Write};

/// Clears the terminal screen in a cross-platform way using secure method
pub fn clear_screen() {
    // Use clearscreen crate which doesn't rely on PATH
    clearscreen::clear().ok();
    
    // Ensure the screen is cleared before continuing
    io::stdout().flush().unwrap();
}

/// Shows the current wallet version
pub fn show_version() {
    println!("Rsk Rust Cli v{}", env!("CARGO_PKG_VERSION"));
}
