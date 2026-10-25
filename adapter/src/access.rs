//! Cloudflare Access の署名(JWT)の検証。
//! 鍵は `{team_domain}/cdn-cgi/access/certs` の JWKS から取り、覚えておく。
//! 知らない `kid` が来たら取り直す(取り直しは `MIN_REFETCH` に 1 回まで)。
//! 設計: docs/summit-map-admin-design.md 2.2

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use common::error::{AppError, AppResult};
use domain::repository::access::{AccessIdentity, AccessTokenVerifier};
use jsonwebtoken::jwk::{Jwk, JwkSet};
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use serde::Deserialize;
use shaku::Component;
use tokio::sync::Mutex;

/// 鍵の取り直しの間隔の下限
const MIN_REFETCH: Duration = Duration::from_secs(60);

/// JWKS の取り先(テストで差し替える)
#[async_trait]
pub trait JwksSource: Send + Sync {
    async fn fetch(&self) -> AppResult<JwkSet>;
}

struct HttpJwks {
    url: String,
    client: reqwest::Client,
}

#[async_trait]
impl JwksSource for HttpJwks {
    async fn fetch(&self) -> AppResult<JwkSet> {
        let resp = self
            .client
            .get(&self.url)
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(|e| {
                tracing::error!("Access の鍵を取れません: {}", e);
                AppError::UnauthenticatedError
            })?;
        resp.json::<JwkSet>().await.map_err(|e| {
            tracing::error!("Access の鍵を読めません: {}", e);
            AppError::UnauthenticatedError
        })
    }
}

#[derive(Default)]
struct KeyCache {
    keys: HashMap<String, Jwk>,
    fetched_at: Option<Instant>,
}

/// 鍵を覚えておく入れ物。 で作って、Component に渡す
#[derive(Clone)]
pub struct AccessKeys {
    team_domain: Option<String>,
    aud: Option<String>,
    source: Arc<dyn JwksSource>,
    cache: Arc<Mutex<KeyCache>>,
    /// 取り直しを 1 度に 1 つにする
    fetch_lock: Arc<Mutex<()>>,
    min_refetch: Duration,
}

impl AccessKeys {
    /// どちらかが None なら、Access の検証は無効(Firebase だけ)
    pub fn new(team_domain: Option<String>, aud: Option<String>) -> Self {
        let team_domain = team_domain.map(|d| d.trim_end_matches('/').to_string());
        let url = format!(
            "{}/cdn-cgi/access/certs",
            team_domain.as_deref().unwrap_or_default()
        );
        Self::with_source(
            team_domain,
            aud,
            Arc::new(HttpJwks {
                url,
                client: reqwest::Client::new(),
            }),
            MIN_REFETCH,
        )
    }

    fn with_source(
        team_domain: Option<String>,
        aud: Option<String>,
        source: Arc<dyn JwksSource>,
        min_refetch: Duration,
    ) -> Self {
        Self {
            team_domain,
            aud,
            source,
            cache: Arc::new(Mutex::new(KeyCache::default())),
            fetch_lock: Arc::new(Mutex::new(())),
            min_refetch,
        }
    }

    /// kid の鍵を返す。知らなければ(間隔を空けて)取り直す。
    /// 取り直しの HTTP を待つあいだも、知っている kid の検証は待たせない
    async fn key_for(&self, kid: &str) -> AppResult<Jwk> {
        if let Some(k) = self.cache.lock().await.keys.get(kid) {
            return Ok(k.clone());
        }
        // 取り直しは 1 度に 1 つだけ。待っているあいだに済んでいれば、それを使う
        let _fetching = self.fetch_lock.lock().await;
        {
            let mut cache = self.cache.lock().await;
            if let Some(k) = cache.keys.get(kid) {
                return Ok(k.clone());
            }
            let may_fetch = cache
                .fetched_at
                .is_none_or(|t| t.elapsed() >= self.min_refetch);
            if !may_fetch {
                return Err(AppError::UnauthenticatedError);
            }
            // 取れなくても、次の取り直しは間隔を空ける
            cache.fetched_at = Some(Instant::now());
        }
        let set = self.source.fetch().await?;
        let mut cache = self.cache.lock().await;
        cache.keys = set
            .keys
            .into_iter()
            .filter_map(|k| k.common.key_id.clone().map(|id| (id, k)))
            .collect();
        cache
            .keys
            .get(kid)
            .cloned()
            .ok_or(AppError::UnauthenticatedError)
    }
}

#[derive(Deserialize)]
struct Claims {
    email: Option<String>,
}

#[derive(Component)]
#[shaku(interface = AccessTokenVerifier)]
pub struct AccessTokenVerifierImpl {
    keys: AccessKeys,
}

