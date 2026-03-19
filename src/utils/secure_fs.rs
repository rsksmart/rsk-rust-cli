use std::fs;
use std::path::Path;
use anyhow::Result;

/// Write data to a file with secure permissions (0o600 - owner read/write only)
pub fn write_secure<P: AsRef<Path>>(path: P, contents: &str) -> Result<()> {
    let path = path.as_ref();
    
    // Create parent directories if they don't exist
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    
    // Write the file
    fs::write(path, contents)?;
    
    // Set secure permissions on Unix systems
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(path)?.permissions();
        perms.set_mode(0o600);
        fs::set_permissions(path, perms)?;
    }
    
    Ok(())
}

/// Create directory with secure permissions (0o700 - owner access only)
pub fn create_dir_secure<P: AsRef<Path>>(path: P) -> Result<()> {
    let path = path.as_ref();
    fs::create_dir_all(path)?;
    
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(path)?.permissions();
        perms.set_mode(0o700);
        fs::set_permissions(path, perms)?;
    }
    
    Ok(())
}
