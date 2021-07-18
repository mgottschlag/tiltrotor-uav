#!/bin/bash

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

# Create the required directories.
mkdir -p build

# Compile the LaTeX sources.
set +e
latexmk -pdflatex=pdflatex -pdf -outdir=build pre-design.tex > /dev/null 2>&1 < /dev/null
success=$?
set -e
if [ "$success" == 0 ]
then
	echo -e "Compiling ${GREEN}SUCCESSFUL${NC}."
	cp build/pre-design.pdf pre-design.pdf
else
	echo -e "Compiling ${RED}FAILED${NC}. See the log file for more information"
	rm -f pre-design.pdf build/pre-design.pdf
fi

# Print warnings.
set +e
GREP_COLOR='0;31' grep --color -e "^! " build/*.log
GREP_COLOR='0;31' grep -A 1 --color "Undefined control sequence" build/*.log
GREP_COLOR='0;31' grep -A 1 --color "Runaway argument?" build/*.log
GREP_COLOR='0;31' grep --color "Missing input file" build/*.log
GREP_COLOR='0;31' grep --color Error build/*.log
GREP_COLOR='0;33' grep --color Warning build/*.log
GREP_COLOR='0;33' grep --color Overfull build/*.log
GREP_COLOR='0;33' grep --color Underfull build/*.log
set -e
