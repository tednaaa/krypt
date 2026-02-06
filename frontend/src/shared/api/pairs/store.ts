import type { SortingState } from '@tanstack/vue-table';
import { defineQuery, useQuery } from '@pinia/colada';
import { computed, ref } from 'vue';
import { PairsService } from './service';

export const PAIRS_QUERY_KEYS = {
  root: ['pairs'] as const,
};

const SORTABLE_FIELDS = new Set(['mfi_1h', 'mfi_4h', 'mfi_1d', 'mfi_1w']);

export function buildPairsSortParam(sorting: SortingState) {
  return sorting
    .filter(field => SORTABLE_FIELDS.has(field.id))
    .map(field => `${field.id}:${field.desc ? 'desc' : 'asc'}`)
    .join(',');
}

export const useGetPairs = defineQuery(() => {
  const sorting = ref<SortingState>([]);
  const sortParam = computed(() => buildPairsSortParam(sorting.value));

  const { state, ...rest } = useQuery({
    key: computed(() => [...PAIRS_QUERY_KEYS.root, sortParam.value]),
    query: () => PairsService.getPairs({ sort: sortParam.value || undefined }),
    placeholderData: previousData => previousData,
  });

  return { ...rest, pairs: state, sorting };
});
