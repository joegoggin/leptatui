#!/usr/bin/env python3
"""Exercise view! through the installed rust-analyzer, without editor plugins.

Run with `python3 tools/lsp/test_view.py`. Fixtures and their Cargo lockfile live
in a temporary directory; build artifacts use the repository's ignored target.
"""
import json
import os
from pathlib import Path
import queue
import subprocess
import tempfile
import threading
import time
import unittest

ROOT = Path(__file__).resolve().parents[2]
SOURCE = '''use leptatui::prelude::*;
mod extra;

mod widgets {
    use leptatui::prelude::*;
    /// A reusable greeting.
    #[component]
    pub fn Greeting(
        /// The greeting's displayed title.
        #[prop(into)] title: String,
    ) -> impl IntoView {
        view! { <Text>{title.get()}</Text> }
    }
    #[component]
    pub fn Footer(#[prop(into)] title: String) -> impl IntoView {
        view! { <Text>{title.get()}</Text> }
    }
}
use widgets::{Greeting, GreetingProps};
use extra::{Panel, RepeatPanel, MutPanel, DefaultPanel};

#[derive(Clone)]
struct Person { name: String }

#[component]
fn App() -> impl IntoView {
    let person = Person { name: "Ada".into() };
    let other = person.clone();
    view! {
        <Div>
            <Greeting title={person.name.clone()} />
            <Button on_press={|| {}}>"Go"</Button>
            <Text>{other.name.clone()}</Text>
        </Div>
    }
}
fn main() {
    let _ = App::new();
    let _ = Greeting::with_props(GreetingProps::builder().title("Hello").build());
    let _ = widgets::Footer::with_props(widgets::FooterProps::builder().title("Footer").build());
}
'''

EXTRA = '''use leptatui::prelude::*;
#[component]
pub fn Panel(children: leptatui::Children) -> impl IntoView {
    drop(children);
    view! { <Text>"Panel"</Text> }
}
#[component]
pub fn RepeatPanel(children: ChildrenFn) -> impl IntoView {
    drop(children);
    view! { <Text>"Repeat"</Text> }
}
#[component]
pub fn MutPanel(children: ChildrenMut) -> impl IntoView {
    drop(children);
    view! { <Text>"Mutable"</Text> }
}
#[component]
pub fn DefaultPanel(#[prop(default = Box::new(Vec::new))] children: Children) -> impl IntoView {
    drop(children);
    view! { <Text>"Default"</Text> }
}

use crate::widgets::{Greeting, GreetingProps};
pub fn extra() {
    let _ = view! { <Greeting title="Extra"></Greeting> };
    let _ = Greeting::with_props(GreetingProps::builder().title("Builder").build());
}
'''


class Client:
    """A small framed JSON-RPC client with server-request acknowledgement."""

    def __init__(self, project, *, snippets=True, completion_command=True, rustfmt_command=None, env=None):
        self.messages = queue.Queue()
        self.sequence = 0
        self.notifications = []
        self.process = subprocess.Popen(
            [os.environ.get("LEPTATUI_LSP", str(ROOT / "target/debug/leptatui-lsp"))],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE,
            stderr=None if os.environ.get("LEPTATUI_LSP_DEBUG") else subprocess.DEVNULL, cwd=project, env=env,
        )
        threading.Thread(target=self._read, daemon=True).start()
        self.initialize_result = self.request("initialize", {
            "processId": os.getpid(), "rootUri": project.as_uri(),
            "capabilities": {
                "experimental": {"serverStatusNotification": True, "commands": {"commands": ["editor.action.triggerSuggest"] if completion_command else []}},
                "textDocument": {
                    "publishDiagnostics": {"versionSupport": True},
                    "completion": {"completionItem": {
                        "snippetSupport": snippets,
                        "insertReplaceSupport": True,
                        "resolveSupport": {"properties": ["documentation", "detail", "additionalTextEdits"]},
                    }},
                },
            },
            "initializationOptions": {
                "cargo": {"targetDir": str(ROOT / "target")},
                "checkOnSave": False,
                **({"rustfmt": {"overrideCommand": rustfmt_command}} if rustfmt_command else {}),
            },
        })
        self.send("initialized", {})
        deadline = time.monotonic() + 180
        while time.monotonic() < deadline:
            message = self.messages.get(timeout=180)
            if message.get("method") == "experimental/serverStatus" and message["params"]["quiescent"]:
                break
        else:
            raise TimeoutError("rust-analyzer did not finish loading the fixture")

    def _read(self):
        while True:
            headers = {}
            while line := self.process.stdout.readline():
                if line == b"\r\n":
                    break
                key, value = line.decode().split(":", 1)
                headers[key.lower()] = value.strip()
            if not headers:
                return
            message = json.loads(self.process.stdout.read(int(headers["content-length"])))
            if "method" in message and "id" in message:
                self._write({"jsonrpc": "2.0", "id": message["id"], "result": None})
            else:
                self.messages.put(message)

    def _write(self, message):
        payload = json.dumps(message).encode()
        self.process.stdin.write(f"Content-Length: {len(payload)}\r\n\r\n".encode() + payload)
        self.process.stdin.flush()

    def send(self, method, params):
        self._write({"jsonrpc": "2.0", "method": method, "params": params})

    def request(self, method, params, retries=0):
        self.sequence += 1
        request_id = self.sequence
        self._write({"jsonrpc": "2.0", "id": request_id, "method": method, "params": params})
        deadline = time.monotonic() + 180
        while time.monotonic() < deadline:
            message = self.messages.get(timeout=180)
            if "method" in message:
                self.notifications.append(message)
            if message.get("id") == request_id:
                if "error" in message:
                    if message["error"]["code"] == -32801 and retries < 5:
                        return self.request(method, params, retries + 1)
                    raise RuntimeError(message["error"])
                return message.get("result")
        raise TimeoutError(method)

    def close(self):
        try:
            self.request("shutdown", None)
            self.send("exit", None)
            self.process.wait(timeout=10)
        finally:
            if self.process.poll() is None:
                self.process.kill()
            self.process.stdin.close()
            self.process.stdout.close()


