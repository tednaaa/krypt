import axios from 'axios';
import { envs } from '@/config/envs';

export const apiInstance = axios.create({
  baseURL: envs.API_URL,
});
