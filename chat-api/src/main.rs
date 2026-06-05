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
    chat_tx: broadcast::Sender<ChatMessage>,
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

async fn create_session(state: &AppState, uuid: &str) -> Result<String, ApiError> {
    let token = random_token(64);
    let token_hash = hash_secret(&token);
    let created_at = now();
    let expires_at = created_at + SESSION_TTL_SECONDS;

    sqlx::query(
        "INSERT INTO sessions (token_hash, uuid, expires_at, created_at) VALUES (?, ?, ?, ?)",
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
        setup_url: format!(
            "{}/chat.html?setup={}",
            state.website_url.trim_end_matches('/'),
            token
        ),
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
    auth_user_from_headers(&state, &headers).await?;
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

async fn chat_message(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<WebsiteMessageRequest>,
) -> Result<Json<ApiResponse<ChatMessage>>, ApiError> {
    let (user, _) = auth_user_from_headers(&state, &headers).await?;
    let message =
        clean_message(&req.message).ok_or_else(|| ApiError::bad_request("Message is empty"))?;
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
            created_at INTEGER NOT NULL
        )",
    )
    .execute(db)
    .await?;

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
