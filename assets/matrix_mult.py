#!/usr/bin/env python3
"""
Matrix multiplication 400x400 (pure Python, no numpy)
"""

SIZE = 400


def create_matrix(size, value_func=None):
    """Create a matrix with optional value function"""
    if value_func is None:
        value_func = lambda i, j: 1.0
    return [[value_func(i, j) for j in range(size)] for i in range(size)]


def multiply_matrices(A, B, size):
    """Multiply two matrices A and B"""
    # Initialize result matrix with zeros
    C = [[0.0] * size for _ in range(size)]
    
    # Standard O(n³) matrix multiplication
    for i in range(size):
        for j in range(size):
            sum_val = 0.0
            for k in range(size):
                sum_val += A[i][k] * B[k][j]
            C[i][j] = sum_val
    
    return C


def main():
    print(f"Matrix multiplication {SIZE}x{SIZE}")
    print("Initializing matrices...")
    
    # Create matrix A: A[i][j] = i + j + 1
    A = create_matrix(SIZE, lambda i, j: float(i + j + 1))
    
    # Create matrix B: B[i][j] = i - j + 1
    B = create_matrix(SIZE, lambda i, j: float(i - j + 1))
    
    print("Multiplying matrices...")
    C = multiply_matrices(A, B, SIZE)
    
    print("Result (first 5x5 corner):")
    for i in range(min(5, SIZE)):
        row_str = " ".join(f"{C[i][j]:10.2f}" for j in range(min(5, SIZE)))
        print(f"  [{i}]: {row_str}")
    
    print(f"\nTotal elements: {SIZE * SIZE}")
    print(f"Done!")


if __name__ == "__main__":
    main()
