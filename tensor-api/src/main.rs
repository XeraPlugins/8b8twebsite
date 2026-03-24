use axum::{
    extract::State,
    http::{StatusCode, header},
    response::Json,
    routing::{get},
    Router,
};
use axum::http::header::HeaderValue;
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};
use std::str::FromStr;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{Duration, Instant};
use flate2::read::GzDecoder;
use std::fs::File;
use std::io::Read;
use fastnbt::from_bytes;
use notify::{RecommendedWatcher, RecursiveMode, Watcher, Event};
use blake3::Hasher;

#[derive(Clone)]
struct AppState {
    db: SqlitePool,
    cache: Arc<RwLock<HashMap<String, (PlayerQuery, Instant)>>>,
    server_path: String,
    block_hashes: Arc<RwLock<HashMap<u32, String>>>,
    file_hashes: Arc<RwLock<HashMap<String, String>>>,
}

#[derive(Debug, Deserialize)]
struct PlayerDat {
    #[serde(rename = "LastKnownName")]
    last_known_name: Option<String>,
    #[serde(rename = "bukkit")]
    bukkit_data: Option<BukkitData>,
    #[serde(rename = "Paper")]
    paper_data: Option<PaperData>,
    #[serde(rename = "firstPlayed")]
    first_played: Option<i64>,
    #[serde(rename = "lastPlayed")]
    last_played: Option<i64>,
}

