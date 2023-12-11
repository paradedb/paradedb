/* dump.js
 *
 * Updates the local tenant.yaml file to match the current Auth0 configuration. This is necessary
 * so that tenant.yaml matches any modifications made in the Auth0 UI, so that none of those modifications
 * are overwritten on a new code push
 *
 * To be called via:
 * yarn dump:[env]
 */

const fs = require("fs");
const YAML = require("yaml");
const { dump } = require("auth0-deploy-cli");
const { getConfig } = require("./config");

// Auth0 tenant
const env = process.argv[2];
const config = getConfig(env);

const arrayEquals = (l1, l2) => {
  let s = new Set(l2);
  return l1.length === l2.length && l1.every((v) => s.has(v));
};

const applyMappings = (obj, mappings) => {
  if (typeof obj === "string") {
    const [key] = mappings.find(([_, v]) => v === obj) || [];
    if (key) {
      return `##${key}##`;
    }
  }
  if (Array.isArray(obj)) {
    const [key] = mappings.find(([_, v]) => arrayEquals(v, obj)) || [];
    if (key) {
      return `@@${key}@@`;
    }
    return obj.map((item) => applyMappings(item, mappings));
  }
  if (typeof obj === "object") {
    return Object.entries(obj).reduce((acc, [key, value]) => {
      acc[key] = applyMappings(value, mappings);
      return acc;
    }, {});
  }
  return obj;
};

// Retrieve the current Auth0 configuration from Auth0, and apply it
// to the local Auth0 tenant.yaml file.
dump({
  output_folder: ".",
  format: "yaml",
  config: config,
})
  .then(() => {
    const tenant = fs.readFileSync("tenant.yaml", "utf8");
    const mappedYaml = applyMappings(
      YAML.parse(tenant),
      Object.entries(config.AUTH0_KEYWORD_REPLACE_MAPPING ?? {}),
    );
    const output = YAML.stringify(mappedYaml, { schema: "yaml-1.1" })
      // Auth0 errors if @@TAG@@ is in quotes, so we remove the quotes
      .replace(/\"@@|@@\"/g, "@@");
    fs.writeFileSync("tenant.yaml", output);
    console.log(`Successfully dumped from Auth0 ${env} tenant!`);
  })
  .catch((err) => {
    console.error(`Failed to dump from Auth0 ${env} tenant. ${err}`);
    process.exit(1);
  });
