import { defineConfig } from 'vite';

export default defineConfig({
  root: 'web',
  build: {
    outDir: '../dist',
    emptyOutDir: true,
    rollupOptions: {
      input: {
        main: 'web/index.html',
        dbtest: 'web/dbtest.html',
      },
    },
  },
});
