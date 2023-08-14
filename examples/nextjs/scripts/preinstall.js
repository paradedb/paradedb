/* eslint-disable @typescript-eslint/no-var-requires */
const { exec } = require("child_process")
const path = require("path")

const directoryPath = path.join(__dirname, "../../../clients/typescript")

exec("npm install && npm run build", { cwd: directoryPath }, error => {
  if (error) {
    console.error(`An error occurred: ${error}`)
    return
  }
})