#[derive(Debug, Deserialize, Default)]
struct BukkitData {
    #[serde(rename = "firstPlayed", default)]
    first_played: Option<i64>,
    #[serde(rename = "lastPlayed", default)]
    last_played: Option<i64>,
    #[serde(rename = "lastKnownName", default)]
    last_known_name: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct PaperData {
    #[serde(rename = "LastSeen", default)]
    last_seen: Option<i64>,
    #[serde(rename = "LastLogin", default)]
    last_login: Option<i64>,
}

fn read_player_dat(server_path: &str, uuid: &str) -> Option<(String, i64, i64, i64)> {
    let dat_path = format!("{}/0b0t/playerdata/{}.dat", server_path, uuid);

    let file = match File::open(&dat_path) {
        Ok(f) => f,
        Err(_) => return None,
    };

    let mut decoder = GzDecoder::new(file);
    let mut data = Vec::new();
    if decoder.read_to_end(&mut data).is_err() {
        return None;
    }

    let player: Result<PlayerDat, _> = from_bytes(data.as_slice());

    match player {
        Ok(p) => {
            let name = p.last_known_name
                .or_else(|| p.bukkit_data.as_ref().and_then(|b| b.last_known_name.clone()))
                .unwrap_or_else(|| uuid.to_string());
            let first = p.first_played.or_else(|| p.bukkit_data.as_ref().and_then(|b| b.first_played)).unwrap_or(0);
            let last = p.last_played.or_else(|| p.bukkit_data.as_ref().and_then(|b| b.last_played)).unwrap_or(0);
            let last_seen = p.paper_data.as_ref().and_then(|p| p.last_seen).unwrap_or(last);
            Some((name, first, last, last_seen))
        }
        Err(_) => None,
    }
}

#[derive(Debug, Deserialize)]
struct UserCacheEntry {
    uuid: String,
    name: String,
}

fn get_player_name_from_usercache(server_path: &str, uuid: &str) -> Option<String> {
    let cache_path = format!("{}/usercache.json", server_path);
    let content = std::fs::read_to_string(cache_path).ok()?;
    let entries: Vec<UserCacheEntry> = serde_json::from_str(&content).ok()?;

    entries.into_iter()
        .find(|e| e.uuid == uuid)
        .map(|e| e.name)
}

fn get_player_uuid_from_usercache(server_path: &str, name: &str) -> Option<String> {
    let cache_path = format!("{}/usercache.json", server_path);
    let content = std::fs::read_to_string(cache_path).ok()?;
    let entries: Vec<UserCacheEntry> = serde_json::from_str(&content).ok()?;

    entries.into_iter()
        .find(|e| e.name.to_lowercase() == name.to_lowercase())
        .map(|e| e.uuid)
}

fn resolve_player_info(server_path: &str, uuid: &str) -> (String, i64, i64, i64) {
    let name_from_cache = get_player_name_from_usercache(server_path, uuid);

    if let Some((name, first, last, last_seen)) = read_player_dat(server_path, uuid) {
        let final_name = name_from_cache.unwrap_or(name);
        return (final_name, first, last, last_seen);
    }

    (name_from_cache.unwrap_or_else(|| uuid.to_string()), 0, 0, 0)
}

#[derive(Debug, Deserialize)]
struct StatsJson {
    stats: StatsData,
}

#[derive(Debug, Deserialize, Default)]
struct StatsData {
    #[serde(rename = "minecraft:custom", default)]
    custom: HashMap<String, i32>,
    #[serde(rename = "minecraft:mined", default)]
    mined: HashMap<String, i32>,
    #[serde(rename = "minecraft:used", default)]
    used: HashMap<String, i32>,
    #[serde(rename = "minecraft:dropped", default)]
    dropped: HashMap<String, i32>,
    #[serde(rename = "minecraft:picked_up", default)]
    picked_up: HashMap<String, i32>,
}

struct PlayerStatsJson {
    time_played: i32,
    deaths: i32,
    kills: i32,
    mobs_killed: i32,
    distance_travelled: i32,
    tnt_used: i32,
    arrows_shot: i32,
    items_dropped: i32,
    elytra_used: i32,
    blocks_broken: i32,
    obsidian_mined: i32,
    obsidian_placed: i32,
    netherite_mined: i32,
}

fn read_player_stats(server_path: &str, uuid: &str) -> Option<PlayerStatsJson> {
    let stats_path = format!("{}/0b0t/stats/{}.json", server_path, uuid);
    let content = std::fs::read_to_string(&stats_path).ok()?;
    let stats: StatsJson = serde_json::from_str(&content).ok()?;

    let custom = &stats.stats.custom;
    let mined = &stats.stats.mined;
    let used = &stats.stats.used;
    let dropped = &stats.stats.dropped;

    Some(PlayerStatsJson {
        time_played: *custom.get("minecraft:play_time").unwrap_or(&0),
        deaths: *custom.get("minecraft:deaths").unwrap_or(&0),
        kills: *custom.get("minecraft:player_kills").unwrap_or(&0),
        mobs_killed: *custom.get("minecraft:mob_kills").unwrap_or(&0),
        distance_travelled: custom.get("minecraft:walk_one_cm").unwrap_or(&0)
            + custom.get("minecraft:sprint_one_cm").unwrap_or(&0)
            + custom.get("minecraft:fly_one_cm").unwrap_or(&0),
        tnt_used: *used.get("minecraft:tnt").unwrap_or(&0),
        arrows_shot: *used.get("minecraft:bow").unwrap_or(&0),
        items_dropped: dropped.values().sum(),
        elytra_used: *used.get("minecraft:elytra").unwrap_or(&0),
        blocks_broken: mined.values().sum(),
        obsidian_mined: *mined.get("minecraft:obsidian").unwrap_or(&0),
        obsidian_placed: *used.get("minecraft:obsidian").unwrap_or(&0),
        netherite_mined: *mined.get("minecraft:netherite_block").unwrap_or(&0),
    })
}

#[derive(Debug, Deserialize)]
struct PlayerStats {
    uuid: String,
    name: String,
    join_date: i64,
    deaths: Option<i32>,
    kills: Option<i32>,
    mobs_killed: Option<i32>,
    blocks_broken: Option<i32>,
    blocks_placed: Option<i32>,
    time_played: Option<i32>,
    tnt_used: Option<i32>,
    arrows_shot: Option<i32>,
    items_dropped: Option<i32>,
    distance_travelled: Option<i32>,
    obsidian_mined: Option<i32>,
    obsidian_placed: Option<i32>,
    netherite_mined: Option<i32>,
    elytra_used: Option<i32>,
    totems_used: Option<i32>,
}

#[derive(sqlx::FromRow, Serialize, Clone)]
struct PlayerResponse {
    uuid: String,
    name: String,
    join_date: i64,
    last_seen: i64,
    deaths: i32,
    kills: i32,
    mobs_killed: i32,
    blocks_broken: i32,
    blocks_placed: i32,
    time_played: i32,
    tnt_used: i32,
    arrows_shot: i32,
    items_dropped: i32,
    distance_travelled: i32,
    obsidian_mined: i32,
    obsidian_placed: i32,
    netherite_mined: i32,
    elytra_used: i32,
    totems_used: i32,
    skin_url: String,
}

#[derive(Serialize)]
struct ApiResponse {
    success: bool,
    data: Option<PlayerResponse>,
    error: Option<String>,
}

fn get_file_hash(path: &str) -> Option<String> {
    let content = std::fs::read(path).ok()?;
    let mut hasher = Hasher::new();
    hasher.update(&content);
    Some(hasher.finalize().to_hex().to_string())
}

async fn sync_single_player(state: &AppState, uuid: &str) -> bool {
    let stats_path = format!("{}/0b0t/stats/{}.json", state.server_path, uuid);
    let file_hash = match get_file_hash(&stats_path) {
        Some(h) => h,
        None => return false,
    };

    {
        let file_hashes = state.file_hashes.read().await;
        if let Some(last_hash) = file_hashes.get(uuid) {
            if file_hash == *last_hash {
                return false;
            }
        }
    }

    if let Some(stats) = read_player_stats(&state.server_path, uuid) {
        let (name, first_played, _last_played, last_seen) = resolve_player_info(&state.server_path, uuid);

        let result = sqlx::query(
            "INSERT INTO player_stats (uuid, name, join_date, last_seen, deaths, kills, mobs_killed, blocks_broken, blocks_placed, time_played, tnt_used, arrows_shot, items_dropped, distance_travelled, obsidian_mined, obsidian_placed, netherite_mined, elytra_used, totems_used, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
             ON CONFLICT(uuid) DO UPDATE SET
                name = excluded.name,
                join_date = excluded.join_date,
                last_seen = excluded.last_seen,
                deaths = excluded.deaths,
                kills = excluded.kills,
                mobs_killed = excluded.mobs_killed,
                blocks_broken = excluded.blocks_broken,
                blocks_placed = excluded.blocks_placed,
                time_played = excluded.time_played,
                tnt_used = excluded.tnt_used,
                arrows_shot = excluded.arrows_shot,
                items_dropped = excluded.items_dropped,
                distance_travelled = excluded.distance_travelled,
                obsidian_mined = excluded.obsidian_mined,
                obsidian_placed = excluded.obsidian_placed,
                netherite_mined = excluded.netherite_mined,
                elytra_used = excluded.elytra_used,
                totems_used = excluded.totems_used,
                updated_at = CURRENT_TIMESTAMP"
        )
        .bind(uuid)
        .bind(&name)
        .bind(first_played)
        .bind(last_seen)
        .bind(stats.deaths)
        .bind(stats.kills)
        .bind(stats.mobs_killed)
        .bind(stats.blocks_broken)
        .bind(0i32)
        .bind(stats.time_played)
        .bind(stats.tnt_used)
        .bind(stats.arrows_shot)
        .bind(stats.items_dropped)
        .bind(stats.distance_travelled)
        .bind(stats.obsidian_mined)
        .bind(stats.obsidian_placed)
        .bind(stats.netherite_mined)
        .bind(stats.elytra_used)
        .bind(0i32)
        .execute(&state.db)
        .await;

        if result.is_ok() {
            let mut file_hashes = state.file_hashes.write().await;
            file_hashes.insert(uuid.to_string(), file_hash);
            return true;
        }
    }
    false
}

async fn sync_from_files(State(state): State<AppState>) -> (StatusCode, Json<ApiResponse>) {
    let stats_dir = format!("{}/0b0t/stats", state.server_path);

    let entries = match std::fs::read_dir(&stats_dir) {
        Ok(e) => e,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Error reading stats dir: {}", e)),
            }));
        }
    };

    let mut synced = 0;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            if let Some(uuid) = path.file_stem().and_then(|s| s.to_str()) {
                if sync_single_player(&state, uuid).await {
                    synced += 1;
                }
            }
        }
    }

    (StatusCode::OK, Json(ApiResponse {
        success: true,
        data: None,
        error: Some(format!("Synced {} players from stats files", synced)),
    }))
}

