import { exec } from "child_process"
import * as path from "path"

const directoryPath = path.join(__dirname, "../../../clients/typescript")

exec("npm install && npm run build", { cwd: directoryPath }, error => {
  if (error) {
    console.error(`An error occurred: ${error}`)
    return
  }
})
