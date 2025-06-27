use anyhow::Result;
use qrcode::{QrCode, EcLevel};
use qrcode::render::unicode::Dense1x2;
use qrcode::render::unicode::Dense1x2::*;

/// Generates a QR code for the given text and returns it as a string
pub fn generate_qr_code(text: &str) -> Result<String> {
    // Create QR code with high error correction level
    let code = QrCode::with_error_correction_level(text, EcLevel::H)?;
    
    // Convert QR code to a string with unicode blocks
    let qr_string = code
        .render::<Dense1x2>()
        .dark_color(Dark)
        .light_color(Light)
        .build();
    
    Ok(qr_string)
}

/// Displays a QR code for a wallet address with a label
pub fn display_address_qr(address: &str, label: &str) -> Result<()> {
    // Create the URI for the QR code (using the standard ethereum: URI scheme)
    let uri = format!("ethereum:{}", address);
    
    // Generate the QR code
    let qr_code = generate_qr_code(&uri)?;
    
    // Display the QR code with the address below it
    println!("\n┌────────────────────────────────────────┐");
    println!("│{:^38}│", label);
    println!("├────────────────────────────────────────┤");
    
    // Print each line of the QR code with borders
    for line in qr_code.lines() {
        println!("│{:^38}│", line);
    }
    
    println!("├────────────────────────────────────────┤");
    println!("│{:^38}│", address);
    println!("└────────────────────────────────────────┘\n");
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_generate_qr_code() {
        let test_text = "0x742d35Cc6634C0532925a3b844Bc454e4438f44e";
        let result = generate_qr_code(test_text);
        assert!(result.is_ok());
        
        let qr_code = result.unwrap();
        assert!(!qr_code.is_empty());
    }
    
    #[test]
    fn test_display_address_qr() {
        let address = "0x742d35Cc6634C0532925a3b844Bc454e4438f44e";
        let result = display_address_qr(address, "Test Address");
        assert!(result.is_ok());
    }
}
