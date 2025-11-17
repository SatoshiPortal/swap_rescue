# Boltz Swap Rescue Tool

A temporary tool to rescue chain swaps for which script metadata is lost (i.e. you lost your phone or deleted the app before a swap completed)


## Prerequisites

### Installing Cargo

Cargo is the Rust package manager and comes bundled with Rust. To install Rust and Cargo:

**On macOS and Linux:**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

After installation, restart your terminal or run:
```bash
source $HOME/.cargo/env
```

**On Windows:**
Download and run the installer from [https://rustup.rs/](https://rustup.rs/)

**Verify installation:**
```bash
cargo --version
```

## Usage

- Ask the wallet providing the swap service: BullBitcoin/Aqua; for the config params using your swap ID
- You will be provided a config for a specifc swap ID with all the `provider_input` values filled
- You must fill all the user input values yourself

```
user_input:
  mnemonic: "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
  passphrase: ""
  swap_index: 0
  return_address: "lq1qqvpfp28manyd036nrvzs5eeg2y48f48tlf27n68u65esxsgjd3sc3u673rp6dvp0y5mdgxp93aj4w67uu72ajgqyfvvj59h3h"
```

- Use the mnemonic of the wallet you were sending from
- If the swap is from BTC to LBTC and the BTC wallet used has a passphrase, add the passphrase to the config as well; else leave it empty
- If you are unsure what to use as your `swap_index`; you can know this by estimating how many swaps you have done; if this was your first chain swap, use 0, if it was your second, use 1 and so on. If you are not sure you can try values until one works
- The return address is where funds will be sent

Once your config.yaml is set, note down its path. Its best to place it in the same location as this repo.

Then run the following command:

```
cargo run -- --config config.yaml

```

OR if its in a different path as this repo:

```
cargo run -- --config ./config.yaml
```
