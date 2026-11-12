import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vite';

const page = (name: string) => fileURLToPath(new URL(`web/${name}.html`, import.meta.url));

export default defineConfig({
  root: 'web',
  build: {
    outDir: '../dist',
    emptyOutDir: true,
    rollupOptions: {
      input: {
        main: page('index'),
        dbtest: page('dbtest'),
        heatmap: page('heatmap'),
      },
    },
  },
});
