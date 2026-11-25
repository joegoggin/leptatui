# Editor support inside `view!` and `stylesheet!`

Use rust-analyzer with procedural macro expansion enabled. Leptatui provides
source references for tag names, props, and embedded Rust expressions, including
unfinished markup. Stylesheet values preserve the same Rust expression support,
while the proxy supplies stylesheet syntax and symbol services.

| Feature | Provider |
| --- | --- |
| Built-in and custom component names | rust-analyzer |
| Complete paired/self-closing tag snippets | `leptatui-lsp` proxy |
| Brace-delimited `view! { }` and `stylesheet! { }` completion | rust-analyzer and `leptatui-lsp` |
| Component props and built-in attributes | rust-analyzer |
| Rust fields, methods, and values inside expressions | rust-analyzer |
| Hover documentation and definitions | rust-analyzer |
| Diagnostics for syntax, unknown props, and value types | rust-analyzer and Cargo |
| Complete component/prop rename and references across generated APIs | `leptatui-lsp` proxy |
| Whole-document formatting of `stylesheet!` | `leptatui-lsp` and rustfmt |

The proxy runs rust-analyzer and forwards normal Rust requests. For component
and prop rename and references, it combines rust-analyzer results for generated
symbols that resolve to the same original declaration. A rename includes markup,
component-local prop bindings, builder methods, and generated props type names.
Comments and strings are excluded from the identifier index. Unsaved documents
participate, and an edit during rename invalidates the result so it can be retried.

## Completion snippets

Accepting `view!` completion inserts `view! { }` with the cursor between the
braces. Inside markup, component completion inserts the full tag structure:

```text
Form or <Form    → <Form>$0</Form>
Input or <Input  → <Input $0 />
```

Here `$0` marks the snippet cursor; it is not inserted literally. Paired tags
place the cursor in the body. Self-closing tags put a space on either side of
the cursor so you can add props. Required props are left for you to enter.

Built-ins accepting child views or text receive paired tags. Leaf controls,
images, Markdown files, outlets, and leaf routes receive self-closing tags.
Custom components receive paired tags when they declare a `children` prop of
type `Children`, `ChildrenFn`, or `ChildrenMut`, including qualified type names.
A defaulted children prop still counts. Other custom components receive
self-closing tags. The resolved component contract follows imports and aliases.

Completion reuses existing delimiters and closing tags and preserves attributes
and child content. Rust expressions, attribute values, strings, and comments
keep their normal completion behavior. Clients without snippet support receive
plain tag text.

## Stylesheet support

Accepting `stylesheet!` inserts `stylesheet! { }` with the cursor inside the
braces. Inside a rule such as `H1 { }`, the proxy lists all supported properties
and aliases before nested type selectors. It also completes pseudo-classes,
media features, directives, local variables, mixins, and imported members.
Property suggestions include descriptions, expected types, and examples; accepting
one inserts the colon, value cursor, and semicolon without duplicating existing
punctuation.

At an empty property value or a partial value name, the proxy prioritizes choices
for that property's expected type: colors, enum variants, flags, and constructors
for spacing, sizing, axes, edges, and grid tracks. Wrapped properties receive
complete expressions, such as `Axes::all(Overflow::Hidden)`. Constructor snippets
provide numbered argument defaults; clients without snippet support receive
editable Rust defaults. Generated expressions use type names such as `Color` and
`LayoutSize` without a crate prefix; import the required types, for example with
`use leptatui::prelude::*;`. Explicitly authored paths such as `leptatui::Color::`
are preserved when completing their members. Completion does not add imports.

Expected values merge with the Rust completions available from rust-analyzer.
Local Rust names remain available as you type, and fields, methods, function
arguments, and struct literals retain ordinary Rust completion. These services
also work while the surrounding stylesheet is unfinished.

```text
flex_di              → flex_direction: $0;
fg: Re               → fg: Color::Red
fg: Color::Li        → fg: Color::LightRed
modifier: Modifier::BOLD | un → modifier: Modifier::BOLD | Modifier::UNDERLINED
&:fo                 → &:focus
@include colors.ch   → @include colors.chrome
fg: colors.$ac       → fg: colors.$accent
```

Typing `:` or manually requesting completion opens value suggestions. After
accepting a property snippet, clients advertising `editor.action.triggerSuggest`
in `capabilities.experimental.commands.commands` also receive a command to open
value completion immediately. Other clients can invoke completion manually;
automatic popup behavior depends on the editor's completion configuration.

