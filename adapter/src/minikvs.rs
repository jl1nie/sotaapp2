use async_trait::async_trait;
use chrono::{Duration, NaiveDateTime, TimeDelta, Utc};
use domain::repository::minikvs::KvsRepositry;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use shaku::Component;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

#[derive(Serialize, Deserialize, Debug)]
struct Entry {
    value: String,
    expires_at: NaiveDateTime,
}

#[derive(Debug)]
pub struct MiniKvs {
    store: Arc<Mutex<HashMap<String, Entry>>>,
    ttl: Duration,
    _handle: JoinHandle<()>,
}

impl MiniKvs {
    pub fn new(ttl_seconds: TimeDelta) -> Self {
        let store = Arc::new(Mutex::new(HashMap::<String, Entry>::new()));
        let ttl = Duration::seconds(ttl_seconds.num_seconds());
        let update = store.clone();
        let _handle = tokio::spawn(async move {
            loop {
                {
                    let mut update = update.lock().await;
                    let now = Utc::now().naive_utc();
                    update.retain(|_, value| value.expires_at > now);
                }
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            }
        });
        MiniKvs {
            store,
            ttl,
            _handle,
        }
    }

    async fn set(&self, key: String, value: Value, expire: Option<Duration>) {
        let expires_at = Utc::now().naive_utc() + expire.unwrap_or(self.ttl);
        // JSON変換失敗は致命的エラー（設計上発生しない）のためexpectを使用
        let value_str = serde_json::to_string(&value)
            .expect("Failed to serialize Value to JSON - this should never happen");
        let entry = Entry {
            value: value_str,
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
    kvs: Arc<MiniKvs>,
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
