/// Provides secure persistent storage for session data like cookies.
/// Implementations should handle platform-specific storage (e.g. NSUserDefaults on iOS)
/// and ensure data is stored securely as some of it may be session tokens.
#[uniffi::export(callback_interface)]
pub trait SecurePersistentStore: Send + Sync {
    /// Removes the entry for the given key
    fn remove_entry(&self, key: String);

    /// Gets the value for the given key, or None if not found
    fn get(&self, key: String) -> Option<Vec<u8>>;

    /// Sets the value for the given key
    fn set(&self, key: String, value: Vec<u8>);
}
