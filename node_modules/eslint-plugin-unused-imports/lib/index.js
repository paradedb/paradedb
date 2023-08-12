/**
 * @fileoverview Find and remove unused es6 modules
 * @author Mikkel Holmer Pedersen
 */
"use strict";

const noUnusedVars = require("./rules/no-unused-vars");
const noUnusedImports = require("./rules/no-unused-imports");

// import all rules in lib/rules
module.exports.rules = {
	"no-unused-vars": noUnusedVars,
	"no-unused-imports": noUnusedImports,
	"no-unused-vars-ts": noUnusedVars,
	"no-unused-imports-ts": noUnusedImports,
};
