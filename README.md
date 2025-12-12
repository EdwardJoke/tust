# tust

Tust is a command-line tool that allows you to test commands first and decide whether to execute them based on their behavior. It runs commands in a temporary directory, shows you the changes that would be made, and only applies them after you confirm.

## How It Works

1. **Isolated Execution**: When you run `tust <command>`, the tool creates a temporary directory and copies your current directory's contents into it.
2. **Command Testing**: The specified command is executed in this temporary directory.
3. **Change Detection**: tust compares the original directory with the modified temporary directory to identify all changes.
4. **Change Preview**: A clear, colored list of changes (files to be created, modified, or deleted) is displayed.
5. **User Confirmation**: Only after you confirm (by typing 'y') are the changes applied to your original directory.

## Installation

```bash
# Clone the repository
git clone https://codeberg.org/EdwardJoke/tust.git
cd tust
# Build the tool
cargo build --release
```

## Usage

### Basic Usage

```bash
tust <command>
```

### Example

```bash
$ tust uv init
Testing command in temporary directory...

Changes that would be made:
  +config
  +commit-msg.sample
  +pyproject.toml
  +.python-version
  +description
  +pre-commit.sample
  +applypatch-msg.sample
  +pre-merge-commit.sample
  +.gitignore
  +post-update.sample
  +pre-receive.sample
  +pre-push.sample
  +update.sample
  +prepare-commit-msg.sample
  +main.py
  +fsmonitor-watchman.sample
  +HEAD
  +sendemail-validate.sample
  +exclude
  +pre-rebase.sample
  +push-to-checkout.sample
  +pre-applypatch.sample
  +README.md

Would you like to apply these changes? (y/n)
y
Changes applied successfully
```

## Command-Line Options

| Option | Short | Description |
|--------|-------|-------------|
| `--clean` | `-c` | Clean up all temporary directories created by tust |
| `--help` | `-h` | Print help information |
| `--version` | `-V` | Print version information |

## Features

- **Safe Testing**: Test commands without risking changes to your actual files
- **Clear Change Preview**: See exactly what files will be created, modified, or deleted
- **Colored Output**: Easy-to-read output with colored indicators for different change types
- **User Confirmation**: Complete control over whether changes are applied
- **Cleanup Option**: Easily remove all temporary directories created by tust
- **Async Architecture**: Built on the Tokio async framework for efficient execution

## Use Cases

- Testing package manager commands (e.g., `tust uv init`, `tust cargo new`)
- Running scripts that modify files (e.g., `tust ./setup.sh`)
- Experimenting with file operations (e.g., `tust mv *.txt docs/`)
- Safely running unfamiliar commands

## License and Inspiration

Tust is licensed under the Apache License 2.0. 
It is inspired by the `try` command-line tool, which provides similar functionality for trying commands before you running it.
