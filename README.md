# Krypt - Real-Time Crypto Market Scanner

A high-performance, real-time cryptocurrency market scanner built in Rust that monitors Binance USDT pairs, detects pump/dump patterns and accumulation/distribution phases, and sends instant Telegram alerts.

## 🎯 Features

- **Real-time monitoring** of 50-150+ active Binance USDT pairs
- **Dynamic symbol prioritization** using a 3-tier scoring system
- **6 detection algorithms**:
  - 🚀 Pump Detection (rapid price increases)
  - 📉 Dump Detection (rapid price decreases)
  - 📊 Accumulation Detection (order flow analysis)
  - ⚠️ Distribution Detection (supply/demand imbalances)
  - ✅ Long Setup Confirmation (breakouts)
  - ✅ Short Setup Confirmation (breakdowns)
- **Cumulative Volume Delta (CVD)** tracking for Tier 1 symbols
- **Smart alerting** with cooldowns and rate limits
- **WebSocket-only** architecture for minimal latency (<500ms)
- **Auto-reconnection** with exponential backoff
- **Alert-only system** - no trading functionality

## 🏗️ Architecture

```
┌─────────────────┐
│  Binance API    │
│  WebSocket      │
└────────┬────────┘
         │
         ├─► All Market Tickers (!ticker@arr)
         │   └─► Updates every ~1 second
         │
         └─► Individual Trade Streams (symbol@trade)
             └─► Only for Tier 1 symbols (top 20)
                 │
                 ▼
┌────────────────────────────────────────────┐
│          Signal Engine                      │
│  ┌──────────────────────────────────────┐  │
│  │  1. Hard Filters                     │  │
│  │     - Min volume: 10M USDT           │  │
│  │     - Min price: 0.0001              │  │
│  │     - Min trades: 5000/24h           │  │
│  └──────────────────────────────────────┘  │
│  ┌──────────────────────────────────────┐  │
│  │  2. Scoring & Tiering                │  │
│  │     - Volume score (40%)             │  │
│  │     - Volatility score (40%)         │  │
│  │     - Activity score (20%)           │  │
│  └──────────────────────────────────────┘  │
│  ┌──────────────────────────────────────┐  │
│  │  3. Pattern Detection                │  │
│  │     - State machine per symbol       │  │
│  │     - CVD analysis (Tier 1)          │  │
│  │     - Momentum detection (all tiers) │  │
│  └──────────────────────────────────────┘  │
└────────────────┬───────────────────────────┘
                 │
                 ▼
┌────────────────────────────────────────────┐
│      Alert Dispatcher                      │
│  - 5-minute cooldown per symbol/type       │
│  - Max 10 alerts/minute globally           │
│  - Priority queue (setups > patterns)      │
└────────────────┬───────────────────────────┘
                 │
                 ▼
         ┌───────────────┐
         │   Telegram    │
         └───────────────┘
```

## 📋 Prerequisites

