use async_trait::async_trait;
use bytes::Bytes;
use chrono::{Duration, NaiveDateTime, TimeDelta, Utc};
use domain::repository::minikvs::KvsRepositry;
use shaku::Component;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

#[derive(Debug)]
struct Entry {
    value: Bytes,
    expires_at: NaiveDateTime,
    /// 最後に使った順番(大きいほど新しい)。上限を超えたときに最も小さいものを捨てる
    last_used: u64,
}

#[derive(Debug, Default)]
struct Store {
    entries: HashMap<String, Entry>,
    clock: u64,
}

impl Store {
    fn tick(&mut self) -> u64 {
        self.clock += 1;
        self.clock
    }

    fn remove_expired(&mut self, now: NaiveDateTime) {
        self.entries.retain(|_, value| value.expires_at > now);
    }

    /// 上限に達していたら、期限切れを捨て、それでも足りなければ最も長く使われていないものを捨てる。
    /// 上限は数百件なので、捨てる対象は全件をなめて探す
    fn make_room(&mut self, capacity: usize, now: NaiveDateTime) {
        if self.entries.len() < capacity {
            return;
        }
        self.remove_expired(now);
        while self.entries.len() >= capacity {
            let Some(oldest) = self
                .entries
                .iter()
                .min_by_key(|(_, e)| e.last_used)
                .map(|(k, _)| k.clone())
            else {
                break;
            };
            self.entries.remove(&oldest);
        }
    }
}

#[derive(Debug)]
pub struct MiniKvs {
    store: Arc<Mutex<Store>>,
    ttl: Duration,
    capacity: usize,
    _handle: JoinHandle<()>,
}

impl MiniKvs {
    pub fn new(ttl_seconds: TimeDelta, capacity: usize) -> Self {
        let store = Arc::new(Mutex::new(Store::default()));
        let ttl = Duration::seconds(ttl_seconds.num_seconds());
        let update = store.clone();
        let _handle = tokio::spawn(async move {
            loop {
                update.lock().await.remove_expired(Utc::now().naive_utc());
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            }
        });
        MiniKvs {
            store,
            ttl,
            capacity: capacity.max(1),
            _handle,
        }
    }

    async fn set(&self, key: String, value: Bytes, expire: Option<Duration>) {
        let now = Utc::now().naive_utc();
        let expires_at = now + expire.unwrap_or(self.ttl);
        let mut store = self.store.lock().await;
        self.insert(&mut store, key, value, expires_at, now);
    }

    fn insert(
        &self,
        store: &mut Store,
        key: String,
        value: Bytes,
        expires_at: NaiveDateTime,
        now: NaiveDateTime,
    ) {
        if !store.entries.contains_key(&key) {
            store.make_room(self.capacity, now);
        }
        let last_used = store.tick();
        store.entries.insert(
            key,
            Entry {
                value,
                expires_at,
                last_used,
            },
        );
    }

    /// キーがまだ無い(または期限切れの)ときだけ入れる。入れられたら true
    pub async fn insert_new(&self, key: String, value: Bytes, expire: Duration) -> bool {
        let now = Utc::now().naive_utc();
        let mut store = self.store.lock().await;
        if store
            .entries
            .get(&key)
            .is_some_and(|entry| now <= entry.expires_at)
        {
            return false;
        }
        self.insert(&mut store, key, value, now + expire, now);
        true
    }

    /// 取り出して消す(1 回だけ使える値に使う)
    pub async fn take(&self, key: &str) -> Option<Bytes> {
        let mut store = self.store.lock().await;
        let entry = store.entries.remove(key)?;
        (Utc::now().naive_utc() <= entry.expires_at).then_some(entry.value)
    }

    async fn get(&self, key: &str) -> Option<Bytes> {
        let mut store = self.store.lock().await;
        let last_used = store.tick();
        match store.entries.get_mut(key) {
            Some(entry) if Utc::now().naive_utc() <= entry.expires_at => {
                entry.last_used = last_used;
                // Bytes の複製は参照カウントを増やすだけで、中身はコピーしない
                Some(entry.value.clone())
            }
            Some(_) => {
                store.entries.remove(key);
                None
            }
            None => None,
        }
    }
}

#[derive(Component)]
#[shaku(interface = KvsRepositry)]
pub struct MiniKvsRepositryImpl {
    kvs: Arc<MiniKvs>,
}

#[async_trait]
impl KvsRepositry for MiniKvsRepositryImpl {
    async fn set(&self, key: String, value: Bytes, expire: Option<Duration>) {
        self.kvs.set(key, value, expire).await;
    }

