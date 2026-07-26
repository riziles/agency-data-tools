import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [sveltekit()],
  server: {
    proxy: {
      '/arrow.flight.protocol': 'http://localhost:8765',
      '/login': 'http://localhost:8765'
    }
  }
});
