# Markdown Reader Showcase

This fixture demonstrates Leptatui's **semantic Markdown rendering**, *inline emphasis*, `inline code`, and [readable links](https://github.com/joegoggin/leptatui). Long paragraphs wrap with the terminal width while preserving Unicode text such as café, λ, and 界.

## Heading Level Two

### Heading Level Three

#### Heading Level Four

##### Heading Level Five

###### Heading Level Six

The reader preserves a soft
line break inside a paragraph and a hard break after this sentence.  
This text begins on the next rendered line.

## Nested Lists

3. Ordered items can begin at a non-one value.
4. Ordered items can contain unordered details.
   - Nested bullets retain their indentation.
   - Bullets can contain another ordered list.
     7. Deeply nested numbering remains readable.
     8. Wrapped list content stays aligned with its marker in narrow terminals.
5. The outer ordered list resumes after nested content.

- Unordered lists can also contain ordered children.
  1. First nested task.
  2. Second nested task.
- Mixed list structures preserve source order.

## Aligned Table

| Feature | Presentation | Status |
| :------ | :----------: | -----: |
| Headings | Semantic | Ready |
| Lists | Nested | Ready |
| Tables | Responsive | Ready |
| Code | Highlighted | Ready |

## Readable Fallback Blocks

> Blockquotes use a visible left border.
>
> > Nested blockquotes retain their source hierarchy.

---

Images become deterministic text when no semantic image view is available:

![Leptatui diagram](https://example.com/leptatui-diagram.png)

Literal HTML remains readable instead of disappearing:

<aside>
Fallback HTML content &amp; entities remain visible.
</aside>

## Highlighted Code Fences

```rust
fn main() {
    println!("Hello, 界!");
}
```

```json
{
  "name": "leptatui",
  "semantic_markdown": true
}
```

```toml
[reader]
theme = "dark"
line-numbers = true
```

Unknown language names remain visible and fall back to unhighlighted source:

```leptatui-example-language
render document --full-screen
```

## End of Fixture

Use the arrow keys or `j` and `k` to scroll one row, Page Up and Page Down to move five rows, `gg` to return to the top, `G` to jump to the bottom, and `q` to quit.