- **Rust** 1.70+ (install from [rustup.rs](https://rustup.rs))
- **Telegram Bot** (create via [@BotFather](https://t.me/BotFather))
- Stable internet connection
- 4+ CPU cores recommended
- 500MB+ RAM

## 🚀 Quick Start

### 1. Clone and Build

```bash
git clone <your-repo-url>
cd krypt
cargo build --release
```

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

### 4. Run

```bash
# Development mode (with debug logs)
RUST_LOG=debug cargo run

# Production mode
cargo run --release

# Or use the compiled binary
./target/release/krypt
```

### 5. Verify

You should see:

```
INFO  🚀 Crypto Market Scanner starting...
INFO  ✅ Configuration loaded successfully
INFO  ✅ All tasks spawned successfully
INFO  📊 Monitoring Binance USDT pairs...
INFO  Connected to ticker stream
INFO  Rescoring complete: 142 total symbols, 20 Tier 1, 45 Tier 2
```

Wait 1-5 minutes for the first alerts to appear in Telegram!

## 📊 Understanding the Tiers

The scanner uses a dynamic 3-tier system to prioritize computational resources:

### Tier 1 (Score ≥ 0.7, Max 20 symbols)

- **Full analysis** with CVD tracking
- Individual trade stream subscriptions
- All 6 detection algorithms active
- Examples: BTC, ETH, BNB, high-volume altcoins during pumps

### Tier 2 (Score 0.4-0.7)

- Basic pump/dump detection only
- No CVD analysis
- Uses ticker stream data
- Examples: Mid-cap altcoins, moderate volume pairs

### Ignored (Score < 0.4)

- No analysis performed
- Filtered out to save resources
- Examples: Low-volume pairs, stablecoins

**Rescoring happens every 10 seconds** - symbols automatically move between tiers based on market activity.

## 🔔 Alert Types

### 1. 🚀 Pump Detected

**Trigger:** Price increases >3% in 60 seconds with 2x volume spike

Example:

```
🚀 PUMP DETECTED
Symbol: BNBUSDT
Price: $312.45 (+4.2%)
Volume: 3.1x average
Timeframe: 60s
Time: 14:23:05 UTC
```

### 2. 📉 Dump Detected

**Trigger:** Price decreases >3% in 60 seconds with 2x volume spike

### 3. 📊 Accumulation Detected

**Trigger:** Flat price (<0.4% range) + rising CVD + elevated volume

- **Requires Tier 1** (trade stream data)
- Indicates smart money buying without moving price
- Often precedes breakouts

### 4. ⚠️ Distribution Detected

**Trigger:** Price stalling + negative CVD + high volume

- **Requires Tier 1**
- Indicates smart money selling into buying pressure
- Often precedes breakdowns

### 5. ✅ Long Setup Confirmed

**Trigger:** Breakout from accumulation zone with volume + CVD confirmation

- **Highest priority alert**
- Price breaks >0.5% above accumulation range
- Strong buy pressure continues

### 6. ✅ Short Setup Confirmed

**Trigger:** Breakdown from distribution zone with volume + CVD confirmation

- **Highest priority alert**
- Price breaks >0.5% below distribution range
- Strong sell pressure continues

## ⚙️ Configuration Reference

### Filters

```toml
[filters]
min_quote_volume = 10_000_000.0      # Min 24h volume (USDT)
min_price = 0.0001                    # Min price per coin
min_trades_24h = 5000                 # Min number of trades
stale_data_threshold_secs = 10        # Remove if no updates
```

### Scoring

```toml
[scoring]
tier1_threshold = 0.7                 # Score needed for Tier 1
tier2_threshold = 0.4                 # Score needed for Tier 2
max_tier1_symbols = 20                # Max concurrent Tier 1 symbols
rescore_interval_secs = 10            # How often to recalculate

[scoring.weights]
volume_weight = 0.4                   # Importance of volume
volatility_weight = 0.4               # Importance of price movement
activity_weight = 0.2                 # Importance of trade count
```

### Detection

```toml
[detection]
pump_threshold_pct = 3.0              # % change for pump
dump_threshold_pct = -3.0             # % change for dump
accumulation_range_pct = 0.4          # Max price range for accumulation
volume_spike_ratio = 2.0              # Volume multiplier threshold
breakout_threshold_pct = 0.5          # Breakout confirmation %
window_size_secs = 60                 # Window for pump/dump
accumulation_window_secs = 120        # Window for accumulation
distribution_window_secs = 180        # Window for distribution
```

### Telegram

```toml
[telegram]
bot_token = "..."                     # Your bot token
chat_id = "..."                       # Your chat/group ID
alert_cooldown_secs = 300             # 5 min cooldown per symbol/type
max_alerts_per_minute = 10            # Global rate limit
```

### Performance

```toml
[performance]
ticker_channel_size = 1000            # Ticker message buffer
trade_channel_size = 1000             # Trade message buffer
alert_channel_size = 100              # Alert message buffer
price_window_size = 60                # How many price points to keep
cvd_history_size = 300                # 5 minutes of CVD history
```

## 🐛 Troubleshooting

### "Failed to parse config file"

- Check that `config.toml` exists in the same directory
- Verify TOML syntax (no typos, quotes around strings)

### "Please set a valid Telegram bot token"

- Make sure you replaced `YOUR_BOT_TOKEN_HERE` with your actual token
- Token should look like: `123456789:ABCdefGHIjklMNOpqrsTUVwxyz`

### No alerts appearing

1. Check logs for errors: `RUST_LOG=debug cargo run`
2. Verify bot can message you:
   ```bash
   curl -X POST "https://api.telegram.org/bot<TOKEN>/sendMessage" \
        -d "chat_id=<CHAT_ID>" \
        -d "text=Test message"
   ```
3. Lower thresholds temporarily to test:
   ```toml
   pump_threshold_pct = 1.0  # Will trigger more often
   ```

### "WebSocket connection failed"

- Check internet connection
- Verify Binance API is accessible: `curl https://stream.binance.com/`
- Check for firewall/proxy issues

### High CPU usage

- Reduce `max_tier1_symbols` (default: 20)
- Increase `min_quote_volume` to monitor fewer symbols
- Increase `rescore_interval_secs` to reduce calculations

### Missed alerts / Lag

- Check system resources (CPU, RAM, network)
- Verify latency: logs show "Ticker stream" connection messages
- Reduce monitored symbols with stricter filters

## 📈 Performance Targets

- **Latency:** <500ms from exchange event to alert
- **Throughput:** 100+ symbols simultaneously
- **Memory:** <500MB RAM usage
- **CPU:** <50% on modern 4-core CPU
- **Uptime:** 24+ hours without restart
- **Accuracy:** No false positives from duplicate alerts (cooldown working)

## 🔒 Security Notes

- **No trading** - This is an alert-only system
- **No API keys** - Uses public WebSocket streams only
- **No private data** - All market data is public
- Keep your `config.toml` secure (contains Telegram credentials)
- Use environment variables for sensitive config in production:
  ```bash
  export TELEGRAM_BOT_TOKEN="your_token"
  export TELEGRAM_CHAT_ID="your_chat_id"
  ```

## 🧪 Testing

Run unit tests:

```bash
cargo test
```

Run with verbose logging:

```bash
RUST_LOG=trace cargo run
```

Monitor a specific symbol:

```bash
# Edit filters to only show one symbol
[filters]
min_quote_volume = 1000000000.0  # Very high - only BTC/ETH qualify
```

## 📝 Example Alert Flow

```
Time: 00:00
- BNBUSDT enters Tier 1 (score: 0.75)
- Subscribe to BNBUSDT@trade stream

Time: 00:01
- Detect: Price flat (0.3% range), CVD rising
- State: Accumulation
- Alert: "📊 ACCUMULATION DETECTED"

Time: 00:03
- Detect: Price breaks +0.6%, volume spike, CVD continues
- State: BreakoutLong
- Alert: "✅ LONG SETUP CONFIRMED"

Time: 00:08 (5 min cooldown)
- Ready for next alert on BNBUSDT

Time: 00:10
- BNBUSDT score drops to 0.5 → Tier 2
- Unsubscribe from BNBUSDT@trade
```

## 🛠️ Development

### Project Structure

```
krypt/
├── src/
│   ├── main.rs          # Entry point, task orchestration
│   ├── config.rs        # Configuration loading/validation
│   ├── types.rs         # Data structures
│   ├── scoring.rs       # Symbol scoring and tiering
│   ├── detection.rs     # Pattern detection algorithms
│   ├── streams.rs       # WebSocket connection management
│   ├── telegram.rs      # Alert dispatching
│   └── engine.rs        # Main signal engine
├── config.toml          # User configuration
├── Cargo.toml           # Dependencies
└── README.md            # This file
```

### Adding New Detection Algorithms

1. Add new `AlertType` to `types.rs`
2. Implement detection logic in `detection.rs`
3. Add to `Detector::detect()` method
4. Define alert priority in `telegram.rs`

### Key Design Principles

- **Order flow > indicators** - Focus on volume and CVD, not lagging indicators
- **Volume before price** - Volume divergences signal intent
- **Signals are contextual** - Same move means different things in different states
- **Fail gracefully** - Handle all errors without crashing

## 🤝 Contributing

Contributions welcome! Areas for improvement:

- [ ] Bybit integration
- [ ] Historical backtesting mode
- [ ] Web dashboard (real-time charts)
- [ ] More detection patterns (divergences, supply/demand zones)
- [ ] Multi-timeframe analysis
- [ ] Persistent storage (database)
- [ ] Alert sound/vibration customization

## 📄 License

[MIT License](LICENSE)

## ⚠️ Disclaimer

This software is for **educational and informational purposes only**.

- Not financial advice
- Trading cryptocurrency involves substantial risk of loss
- Past performance does not guarantee future results
- The authors are not responsible for any trading losses
- Always do your own research (DYOR)

## 🙏 Acknowledgments

- Binance for public WebSocket API
- Rust community for excellent async libraries
- Order flow traders for sharing insights

---

**Built with ❤️ and Rust 🦀**

For questions, issues, or suggestions, please open a GitHub issue.
