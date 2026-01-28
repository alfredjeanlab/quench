"""This should be ignored by the venv ignore pattern."""
# This file exists to verify that .venv/ directories are properly ignored
# It should NOT be counted in source lines


def ignored_function():
    """This function should not be counted."""
    return "ignored"


def another_ignored_function():
    """Another function that should not be counted."""
    return "also ignored"
