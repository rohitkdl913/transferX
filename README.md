# transferX

A simple, fast, and efficient file transfer application built with Rust. This project uses a custom chunk-based protocol for transferring files between a sender and a receiver, supporting parallel downloads for improved performance.

## Features

- **Chunked File Transfer:** Files are divided into chunks, allowing simultaneous transfers.
- **Custom Protocol:** Utilizes a protocol to request metadata and file chunks.
- **Concurrency:** Built with Tokio for asynchronous networking.
- **Progress Tracking:** Real-time progress bar to monitor file transfer.
- **Configurable:** Uses TOML configuration for flexibility.

## How It Works

1. **Sender:** Hosts the file, listens for incoming connections, and responds to metadata and chunk requests.
2. **Receiver:** Connects to the sender, requests file metadata, and downloads file chunks concurrently.

## Usage

### Command-line Interface

```
Usage: transferx <COMMAND>

Commands:
  send     Send a file to the receiver
  receive  Receive a file from the sender
  help     Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

### Running the Sender
```bash
transferx send <FILE>
```

### Running the Receiver
```bash
transferx receive <ADDRESS>
```

## Dependencies

- **Tokio:** For async networking.
- **Bincode:** For serializing and deserializing protocol messages.
- **Local IP Address:** To get the local IP for binding the sender.

## License

This project is licensed under the MIT License.

