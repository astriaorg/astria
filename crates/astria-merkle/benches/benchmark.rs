use astria_merkle::Tree;
use ct_merkle::CtMerkleTree;
use divan::{
    black_box,
    black_box_drop,
    Bencher,
};
use sha2::Sha256;

// allow: unused warning if `bench_include_allocs` feature is not enabled.
#[allow(dead_code)]
#[cfg_attr(feature = "bench_include_allocs", global_allocator)]
static ALLOC: divan::AllocProfiler = divan::AllocProfiler::system();

/// Used to specify the size of data for leaves.
#[derive(Copy, Clone)]
enum InputSizes {
    /// All leaves will have the specified number of bytes.
    Fixed(usize),
    /// Leaves will have a variety of sizes ranging from 1 byte to 1 MB.
    Mixed,
}

macro_rules! benchmark_mods {
    ($([$leaf_module:ident $leaf_count:literal])+ ===== $([$data_module:ident $input_sizes:expr])+) => {
        macro_rules! inner_mods {
            ($lf_count:literal) => {
                $(
                    mod $data_module {
                        #[divan::bench]
                        fn construct_tree_astria(bencher: divan::Bencher) {
                            crate::construct_tree_astria(bencher, $lf_count, $input_sizes);
                        }

                        #[divan::bench]
                        fn construct_tree_ct_merkle(bencher: divan::Bencher) {
                            crate::construct_tree_ct_merkle(bencher, $lf_count, $input_sizes);
                        }

                        #[divan::bench]
                        fn construct_proof_astria(bencher: divan::Bencher) {
                            crate::construct_proof_astria(bencher, $lf_count, $input_sizes);
                        }

                        #[divan::bench]
                        fn construct_proof_ct_merkle(bencher: divan::Bencher) {
                            crate::construct_proof_ct_merkle(bencher, $lf_count, $input_sizes);
                        }

                        #[divan::bench]
                        fn verify_leaf_astria(bencher: divan::Bencher) {
                            crate::verify_leaf_astria(bencher, $lf_count, $input_sizes);
                        }

                        #[divan::bench]
                        fn verify_leaf_ct_merkle(bencher: divan::Bencher) {
                            crate::verify_leaf_ct_merkle(bencher, $lf_count, $input_sizes);
                        }
                    }
                )+
            }
        }

        $(
            mod $leaf_module {
                inner_mods! { $leaf_count }
            }
        )+
    };
}

benchmark_mods! {
    [one_leaf 1]
    [five_leaves 5]
    [twenty_leaves 20]
    [one_hundred_leaves 100]
    =====
    [empty_data crate::InputSizes::Fixed(0)]
    [ten_bytes crate::InputSizes::Fixed(10)]
    [one_kb crate::InputSizes::Fixed(1_000)]
    [one_hundred_kb crate::InputSizes::Fixed(100_000)]
    [one_mb crate::InputSizes::Fixed(1_000_000)]
    [mixed_sizes crate::InputSizes::Mixed]
}

/// Benchmark construction of a new `astria_merkle::Tree`.
fn construct_tree_astria(bencher: Bencher, leaf_count: usize, input_sizes: InputSizes) {
    bencher
        .with_inputs(|| raw_leaves(leaf_count, input_sizes))
        .bench_local_refs(|raw_leaves| black_box(Tree::from_leaves(black_box(raw_leaves).iter())));
}

/// Benchmark construction of a new `CtMerkleTree`.
fn construct_tree_ct_merkle(bencher: Bencher, leaf_count: usize, input_sizes: InputSizes) {
    bencher
        .with_inputs(|| raw_leaves(leaf_count, input_sizes))
        .bench_local_refs(|raw_leaves| {
            let mut tree = CtMerkleTree::<Sha256, Vec<u8>>::new();
            black_box(raw_leaves)
                .drain(..)
                .for_each(|value| tree.push(value));
            black_box_drop(tree);
        });
}

