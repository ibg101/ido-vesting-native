set -e

echo "Building BPF.."
cargo build-sbf --manifest-path Cargo.toml

echo "Deploying program.."
solana program deploy ./target/deploy/ido_with_vesting.so

echo "Running client example.."
cargo run --manifest-path Cargo.toml --features program-test --example client