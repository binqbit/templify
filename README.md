# templify

A small CLI for creating reusable text templates from a description and rendering them from command line parameters.

## Requirements
- Rust toolchain (stable)
- OpenAI API key
- Microphone input device

On Linux, `cpal` may require ALSA development libraries (for example `libasound2-dev` on Debian/Ubuntu).
For robust clipboard support, install one of:
- Wayland: `wl-clipboard` (`wl-copy`, `wl-paste`)
- X11: `xclip` or `xsel`
- KDE fallback: `qdbus` + Klipper (works, but `wl-clipboard`/`xclip` is recommended)

## Setup
Create a `.env` file in this folder:

```bash
OPENAI_API_KEY=your_api_key_here
```

Optional:

```bash
OPENAI_BASE_URL=https://api.openai.com/v1
TEMPLIFY_TEMPLATE_MODEL=gpt-5.2
TEMPLIFY_TRANSCRIBE_MODEL=gpt-4o-transcribe
# Legacy fallback also supported:
# OPENAI_TEMPLATE_MODEL=gpt-5.2
```

## Build

```bash
./build.sh
```

## Commands

### Create template

```bash
./templify.sh new followup
```

`templify new` captures speech from the microphone, transcribes it, and uses that text as the template description.
You can override this with `--prompt "..."`.
Use `--device <index>` to select a specific input device.
Use `--force` to overwrite an existing key.
`followup` here is the template key used later in `get`.

Current clipboard content is included as extra context during template creation.

Template is saved in `<project_dir>/data/templates.json` by default, or in the path passed by `--data-dir`.

### Render template

```bash
./templify.sh get followup "Dev" "project X"
```

The template renderer replaces:

- `$1`, `$2`, ... with positional parameters
- `$PASTE` with current clipboard text

Rendered result is copied to clipboard and printed to stdout.

### Show template

```bash
./templify.sh show followup
```

Prints raw template text by key and waits until you press Enter.
Prints description, creation time, and raw template text by key, then waits until you press Enter.
