import { defineQuery, useQuery } from '@pinia/colada';
import { PairsService } from './service';

export const PAIRS_QUERY_KEYS = {
  root: ['pairs'] as const,
};

export const useGetPairs = defineQuery(() => {
  const { state, ...rest } = useQuery({
    key: PAIRS_QUERY_KEYS.root,
    query: PairsService.getPairs,
  });

  return { ...rest, pairs: state };
});
