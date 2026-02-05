import path from 'node:path';
import tailwindcss from '@tailwindcss/vite';
import vue from '@vitejs/plugin-vue';
import { defineConfig } from 'vite';
import VueRouter from 'vue-router/vite';

export default defineConfig({
  plugins: [
    VueRouter({
      dts: 'src/globals/route-map.d.ts',
    }),
    vue(),
    tailwindcss(),
  ],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
});
