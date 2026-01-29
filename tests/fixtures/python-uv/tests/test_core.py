"""Tests for core module."""
from uvapp.core import process


def test_process():
    assert process(["hello", "world"]) == ["HELLO", "WORLD"]
