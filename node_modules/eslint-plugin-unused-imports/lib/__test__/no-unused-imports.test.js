const rule = require("../rules/no-unused-imports");
RuleTester = require("eslint").RuleTester;

const ruleTester = new RuleTester({ parserOptions: { ecmaVersion: 2015, sourceType: "module", } });

ruleTester.run("no-unused-imports", rule, {
	valid: [
		{
			code: `
import x from "package";
import { a, b } from "./utils";
import y from "package";

const c = a() + b + x() + y();
`,
		}
	],

	invalid: [
		{
			code: `
import x from "package";
import { a, b } from "./utils";
import y from "package";

const c = b(x, y);
`,
			errors: ["'a' is defined but never used."],
			output: `
import x from "package";
import { b } from "./utils";
import y from "package";

const c = b(x, y);
`
		},
		{
			code: `
import { a, b } from "./utils";
import y from "package";

/**
 * this is a jsdoc!
 */
const c = a(y);
`,
			errors: ["'b' is defined but never used."],
			output: `
import { a } from "./utils";
import y from "package";

/**
 * this is a jsdoc!
 */
const c = a(y);
`
		},
		{
			code: `
import { a } from "./utils";
import y from "package";

const c = 4;
console.log(y);
`,
			errors: ["'a' is defined but never used."],
			output: `
import y from "package";

const c = 4;
console.log(y);
`
		},
		{
			code: `
import y from "package";
import { a } from "./utils";

/**
 * c is the number 4
 */
const c = 4;
console.log(y);
`,
			errors: ["'a' is defined but never used."],
			output: `
import y from "package";

/**
 * c is the number 4
 */
const c = 4;
console.log(y);
`
		}
	]
});