#[async_trait]
impl AccessTokenVerifier for AccessTokenVerifierImpl {
    fn is_enabled(&self) -> bool {
        self.keys.team_domain.is_some() && self.keys.aud.is_some()
    }

    async fn verify(&self, token: &str) -> AppResult<AccessIdentity> {
        let (Some(iss), Some(aud)) = (&self.keys.team_domain, &self.keys.aud) else {
            return Err(AppError::UnauthenticatedError);
        };
        let header = decode_header(token).map_err(|_| AppError::UnauthenticatedError)?;
        if header.alg != Algorithm::RS256 {
            return Err(AppError::UnauthenticatedError);
        }
        let kid = header.kid.ok_or(AppError::UnauthenticatedError)?;
        let jwk = self.keys.key_for(&kid).await?;
        let key = DecodingKey::from_jwk(&jwk).map_err(|_| AppError::UnauthenticatedError)?;

        let mut v = Validation::new(Algorithm::RS256);
        v.set_audience(&[aud]);
        v.set_issuer(&[iss]);
        v.set_required_spec_claims(&["exp", "iss", "aud"]);
        v.validate_nbf = true;
        let data = decode::<Claims>(token, &key, &v).map_err(|e| {
            tracing::warn!("Access の署名を確かめられません: {}", e);
            AppError::UnauthenticatedError
        })?;
        // サービストークンなど、メールのない署名は管理画面には使わない
        let email = data
            .claims
            .email
            .map(|e| e.trim().to_lowercase())
            .filter(|e| !e.is_empty())
            .ok_or(AppError::UnauthenticatedError)?;
        Ok(AccessIdentity { email })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{encode, EncodingKey, Header};
    use serde_json::json;
    use std::sync::atomic::{AtomicUsize, Ordering};

    // テスト専用の鍵(本番では使っていない)
    const KEY1_PEM: &str = "-----BEGIN PRIVATE KEY-----
MIIEvAIBADANBgkqhkiG9w0BAQEFAASCBKYwggSiAgEAAoIBAQDK21nz83Y8Edws
C2SoxdjDLSUf3C52GCDPHGVNdgDxNAXkZ28/kz31FrfRQhqS4Lhxz5ErMrxaD9qt
CZKPARiGsiLWz1zIfz5lEeBfvcBwzV4Rb+Jyp1+GXVEQxKoO7lPf9AXAZqL/gFRT
iziD3+zpxiiaGetlHeyIMSUmm7AC0wCVgxTg+9Ts4ts/hB/4Yng7n6p2KZypjNjZ
qxiPLP5UwpFnSA1MGCPYnZ3sTok8tYOj3EPKc+MZJMiv80KMlI2tu4xX0RL0vikn
iThYt6/BoQ6AGXgPSEMQaPPP93nJgLlFe5xZScc8F6rwS1J1bUyujDivggo3OT/l
ACYoVyIrAgMBAAECggEATXwV0vn2+qDz0ZsUeub/kiXHXXEUCiXTOmklRL5DQ97U
NVdRBh6u9tQTqImwbtQJf+eCEtcRXPFbXOcgJSGNBb47UoPoc+kuoxp98vQe8rdv
LAfpnlPSSBeS5G3Ka2ObhBrIq6JItoGFNFgkYTMCYpU/YwVthbU1aRwkjafARoM2
QcS5PIq+k236EcGdFnoxIb4xQ4PoIOsdkrKE0/MNsK4wZKFeWeYEg1/B7nQpD2dM
0UR35AtN5RS+iW23DW9yFdjcIX+0weJfg4VIYN8TUKfpWTQUqQ138xS/7bOkRDLv
Q+esHedu630Vz5K5mPtaFflXkR6/6+0jVfD1BTPyaQKBgQDoJ9YQbexTLA0i5Muw
85vYN+66hFa5Oicm5Kam1yYm9yb9ox4uSEg3piwRT361+bcaP8XhXxho9Xa05hoG
M4E/lTwlu1cHWVqlHz2GdieVoSiDyZgKf3ZCxrcCSJQ49doPuqHvEP+fVtfdGXcs
bRq29sr63FP/wHetLnd9/9+p5wKBgQDfsSaHU1PznQQXm1MR93+/yfBUz8SN9B/f
sSIGLBQJgkf8K6NCiVX6RmHo6i9A8gRE6t6rOhBIQsMTgrXgt4rwh8sRImzUgCTd
OAO+Axa4yJT97wiFfY3xmNSxag//5nx2hiWRSVuaozyzzlXRoD99w5UhCvZQ9V6u
HoYsFqmlHQKBgBIydSwKgbivPkRa7MBtB+01zSefkfCo50K8L3fqa1ooi0zcSVRV
ZBvAkbm1onf/H+QV1ZvpyU8I1ootcyLDTMFAIFvQyGySC3Hf9gGhJ08qN7PEzH8e
KJXpX5T1PXENjE8pUOIdUfwaYYc6JMY9sxdeVuC8/nSQkJDpkII22UBHAoGAZLQb
5FBfOw01CmuiujPyDyWaTJQ8qKt+rfXQm3a8jotYXO5IUWwCiOmmqnTDUTmjMmOK
WwuIyP9FU0NFHiWDJ0X65XvaAtCtx+mvxZseX5HtgWRbMb2UaW+kUTqueUYKQlGi
GAYudy+F5Gy/td+uta+XM4j72vab6UdUoAST2wECgYA/fsqFh6/VwwLJs95AOQQV
cw5nkU80BJP1qM+srwTmNelHMmYhMmuYmqG9mFd5Wxf64iH3pRpKUbxT59Zl3Eml
abuyAcTnokDrmaItKmFUzQhMYJLRIyZWP/SU1EFkSlHNKdZuoo/JJTK1Mz3+9C58
kAaDHz4nuPBPghJ7w+WTxA==
-----END PRIVATE KEY-----";
    const KEY1_N: &str = "yttZ8_N2PBHcLAtkqMXYwy0lH9wudhggzxxlTXYA8TQF5GdvP5M99Ra30UIakuC4cc-RKzK8Wg_arQmSjwEYhrIi1s9cyH8-ZRHgX73AcM1eEW_icqdfhl1REMSqDu5T3_QFwGai_4BUU4s4g9_s6cYomhnrZR3siDElJpuwAtMAlYMU4PvU7OLbP4Qf-GJ4O5-qdimcqYzY2asYjyz-VMKRZ0gNTBgj2J2d7E6JPLWDo9xDynPjGSTIr_NCjJSNrbuMV9ES9L4pJ4k4WLevwaEOgBl4D0hDEGjzz_d5yYC5RXucWUnHPBeq8EtSdW1Mrow4r4IKNzk_5QAmKFciKw";
    const KEY2_PEM: &str = "-----BEGIN PRIVATE KEY-----
MIIEvAIBADANBgkqhkiG9w0BAQEFAASCBKYwggSiAgEAAoIBAQC7VSG11p6dj6GE
U+TJRpTuaDjKmyDiGwpEBYHlUs7a7XOVQUciBgWzDPEzv+0sAXPgnN34nlM7jTQL
L47MRJT3aH/zp8mq9qd80umpfE5SiDpgm5DBa/pNyiej2Uj4sNB0wAnjQmba61po
DMSbjiAjZpEOYOQpwIxNxqVj3CRv+1qqSzNryKBVRKLm6YOLjhsT1JYYbcwgKxji
j5qmAeAoMhxGSBInE/wYoAZs5nr98+QSwfKSkDXKgldqXKUISZKEPDLf/c1+ejsN
B4LX+2eQpQWnVvN4ZZgKsaMWkFI6Vpq/zHNIIxvjBDe/7bK3Mx6KYa++Y8hNpLsT
5szGIpVzAgMBAAECggEAB66YiMvpyj1LGtGT5rsZvWMRvSE6antAYwS0zHImYGzv
WfYI1DjZ//suYvhiYfUXSuUz5w+bCyyhrzm+idrgHvsukA/X4cZmnORLR7N8xL6o
YjDO8VebtpftIzd0WhpqGfeMEEE3VY/kL7MiVtyvrbP1i9jmUl3+guv7+UncRqk+
k5I2LE/0Z+Eax2vIT6WxSW5IzZa1JCiZUP3JjWJ6wSjIPo/Q8zeMLsVggouSeLyg
yQCxRC0Zs8tVVlqZJP029JGk9wR27BFiABWfblpVCWssDNUQMvqLx6nVnK4ZDKVf
MaN3g2cn8PNgxYh5VmOIpWA/lrWcbd2ckrK3uIC5wQKBgQDn4/z6uqeE8cp/6P9o
6feCwlEHLLeyQspwB457YaiFFNVVHOoNxVVPJEQa7mAarE1+ne1bWO7qHiztSSHu
ojCdYyi2eiASuMLvzmAygjcvMlFpb7SR23VToF1nhrUJhZDPW/7dA7r1U8lcgOAX
tDmF96LjiWjyX/tD6wpFF2uYswKBgQDOzy89c9Htuq7MTn66hlhBn7ZOrstSOKhL
WnJgjGgz0Sf3wcVSJ6/HEmgV/FmaupzF/FV+bGoC3DJArPnLulEp6+nzf/kS4YBr
iiHPscIuxXgAPEqBO+WrUb387gtp+Y4Jl0YGA8J3zXL9EhGqjSfjEmXtaFnvdYpe
ytD20zPwQQKBgCJVD/RzRDAbuFZ9warf0QRsDqEYTs42s1Ao3kCIMfIaJoemc0S8
rae8/9RY7KOmVQblDOpUXXMOKqqrBDoXi0kSEEoMBMmgz0O/82AnBVKYiZMptbtO
ma1a9dlJCW6ydsEspqxETB7cW9i1P/hid43NHzG++2m/Ki1hFDEsiYKfAoGAfJcA
xKrT275NtDPN0R1CajaHpBtVzjDn4o2YvmWYGS92PqJz96yFDw8dubEPYlHuRLBh
6QnzS8nQVHpg+sVCRFkl1lZQfXlw3liXSgKzE25PfwTTdcvZ7z4NkSTvp2Z6lXiW
7OSy8W3KSn/qOK9VSk05Lq9hDwvm0nipLynJ7oECgYApndWmEw//w1084CkEdqOr
a/hZ+jeTj0pDJM+fXNgHk6HZPPWRGXxSsN6XXdAVGSgrKbE2hzbMzGVH8d5yaiMw
LaDFSU9BQoStSzXv4NlOQEo7twt6EjR9FpzDldNrbCbMd8y2KQ3YJgoctyc34e5g
V7ZPBxpkkFfrC9adEoI6EA==
-----END PRIVATE KEY-----";
    const KEY2_N: &str = "u1UhtdaenY-hhFPkyUaU7mg4ypsg4hsKRAWB5VLO2u1zlUFHIgYFswzxM7_tLAFz4Jzd-J5TO400Cy-OzESU92h_86fJqvanfNLpqXxOUog6YJuQwWv6Tcono9lI-LDQdMAJ40Jm2utaaAzEm44gI2aRDmDkKcCMTcalY9wkb_taqksza8igVUSi5umDi44bE9SWGG3MICsY4o-apgHgKDIcRkgSJxP8GKAGbOZ6_fPkEsHykpA1yoJXalylCEmShDwy3_3Nfno7DQeC1_tnkKUFp1bzeGWYCrGjFpBSOlaav8xzSCMb4wQ3v-2ytzMeimGvvmPITaS7E-bMxiKVcw";

    const TEAM: &str = "https://team.cloudflareaccess.com";
    const AUD: &str = "aud-tag";

    fn jwk(kid: &str, n: &str) -> Jwk {
        serde_json::from_value(json!({
            "kty": "RSA", "alg": "RS256", "use": "sig", "kid": kid, "n": n, "e": "AQAB"
        }))
        .unwrap()
    }

    /// 呼ばれた回数を数え、鍵の集合を差し替えられる取り先
    struct FakeSource {
        keys: std::sync::Mutex<Vec<Jwk>>,
        calls: AtomicUsize,
    }

    #[async_trait]
    impl JwksSource for FakeSource {
        async fn fetch(&self) -> AppResult<JwkSet> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(JwkSet {
                keys: self.keys.lock().unwrap().clone(),
            })
        }
    }

