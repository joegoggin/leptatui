# Editor support inside `view!`

Use rust-analyzer with procedural macro expansion enabled. Leptatui provides
source references for tag names, props, and embedded Rust expressions, including
unfinished markup. No alternative markup syntax is required.

| Feature | Provider |
| --- | --- |
| Built-in and custom component completion | rust-analyzer |
| Component props and built-in attributes | rust-analyzer |
| Rust fields, methods, and values inside expressions | rust-analyzer |
| Hover documentation and definitions | rust-analyzer |
| Diagnostics for syntax, unknown props, and value types | rust-analyzer and Cargo |
| Complete component/prop rename and references across generated APIs | `leptatui-lsp` proxy |

The proxy runs rust-analyzer and forwards normal Rust requests. For component
and prop rename and references, it combines rust-analyzer results for generated
symbols that resolve to the same original declaration. A rename includes markup,
component-local prop bindings, builder methods, and generated props type names.
Comments and strings are excluded from the identifier index. Unsaved documents
participate, and an edit during rename invalidates the result so it can be retried.

## Install

From this checkout:

```sh
rustup component add rust-analyzer
cargo install --path crates/leptatui-lsp --locked
```

`rust-analyzer` must be on `PATH`. Set `RUST_ANALYZER` to an alternate executable
path if needed. Arguments passed to `leptatui-lsp` are forwarded to rust-analyzer.

## Neovim

For an existing Neovim LSP configuration, change the Rust server command to
`{ "leptatui-lsp" }`. Keep your completion capabilities, settings, and keymaps.
The proxy replaces the rust-analyzer command; use one Rust server client per
buffer.

For Neovim 0.11 or newer with `nvim-lspconfig`:

```lua
vim.lsp.config("rust_analyzer", {
  cmd = { "leptatui-lsp" },
  settings = {
    ["rust-analyzer"] = {
      procMacro = { enable = true },
      cargo = { buildScripts = { enable = true } },
    },
  },
})
vim.lsp.enable("rust_analyzer")
```

If another plugin manages Rust LSP startup, configure its server command instead.
Use `:checkhealth vim.lsp` to check which client is attached. Restart the server
after rebuilding the macro crate or installing a new proxy binary.

Do not put Leptatui's `view` or `component` macros on rust-analyzer's ignored macro
list: their expansion supplies the semantic information.

## Validation and boundaries

Run the protocol suite from the repository root:

```sh
cargo build -p leptatui-lsp
python3 tools/lsp/test_view.py
```

Use `LEPTATUI_LSP=rust-analyzer` to run the same cases against the native server
and inspect the gaps covered by the proxy. The suite uses temporary applications
and applies rename edits before compiling the result.

The tested baseline is rust-analyzer/Rust 1.92.0. Formatting, automatic closing-tag
insertion, and language services inside arbitrary strings are outside this
feature. Diagnostics still reject invalid markup; recovery preserves semantic
information around the error rather than accepting that markup as valid Rust.