const BLOCK_SIZE: usize = 400;

fn get_block_hash(stats_dir: &str, start_idx: usize, uuids: &[String]) -> String {
    let mut hasher = Hasher::new();
    for uuid in uuids.iter().skip(start_idx).take(BLOCK_SIZE) {
        let path = format!("{}/{}.json", stats_dir, uuid);
        if let Ok(content) = std::fs::read(&path) {
            hasher.update(&content);
        }
    }
    hasher.finalize().to_hex().to_string()
}

async fn sync_blocks_on_startup(state: &AppState) -> String {
    let stats_dir = format!("{}/0b0t/stats", state.server_path);

    let entries = match std::fs::read_dir(&stats_dir) {
        Ok(e) => e,
        Err(e) => return format!("Error reading stats dir: {}", e),
    };

    let all_uuids: Vec<String> = entries
        .flatten()
        .filter_map(|e| {
            let path = e.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                path.file_stem().and_then(|s| s.to_str()).map(|s| s.to_string())
            } else {
                None
            }
        })
        .collect();

    let total_players = all_uuids.len();
    let total_blocks = (total_players + BLOCK_SIZE - 1) / BLOCK_SIZE;

    let mut synced_players = 0;
    let mut updated_blocks = 0;

    let mut current_block_hashes = state.block_hashes.write().await;

    for block_idx in 0..total_blocks {
        let block_hash = get_block_hash(&stats_dir, block_idx * BLOCK_SIZE, &all_uuids);

        let block_key = block_idx as u32;
        if let Some(last_hash) = current_block_hashes.get(&block_key) {
            if block_hash == *last_hash {
                continue;
            }
        }

        updated_blocks += 1;

        let start = block_idx * BLOCK_SIZE;
        for uuid in all_uuids.iter().skip(start).take(BLOCK_SIZE) {
            if sync_single_player(state, uuid).await {
                synced_players += 1;
            }
        }

        current_block_hashes.insert(block_key, block_hash);
    }

    format!("Checked {} blocks, updated {} blocks, synced {} players out of {}", total_blocks, updated_blocks, synced_players, total_players)
}

