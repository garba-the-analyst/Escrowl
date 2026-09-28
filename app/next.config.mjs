/** @type {import('next').NextConfig} */
const nextConfig = {
  reactStrictMode: true,
  // @escrowl/sdk is a local file: dependency symlinked with TS sources.
  transpilePackages: ["@escrowl/sdk"],
  // Static export for GitHub Pages project site
  // (https://garba-the-analyst.github.io/Escrowl).
  output: "export",
  basePath: "/Escrowl",
};

export default nextConfig;