Property hover describes the expected Rust value type. Variables, mixins, and
module namespaces support hover, definitions, references, and rename. Imported
helpers are resolved through rust-analyzer, so qualified paths and `as` aliases
keep their Rust meaning. Static exports are discovered from helpers that directly
return a single `stylesheet!` module. Runtime-built or dynamically extended
`StyleModule` values are not evaluated, and their unknown exports are not
reported as missing. Renaming an implicit import namespace introduces an `as`
alias; renaming the Rust helper also updates namespaces that follow its name.

Class and ID selectors link to matching literal `class` and `id` attributes
across workspace files, including unsaved edits, plain, escaped, or raw strings, and
single-literal brace expressions. These are associations by name, with separate
class and ID namespaces; they do not infer component scope at runtime. BEM
suffixes resolve to their full class names. Parent renames update expanded names
in view attributes while preserving implicit suffixes in stylesheets. Renames
that require changing a different BEM parent are rejected. Dynamic attribute expressions are left to Rust tooling.

The proxy contributes stylesheet syntax and symbol diagnostics and merges them
with rust-analyzer's diagnostics. Compiler validation and Rust type checking
remain active. Comments and string contents do not trigger stylesheet syntax
completion; class and ID literal attributes have their own name completions.
Document formatting is described below. Formatting while typing and automatic
edits outside accepted completion, formatting, or rename requests are not provided.

## Stylesheet formatting

The proxy extends ordinary `textDocument/formatting` requests with formatting for
complete `stylesheet!` invocations, including qualified names and imported aliases.
Rust formatting remains provided by rust-analyzer and its configured formatter.
The proxy composes those results with stylesheet edits against the current unsaved
buffer; it does not write source files. Range formatting keeps rust-analyzer's
existing behavior.

The outer macro body is multiline. A short rule with one declaration stays inline;
rules containing multiple declarations, nested blocks, comments, or multiline
values expand. Media and mixin blocks are always multiline. Consecutive declarations
stay together, with one blank line between rules and around nested blocks:

```rust
stylesheet! {
    H1 { fg: Color::LightCyan; }

    H2 { fg: Color::Cyan; }

    .shell {
        padding: TuiSpacing::uniform(1);

        &__actions {
            display: Display::Flex;
            gap: Axes::all(Length::cells(1.0));

            @media (max-width: 60) {
                flex_direction: FlexDirection::Column;
            }
        }
    }
}
```

The formatter preserves statement order, comments, literal contents, stylesheet
variable references, and `!important`. It uses rustfmt for embedded Rust expressions
and leaves an expression unchanged if it cannot safely format it. Incomplete or
malformed macros are left alone. A buffer change or cancellation during formatting
invalidates the response so stale edits are not applied.

Editor formatting options determine indentation; project rustfmt settings determine
line width, with four spaces and 100 columns as defaults. The consumer package's
Rust edition is discovered through Cargo metadata without updating its lockfile.
Ensure `rustfmt` is available with `rustup component add rustfmt`.

`view!` formatting can continue to use the external `leptosfmt` executable. To
compose it with stylesheet formatting through one LSP request, add this setting
under `settings["rust-analyzer"]` in the Neovim configuration below:

```lua
rustfmt = {
    overrideCommand = { "leptosfmt", "--stdin", "--rustfmt" },
},
```

This optional setting requires `leptosfmt` on `PATH`. When none-ls also supplies
Rust formatting, select only the Rust LSP for buffers containing stylesheet macros
and suppress the none-ls source for those requests. Other Rust buffers can retain
their existing none-ls formatting. Wait for synchronous formatting to finish before
saving the buffer.

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

Do not put Leptatui's `view`, `stylesheet`, or `component` macros on
rust-analyzer's ignored macro list: their expansion supplies the semantic information.

## Validation and boundaries

Run the protocol suite from the repository root:

```sh
cargo build -p leptatui-lsp
python3 tools/lsp/test_view.py
python3 tools/lsp/test_stylesheet.py
python3 tools/lsp/test_formatting.py
```

Use `LEPTATUI_LSP=rust-analyzer` to run the same cases against the native server
and inspect the gaps covered by the proxy. The suite uses temporary applications
and applies rename edits before compiling the result.

The tested baseline is rust-analyzer/Rust 1.92.0. Automatically adding closing
tags while typing outside completion and language services inside
arbitrary strings are outside this feature. Diagnostics still reject invalid
markup; recovery preserves semantic information around the error rather than
accepting that markup as valid Rust.
