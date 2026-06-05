use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use axum::{
    Json, Router,
    extract::{
        Path, Query, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::{HeaderMap, Method, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use futures_util::{SinkExt, StreamExt};
use rand::{Rng, distributions::Alphanumeric};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool, sqlite::SqliteConnectOptions};
use std::{
    collections::HashMap,
    net::SocketAddr,
    str::FromStr,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::broadcast;
use tower_http::cors::{Any, CorsLayer};

const SETUP_TOKEN_TTL_SECONDS: i64 = 15 * 60;
const SESSION_TTL_SECONDS: i64 = 30 * 24 * 60 * 60;
const MAX_MESSAGE_LENGTH: usize = 256;

#[derive(Clone)]
struct AppState {
    db: SqlitePool,
    plugin_secret: Arc<String>,
    website_url: Arc<String>,
    config: Arc<AppConfig>,
    http_client: Client,
    chat_tx: broadcast::Sender<ChatMessage>,
}

#[derive(Clone)]
struct AppConfig {
    hcaptcha_enabled: bool,
    hcaptcha_site_key: String,
    hcaptcha_secret: String,
    rate_limit_enabled: bool,
    rate_window_seconds: i64,
    rate_max_messages: i64,
    rate_short_limit_seconds: i64,
    rate_long_limit_seconds: i64,
    rate_long_after_violations: i64,
    rate_violation_reset_seconds: i64,
    allow_links: bool,
    blocked_link_patterns: Vec<String>,
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }

    fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, message)
    }

    fn unauthorized(message: impl Into<String>) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, message)
    }

    fn forbidden(message: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, message)
    }

    fn too_many_requests(message: impl Into<String>) -> Self {
        Self::new(StatusCode::TOO_MANY_REQUESTS, message)
    }

    fn internal(message: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, message)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = Json(ApiResponse::<()> {
            success: false,
            data: None,
            error: Some(self.message),
        });
        (self.status, body).into_response()
    }
}

#[derive(Serialize)]
struct ApiResponse<T: Serialize> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

fn ok<T: Serialize>(data: T) -> Json<ApiResponse<T>> {
    Json(ApiResponse {
        success: true,
        data: Some(data),
        error: None,
    })
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn random_token(length: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

fn hash_secret(secret: &str) -> String {
    blake3::hash(secret.as_bytes()).to_hex().to_string()
}

fn normalize_username(username: &str) -> String {
    username.trim().to_ascii_lowercase()
}

fn is_valid_username(username: &str) -> bool {
    let username = username.trim();
    !username.is_empty()
        && username.len() <= 32
        && username
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn is_valid_uuid(uuid: &str) -> bool {
    uuid.len() <= 36 && uuid.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
}

fn clean_message(message: &str) -> Option<String> {
    let cleaned: String = message
        .trim()
        .chars()
        .filter(|c| !c.is_control() || *c == '\t')
        .take(MAX_MESSAGE_LENGTH)
        .collect();

    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned)
    }
}

fn password_hash(password: &str) -> Result<String, ApiError> {
    if password.len() < 8 || password.len() > 128 {
        return Err(ApiError::bad_request("Password must be 8-128 characters"));
    }

    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| ApiError::internal("Failed to hash password"))
}

fn password_matches(password: &str, hash: &str) -> bool {
    PasswordHash::new(hash)
        .ok()
        .and_then(|parsed| {
            Argon2::default()
                .verify_password(password.as_bytes(), &parsed)
                .ok()
        })
        .is_some()
}

fn extract_bearer(headers: &HeaderMap) -> Option<String> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::to_string)
        .or_else(|| {
            headers
                .get("x-session-token")
                .and_then(|value| value.to_str().ok())
                .map(str::to_string)
        })
}

fn plugin_authorized(headers: &HeaderMap, state: &AppState) -> bool {
    headers
        .get("x-plugin-secret")
        .and_then(|value| value.to_str().ok())
        .map(|value| value == state.plugin_secret.as_str())
        .unwrap_or(false)
}

