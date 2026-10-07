.PHONY: all watch

all: watch

watch:
	@mkdir -p target
	typst watch main.typ target/thesis.pdf