    async fn get(&self, key: &str) -> Option<Bytes> {
        self.kvs.get(key).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kvs(capacity: usize) -> MiniKvs {
        MiniKvs::new(TimeDelta::hours(1), capacity)
    }

    async fn len(kvs: &MiniKvs) -> usize {
        kvs.store.lock().await.entries.len()
    }

    #[tokio::test]
    async fn get_returns_what_was_set() {
        let kvs = kvs(4);
        kvs.set("a".into(), Bytes::from_static(b"{\"x\":1}"), None)
            .await;
        assert_eq!(kvs.get("a").await, Some(Bytes::from_static(b"{\"x\":1}")));
        assert_eq!(kvs.get("b").await, None);
    }

    #[tokio::test]
    async fn expired_entry_is_not_returned() {
        let kvs = kvs(4);
        kvs.set(
            "a".into(),
            Bytes::from_static(b"1"),
            Some(Duration::seconds(-1)),
        )
        .await;
        assert_eq!(kvs.get("a").await, None);
        assert_eq!(len(&kvs).await, 0);
    }

    // 別のキーをいくら入れても、エントリ数は上限を超えない
    #[tokio::test]
    async fn entries_never_exceed_capacity() {
        let kvs = kvs(3);
        for i in 0..100 {
            kvs.set(format!("k{i}"), Bytes::from_static(b"v"), None)
                .await;
            assert!(len(&kvs).await <= 3);
        }
        assert_eq!(kvs.get("k99").await, Some(Bytes::from_static(b"v")));
        assert_eq!(kvs.get("k0").await, None);
    }

    // 上限を超えたら、最も長く使われていないものから捨てる(最近 get したものは残る)
    #[tokio::test]
    async fn least_recently_used_is_evicted_first() {
        let kvs = kvs(2);
        kvs.set("a".into(), Bytes::from_static(b"a"), None).await;
        kvs.set("b".into(), Bytes::from_static(b"b"), None).await;
        assert!(kvs.get("a").await.is_some());
        kvs.set("c".into(), Bytes::from_static(b"c"), None).await;
        assert!(kvs.get("a").await.is_some());
        assert_eq!(kvs.get("b").await, None);
        assert!(kvs.get("c").await.is_some());
    }

    // 上限に達していても、期限切れがあればそちらを先に捨てる
    #[tokio::test]
    async fn expired_entries_are_dropped_before_live_ones() {
        let kvs = kvs(2);
        kvs.set(
            "old".into(),
            Bytes::from_static(b"o"),
            Some(Duration::seconds(-1)),
        )
        .await;
        kvs.set("live".into(), Bytes::from_static(b"l"), None).await;
        kvs.set("new".into(), Bytes::from_static(b"n"), None).await;
        assert!(kvs.get("live").await.is_some());
        assert!(kvs.get("new").await.is_some());
    }

    // 同じキーの上書きでは何も捨てない
    #[tokio::test]
    async fn overwriting_a_key_does_not_evict() {
        let kvs = kvs(2);
        kvs.set("a".into(), Bytes::from_static(b"1"), None).await;
        kvs.set("b".into(), Bytes::from_static(b"1"), None).await;
        kvs.set("a".into(), Bytes::from_static(b"2"), None).await;
        assert_eq!(kvs.get("a").await, Some(Bytes::from_static(b"2")));
        assert!(kvs.get("b").await.is_some());
    }

    // insert_new は、まだ使われているキーには入れない
    #[tokio::test]
    async fn insert_new_does_not_overwrite_live_key() {
        let kvs = kvs(4);
        assert!(
            kvs.insert_new("a".into(), Bytes::from_static(b"1"), Duration::minutes(1))
                .await
        );
        assert!(
            !kvs.insert_new("a".into(), Bytes::from_static(b"2"), Duration::minutes(1))
                .await
        );
        assert_eq!(kvs.take("a").await, Some(Bytes::from_static(b"1")));
    }

    // 期限切れのキーには insert_new で入れ直せる
    #[tokio::test]
    async fn insert_new_reuses_expired_key() {
        let kvs = kvs(4);
        assert!(
            kvs.insert_new("a".into(), Bytes::from_static(b"1"), Duration::seconds(-1))
                .await
        );
        assert!(
            kvs.insert_new("a".into(), Bytes::from_static(b"2"), Duration::minutes(1))
                .await
        );
        assert_eq!(kvs.take("a").await, Some(Bytes::from_static(b"2")));
    }

    // take は 1 回しか取り出せず、期限切れは取り出せない
    #[tokio::test]
    async fn take_returns_value_only_once() {
        let kvs = kvs(4);
        kvs.insert_new("a".into(), Bytes::from_static(b"1"), Duration::minutes(1))
            .await;
        assert_eq!(kvs.take("a").await, Some(Bytes::from_static(b"1")));
        assert_eq!(kvs.take("a").await, None);
        kvs.insert_new("b".into(), Bytes::from_static(b"1"), Duration::seconds(-1))
            .await;
        assert_eq!(kvs.take("b").await, None);
    }
}
