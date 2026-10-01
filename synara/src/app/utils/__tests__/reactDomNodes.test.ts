import assert from 'node:assert/strict';
import { test } from 'node:test';
import { CDATA, Document, Element, Text } from 'domhandler';
import { reactDomNodes } from '../reactDomNodes';

test('React DOM adapter preserves text inside CDATA and document containers', () => {
  const text = new Text('kept literal <code>');
  const element = new Element('strong', {}, [new Text('bold')]);
  assert.deepEqual(reactDomNodes([new Document([new CDATA([text]), element])]), [text, element]);
});
