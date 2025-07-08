set -e

echo "Building BPF.."
cargo build-sbf --manifest-path ido-with-vesting/Cargo.toml

echo "Deploying program.."
solana program deploy ./target/deploy/ido_with_vesting.so

echo "Running client example.."
cargo run --manifest-path ido-with-vesting/Cargo.toml --features program-test --example client