fn env_bool(name: &str, default: bool) -> bool {
    std::env::var(name)
        .ok()
        .map(|value| {
            matches!(
                value.to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(default)
}

fn env_i64(name: &str, default: i64) -> i64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

fn load_config() -> AppConfig {
    let hcaptcha_enabled = env_bool("HCAPTCHA_ENABLED", false);
    let hcaptcha_site_key = std::env::var("HCAPTCHA_SITE_KEY").unwrap_or_default();
    let hcaptcha_secret = std::env::var("HCAPTCHA_SECRET").unwrap_or_default();

    if hcaptcha_enabled && (hcaptcha_site_key.is_empty() || hcaptcha_secret.is_empty()) {
        panic!("HCAPTCHA_SITE_KEY and HCAPTCHA_SECRET must be set when HCAPTCHA_ENABLED=true");
    }

    let blocked_link_patterns = std::env::var("CHAT_BLOCKED_LINK_PATTERNS")
        .unwrap_or_else(|_| {
            "http://,https://,www.,discord.gg,.com,.net,.org,.me,.gg,.io".to_string()
        })
        .split(',')
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .collect();

    AppConfig {
        hcaptcha_enabled,
        hcaptcha_site_key,
        hcaptcha_secret,
        rate_limit_enabled: env_bool("CHAT_RATE_LIMIT_ENABLED", true),
        rate_window_seconds: env_i64("CHAT_RATE_WINDOW_SECONDS", 10),
        rate_max_messages: env_i64("CHAT_RATE_MAX_MESSAGES", 5),
        rate_short_limit_seconds: env_i64("CHAT_RATE_SHORT_LIMIT_SECONDS", 60),
        rate_long_limit_seconds: env_i64("CHAT_RATE_LONG_LIMIT_SECONDS", 600),
        rate_long_after_violations: env_i64("CHAT_RATE_LONG_AFTER_VIOLATIONS", 2),
        rate_violation_reset_seconds: env_i64("CHAT_RATE_VIOLATION_RESET_SECONDS", 3600),
        allow_links: env_bool("CHAT_ALLOW_LINKS", false),
        blocked_link_patterns,
    }
}

fn contains_blocked_link(config: &AppConfig, message: &str) -> bool {
    if config.allow_links {
        return false;
    }

    let normalized = message.to_ascii_lowercase();
    config
        .blocked_link_patterns
        .iter()
        .any(|pattern| normalized.contains(pattern))
}

#[derive(FromRow, Serialize, Clone)]
struct User {
    uuid: String,
    username: String,
    username_lower: String,
    disabled: i64,
}

async fn auth_user(state: &AppState, token: &str) -> Result<User, ApiError> {
    let token_hash = hash_secret(token);
    let user = sqlx::query_as::<_, User>(
        "SELECT users.uuid, users.username, users.username_lower, users.disabled
         FROM sessions
         JOIN users ON users.uuid = sessions.uuid
         WHERE sessions.token_hash = ? AND sessions.expires_at > ? AND users.disabled = 0",
    )
    .bind(token_hash)
    .bind(now())
    .fetch_optional(&state.db)
    .await
    .map_err(|_| ApiError::internal("Database error"))?;

    user.ok_or_else(|| ApiError::unauthorized("Invalid or expired session"))
}

async fn auth_user_from_headers(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<(User, String), ApiError> {
    let token =
        extract_bearer(headers).ok_or_else(|| ApiError::unauthorized("Missing session token"))?;
    let user = auth_user(state, &token).await?;
    Ok((user, token))
}

async fn require_captcha_verified(state: &AppState, token: &str) -> Result<(), ApiError> {
    if !state.config.hcaptcha_enabled {
        return Ok(());
    }

    let verified_at: Option<(Option<i64>,)> = sqlx::query_as(
        "SELECT captcha_verified_at FROM sessions WHERE token_hash = ? AND expires_at > ?",
    )
    .bind(hash_secret(token))
    .bind(now())
    .fetch_optional(&state.db)
    .await
    .map_err(|_| ApiError::internal("Database error"))?;

    match verified_at {
        Some((Some(_),)) => Ok(()),
        _ => Err(ApiError::forbidden("hCaptcha verification required")),
    }
}

async fn create_session(state: &AppState, uuid: &str) -> Result<String, ApiError> {
    let token = random_token(64);
    let token_hash = hash_secret(&token);
    let created_at = now();
    let expires_at = created_at + SESSION_TTL_SECONDS;

    sqlx::query(
        "INSERT INTO sessions (token_hash, uuid, expires_at, created_at, captcha_verified_at) VALUES (?, ?, ?, ?, NULL)",
    )
    .bind(token_hash)
    .bind(uuid)
    .bind(expires_at)
    .bind(created_at)
    .execute(&state.db)
    .await
    .map_err(|_| ApiError::internal("Failed to create session"))?;

    Ok(token)
}

#[derive(Deserialize)]
struct PluginSetupLinkRequest {
    uuid: String,
    username: String,
    reset: Option<bool>,
}

#[derive(Serialize)]
struct PluginSetupLinkResponse {
    setup_url: String,
    expires_at: i64,
}

async fn plugin_setup_link(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<PluginSetupLinkRequest>,
) -> Result<Json<ApiResponse<PluginSetupLinkResponse>>, ApiError> {
    if !plugin_authorized(&headers, &state) {
        return Err(ApiError::forbidden("Invalid plugin secret"));
    }
    if !is_valid_uuid(&req.uuid) || !is_valid_username(&req.username) {
        return Err(ApiError::bad_request("Invalid player identity"));
    }

    let token = random_token(48);
    let token_hash = hash_secret(&token);
    let expires_at = now() + SETUP_TOKEN_TTL_SECONDS;
    let username = req.username.trim();
    let username_lower = normalize_username(username);
    let reset = req.reset.unwrap_or(false);

    if !reset {
        let account_exists: Option<(String,)> =
            sqlx::query_as("SELECT uuid FROM users WHERE uuid = ? AND disabled = 0")
                .bind(&req.uuid)
                .fetch_optional(&state.db)
                .await
                .map_err(|_| ApiError::internal("Database error"))?;

        if account_exists.is_some() {
            return Err(ApiError::bad_request(
                "Account already exists. Use /chatbridge reset.",
            ));
        }
    }

    sqlx::query("DELETE FROM setup_tokens WHERE uuid = ? AND used_at IS NULL")
        .bind(&req.uuid)
        .execute(&state.db)
        .await
        .map_err(|_| ApiError::internal("Failed to replace old setup token"))?;

    sqlx::query(
        "INSERT INTO setup_tokens (token_hash, uuid, username, username_lower, expires_at, used_at)
         VALUES (?, ?, ?, ?, ?, NULL)",
    )
    .bind(token_hash)
    .bind(&req.uuid)
    .bind(username)
    .bind(username_lower)
    .bind(expires_at)
    .execute(&state.db)
    .await
    .map_err(|_| ApiError::internal("Failed to create setup token"))?;

    Ok(ok(PluginSetupLinkResponse {
        setup_url: format!("{}/chat{}", state.website_url.trim_end_matches('/'), token),
        expires_at,
    }))
}

#[derive(Deserialize)]
struct PluginMessageRequest {
    uuid: String,
    username: String,
    message: String,
}

async fn plugin_minecraft_message(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<PluginMessageRequest>,
) -> Result<Json<ApiResponse<ChatMessage>>, ApiError> {
    if !plugin_authorized(&headers, &state) {
        return Err(ApiError::forbidden("Invalid plugin secret"));
    }
    if !is_valid_uuid(&req.uuid) || !is_valid_username(&req.username) {
        return Err(ApiError::bad_request("Invalid player identity"));
    }

    let message =
        clean_message(&req.message).ok_or_else(|| ApiError::bad_request("Message is empty"))?;
    let chat = insert_message(
        &state,
        "minecraft",
        &req.uuid,
        req.username.trim(),
        &message,
    )
    .await?;
    let _ = state.chat_tx.send(chat.clone());
    Ok(ok(chat))
}

#[derive(FromRow)]
struct SetupTokenRow {
    uuid: String,
    username: String,
    username_lower: String,
    expires_at: i64,
    used_at: Option<i64>,
}

#[derive(Serialize)]
struct SetupInfoResponse {
    username: String,
    expires_at: i64,
}

async fn auth_setup_info(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<Json<ApiResponse<SetupInfoResponse>>, ApiError> {
    let row = setup_token(&state, &token).await?;
    Ok(ok(SetupInfoResponse {
        username: row.username,
        expires_at: row.expires_at,
    }))
}

async fn setup_token(state: &AppState, token: &str) -> Result<SetupTokenRow, ApiError> {
    let row = sqlx::query_as::<_, SetupTokenRow>(
        "SELECT uuid, username, username_lower, expires_at, used_at FROM setup_tokens WHERE token_hash = ?",
    )
    .bind(hash_secret(token))
    .fetch_optional(&state.db)
    .await
    .map_err(|_| ApiError::internal("Database error"))?;

    let row = row.ok_or_else(|| ApiError::bad_request("Invalid setup link"))?;
    if row.used_at.is_some() {
        return Err(ApiError::bad_request("Setup link has already been used"));
    }
    if row.expires_at <= now() {
        return Err(ApiError::bad_request("Setup link has expired"));
    }
    Ok(row)
}

#[derive(Deserialize)]
struct AuthSetupRequest {
    token: String,
    password: String,
}

#[derive(Serialize)]
struct AuthResponse {
    token: String,
    user: UserResponse,
}

#[derive(Serialize)]
struct UserResponse {
    uuid: String,
    username: String,
}

async fn auth_setup(
    State(state): State<AppState>,
    Json(req): Json<AuthSetupRequest>,
) -> Result<Json<ApiResponse<AuthResponse>>, ApiError> {
    let setup = setup_token(&state, &req.token).await?;
    let hash = password_hash(&req.password)?;
    let timestamp = now();

    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|_| ApiError::internal("Database error"))?;
    sqlx::query(
        "INSERT INTO users (uuid, username, username_lower, password_hash, created_at, password_changed_at, disabled)
         VALUES (?, ?, ?, ?, ?, ?, 0)
         ON CONFLICT(uuid) DO UPDATE SET
            username = excluded.username,
            username_lower = excluded.username_lower,
            password_hash = excluded.password_hash,
            password_changed_at = excluded.password_changed_at,
            disabled = 0",
    )
    .bind(&setup.uuid)
    .bind(&setup.username)
    .bind(&setup.username_lower)
    .bind(hash)
    .bind(timestamp)
    .bind(timestamp)
    .execute(&mut *tx)
    .await
    .map_err(|_| ApiError::bad_request("That username is already linked to another account"))?;

    sqlx::query("UPDATE setup_tokens SET used_at = ? WHERE token_hash = ?")
        .bind(timestamp)
        .bind(hash_secret(&req.token))
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::internal("Failed to mark setup link used"))?;

    tx.commit()
        .await
        .map_err(|_| ApiError::internal("Database error"))?;

    let session = create_session(&state, &setup.uuid).await?;
    Ok(ok(AuthResponse {
        token: session,
        user: UserResponse {
            uuid: setup.uuid,
            username: setup.username,
        },
    }))
}

#[derive(Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(FromRow)]
struct LoginRow {
    uuid: String,
    username: String,
    password_hash: String,
    disabled: i64,
}

async fn auth_login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<ApiResponse<AuthResponse>>, ApiError> {
    let username_lower = normalize_username(&req.username);
    let user = sqlx::query_as::<_, LoginRow>(
        "SELECT uuid, username, password_hash, disabled FROM users WHERE username_lower = ?",
    )
    .bind(username_lower)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| ApiError::internal("Database error"))?
    .ok_or_else(|| ApiError::unauthorized("Invalid username or password"))?;

    if user.disabled != 0 || !password_matches(&req.password, &user.password_hash) {
        return Err(ApiError::unauthorized("Invalid username or password"));
    }

    let timestamp = now();
    sqlx::query("UPDATE users SET last_login_at = ? WHERE uuid = ?")
        .bind(timestamp)
        .bind(&user.uuid)
        .execute(&state.db)
        .await
        .map_err(|_| ApiError::internal("Failed to update login time"))?;

    let session = create_session(&state, &user.uuid).await?;
    Ok(ok(AuthResponse {
        token: session,
        user: UserResponse {
            uuid: user.uuid,
            username: user.username,
        },
    }))
}

async fn auth_logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let (_, token) = auth_user_from_headers(&state, &headers).await?;
    sqlx::query("DELETE FROM sessions WHERE token_hash = ?")
        .bind(hash_secret(&token))
        .execute(&state.db)
        .await
        .map_err(|_| ApiError::internal("Failed to log out"))?;
    Ok(ok(serde_json::json!({ "logged_out": true })))
}

async fn auth_me(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<UserResponse>>, ApiError> {
    let (user, _) = auth_user_from_headers(&state, &headers).await?;
    Ok(ok(UserResponse {
        uuid: user.uuid,
        username: user.username,
    }))
}

#[derive(Serialize)]
struct CaptchaConfigResponse {
    enabled: bool,
    site_key: String,
}

async fn captcha_config(State(state): State<AppState>) -> Json<ApiResponse<CaptchaConfigResponse>> {
    ok(CaptchaConfigResponse {
        enabled: state.config.hcaptcha_enabled,
        site_key: if state.config.hcaptcha_enabled {
            state.config.hcaptcha_site_key.clone()
        } else {
            String::new()
        },
    })
}

#[derive(Deserialize)]
struct CaptchaVerifyRequest {
    response: String,
}

#[derive(Deserialize)]
struct HcaptchaVerifyResponse {
    success: bool,
}

async fn captcha_verify(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CaptchaVerifyRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let (_, token) = auth_user_from_headers(&state, &headers).await?;

    if state.config.hcaptcha_enabled {
        if req.response.trim().is_empty() {
            return Err(ApiError::bad_request("Missing hCaptcha response"));
        }

        let response = state
            .http_client
            .post("https://hcaptcha.com/siteverify")
            .form(&[
                ("secret", state.config.hcaptcha_secret.as_str()),
                ("response", req.response.as_str()),
            ])
            .send()
            .await
            .map_err(|_| ApiError::internal("Failed to verify hCaptcha"))?;

        let verification: HcaptchaVerifyResponse = response
            .json()
            .await
            .map_err(|_| ApiError::internal("Invalid hCaptcha verification response"))?;

        if !verification.success {
            return Err(ApiError::forbidden("hCaptcha verification failed"));
        }
    }

    sqlx::query("UPDATE sessions SET captcha_verified_at = ? WHERE token_hash = ?")
        .bind(now())
        .bind(hash_secret(&token))
        .execute(&state.db)
        .await
        .map_err(|_| ApiError::internal("Failed to save hCaptcha verification"))?;

    Ok(ok(serde_json::json!({ "verified": true })))
}

#[derive(FromRow, Serialize, Clone)]
struct ChatMessage {
    id: i64,
    source: String,
    uuid: String,
    username: String,
    message: String,
    created_at: i64,
}

async fn insert_message(
    state: &AppState,
    source: &str,
    uuid: &str,
    username: &str,
    message: &str,
) -> Result<ChatMessage, ApiError> {
    let created_at = now();
    let result = sqlx::query(
        "INSERT INTO messages (source, uuid, username, message, created_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(source)
    .bind(uuid)
    .bind(username)
    .bind(message)
    .bind(created_at)
    .execute(&state.db)
    .await
    .map_err(|_| ApiError::internal("Failed to save chat message"))?;

    Ok(ChatMessage {
        id: result.last_insert_rowid(),
        source: source.to_string(),
        uuid: uuid.to_string(),
        username: username.to_string(),
        message: message.to_string(),
        created_at,
    })
}

async fn chat_history(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<ApiResponse<Vec<ChatMessage>>>, ApiError> {
    let (_, token) = auth_user_from_headers(&state, &headers).await?;
    require_captcha_verified(&state, &token).await?;
    let limit = params
        .get("limit")
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(100)
        .clamp(1, 200);
    let before = params
        .get("before")
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(i64::MAX);

    let mut messages = sqlx::query_as::<_, ChatMessage>(
        "SELECT id, source, uuid, username, message, created_at
         FROM messages WHERE id < ? ORDER BY id DESC LIMIT ?",
    )
    .bind(before)
    .bind(limit)
    .fetch_all(&state.db)
    .await
    .map_err(|_| ApiError::internal("Failed to load chat history"))?;

    messages.reverse();
    Ok(ok(messages))
}

#[derive(Deserialize)]
struct WebsiteMessageRequest {
    message: String,
}

#[derive(FromRow)]
struct RateLimitRow {
    window_start: i64,
    message_count: i64,
    violations: i64,
    limited_until: i64,
    updated_at: i64,
}

async fn enforce_chat_policy(state: &AppState, user: &User, message: &str) -> Result<(), ApiError> {
    if contains_blocked_link(&state.config, message) {
        return Err(ApiError::bad_request(
            "Links are not allowed in website chat",
        ));
    }

    if !state.config.rate_limit_enabled {
        return Ok(());
    }

    let timestamp = now();
    let row = sqlx::query_as::<_, RateLimitRow>(
        "SELECT window_start, message_count, violations, limited_until, updated_at
         FROM rate_limits WHERE uuid = ?",
    )
    .bind(&user.uuid)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| ApiError::internal("Database error"))?;

    let Some(row) = row else {
        sqlx::query(
            "INSERT INTO rate_limits (uuid, window_start, message_count, violations, limited_until, updated_at)
             VALUES (?, ?, 1, 0, 0, ?)",
        )
        .bind(&user.uuid)
        .bind(timestamp)
        .bind(timestamp)
        .execute(&state.db)
        .await
        .map_err(|_| ApiError::internal("Failed to update rate limit"))?;
        return Ok(());
    };

    let mut violations = if timestamp - row.updated_at > state.config.rate_violation_reset_seconds {
        0
    } else {
        row.violations
    };

    if row.limited_until > timestamp {
        violations += 1;
        let duration = if violations >= state.config.rate_long_after_violations {
            state.config.rate_long_limit_seconds
        } else {
            row.limited_until - timestamp
        };
        let limited_until = timestamp + duration;

        sqlx::query(
            "UPDATE rate_limits SET violations = ?, limited_until = ?, updated_at = ? WHERE uuid = ?",
        )
        .bind(violations)
        .bind(limited_until)
        .bind(timestamp)
        .bind(&user.uuid)
        .execute(&state.db)
        .await
        .map_err(|_| ApiError::internal("Failed to update rate limit"))?;

        return Err(ApiError::too_many_requests(format!(
            "You are rate limited for {} more seconds",
            (limited_until - timestamp).max(1)
        )));
    }

    if timestamp - row.window_start > state.config.rate_window_seconds {
        sqlx::query(
            "UPDATE rate_limits SET window_start = ?, message_count = 1, violations = ?, limited_until = 0, updated_at = ? WHERE uuid = ?",
        )
        .bind(timestamp)
        .bind(violations)
        .bind(timestamp)
        .bind(&user.uuid)
        .execute(&state.db)
        .await
        .map_err(|_| ApiError::internal("Failed to update rate limit"))?;
        return Ok(());
    }

    let next_count = row.message_count + 1;
    if next_count > state.config.rate_max_messages {
        violations += 1;
        let duration = if violations >= state.config.rate_long_after_violations {
            state.config.rate_long_limit_seconds
        } else {
            state.config.rate_short_limit_seconds
        };
        let limited_until = timestamp + duration;

        sqlx::query(
            "UPDATE rate_limits SET message_count = ?, violations = ?, limited_until = ?, updated_at = ? WHERE uuid = ?",
        )
        .bind(next_count)
        .bind(violations)
        .bind(limited_until)
        .bind(timestamp)
        .bind(&user.uuid)
        .execute(&state.db)
        .await
        .map_err(|_| ApiError::internal("Failed to update rate limit"))?;

        return Err(ApiError::too_many_requests(format!(
            "Too many messages too fast. You are rate limited for {} seconds",
            duration
        )));
    }

    sqlx::query(
        "UPDATE rate_limits SET message_count = ?, violations = ?, updated_at = ? WHERE uuid = ?",
    )
    .bind(next_count)
    .bind(violations)
    .bind(timestamp)
    .bind(&user.uuid)
    .execute(&state.db)
    .await
    .map_err(|_| ApiError::internal("Failed to update rate limit"))?;

    Ok(())
}

async fn chat_message(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<WebsiteMessageRequest>,
) -> Result<Json<ApiResponse<ChatMessage>>, ApiError> {
    let (user, token) = auth_user_from_headers(&state, &headers).await?;
    require_captcha_verified(&state, &token).await?;
    let message =
        clean_message(&req.message).ok_or_else(|| ApiError::bad_request("Message is empty"))?;
    enforce_chat_policy(&state, &user, &message).await?;
    let chat = insert_message(&state, "website", &user.uuid, &user.username, &message).await?;
    let _ = state.chat_tx.send(chat.clone());
    Ok(ok(chat))
}

async fn chat_ws(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    ws: WebSocketUpgrade,
) -> Result<Response, ApiError> {
    let token = params
        .get("token")
        .ok_or_else(|| ApiError::unauthorized("Missing session token"))?;
    auth_user(&state, token).await?;
    require_captcha_verified(&state, token).await?;
    Ok(ws.on_upgrade(move |socket| website_socket(socket, state.chat_tx.subscribe())))
}

async fn plugin_ws(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    ws: WebSocketUpgrade,
) -> Result<Response, ApiError> {
    let secret = params
        .get("secret")
        .ok_or_else(|| ApiError::forbidden("Missing plugin secret"))?;
    if secret != state.plugin_secret.as_str() {
        return Err(ApiError::forbidden("Invalid plugin secret"));
    }
    Ok(ws.on_upgrade(move |socket| plugin_socket(socket, state.chat_tx.subscribe())))
}

async fn website_socket(socket: WebSocket, mut rx: broadcast::Receiver<ChatMessage>) {
    let (mut sender, mut receiver) = socket.split();

    loop {
        tokio::select! {
            message = rx.recv() => {
                match message {
                    Ok(chat) => {
                        if sender.send(Message::Text(serde_json::to_string(&chat).unwrap_or_default())).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => break,
                }
            }
            inbound = receiver.next() => {
                match inbound {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Ping(bytes))) => {
                        let _ = sender.send(Message::Pong(bytes)).await;
                    }
                    Some(Err(_)) => break,
                    _ => {}
                }
            }
        }
    }
}

async fn plugin_socket(socket: WebSocket, mut rx: broadcast::Receiver<ChatMessage>) {
    let (mut sender, mut receiver) = socket.split();

    loop {
        tokio::select! {
            message = rx.recv() => {
                match message {
                    Ok(chat) if chat.source == "website" => {
                        if sender.send(Message::Text(serde_json::to_string(&chat).unwrap_or_default())).await.is_err() {
                            break;
                        }
                    }
                    Ok(_) => continue,
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => break,
                }
            }
            inbound = receiver.next() => {
                match inbound {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Ping(bytes))) => {
                        let _ = sender.send(Message::Pong(bytes)).await;
                    }
                    Some(Err(_)) => break,
                    _ => {}
                }
            }
        }
    }
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": "chat-api",
        "version": "0.1.0"
    }))
}

