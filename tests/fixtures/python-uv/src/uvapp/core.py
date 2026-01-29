"""Core functionality."""


def process(data: list[str]) -> list[str]:
    """Process a list of strings by uppercasing."""
    return [s.upper() for s in data]
