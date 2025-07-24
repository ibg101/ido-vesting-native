## Project Features:
- Contains all features mentioned in [ido-vesting-monorepo](https://github.com/ibg101/ido-vesting-monorepo).
- Uses my [`solana-mint-fixture`](https://github.com/ibg101/solana-mint-fixture) crate to simplify setup of `SPL Token 2022` and `Associated Token Accounts` in tests.
- Implements 2 comprehensive **e2e tests** against:
  - RpcClient
  - BanksClient

---

## Program Features:
- `program-test` - **not enabled by default**  
  - Enables both the `instruction` and `ergonomic-init` features for testing purposes.

- `instruction` - **not enabled by default**  
  - Adds ergonomic instruction builders under `ido_with_vesting::instruction`.

- `ergonomic-init` - **not enabled by default**  
  - Provides ergonomic builder methods for `LinearVestingStrategy`.

---

## Testing
> **Prerequisites**:  
> Make sure you have **Rust** and the **Solana CLI** installed.

### 1. Run against `BanksClient`:
> **fast**, in-memory deployment & tests core functionality
```bash
bash run_test.sh
```

### 2. Run against `RpcClient`:
> **slower**, deployment to the specified cluster & tests more edge cases
  - It's recommended to run test-validator in separate terminal.
  ```bash
  solana-test-validator --reset
  ```

  - Execute the testing script.
  ```bash
  bash run_example.sh
  ```