async fn migrate(db: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS users (
            uuid TEXT PRIMARY KEY,
            username TEXT NOT NULL,
            username_lower TEXT UNIQUE NOT NULL,
            password_hash TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            last_login_at INTEGER,
            password_changed_at INTEGER,
            disabled INTEGER NOT NULL DEFAULT 0
        )",
    )
    .execute(db)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS setup_tokens (
            token_hash TEXT PRIMARY KEY,
            uuid TEXT NOT NULL,
            username TEXT NOT NULL,
            username_lower TEXT NOT NULL,
            expires_at INTEGER NOT NULL,
            used_at INTEGER
        )",
    )
    .execute(db)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_setup_tokens_uuid ON setup_tokens(uuid)")
        .execute(db)
        .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS sessions (
            token_hash TEXT PRIMARY KEY,
            uuid TEXT NOT NULL,
            expires_at INTEGER NOT NULL,
            created_at INTEGER NOT NULL,
            captcha_verified_at INTEGER
        )",
    )
    .execute(db)
    .await?;

    let _ = sqlx::query("ALTER TABLE sessions ADD COLUMN captcha_verified_at INTEGER")
        .execute(db)
        .await;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_sessions_uuid ON sessions(uuid)")
        .execute(db)
        .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS messages (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            source TEXT NOT NULL,
            uuid TEXT NOT NULL,
            username TEXT NOT NULL,
            message TEXT NOT NULL,
            created_at INTEGER NOT NULL
        )",
    )
    .execute(db)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_messages_id ON messages(id)")
        .execute(db)
        .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS rate_limits (
            uuid TEXT PRIMARY KEY,
            window_start INTEGER NOT NULL,
            message_count INTEGER NOT NULL,
            violations INTEGER NOT NULL,
            limited_until INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        )",
    )
    .execute(db)
    .await?;

    Ok(())
}

