use anyhow::{Context as AnyhowContext, Result};
use reqwest::Client as HttpClient;
use serde::Deserialize;
use serenity::{
    all::{
        ChannelId, Command, CommandDataOptionValue, CommandInteraction, CommandOptionType, Context,
        CreateCommand, CreateCommandOption, CreateEmbed, CreateInteractionResponse,
        CreateInteractionResponseMessage, GatewayIntents, GuildId, Interaction, Ready,
    },
    async_trait,
    client::{Client as DiscordClient, EventHandler},
};
use std::{
    fs,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

#[derive(Debug, Deserialize, Clone)]
struct Config {
    discord_token: String,
    guild_id: Option<u64>,
    tensor_api_url: String,
    promo: PromoConfig,
}

#[derive(Debug, Deserialize, Clone)]
struct PromoConfig {
    enabled: bool,
    interval_hours: u64,
    shop_url: String,
    message: String,
    channel_ids: Vec<u64>,
}

#[derive(Clone)]
struct BotState {
    config: Arc<Config>,
    http: HttpClient,
    promo_started: Arc<AtomicBool>,
}

struct Handler {
    state: BotState,
}

#[derive(Debug, Deserialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PlayerStats {
    uuid: String,
    name: String,
    join_date: i64,
    last_seen: i64,
    deaths: i64,
    kills: i64,
    mobs_killed: i64,
    blocks_broken: i64,
    time_played: i64,
    tnt_used: i64,
    arrows_shot: i64,
    items_dropped: i64,
    distance_travelled: i64,
    obsidian_mined: i64,
    obsidian_placed: i64,
    netherite_mined: i64,
    elytra_used: i64,
    totems_used: i64,
    skin_url: String,
}

#[derive(Debug, Deserialize)]
struct LeaderboardEntry {
    name: String,
    kills: i64,
    deaths: i64,
    mobs_killed: i64,
    time_played: i64,
    blocks_broken: i64,
    tnt_used: i64,
    obsidian_mined: i64,
    netherite_mined: i64,
    elytra_used: i64,
    totems_used: i64,
    arrows_shot: i64,
    distance_travelled: i64,
}

#[derive(Debug, Deserialize)]
struct HealthResponse {
    status: String,
    version: Option<String>,
}

const LEADERBOARD_SORTS: [&str; 12] = [
    "kills",
    "deaths",
    "mobs_killed",
    "time_played",
    "blocks_broken",
    "tnt_used",
    "obsidian_mined",
    "netherite_mined",
    "elytra_used",
    "totems_used",
    "arrows_shot",
    "distance_travelled",
];

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected", ready.user.name);
        if let Err(error) = register_commands(&ctx, &self.state.config).await {
            eprintln!("Failed to register commands: {error:?}");
        }

        if self.state.config.promo.enabled
            && self
                .state
                .promo_started
                .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
        {
            start_promo_task(ctx, self.state.clone());
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        let Interaction::Command(command) = interaction else {
            return;
        };

        let result = match command.data.name.as_str() {
            "stats" => handle_stats(&ctx, &command, &self.state).await,
            "leaderboard" => handle_leaderboard(&ctx, &command, &self.state).await,
            "status" => handle_status(&ctx, &command, &self.state).await,
            _ => Ok(()),
        };

        if let Err(error) = result {
            eprintln!("Command failed: {error:?}");
            let _ = respond_text(
                &ctx,
                &command,
                "Something went wrong while contacting the stats API.",
                true,
            )
            .await;
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = Arc::new(load_config()?);
    let state = BotState {
        config: Arc::clone(&config),
        http: HttpClient::new(),
        promo_started: Arc::new(AtomicBool::new(false)),
    };

    let intents = GatewayIntents::GUILDS;
    let mut client = DiscordClient::builder(&config.discord_token, intents)
        .event_handler(Handler { state })
        .await
        .context("Failed to create Discord client")?;

    client.start().await.context("Discord client failed")?;
    Ok(())
}

fn load_config() -> Result<Config> {
    let path = std::env::var("DISCORD_STATS_BOT_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("config.toml"));
    let content = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read config at {}", path.display()))?;
    let mut config: Config = toml::from_str(&content).context("Failed to parse config.toml")?;
    config.tensor_api_url = trim_trailing_slash(&config.tensor_api_url);

    if config.discord_token.trim().is_empty() || config.discord_token == "YOUR_DISCORD_BOT_TOKEN" {
        anyhow::bail!("discord_token must be configured");
    }
    if config.promo.enabled && config.promo.channel_ids.is_empty() {
        anyhow::bail!("promo.channel_ids must contain at least one channel when promo is enabled");
    }

    Ok(config)
}

fn trim_trailing_slash(value: &str) -> String {
    value.trim_end_matches('/').to_string()
}

async fn register_commands(ctx: &Context, config: &Config) -> Result<()> {
    let commands = vec![stats_command(), leaderboard_command(), status_command()];
    match config.guild_id.filter(|id| *id != 0) {
        Some(guild_id) => {
            GuildId::new(guild_id)
                .set_commands(&ctx.http, commands)
                .await
                .context("Failed to register guild commands")?;
        }
        None => {
            Command::set_global_commands(&ctx.http, commands)
                .await
                .context("Failed to register global commands")?;
        }
    }
    Ok(())
}

fn stats_command() -> CreateCommand {
    CreateCommand::new("stats")
        .description("Look up 8b8t player stats")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "player",
                "Minecraft username or UUID",
            )
            .required(true),
        )
}

