/* deploy.js
 *
 * Deploys the configuration defined by tenant.yaml to a given Auth0 tenant.
 *
 * To be called via:
 * yarn deploy:[env]
 */

const { deploy } = require("auth0-deploy-cli");
const { getConfig } = require("./config");

const env = process.argv[2];

// Deploy to Auth0 tenant
deploy({
  input_file: "tenant.yaml",
  config: getConfig(env),
})
  .then(() => console.log(`Successfully deployed to Auth0 ${env} tenant!`))
  .catch((err) => {
    console.error(`Failed to deploy to Auth0 ${env} tenant. ${err}`);
    process.exit(1);
  });
