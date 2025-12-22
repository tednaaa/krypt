# Pump Scanner → Short Bias Alert Bot

A read-only Telegram bot built in Rust using `teloxide` that detects short-term cryptocurrency pump events and posts high-quality alerts to a Telegram channel.

## 🎯 Purpose

This bot:

- **Detects pump events** across Binance Futures and Bybit Perpetuals
- **Evaluates derivatives overheating** (OI, funding rate, long/short ratio)
- **Analyzes technical resistance** (EMAs, pivot points)
- **Posts contextual alerts** to Telegram (no trade signals, no entries)
- **Runs independently** with one strategy per bot instance

## 🏗️ Architecture

### Single Responsibility

Each bot instance:

- Tracks **one strategy** (Pump → Overheating → Short Bias Context)
- Posts to **one Telegram destination** (channel or forum topic)
- Runs **independently** to avoid signal mixing

### Exchange Abstraction

The design uses a `trait Exchange` to allow:

- Adding new exchanges easily
- Replacing metrics sources (e.g., switching to Coinglass)
- Testing with mock exchanges

## 📊 Supported Exchanges

- **Binance Futures (USDT-M)** - Public WebSocket + REST API
- **Bybit USDT Perpetuals** - Public WebSocket + REST API

> ⚠️ **FREE ONLY** - No paid APIs or authentication required

## 🔍 Detection Logic

### 1. Pump Trigger

A symbol enters **PUMP CANDIDATE** state when:

```
Price increase ≥ X% within 5-15 minutes
AND
Volume ≥ Y × average volume
```

Defaults: `X = 5%`, `Y = 2.5×`

### 2. Overheating Qualification

A pump is **qualified** if at least **2 conditions** are met:

**Derivatives Signals:**

- ✅ Open Interest increasing (≥ 10%)
- ✅ Funding rate ≥ threshold (default: 0.025 = 2.5%)
- ✅ Long ratio ≥ threshold (default: 65% longs)

**Technical Signals:**

- ✅ Price extended above EMA 50 / EMA 200
- ✅ Price near Pivot R1 / R2
- ✅ Momentum slowing (rejection, deceleration)

### 3. Alert Emission Rules

- ✅ **One alert per symbol** per cooldown window (default: 5 minutes)
- ❌ No updates or follow-ups
- ❌ No confirmations
- ❌ No "entry" language

## 📨 Alert Format

```
🚨 PUMP DETECTED — BTC/USDT

Price: 52,500.00 USDT (+5.3% in 12m)
Volume: x3.1 vs average

Open Interest: +11.0%
Funding: 3.100%
Long / Short: 71% / 29%

📍 Technical context:
• Price above EMA50: +2.5%, EMA200: +5.1%
• Near Pivot R1
• Momentum slowing: deceleration detected

🔗 Coinglass
```

## 🚀 Setup

### 1. Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### 2. Get Telegram Credentials

**Create a Telegram Bot:**

