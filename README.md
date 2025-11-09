<h1 align="center">Miou</h1>

<div align="center">
  <em>A <a href="https://matrix.org/">Matrix</a> bot for <a href="https://github.com/terraforming-mars/terraforming-mars">Terraforming Mars</a> game notifications</em>
  <br /> 
  <br />  
  <img src="assets/miou.png" style="width:250px;"/>
  <hr />
  <a href="https://github.com/florianduros/miou/releases"><img src="https://img.shields.io/github/v/release/florianduros/miou?style=flat&labelColor=516B3A&color=E6E1A1&logo=GitHub&logoColor=white"></a>
  <a href="https://crates.io/crates/miou"><img src="https://img.shields.io/crates/v/miou?style=flat&labelColor=516B3A&color=E6E1A1&logo=Rust&logoColor=white"></a>
  <a href="https://docs.rs/miou"><img src="https://img.shields.io/docsrs/miou?style=flat&labelColor=516B3A&color=E6E1A1&logo=Rust&logoColor=white"></a>
  <br />
  <a href="https://app.codecov.io/gh/florianduros/miou"><img src="https://img.shields.io/codecov/c/gh/florianduros/miou?label=coverage&style=flat&labelColor=516B3A&color=E6E1A1&logo=codecov&logoColor=white"></a>
  <a href="https://github.com/florianduros/miou/actions/workflows/tests.yml"><img src="https://img.shields.io/github/actions/workflow/status/florianduros/miou/tests.yml?label=tests&style=flat&labelColor=516B3A&color=E6E1A1&logo=GitHub&logoColor=white"></a>
  <a href="https://github.com/florianduros/miou/actions/workflows/analysis.yml"><img src="https://img.shields.io/github/actions/workflow/status/florianduros/miou/analysis.yml?label=analysis&style=flat&labelColor=516B3A&color=E6E1A1&logo=GitHub&logoColor=white"></a>
</div>

<div align="center">Miou is only compatible with the <a href="https://github.com/terraforming-mars/terraforming-mars">Terraforming Mars server</a> and works in <strong>encrypted</strong> Matrix rooms.</div>
<br />

