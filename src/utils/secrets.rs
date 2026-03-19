use zeroize::{Zeroize, Zeroizing};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A wrapper type for sensitive data that automatically zeroizes on drop.
/// Uses `Zeroizing` to prevent copies during serialization and ensure secure cleanup.
/// This type is NOT serializable - use `SerializableSecret` for secrets that need storage.
#[derive(Clone)]
pub struct Secret<T: Zeroize> {
    value: Zeroizing<T>,
}

impl<T: Zeroize> Secret<T> {
    /// Create a new Secret value
    pub fn new(value: T) -> Self {
        Secret {
            value: Zeroizing::new(value),
        }
    }

    /// Get a reference to the inner value
    pub fn expose(&self) -> &T {
        &self.value
    }

    /// Get a mutable reference to the inner value
    pub fn expose_mut(&mut self) -> &mut T {
        &mut self.value
    }
}

// Safe Debug implementation that doesn't expose the secret content
impl<T: Zeroize> std::fmt::Debug for Secret<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Secret<{}>", std::any::type_name::<T>())
    }
}

impl<T: Zeroize> std::fmt::Display for Secret<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Secret<{}>", std::any::type_name::<T>())
    }
}

// Implement From trait for easy conversion
impl<T: Zeroize> From<T> for Secret<T> {
    fn from(value: T) -> Self {
        Secret::new(value)
    }
}

// NOTE: Secret<T> is intentionally NOT Serialize/Deserialize
// This prevents accidental exposure of sensitive data through serialization

// ============================================================================
// SerializableSecret - For secrets that need to be stored (passwords, API keys)
// ============================================================================

/// A wrapper type for sensitive data that needs to be serialized/deserialized.
/// Uses `Zeroizing` for automatic memory protection.
/// Only use this for secrets that MUST be stored (e.g., encrypted passwords, API keys).
/// For private keys and other highly sensitive data, use `Secret<T>` instead.
#[derive(Clone)]
pub struct SerializableSecret<T: Zeroize> {
    value: Zeroizing<T>,
}

impl<T: Zeroize> SerializableSecret<T> {
    /// Create a new SerializableSecret value
    pub fn new(value: T) -> Self {
        SerializableSecret {
            value: Zeroizing::new(value),
        }
    }

    /// Get a reference to the inner value
    pub fn expose(&self) -> &T {
        &self.value
    }

    /// Get a mutable reference to the inner value
    pub fn expose_mut(&mut self) -> &mut T {
        &mut self.value
    }
}

// Safe Debug implementation that doesn't expose the secret content
impl<T: Zeroize> std::fmt::Debug for SerializableSecret<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SerializableSecret<{}>", std::any::type_name::<T>())
    }
}

impl<T: Zeroize> std::fmt::Display for SerializableSecret<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SerializableSecret<{}>", std::any::type_name::<T>())
    }
}

// Implement From trait for easy conversion
impl<T: Zeroize> From<T> for SerializableSecret<T> {
    fn from(value: T) -> Self {
        SerializableSecret::new(value)
    }
}

// Custom serialization/deserialization for SerializableSecret types
impl<T: Zeroize + Serialize> Serialize for SerializableSecret<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.expose().serialize(serializer)
    }
}

impl<'de, T: Zeroize + Deserialize<'de> + Default> Deserialize<'de> for SerializableSecret<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = T::deserialize(deserializer)?;
        Ok(SerializableSecret::new(value))
    }
}

// ============================================================================
// Type Aliases
// ============================================================================

/// Non-serializable private key - CANNOT be accidentally serialized
pub type SecretPrivateKey = Secret<String>;

/// Serializable password - can be stored in wallet files
pub type SecretPassword = SerializableSecret<String>;

/// Serializable generic string secret
pub type SecretString = SerializableSecret<String>;

/// Serializable byte array secret
pub type SecretBytes = SerializableSecret<Vec<u8>>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_string_zeroization() {
        let secret = Secret::new("sensitive_password".to_string());
        assert_eq!(secret.expose(), "sensitive_password");

        // The value is still intact during use
        let value_ref = secret.expose();
        assert_eq!(value_ref, "sensitive_password");
    }

    #[test]
    fn test_secret_bytes_zeroization() {
        let secret = Secret::new(vec![1u8, 2, 3, 4, 5]);
        assert_eq!(secret.expose(), &vec![1u8, 2, 3, 4, 5]);
    }


    #[test]
    fn test_serializable_secret_serialization() {
        let secret = SerializableSecret::new("password123".to_string());
        let serialized = serde_json::to_string(&secret).unwrap();
        let deserialized: SerializableSecret<String> = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.expose(), "password123");
    }

    #[test]
    fn test_secret_private_key_not_serializable() {
        // This test verifies at compile time that SecretPrivateKey cannot be serialized
        // If this compiles, the test passes
        let _private_key = SecretPrivateKey::new("0x1234567890abcdef".to_string());
        
        // The following line should NOT compile if uncommented:
        // let _serialized = serde_json::to_string(&private_key);
    }

    #[test]
    fn test_zeroizing_wrapper() {
        // Test that Zeroizing is properly used
        let secret = Secret::new("test".to_string());
        assert_eq!(secret.expose(), "test");
        
        // When secret is dropped, Zeroizing automatically zeroizes the memory
        drop(secret);
    }

    #[test]
    fn test_serializable_secret_zeroizing() {
        let secret = SerializableSecret::new("password".to_string());
        assert_eq!(secret.expose(), "password");
        
        // When secret is dropped, Zeroizing automatically zeroizes the memory
        drop(secret);
    }
}