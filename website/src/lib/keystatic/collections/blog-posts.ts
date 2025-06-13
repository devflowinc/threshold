import { collection, fields } from "@keystatic/core";
import { opengraph } from "../shared/opengraph";

export const blogPosts = collection({
  label: "Blog posts",
  path: "src/content/blog-posts/*/",
  slugField: "title",
  format: {
    contentField: "content",
  },
  columns: ["title", "createdAt", "isDraft"],
  schema: {
    opengraph,
    title: fields.slug({
      name: {
        label: "Title",
        description: "The title of the post",
      },
      slug: {
        label: "SEO-friendly slug",
        description: "The URL-friendly slug for the post",
      },
    }),
    createdAt: fields.datetime({
      label: "Created at",
      defaultValue: { kind: "now" },
    }),
    lastUpdatedAt: fields.datetime({
      label: "Last updated at",
      defaultValue: { kind: "now" },
    }),
    isDraft: fields.checkbox({
      label: "Draft",
      description: "Is this a draft?",
    }),
    categories: fields.array(
      fields.relationship({
        label: "Categories",
        collection: "blogCategories",
      }),
      { label: "Categories", itemLabel: (i) => i.value! }
    ),
    content: fields.mdx({
      label: "Content",
      description: "The content of the post",
      options: {
        image: {
          directory: "src/assets/images/blog-posts",
          publicPath: "/src/assets/images/blog-posts/",
        },
      },
      components: {},
    }),
  },
});