- [Usage](#usage)
  - [Commands](#commands)
  - [Alert](#alert)
- [Installation](#installation)
  - [Docker](#docker)
  - [Github release](#github-release)
  - [From Source](#from-source)
  - [From Cargo](#from-cargo)
- [Configuration](#configuration)
  - [Configuration Options](#configuration-options)
  - [Command-Line Arguments](#command-line-arguments)
- [License](#license)

## Usage

To use Miou in your Matrix room, you need to invite the bot using its Matrix ID (configured in your deployment). Miou will join the room and listen for commands.


You can interact with it by sending messages starting with the prefix `!miou`. For example, to see the help message, you can send:
```sh
!miou help
```

All available commands are listed below:

- `games`: list all the ongoing games
- `alerts`: list your registered alerts
- `register <game_id> <player_name> <delay_in_minutes>`: register a new alert
- `unregister <game_id>`: unregister an alert
- `help`: show this help message

### Commands

#### Games

```sh
!miou games
```

The `games` command lists all the ongoing games on the Terraforming Mars server. It shows the game ID, the current phase, and the list of players in each game. Players who are currently taking their turn are marked with an hourglass (⏳).

Response:
```sh
Games:
  - game_id1(Research), players: Player_1(⏳), Player_2, Player_3(⏳)
  - game_id2(Drafting), players: Player_1
```

#### Alerts

```sh
!miou alerts
```

The `alerts` command lists all the alerts you have registered in the current Matrix room. It shows the game ID and the player name for each alert.

Response:
```sh
Registered alerts:
  - game_id1: Player_1
  - game_id2: Player_2
```

#### Register

```sh
!miou register game_id1 Player_1 2
```

The `register` command allows you to register an alert for a specific game and player. You need to provide the game ID, the player name, and the delay in minutes before the alert is sent. The delay must be between 1 minute and 1 week.

Response:
```sh
You have been registered successfully.
```

#### Unregister

```sh
!miou unregister game_id1
```

The `unregister` command allows you to unregister an alert for a specific game. You need to provide the game ID.

Response:
```sh
You have been unregistered successfully.
```

#### Help

```sh
!miou help
```

The `help` command shows the help message with all available commands.

### Alert

When you register an alert for a game, Miou will send you a notification in the Matrix room when it's your turn to play. The notification will be sent after the specified delay in minutes.

The bot polls data from the Terraforming Mars server at a configured interval (specified in your [`config.yaml` as `polling_interval`](#polling-interval)). The alert triggers after the bot detects that it's the player's turn (depending on the polling interval) and the specified delay has passed.

## Installation

### Docker

You can use the following `docker-compose.yml` file:
```yaml
services:
  miou:
    image: florianduros/miou:latest
    container_name: miou
    restart: unless-stopped
    volumes:
      # Mount configuration file or/and use environment variables to configure
      # - ./miou.yaml:/config/miou.yaml
      - ./data:/data
    environment:
      # Set log level, info by default
      - RUST_LOG=debug
      # Override configuration via environment variables
      # Use MIOU_ prefix with double underscores for nested paths
      - MIOU_TMARS__URL=https://tmars.example.com
      - MIOU_TMARS__SERVER_ID=your-server-id
      - MIOU_TMARS__POLLING_INTERVAL=120
      - MIOU_MATRIX__USER_ID=@miou:matrix.org
      - MIOU_MATRIX__PASSWORD=your-password
      - MIOU_MATRIX__PASSPHRASE=your-passphrase
```

### Github release 

You can download the latest release from the [Releases](https://github.com/florianduros/miou/releases) page.

Run
```bash
miou --config config.yaml --data-path ./data
```

### From Source

Prerequisites
- Rust 1.70 or later


1. Clone the repository:
```bash
git clone https://github.com/florianduros/miou.git
cd miou
```

2. Build the project:
```bash
cargo build --release
```

3. Create a `config.yaml` file (see Configuration section below)

4. Run the bot:
```bash
cargo run --release -- --config config.yaml --data-path ./data
```

### From Cargo

```bash
cargo install miou
miou --config config.yaml --data-path ./data
```

## Configuration

Create a `config.yaml` file with the following structure:

```yaml
# TMars Server Configuration
tmars:
  url: "https://terraforming-mars.herokuapp.com"
  server_id: "your-server-id"
  polling_interval: 120  # seconds between polling the TMars server

# Matrix Account Configuration
matrix:
  user_id: "@miou:matrix.org"
  password: "your-bot-password"
  passphrase: "your-recovery-passphrase"
```

### Environment Variable Overrides

You can override any configuration value using environment variables with the `MIOU_` prefix.
Use double underscores (`__`) to separate nested configuration paths:

```bash
export MIOU_TMARS__URL="https://terraforming-mars.herokuapp.com"
export MIOU_TMARS__SERVER_ID="your-server-id"
export MIOU_TMARS__POLLING_INTERVAL="120"
export MIOU_MATRIX__USER_ID="@miou:matrix.org"
export MIOU_MATRIX__PASSWORD="your-bot-password"
export MIOU_MATRIX__PASSPHRASE="your-recovery-passphrase"
miou --config config.yaml --data-path ./data
```

Environment variables take precedence over values in the YAML file. This allows you to:
- Keep sensitive credentials out of config files
- Use the same config file across different environments
- Easily integrate with Docker, Kubernetes, and CI/CD systems

### Configuration Options

- `tmars.url`: Base URL of the Terraforming Mars server
  - Environment variable: `MIOU_TMARS__URL`
- `tmars.server_id`: Server identifier for the TMars instance
  - Environment variable: `MIOU_TMARS__SERVER_ID`
<a name="polling-interval"></a>
- `tmars.polling_interval`: Seconds between game state polls
  - Environment variable: `MIOU_TMARS__POLLING_INTERVAL`
- `matrix.user_id`: Matrix user ID for the bot account
  - Environment variable: `MIOU_MATRIX__USER_ID`
- `matrix.password`: Password for the Matrix account
  - Environment variable: `MIOU_MATRIX__PASSWORD`
- `matrix.passphrase`: Recovery passphrase for end-to-end encryption
  - Environment variable: `MIOU_MATRIX__PASSPHRASE`

**Environment Variables**: All configuration values can be overridden using environment variables with the `MIOU_` prefix. Use double underscores (`__`) to represent nested paths (e.g., `MIOU_TMARS__URL` for `tmars.url`).

### Command-Line Arguments

- `--config`: Path to the configuration file (required)
- `--data-path`: Directory for storing matrix session data and alerts (required)

The data directory will contain:
- `session/`: Matrix session data and encryption keys
- `alerts`: JSON file with registered alerts

**Security Note**: The data directory contains sensitive information including authentication tokens and encryption keys. Ensure it has appropriate permissions.

## License

[AGPL](https://www.gnu.org/licenses/#AGPL)

[Matrix]: https://matrix.org/
[Terraforming Mars]: https://github.com/terraforming-mars/terraforming-mars
