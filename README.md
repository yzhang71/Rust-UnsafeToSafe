<p align="center">
  <img
    src="https://raw.githubusercontent.com/rust-analyzer/rust-analyzer/master/assets/logo-wide.svg"
    alt="rust-analyzer logo">
</p>

rust-analyzer is a modular compiler frontend for the Rust language.
It is a part of a larger rls-2.0 effort to create excellent IDE support for Rust.

## Quick Start

1. Install the VScode for Mac OS from link: https://go.microsoft.com/fwlink/?LinkID=534106
2. Install Rust for Mac OS by command: 
```
curl https://sh.rustup.rs -sSf | sh
```
3. Get the extension source code from Github:
```
git clone https://github.com/yzhang71/rust-analyzer-UnsafeToSafe.git && cd rust-analyzer-UnsafeToSafe
```
4. Install Nodejs and nmp for Mac OS by command: 
```
brew update && brew install node
```
5. Install the extension to VScode: 
```
cargo xtask install
```
Now the extension is installed in your VScode, you can open any Rust project and test the UnsafeToSafe extension
Source code link: https://github.com/yzhang71/rust-analyzer-UnsafeToSafe/blob/master/crates/ide-assists/src/handlers/convert_unsafe_to_safe.rs
