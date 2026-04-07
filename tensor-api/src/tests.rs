use super::*;

const TEST_DATA: &str = "test_data";

#[test]
fn test_parse_player_dat_mindcomplexity() {
    let dat_path = format!(
        "{}/0b0t/playerdata/5b3a39be-24cc-3861-b90a-619502c73562.dat",
        TEST_DATA
    );
    let file = File::open(&dat_path).expect("Failed to open test .dat file");
    let mut decoder = GzDecoder::new(file);
    let mut data = Vec::new();
    decoder
        .read_to_end(&mut data)
        .expect("Failed to decompress");

    let player: PlayerDat = from_bytes(data.as_slice()).expect("Failed to parse NBT");
    let bukkit = player.bukkit_data.expect("Should have bukkit data");

    assert_eq!(bukkit.last_known_name, Some("MindComplexity".to_string()));
    assert_eq!(bukkit.first_played, Some(1773928561072));
    assert_eq!(bukkit.last_played, Some(1774211991033));

    if let Some(paper) = player.paper_data {
        assert_eq!(paper.last_seen, Some(1774211991033));
    }
}

#[test]
fn test_parse_player_dat_wqm() {
    let dat_path = format!(
        "{}/0b0t/playerdata/dd9be595-03ac-32fc-bcd8-d3c3f272889c.dat",
        TEST_DATA
    );
    let file = File::open(&dat_path).expect("Failed to open test .dat file");
    let mut decoder = GzDecoder::new(file);
    let mut data = Vec::new();
    decoder
        .read_to_end(&mut data)
        .expect("Failed to decompress");

    let player: PlayerDat = from_bytes(data.as_slice()).expect("Failed to parse NBT");
    let bukkit = player.bukkit_data.expect("Should have bukkit data");

    assert_eq!(bukkit.last_known_name, Some("wqm".to_string()));
}

#[test]
fn test_parse_stats_json() {
    let json_path = format!(
        "{}/0b0t/stats/d4c3cd0f-2fff-419d-af3f-3c2dc5e01deb.json",
        TEST_DATA
    );
    let content = std::fs::read_to_string(&json_path).expect("Failed to read stats JSON");
    let stats: StatsJson = serde_json::from_str(&content).expect("Failed to parse JSON");

    let custom = &stats.stats.custom;
    let used = &stats.stats.used;
    let mined = &stats.stats.mined;

    assert_eq!(*custom.get("minecraft:play_time").unwrap_or(&0), 64328);
    assert_eq!(*custom.get("minecraft:deaths").unwrap_or(&0), 1);
    assert_eq!(*custom.get("minecraft:mob_kills").unwrap_or(&0), 2);
    assert_eq!(*used.get("minecraft:tnt").unwrap_or(&0), 13);
    assert_eq!(*used.get("minecraft:obsidian").unwrap_or(&0), 16);
    assert!(mined.is_empty());
}

#[test]
fn test_parse_stats_with_mined() {
    let json_path = format!(
        "{}/0b0t/stats/dd9be595-03ac-32fc-bcd8-d3c3f272889c.json",
        TEST_DATA
    );
    let content = std::fs::read_to_string(&json_path).expect("Failed to read stats JSON");
    let stats: StatsJson = serde_json::from_str(&content).expect("Failed to parse JSON");

    let mined = &stats.stats.mined;

    assert_eq!(*mined.get("minecraft:short_grass").unwrap_or(&0), 8);
    assert_eq!(*mined.get("minecraft:pink_petals").unwrap_or(&0), 3);
    assert_eq!(mined.values().sum::<i32>(), 11);
}

#[test]
fn test_skin_url_uses_name() {
    assert_eq!(
        get_skin_url("MindComplexity"),
        "https://mc-heads.net/avatar/MindComplexity"
    );
    assert_eq!(get_skin_url("wqm"), "https://mc-heads.net/avatar/wqm");
}

#[test]
fn test_resolve_player_info() {
    let (name, first, _, _) =
        resolve_player_info(TEST_DATA, "5b3a39be-24cc-3861-b90a-619502c73562");
    assert_eq!(name, "MindComplexity");
    assert!(first > 0);

    let (name, _, _, _) = resolve_player_info(TEST_DATA, "00000000-0000-0000-0000-000000000000");
    assert_eq!(name, "00000000-0000-0000-0000-000000000000");
}

#[test]
fn test_read_player_stats() {
    let stats = read_player_stats(TEST_DATA, "d4c3cd0f-2fff-419d-af3f-3c2dc5e01deb")
        .expect("Should parse stats");

    assert_eq!(stats.time_played, 64328);
    assert_eq!(stats.deaths, 1);
    assert_eq!(stats.kills, 0);
    assert_eq!(stats.mobs_killed, 2);
    assert_eq!(stats.tnt_used, 13);
    assert_eq!(stats.obsidian_placed, 16);
    assert_eq!(stats.blocks_broken, 0);
    assert_eq!(stats.netherite_mined, 0);
}

#[test]
fn test_read_player_stats_with_mined() {
    let stats = read_player_stats(TEST_DATA, "dd9be595-03ac-32fc-bcd8-d3c3f272889c")
        .expect("Should parse stats");

    assert_eq!(stats.blocks_broken, 11);
    assert_eq!(stats.deaths, 2);
}
