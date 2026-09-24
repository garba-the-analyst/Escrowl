/** @type {import('next').NextConfig} */
const nextConfig = {
  reactStrictMode: true,
  // @escrowl/sdk is a local file: dependency symlinked with TS sources.
  transpilePackages: ["@escrowl/sdk"],
};

export default nextConfig;
