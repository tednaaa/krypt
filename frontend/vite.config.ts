/// <reference types="vitest/config" />

import path from 'node:path';
import tailwindcss from '@tailwindcss/vite';
import vue from '@vitejs/plugin-vue';
import { defineConfig } from 'vite';
import VueRouter from 'vue-router/vite';

export default defineConfig({
  plugins: [
    VueRouter({
      dts: 'src/globals/route-map.d.ts',
      exclude: ['src/pages/**/ui/*.vue'],
    }),
    vue(),
    tailwindcss(),
  ],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },

  // @ts-expect-error TODO: strange error
  test: {
    globals: true,
    mockReset: true,
    clearMocks: true,
    restoreMocks: true,
    open: false,
    projects: [
      {
        extends: true,
        test: {
          include: ['src/**/*.spec.ts'],
          setupFiles: ['./spec.setup.ts'],
          name: 'unit',
          environment: 'jsdom',
        },
      },
    ],
  },
});
