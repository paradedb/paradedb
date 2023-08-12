# eslint-plugin-n

> forked from [eslint-plugin-node](https://www.npmjs.com/package/eslint-plugin-node) v11.1.0. as the original repository seems [no longer maintained](https://github.com/mysticatea/eslint-plugin-node/issues/300).

[![npm version](https://img.shields.io/npm/v/eslint-plugin-n.svg)](https://www.npmjs.com/package/eslint-plugin-n) [![Downloads/month](https://img.shields.io/npm/dm/eslint-plugin-n.svg)](http://www.npmtrends.com/eslint-plugin-n) [![Build Status](https://github.com/weiran-zsd/eslint-plugin-node/workflows/CI/badge.svg)](https://github.com/weiran-zsd/eslint-plugin-node/actions) [![Coverage Status](https://codecov.io/gh/weiran-zsd/eslint-plugin-node/branch/master/graph/badge.svg)](https://codecov.io/gh/weiran-zsd/eslint-plugin-node) [![Dependency Status](https://david-dm.org/weiran-zsd/eslint-plugin-node.svg)](https://david-dm.org/weiran-zsd/eslint-plugin-node)

Additional ESLint's rules for Node.js

## 💿 Install & Usage

```sh
npm install --save-dev eslint eslint-plugin-n
```

- Requires Node.js `>=16.0.0`
- Requires ESLint `>=7.0.0`

**Note:** It recommends a use of [the "engines" field of package.json](https://docs.npmjs.com/files/package.json#engines). The "engines" field is used by `n/no-unsupported-features/*` rules.

### **[.eslintrc.json](https://eslint.org/docs/latest/use/configure/configuration-files)** (An example)

```jsonc
{
    "extends": ["eslint:recommended", "plugin:n/recommended"],
    "parserOptions": {
        "ecmaVersion": 2021
    },
    "rules": {
        "n/exports-style": ["error", "module.exports"]
    }
}
```

### [`eslint.config.js`](https://eslint.org/docs/latest/use/configure/configuration-files-new) (requires eslint>=v8.23.0)

```js
const nodeRecommendedScript = require("eslint-plugin-n/configs/recommended-script")

module.exports = [
    nodeRecommendedScript,
    {
        rules: {
            "n/exports-style": ["error", "module.exports"]
        }
    }
]
```

**package.json** (An example)

```json
{
    "name": "your-module",
    "version": "1.0.0",
    "type": "commonjs",
    "engines": {
        "node": ">=8.10.0"
    }
}
```

### Configured Node.js version range

The rules get the supported Node.js version range from the following, falling back to the next if unspecified:

1. Rule configuration `version`
2. ESLint [shared setting](http://eslint.org/docs/user-guide/configuring.html#adding-shared-settings) `node.version`
3. `package.json` [`engines`] field
4. `>=16.0.0`

If you omit the [engines] field, this rule chooses `>=16.0.0` as the configured Node.js version since `16` is the maintained lts (see also [Node.js Release Working Group](https://github.com/nodejs/Release#readme)).

For Node.js packages, using the [`engines`] field is recommended because it's the official way to indicate support:

```json
{
    "name": "your-module",
    "version": "1.0.0",
    "engines": {
        "node": ">=16.0.0"
    }
}
```

For [Shareable Configs](https://eslint.org/docs/latest/developer-guide/shareable-configs) or packages with a different development environment (e.g. pre-compiled, web package, etc.), you can configure ESLint with `settings.node.version` to specify support.

## 📖 Rules

<!-- begin auto-generated rules list -->

💼 [Configurations](https://github.com/eslint-community/eslint-plugin-n#-configs) enabled in.\
✅ Set in the `recommended` [configuration](https://github.com/eslint-community/eslint-plugin-n#-configs).\
🔧 Automatically fixable by the [`--fix` CLI option](https://eslint.org/docs/user-guide/command-line-interface#--fix).\
❌ Deprecated.

### Best Practices

| Name                                                 | Description              | 💼 | 🔧 | ❌  |
| :--------------------------------------------------- | :----------------------- | :- | :- | :- |
| [no-deprecated-api](docs/rules/no-deprecated-api.md) | disallow deprecated APIs | ✅  |    |    |

### Possible Errors

| Name                                                                                         | Description                                                                 | 💼 | 🔧 | ❌  |
| :------------------------------------------------------------------------------------------- | :-------------------------------------------------------------------------- | :- | :- | :- |
| [handle-callback-err](docs/rules/handle-callback-err.md)                                     | require error handling in callbacks                                         |    |    |    |
| [no-callback-literal](docs/rules/no-callback-literal.md)                                     | enforce Node.js-style error-first callback pattern is followed              |    |    |    |
| [no-exports-assign](docs/rules/no-exports-assign.md)                                         | disallow the assignment to `exports`                                        | ✅  |    |    |
| [no-extraneous-import](docs/rules/no-extraneous-import.md)                                   | disallow `import` declarations which import extraneous modules              | ✅  |    |    |
| [no-extraneous-require](docs/rules/no-extraneous-require.md)                                 | disallow `require()` expressions which import extraneous modules            | ✅  |    |    |
| [no-hide-core-modules](docs/rules/no-hide-core-modules.md)                                   | disallow third-party modules which are hiding core modules                  |    |    | ❌  |
| [no-missing-import](docs/rules/no-missing-import.md)                                         | disallow `import` declarations which import non-existence modules           | ✅  |    |    |
| [no-missing-require](docs/rules/no-missing-require.md)                                       | disallow `require()` expressions which import non-existence modules         | ✅  |    |    |
| [no-new-require](docs/rules/no-new-require.md)                                               | disallow `new` operators with calls to `require`                            |    |    |    |
| [no-path-concat](docs/rules/no-path-concat.md)                                               | disallow string concatenation with `__dirname` and `__filename`             |    |    |    |
| [no-process-exit](docs/rules/no-process-exit.md)                                             | disallow the use of `process.exit()`                                        | ✅  |    |    |
| [no-unpublished-bin](docs/rules/no-unpublished-bin.md)                                       | disallow `bin` files that npm ignores                                       | ✅  |    |    |
| [no-unpublished-import](docs/rules/no-unpublished-import.md)                                 | disallow `import` declarations which import private modules                 | ✅  |    |    |
| [no-unpublished-require](docs/rules/no-unpublished-require.md)                               | disallow `require()` expressions which import private modules               | ✅  |    |    |
| [no-unsupported-features](docs/rules/no-unsupported-features.md)                             | disallow unsupported ECMAScript features on the specified version           |    |    | ❌  |
| [no-unsupported-features/es-builtins](docs/rules/no-unsupported-features/es-builtins.md)     | disallow unsupported ECMAScript built-ins on the specified version          | ✅  |    |    |
| [no-unsupported-features/es-syntax](docs/rules/no-unsupported-features/es-syntax.md)         | disallow unsupported ECMAScript syntax on the specified version             | ✅  |    |    |
| [no-unsupported-features/node-builtins](docs/rules/no-unsupported-features/node-builtins.md) | disallow unsupported Node.js built-in APIs on the specified version         | ✅  |    |    |
| [process-exit-as-throw](docs/rules/process-exit-as-throw.md)                                 | require that `process.exit()` expressions use the same code path as `throw` | ✅  |    |    |
| [shebang](docs/rules/shebang.md)                                                             | require correct usage of shebang                                            | ✅  | 🔧 |    |

### Stylistic Issues

| Name                                                                             | Description                                                             | 💼 | 🔧 | ❌  |
| :------------------------------------------------------------------------------- | :---------------------------------------------------------------------- | :- | :- | :- |
| [callback-return](docs/rules/callback-return.md)                                 | require `return` statements after callbacks                             |    |    |    |
| [exports-style](docs/rules/exports-style.md)                                     | enforce either `module.exports` or `exports`                            |    | 🔧 |    |
| [file-extension-in-import](docs/rules/file-extension-in-import.md)               | enforce the style of file extensions in `import` declarations           |    | 🔧 |    |
| [global-require](docs/rules/global-require.md)                                   | require `require()` calls to be placed at top-level module scope        |    |    |    |
| [no-mixed-requires](docs/rules/no-mixed-requires.md)                             | disallow `require` calls to be mixed with regular variable declarations |    |    |    |
| [no-process-env](docs/rules/no-process-env.md)                                   | disallow the use of `process.env`                                       |    |    |    |
| [no-restricted-import](docs/rules/no-restricted-import.md)                       | disallow specified modules when loaded by `import` declarations         |    |    |    |
| [no-restricted-require](docs/rules/no-restricted-require.md)                     | disallow specified modules when loaded by `require`                     |    |    |    |
| [no-sync](docs/rules/no-sync.md)                                                 | disallow synchronous methods                                            |    |    |    |
| [prefer-global/buffer](docs/rules/prefer-global/buffer.md)                       | enforce either `Buffer` or `require("buffer").Buffer`                   |    |    |    |
| [prefer-global/console](docs/rules/prefer-global/console.md)                     | enforce either `console` or `require("console")`                        |    |    |    |
| [prefer-global/process](docs/rules/prefer-global/process.md)                     | enforce either `process` or `require("process")`                        |    |    |    |
| [prefer-global/text-decoder](docs/rules/prefer-global/text-decoder.md)           | enforce either `TextDecoder` or `require("util").TextDecoder`           |    |    |    |
| [prefer-global/text-encoder](docs/rules/prefer-global/text-encoder.md)           | enforce either `TextEncoder` or `require("util").TextEncoder`           |    |    |    |
| [prefer-global/url](docs/rules/prefer-global/url.md)                             | enforce either `URL` or `require("url").URL`                            |    |    |    |
| [prefer-global/url-search-params](docs/rules/prefer-global/url-search-params.md) | enforce either `URLSearchParams` or `require("url").URLSearchParams`    |    |    |    |
| [prefer-promises/dns](docs/rules/prefer-promises/dns.md)                         | enforce `require("dns").promises`                                       |    |    |    |
| [prefer-promises/fs](docs/rules/prefer-promises/fs.md)                           | enforce `require("fs").promises`                                        |    |    |    |

<!-- end auto-generated rules list -->

## 🔧 Configs

This plugin provides three configs:

| Name | Description |
| :-- | :-- |
| `plugin:n/recommended` | Considers both CommonJS and ES Modules. If [`"type":"module"` field](https://medium.com/@nodejs/announcing-a-new-experimental-modules-1be8d2d6c2ff#b023) existed in package.json then it considers files as ES Modules. Otherwise it considers files as CommonJS. In addition, it considers `*.mjs` files as ES Modules and `*.cjs` files as CommonJS. |
| `plugin:n/recommended-module` | Considers all files as ES Modules. |
| `plugin:n/recommended-script` | Considers all files as CommonJS. |

These preset configs:

- enable [no-process-exit](http://eslint.org/docs/rules/no-process-exit) rule because [the official document](https://nodejs.org/api/process.html#process_process_exit_code) does not recommend a use of `process.exit()`.
- enable plugin rules which are given ✅ in the above table.
- add `{ecmaVersion: 2021}` and etc into `parserOptions`.
- add proper globals into `globals`.
- add this plugin into `plugins`.

## 👫 FAQ

- Q: The `no-missing-import` / `no-missing-require` rules don't work with nested folders in SublimeLinter-eslint
- A: See [context.getFilename() in rule returns relative path](https://github.com/roadhump/SublimeLinter-eslint#contextgetfilename-in-rule-returns-relative-path) in the SublimeLinter-eslint FAQ.

- Q: How to use the new eslint config with mixed commonjs and es modules?
- A: The `recommended` config is no longer exported. You can create a config based on `recommended-script` and `recommended-module`. An example:

```js
const nodeRecommendedScript = require("eslint-plugin-n/configs/recommended-script");
const nodeRecommendedModule = require("eslint-plugin-n/configs/recommended-module");

module.exports = [
    {
        files: ["**/*.js", "**/*.cjs"],
        ...nodeRecommendedScript
    },
    {
        files: ["**/*.mjs"],
        ...nodeRecommendedModule
    }
]
```

## 🚥 Semantic Versioning Policy

`eslint-plugin-n` follows [semantic versioning](http://semver.org/) and [ESLint's Semantic Versioning Policy](https://github.com/eslint/eslint#semantic-versioning-policy).

- Patch release (intended to not break your lint build)
  - A bug fix in a rule that results in it reporting fewer errors.
  - Improvements to documentation.
  - Non-user-facing changes such as refactoring code, adding, deleting, or modifying tests, and increasing test coverage.
  - Re-releasing after a failed release (i.e., publishing a release that doesn't work for anyone).
- Minor release (might break your lint build)
  - A bug fix in a rule that results in it reporting more errors.
  - A new rule is created.
  - A new option to an existing rule is created.
  - An existing rule is deprecated.
- Major release (likely to break your lint build)
  - A support for old Node version is dropped.
  - A support for old ESLint version is dropped.
  - An existing rule is changed in it reporting more errors.
  - An existing rule is removed.
  - An existing option of a rule is removed.
  - An existing config is updated.

Deprecated rules follow ESLint's [deprecation policy](https://eslint.org/docs/user-guide/rule-deprecation).

## 📰 Changelog

- [GitHub Releases](https://github.com/weiran-zsd/eslint-plugin-node/releases)

## ❤️ Contributing

Welcome contributing!

Please use GitHub's Issues/PRs.

### Development Tools

- `npm test` runs tests and measures coverage.
- `npm run coverage` shows the coverage result of `npm test` command.
- `npm run clean` removes the coverage result of `npm test` command.
