#!/usr/bin/env bash
SHELL_FOLDER=$(dirname $(readlink -f "$0"))
QUERY=${SHELL_FOLDER}/target/release/vrlifetime-query

$QUERY "$1"
