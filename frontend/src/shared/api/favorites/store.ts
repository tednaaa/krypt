import { defineMutation, useQueryCache } from '@pinia/colada';
import { PAIRS_QUERY_KEYS } from '../pairs';
import { FavoritesService } from './service';

export const useMarkFavorite = defineMutation({
  mutation: FavoritesService.markFavorite,
  onSuccess: () => {
    const cache = useQueryCache();
    cache.invalidateQueries({ key: PAIRS_QUERY_KEYS.root });
  },
});

export const useUnmarkFavorite = defineMutation({
  mutation: FavoritesService.unmarkFavorite,
  onSuccess: () => {
    const cache = useQueryCache();
    cache.invalidateQueries({ key: PAIRS_QUERY_KEYS.root });
  },
});
