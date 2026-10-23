"""Analysis scripts of the petrivet evaluation (SPEC sections 11 and 12).

One command regenerates everything under ``export/`` from the result files::

    python -m analysis --results results --truth results/truth.jsonl --out export

Each experiment is one module (``e1``, ``e1b``, ``e2``, ``e3a``, ``e3b``, ``e4``, ``e5a``, ``e5b``, ``e6``, ``e7``)
with a ``run(ctx)`` function; a module with missing input records a note under ``skipped.<id>`` in ``numbers.json``.
"""

__version__ = "1"
