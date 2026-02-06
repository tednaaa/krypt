<script setup lang="ts" generic="TData, TValue">
import type { ColumnDef, RowSelectionState, SortingState, VisibilityState } from '@tanstack/vue-table';
import {
  FlexRender,
  getCoreRowModel,
  getSortedRowModel,
  useVueTable,
} from '@tanstack/vue-table';

import { ArrowDownIcon, ArrowUpDownIcon, ArrowUpIcon } from 'lucide-vue-next';
import { computed, ref } from 'vue';

import { valueUpdater } from '@/shared/lib/utils';
import { Button } from '@/shared/ui/button';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/shared/ui/table';

const props = defineProps<{
  columns: ColumnDef<TData, TValue>[];
  data: TData[];
}>();

const sorting = defineModel<SortingState>('sorting', { default: [] });
const columnVisibility = ref<VisibilityState>({});
const rowSelection = ref<RowSelectionState>({});

function getSortIcon(sorted: false | 'asc' | 'desc') {
  if (sorted === 'asc') {
    return ArrowUpIcon;
  }

  if (sorted === 'desc') {
    return ArrowDownIcon;
  }

  return ArrowUpDownIcon;
}

const table = useVueTable({
  get data() { return props.data; },
  get columns() { return props.columns; },

  manualSorting: true,

  getCoreRowModel: getCoreRowModel(),
  // getPaginationRowModel: getPaginationRowModel(),
  getSortedRowModel: getSortedRowModel(),
  onSortingChange: updaterOrValue => valueUpdater(updaterOrValue, sorting),
  onColumnVisibilityChange: updaterOrValue => valueUpdater(updaterOrValue, columnVisibility),
  onRowSelectionChange: updaterOrValue => valueUpdater(updaterOrValue, rowSelection),
  state: {
    get sorting() { return sorting.value; },
    // get columnVisibility() { return columnVisibility.value; },
    get rowSelection() { return rowSelection.value; },
  },
});

const selectedRows = computed(() => table.getFilteredSelectedRowModel().rows.map(row => row.original));

defineExpose({
  table,
  selectedRows,
});
</script>

<template>
  <div>
    <div class="border rounded-sm">
      <Table>
        <TableHeader>
          <TableRow v-for="headerGroup in table.getHeaderGroups()" :key="headerGroup.id">
            <TableHead v-for="header in headerGroup.headers" :key="header.id">
              <div v-if="header.isPlaceholder" />
              <Button
                v-else-if="header.column.getCanSort()"
                variant="ghost"
                size="sm"
                class="h-8 px-2 -ml-2"
                @click="header.column.getToggleSortingHandler()?.($event)"
              >
                <FlexRender
                  :render="header.column.columnDef.header"
                  :props="header.getContext()"
                />
                <component
                  :is="getSortIcon(header.column.getIsSorted())"
                  class="size-4 text-muted-foreground"
                />
              </Button>
              <FlexRender
                v-else
                :render="header.column.columnDef.header"
                :props="header.getContext()"
              />
            </TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          <template v-if="table.getRowModel().rows?.length">
            <TableRow
              v-for="row in table.getRowModel().rows"
              :key="row.id"
              :data-state="row.getIsSelected() ? 'selected' : undefined"
            >
              <TableCell v-for="cell in row.getVisibleCells()" :key="cell.id">
                <FlexRender :render="cell.column.columnDef.cell" :props="cell.getContext()" />
              </TableCell>
            </TableRow>
          </template>
          <template v-else>
            <TableRow>
              <TableCell :colspan="columns.length" class="h-24 text-center">
                No results.
              </TableCell>
            </TableRow>
          </template>
        </TableBody>
      </Table>
    </div>
  </div>
</template>