async fn sync_blocks(State(state): State<AppState>) -> (StatusCode, Json<ApiResponse>) {
    let stats_dir = format!("{}/0b0t/stats", state.server_path);

    let entries = match std::fs::read_dir(&stats_dir) {
        Ok(e) => e,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Error reading stats dir: {}", e)),
            }));
        }
    };

    let all_uuids: Vec<String> = entries
        .flatten()
        .filter_map(|e| {
            let path = e.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                path.file_stem().and_then(|s| s.to_str()).map(|s| s.to_string())
            } else {
                None
            }
        })
        .collect();

    let total_players = all_uuids.len();
    let total_blocks = (total_players + BLOCK_SIZE - 1) / BLOCK_SIZE;

    let mut synced_players = 0;
    let mut checked_blocks = 0;
    let mut updated_blocks = 0;

    let mut current_block_hashes = state.block_hashes.write().await;

    for block_idx in 0..total_blocks {
        let block_hash = get_block_hash(&stats_dir, block_idx * BLOCK_SIZE, &all_uuids);
        checked_blocks += 1;

        let block_key = block_idx as u32;
        if let Some(last_hash) = current_block_hashes.get(&block_key) {
            if block_hash == *last_hash {
                continue;
            }
        }

        updated_blocks += 1;

        let start = block_idx * BLOCK_SIZE;
        for uuid in all_uuids.iter().skip(start).take(BLOCK_SIZE) {
            if sync_single_player(&state, uuid).await {
                synced_players += 1;
            }
        }

        current_block_hashes.insert(block_idx as u32, block_hash);
    }

    (StatusCode::OK, Json(ApiResponse {
        success: true,
        data: None,
        error: Some(format!("Checked {} blocks, updated {} blocks, synced {} players out of {}", checked_blocks, updated_blocks, synced_players, total_players)),
    }))
}

