# Axum Chat Api

A chat server built with Axum and Sqlx

## Features

- High performance through Rust
- Authentication based on jsonwebtoken
- Connect to the postgres database
- Cache with redis

## Getting Started

### Clone this Repository

```
$ git clone https://github.com/bwen19/axum-chat-api.git
$ cd axum-chat-api
```

### Installing Rust and Cargo

Install Rust as described in [The Rust Programming Language, chapter 1.](https://doc.rust-lang.org/book/ch01-01-installation.html)

This is the official Rust language manual and is freely available on doc.rust-lang.org.

The latest stable version is fine.

### Installing sqlx-cli

SQLx provides a command-line tool for creating and managing databases as well as migrations. It is published on the Cargo crates registry as sqlx-cli and can be installed like so:

```
$ cargo install sqlx-cli --features postgres
```

### Running Postgres

By far the easiest way to run Postgres these days is using a container with a pre-built image.

The following command will start version 14 of Postgres (the latest at time of writing) using Docker (this command should also work with Podman, a daemonless FOSS alternative).

```
$ make postgres
```

### Setting Up the Application Database

With sqlx-cli installed and your .env file set up, you only need to run the following command to get the Postgres database ready for use:

```
$ sqlx migrate run
```

### Starting the Application

With everything else set up, all you should have to do at this point is:

```
$ cargo run
```

If successful, the Realworld-compatible API is now listening at port 8080.

## Licence

[MIT © bwen19](/LICENSE)