def position(source, offset):
    """Return an LSP UTF-16 position for a Python string offset."""
    before = source[:offset]
    return {"line": before.count("\n"), "character": len(before.rsplit("\n", 1)[-1].encode("utf-16-le")) // 2}


class ViewEditorTests(unittest.TestCase):
    """Assert user-visible semantic results, rather than generated token shapes."""

    @classmethod
    def setUpClass(cls):
        cls.temporary = tempfile.TemporaryDirectory(prefix="leptatui-lsp-")
        project = Path(cls.temporary.name)
        (project / "src").mkdir()
        (project / "Cargo.toml").write_text(
            '[package]\nname = "view-editor-fixture"\nversion = "0.0.0"\nedition = "2024"\n'
            '[dependencies]\nleptatui = { path = ' + json.dumps(str(ROOT / "crates/leptatui")) + ' }\n'
        )
        cls.file = project / "src/main.rs"
        cls.file.write_text(SOURCE)
        cls.extra = project / "src/extra.rs"
        cls.extra.write_text(EXTRA)
        cls.uri = cls.file.as_uri()
        # Reuse the workspace's dependency versions and build the consumer first.
        # Fresh CI machines then spend compilation time outside LSP startup.
        (project / "Cargo.lock").write_text((ROOT / "Cargo.lock").read_text())
        build = subprocess.run(
            ["cargo", "check", "--quiet", "--target-dir", str(ROOT / "target")],
            cwd=project, capture_output=True, text=True, timeout=900,
        )
        if build.returncode:
            raise RuntimeError("Consumer fixture failed to compile:\n" + build.stderr)
        cls.client = Client(project)
        cls.client.send("textDocument/didOpen", {"textDocument": {
            "uri": cls.uri, "languageId": "rust", "version": 0, "text": SOURCE,
        }})
        cls.version = 0

    @classmethod
    def tearDownClass(cls):
        cls.client.close()
        cls.temporary.cleanup()

    def query(self, method, needle, *, source=SOURCE, end=False, **extra):
        type(self).version += 1
        self.client.send("textDocument/didChange", {
            "textDocument": {"uri": self.uri, "version": self.version},
            "contentChanges": [{"text": source}],
        })
        offset = source.index(needle) + (len(needle) if end else 0)
        return self.client.request("textDocument/" + method, {
            "textDocument": {"uri": self.uri}, "position": position(source, offset), **extra,
        })

    def completions(self, needle, source):
        result = self.query("completion", needle, source=source, end=True)
        items = result.get("items") or [] if isinstance(result, dict) else result or []
        return {item["label"].split("(")[0].strip() for item in items}

    def completion_item(self, label, source, needle):
        result = self.query("completion", needle, source=source, end=True)
        items = result.get("items") or [] if isinstance(result, dict) else result or []
        found = [item for item in items if item["label"] == label or item["label"].startswith(label + " ")]
        self.assertTrue(found, "Missing " + label + ": " + repr([item["label"] for item in items]))
        return found[0]

    def apply_completion(self, source, item):
        edits = [item["textEdit"]] + item.get("additionalTextEdits", [])
        def offset(pos):
            lines = source.splitlines(keepends=True) + [""]
            return sum(map(len, lines[:pos["line"]])) + len(
                lines[pos["line"]].encode("utf-16-le")[:pos["character"] * 2].decode("utf-16-le")
            )
        def edit_range(edit):
            return edit.get("range") or edit.get("replace") or edit["insert"]
        result = source
        for edit in sorted(edits, key=lambda edit: offset(edit_range(edit)["start"]), reverse=True):
            text = edit["newText"].replace("$0", "")
            if edit is item["textEdit"] and item.get("insertTextFormat") == 2:
                import re
                text = re.sub(r"\$\{\d+:([^}]*)\}", lambda match: match.group(1), text)
                text = re.sub(r"\\([\\$}])", r"\1", text)
            start, end = (offset(edit_range(edit)[key]) for key in ("start", "end"))
            result = result[:start] + text + result[end:]
        return result

    def test_all_builtin_tag_shapes(self):
        paired = ["Block", "Div", "Form", "Text", "H1", "H2", "H3", "H4", "H5", "H6", "Paragraph", "Link", "A", "Routes", "ParentRoute", "CodeBlock", "OrderedList", "UnorderedList", "ListItem", "Table", "TableHead", "TableBody", "TableRow", "TableCell", "Button"]
        leaves = ["Input", "TextArea", "Image", "ProgressBar", "Markdown", "Outlet", "Route"]
        for name in paired + leaves:
            with self.subTest(component=name):
                source = SOURCE.replace('<Greeting title={person.name.clone()} />', '<' + name)
                item = self.completion_item(name, source, '            <' + name)
                expected = '<' + name + '>$0</' + name + '>' if name in paired else '<' + name + ' $0 />'
                self.assertEqual(item["textEdit"]["newText"], expected)
                result = self.apply_completion(source, item)
                self.assertNotIn("<<", result)

    def test_custom_children_tag_shapes(self):
        for name in ["Panel", "RepeatPanel", "MutPanel", "DefaultPanel"]:
            with self.subTest(component=name):
                source = SOURCE.replace('<Greeting title={person.name.clone()} />', name[:3])
                item = self.completion_item(name, source, '            ' + name[:3])
                self.assertEqual(item["textEdit"]["newText"], '<' + name + '>$0</' + name + '>')
                result = self.apply_completion(source, item)
                result = result.replace('<' + name + '></' + name + '>', '<' + name + '><Text>"Child"</Text></' + name + '>')
                self.check_source(result)

    def test_aliased_tag_snippet(self):
        source = SOURCE.replace('mod extra;', 'mod extra;\nuse extra::Panel as Wrapper;')
        source = source.replace('<Greeting title={person.name.clone()} />', '<Wrap')
        item = self.completion_item("Wrapper", source, "<Wrap")
        self.assertEqual(item["textEdit"]["newText"], "<Wrapper>$0</Wrapper>")

    def test_unsaved_children_metadata(self):
        source = SOURCE.replace('#[prop(into)] title: String,', '#[prop(into)] title: String, children: Children,', 1)
        source = source.replace('<Greeting title={person.name.clone()} />', '<Gree')
        item = self.completion_item("Greeting", source, "<Gree")
        self.assertEqual(item["textEdit"]["newText"], "<Greeting>$0</Greeting>")

    def test_existing_tags_and_attributes(self):
        for original, expected in [
            ('<For />', '<Form></Form>'),
            ('<For>', '<Form></Form>'),
            ('<For class="panel" />', '<Form class="panel"></Form>'),
            ('<For class="panel"><Text>"Nested"</Text></For>', '<Form class="panel"><Text>"Nested"</Text></Form>'),
            ('<For><Text>"Nested"</Text></Form>', '<Form><Text>"Nested"</Text></Form>'),
        ]:
            with self.subTest(original=original):
                source = SOURCE.replace('<Greeting title={person.name.clone()} />', original)
                item = self.completion_item("Form", source, '<For')
                result = self.apply_completion(source, item)
                self.assertIn(expected, result)
                self.assertEqual(result.count('</Form>'), 1)

    def test_resolve_keeps_tag_snippet(self):
        source = SOURCE.replace('<Greeting title={person.name.clone()} />', '<For><Text>"Child"</Text></For>')
        item = self.completion_item("Form", source, "<For")
        resolved = self.client.request("completionItem/resolve", item)
        self.assertEqual(resolved["textEdit"], item["textEdit"])
        self.assertEqual(resolved.get("data"), item.get("data"))
        result = self.apply_completion(source, resolved)
        self.assertIn('<Form><Text>"Child"</Text></Form>', result)
        self.check_source(result)

    def test_lazy_resolve_in_empty_markup(self):
        source = SOURCE.replace('<Greeting title={person.name.clone()} />', '')
        result = self.query("completion", '<Div>\n            ', source=source, end=True)
        items = result.get("items") or [] if isinstance(result, dict) else result or []
        item = next(item for item in items if item["label"] == "Form")
        resolved = self.client.request("completionItem/resolve", item)
        self.assertEqual(resolved["textEdit"]["newText"], "<Form>$0</Form>")
        self.assertIn('<Form></Form>', self.apply_completion(source, resolved))

    def test_lazy_resolve_nonmatching_candidate(self):
        source = SOURCE.replace('<Greeting title={person.name.clone()} />', '<Pan')
        item = self.completion_item("Form", source, "<Pan")
        resolved = self.client.request("completionItem/resolve", item)
        self.assertEqual(resolved["textEdit"]["newText"], "<Form>$0</Form>")
        self.assertIn('<Form></Form>', self.apply_completion(source, resolved))

    def test_rust_expression_types_are_not_tags(self):
        source = SOURCE.replace('<Greeting title={person.name.clone()} />', '{Pane}')
        item = self.completion_item("Panel", source, "{Pane")
        self.assertFalse(item["textEdit"]["newText"].startswith('<'))

    def test_closing_tag_completion_stays_a_name(self):
        source = SOURCE.replace('</Div>', '</Di')
        item = self.completion_item("Div", source, "</Di")
        self.assertEqual(item["textEdit"]["newText"], "Div")

    def test_form_tag_snippet(self):
        source = SOURCE.replace('<Greeting title={person.name.clone()} />', '<For')
        item = self.completion_item("Form", source, "<For")
        self.assertEqual(item["textEdit"]["newText"], "<Form>$0</Form>")
        self.assertEqual(item["insertTextFormat"], 2)

    def test_completion_errors_keep_jsonrpc_error_shape(self):
        request_id = "invalid-completion-test"
        self.client._write({"jsonrpc": "2.0", "id": request_id, "method": "textDocument/completion", "params": {
            "textDocument": {"uri": "not a URI"}, "position": {"line": 0, "character": 0},
        }})
        while True:
            message = self.client.messages.get(timeout=30)
            if message.get("id") == request_id:
                self.assertIn("error", message)
                self.assertNotIn("result", message)
                break
            if "method" in message:
                self.client.notifications.append(message)

    def test_completion_before_existing_tag_name(self):
        source = SOURCE.replace('<Greeting title={person.name.clone()} />', '<  For />')
        item = self.completion_item("Form", source, "< ")
        resolved = self.client.request("completionItem/resolve", item)
        self.assertIn('<Form></Form>', self.apply_completion(source, resolved))

    def test_input_tag_snippet(self):
        source = SOURCE.replace('<Greeting title={person.name.clone()} />', '<Inp')
        item = self.completion_item("Input", source, "<Inp")
        self.assertEqual(item["textEdit"]["newText"], "<Input $0 />")

    def test_bare_component_tag_snippet(self):
        source = SOURCE.replace('<Greeting title={person.name.clone()} />', 'Gree')
        item = self.completion_item("Greeting", source, "            Gree")
        self.assertEqual(item["textEdit"]["newText"], "<Greeting $0 />")

    def test_macro_braces_snippet(self):
        source = SOURCE.replace('fn main() {', 'fn main() {\n    let _ = vie;')
        result = self.query("completion", "fn main() {\n    let _ = vie", source=source, end=True)
        items = result.get("items") or [] if isinstance(result, dict) else result or []
        item = next((item for item in items if item["label"].startswith("view!")), None)
        self.assertIsNotNone(item, repr([item["label"] for item in items]))
        self.assertEqual(item["textEdit"]["newText"], "view! { $0 }")
        result = self.apply_completion(source, item).replace('view! {  }', 'view! { <Text>"Macro"</Text> }')
        self.check_source(result)

    def test_stylesheet_completions(self):
        source = SOURCE.replace('fn main() {', 'fn main() {\n    stylesheet! { .panel { flex_di } };')
        item = self.completion_item("flex_direction", source, "flex_di")
        self.assertEqual(item["textEdit"]["newText"], "flex_direction: $0;")
        resolved = self.client.request("completionItem/resolve", item)
        self.assertEqual(resolved, item)

    def test_stylesheet_macro_braces(self):
        source = SOURCE.replace('fn main() {', 'fn main() {\n    let _ = stylesh;')
        result = self.query("completion", "let _ = stylesh", source=source, end=True)
        items = result.get("items", []) if isinstance(result, dict) else result or []
        item = next(item for item in items if item["label"].startswith("stylesheet!"))
        self.assertEqual(item["textEdit"]["newText"], "stylesheet! { $0 }")

    def test_stylesheet_local_navigation(self):
        source = SOURCE.replace('fn main() {', 'fn main() {\n    stylesheet! { $accent: Color::Red; Text { fg: $accent; } };')
        definition = self.query("definition", "accent;", source=source, end=False)
        self.assertEqual(definition["uri"], self.uri)
        renamed = self.query("rename", "accent;", source=source, newName="primary")
        self.assertEqual(len(renamed["changes"][self.uri]), 2)

    def test_custom_prop_completion(self):
        source = SOURCE.replace('title={person.name.clone()}', 'tit')
        self.assertIn("title", self.completions("<Greeting tit", source))

    def test_builtin_prop_completion(self):
        source = SOURCE.replace('on_press={|| {}}', 'on_')
        self.assertIn("on_press", self.completions("<Button on_", source))

    def test_component_completion(self):
        source = SOURCE.replace('<Greeting title={person.name.clone()} />', '<Gree')
        self.assertIn("Greeting", self.completions("<Gree", source))

    def test_expression_completion(self):
        source = SOURCE.replace('{person.name.clone()}', '{person.na}', 1)
        self.assertIn("name", self.completions("<Greeting title={person.na", source))

    def test_prop_hover(self):
        result = self.query("hover", "title={person")
        self.assertIn("displayed title", json.dumps(result))

    def test_component_definition(self):
        result = self.query("definition", "Greeting title=")
        self.assertTrue(result)
        self.assertIn(self.uri, json.dumps(result))

    def test_closing_tag_definition(self):
        result = self.query("definition", "Div>\n    }")
        self.assertTrue(result)

    def renamed(self, needle, new_name):
        result = self.query("rename", needle, newName=new_name)
        self.assertTrue(result)
        by_uri = result.get("changes", {})
        for change in result.get("documentChanges", []):
            by_uri.setdefault(change["textDocument"]["uri"], []).extend(change["edits"])
        self.renamed_documents = {}
        for uri, edits in by_uri.items():
            original = SOURCE if uri == self.uri else EXTRA
            def offset(pos):
                lines = original.splitlines(keepends=True)
                return sum(map(len, lines[:pos["line"]])) + len(
                    lines[pos["line"]].encode("utf-16-le")[:pos["character"] * 2].decode("utf-16-le")
                )
            source = original
            for edit in sorted(edits, key=lambda edit: offset(edit["range"]["start"]), reverse=True):
                start, end = (offset(edit["range"][key]) for key in ("start", "end"))
                source = source[:start] + edit["newText"] + source[end:]
            self.renamed_documents[uri] = source
        return self.renamed_documents[self.uri]

    def check_source(self, source):
        self.file.write_text(source)
        if hasattr(self, "renamed_documents"):
            self.extra.write_text(self.renamed_documents.get(self.extra.as_uri(), EXTRA))
        try:
            result = subprocess.run(
                ["cargo", "check", "--offline", "--quiet", "--target-dir", str(ROOT / "target")],
                cwd=self.file.parent.parent, capture_output=True, text=True, timeout=180,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
        finally:
            self.file.write_text(SOURCE)
            self.extra.write_text(EXTRA)

    def test_prop_rename(self):
        source = self.renamed("title={person", "caption")
        self.assertIn("caption={person", source)
        self.assertIn("caption: String", source)
        self.assertIn("caption.get()", source)
        self.assertIn('.caption("Hello")', source)
        self.assertIn('.title("Footer")', source)
        self.assertIn('caption="Extra"', self.renamed_documents[self.extra.as_uri()])
        self.assertIn('.caption("Builder")', self.renamed_documents[self.extra.as_uri()])
        self.check_source(source)

    def test_component_rename(self):
        source = self.renamed("Greeting title=", "Welcome")
        self.assertIn("fn Welcome(", source)
        self.assertIn("<Welcome title=", source)
        self.assertIn("WelcomeProps::builder()", source)
        self.assertIn('</Welcome>', self.renamed_documents[self.extra.as_uri()])
        self.check_source(source)

    def test_empty_prop_completion(self):
        source = SOURCE.replace('title={person.name.clone()}', '')
        self.assertIn("title", self.completions("<Greeting ", source))

    def test_empty_builtin_prop_completion(self):
        source = SOURCE.replace('on_press={|| {}}', '')
        self.assertIn("on_press", self.completions("<Button ", source))

    def test_prop_references(self):
        result = self.query("references", "title={person", context={"includeDeclaration": False})
        self.assertGreaterEqual(len(result or []), 5)
        self.assertTrue(any(location["uri"] == self.extra.as_uri() for location in result))

    def test_unicode_completion_position(self):
        source = SOURCE.replace('<Greeting title={person.name.clone()} />', '"🙂 café" <Greeting tit />')
        self.assertIn("title", self.completions("<Greeting tit", source))

    def test_route_prop_completion(self):
        source = SOURCE.replace('<Greeting title={person.name.clone()} />', '<Routes fallback={|| text("Missing")}><Route pa /></Routes>')
        labels = self.completions("<Route pa", source)
        self.assertIn("path", labels)
        self.assertIn("view", labels)
        self.assertNotIn("class", labels)

    def test_builtin_tag_completion(self):
        source = SOURCE.replace('<Greeting title={person.name.clone()} />', '<Butt')
        self.assertIn("Button", self.completions("<Butt", source))

    def test_bare_dot_completion(self):
        source = SOURCE.replace('{person.name.clone()}', '{person.}', 1)
        self.assertIn("name", self.completions("<Greeting title={person.", source))

    def test_second_prop_completion(self):
        source = SOURCE.replace('on_press={|| {}}', 'class="primary" on_')
        self.assertIn("on_press", self.completions('<Button class="primary" on_', source))

    def wait_diagnostics(self):
        deadline = time.monotonic() + 30
        while time.monotonic() < deadline:
            for notification in self.client.notifications:
                if notification.get("method") == "textDocument/publishDiagnostics":
                    params = notification["params"]
                    if params.get("uri") == self.uri and params.get("version") == self.version:
                        return params["diagnostics"]
            try:
                self.client.notifications.append(self.client.messages.get(timeout=1))
            except queue.Empty:
                pass
        self.fail("No diagnostics received after changing the diagnostic result")

    def test_valid_document_has_no_semantic_errors(self):
        # Change the diagnostic result before returning to valid source: the
        # server deliberately suppresses repeated identical empty reports.
        source = SOURCE.replace("<Button on_press", "<Button wrong")
        self.query("hover", "title={person", source=source)
        errors = [item for item in self.wait_diagnostics() if item.get("severity") == 1]
        self.assertTrue(errors)
        self.assertTrue(any("attribute" in item["message"] for item in errors))
        self.query("hover", "title={person")
        self.assertEqual([item for item in self.wait_diagnostics() if item.get("severity") == 1], [])

    def test_builtin_rename_is_rejected(self):
        try:
            result = self.query("rename", "Button on_press", newName="Action")
        except RuntimeError:
            return
        self.assertFalse(result)

    def test_aliased_component(self):
        source = SOURCE.replace('use widgets::{Greeting, GreetingProps};', 'use widgets::{Greeting as Hello, GreetingProps};')
        source = source.replace('<Greeting ', '<Hello ').replace('Greeting::with_props', 'Hello::with_props')
        result = self.query("hover", "title={person", source=source)
        self.assertIn("displayed title", json.dumps(result))
        self.check_source(source)

    def test_component_references(self):
        result = self.query("references", "Greeting title=", context={"includeDeclaration": True})
        self.assertGreaterEqual(len(result or []), 3)

    def test_sibling_survives_error(self):
        source = SOURCE.replace('<Button on_press={|| {}}>', '<Button wrong=1>')
        result = self.query("hover", "other.name.clone()}</Text>", source=source)
        self.assertTrue(result)


if __name__ == "__main__":
    unittest.main(verbosity=2)
