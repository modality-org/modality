use modality_utils::shuffle::fisher_yates_shuffle;

fn main() {
    println!("=== Fisher-Yates Shuffle Demo ===\n");

    // Shuffle a small array
    println!("Shuffling 0-9 with seed 12345:");
    let shuffled = fisher_yates_shuffle(12345, 10);
    println!("Result: {:?}\n", shuffled);

    // Same seed produces same result
    println!("Same seed (12345) again:");
    let shuffled2 = fisher_yates_shuffle(12345, 10);
    println!("Result: {:?}", shuffled2);
    println!("Are they equal? {}\n", shuffled == shuffled2);

    // Different seed produces different result
    println!("Different seed (54321):");
    let shuffled3 = fisher_yates_shuffle(54321, 10);
    println!("Result: {:?}", shuffled3);
    println!("Are they different? {}\n", shuffled != shuffled3);

    // Larger array
    println!("Shuffling 0-99 with seed 999:");
    let large_shuffled = fisher_yates_shuffle(999, 100);
    println!("First 20 elements: {:?}", &large_shuffled[..20]);
    println!("Last 20 elements: {:?}\n", &large_shuffled[80..]);

    // Verify it's a valid permutation
    let mut sorted = large_shuffled.clone();
    sorted.sort();
    let expected: Vec<usize> = (0..100).collect();
    println!("Is it a valid permutation? {}", sorted == expected);
}

