<p align="center">
  <img
    src="https://raw.githubusercontent.com/rust-analyzer/rust-analyzer/master/assets/logo-wide.svg"
    alt="rust-analyzer logo">
</p>

rust-analyzer is a modular compiler frontend for the Rust language.
It is a part of a larger rls-2.0 effort to create excellent IDE support for Rust.

## Quick Start

1. Install the VScode for Linux from link: https://code.visualstudio.com/docs/setup/linux
2. Install Rust for Linux by command: 
```
$ curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```
3. Go the extension source code:
```
$ cd rust-analyzer
```
4. Install Nodejs and npm for Linux by command: 
```
$ sudo apt install nodejs
$ curl -L https://npmjs.org/install.sh | sudo sh 
```
5. Install the extension to VScode: 
```
cargo xtask install
```
Now the extension is installed in your VScode, you can open the "Demo" project and test the safe suggestion plugin
