#!/usr/bin/env bash

SCRIPT_DIR=$(dirname "${BASH_SOURCE[0]}") 

cd ${SCRIPT_DIR} && antlr -Dlanguage=JavaScript ../../../../../grammars/antlr4/Modality.g4 -o ./build -Xexact-output-dir -visitor 