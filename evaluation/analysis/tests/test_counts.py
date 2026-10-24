from analysis.common import classify


def row(**kw):
    base = {"verdict": "COUNT", "states": 10, "edges": 20, "truth_kind": "count", "truth_states": 10, "truth_edges": 20, "truth_bool": None}
    base.update(kw)
    return base


def test_counts_with_both_numbers():
    assert classify(row(), False) == "AGREE"
    assert classify(row(states=11), False) == "DISAGREE"
    assert classify(row(edges=21), False) == "DISAGREE"


def test_a_tool_that_reports_states_only_is_compared_on_states():
    assert classify(row(edges=None), False) == "AGREE"
    assert classify(row(edges=None, states=11), False) == "DISAGREE"


def test_count_without_states_is_a_runner_error():
    assert classify(row(states=None), False) == "RUNNER_ERROR"
