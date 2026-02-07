import type { SortingState } from '@tanstack/vue-table';
import type { PairQueryParams } from './schemes';
import { defineQuery, useQuery } from '@pinia/colada';
import { computed, ref } from 'vue';
import { PairsService } from './service';

export const PAIRS_QUERY_KEYS = {
  root: ['pairs'] as const,
  all: (params: PairQueryParams) => [...PAIRS_QUERY_KEYS.root, params] as const,
};

const SORTABLE_FIELDS = new Set(['price', 'mfi_1h', 'mfi_4h', 'mfi_1d', 'mfi_1w']);

export function buildPairsSortParam(sorting: SortingState) {
  if (!sorting.length)
    return undefined;

  return sorting
    .filter(field => SORTABLE_FIELDS.has(field.id))
    .map(field => `${field.id}:${field.desc ? 'desc' : 'asc'}`)
    .join(',');
}

export const useGetPairs = defineQuery(() => {
  const sorting = ref<SortingState>([]);
  const sortParam = computed(() => buildPairsSortParam(sorting.value));

  const isShowingFavorites = ref(false);
  const searchQuery = ref('');

  const params = computed<PairQueryParams>(() => ({
    sort: sortParam.value,
    favorite: isShowingFavorites.value,
  }));

  const { state, ...rest } = useQuery({
    key: () => PAIRS_QUERY_KEYS.all(params.value),
    query: () => PairsService.getPairs(params.value),
    placeholderData: previousData => previousData,
    autoRefetch: true,
  });

  const foundPairs = computed(() => {
    if (state.value.status !== 'success')
      return [];

    return state.value.data.filter(({ pair }) => pair.toLowerCase().includes(searchQuery.value.toLowerCase()));
  });

  return { ...rest, pairs: state, sorting, isShowingFavorites, searchQuery, foundPairs };
});
