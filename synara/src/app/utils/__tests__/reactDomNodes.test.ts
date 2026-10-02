import assert from 'node:assert/strict';
import { test } from 'node:test';
import { CDATA, Document, Element, Text } from 'domhandler';
import { reactDomNodes } from '../reactDomNodes';

test('React DOM adapter preserves text inside CDATA and document containers', () => {
  const text = new Text('kept literal <code>');
  const element = new Element('strong', {}, [new Text('bold')]);
  const nodes = reactDomNodes([new Document([new CDATA([text]), element])]);
  assert.equal(nodes.length, 2);
  assert.equal(nodes[0].type, 'text');
  assert.equal((nodes[0] as Text).data, text.data);
  assert.equal((nodes[1] as Element).children[0].parent, nodes[1]);
  assert.equal(nodes[0].next, nodes[1]);
  assert.equal(nodes[1].prev, nodes[0]);
});

test('React DOM adapter normalizes nested containers without changing source nodes', () => {
  const content = new Text('nested literal <kept>');
  const wrapper = new CDATA([content]);
  const element = new Element('strong', {}, [wrapper]);
  const [converted] = reactDomNodes([element]) as Element[];
  assert.notEqual(converted, element);
  assert.equal(converted.children[0].type, 'text');
  assert.equal((converted.children[0] as Text).data, content.data);
  assert.equal(converted.children[0].parent, converted);
  assert.equal(element.children[0], wrapper);
});

test('React DOM adapter keeps the enclosing element context for recursive parser callbacks', () => {
  const code = new Element('code', {}, [new Text('hello')]);
  const pre = new Element('pre', {}, [code]);
  code.parent = pre;
  const [converted] = reactDomNodes(pre.children);
  assert.equal(converted.parent, pre);
});
