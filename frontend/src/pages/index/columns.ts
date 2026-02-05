import type { ColumnDef } from '@tanstack/vue-table';
import type { Pair } from '@/shared/api/pairs';
import { MessageCircleIcon, StarIcon } from 'lucide-vue-next';
import { h } from 'vue';
import { Button } from '@/shared/ui/button';
import { HoverCard, HoverCardContent, HoverCardTrigger } from '@/shared/ui/hover-card';

export const columns: ColumnDef<Pair>[] = [
  {
    accessorKey: 'is_favorite',
    header: '',
    enableSorting: false,
    cell: ({ row }) => {
      row.getValue<string>('pair');

      return h(Button, {
        variant: 'ghost',
        size: 'icon',
      }, () => [h(StarIcon)]);
    },
  },
  {
    accessorKey: 'comments',
    header: '',
    enableSorting: false,
    cell: ({ row }) => {
      row.getValue<string>('pair');

      return h(Button, {
        variant: 'ghost',
        size: 'icon',
      }, () => [h(MessageCircleIcon)]);
    },
  },
  {
    accessorKey: 'pair',
    header: 'Symbol',
    enableSorting: false,
    cell: ({ row }) => {
      const pair = row.getValue<string>('pair');
      const exchange = 'Binance';
      const coinglassSymbol = `${pair}`;
      const tradingviewSymbol = `${pair}.P`;

      return h(HoverCard, null, [
        h(HoverCardTrigger, { class: 'cursor-pointer hover:underline' }, pair),
        h(HoverCardContent, { class: 'w-auto' }, h('div', { class: 'flex flex-col gap-2' }, [
          h('a', {
            href: `https://www.coinglass.com/tv/${exchange}_${coinglassSymbol}`,
            target: '_blank',
            rel: 'noopener noreferrer',
            class: 'text-blue-600 hover:text-blue-800',
          }, 'CoinGlass'),
          h('a', {
            href: `https://www.tradingview.com/chart?symbol=${exchange}:${tradingviewSymbol}`,
            target: '_blank',
            rel: 'noopener noreferrer',
            class: 'text-blue-600 hover:text-blue-800',
          }, 'TradingView'),
          h('a', {
            href: `https://www.coinglass.com/pro/futures/LiquidationHeatMap?coin=${pair}`,
            target: '_blank',
            rel: 'noopener noreferrer',
            class: 'text-blue-600 hover:text-blue-800',
          }, 'Liquidations HeatMap'),
        ])),
      ]);
    },
  },
  { accessorKey: 'mfi_1h', header: 'MFI (1h)', enableMultiSort: true },
  { accessorKey: 'mfi_4h', header: 'MFI (4h)', enableMultiSort: true },
  { accessorKey: 'mfi_1d', header: 'MFI (1d)', enableMultiSort: true },
  { accessorKey: 'mfi_1w', header: 'MFI (1w)', enableMultiSort: true },
];
