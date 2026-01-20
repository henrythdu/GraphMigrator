import os
from sys import exit

def my_func():
    os.path.exists("file")  # Unresolved: "os.path.exists" not in graph
    exit(0)  # Unresolved: "exit" not defined in this file
    helper()

def helper():
    pass
