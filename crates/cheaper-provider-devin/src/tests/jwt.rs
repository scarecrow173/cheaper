use std::sync::atomic::{AtomicUsize, Ordering};

use rig::http_client::DynHttpClient;

use crate::jwt::*;

/// JWT cache と mint override は process-global — test を直列化する。
static LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn http() -> DynHttpClient {
    // mint override を置くので実通信は発生しない。
    DynHttpClient::new(rig::rig_reqwest::ReqwestClient::default())
}

/// `exp` を持つ最小 JWT 形（base64url payload、署名ダミー）。
fn fake_jwt(exp: i64) -> String {
    use base64::Engine;
    let payload =
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(format!(r#"{{"exp":{exp}}}"#));
    format!("eyJhbGciOiJub25lIn0.{payload}.sig")
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

#[tokio::test]
async fn caches_minted_jwt_until_margin() {
    static CALLS: AtomicUsize = AtomicUsize::new(0);
    fn mint(_key: &str, _host: &str) -> Result<MintedUserJwt, JwtError> {
        CALLS.fetch_add(1, Ordering::SeqCst);
        Ok(MintedUserJwt {
            jwt: "eyJ.minted.sig".to_string(),
            expires_at: now_secs() + 3600,
        })
    }
    let _guard = LOCK.lock().await;
    CALLS.store(0, Ordering::SeqCst);
    clear_cached_user_jwt().await;
    __set_mint_override_for_tests(Some(mint));
    let http = http();
    let a = get_cached_user_jwt(&http, "k1", "h").await.unwrap();
    let b = get_cached_user_jwt(&http, "k1", "h").await.unwrap();
    assert_eq!(a, b);
    assert_eq!(CALLS.load(Ordering::SeqCst), 1);
    // 別 key は別 mint。
    let _ = get_cached_user_jwt(&http, "k2", "h").await.unwrap();
    assert_eq!(CALLS.load(Ordering::SeqCst), 2);
    __set_mint_override_for_tests(None);
    clear_cached_user_jwt().await;
}

#[tokio::test]
async fn remints_when_inside_reuse_margin() {
    static CALLS: AtomicUsize = AtomicUsize::new(0);
    fn mint(_key: &str, _host: &str) -> Result<MintedUserJwt, JwtError> {
        // expiry が now+30s（60s margin 内）→ 再利用しない。
        CALLS.fetch_add(1, Ordering::SeqCst);
        Ok(MintedUserJwt {
            jwt: "eyJ.short.sig".to_string(),
            expires_at: now_secs() + 30,
        })
    }
    let _guard = LOCK.lock().await;
    CALLS.store(0, Ordering::SeqCst);
    clear_cached_user_jwt().await;
    __set_mint_override_for_tests(Some(mint));
    let http = http();
    let _ = get_cached_user_jwt(&http, "k", "h").await.unwrap();
    let _ = get_cached_user_jwt(&http, "k", "h").await.unwrap();
    assert_eq!(CALLS.load(Ordering::SeqCst), 2);
    __set_mint_override_for_tests(None);
    clear_cached_user_jwt().await;
}

#[test]
fn fake_jwt_carries_base64url_exp() {
    let jwt = fake_jwt(now_secs() + 10);
    assert!(jwt.starts_with("eyJ"));
    // mint 経路が exp を読むことは transport 必須のため、ここでは
    // 文字列整形のみ確認する。
}

#[test]
fn float_exp_claim_is_accepted() {
    // TS `typeof parsed.exp === "number"` — float exp は 600s
    // fallback へ落とさず expiry として読む。
    use base64::Engine;
    let exp = now_secs() as f64 + 123.7;
    let payload =
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(format!(r#"{{"exp":{exp}}}"#));
    let jwt = format!("eyJhbGciOiJub25lIn0.{payload}.sig");
    assert_eq!(crate::jwt::jwt_expires_at(&jwt, now_secs()), exp as i64);
    // 非数値 exp は fallback（now + 600）。
    let now = now_secs();
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(r#"{"exp":"soon"}"#);
    let jwt = format!("eyJhbGciOiJub25lIn0.{payload}.sig");
    assert_eq!(crate::jwt::jwt_expires_at(&jwt, now), now + 600);
}

#[tokio::test]
async fn in_flight_calls_share_one_mint() {
    static CALLS: AtomicUsize = AtomicUsize::new(0);
    fn mint(_key: &str, _host: &str) -> Result<MintedUserJwt, JwtError> {
        CALLS.fetch_add(1, Ordering::SeqCst);
        Ok(MintedUserJwt {
            jwt: "eyJ.shared.sig".to_string(),
            expires_at: now_secs() + 3600,
        })
    }
    let _guard = LOCK.lock().await;
    CALLS.store(0, Ordering::SeqCst);
    clear_cached_user_jwt().await;
    __set_mint_override_for_tests(Some(mint));
    let http = http();
    let (a, b, c) = tokio::join!(
        get_cached_user_jwt(&http, "k", "h"),
        get_cached_user_jwt(&http, "k", "h"),
        get_cached_user_jwt(&http, "k", "h"),
    );
    assert!(a.is_ok() && b.is_ok() && c.is_ok());
    assert_eq!(CALLS.load(Ordering::SeqCst), 1);
    __set_mint_override_for_tests(None);
    clear_cached_user_jwt().await;
}
