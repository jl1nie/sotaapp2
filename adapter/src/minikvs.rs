use async_trait::async_trait;
use chrono::{Duration, NaiveDateTime, TimeDelta, Utc};
use domain::repository::minikvs::KvsRepositry;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use shaku::Component;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Serialize, Deserialize, Debug)]
struct Entry {
    value: String, // JSON文字列として格納
    expires_at: NaiveDateTime,
}
#[derive(Debug, Clone)]
pub struct MiniKvs {
    store: Arc<Mutex<HashMap<String, Entry>>>,
    ttl: Duration,
}

impl MiniKvs {
    pub fn new(ttl_seconds: TimeDelta) -> Self {
        MiniKvs {
            store: Arc::new(Mutex::new(HashMap::new())),
            ttl: Duration::seconds(ttl_seconds.num_seconds()),
        }
    }

    async fn set(&self, key: String, value: Value, expire: Option<Duration>) {
        let expires_at = Utc::now().naive_utc() + expire.unwrap_or(self.ttl);
        let entry = Entry {
            value: serde_json::to_string(&value).unwrap(),
            expires_at,
        };
        let mut store = self.store.lock().await;
        store.insert(key, entry);
    }

    async fn get(&self, key: &str) -> Option<Value> {
        let mut store = self.store.lock().await;
        if let Some(entry) = store.get(key) {
            if Utc::now().naive_utc() <= entry.expires_at {
                return serde_json::from_str(&entry.value).ok();
            } else {
                store.remove(key);
            }
        }
        None
    }
}

#[derive(Component)]
#[shaku(interface = KvsRepositry)]
pub struct MiniKvsRepositryImpl {
    kvs: MiniKvs,
}

#[async_trait]
impl KvsRepositry for MiniKvsRepositryImpl {
    async fn set(&self, key: String, value: Value, expire: Option<Duration>) {
        self.kvs.set(key, value, expire).await;
    }

    async fn get(&self, key: &str) -> Option<Value> {
        self.kvs.get(key).await
    }
}
