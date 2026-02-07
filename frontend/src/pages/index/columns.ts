import type { ColumnDef } from '@tanstack/vue-table';
import type { Pair } from '@/shared/api/pairs';
import { MessageCircleIcon } from 'lucide-vue-next';
import { h } from 'vue';
import { extractTokenFromPair } from '@/shared/lib/pairs/extractTokenFromPair';
import { Button } from '@/shared/ui/button';
import FavoriteButton from './ui/FavoriteButton.vue';
import PairInfoPopover from './ui/PairInfoPopover.vue';

export const columns: ColumnDef<Pair>[] = [
  {
    accessorKey: 'is_favorite',
    header: '',
    enableSorting: false,
    cell: ({ row }) => {
      const pair = row.getValue<string>('pair');
      const isFavorite = row.getValue<boolean>('is_favorite');

      return h(FavoriteButton, { pair, isFavorite });
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
      }, () => [h(MessageCircleIcon, { class: 'size-5' })]);
    },
  },
  {
    accessorKey: 'pair',
    header: 'Symbol',
    enableSorting: false,
    cell: ({ row }) => {
      const pair = row.getValue<string>('pair');
      return h(PairInfoPopover, {
        token: extractTokenFromPair(pair),
        pairImgSrc: row.original.icon,
      });
    },
  },
  {
    accessorKey: 'price',
    header: 'Price',
    cell: ({ row }) => `$${Number.parseFloat(row.getValue<number>('price').toFixed(4))}`,
    enableMultiSort: true,
  },
  { accessorKey: 'mfi_1h', header: 'MFI (1h)', enableMultiSort: true },
  { accessorKey: 'mfi_4h', header: 'MFI (4h)', enableMultiSort: true },
  { accessorKey: 'mfi_1d', header: 'MFI (1d)', enableMultiSort: true },
  { accessorKey: 'mfi_1w', header: 'MFI (1w)', enableMultiSort: true },
];
