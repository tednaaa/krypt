import type { PairQueryParams } from './schemes';
import { apiInstance } from '..';
import { PairsResponseSchema } from './schemes';

export class PairsService {
  static async getPairs(params?: PairQueryParams) {
    const response = await apiInstance.get('/pairs', {
      params: {
        sort: params?.sort,
        favorite: params?.favorite ? 'true' : undefined,
      },
    });
    return PairsResponseSchema.parse(response.data);
  }
}
