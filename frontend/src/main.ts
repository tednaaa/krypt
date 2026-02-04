import { createApp } from 'vue';
import App from './App.vue';
import { router } from './pages/router';
import './assets/css/main.css';

createApp(App)
  .use(router)
  .mount('#app');
