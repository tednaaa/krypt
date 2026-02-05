import type { ColumnDef } from '@tanstack/vue-table';
import type { Pair } from '@/shared/api/pairs';
import { MessageCircleIcon, StarIcon } from 'lucide-vue-next';
import { h } from 'vue';
import { Button } from '@/shared/ui/button';

export const columns: ColumnDef<Pair>[] = [
  {
    accessorKey: 'is_favorite',
    header: '',
    enableSorting: false,
    cell: ({ row }) => {
      const pair = row.getValue<string>('pair');

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
      const pair = row.getValue<string>('pair');

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

      return h('div', pair);
    },
  },
  { accessorKey: 'mfi_1h', header: 'MFI (1h)', enableMultiSort: true },
  { accessorKey: 'mfi_4h', header: 'MFI (4h)', enableMultiSort: true },
  { accessorKey: 'mfi_1d', header: 'MFI (1d)', enableMultiSort: true },
  { accessorKey: 'mfi_1w', header: 'MFI (1w)', enableMultiSort: true },
];
