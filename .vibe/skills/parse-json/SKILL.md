---
name: parse-json
description: Load this skill when you need to parse, query, or transform JSON data using jaq (Just Another Query Tool), a lightweight jq-compatible command-line JSON processor.
---

# Parse JSON with jaq

Use `jaq` to parse, filter, and transform JSON data from files, stdin, or string inputs.

## When to use

- Parsing JSON from files or command output
- Extracting specific fields from JSON structures
- Filtering arrays or objects
- Transforming JSON from one format to another
- Validating JSON syntax

## Basic Usage

### Filter JSON from stdin

```bash
cat data.json | jaq '.field'
```

### Filter JSON from a file

```bash
jaq '.items[]' data.json
```

### Parse raw JSON string

```bash
echo '{"key": "value"}' | jaq '.'
```

## Core Commands

| Option | Description |
|--------|-------------|
| `-n, --null-input` | Use null as single input value (no input) |
| `-R, --raw-input` | Read lines of input as strings |
| `-s, --slurp` | Read all input into a single array |
| `-c, --compact-output` | Print JSON without whitespace |
| `-r, --raw-output` | Output strings without quotes |
| `-j, --join-output` | No newline after each output value |
| `-S, --sort-keys` | Sort object keys alphabetically |
| `-i, --in-place` | Overwrite input file with output |
| `-f, --from-file <PATH>` | Read filter from a file |
| `--arg <NAME> <VALUE>` | Set string variable `$NAME` |
| `--argjson <NAME> <VALUE>` | Set JSON variable `$NAME` |

## Common Filter Patterns

### Extract a field

```bash
jaq '.username' user.json
```

### Extract nested field

```bash
jaq '.data.items[0].name' data.json
```

### Filter array elements

```bash
jaq '.items[] | select(.price > 100)' data.json
```

### Transform structure

```bash
jaq '{name: .first_name + " " + .last_name, email: .contact.email}' user.json
```

### Length of array

```bash
jaq '.items | length' data.json
```

### Check if field exists

```bash
jaq 'if .field then true else false end' data.json
```

## Input Options

### Read as raw strings (line by line)

```bash
echo -e "line1\nline2" | jaq -R '.[]'
```

### Slurp all input into array

```bash
jaq -s '.' file1.json file2.json
```

### Read from specific format

```bash
jaq --from yaml '.' data.yaml
```

## Output Options

### Compact output (no whitespace)

```bash
jaq -c '.' data.json
```

### Raw output (no quotes on strings)

```bash
jaq -r '.message' data.json
```

### Sort object keys

```bash
jaq -S '.' data.json
```

### Custom indentation

```bash
jaq --indent 4 '.' data.json
```

### Output as different format

```bash
jaq --to cbor '.' data.json
```

## Variables

### String variable

```bash
jaq --arg name "John" '.name = $name' data.json
```

### JSON variable

```bash
jaq --argjson threshold 100 '.items[] | select(.value > $threshold)' data.json
```

### File contents as variable

```bash
jaq --rawfile config config.txt '.config = $config' data.json
```

## Multiple files

Process multiple input files:

```bash
jaq '.[]' file1.json file2.json
```

## Validation

Validate JSON syntax (exit code 0 = valid):

```bash
echo '{"test": true}' | jaq empty
```

## Tips

- `jaq` is mostly compatible with `jq` syntax
- Use `.` to pretty-print JSON
- Use `empty` to validate without output
- Chain filters with `|` pipe
- Use `select()` for conditional filtering

## Documentation

Full manual: <https://gedenkt.at/jaq/manual/>
