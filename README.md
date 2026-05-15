# Boltz Swap Rescue Tool

A graphical tool to rescue Boltz swaps when local state is lost (lost phone, deleted app, etc.). It rebuilds the rescue transaction locally and lets you broadcast it.

## Supported flows

| Swap type | Rescue type | What happens |
|-----------|-------------|--------------|
| `chain`   | `claim`     | Sweep the counterparty's on-chain lockup on `to_network` (you need the preimage). |
| `chain`   | `refund`    | Reclaim your on-chain lockup on `from_network` after the timelock. |
| `submarine` | `refund`  | Reclaim your on-chain lockup on `from_network` after the LN side failed. |
| `reverse` | `claim`     | Sweep the server's on-chain HTLC on `from_network` (you need the preimage). |

## Installation

### Option 1 — Prebuilt binary (recommended)

Grab the latest release for your platform from the [Releases page](https://github.com/SatoshiPortal/swap_rescue/releases):

- **Linux (x86_64)**: `swap_rescue-linux-x86_64.tar.gz`
- **macOS (Intel)**: `swap_rescue-macos-x86_64.tar.gz`
- **macOS (Apple Silicon)**: `swap_rescue-macos-arm64.tar.gz`
- **Windows (x86_64)**: `swap_rescue-windows-x86_64.zip`

**Linux/macOS:**
```bash
tar -xzf swap_rescue-*.tar.gz
chmod +x swap_rescue
# Optional:
sudo mv swap_rescue /usr/local/bin/
```

**Windows:** extract the zip and double-click `swap_rescue.exe` (or run it from PowerShell).

### Option 2 — Build from source

Install Rust + Cargo:

**macOS / Linux:**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

**Windows:** download the installer from [rustup.rs](https://rustup.rs/). If you hit a `link.exe not found` error, install the "Desktop development with C++" workload from the [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022).

Then build:
```bash
cargo build --release
./target/release/swap_rescue
```

## Usage

The tool is GUI-only — there are no CLI flags. Run the binary (or `cargo run`) and you'll get a window.

### 1. Get a provider config

Ask your swap provider (BullBitcoin / Aqua / etc.) for the `provider_input` YAML for your swap ID. See [`example.config.yaml`](./example.config.yaml) for the fully-commented schema. The file contains **only** provider data — your mnemonic, passphrase, swap index, and return address are entered in the GUI and never written to disk.

A minimal config:

```yaml
provider_input:
  rescue_type: "refund"      # claim or refund
  swap_type: "submarine"     # chain, submarine, or reverse
  claim_leaf:
    output: "..."
    version: 196
  refund_leaf:
    output: "..."
    version: 196
  from_network: "bitcoin"    # bitcoin/btc or liquid/lbtc
  to_network: ""             # unused for submarine refund + reverse claim
  blinding_key: "..."        # required for Liquid; null otherwise
  timeout_block_height: 0
  server_public_key: "..."
  user_lockup_address: "..."
  server_lockup_address: ""  # only required for chain claim
  amount: 0
  swap_id: "..."
  preimage: ""               # required for chain claim + reverse claim
```

### 2. In the GUI

1. **Load Provider Config** — pick the YAML file.
2. **Enter your mnemonic** in the word grid. Each box autocompletes against the BIP39 list; once a word matches it auto-locks (green) and focus moves to the next box. Click a locked word to re-edit. Pasting all 12/24 words into the first box distributes them across the grid. A **Hide / Show** toggle masks the words; helper text confirms the mnemonic is not saved locally. Once all words are entered the whole section collapses to a single banner — click **Edit** to reopen.
3. **Passphrase** — optional BIP39 passphrase (masked).
4. **Return address** — where the rescued funds go. The label shows the expected network (e.g. `Bitcoin (BTC)` or `Liquid (LBTC)`), and the field is validated live:
   - Chain claim → must be a `to_network` address.
   - Everything else (chain refund, submarine refund, reverse claim) → must be a `from_network` address.
5. **Swap index** — Auto-Increment Index is **on** by default: enter a max (default 210) and the tool will keep retrying with `index + 1` until the rescue succeeds. Toggle it off to set a fixed index manually.
6. **Execute Rescue** — disabled until the mnemonic is fully entered (all words valid BIP39) and the return address parses for the expected network. The blocker reason is shown beneath the button.
7. **Review** the transaction summary in the logs panel.
8. **Confirm & Broadcast** to send the transaction.

You can copy or save the log panel contents at any time — useful when you need to share output with support.

## Schema reference

See [`example.config.yaml`](./example.config.yaml) for the fully-commented schema. Copy it to `config.yaml` (or any path) and replace the values with what your provider gives you.

## Security notes

- The tool never writes your mnemonic, passphrase, or derived keys to disk.
- It connects to public Boltz and Electrum servers to fetch swap state and broadcast transactions.
- Always review the transaction summary before confirming the broadcast.
