# moccasin

A TUI feed reader for RSS, Atom, and (eventually) Podcasts. VIM keybindings. Ranger-inspired interface. Configurable.

![Crates.io (version)](https://img.shields.io/crates/v/moccasin.svg?style=flat-square)
![CI status](https://img.shields.io/github/actions/workflow/status/rektdeckard/moccasin/vhs.yaml?style=flat-square)

[![GitHub stars](https://img.shields.io/github/stars/rektdeckard/moccasin?style=flat-square&label=Star)](https://github.com/rektdeckard/moccasin)
[![GitHub forks](https://img.shields.io/github/forks/rektdeckard/moccasin?style=flat-square&label=Fork)](https://github.com/rektdeckard/moccasin/fork)
[![GitHub watchers](https://img.shields.io/github/watchers/rektdeckard/moccasin?style=flat-square&label=Watch)](https://github.com/rektdeckard/moccasin)
[![Follow on GitHub](https://img.shields.io/github/followers/rektdeckard?style=flat-square&label=Follow)](https://github.com/rektdeckard)

![tabs TUI in action](https://github.com/rektdeckard/moccasin/blob/main/meta/vhs.gif?raw=true)

## Installation

```bash
cargo install moccasin
```

### NetBSD

If you are on NetBSD, a pre-compiled binary is available from the official repositories. To install it, simply run:

```bash
pkgin install moccasin
```

Or, if you prefer to build from source:
```
cd /usr/pkgsrc/news/moccasin
make install
```

## Usage

Since "moccasin" is hard to spell and has too many letters, the executable is just called `mcsn`.

```bash
mcsn [OPTIONS]
```

### Options

Command line arguments will override any values set in your [config file](#moccasintoml) for that session.

| Short | Long             | Args             | Description                                                                                             |
| ----- | ---------------- | ---------------- | ------------------------------------------------------------------------------------------------------- |
| `-c`  | `--config`       | \<PATH\>         | Set a custom config file                                                                                |
| `-s`  | `--color-scheme` | \<COLOR_SCHEME\> | Set a color scheme, either [built-in](#moccasintoml) or a path to a [custom theme](#color-schemes) file |
| `-i`  | `--interval`     | \<INTERVAL\>     | Set a custom refresh rate in seconds                                                                    |
| `-t`  | `--timeout`      | \<TIMEOUT\>      | Set a custom request timeout in seconds                                                                 |
| `-n`  | `--no-cache`     |                  | Do not cache feeds in local file-backed database                                                        |
| `-h`  | `--help`         |                  | Print help                                                                                              |
| `-V`  | `--version`      |                  | Print version                                                                                           |

## Config

On first boot, Moccasin will create both a database and a config file in your default config directory, which varies by platform:

| Platform | Value                                                      | Example                                                         |
| -------- | ---------------------------------------------------------- | --------------------------------------------------------------- |
| Linux    | `$HOME`/.config/moccasin/                                  | /home/alice/.config/moccasin/                                   |
| macOS    | `$HOME`/Library/Application Support/com.rektsoft.moccasin/ | /Users/Alice/Library/Application Support/com.rektsoft.moccasin/ |
| Windows  | `{FOLDERID_LocalAppData}`\\rektsoft\moccasin\\config       | C:\Users\Alice\AppData\Local\rektsoft\moccasin\config           |

The `moccasin.toml` file in this directory can be edited to customize app behavior, add feeds in bulk, change the color scheme, etc. Most of these properties can be changed from within the application as well, which will write to this file. Configuration options are as follows:

### `moccasin.toml`

| Table           | Field              | Type          | Default     | Description                                                                                                                                                                                                         |
| --------------- | ------------------ | ------------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `[sources]`     |                    | Table         |             |                                                                                                                                                                                                                     |
|                 | `feeds`            | Array         | `[]`        | URLs of Atom/RSS feeds you wish to see in-app.                                                                                                                                                                      |
| `[preferences]` |                    | Table         |             |                                                                                                                                                                                                                     |
|                 | `color_scheme`     | Enum \| Table | `"default"` | Either a built-in color scheme name, one of `"default"` \| `"borland"` \| `"darcula"` \| `"focus"` \| `"jungle"` \| `"matrix"` \| `"redshift"` \| `"wyse"`, or a table of values described [below](#color-schemes). |
|                 | `sort_feeds`       | Enum          | `"a-z"`     | Order in which to list feeds, one of `"a-z"` \| `"z-a"` \| `"newest"` \| `"oldest"` \| `"unread"` \| `"custom"`                                                                                                     |
|                 | `cache_feeds`      | Boolean       | `true`      | Whether or not to write feeds to a local database for faster startup and access. When `false`, the app will use an in-memory database.                                                                              |
|                 | `refresh_interval` | Integer       | `3600`      | How often to refetch feeds, in seconds.                                                                                                                                                                             |
|                 | `refresh_timeout`  | Integer       | `5`         | How long to wait for each feed before aborting, in seconds.                                                                                                                                                         |

### Color Schemes

To create a custom color scheme, the `color_scheme` field can be declared as a table in which the keys are interface elements and the values are either a built-in ANSI color (which will inherit from your terminal emulator), a HEX color, or in InlineTable with `fg` and `bg` properties of the same type.

```toml
[preferences.color_scheme]
base = { fg = "white", bg = "#000080" }
status = { fg = "gray", bg = "#000080" }
border = "gray"
selection_active = { fg = "#000080", bg = "#fefd72" }
scrollbar = { fg = "white", bg = "gray" }
```

The built-in color names are

- `"white"`
- `"black"`
- `"red"`
- `"green"`
- `"yellow"`
- `"blue"`
- `"magenta"`
- `"cyan"`
- `"gray"`
- `"lightred"`
- `"lightgreen"`
- `"lightyellow"`
- `"lightblue"`
- `"lightmagenta"`
- `"lightcyan"`
- `"lightblack"` | `"darkgray"`

The styleable properties are all optional, inheriting sensible defaults. Available properties are as follows:

| Field              | Default            | Description                                   |
| ------------------ | ------------------ | --------------------------------------------- |
| `base`             | _terminal default_ | Base foreground and background colors         |
| `overlay`          | `base`             | Modal overlays                                |
| `status`           | `base`             | The top menu bar and bottom status bar colors |
| `selection`        | `~base`            | Selected list item                            |
| `selection_active` | `selection`        | Selected list item of active panel            |
| `border`           | `border_active`\*  | Border and titles around panels               |
| `border_active`    | `base`             | Border and title of active panel              |
| `scrollbar`        | `base`             | Thumb (`fg`) and track (`bg`) of scrollbars   |

> \* NOTE: it is important to define `border` when the style it inherits (either `base` or `border_active`) is defined as a hex color, otherwise it will be difficult to know which panel is currently active.

## Keybinds

The application uses VIM-style keybinds, but arrow keys can also be used for navigation. At the moment, the app has a `NORMAL` mode and a `COMMAND` mode. In future, you should also be able to tag and group feeds and items in `GROUP` mode.

### NORMAL mode

| Keys        | Description                       |
| ----------- | --------------------------------- |
| `j`/`k`     | Focus next/previous item          |
| `h`/`l`     | Focus previous/next panel         |
| `Enter`     | Select current item               |
| `Esc`       | Deselect current item/mode        |
| `Tab`       | Cycle tabs                        |
| `b`/`f`/`t` | View Browse/Favorites/Tags tab    |
| `r`         | Refresh all feeds                 |
| `o`         | Open current feed/item in browser |
| `:`         | Enter `COMMAND` mode              |
| `,`         | Open config file                  |
| `?`         | Show keybinds                     |

### COMMAND mode

| Command         | Args     | Description                                                                                            |
| --------------- | -------- | ------------------------------------------------------------------------------------------------------ |
| `:a`, `:add`    | \<URL\>  | Add a feed                                                                                             |
| `:d`, `:delete` | [URL]    | Delete feed for `URL`, or current feed if not supplied. Removes this entry from config file and cache. |
| `:s`, `:search` | \<TEXT\> | Search for a feed, item, or text content                                                               |

## License

MIT © [Tobias Fried](https://github.com/rektdeckard)
