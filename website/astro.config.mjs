// @ts-check
import { defineConfig, fontProviders } from "astro/config";
import tailwindcss from "@tailwindcss/vite";
import mdx from "@astrojs/mdx";
import keystatic from "@keystatic/astro";
import react from "@astrojs/react";
import icon from "astro-icon";
import sitemap from "@astrojs/sitemap";
import { SiteUrl } from "./src/theme.config";

// https://astro.build/config
export default defineConfig({
  site: SiteUrl,
  base: "/",
  output: "static",
  outDir: "./dist",
  devToolbar: {
    enabled: false,
  },
  image: {
    domains: ["127.0.0.1"],
  },
  vite: {
    resolve: {
      alias: {
        "@": "/src",
      },
    },
    plugins: [tailwindcss()],
  },
  integrations: [
    mdx(),
    ...(process.env.SKIP_KEYSTATIC ? [] : [keystatic()]),
    react(),
    icon(),
    sitemap(),
  ],
  experimental: {
    fonts: [
      {
        provider: fontProviders.google(),
        name: "Lora",
        cssVariable: "--font-lora",
      },
    ],
  },
});
