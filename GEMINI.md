# GEMINI.md

## Project Overview

This project is a server monitoring solution written in Rust. It consists of two main components: an `agent` and a `hub`.

*   **`agent`**: A lightweight web server built with the Rocket framework. It runs on the servers you want to monitor and exposes a `/metrics` endpoint to provide system information, including CPU usage, memory consumption, and component temperatures. Access to the `/metrics` endpoint is secured with a secret key.

*   **`hub`**: The central component of the monitoring system. It reads a configuration file that specifies which servers to monitor. It then dispatches monitor tasks to collect data from the agents.

The project uses the `sysinfo` crate to gather system metrics and `serde` for data serialization.

## Configuration

The `hub` is configured using a JSON file. An example configuration is provided in `config.example.json`. The configuration file allows you to define the servers to be monitored, the monitoring interval, and alerting rules.

### Server Configuration

Each server entry in the configuration file has the following properties:

*   `ip`: The IP address of the server to be monitored.
*   `display`: A user-friendly name for the server.
*   `port`: The port on which the agent is running (defaults to 3000).
*   `interval`: The monitoring interval in seconds (defaults to 15).
*   `token`: The secret token to access the agent's `/metrics` endpoint.

### Alerting

The system can send alerts to Discord webhooks when predefined thresholds for temperature and CPU usage are exceeded. You can configure the following for each alert:

*   `limit`: The threshold value.
*   `grace`: The number of consecutive times the threshold must be exceeded before an alert is sent.
*   `alert`: The alert configuration, which can be a `discord` or `webhook` object.

#### Discord Alert

*   `url`: The Discord webhook URL.
*   `user_id`: An optional Discord user or role ID to mention in the alert.

#### Webhook Alert

*   `url`: The URL of the webhook to send the alert to.

## Building and Running

The project uses `just` as a command runner. The following commands are available in the `justfile`:

*   **Build the project:**
    ```bash
    just build
    ```
    or
    ```bash
    cargo build
    ```

*   **Build the project in release mode:**
    ```bash
    just build-release
    ```
    or
    ```bash
    cargo build --release
    ```

*   **Run the tests:**
    ```bash
    just test
    ```
    or
    ```bash
    cargo test --workspace
    ```

*   **Build the binaries:**
    ```bash
    just bins
    ```
    or
    ```bash
    cargo build --bins
    ```

*   **Build the binaries in release mode:**
    ```bash
    just bins-release
    ```
    or
    ```bash
    cargo build --bins --release
    ```

*   **Watch for changes and rebuild the binaries:**
    ```bash
    just watch
    ```

*   **Install the binaries:**
    ```bash
    just install
    ```

## Development Conventions

*   **Testing:** The project has a test suite that can be run with `just test`.
*   **Dependencies:** The project uses `cargo` to manage dependencies, which are listed in the `Cargo.toml` file.
*   **Code Style:** The code follows standard Rust conventions.
