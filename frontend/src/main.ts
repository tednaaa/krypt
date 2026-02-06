import { PiniaColada } from '@pinia/colada';
import { PiniaColadaAutoRefetch } from '@pinia/colada-plugin-auto-refetch';
import { createPinia } from 'pinia';

import { createApp } from 'vue';
import App from './App.vue';

import { router } from './pages/router';
import './assets/css/main.css';

const app = createApp(App);

app.use(router);

app.use(createPinia());
app.use(PiniaColada, {
  queryOptions: {
    staleTime: 60 * 1000, // 1 minute
  },
  plugins: [PiniaColadaAutoRefetch()],
});

app.mount('#app');
