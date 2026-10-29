.PHONY: all watch nets

all: watch

watch:
	@mkdir -p target
	typst watch main.typ target/thesis.pdf


# Petri net figures: the family overview from the corpus, the examples of the background section and of the evaluation collections, and every figures/nets/*.pnml drawn as figures/nets/*.svg
nets:
	python3 tools/make_family_figure.py --cols 2
	python3 tools/make_background_figures.py
	python3 tools/make_collection_figures.py
	for f in figures/nets/*.pnml; do [ -e "$$f" ] && python3 tools/petrinet_svg.py "$$f" -o "$${f%.pnml}.svg"; done; true
