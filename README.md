- currently EMA and Pivot are working incorrect, based on websocket accumulated data
- need to fetch big history via rest api to calculate EMA/Pivot from ~1hr/4hr/1day intervals

- [Flat Detector](https://www.youtube.com/watch?v=dN4G4NQOSDQ)
- [Open Interest](https://www.youtube.com/watch?v=yqnTmqOlYRc)
- [Pump/Dump Screener](https://www.youtube.com/watch?v=pkCOC4F4cJ4)

- Дедубликацию нужно делать на уровне (chain_id, contract_address)

- [ласт версия кода](https://github.dev/tednaaa/krypt/tree/6b5f0a3bd78e6b95ba78fa41199aaca0afedca98)

- Смотрим если в течении нескольких дней (через конфиг) цена, находится по флете, отклонения максимум 5/10%
- Делаем запрос на открытый интерес рест апи и если оно растет или падает, значит, будет памп или дамп

### find cascade liquidations

- If total liquidations in last 60s > threshold
- AND they are mostly one-sided (long or short) => signal

### Instead of fixed price USD, do relative

- лучше следить за ценой + volume
- +10% price change & volume x5 в течении 5 минут в относительно 1-часового окна

> ChatGPT says `You’ll catch hidden gems on alts.`

### Funding

- `High positive funding + long liquidations` => very strong long setup
- `Negative funding + short liquidations` => strong short setup

### Scoring

```
score =
+ liquidation cluster size
+ % OI drop
+ funding extreme
+ counter-trend
+ low-liquidity time
```

- Telegram message like `SCORE: 8.4 / 10`
