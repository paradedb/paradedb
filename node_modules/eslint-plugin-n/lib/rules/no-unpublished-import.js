/**
 * @author Toru Nagashima
 * See LICENSE file in root directory for full license.
 */
"use strict"

const { checkPublish, messages } = require("../util/check-publish")
const getAllowModules = require("../util/get-allow-modules")
const getConvertPath = require("../util/get-convert-path")
const getResolvePaths = require("../util/get-resolve-paths")
const visitImport = require("../util/visit-import")

module.exports = {
    meta: {
        docs: {
            description:
                "disallow `import` declarations which import private modules",
            category: "Possible Errors",
            recommended: true,
            url: "https://github.com/weiran-zsd/eslint-plugin-node/blob/HEAD/docs/rules/no-unpublished-import.md",
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
                    ignoreTypeImport: { type: "boolean", default: false },
                },
                additionalProperties: false,
            },
        ],
        messages,
    },
    create(context) {
        const filePath = context.getFilename()
        const options = context.options[0] || {}
        const ignoreTypeImport =
            options.ignoreTypeImport === void 0
                ? false
                : options.ignoreTypeImport

        if (filePath === "<input>") {
            return {}
        }

        return visitImport(context, { ignoreTypeImport }, targets => {
            checkPublish(context, filePath, targets)
        })
    },
}
