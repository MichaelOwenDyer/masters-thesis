import json
from pathlib import Path

import pytest

from analysis import fake
from analysis.__main__ import main


@pytest.fixture(scope="session")
def fake_dir(tmp_path_factory) -> Path:
    d = tmp_path_factory.mktemp("fake")
    fake.generate(d, seed=3)
    return d


def run_pipeline(d: Path, out: Path, *extra: str) -> int:
    return main(["--results", str(d / "results"), "--truth", str(d / "results" / "truth.jsonl"), "--out", str(out),
                 "--cache-coherence", str(d / "cache_coherence.json"), "--generated-at", "T", *extra])


@pytest.fixture(scope="session")
def export(fake_dir, tmp_path_factory) -> Path:
    out = tmp_path_factory.mktemp("export")
    assert run_pipeline(fake_dir, out) == 0
    return out


@pytest.fixture(scope="session")
def numbers(export) -> dict:
    return json.loads((export / "numbers.json").read_text())
