# 8b8t Discord Stats Bot

Rust Discord bot for the existing Tensor stats API only. It does not connect to the chat bridge API.

## Setup

1. Create a Discord bot in the Discord Developer Portal.
2. Enable bot permissions for slash commands and sending messages.
3. Invite the bot to your server.
4. Copy config:

```bash
cp config.example.toml config.toml
```

5. Edit `config.toml`:

```toml
discord_token = "YOUR_TOKEN"
guild_id = 123456789012345678
tensor_api_url = "https://api.xera.ca"

[promo]
enabled = true
interval_hours = 6
shop_url = "https://shop.8b8t.me"
message = "Support 8b8t and unlock rank perks at https://shop.8b8t.me"
channel_ids = [123456789012345678, 234567890123456789]
```

6. Run:

```bash
cargo run --release
```

Optional config path:

```bash
DISCORD_STATS_BOT_CONFIG=/path/to/config.toml cargo run --release
```

## Slash Commands

- `/stats player:<name>` - Get player stats from Tensor API.
- `/leaderboard sort:<metric> limit:<1-10>` - Show top players for a metric.
- `/status` - Check Tensor API health.

## Notes

- `guild_id` is recommended during setup because guild slash commands update immediately.
- Set `guild_id = 0` or remove it if you want global slash commands.
- The bot name `8b8t` is configured in the Discord Developer Portal, not in code.
