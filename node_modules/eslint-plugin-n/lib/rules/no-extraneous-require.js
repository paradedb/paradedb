/**
 * @author Toru Nagashima
 * See LICENSE file in root directory for full license.
 */
"use strict"

const { checkExtraneous, messages } = require("../util/check-extraneous")
const getAllowModules = require("../util/get-allow-modules")
const getConvertPath = require("../util/get-convert-path")
const getResolvePaths = require("../util/get-resolve-paths")
const getTryExtensions = require("../util/get-try-extensions")
const visitRequire = require("../util/visit-require")

module.exports = {
    meta: {
        docs: {
            description:
                "disallow `require()` expressions which import extraneous modules",
            category: "Possible Errors",
            recommended: true,
            url: "https://github.com/weiran-zsd/eslint-plugin-node/blob/HEAD/docs/rules/no-extraneous-require.md",
        },
        type: "problem",
        fixable: null,
        schema: [
            {
                type: "object",
                properties: {
                    allowModules: getAllowModules.schema,
                    convertPath: getConvertPath.schema,
                    resolvePaths: getResolvePaths.schema,
                    tryExtensions: getTryExtensions.schema,
                },
                additionalProperties: false,
            },
        ],
        messages,
    },
    create(context) {
        const filePath = context.getFilename()
        if (filePath === "<input>") {
            return {}
        }

        return visitRequire(context, {}, targets => {
            checkExtraneous(context, filePath, targets)
        })
    },
}
