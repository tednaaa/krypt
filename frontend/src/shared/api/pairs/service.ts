import { apiInstance } from '..';
import { PairsResponseSchema } from './schemes';

export class PairsService {
  static async getPairs(params?: { sort?: string }) {
    const response = await apiInstance.get('/pairs', {
      params: params?.sort ? { sort: params.sort } : undefined,
    });
    return PairsResponseSchema.parse(response.data);
  }
}
