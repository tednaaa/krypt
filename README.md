## Release

- Create tag, it will push docker image to github registry
- In server run this to update

```bash
URL=ghcr.io/tednaaa/krypt:v1.0.0
docker pull $URL
docker run -d --restart unless-stopped $URL
```

- https://www.binance.com/en/binance-api
- https://developers.binance.com/docs/binance-spot-api-docs/web-socket-streams
- https://developers.binance.com/docs/derivatives/usds-margined-futures/general-info

## Setup

### 2. Get Telegram Credentials

**Create a Telegram Bot:**

1. Message [@BotFather](https://t.me/BotFather) on Telegram
2. Send `/newbot` and follow the prompts
3. Save the **bot token** (looks like: `123456789:ABCdefGHIjklMNOpqrsTUVwxyz`)

**Get Your Chat ID:**

1. Message [@userinfobot](https://t.me/userinfobot) on Telegram
2. Save your **chat ID** (numeric, e.g., `123456789`)

Or, to send alerts to a group:

1. Create a Telegram group
2. Add your bot to the group
3. Send a message in the group
4. Visit: `https://api.telegram.org/bot<YOUR_BOT_TOKEN>/getUpdates`
5. Find the `"chat":{"id":` value (negative number for groups)

### 3. Configure

Edit `config.toml`:

```toml
[telegram]
bot_token = "YOUR_BOT_TOKEN_HERE"  # ← Replace this
chat_id = "YOUR_CHAT_ID_HERE"      # ← Replace this
```

**Optional:** Adjust detection parameters:

```toml
[detection]
pump_threshold_pct = 3.0           # Minimum % for pump alert
dump_threshold_pct = -3.0          # Minimum % for dump alert
volume_spike_ratio = 2.0           # Volume must be 2x average

[filters]
min_quote_volume = 10_000_000.0    # Minimum 24h volume in USDT
min_trades_24h = 5000              # Minimum number of trades
```