    fn verifier(
        keys: Vec<Jwk>,
        min_refetch: Duration,
    ) -> (AccessTokenVerifierImpl, Arc<FakeSource>) {
        let src = Arc::new(FakeSource {
            keys: std::sync::Mutex::new(keys),
            calls: AtomicUsize::new(0),
        });
        let keys = AccessKeys::with_source(
            Some(TEAM.to_string()),
            Some(AUD.to_string()),
            src.clone(),
            min_refetch,
        );
        (AccessTokenVerifierImpl { keys }, src)
    }

    fn token(
        pem: &str,
        kid: &str,
        aud: &str,
        iss: &str,
        exp_offset: i64,
        email: Option<&str>,
    ) -> String {
        let exp = chrono::Utc::now().timestamp() + exp_offset;
        let mut claims = json!({ "aud": [aud], "iss": iss, "exp": exp });
        if let Some(e) = email {
            claims["email"] = json!(e);
        }
        let mut h = Header::new(Algorithm::RS256);
        h.kid = Some(kid.to_string());
        encode(
            &h,
            &claims,
            &EncodingKey::from_rsa_pem(pem.as_bytes()).unwrap(),
        )
        .unwrap()
    }

    #[tokio::test]
    async fn accepts_valid_token_and_lowercases_email() {
        let (v, _) = verifier(vec![jwk("k1", KEY1_N)], MIN_REFETCH);
        let t = token(KEY1_PEM, "k1", AUD, TEAM, 600, Some("Tester@Example.com"));
        let id = v.verify(&t).await.unwrap();
        assert_eq!(id.email, "tester@example.com");
    }

