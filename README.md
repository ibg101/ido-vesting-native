## Features:
- Contains all features mentioned in [ido-vesting-monorepo](https://github.com/ibg101/ido-vesting-monorepo).
- Adds `mint-fixture` library crate for ergonomic initialization of required `SPL Token 2022` and `SPL Associated Token Account` accounts.
- Implements 2 comprehensive **e2e Tests** against:
  - RpcClient
  - BanksClient

## Testing
Make sure you have Rust & Solana CLI installed!

1. To test against `BanksClient` (**fast**, in-memory deployment & tests core functionallity):
```bash
bash run_test.sh
```

2. To test against `RpcClient` (**slower**, deployment to the specified cluster & tests more edge cases):
  - It's recommended to run test-validator in separate terminal.
  ```bash
  solana-test-validator --reset
  ```

  - Execute the testing script.
  ```bash
  bash run_example.sh
  ```
