```
//@version=6
indicator("MFI", overlay=false)

mfiLength = input.int(14, "MFI Period", minval=1)
oversoldLevel = input.int(20, "Oversold Level", minval=0, maxval=100)
overboughtLevel = input.int(80, "Overbought Level", minval=0, maxval=100)

mfiValue = ta.mfi(close, mfiLength)

// Зелёная зона когда MFI пересекает уровень снизу вверх
// и продолжается пока не достигнет определенного уровня
var bool inOversoldZone = false

if ta.crossover(mfiValue, oversoldLevel)
    inOversoldZone := true

if mfiValue > 50 or mfiValue < oversoldLevel
    inOversoldZone := false

plot(mfiValue, color=color.orange, title="MFI", linewidth=2)
hline(oversoldLevel, "Oversold", color=color.green, linestyle=hline.style_dashed)
hline(overboughtLevel, "Overbought", color=color.red, linestyle=hline.style_dashed)
hline(50, "Middle", color=color.gray, linestyle=hline.style_dotted)

bgcolor(inOversoldZone ? color.new(color.green, 70) : na, title="Oversold Zone")
```
