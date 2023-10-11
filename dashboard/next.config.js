/** @type {import('next').NextConfig} */
const nextConfig = {
  env: {
    INTERCOM_APP_ID: process.env.INTERCOM_APP_ID,
  },
};

module.exports = nextConfig;