    #[tokio::test]
    async fn rejects_wrong_aud_iss_expired_and_missing_email() {
        let (v, _) = verifier(vec![jwk("k1", KEY1_N)], MIN_REFETCH);
        let cases = [
            token(KEY1_PEM, "k1", "other", TEAM, 600, Some("a@example.com")),
            token(
                KEY1_PEM,
                "k1",
                AUD,
                "https://evil.cloudflareaccess.com",
                600,
                Some("a@example.com"),
            ),
            token(KEY1_PEM, "k1", AUD, TEAM, -3600, Some("a@example.com")),
            token(KEY1_PEM, "k1", AUD, TEAM, 600, None),
        ];
        for t in cases {
            assert!(matches!(
                v.verify(&t).await,
                Err(AppError::UnauthenticatedError)
            ));
        }
    }

    #[tokio::test]
    async fn rejects_token_not_yet_valid() {
        let (v, _) = verifier(vec![jwk("k1", KEY1_N)], MIN_REFETCH);
        let now = chrono::Utc::now().timestamp();
        let claims =
            json!({"aud":[AUD],"iss":TEAM,"exp":now+7200,"nbf":now+3600,"email":"a@example.com"});
        let mut h = Header::new(Algorithm::RS256);
        h.kid = Some("k1".into());
        let t = encode(
            &h,
            &claims,
            &EncodingKey::from_rsa_pem(KEY1_PEM.as_bytes()).unwrap(),
        )
        .unwrap();
        assert!(v.verify(&t).await.is_err());
    }

