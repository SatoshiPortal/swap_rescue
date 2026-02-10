# Boltz Swap Rescue Tool

A temporary tool to rescue chain swaps for which script metadata is lost (i.e. you lost your phone or deleted the app before a swap completed)

## Installation

### Option 1: Download Pre-built Binary (Recommended)

Download the latest release for your platform from the [Releases page](https://github.com/YOUR_USERNAME/swap_rescue/releases):

- **Linux (x86_64)**: `swap_rescue-linux-x86_64.tar.gz`
- **macOS (Intel)**: `swap_rescue-macos-x86_64.tar.gz`
- **macOS (Apple Silicon)**: `swap_rescue-macos-arm64.tar.gz`
- **Windows (x86_64)**: `swap_rescue-windows-x86_64.zip`

**Linux/macOS:**
```bash
# Extract the archive
tar -xzf swap_rescue-*.tar.gz

# Make it executable (if needed)
chmod +x swap_rescue

# Move to a directory in your PATH (optional)
sudo mv swap_rescue /usr/local/bin/
```

**Windows:**
1. Extract the zip file
2. Run `swap_rescue.exe` from Command Prompt or PowerShell

### Option 2: Build from Source

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

**Windows Linker Setup:**

If you encounter a `link.exe not found` or `dlltool.exe not found` error, you have two options:

**Option 1: Use GNU Toolchain with MinGW-w64**

1. Install MSYS2 (which includes MinGW-w64):
   - Download from [https://www.msys2.org/](https://www.msys2.org/)
   - Run the installer and follow the setup instructions
   - After installation, open MSYS2 terminal and run:
     ```bash
     pacman -Syu
     pacman -S mingw-w64-x86_64-toolchain
     ```

2. Add MinGW-w64 to your PATH:
   - Add `C:\msys64\mingw64\bin` to your system PATH environment variable
   - Or if using MSYS2, add to your `~/.cargo/config`:
     ```toml
     [target.x86_64-pc-windows-gnu]
     linker = "gcc"
     ```

3. Install and set the GNU toolchain:
   ```bash
   rustup toolchain install stable-x86_64-pc-windows-gnu
   rustup default stable-x86_64-pc-windows-gnu
   ```

**Option 2: Install Visual Studio Build Tools (Easier - Recommended)**

**Step-by-step installation:**

1. **Download the installer:**
   - Go to: https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022
   - Click "Download" under "Build Tools for Visual Studio 2022"
   - This will download `vs_buildtools.exe` (about 1-2 MB)

2. **Run the installer:**
   - Double-click `vs_buildtools.exe`
   - If prompted by User Account Control, click "Yes"

3. **Select the workload:**
   - In the installer window, you'll see "Workloads" tab
   - Check the box for **"Desktop development with C++"**
   - This will automatically select the necessary components (C++ build tools, Windows SDK, etc.)
   - Click "Install" button (bottom right)

4. **Wait for installation:**
   - The installer will download and install the components (this may take 10-30 minutes depending on your internet speed)
   - You'll see a progress bar showing what's being installed
   - When complete, click "Close"

5. **Restart your terminal:**
   - Close any open terminal/command prompt windows
   - Open a new terminal window

6. **Verify Rust is using MSVC toolchain:**
   ```bash
   rustup default stable-x86_64-pc-windows-msvc
   ```

7. **Try building:**
   ```bash
   cargo build
   ```

**Verify installation:**
```bash
cargo --version
```

## Usage

### GUI Mode (Recommended)

The tool includes a modern graphical interface that makes it easier to rescue your swaps.

**Launch the GUI:**

```bash
# Using pre-built binary
./swap_rescue --gui

# Or with cargo
cargo run -- --gui

# Or simply run without arguments to launch GUI
./swap_rescue
cargo run
```

**Using the GUI:**

1. **Get your config file**: Ask the wallet provider (BullBitcoin/Aqua) for the config params using your swap ID
2. **Prepare your config**: Fill in your user information (mnemonic, passphrase, swap_index, return_address)
3. **Load the config**: Click "Load Config" and select your `config.yaml` file
4. **Adjust swap index** (if needed): Use the +/- buttons or type directly to try different swap index values without editing the config file
5. **Execute**: Click "Execute Rescue" to construct the transaction
6. **Review**: The transaction summary will be displayed in the logs
7. **Confirm & Broadcast**: Review the details and click "Confirm & Broadcast" to complete the rescue

**Tip**: If the rescue fails, try incrementing the swap index and executing again. This is common when you're unsure how many swaps you've done previously.

You can take screenshots of the GUI logs to share with support if you need help debugging issues.

### CLI Mode

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

Once your config.yaml is set, note down its path. Its best to place it in the same directory as the binary/repo.

### If using the pre-built binary:

```bash
./swap_rescue --config config.yaml
```

On Windows:
```cmd
swap_rescue.exe --config config.yaml
```

### If building from source:

```bash
cargo run -- --config config.yaml
```

Or if the config is in a different path:

```bash
./swap_rescue --config /path/to/config.yaml
# OR with cargo
cargo run -- --config /path/to/config.yaml
```
