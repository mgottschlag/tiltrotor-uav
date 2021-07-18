
export TEXINPUTS := .:$(PWD)/template/:

all: build/pre-design.pdf

clean:
	rm -rf build pre-design.pdf

build/pre-design.pdf: run_latexmk.sh pre-design.tex bibliography.bib
	@bash run_latexmk.sh

FORCE: ;

