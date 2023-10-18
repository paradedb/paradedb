#!/bin/bash

# Exit on subcommand errors
set -Eeuo pipefail

while read -r file
do
  if [[ "$(head -c 11 "$file")" = "#!/bin/bash" ]]
  then
    echo "[#!/bin/bash -> Present] $file"
  elif [[ "$(head -c 11 "$file")" = "#!/bin/sh" ]]
  then
    echo "[#!/bin/sh -> Present] $file"
  else
    echo "[#!/bin/bash -> NOT FOUND] $file" && exit 1
  fi
done < <(find . -name '*.sh')