fn leaderboard_command() -> CreateCommand {
    let mut sort_option =
        CreateCommandOption::new(CommandOptionType::String, "sort", "Leaderboard metric")
            .required(true);

    for sort in LEADERBOARD_SORTS {
        sort_option = sort_option.add_string_choice(sort, sort);
    }

    CreateCommand::new("leaderboard")
        .description("Show an 8b8t stats leaderboard")
        .add_option(sort_option)
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Integer,
                "limit",
                "Number of players, 1-10",
            )
            .min_int_value(1)
            .max_int_value(10),
        )
}

fn status_command() -> CreateCommand {
    CreateCommand::new("status").description("Check the Tensor stats API status")
}

async fn handle_stats(ctx: &Context, command: &CommandInteraction, state: &BotState) -> Result<()> {
    let player = command
        .data
        .options
        .iter()
        .find(|option| option.name == "player")
        .and_then(|option| string_option(&option.value))
        .context("Missing player option")?;

    let url = format!(
        "{}/player/{}",
        state.config.tensor_api_url,
        url_encode(player)
    );
    let response: ApiResponse<PlayerStats> = state.http.get(url).send().await?.json().await?;
    if !response.success {
        return respond_text(
            ctx,
            command,
            response.error.as_deref().unwrap_or("Player not found"),
            true,
        )
        .await;
    }

    let Some(player) = response.data else {
        return respond_text(ctx, command, "Player not found", true).await;
    };

    let kd = if player.deaths > 0 {
        format!("{:.2}", player.kills as f64 / player.deaths as f64)
    } else if player.kills > 0 {
        "∞".to_string()
    } else {
        "0".to_string()
    };

    let embed = CreateEmbed::new()
        .title(format!("{} Stats", player.name))
        .thumbnail(player.skin_url)
        .field("Kills", format_number(player.kills), true)
        .field("Deaths", format_number(player.deaths), true)
        .field("K/D", kd, true)
        .field("Time Played", format_time(player.time_played), true)
        .field("Blocks Broken", format_number(player.blocks_broken), true)
        .field("Distance", format_distance(player.distance_travelled), true)
        .field("Mob Kills", format_number(player.mobs_killed), true)
        .field("TNT Used", format_number(player.tnt_used), true)
        .field("Arrows Shot", format_number(player.arrows_shot), true)
        .field("Obsidian Mined", format_number(player.obsidian_mined), true)
        .field(
            "Obsidian Placed",
            format_number(player.obsidian_placed),
            true,
        )
        .field(
            "Netherite Mined",
            format_number(player.netherite_mined),
            true,
        )
        .field("Elytra Used", format_number(player.elytra_used), true)
        .field("Totems Used", format_number(player.totems_used), true)
        .field("Items Dropped", format_number(player.items_dropped), true)
        .footer(serenity::builder::CreateEmbedFooter::new(format!(
            "UUID: {} | Joined: {} | Last seen: {}",
            player.uuid,
            format_timestamp_ms(player.join_date),
            format_timestamp_ms(player.last_seen)
        )));

    respond_embed(ctx, command, embed).await
}

