#!/usr/bin/env python3
"""Exercise stylesheet formatting, upstream composition, and request lifecycle."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import threading
import time
import unittest

import test_view as harness


class FormattingTests(unittest.TestCase):
    """Check formatted source and compilable edits through a real rust-analyzer."""

    setUpClass = classmethod(harness.ViewEditorTests.setUpClass.__func__)
    tearDownClass = classmethod(harness.ViewEditorTests.tearDownClass.__func__)
    apply_completion = harness.ViewEditorTests.apply_completion
    check_source = harness.ViewEditorTests.check_source

    def formatted(self, source, *, tab_size=4, spaces=True):
        type(self).version += 1
        self.client.send('textDocument/didChange', {
            'textDocument': {'uri': self.uri, 'version': self.version},
            'contentChanges': [{'text': source}],
        })
        edits = self.client.request('textDocument/formatting', {
            'textDocument': {'uri': self.uri},
            'options': {'tabSize': tab_size, 'insertSpaces': spaces},
        }) or []
        after = self.apply_completion(source, {'textEdit': edits[0], 'additionalTextEdits': edits[1:]}) if edits else source
        return after, edits

    def test_rust_and_stylesheet_compose(self):
        source = harness.SOURCE.replace('fn main() {', 'fn main(){let number=1+2; let _=number; stylesheet!{H1{fg:Color::Cyan;} H2{fg:Color::Blue;} .shell{padding:TuiSpacing::uniform( 1 ); @media(max-width:60){display:Display::Flex;}}};')
        after, edits = self.formatted(source)
        self.assertTrue(edits)
        self.assertIn('let number = 1 + 2;', after)
        self.assertIn('    stylesheet! {\n        H1 { fg: Color::Cyan; }\n\n        H2 { fg: Color::Blue; }', after)
        self.assertIn('            @media (max-width: 60) {\n                display: Display::Flex;\n            }', after)
        self.check_source(after)
        self.assertEqual(self.formatted(after)[1], [])

    def test_alias_comments_and_values(self):
        source = harness.SOURCE + '''
use leptatui::stylesheet as styles;
fn local_styles(){styles!{@mixin chrome{fg:Color::Cyan;} H1{
// Keep this note.
@include chrome; padding:TuiSpacing::new(1,2,3,4); // Keep this too.
}};}
'''
        after, _ = self.formatted(source)
        self.assertIn('// Keep this note.', after)
        self.assertIn('padding: TuiSpacing::new(1, 2, 3, 4); // Keep this too.', after)
        self.assertIn('@mixin chrome {\n            fg: Color::Cyan;\n        }', after)
        self.check_source(after)
        self.assertEqual(self.formatted(after)[1], [])

    def test_invalid_macro_body_is_preserved(self):
        source = harness.SOURCE.replace('fn main() {', 'fn main() { stylesheet! { H1 { fg: Color::Red } };')
        after, _ = self.formatted(source)
        self.assertIn('stylesheet! { H1 { fg: Color::Red } }', after)

    def test_editor_indentation(self):
        source = harness.SOURCE.replace('fn main() {', 'fn main() { stylesheet! { H1{fg:Color::Cyan;bg:Color::Black;} };')
        after, _ = self.formatted(source, tab_size=2)
        # The containing Rust indentation belongs to rustfmt; stylesheet levels
        # follow the editor's requested two-space indentation.
        self.assertIn('    stylesheet! {\n      H1 {\n        fg: Color::Cyan;\n        bg: Color::Black;\n      }\n    }', after)
        self.check_source(after)


@unittest.skipUnless(shutil.which('leptosfmt'), 'leptosfmt is optional outside the Neovim setup')
class MixedFormattingTests(unittest.TestCase):
    """Exercise the user's external view formatter alongside stylesheet formatting."""

    tearDownClass = classmethod(harness.ViewEditorTests.tearDownClass.__func__)
    formatted = FormattingTests.formatted
    apply_completion = harness.ViewEditorTests.apply_completion
    check_source = harness.ViewEditorTests.check_source

    @classmethod
    def setUpClass(cls):
        harness.ViewEditorTests.setUpClass.__func__(cls)
        cls.client.close()
        cls.client = harness.Client(cls.file.parent.parent, rustfmt_command=['leptosfmt', '--stdin', '--rustfmt'])
        cls.client.send('textDocument/didOpen', {'textDocument': {
            'uri': cls.uri, 'languageId': 'rust', 'version': 0, 'text': harness.SOURCE,
        }})
        cls.version = 0

    def test_view_rust_and_stylesheet_pipeline(self):
        source = harness.SOURCE + '''
fn formatted_view()->impl IntoView{stylesheet!{H1{fg:Color::Cyan;}};view!{<Div><Text>"Hello"</Text><Button>"Go"</Button></Div>}}
'''
        after, _ = self.formatted(source)
        self.assertIn('fn formatted_view() -> impl IntoView {', after)
        self.assertIn('stylesheet! {\n        H1 { fg: Color::Cyan; }\n    }', after)
        self.assertIn('view! {\n        <Div>', after)
        self.check_source(after)
        self.assertEqual(self.formatted(after)[1], [])


