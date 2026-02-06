import { getEnv } from './get-env';

export const envs = {
  API_URL: getEnv('VITE_API_URL'),
};