/// Benchmark construction of a new inclusion proof using an `astria_merkle::Tree`.
fn construct_proof_astria(bencher: Bencher, leaf_count: usize, input_sizes: InputSizes) {
    bencher
        .with_inputs(|| Tree::from_leaves(raw_leaves(leaf_count, input_sizes)))
        .bench_local_refs(|tree| {
            for i in 0..leaf_count {
                black_box(tree.construct_proof(i).unwrap());
            }
        });
}

/// Benchmark construction of a new inclusion proof using a `CtMerkleTree`.
fn construct_proof_ct_merkle(bencher: Bencher, leaf_count: usize, input_sizes: InputSizes) {
    bencher
        .with_inputs(|| {
            let mut tree = CtMerkleTree::<Sha256, Vec<u8>>::new();
            raw_leaves(leaf_count, input_sizes)
                .drain(..)
                .for_each(|value| tree.push(value));
            tree
        })
        .bench_local_refs(|tree| {
            for i in 0..leaf_count {
                black_box(tree.prove_inclusion(i));
            }
        });
}

/// Benchmark verification of inclusion using an `astria_merkle::Tree`.
fn verify_leaf_astria(bencher: Bencher, leaf_count: usize, input_sizes: InputSizes) {
    bencher
        .with_inputs(|| {
            let raw_leaves = raw_leaves(leaf_count, input_sizes);
            let tree = Tree::from_leaves(raw_leaves.iter());
            let root = tree.root();
            let leaves_and_proofs =
                raw_leaves
                    .into_iter()
                    .enumerate()
                    .map(move |(index, raw_leaf)| {
                        let proof = tree.construct_proof(index).unwrap();
                        (raw_leaf, proof)
                    });
            (leaves_and_proofs, root)
        })
        .bench_local_values(|(leaves_and_proofs, root)| {
            for (raw_leaf, proof) in leaves_and_proofs {
                assert!(proof.verify(&raw_leaf, root));
            }
        });
}

/// Benchmark verification of inclusion using a `CtMerkleTree`.
fn verify_leaf_ct_merkle(bencher: Bencher, leaf_count: usize, input_sizes: InputSizes) {
    bencher
        .with_inputs(|| {
            let raw_leaves = raw_leaves(leaf_count, input_sizes);
            let mut tree = CtMerkleTree::<Sha256, Vec<u8>>::new();
            raw_leaves.iter().for_each(|value| tree.push(value.clone()));
            let root = tree.root();
            let leaves_and_proofs =
                raw_leaves
                    .into_iter()
                    .enumerate()
                    .map(move |(index, raw_leaf)| {
                        let proof = tree.prove_inclusion(index);
                        (raw_leaf, proof)
                    });
            (leaves_and_proofs, root)
        })
        .bench_local_values(|(leaves_and_proofs, root)| {
            for (index, (raw_leaf, proof)) in leaves_and_proofs.enumerate() {
                assert!(root.verify_inclusion(&raw_leaf, index, &proof).is_ok());
            }
        });
}

/// Returns `leaf_count` `Vec<u8>`s, each with lengths specified by `input_sizes`.
fn raw_leaves(leaf_count: usize, input_sizes: InputSizes) -> Vec<Vec<u8>> {
    const MIXED_SIZES: [usize; 7] = [1_000, 1_000_000, 10, 100_000, 1, 10_000, 100];
    match input_sizes {
        InputSizes::Fixed(size) => vec![vec![1; size]; leaf_count],
        InputSizes::Mixed => MIXED_SIZES
            .iter()
            .map(|size| vec![1; *size])
            .cycle()
            .take(leaf_count)
            .collect(),
    }
}

fn main() {
    // Handle `nextest` querying the benchmark binary for tests.  Currently `divan` is incompatible
    // with `nextest`, so just report no tests available.
    // See https://github.com/nvzqz/divan/issues/43 for further details.
    let args: Vec<_> = std::env::args().collect();
    if args.contains(&"--list".to_string())
        && args.contains(&"--format".to_string())
        && args.contains(&"terse".to_string())
    {
        return;
    }
    // Run registered benchmarks.
    divan::main();
}
