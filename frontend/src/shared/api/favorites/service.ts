import { apiInstance } from '..';

export class FavoritesService {
  static async markFavorite(pair: string) {
    const response = await apiInstance.post('/favorites', { params: { pair } });
    return response.data;
  }

  static async unmarkFavorite(pair: string) {
    const response = await apiInstance.delete('/favorites', { params: { pair } });
    return response.data;
  }
}
