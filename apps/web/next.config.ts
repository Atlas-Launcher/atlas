import type { NextConfig } from "next";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);

const nextConfig: NextConfig = {
  pageExtensions: ["ts", "tsx", "md", "mdx"],
};

let withMdx = (config: NextConfig) => config;

try {
  const createMdx = require("@next/mdx");
  withMdx = createMdx({
    extension: /\.mdx?$/,
  });
} catch {
  // MDX package is optional here; docs rendering still works with markdown parsing utilities.
}

export default withMdx(nextConfig);
