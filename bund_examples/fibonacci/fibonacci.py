"""Naive recursive Fibonacci, the same algorithm as fibonacci.bund."""

import argparse
import sys


def fib(n: int) -> int:
    return n if n < 2 else fib(n - 1) + fib(n - 2)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--n", type=int, default=30)
    n = parser.parse_args().n
    sys.setrecursionlimit(max(1000, n + 100))
    print(fib(n))


if __name__ == "__main__":
    main()
