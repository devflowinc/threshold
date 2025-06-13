import { glob } from "astro/loaders";
import { defineCollection, z } from "astro:content";

const blogPosts = defineCollection({
  loader: glob({
    pattern: "**/*.mdx",
    base: "./src/content/blog-posts",
  }),
  schema: ({ image }) =>
    z.object({
      opengraph: z.object({
        title: z.string(),
        description: z.string(),
        type: z.enum(["website", "article"]).default("website"),
        image: image(),
        publishedTime: z.date().optional(),
        modifiedTime: z.date().optional(),
        author: z.string().optional(),
        primaryCategory: z
          .enum(["news", "blog", "tutorial", "documentation", "about", "other"])
          .default("about"),
      }),
      title: z.string(),
      createdAt: z.date().nullish(),
      lastUpdatedAt: z.date().nullish(),
      ogSection: z.string().optional(),
      isDraft: z.boolean(),
      categories: z.array(z.string()),
    }),
});

const blogCategories = defineCollection({
  loader: glob({
    pattern: "**/*.yaml",
    base: "./src/content/blog-categories",
  }),
  schema: () =>
    z.object({
      name: z.string(),
    }),
});

export const collections = {
  blogPosts,
  blogCategories,
};
