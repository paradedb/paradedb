/* config.js
 *
 * Retrieves the configuration for the specified Auth0 tenant.
 *
 * To be called via:
 * yarn deploy:[env]
 */

// These don't need to be secrets, they're public values.
const clientIDs = {
  dev: "BpqK7BouXcLWCocOJAZGJjtArh750M1Q",
  prod: "vQ5EPopoZA5tKaYROMRrq2u1zBuWcLrM",
};

const priceIDs = {
  dev: "price_1ODBnKFLdqcXYNJa8VWQPkw8",
  prod: "price_1OMBaBFLdqcXYNJaOoodnpi7",
};

const getConfig = (env) => {
  // List of environment variables that must be defined in order for deploy to succeed.
  const REQUIRED_ENV_VARS = [
    "AUTH0_CLIENT_SECRET",
    "STRIPE_API_KEY",
    "INTERCOM_ACCESS_TOKEN",
  ];

  REQUIRED_ENV_VARS.forEach((v) => {
    if (!process.env[v]) {
      console.error(`Environment variable "${v}" not defined`);
      process.exit(1);
    }
  });

  return {
    AUTH0_DOMAIN: `paradedb-${env}.us.auth0.com`,
    AUTH0_CLIENT_ID: clientIDs[env],
    AUTH0_CLIENT_SECRET: process.env.AUTH0_CLIENT_SECRET,
    AUTH0_BASE_PATH: "src",
    AUTH0_REPLACE_KEYWORD_MAPPINGS: {
      STRIPE_API_KEY: process.env.STRIPE_API_KEY,
      DEFAULT_PRICE_ID: priceIDs[env],
      INTERCOM_ACCESS_TOKEN: process.env.INTERCOM_ACCESS_TOKEN,
    },
    // Only auto-delete resources on dev
    AUTH0_ALLOW_DELETE: env === "dev" && false,
    EXCLUDED_PROPS: {
      clients: ["client_secret"],
    },
  };
};

module.exports = {
  getConfig,
};
