<script setup lang='ts'>
import { computed } from 'vue';
import { Popover, PopoverContent, PopoverTrigger } from '@/shared/ui/popover';
import Link from './Link.vue';

const props = defineProps<{
  token: string;
}>();

const exchange = 'Binance';
const coinglassSymbol = computed(() => `${props.token}USDT`);
const tradingviewSymbol = computed(() => `${props.token}USDT.P`);

const coinglassUrl = computed(() => `https://www.coinglass.com/tv/${exchange}_${coinglassSymbol.value}`);
const tradingviewUrl = computed(() => `https://www.tradingview.com/chart?symbol=${exchange}:${tradingviewSymbol.value}`);
const liquidationHeatmapUrl = computed(() => `https://www.coinglass.com/pro/futures/LiquidationHeatMap?coin=${props.token}`);

const bingxUrl = computed(() => `https://bingx.com/en/perpetual/${props.token}-USDT`);
</script>

<template>
  <Popover>
    <PopoverTrigger class="font-medium cursor-pointer hover:text-blue-600 ">{{ props.token }}</PopoverTrigger>
    <PopoverContent align="start">
      <h4>{{ props.token }}</h4>
      <hr class="my-4">
      <div class="flex flex-col gap-2">
        <Link
          label="CoinGlass"
          img-src="https://www.coinglass.com/favicon.ico"
          :href="coinglassUrl"
        />

        <Link
          label="Liquidations HeatMap"
          img-src="https://www.coinglass.com/favicon.ico"
          :href="liquidationHeatmapUrl"
        />

        <Link
          label="TradingView"
          img-src="https://static.tradingview.com/static/images/favicon.ico"
          :href="tradingviewUrl"
        />
      </div>
      <hr class="my-4">
      <div>
        <Link
          label="BingX"
          img-src="https://bingx.com/favicon.ico"
          :href="bingxUrl"
        />
      </div>
    </PopoverContent>
  </Popover>
</template>
