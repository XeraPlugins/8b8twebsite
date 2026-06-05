# 8b8t Chat API

Environment variables:

```bash
PLUGIN_SECRET="shared-plugin-secret"
WEBSITE_URL="https://www.8b8t.me"
DATABASE_URL="sqlite://chat.db"
BIND_ADDR="0.0.0.0:5400"

HCAPTCHA_ENABLED="true"
HCAPTCHA_SITE_KEY="your-hcaptcha-site-key"
HCAPTCHA_SECRET="your-hcaptcha-secret"

CHAT_RATE_LIMIT_ENABLED="true"
CHAT_RATE_WINDOW_SECONDS="10"
CHAT_RATE_MAX_MESSAGES="5"
CHAT_RATE_SHORT_LIMIT_SECONDS="60"
CHAT_RATE_LONG_LIMIT_SECONDS="600"
CHAT_RATE_LONG_AFTER_VIOLATIONS="2"
CHAT_RATE_VIOLATION_RESET_SECONDS="3600"

CHAT_ALLOW_LINKS="false"
CHAT_BLOCKED_LINK_PATTERNS="http://,https://,www.,discord.gg,.com,.net,.org,.me,.gg,.io"
```

Security behavior:

- hCaptcha is verified server-side before chat history, live chat, or website message sending.
- Website chat messages are rate-limited per linked Minecraft UUID.
- First spam violation defaults to a 60 second limit.
- Continued spam defaults to a 10 minute limit.
- Links are blocked by default for website-originated messages.
