# Lan Chat ![crates.io](https://img.shields.io/crates/v/lan-chat.svg) ![Build Passing](https://github.com/Saphereye/lan-chat/actions/workflows/rust.yml/badge.svg)

Lan Chat is a terminal-based chat application featuring a user-friendly terminal interface.

## Installation

To install Lan Chat, use the following command:

```bash
cargo install lan-chat
```

This will install the binary. For usage instructions, refer to the [Usage](#usage) section.

Alternatively, you can clone the repository and build it manually using the `cargo build` command. Please note that in this case, you will need to run the executable directly.

## Usage

1. To learn about the available commands, run:

```bash
lan-chat --help
```

2. To start the server, run:

```bash
lan-chat -i
```

The output will display the server IP.

3. To connect to the server, use:

```bash
lan-chat -s <server-ip>
```

You will be prompted to enter a pseudonym. Alternatively, you can set the pseudonym directly using the following command:

```bash
lan-chat -s <server-ip> -p <pseudonym>
```

## Running Example

![image](https://github.com/Saphereye/lan-chat/assets/59739923/daada2ff-4cf0-4251-8e91-92b867f1c2bc)