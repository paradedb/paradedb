#!/bin/bash

# Exit on subcommand errors
set -Eeuo pipefail

while read -r file
do
  if [[ "$(head -c 11 "$file")" = "#!/bin/bash" ]]
  then
    echo "[#!/bin/bash -> Present] $file"
  else
    echo "[#!/bin/bash -> NOT FOUND] $file" && exit 1
  fi
done < <(find . -name '*.sh')
