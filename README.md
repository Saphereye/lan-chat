# Lan Chat ![crates.io](https://img.shields.io/crates/v/lan-chat.svg) ![Build Passing](https://github.com/Saphereye/lan-chat/actions/workflows/rust.yml/badge.svg)

![Running example](https://github.com/Saphereye/lan-chat/blob/main/assets/example.png)

Lan Chat is a terminal-based chat application featuring a user-friendly terminal interface.

## Installation

> This requires `cargo` to be installed on your target system. Refer to the [cargo installation guide](https://doc.rust-lang.org/cargo/getting-started/installation.html) if `cargo` is absent on your system.

To install Lan Chat, use the following command:

```bash
cargo install lan-chat --locked
```

This will install the binary. For usage instructions, refer to the [Usage](#usage) section.

Alternatively, the project can be cloned and built using `cargo`.

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

4. To insert emojis in the chat, use the following format: `:<emoji name>:`. For example is you type `That's funny :laughing:` it will be rendered as `That's funny 😂`.

The supported emojis are as follows


| Command     | Emoji |
|-------------|-------|
| `:smile:`     | 😊     |
| `:laughing:`     | 😂     |
| `:thumbsup:` | 👍     |
| `:cry: `      | 😢     |

For all codes please refer to [Emoji Cheat Sheet](https://github.com/ikatyang/emoji-cheat-sheet/tree/master).
