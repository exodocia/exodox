#!/usr/bin/env bash

## Here goes function description
## @usage  <string>
##    string - Argument which will be echoed.
main() {
  local string=${1:?}
  echo "$1"
}