# This server makes lifecycle tests independent of rust-analyzer indexing timing.
FAKE_ANALYZER = '''#!/usr/bin/env python3
import json, sys

def send(message):
    payload = json.dumps(message).encode()
    sys.stdout.buffer.write(f"Content-Length: {len(payload)}\\r\\n\\r\\n".encode() + payload)
    sys.stdout.buffer.flush()

while True:
    headers = {}
    while line := sys.stdin.buffer.readline():
        if line == b"\\r\\n": break
        key, value = line.decode().split(":", 1)
        headers[key.lower()] = value.strip()
    if not headers: break
    message = json.loads(sys.stdin.buffer.read(int(headers['content-length'])))
    method = message.get('method')
    if method == 'exit': break
    if method == 'initialized':
        send({'jsonrpc':'2.0','method':'experimental/serverStatus','params':{'quiescent':True}})
    if 'id' in message:
        result = {'capabilities':{'documentFormattingProvider':True}} if method == 'initialize' else [] if method == 'textDocument/formatting' else None
        send({'jsonrpc':'2.0','id':message['id'],'result':result})
'''


class FormattingLifecycleTests(unittest.TestCase):
    """Hold the worker at rustfmt configuration discovery and send client events."""

    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix='leptatui-format-lifecycle-')
        self.project = Path(self.temporary.name)
        self.file = self.project / 'main.rs'
        self.uri = self.file.as_uri()
        self.source = 'fn main() { stylesheet! { H1 { fg: Color::Cyan; } } }'
        self.file.write_text(self.source)
        self.ready = self.project / 'ready'
        self.release = self.project / 'release'
        analyzer = self.project / 'analyzer'
        analyzer.write_text(FAKE_ANALYZER)
        analyzer.chmod(0o755)
        real_rustfmt = shutil.which('rustfmt')
        shim = self.project / 'rustfmt'
        shim.write_text('#!/usr/bin/env python3\nimport os, pathlib, time\n'
                        + 'root = pathlib.Path(' + repr(str(self.project)) + ')\n'
                        + '(root / "ready").touch()\n'
                        + 'deadline = time.monotonic() + 10\n'
                        + 'while not (root / "release").exists() and time.monotonic() < deadline: time.sleep(0.01)\n'
                        + 'os.execv(' + repr(real_rustfmt) + ', [' + repr(real_rustfmt) + '] + __import__("sys").argv[1:])\n')
        shim.chmod(0o755)
        env = dict(os.environ, RUST_ANALYZER=str(analyzer), PATH=str(self.project) + os.pathsep + os.environ['PATH'])
        self.client = harness.Client(self.project, env=env)
        self.client.send('textDocument/didOpen', {'textDocument': {
            'uri': self.uri, 'languageId': 'rust', 'version': 0, 'text': self.source,
        }})

    def tearDown(self):
        self.release.touch()
        self.client.close()
        self.temporary.cleanup()

    def interrupted_format(self, event):
        outcome = []
        def request():
            try:
                outcome.append(self.client.request('textDocument/formatting', {
                    'textDocument': {'uri': self.uri}, 'options': {'tabSize': 4, 'insertSpaces': True},
                }, retries=5))
            except Exception as error:
                outcome.append(error)
        thread = threading.Thread(target=request, daemon=True)
        thread.start()
        deadline = time.monotonic() + 5
        while not self.ready.exists() and time.monotonic() < deadline:
            time.sleep(0.01)
        self.assertTrue(self.ready.exists(), 'formatter worker did not reach rustfmt')
        event()
        thread.join(5)
        self.assertFalse(thread.is_alive(), 'client event was blocked by the formatter subprocess')
        self.assertEqual(len(outcome), 1)
        self.assertIsInstance(outcome[0], RuntimeError)
        return str(outcome[0])

    def test_cancellation_while_worker_runs(self):
        error = self.interrupted_format(lambda: self.client.send('$/cancelRequest', {'id': self.client.sequence}))
        self.assertIn('-32800', error)

    def test_document_change_rejects_stale_edits(self):
        error = self.interrupted_format(lambda: self.client.send('textDocument/didChange', {
            'textDocument': {'uri': self.uri, 'version': 1}, 'contentChanges': [{'text': self.source + '\n'}],
        }))
        self.assertIn('-32801', error)


if __name__ == '__main__':
    unittest.main(verbosity=2)
