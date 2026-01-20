def outer_function():
    """An outer function."""

    def inner_function():
        """A nested function - should NOT be extracted."""
        pass

    return inner_function


class OuterClass:
    """An outer class."""

    class InnerClass:
        """A nested class - should NOT be extracted."""
        pass
