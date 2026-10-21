/// Matrix multiplication 400x400 (pure Rust, no external crates)
/// Equivalent to matrix_mult.py and matrix_mult.cf

const SIZE: usize = 400;

/// Create a matrix with optional value function
fn create_matrix<F>(size: usize, value_func: F) -> Vec<Vec<f64>>
where
    F: Fn(usize, usize) -> f64,
{
    (0..size)
        .map(|i| (0..size).map(|j| value_func(i, j)).collect())
        .collect()
}

/// Multiply two matrices A and B
fn multiply_matrices(a: &[Vec<f64>], b: &[Vec<f64>], size: usize) -> Vec<Vec<f64>> {
    // Initialize result matrix with zeros
    let mut c = vec![vec![0.0; size]; size];

    // Standard O(n³) matrix multiplication
    for i in 0..size {
        for j in 0..size {
            let mut sum = 0.0;
            for k in 0..size {
                sum += a[i][k] * b[k][j];
            }
            c[i][j] = sum;
        }
    }

    c
}

/// Print first 5x5 corner of the matrix
fn print_matrix_corner(matrix: &[Vec<f64>], size: usize, prefix: &str) {
    let max_row = 5.min(size);
    let max_col = 5.min(size);

    for i in 0..max_row {
        let row_str: String = (0..max_col)
            .map(|j| format!("{:10.2}", matrix[i][j]))
            .collect::<Vec<_>>()
            .join(" ");
        println!("{}[{}]: {}", prefix, i, row_str);
    }
}

fn main() {
    println!("Matrix multiplication {}x{}", SIZE, SIZE);
    println!("Initializing matrices...");

    // Create matrix A: A[i][j] = i + j + 1
    let a = create_matrix(SIZE, |i, j| (i + j + 1) as f64);

    // Create matrix B: B[i][j] = i - j + 1
    let b = create_matrix(SIZE, |i, j| (i as isize - j as isize + 1) as f64);

    println!("Multiplying matrices...");
    let c = multiply_matrices(&a, &b, SIZE);

    println!("Result (first 5x5 corner):");
    print_matrix_corner(&c, SIZE, "  ");

    println!("\nTotal elements: {}", SIZE * SIZE);
    println!("Done!");
}
