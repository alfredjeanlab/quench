"""Tests for math module."""

import pytest

from myproject.math import add, subtract, multiply, divide


def test_add():
    """Test add function."""
    assert add(2, 3) == 5
    assert add(-1, 1) == 0
    assert add(0, 0) == 0


def test_subtract():
    """Test subtract function."""
    assert subtract(5, 3) == 2
    assert subtract(1, 1) == 0


def test_multiply():
    """Test multiply function."""
    assert multiply(3, 4) == 12
    assert multiply(0, 5) == 0


def test_divide():
    """Test divide function."""
    assert divide(10, 2) == 5.0
    assert divide(7, 2) == 3.5


def test_divide_by_zero():
    """Test divide raises error for zero divisor."""
    with pytest.raises(ValueError, match="Cannot divide by zero"):
        divide(10, 0)