1. Message [@BotFather](https://t.me/BotFather) on Telegram
2. Send `/newbot` and follow the prompts
3. Save the **bot token** (looks like: `123456789:ABCdefGHIjklMNOpqrsTUVwxyz`)

**Get Your Chat ID:**

1. Message [@userinfobot](https://t.me/userinfobot) on Telegram
2. Save your **chat ID** (numeric, e.g., `123456789`)

Or, to send alerts to a **group/channel**:

1. Create a Telegram group or channel
2. Add your bot to the group/channel (make it an admin for channels)
3. Send a message in the group
4. Visit: `https://api.telegram.org/bot<YOUR_BOT_TOKEN>/getUpdates`
5. Find the `"chat":{"id":` value (negative number for groups/channels)

### 3. Configure

Copy the example config:

```bash
cp config.toml.example config.toml
```

Edit `config.toml`:

```toml
[telegram]
bot_token = "YOUR_BOT_TOKEN_HERE"  # ← Replace this
chat_id = "YOUR_CHAT_ID_HERE"      # ← Replace this
```

**Optional:** Adjust detection parameters:

```toml
[pump]
price_threshold_pct = 5.0         # Minimum % for pump detection
volume_multiplier = 2.5           # Volume must be 2.5x average

[derivatives]
min_funding_rate = 0.025          # Minimum funding rate (2.5%)
min_long_ratio = 0.65             # Minimum long ratio (65%)
min_oi_increase_pct = 10.0        # Minimum OI increase (10%)

[technical]
ema_extension = true              # Check EMA overextension
pivot_proximity = true            # Check pivot resistance proximity
pivot_timeframe_mins = 60         # Use 1H pivots (or 240 for 4H)
emas = [7, 14, 28, 50, 200]       # EMA periods to track
```

### 4. Build and Run

```bash
# Development mode (with logs)
RUST_LOG=info cargo run

# Release mode (optimized)
cargo build --release
./target/release/krypt
```

## 🐳 Docker Deployment

### Build

```bash
docker build -t pump-scanner .
```

### Run

```bash
# Make sure config.toml exists and is configured
docker run -d \
  --name pump-scanner \
  --restart unless-stopped \
  -v $(pwd)/config.toml:/app/config.toml:ro \
  pump-scanner
```

### Production Deployment

```bash
# Create tag to trigger GitHub Actions
git tag -a v1.0.0 -m "Release v1.0.0"
git push origin v1.0.0

# On server, pull and run
URL=ghcr.io/tednaaa/krypt:v1.0.0
docker pull $URL
docker run -d --restart unless-stopped -v $(pwd)/config.toml:/app/config.toml:ro $URL
```

## 📐 Project Structure

```
krypt/
├── src/
│   ├── main.rs              # Main application orchestration
│   ├── config.rs            # Configuration management
│   ├── exchange/            # Exchange abstraction layer
│   │   ├── mod.rs           # Exchange trait definition
│   │   ├── types.rs         # Common types (Symbol, Candle, etc.)
│   │   ├── binance.rs       # Binance Futures implementation
│   │   └── bybit.rs         # Bybit Perpetuals implementation
│   ├── indicators/          # Technical analysis indicators
│   │   ├── ema.rs           # Exponential Moving Average
│   │   └── pivot.rs         # Pivot points calculation
│   ├── pump_scanner/        # Core pump detection logic
│   │   ├── tracker.rs       # Symbol state tracking
│   │   ├── detector.rs      # Pump detection algorithm
│   │   └── qualifier.rs     # Overheating qualification
│   └── telegram/            # Telegram integration
│       └── mod.rs           # Alert formatting and posting
├── config.toml              # Your configuration (not in git)
├── config.toml.example      # Example configuration
└── Cargo.toml               # Rust dependencies
```

## 🔧 Configuration Reference

### Pump Detection

- `price_threshold_pct`: Minimum price increase % (default: 5.0)
- `min_window_mins`: Minimum time window in minutes (default: 5)
- `max_window_mins`: Maximum time window in minutes (default: 15)
- `volume_multiplier`: Volume spike threshold (default: 2.5x)

### Derivatives Thresholds

- `min_funding_rate`: Minimum funding rate (default: 0.025 = 2.5%)
- `min_long_ratio`: Minimum long/short ratio (default: 0.65 = 65% longs)
- `min_oi_increase_pct`: Minimum OI increase (default: 10%)
- `poll_interval_secs`: API polling interval (default: 45s)

### Technical Analysis

- `ema_extension`: Enable EMA overextension checks (default: true)
- `pivot_proximity`: Enable pivot resistance checks (default: true)
- `pivot_timeframe_mins`: Pivot timeframe (60 = 1H, 240 = 4H)
- `emas`: EMA periods to track (default: [7, 14, 28, 50, 200])

### Telegram

- `bot_token`: Your Telegram bot token
- `chat_id`: Target chat/channel ID
- `alert_cooldown_secs`: Cooldown between alerts (default: 300 = 5 min)

## 🎛️ Data Sources

### Real-Time (WebSocket)

- **Binance:** 1m & 5m klines, mark price
- **Bybit:** 1m & 5m klines, tickers

### Periodic REST Polling (30-60s)

- **Open Interest** - Total open positions
- **Funding Rate** - Perpetual swap funding
- **Long/Short Ratio** - Account ratio (global)

## 🚫 Explicit Non-Goals

This bot is **NOT**:

- ❌ An interactive bot (no commands, no user interaction)
- ❌ A signal generator (context only, not trade signals)
- ❌ A trade executor (alerts only)
- ❌ An AI-powered system (rule-based logic)
- ❌ A multi-strategy bot (one strategy per instance)

## 📖 Design Philosophy

> **This bot is a market filter, not a signal generator.**
>
> It reduces noise and highlights structural weakness after strength.

The alerts are designed to:

1. **Identify rapid price movements** (pumps)
2. **Evaluate overheating conditions** (derivatives + technical)
3. **Provide market context** (not trading advice)
4. **Avoid false positives** (minimum 2 conditions required)

## 🔬 Technical Details

### Exchange Trait

```rust
#[async_trait]
pub trait Exchange: Send + Sync {
    fn name(&self) -> &'static str;
    async fn symbols(&self) -> Result<Vec<Symbol>>;
    async fn stream_candles(&self, symbols: &[Symbol], intervals: &[&str]) -> Result<MessageStream>;
    async fn fetch_derivatives_metrics(&self, symbol: &Symbol) -> Result<DerivativesMetrics>;
    async fn fetch_historical_candles(&self, symbol: &Symbol, interval: &str, limit: u32) -> Result<Vec<Candle>>;
}
```

### Adding New Exchanges

1. Implement the `Exchange` trait in `src/exchange/your_exchange.rs`
2. Add to `create_exchange()` function in `src/exchange/mod.rs`
3. Add configuration in `config.rs` and `config.toml`

### Testing

```bash
# Run all tests
cargo test

# Run specific module tests
cargo test pump_scanner::detector
cargo test indicators::ema
cargo test indicators::pivot
```

## 🐛 Troubleshooting

### Bot doesn't connect to Telegram

- Verify `bot_token` is correct
- Check `chat_id` format (numeric, negative for groups)
- Ensure bot is added to channel/group as admin

### No alerts appearing

- Check `RUST_LOG=debug cargo run` for detailed logs
- Verify symbols are being tracked: look for "Tracking N symbols"
- Confirm derivatives data is fetching: look for "Updated derivatives metrics"
- Check pump thresholds aren't too aggressive

### Rate limiting

- Increase `poll_interval_secs` in `[derivatives]` section
- Reduce number of tracked symbols (currently limited to top 50)
- Add delays between API calls (already implemented)

## 📊 Monitoring

The bot logs important events:

- `INFO` - Successful operations (alerts sent, data fetched)
- `WARN` - Non-critical issues (API failures, stale data)
- `DEBUG` - Detailed pump detection analysis
- `ERROR` - Critical failures

View logs:

```bash
# Real-time logs
RUST_LOG=info cargo run

# Docker logs
docker logs -f pump-scanner

# Increase verbosity for debugging
RUST_LOG=debug cargo run
```

## 🤝 Contributing

This is a single-purpose bot. If you want to extend it:

1. **Keep it simple** - Don't add features that violate the non-goals
2. **Maintain abstractions** - Keep the Exchange trait clean
3. **Test your changes** - Add tests for new functionality
4. **Document configuration** - Update config.toml.example

## 📄 License

See [LICENSE](LICENSE) file.

## 🔗 Resources

- [Binance Futures API Docs](https://developers.binance.com/docs/derivatives/usds-margined-futures/general-info)
- [Bybit API Docs](https://bybit-exchange.github.io/docs/v5/intro)
- [Teloxide Documentation](https://docs.rs/teloxide/)
- [Coinglass](https://www.coinglass.com/) - Derivatives data visualization

## ⚠️ Disclaimer

This bot provides **market context only** and is **not financial advice**.

- No guarantees of accuracy or profitability
- Cryptocurrency trading carries significant risk
- Use at your own risk
- Always do your own research (DYOR)

---

Built with 🦀 Rust for speed, safety, and reliability.