    #[tokio::test]
    async fn rejects_signature_by_another_key_with_same_kid() {
        let (v, _) = verifier(vec![jwk("k1", KEY1_N)], MIN_REFETCH);
        let t = token(KEY2_PEM, "k1", AUD, TEAM, 600, Some("a@example.com"));
        assert!(v.verify(&t).await.is_err());
    }

    #[tokio::test]
    async fn rejects_garbage_and_non_rs256() {
        let (v, src) = verifier(vec![jwk("k1", KEY1_N)], MIN_REFETCH);
        assert!(v.verify("not-a-jwt").await.is_err());
        let hs = encode(
            &Header::new(Algorithm::HS256),
            &json!({"aud":[AUD],"iss":TEAM,"exp":chrono::Utc::now().timestamp()+600,"email":"a@example.com"}),
            &EncodingKey::from_secret(b"secret"),
        )
        .unwrap();
        assert!(v.verify(&hs).await.is_err());
        // 鍵は取りに行かない
        assert_eq!(src.calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn refetches_on_unknown_kid_and_remembers_keys() {
        let (v, src) = verifier(vec![jwk("k1", KEY1_N)], Duration::ZERO);
        let t1 = token(KEY1_PEM, "k1", AUD, TEAM, 600, Some("a@example.com"));
        v.verify(&t1).await.unwrap();
        v.verify(&t1).await.unwrap();
        // 覚えているので、2 回目は取りに行かない
        assert_eq!(src.calls.load(Ordering::SeqCst), 1);

        // 鍵が増えた。知らない kid が来たら取り直す
        src.keys.lock().unwrap().push(jwk("k2", KEY2_N));
        let t2 = token(KEY2_PEM, "k2", AUD, TEAM, 600, Some("b@example.com"));
        assert_eq!(v.verify(&t2).await.unwrap().email, "b@example.com");
        assert_eq!(src.calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn refetch_is_throttled() {
        let (v, src) = verifier(vec![jwk("k1", KEY1_N)], Duration::from_secs(3600));
        let unknown = token(KEY2_PEM, "k2", AUD, TEAM, 600, Some("a@example.com"));
        for _ in 0..3 {
            assert!(v.verify(&unknown).await.is_err());
        }
        // 知らない kid を何度送られても、取り直しは 1 回
        assert_eq!(src.calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn disabled_without_config() {
        let v = AccessTokenVerifierImpl {
            keys: AccessKeys::new(None, Some(AUD.to_string())),
        };
        assert!(!v.is_enabled());
        assert!(v.verify("x.y.z").await.is_err());
        let v = AccessTokenVerifierImpl {
            keys: AccessKeys::new(Some(TEAM.to_string()), Some(AUD.to_string())),
        };
        assert!(v.is_enabled());
    }
}