async fn sync_stats(
    State(state): State<AppState>,
    Json(players): Json<Vec<PlayerStats>>,
) -> (StatusCode, Json<ApiResponse>) {
    let mut synced = 0;

    for player in players {
        let result = sqlx::query(
            "INSERT INTO player_stats (uuid, name, join_date, deaths, kills, mobs_killed, blocks_broken, blocks_placed, time_played, tnt_used, arrows_shot, items_dropped, distance_travelled, obsidian_mined, obsidian_placed, netherite_mined, elytra_used, totems_used, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
             ON CONFLICT(uuid) DO UPDATE SET
                name = excluded.name,
                deaths = excluded.deaths,
                kills = excluded.kills,
                mobs_killed = excluded.mobs_killed,
                blocks_broken = excluded.blocks_broken,
                blocks_placed = excluded.blocks_placed,
                time_played = excluded.time_played,
                tnt_used = excluded.tnt_used,
                arrows_shot = excluded.arrows_shot,
                items_dropped = excluded.items_dropped,
                distance_travelled = excluded.distance_travelled,
                obsidian_mined = excluded.obsidian_mined,
                obsidian_placed = excluded.obsidian_placed,
                netherite_mined = excluded.netherite_mined,
                elytra_used = excluded.elytra_used,
                totems_used = excluded.totems_used,
                updated_at = CURRENT_TIMESTAMP"
        )
        .bind(&player.uuid)
        .bind(&player.name)
        .bind(player.join_date)
        .bind(player.deaths.unwrap_or(0))
        .bind(player.kills.unwrap_or(0))
        .bind(player.mobs_killed.unwrap_or(0))
        .bind(player.blocks_broken.unwrap_or(0))
        .bind(player.blocks_placed.unwrap_or(0))
        .bind(player.time_played.unwrap_or(0))
        .bind(player.tnt_used.unwrap_or(0))
        .bind(player.arrows_shot.unwrap_or(0))
        .bind(player.items_dropped.unwrap_or(0))
        .bind(player.distance_travelled.unwrap_or(0))
        .bind(player.obsidian_mined.unwrap_or(0))
        .bind(player.obsidian_placed.unwrap_or(0))
        .bind(player.netherite_mined.unwrap_or(0))
        .bind(player.elytra_used.unwrap_or(0))
        .bind(player.totems_used.unwrap_or(0))
        .execute(&state.db)
        .await;

        if result.is_ok() {
            synced += 1;
        }
    }

    (StatusCode::OK, Json(ApiResponse {
        success: true,
        data: None,
        error: Some(format!("Synced {} players", synced)),
    }))
}

#[derive(sqlx::FromRow, Serialize, Clone)]
struct PlayerQuery {
    uuid: String,
    name: String,
    join_date: i64,
    last_seen: i64,
    deaths: i32,
    kills: i32,
    mobs_killed: i32,
    blocks_broken: i32,
    blocks_placed: i32,
    time_played: i32,
    tnt_used: i32,
    arrows_shot: i32,
    items_dropped: i32,
    distance_travelled: i32,
    obsidian_mined: i32,
    obsidian_placed: i32,
    netherite_mined: i32,
    elytra_used: i32,
    totems_used: i32,
}

fn get_skin_url(uuid: &str) -> String {
    format!("https://api.mineatar.io/face/{}", uuid)
}

