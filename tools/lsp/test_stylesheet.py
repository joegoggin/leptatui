#!/usr/bin/env python3
"""Exercise stylesheet semantics through the proxy and a real rust-analyzer."""
import unittest
import test_view as harness

SOURCE = harness.SOURCE
harness.EXTRA += '''
pub mod alternate {
    pub fn palette() -> leptatui::StyleModule {
        leptatui::stylesheet! { $different: leptatui::Color::Blue; }
    }
}

pub fn dynamic_palette() -> leptatui::StyleModule { palette().variable("runtime", leptatui::Color::Blue) }

pub fn palette() -> leptatui::StyleModule {
    leptatui::stylesheet! {
        $accent: leptatui::Color::Red;
        @mixin chrome { fg: $accent; }
    }
}
'''


class StylesheetEditorTests(unittest.TestCase):
    """Check source-visible completion edits and semantic associations."""

    setUpClass = classmethod(harness.ViewEditorTests.setUpClass.__func__)
    tearDownClass = classmethod(harness.ViewEditorTests.tearDownClass.__func__)
    query = harness.ViewEditorTests.query
    completions = harness.ViewEditorTests.completions
    completion_item = harness.ViewEditorTests.completion_item
    apply_completion = harness.ViewEditorTests.apply_completion
    check_source = harness.ViewEditorTests.check_source

    def styled(self, stylesheet):
        return SOURCE.replace('fn main() {', 'fn main() {\n    ' + stylesheet + '\n')

    def test_imported_variable_completion(self):
        source = self.styled('stylesheet! { @use extra::palette as colors; Text { fg: colors.$ac } };')
        item = self.completion_item('accent', source, 'colors.$ac')
        self.assertEqual(item['textEdit']['newText'], 'accent')

    def test_imported_mixin_completion(self):
        source = self.styled('stylesheet! { @use extra::palette as colors; Text { @include colors.ch } };')
        item = self.completion_item('chrome', source, 'colors.ch')
        self.assertEqual(item['textEdit']['newText'], 'chrome')

    def test_imported_definition_and_rename(self):
        source = self.styled('stylesheet! { @use extra::palette as colors; Text { fg: colors.$accent; } };')
        target = self.query('definition', 'accent;', source=source)
        self.assertEqual(target['uri'], self.extra.as_uri())
        renamed = self.query('rename', 'accent;', source=source, newName='primary')
        self.assertEqual(len(renamed['changes'][self.uri]), 1)
        self.assertEqual(len(renamed['changes'][self.extra.as_uri()]), 2)
        self.renamed_documents = {}
        for uri, edits in renamed['changes'].items():
            text = source if uri == self.uri else harness.EXTRA
            self.renamed_documents[uri] = self.apply_completion(text, {'textEdit': edits[0], 'additionalTextEdits': edits[1:]})
        self.check_source(self.renamed_documents[self.uri])

    def test_rust_enum_completion(self):
        source = self.styled('stylesheet! { Text { fg: Color:: } };')
        self.assertIn('Red', self.completions('fg: Color::', source))

    def test_rust_variable_initializer_completion(self):
        source = self.styled('stylesheet! { $accent: Color::; Text { fg: $accent; } };')
        self.assertIn('Red', self.completions('$accent: Color::', source))

    def test_rust_struct_field_completion(self):
        source = self.styled('stylesheet! { Text { padding: TuiSpacing { le }; } };')
        self.assertIn('left', self.completions('TuiSpacing { le', source))

    def test_rust_value_hover(self):
        source = self.styled('stylesheet! { Text { fg: Color::Red; } };')
        result = self.query('hover', 'Color::Red;', source=source)
        self.assertIsNotNone(result)

    def test_property_hover_and_definition(self):
        source = self.styled('stylesheet! { Text { flex_direction: FlexDirection::Column; } };')
        result = self.query('hover', 'flex_direction:', source=source)
        self.assertIn('FlexDirection', str(result))
        result = self.query('definition', 'flex_direction:', source=source)
        self.assertTrue(result)

    def test_alias_macro_completion(self):
        source = SOURCE.replace('use leptatui::prelude::*;', 'use leptatui::prelude::*;\nuse leptatui::stylesheet as styles;', 1)
        source = source.replace('fn main() {', 'fn main() { let _ = styl;')
        result = self.query('completion', 'let _ = styl', source=source, end=True)
        items = result.get('items', []) if isinstance(result, dict) else result or []
        item = next(item for item in items if item['label'].startswith('styles!'))
        resolved = self.client.request('completionItem/resolve', item)
        self.assertEqual(resolved['textEdit']['newText'], 'styles! { $0 }')

    def test_class_references_and_bem_rename(self):
        source = SOURCE.replace('<Div>', '<Div class="panel"><Text class="panel__title">"Title"</Text>', 1)
        source = source.replace('fn main() {', 'fn main() { stylesheet! { .panel { &__title { fg: Color::Red; } } };')
        refs = self.query('references', '.panel', source=source, end=True, context={'includeDeclaration': True})
        self.assertEqual(len(refs), 2)
        rename = self.query('rename', '.panel', source=source, end=True, newName='card')
        self.assertEqual(len(rename['changes'][self.uri]), 3)
        self.assertEqual({e['newText'] for e in rename['changes'][self.uri]}, {'card', 'card__title'})
        edits = rename['changes'][self.uri]
        self.check_source(self.apply_completion(source, {'textEdit': edits[0], 'additionalTextEdits': edits[1:]}))

    def test_class_and_id_namespaces_are_separate(self):
        source = self.styled('stylesheet! { .panel { fg: Color::Red; } #panel { fg: Color::Blue; } };')
        refs = self.query('references', '.panel', source=source, end=True, context={'includeDeclaration': True})
        self.assertEqual(len(refs), 1)

    def test_workspace_class_completion(self):
        source = SOURCE.replace('<Div>', '<Div class="panel">', 1)
        source = source.replace('fn main() {', 'fn main() { stylesheet! { .pa };')
        item = self.completion_item('panel', source, '.pa')
        self.assertEqual(item['textEdit']['newText'], 'panel')

    def test_literal_class_attribute_completion(self):
        source = SOURCE.replace('<Div>', '<Div class="pa">', 1)
        source = source.replace('fn main() {', 'fn main() { stylesheet! { .panel { fg: Color::Red; } };')
        item = self.completion_item('panel', source, 'class="pa')
        self.assertEqual(item['textEdit']['newText'], 'panel')

    def test_nested_and_directive_completion(self):
        cases = [
            ('stylesheet! { @mi };', '@mi', 'mixin'),
            ('stylesheet! { .panel { @in } };', '@in', 'include'),
            ('stylesheet! { .panel { &:fo } };', '&:fo', 'focus'),
            ('stylesheet! { @media (max-wi) {} };', 'max-wi', 'max-width'),
            ('stylesheet! { @mixin chrome { pad } };', 'chrome { pad', 'padding'),
        ]
        for macro, needle, expected in cases:
            with self.subTest(macro=macro):
                self.assertIn(expected, self.completions(needle, self.styled(macro)))

    def test_macro_local_symbol_scopes(self):
        source = self.styled('stylesheet! { $accent: Color::Red; Text { fg: $accent; } }; stylesheet! { $accent: Color::Blue; Text { fg: $accent; } };')
        refs = self.query('references', 'fg: $accent', source=source, end=True, context={'includeDeclaration': True})
        self.assertEqual(len(refs), 2)

    def test_value_completions_include_stylesheet_names(self):
        source = self.styled('stylesheet! { @use extra::palette as colors; $accent: Color::Red; Text { fg:  } };')
        items = self.completions('fg: ', source)
        self.assertIn('$accent', items)
        self.assertIn('colors', items)

    def test_implicit_module_namespace_rename(self):
        source = self.styled('stylesheet! { @use extra::palette; Text { fg: palette.$accent; } };')
        renamed = self.query('rename', 'fg: palette', source=source, end=True, newName='colors')
        edits = renamed['changes'][self.uri]
        self.assertEqual({e['newText'] for e in edits}, {' as colors', 'colors'})

    def test_helper_rename_updates_implicit_namespace(self):
        source = self.styled('stylesheet! { @use extra::palette; Text { fg: palette.$accent; } };')
        renamed = self.query('rename', 'extra::palette', source=source, end=True, newName='color_tokens')
        self.assertEqual(len(renamed['changes'][self.uri]), 2)
        self.assertTrue(all(e['newText'] == 'color_tokens' for e in renamed['changes'][self.uri]))

    def test_invalid_bem_parent_rename_is_rejected(self):
        source = self.styled('stylesheet! { .panel { &__title { fg: Color::Red; } } };')
        with self.assertRaisesRegex(RuntimeError, 'BEM parent'):
            self.query('rename', '__title', source=source, end=True, newName='card__title')

    def test_completion_resolves_after_typing_more_characters(self):
        source = self.styled('stylesheet! { Text { flex_di } };')
        item = self.completion_item('flex_direction', source, 'flex_di')
        self.query('hover', 'App', source=source.replace('flex_di', 'flex_dir'))
        for _ in range(2):
            self.assertEqual(self.client.request('completionItem/resolve', item), item)

    def test_completion_resolves_after_another_document_changes(self):
        source = self.styled('stylesheet! { Text { flex_di } };')
        item = self.completion_item('flex_direction', source, 'flex_di')
        self.client.send('textDocument/didOpen', {'textDocument': {
            'uri': self.extra.as_uri(), 'languageId': 'rust', 'version': 0,
            'text': harness.EXTRA,
        }})
        try:
            self.client.send('textDocument/didChange', {
                'textDocument': {'uri': self.extra.as_uri(), 'version': 1},
                'contentChanges': [{'text': harness.EXTRA + '\n// Unsaved edit\n'}],
            })
            self.assertEqual(self.client.request('completionItem/resolve', item), item)
        finally:
            self.client.send('textDocument/didClose', {
                'textDocument': {'uri': self.extra.as_uri()},
            })

    def test_local_diagnostics(self):
        self.client.notifications.clear()
        source = self.styled('stylesheet! { Text { unknown: 1; fg: $missing; } };')
        self.query('hover', 'unknown', source=source)
        messages = [str(d) for n in self.client.notifications
                    if n.get('method') == 'textDocument/publishDiagnostics'
                    for d in n['params']['diagnostics']]
        self.assertTrue(any('unsupported stylesheet declaration' in m for m in messages))
        self.assertTrue(any('unknown stylesheet variable' in m for m in messages))

    def test_unknown_static_import_member_is_diagnosed(self):
        self.client.notifications.clear()
        source = self.styled('stylesheet! { @use extra::palette as colors; Text { fg: colors.$missing; } };')
        self.query('definition', 'missing', source=source)
        messages = [str(d) for n in self.client.notifications
                    if n.get('method') == 'textDocument/publishDiagnostics'
                    for d in n['params']['diagnostics']]
        self.assertTrue(any('unknown imported stylesheet variable' in m for m in messages))

    def test_runtime_exports_are_not_reported_missing(self):
        self.client.notifications.clear()
        source = self.styled('stylesheet! { @use extra::dynamic_palette as colors; Text { fg: colors.$runtime; } };')
        self.query('definition', 'runtime;', source=source)
        messages = [str(d) for n in self.client.notifications
                    if n.get('method') == 'textDocument/publishDiagnostics'
                    for d in n['params']['diagnostics']]
        self.assertFalse(any('unknown imported' in m for m in messages))

    def test_custom_selector_completion(self):
        source = self.styled('stylesheet! { Gree };')
        self.assertIn('Greeting', self.completions('stylesheet! { Gree', source))


    def test_import_resolution_distinguishes_same_named_helpers(self):
        source = self.styled('stylesheet! { @use extra::alternate::palette as colors; Text { fg: colors.$ } };')
        items = self.completions('colors.$', source)
        self.assertIn('different', items)
        self.assertNotIn('accent', items)

    def test_escaped_literal_class_rename(self):
        source = SOURCE.replace('<Div>', r'<Div class="pan\u{65}l">', 1)
        source = source.replace('fn main() {', 'fn main() { stylesheet! { .panel { fg: Color::Red; } };')
        renamed = self.query('rename', '.panel', source=source, newName='card')
        edits = renamed['changes'][self.uri]
        self.assertEqual(len(edits), 2)
        self.assertIn('class="card"', self.apply_completion(source, {'textEdit': edits[0], 'additionalTextEdits': edits[1:]}))

    def test_completion_preserves_existing_semicolon(self):
        source = self.styled('stylesheet! { Text { flex_di; } };')
        item = self.completion_item('flex_direction', source, 'flex_di')
        self.assertEqual(item['textEdit']['newText'], 'flex_direction: $0')
        applied = self.apply_completion(source, item).replace('flex_direction: ;', 'flex_direction: FlexDirection::Column;')
        self.check_source(applied)



if __name__ == '__main__':
    unittest.main(verbosity=2)
