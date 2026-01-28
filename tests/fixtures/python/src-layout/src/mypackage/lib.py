"""Library module."""


def helper(x: int) -> int:
    """Helper function."""
    return x * 2


def compute(a: int, b: int) -> int:
    """Compute result."""
    return helper(a) + helper(b)