async fn handle_leaderboard(
    ctx: &Context,
    command: &CommandInteraction,
    state: &BotState,
) -> Result<()> {
    let sort = command
        .data
        .options
        .iter()
        .find(|option| option.name == "sort")
        .and_then(|option| string_option(&option.value))
        .unwrap_or("kills");
    let limit = command
        .data
        .options
        .iter()
        .find(|option| option.name == "limit")
        .and_then(|option| integer_option(&option.value))
        .unwrap_or(5)
        .clamp(1, 10);

    let url = format!(
        "{}/leaderboard?sort={}&limit={}",
        state.config.tensor_api_url, sort, limit
    );
    let response: ApiResponse<Vec<LeaderboardEntry>> =
        state.http.get(url).send().await?.json().await?;
    if !response.success {
        return respond_text(
            ctx,
            command,
            response
                .error
                .as_deref()
                .unwrap_or("Leaderboard unavailable"),
            true,
        )
        .await;
    }

    let entries = response.data.unwrap_or_default();
    if entries.is_empty() {
        return respond_text(ctx, command, "No leaderboard entries found.", true).await;
    }

    let lines = entries
        .iter()
        .enumerate()
        .map(|(index, player)| {
            format!(
                "`#{}` **{}** - {}",
                index + 1,
                player.name,
                format_metric(sort, player)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let embed = CreateEmbed::new()
        .title(format!("8b8t {} Leaderboard", title_case(sort)))
        .description(lines);
    respond_embed(ctx, command, embed).await
}

async fn handle_status(
    ctx: &Context,
    command: &CommandInteraction,
    state: &BotState,
) -> Result<()> {
    let url = format!("{}/health", state.config.tensor_api_url);
    let response: HealthResponse = state.http.get(url).send().await?.json().await?;
    let version = response.version.unwrap_or_else(|| "unknown".to_string());
    respond_text(
        ctx,
        command,
        &format!(
            "Tensor stats API status: `{}` | version `{}`",
            response.status, version
        ),
        false,
    )
    .await
}

async fn respond_text(
    ctx: &Context,
    command: &CommandInteraction,
    content: &str,
    ephemeral: bool,
) -> Result<()> {
    command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content(content)
                    .ephemeral(ephemeral),
            ),
        )
        .await
        .context("Failed to respond to interaction")?;
    Ok(())
}

async fn respond_embed(
    ctx: &Context,
    command: &CommandInteraction,
    embed: CreateEmbed,
) -> Result<()> {
    command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().add_embed(embed),
            ),
        )
        .await
        .context("Failed to respond to interaction")?;
    Ok(())
}

fn start_promo_task(ctx: Context, state: BotState) {
    tokio::spawn(async move {
        let interval_hours = state.config.promo.interval_hours.max(1);
        let interval = Duration::from_secs(interval_hours * 60 * 60);

        loop {
            tokio::time::sleep(interval).await;
            send_promo_messages(&ctx, &state).await;
        }
    });
}

async fn send_promo_messages(ctx: &Context, state: &BotState) {
    let promo = &state.config.promo;
    let content = if promo.message.contains(&promo.shop_url) {
        promo.message.clone()
    } else {
        format!("{}\n{}", promo.message, promo.shop_url)
    };

    for channel_id in &promo.channel_ids {
        if *channel_id == 0 {
            continue;
        }

        if let Err(error) = ChannelId::new(*channel_id).say(&ctx.http, &content).await {
            eprintln!("Failed to send promo message to channel {channel_id}: {error:?}");
        }
    }
}

fn format_metric(sort: &str, player: &LeaderboardEntry) -> String {
    match sort {
        "time_played" => format_time(player.time_played),
        "distance_travelled" => format_distance(player.distance_travelled),
        "kills" => format_number(player.kills),
        "deaths" => format_number(player.deaths),
        "mobs_killed" => format_number(player.mobs_killed),
        "blocks_broken" => format_number(player.blocks_broken),
        "tnt_used" => format_number(player.tnt_used),
        "obsidian_mined" => format_number(player.obsidian_mined),
        "netherite_mined" => format_number(player.netherite_mined),
        "elytra_used" => format_number(player.elytra_used),
        "totems_used" => format_number(player.totems_used),
        "arrows_shot" => format_number(player.arrows_shot),
        _ => format_number(player.kills),
    }
}

fn format_number(value: i64) -> String {
    if value >= 1_000_000 {
        format!("{:.1}M", value as f64 / 1_000_000.0)
    } else if value >= 1_000 {
        format!("{:.1}K", value as f64 / 1_000.0)
    } else {
        value.to_string()
    }
}

fn format_time(ticks: i64) -> String {
    let hours = ticks / 72_000;
    let minutes = (ticks % 72_000) / 1_200;
    if hours > 0 {
        format!("{hours}h {minutes}m")
    } else {
        format!("{minutes}m")
    }
}

fn format_distance(cm: i64) -> String {
    if cm >= 100_000 {
        format!("{:.1}km", cm as f64 / 100_000.0)
    } else if cm >= 100 {
        format!("{}m", cm / 100)
    } else {
        format!("{cm}cm")
    }
}

fn format_timestamp_ms(timestamp_ms: i64) -> String {
    if timestamp_ms <= 0 {
        "unknown".to_string()
    } else {
        format!("<t:{}:d>", timestamp_ms / 1000)
    }
}

fn title_case(value: &str) -> String {
    value
        .split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn string_option(value: &CommandDataOptionValue) -> Option<&str> {
    match value {
        CommandDataOptionValue::String(value) => Some(value.as_str()),
        _ => None,
    }
}

fn integer_option(value: &CommandDataOptionValue) -> Option<i64> {
    match value {
        CommandDataOptionValue::Integer(value) => Some(*value),
        _ => None,
    }
}

fn url_encode(value: &str) -> String {
    value
        .bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                vec![byte as char]
            }
            _ => format!("%{byte:02X}").chars().collect::<Vec<_>>(),
        })
        .collect()
}
