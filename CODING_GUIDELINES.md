# Coding Guidelines

## String Handling

### UTF-8 Safety: Never Use Byte-Based String Slicing

Rust strings are UTF-8 encoded. Multi-byte characters (Japanese, emoji, etc.) will cause **panics** if sliced at byte boundaries.

**NEVER do this:**
```rust
// WRONG: Byte-based slicing - will panic on multi-byte characters
let truncated = &text[..10];
let truncated = &text[..(width as usize)];
let truncated = &text[..text.len().min(8)];
```

**ALWAYS do this:**
```rust
// CORRECT: Character-based operations
let truncated: String = text.chars().take(10).collect();
let truncated: String = text.chars().take(width as usize).collect();
let truncated: String = text.chars().take(8).collect();

// For length checks, use chars().count() instead of len()
if text.chars().count() > max_width {
    let truncated: String = text.chars().take(max_width - 3).collect();
    format!("{}...", truncated)
} else {
    text.clone()
}
```

### Exception: ASCII-Only Strings

Byte slicing is acceptable ONLY for strings guaranteed to be ASCII:
- Git OIDs (commit hashes): `oid.to_string()[..7]`
- Hex color codes: `&hex[0..2]`

When in doubt, use character-based operations.

### Display Width Considerations

For terminal display, note that:
- Full-width characters (CJK) occupy 2 columns
- Half-width characters occupy 1 column

Consider using the `unicode-width` crate for accurate display width calculation if needed.
