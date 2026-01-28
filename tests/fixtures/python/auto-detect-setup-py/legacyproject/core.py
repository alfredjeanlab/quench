"""Core module."""


def process(data: str) -> str:
    """Process data."""
    return data.upper()


def validate(value: int) -> bool:
    """Validate value."""
    return value > 0
