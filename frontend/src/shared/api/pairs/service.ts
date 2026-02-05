import { apiInstance } from '..';
import { PairsResponseSchema } from './schemes';

export class PairsService {
  static async getPairs() {
    const response = await apiInstance.get('/pairs');
    return PairsResponseSchema.parse(response.data);
  }
}
