```
//@version=6
indicator("ATR Bottom", overlay=true)

// Параметры
atrLength = input.int(14, "ATR Period")
atrMultiplier = input.float(1.5, "Multiplier")
smoothLength1 = input.int(20, "First Smooth")
smoothLength2 = input.int(10, "Second Smooth")

// Расчёт ATR
atrValue = ta.atr(atrLength)

// Динамическое дно с двойным сглаживанием
atrBottomRaw = low - atrValue * atrMultiplier
atrBottomSmooth1 = ta.ema(atrBottomRaw, smoothLength1)
atrBottom = ta.ema(atrBottomSmooth1, smoothLength2)

plot(atrBottom, color=color.green, title="ATR Bottom", linewidth=2)
```

---
