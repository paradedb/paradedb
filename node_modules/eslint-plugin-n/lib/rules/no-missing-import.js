/**
 * @author Toru Nagashima
 * See LICENSE file in root directory for full license.
 */
"use strict"

const { checkExistence, messages } = require("../util/check-existence")
const getAllowModules = require("../util/get-allow-modules")
const getResolvePaths = require("../util/get-resolve-paths")
const visitImport = require("../util/visit-import")

module.exports = {
    meta: {
        docs: {
            description:
                "disallow `import` declarations which import non-existence modules",
            category: "Possible Errors",
            recommended: true,
            url: "https://github.com/weiran-zsd/eslint-plugin-node/blob/HEAD/docs/rules/no-missing-import.md",
        },
        type: "problem",
        fixable: null,
        schema: [
            {
                type: "object",
                properties: {
                    allowModules: getAllowModules.schema,
                    resolvePaths: getResolvePaths.schema,
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

        return visitImport(context, {}, targets => {
            checkExistence(context, targets)
        })
    },
}
