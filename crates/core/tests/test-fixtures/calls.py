def helper():
    pass

def caller():
    helper()  # Should create: caller → helper

def another_caller():
    helper()  # Should create: another_caller → helper

def isolated():
    pass  # No calls, no edges from this node