#[tokio::main]
async fn main() {
    let plugin_secret = std::env::var("PLUGIN_SECRET").expect("PLUGIN_SECRET must be set");
    let website_url =
        std::env::var("WEBSITE_URL").unwrap_or_else(|_| "https://www.8b8t.me".to_string());
    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://chat.db".to_string());
    let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:5400".to_string());
    let config = load_config();

    let options = SqliteConnectOptions::from_str(&database_url)
        .expect("Invalid DATABASE_URL")
        .create_if_missing(true);
    let db = SqlitePool::connect_with(options)
        .await
        .expect("Failed to connect to database");
    migrate(&db).await.expect("Failed to migrate database");

    let (chat_tx, _) = broadcast::channel(512);
    let state = AppState {
        db,
        plugin_secret: Arc::new(plugin_secret),
        website_url: Arc::new(website_url),
        config: Arc::new(config),
        http_client: Client::new(),
        chat_tx,
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers(Any);

    let app = Router::new()
        .route("/plugin/setup-link", post(plugin_setup_link))
        .route("/plugin/minecraft-message", post(plugin_minecraft_message))
        .route("/plugin/ws", get(plugin_ws))
        .route("/auth/setup/:token", get(auth_setup_info))
        .route("/auth/setup", post(auth_setup))
        .route("/auth/login", post(auth_login))
        .route("/auth/logout", post(auth_logout))
        .route("/auth/me", get(auth_me))
        .route("/captcha/config", get(captcha_config))
        .route("/captcha/verify", post(captcha_verify))
        .route("/chat/history", get(chat_history))
        .route("/chat/message", post(chat_message))
        .route("/chat/ws", get(chat_ws))
        .route("/health", get(health))
        .layer(cors)
        .with_state(state);

    let addr: SocketAddr = bind_addr.parse().expect("Invalid BIND_ADDR");
    println!("chat-api listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind listener");
    axum::serve(listener, app).await.expect("Server failed");
}