async fn get_player(
    State(state): State<AppState>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> (StatusCode, Json<ApiResponse>) {
    let search_key = name.to_lowercase();

    {
        let cache = state.cache.read().await;
        if let Some((player, time)) = cache.get(&search_key) {
            if time.elapsed() < Duration::from_secs(300) {
                let response = PlayerResponse {
                    uuid: player.uuid.clone(),
                    name: player.name.clone(),
                    join_date: player.join_date,
                    last_seen: player.last_seen,
                    deaths: player.deaths,
                    kills: player.kills,
                    mobs_killed: player.mobs_killed,
                    blocks_broken: player.blocks_broken,
                    blocks_placed: player.blocks_placed,
                    time_played: player.time_played,
                    tnt_used: player.tnt_used,
                    arrows_shot: player.arrows_shot,
                    items_dropped: player.items_dropped,
                    distance_travelled: player.distance_travelled,
                    obsidian_mined: player.obsidian_mined,
                    obsidian_placed: player.obsidian_placed,
                    netherite_mined: player.netherite_mined,
                    elytra_used: player.elytra_used,
                    totems_used: player.totems_used,
                    skin_url: get_skin_url(&player.uuid),
                };
                return (StatusCode::OK, Json(ApiResponse {
                    success: true,
                    data: Some(response),
                    error: None,
                }));
            }
        }
    }

    let result = sqlx::query_as::<_, PlayerQuery>(
         "SELECT uuid, name, join_date, last_seen, deaths, kills, mobs_killed, blocks_broken, blocks_placed, time_played, tnt_used, arrows_shot, items_dropped, distance_travelled, obsidian_mined, obsidian_placed, netherite_mined, elytra_used, totems_used
         FROM player_stats WHERE LOWER(name) = ? OR LOWER(uuid) = ?"
    )
    .bind(&search_key)
    .bind(&search_key)
    .fetch_optional(&state.db)
    .await;

    match result {
        Ok(Some(player)) => {
            let response = PlayerResponse {
                uuid: player.uuid.clone(),
                name: player.name.clone(),
                join_date: player.join_date,
                last_seen: player.last_seen,
                deaths: player.deaths,
                kills: player.kills,
                mobs_killed: player.mobs_killed,
                blocks_broken: player.blocks_broken,
                blocks_placed: player.blocks_placed,
                time_played: player.time_played,
                tnt_used: player.tnt_used,
                arrows_shot: player.arrows_shot,
                items_dropped: player.items_dropped,
                distance_travelled: player.distance_travelled,
                obsidian_mined: player.obsidian_mined,
                obsidian_placed: player.obsidian_placed,
                netherite_mined: player.netherite_mined,
                elytra_used: player.elytra_used,
                totems_used: player.totems_used,
                skin_url: get_skin_url(&player.uuid),
            };

            let mut cache = state.cache.write().await;
            cache.insert(search_key, (player.clone(), Instant::now()));

            (StatusCode::OK, Json(ApiResponse {
                success: true,
                data: Some(response),
                error: None,
            }))
        }
        _ => {
            if let Some(uuid) = get_player_uuid_from_usercache(&state.server_path, &name) {
                if sync_single_player(&state, &uuid).await {
                    let result = sqlx::query_as::<_, PlayerQuery>(
                         "SELECT uuid, name, join_date, last_seen, deaths, kills, mobs_killed, blocks_broken, blocks_placed, time_played, tnt_used, arrows_shot, items_dropped, distance_travelled, obsidian_mined, obsidian_placed, netherite_mined, elytra_used, totems_used
                         FROM player_stats WHERE LOWER(name) = ? OR LOWER(uuid) = ?"
                    )
                    .bind(&search_key)
                    .bind(&search_key)
                    .fetch_optional(&state.db)
                    .await;

                    if let Ok(Some(player)) = result {
                        let response = PlayerResponse {
                            uuid: player.uuid.clone(),
                            name: player.name.clone(),
                            join_date: player.join_date,
                            last_seen: player.last_seen,
                            deaths: player.deaths,
                            kills: player.kills,
                            mobs_killed: player.mobs_killed,
                            blocks_broken: player.blocks_broken,
                            blocks_placed: player.blocks_placed,
                            time_played: player.time_played,
                            tnt_used: player.tnt_used,
                            arrows_shot: player.arrows_shot,
                            items_dropped: player.items_dropped,
                            distance_travelled: player.distance_travelled,
                            obsidian_mined: player.obsidian_mined,
                            obsidian_placed: player.obsidian_placed,
                            netherite_mined: player.netherite_mined,
                            elytra_used: player.elytra_used,
                            totems_used: player.totems_used,
                            skin_url: get_skin_url(&player.uuid),
                        };

                        (StatusCode::OK, Json(ApiResponse {
                            success: true,
                            data: Some(response),
                            error: None,
                        }))
                    } else {
                        (StatusCode::NOT_FOUND, Json(ApiResponse {
                            success: false,
                            data: None,
                            error: Some("Player not found".to_string()),
                        }))
                    }
                } else {
                    (StatusCode::NOT_FOUND, Json(ApiResponse {
                        success: false,
                        data: None,
                        error: Some("Player not found".to_string()),
                    }))
                }
            } else {
                (StatusCode::NOT_FOUND, Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Player not found".to_string()),
                }))
            }
        }
    }
}

#[derive(Serialize)]
struct LeaderboardEntry {
    uuid: String,
    name: String,
    join_date: i64,
    deaths: i32,
    kills: i32,
    time_played: i32,
    skin_url: String,
}

