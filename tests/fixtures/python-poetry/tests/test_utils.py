"""Tests for utils module."""
from poetryapp.utils import greet


def test_greet():
    assert greet("World") == "Hello, World!"
