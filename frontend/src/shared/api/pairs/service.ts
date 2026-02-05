import { apiInstance } from '..';
import { PairsResponse } from './schemes';

export class PairsService {
  static async getPairs() {
    const response = await apiInstance.get('/pairs');
    return PairsResponse.parse(response.data);
  }
}
