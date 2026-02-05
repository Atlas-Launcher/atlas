import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  outputFileTracingIncludes: {
    "/api/github/repos": ["./templates/atlas-pack/**"],
  },
};

export default nextConfig;
