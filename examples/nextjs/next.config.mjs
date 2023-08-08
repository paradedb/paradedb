// next.config.mjs

/**
 * Next.js configuration.
 */

/**
 * @type {import('next').NextConfig}
 */

const config = {
  images: {
    remotePatterns: [
      {
        protocol: "https",
        hostname: "*",
      },
    ],
  },
  env: {
    RETAKE_API_URL: process.env.RETAKE_API_URL,
    RETAKE_API_KEY: process.env.RETAKE_API_KEY,
    DATABASE_TABLE_NAME: process.env.DATABASE_TABLE_NAME,
    DATABASE_TABLE_COLUMNS: process.env.DATABASE_TABLE_COLUMNS,
  },
  eslint: {
    ignoreDuringBuilds: true,
  },
}

export default config
