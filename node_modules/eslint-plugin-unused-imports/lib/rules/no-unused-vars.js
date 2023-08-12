/**
 * @fileoverview Filter imports from no-unused-vars.
 * @author Mikkel Holmer Pedersen <mikkel@holmerp.dk>
 */
"use strict";

const { unusedVarsPredicate } = require("./predicates");

const ruleComposer = require("eslint-rule-composer");

let rule;
try {
	const tslint = require("@typescript-eslint/eslint-plugin");
	rule = tslint.rules["no-unused-vars"];
} catch (_) {
	const eslint = require("eslint");
	rule = new eslint.Linter().getRules().get("no-unused-vars");
}

rule.meta.fixable = "code";
rule.meta.docs.url = "https://github.com/sweepline/eslint-plugin-unused-imports/blob/master/docs/rules/no-unused-vars.md";

module.exports = ruleComposer.filterReports(rule, unusedVarsPredicate);
