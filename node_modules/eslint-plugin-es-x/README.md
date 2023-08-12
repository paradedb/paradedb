# eslint-plugin-es-x

[![npm version](https://img.shields.io/npm/v/eslint-plugin-es-x.svg)](https://www.npmjs.com/package/eslint-plugin-es-x)
[![Downloads/month](https://img.shields.io/npm/dm/eslint-plugin-es-x.svg)](http://www.npmtrends.com/eslint-plugin-es-x)
[![Build Status](https://github.com/eslint-community/eslint-plugin-es-x/workflows/CI/badge.svg)](https://github.com/eslint-community/eslint-plugin-es-x/actions)

ESLint plugin which disallows each ECMAScript syntax.

> Forked from [eslint-plugin-es](https://github.com/mysticatea/eslint-plugin-es). As the original repository seems [no longer maintained](https://github.com/mysticatea/eslint-plugin-es/issues/72).

## 🏁 Goal

[Espree](https://github.com/eslint/espree#readme), the default parser of [ESLint](https://eslint.org/), has supported `ecmaVersion` option.
However, the error messages of new syntax are not readable (e.g., "unexpected token" or something like).

When we use this plugin along with the latest `ecmaVersion` option value, it tells us the readable error message for the new syntax, such as "ES2020 BigInt is forbidden."
Plus, this plugin lets us disable each syntactic feature individually.

## 📖 Usage

See [documentation](https://eslint-community.github.io/eslint-plugin-es-x/)

## 🚥 Semantic Versioning Policy

This plugin follows [semantic versioning](http://semver.org/) and [ESLint's semantic versioning policy](https://github.com/eslint/eslint#semantic-versioning-policy).

- We will release a new minor version to add new rules when TC39 decided to advance proposals to Stage 4. In the minor releases, we don't update configs.
- We will release a new major version to update configs when new ECMAScript snapshots are available.

## 📰 Changelog

See [releases](https://github.com/eslint-community/eslint-plugin-es-x/releases).

## ❤️ Contributing

Welcome contributing!

Please use GitHub's Issues/PRs.

### Development Tools

- `npm test` runs tests and measures coverage.
- `npm run clean` removes the coverage result of `npm test` command.
- `npm run coverage` shows the coverage result of the last `npm test` command.
- `npm run docs:build` builds documentation.
- `npm run docs:watch` builds documentation on each file change.
- `npm run watch` runs tests on each file change.
