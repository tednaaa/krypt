- currently EMA and Pivot are working incorrect, based on websocket accumulated data
- need to fetch big history via rest api to calculate EMA/Pivot from ~1hr/4hr/1day intervals

- [Flat Detector](https://www.youtube.com/watch?v=dN4G4NQOSDQ)
- [Open Interest](https://www.youtube.com/watch?v=yqnTmqOlYRc)
- [Pump/Dump Screener](https://www.youtube.com/watch?v=pkCOC4F4cJ4)

- Дедубликацию нужно делать на уровне (chain_id, contract_address)

- [ласт версия кода](https://github.dev/tednaaa/krypt/tree/6b5f0a3bd78e6b95ba78fa41199aaca0afedca98)

- Смотрим если в течении нескольких дней (через конфиг) цена, находится по флете, отклонения максимум 5/10%
- Делаем запрос на открытый интерес рест апи и если оно растет или падает, значит, будет памп или дамп
