# Duplicate function names (should not create false edges within same file)
def helper():
    pass

def caller():
    helper()  # Resolves to first helper

def helper():  # Redeclaration (Python allows this)
    pass

# Dotted call (unresolved - dotted name not in graph)
def dotted_caller():
    import os
    os.path.exists("file")  # Won't resolve "os.path.exists"

# Method call (unresolved - methods not extracted)
def method_caller():
    obj = object()
    obj.method()  # Won't resolve "obj.method"
