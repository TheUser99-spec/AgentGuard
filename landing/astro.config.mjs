import { defineConfig } from 'astro/config';
import tailwind from '@astrojs/tailwind';
import react from '@astrojs/react';
import sitemap from '@astrojs/sitemap';

export default defineConfig({
  site: 'https://phylax.pages.dev',
  base: '/',
  integrations: [tailwind(), react()],
  output: 'static',
  build: { assets: 'assets' },
  vite: {
    ssr: { noExternal: ['three', '@react-three/fiber', '@react-three/drei', '@react-three/postprocessing'] }
  }
});
