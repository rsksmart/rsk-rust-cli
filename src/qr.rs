use image::{ImageBuffer, Luma};
use qrcode::{EcLevel, QrCode};

pub fn generate_qr_code(data: &str, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let code = QrCode::with_error_correction_level(data.as_bytes(), EcLevel::M)?;
    let image: ImageBuffer<Luma<u8>, Vec<u8>> = code.render::<Luma<u8>>().build();

    image.save(path)?;
    Ok(())
}
