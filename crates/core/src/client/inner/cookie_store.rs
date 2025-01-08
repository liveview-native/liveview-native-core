use std::sync::Arc;

use log::{error, warn};
use reqwest::{cookie::CookieStore, header::HeaderValue, Url};

use crate::callbacks::SecurePersistentStore;

const COOKIE_STORE_KEY: &str = "COOKIE_CACHE";

pub struct PersistentCookieStore {
    store: Arc<reqwest_cookie_store::CookieStoreMutex>,
    persistent_store: Option<Arc<dyn SecurePersistentStore>>,
}

impl PersistentCookieStore {
    pub fn new(persistent_store: Option<Arc<dyn SecurePersistentStore>>) -> Self {
        if persistent_store.is_none() {
            warn!("No persistent store provided - cookies will not be persisted");
        }

        let cookie_store = if let Some(store) = &persistent_store {
            if let Some(binary_json) = store.get(COOKIE_STORE_KEY.to_owned()) {
                match cookie_store::serde::json::load(binary_json.as_slice()) {
                    Ok(store) => store,
                    Err(e) => {
                        error!(
                            "Failed to load cookie store: {} - defaulting to empty store",
                            e
                        );
                        reqwest_cookie_store::CookieStore::default()
                    }
                }
            } else {
                reqwest_cookie_store::CookieStore::default()
            }
        } else {
            warn!("No persistent store configured, no cookies will be loaded from disk.");
            reqwest_cookie_store::CookieStore::default()
        };

        let store = Arc::new(reqwest_cookie_store::CookieStoreMutex::new(cookie_store));

        Self {
            store,
            persistent_store,
        }
    }

    pub fn save(&self) {
        let Some(store) = &self.persistent_store else {
            warn!("No persistence provider while attempting to save, Cookies will not persist");
            return;
        };

        let mut buffer = Vec::new();
        let store_guard = self.store.lock().unwrap();

        if let Err(e) = cookie_store::serde::json::save(&store_guard, &mut buffer) {
            warn!("Failed to serialize cookie store: {}", e);
            return;
        }

        store.set(COOKIE_STORE_KEY.to_owned(), buffer)
    }
}

impl CookieStore for PersistentCookieStore {
    fn set_cookies(&self, cookie_headers: &mut dyn Iterator<Item = &HeaderValue>, url: &Url) {
        CookieStore::set_cookies(self.store.as_ref(), cookie_headers, url);
    }

    fn cookies(&self, url: &Url) -> Option<HeaderValue> {
        CookieStore::cookies(self.store.as_ref(), url)
    }
}

impl Drop for PersistentCookieStore {
    fn drop(&mut self) {
        self.save();
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Mutex};

    use super::*;

    #[derive(Default, Debug)]
    struct InMemoryStore(Mutex<HashMap<String, Vec<u8>>>);

    impl SecurePersistentStore for InMemoryStore {
        fn remove_entry(&self, key: String) {
            self.0.lock().unwrap().remove(&key);
        }

        fn get(&self, key: String) -> Option<Vec<u8>> {
            self.0.lock().unwrap().get(&key).cloned()
        }

        fn set(&self, key: String, value: Vec<u8>) {
            self.0.lock().unwrap().insert(key, value);
        }
    }

    #[test]
    fn test_cookie_persistence() {
        let _ = env_logger::builder()
            .parse_default_env()
            .is_test(true)
            .try_init();

        let store = Arc::new(InMemoryStore::default());
        let cookie_store = PersistentCookieStore::new(Some(store.clone()));

        let url = "https://example.com".parse().unwrap();

        let persistent_cookie =
            "session=123; Domain=example.com; Expires=Fri, 31 Dec 9999 23:59:59 GMT";

        let headers = [HeaderValue::from_static(persistent_cookie)];

        cookie_store.set_cookies(&mut headers.iter(), &url);
        cookie_store.save();

        let new_store = PersistentCookieStore::new(Some(store));
        assert!(new_store.cookies(&url).is_some());
    }

    #[test]
    fn test_no_persistence() {
        let _ = env_logger::builder()
            .parse_default_env()
            .is_test(true)
            .try_init();

        let store = PersistentCookieStore::new(None);
        let url = "https://example.com".parse().unwrap();
        let headers = [HeaderValue::from_static("session=123")];

        store.set_cookies(&mut headers.iter(), &url);
        store.save();
    }
}
