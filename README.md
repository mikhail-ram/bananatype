# Bananatype

![usage.gif](https://imgur.com/a/9JYtFBw)

Bananatype is a terminal-based typing test, inspired by [monkeytype](https://monkeytype.com) and built in Rust 🦀.

## Installation

Installing the project requires ```cargo``` to be installed on your system. If cargo is not installed see [here](https://doc.rust-lang.org/cargo/getting-started/installation.html).

To install and try out bananatype, clone this repository locally and run it with Cargo.

```bash
git clone https://github.com/mikhail-ram/bananatype
cd bananatype
cargo run
```

### Adding to PATH

Adding the project to your PATH requires building the project using the release profile.

```bash
cargo build --release
```

To add to the PATH modify your ```.zshrc```:

```bash
export PATH="/path/to/bananatype/target/release:$PATH" # For ZShell users

```

## Usage

To run the typing test, run ```bananatype``` from within your terminal.

## Contributing

Pull requests are welcome. For major changes, please open an issue first
to discuss what you would like to change.

Please make sure to update tests as appropriate.
