import type { AddCommentPayload, RemoveCommentPayload } from './schemes';
import { apiInstance } from '..';

export class CommentsService {
  static async addComment(pair: string, payload: AddCommentPayload) {
    const response = await apiInstance.post('/comments', payload, { params: { pair } });
    return response.data;
  }

  static async removeComment(pair: string, payload: RemoveCommentPayload) {
    const response = await apiInstance.delete('/comments', { params: { pair }, data: payload });
    return response.data;
  }
}