async fn get_leaderboard(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> (StatusCode, Json<serde_json::Value>) {
    let limit = params.get("limit").and_then(|l| l.parse().ok()).unwrap_or(10);
    let sort = params.get("sort").map(|s| s.as_str()).unwrap_or("kills");

    let order_column = match sort {
        "deaths" => "deaths",
        "mobs_killed" => "mobs_killed",
        "time_played" => "time_played",
        "blocks_broken" => "blocks_broken",
        "blocks_placed" => "blocks_placed",
        "tnt_used" => "tnt_used",
        "obsidian_mined" => "obsidian_mined",
        "netherite_mined" => "netherite_mined",
        _ => "kills",
    };

    let query = format!(
         "SELECT uuid, name, join_date, last_seen, deaths, kills, mobs_killed, blocks_broken, blocks_placed, time_played, tnt_used, arrows_shot, items_dropped, distance_travelled, obsidian_mined, obsidian_placed, netherite_mined, elytra_used, totems_used
         FROM player_stats ORDER BY {} DESC LIMIT ?",
         order_column
    );

    let result = sqlx::query_as::<_, PlayerQuery>(&query)
    .bind(limit)
    .fetch_all(&state.db)
    .await;

    match result {
        Ok(players) => {
            let entries: Vec<LeaderboardEntry> = players.into_iter().map(|p| {
                let uuid = p.uuid.clone();
                LeaderboardEntry {
                    uuid: p.uuid,
                    name: p.name,
                    join_date: p.join_date,
                    deaths: p.deaths,
                    kills: p.kills,
                    time_played: p.time_played,
                    skin_url: get_skin_url(&uuid),
                }
            }).collect();

            (StatusCode::OK, Json(serde_json::json!({
                "success": true,
                "data": entries
            })))
        }
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
            "success": false,
            "error": "Database error"
        }))),
    }
}

#[tokio::main]
async fn main() {
    let server_path = std::env::var("SERVER_PATH").unwrap_or_else(|_| "../../server".to_string());

    let cwd = std::env::current_dir().expect("Failed to get current directory");
    let db_path = cwd.join("tensor.db");

    let db_url = format!("sqlite://{}", db_path.display());
    let options = SqliteConnectOptions::from_str(&db_url)
        .expect("Failed to parse DB URL")
        .create_if_missing(true);
    let db = SqlitePool::connect_with(options).await.unwrap();

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS player_stats (
            uuid TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            join_date INTEGER NOT NULL,
            last_seen INTEGER DEFAULT 0,
            deaths INTEGER DEFAULT 0,
            kills INTEGER DEFAULT 0,
            mobs_killed INTEGER DEFAULT 0,
            blocks_broken INTEGER DEFAULT 0,
            blocks_placed INTEGER DEFAULT 0,
            time_played INTEGER DEFAULT 0,
            tnt_used INTEGER DEFAULT 0,
            arrows_shot INTEGER DEFAULT 0,
            items_dropped INTEGER DEFAULT 0,
            distance_travelled INTEGER DEFAULT 0,
            obsidian_mined INTEGER DEFAULT 0,
            obsidian_placed INTEGER DEFAULT 0,
            netherite_mined INTEGER DEFAULT 0,
            elytra_used INTEGER DEFAULT 0,
            totems_used INTEGER DEFAULT 0,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )"
    )
    .execute(&db)
    .await
    .unwrap();

    let state = AppState {
        db,
        cache: Arc::new(RwLock::new(HashMap::new())),
        server_path: server_path.clone(),
        block_hashes: Arc::new(RwLock::new(HashMap::new())),
        file_hashes: Arc::new(RwLock::new(HashMap::new())),
    };

    // FIX: run startup sync in background so the server binds immediately
    let state_for_sync = state.clone();
    tokio::spawn(async move {
        let result = sync_blocks_on_startup(&state_for_sync).await;
        println!("Startup sync complete: {}", result);
    });

    let state_clone = state.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let stats_dir = format!("{}/0b0t/stats", server_path);
            let (tx, mut rx) = tokio::sync::mpsc::channel(100);

            let mut watcher = RecommendedWatcher::new(
                move |res: Result<Event, notify::Error>| {
                    if let Ok(event) = res {
                        if event.kind.is_modify() || event.kind.is_create() {
                            for path in event.paths {
                                if let Some(uuid) = path.file_stem().and_then(|s| s.to_str()) {
                                    let _ = tx.blocking_send(uuid.to_string());
                                }
                            }
                        }
                    }
                },
                notify::Config::default(),
            ).unwrap();

            watcher.watch(std::path::Path::new(&stats_dir), RecursiveMode::NonRecursive).unwrap();

            while let Some(uuid) = rx.recv().await {
                sync_single_player(&state_clone, &uuid).await;
            }
        });
    });

    let cors = tower_http::cors::CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_methods(tower_http::cors::Any)
        .allow_headers(tower_http::cors::Any);

    let app = Router::new()
        .route("/player/:name", get(get_player))
        .route("/leaderboard", get(get_leaderboard))
        .layer(cors)
        .with_state(state);

    println!("Server listening on 0.0.0.0:5300");
    let listener = tokio::net::TcpListener::bind("0.0.0.0:5300").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}