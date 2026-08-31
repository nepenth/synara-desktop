import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createEditor, Descendant, Element, Text } from 'slate';
import { withHistory } from 'slate-history';
import {
  clipboardDataToEditorInput,
  ClipboardInsertResult,
  htmlToEditorInput,
  insertClipboardData,
  MAX_EDITOR_CLIPBOARD_CHARS,
  shouldPreventDefaultForClipboardInsert,
} from '../input';
import { indentListItem, outdentListItem, toggleKeyboardShortcut } from '../keyboard';
import {
  stripEditorMetadataFromCustomHtml,
  toMatrixCustomHTML,
  toPlainText,
  trimCustomHtml,
} from '../output';
import { BlockType } from '../types';
import { toggleBlock } from '../utils';
import { sanitizeCustomHtml } from '../../../utils/sanitize';
import { MATRIX_HTML_PROFILE } from '../../../utils/matrixHtmlProfile';

test('ordered list output preserves start attribute and plain-text numbering', () => {
  const nodes: Descendant[] = [
    {
      type: BlockType.OrderedList,
      start: 10,
      children: [
        { type: BlockType.ListItem, children: [{ text: 'alpha' }] },
        { type: BlockType.ListItem, children: [{ text: 'beta' }] },
      ],
    },
  ];

  assert.equal(
    trimCustomHtml(toMatrixCustomHTML(nodes, { allowTextFormatting: true })),
    '<ol start="10"><li><p>alpha</p></li><li><p>beta</p></li></ol>'
  );
  assert.equal(toPlainText(nodes, false).trim(), '10. alpha\n11. beta');
});

test('formatted ordered list input preserves start attribute in Slate', () => {
  const [node] = htmlToEditorInput('<ol start="10"><li>alpha</li><li>beta</li></ol>');

  assert.equal(Element.isElement(node), true);
  if (!Element.isElement(node)) throw new Error('Expected element');
  assert.equal(node.type, BlockType.OrderedList);
  if (node.type !== BlockType.OrderedList) throw new Error('Expected ordered list');
  assert.equal(node.start, 10);
  assert.equal(node.children.length, 2);
});

test('markdown ordered list supports multi-digit starts', () => {
  const nodes: Descendant[] = [
    { type: BlockType.Paragraph, children: [{ text: '10. alpha' }] },
    { type: BlockType.Paragraph, children: [{ text: '11. beta' }] },
  ];

  assert.equal(
    trimCustomHtml(
      toMatrixCustomHTML(nodes, {
        allowTextFormatting: true,
        allowBlockMarkdown: true,
        allowInlineMarkdown: true,
      })
    ),
    '<ol start="10"><li><p>alpha</p></li><li><p>beta</p></li></ol>'
  );
});

test('spoiler plain-text fallback does not reveal hidden text', () => {
  const nodes = [
    {
      type: BlockType.Paragraph,
      children: [{ text: 'hidden launch plan', spoiler: true }],
    },
  ] as unknown as Descendant[];

  assert.equal(toPlainText(nodes, false).trim(), '[spoiler]');
  assert.equal(
    trimCustomHtml(toMatrixCustomHTML(nodes, { allowTextFormatting: true })),
    '<span data-mx-spoiler>hidden launch plan</span>'
  );
});

test('rich links serialize visible labels in html and plain text', () => {
  const nodes: Descendant[] = [
    {
      type: BlockType.Paragraph,
      children: [
        { text: 'Read ' },
        {
          type: BlockType.Link,
          href: 'https://example.com/path?a=1&b=2',
          children: [{ text: 'the spec' }],
        },
      ],
    },
  ];

  assert.equal(
    trimCustomHtml(toMatrixCustomHTML(nodes, { allowTextFormatting: true })),
    'Read <a href="https://example.com/path?a=1&amp;b=2">the spec</a>'
  );
  assert.equal(
    toPlainText(nodes, false).trim(),
    'Read [the spec](https://example.com/path?a=1&b=2)'
  );
});

test('outbound Matrix HTML strips Synara-only editor metadata', () => {
  const customHtml =
    '<blockquote data-md=">"><span data-md="**">hello</span></blockquote><ol data-md="10" start="10"><li><p>world</p></li></ol>';

  assert.equal(
    stripEditorMetadataFromCustomHtml(customHtml),
    '<blockquote><span>hello</span></blockquote><ol start="10"><li><p>world</p></li></ol>'
  );
});

test('sanitizer follows the documented Matrix HTML profile with legacy inbound tolerance', () => {
  assert.equal(MATRIX_HTML_PROFILE.version, '2026-05-19');
  assert.equal((MATRIX_HTML_PROFILE.emittedTags as readonly string[]).includes('font'), false);
  assert.equal(MATRIX_HTML_PROFILE.allowedTags.includes('font'), true);

  const sanitized = sanitizeCustomHtml(
    '<mx-reply><blockquote>quoted</blockquote></mx-reply><script>alert(1)</script><span data-md="**" data-mx-spoiler>hidden</span><a href="javascript:alert(1)">bad</a><a href="https://example.com">ok</a>'
  );

  assert.doesNotMatch(sanitized, /mx-reply|script|javascript/);
  assert.match(sanitized, /data-md="\*\*"/);
  assert.match(sanitized, /data-mx-spoiler/);
  assert.match(sanitized, /href="https:\/\/example\.com"/);
  assert.match(sanitized, /rel="noreferrer noopener"/);
});

test('sanitizer preserves the Hermes approval HTML vocabulary', () => {
  const hermesApprovalHtml = [
    '<p>⚠️ <strong>Dangerous command requires approval</strong></p>',
    '<pre><code>rm -rf /tmp/example\n</code></pre>',
    '<p>Reason: destructive command</p>',
    '<p>Reply <code>!approve</code> to execute, <code>!approve session</code> to approve this pattern for the session, <code>!approve always</code> to approve permanently, or <code>!deny</code> to cancel.</p>',
    '<p>You can also react to this prompt:<br>\n✅ = approve once<br>\n♾️ = approve always<br>\n❌ = deny</p>',
  ].join('\n');

  // DOMPurify serializes HTML void elements as XHTML-style `<br />`; the
  // change is byte-level only and preserves the Matrix presentation.
  assert.equal(
    sanitizeCustomHtml(hermesApprovalHtml),
    hermesApprovalHtml.replaceAll('<br>', '<br />')
  );
});

test('nested lists round-trip through Matrix HTML and plain-text fallback', () => {
  const nodes: Descendant[] = [
    {
      type: BlockType.OrderedList,
      children: [
        {
          type: BlockType.ListItem,
          children: [
            { text: 'alpha' },
            {
              type: BlockType.UnorderedList,
              children: [
                {
                  type: BlockType.ListItem,
                  children: [{ text: 'nested' }],
                },
              ],
            },
          ],
        },
        {
          type: BlockType.ListItem,
          children: [{ text: 'beta' }],
        },
      ],
    },
  ];

  const html = trimCustomHtml(toMatrixCustomHTML(nodes, { allowTextFormatting: true }));
  assert.equal(
    html,
    '<ol><li><p>alpha</p><ul><li><p>nested</p></li></ul></li><li><p>beta</p></li></ol>'
  );
  assert.equal(toPlainText(nodes, false).trim(), '1. alpha\n  - nested\n2. beta');

  const [roundTripped] = htmlToEditorInput(html);
  assert.deepEqual(roundTripped, {
    type: BlockType.OrderedList,
    children: [
      {
        type: BlockType.ListItem,
        children: [
          {
            type: BlockType.Paragraph,
            children: [{ text: 'alpha' }],
          },
          {
            type: BlockType.UnorderedList,
            children: [
              {
                type: BlockType.ListItem,
                children: [{ text: 'nested' }],
              },
            ],
          },
        ],
      },
      {
        type: BlockType.ListItem,
        children: [{ text: 'beta' }],
      },
    ],
  });
});

test('list item indent and outdent create stable nested list structure', () => {
  const editor = withHistory(createEditor());
  editor.children = [
    {
      type: BlockType.OrderedList,
      children: [
        {
          type: BlockType.ListItem,
          children: [{ text: 'alpha' }],
        },
        {
          type: BlockType.ListItem,
          children: [{ text: 'beta' }],
        },
      ],
    },
  ];
  editor.selection = {
    anchor: { path: [0, 1, 0], offset: 0 },
    focus: { path: [0, 1, 0], offset: 0 },
  };

  assert.equal(indentListItem(editor), true);
  assert.deepEqual(editor.children, [
    {
      type: BlockType.OrderedList,
      children: [
        {
          type: BlockType.ListItem,
          children: [
            {
              type: BlockType.Paragraph,
              children: [{ text: 'alpha' }],
            },
            {
              type: BlockType.OrderedList,
              children: [
                {
                  type: BlockType.ListItem,
                  children: [{ text: 'beta' }],
                },
              ],
            },
          ],
        },
      ],
    },
  ]);

  const betaPath = [0, 0, 1, 0];
  editor.selection = {
    anchor: { path: [...betaPath, 0], offset: 0 },
    focus: { path: [...betaPath, 0], offset: 0 },
  };

  assert.equal(outdentListItem(editor), true);
  assert.deepEqual(editor.children, [
    {
      type: BlockType.OrderedList,
      children: [
        {
          type: BlockType.ListItem,
          children: [
            {
              type: BlockType.Paragraph,
              children: [{ text: 'alpha' }],
            },
          ],
        },
        {
          type: BlockType.ListItem,
          children: [{ text: 'beta' }],
        },
      ],
    },
  ]);
});

test('shift-enter inside a list item creates the next list item', () => {
  const editor = withHistory(createEditor());
  editor.children = [
    {
      type: BlockType.OrderedList,
      children: [
        {
          type: BlockType.ListItem,
          children: [{ text: 'alpha' }],
        },
      ],
    },
  ];
  editor.selection = {
    anchor: { path: [0, 0, 0], offset: 5 },
    focus: { path: [0, 0, 0], offset: 5 },
  };

  let defaultPrevented = false;
  const handled = toggleKeyboardShortcut(editor, {
    key: 'Enter',
    shiftKey: true,
    preventDefault: () => {
      defaultPrevented = true;
    },
  } as Parameters<typeof toggleKeyboardShortcut>[1]);

  assert.equal(handled, true);
  assert.equal(defaultPrevented, true);
  assert.deepEqual(editor.children, [
    {
      type: BlockType.OrderedList,
      children: [
        {
          type: BlockType.ListItem,
          children: [{ text: 'alpha' }],
        },
        {
          type: BlockType.ListItem,
          children: [{ text: '' }],
        },
      ],
    },
  ]);
});

test('shift-enter on an empty nested list item outdents before exiting the list', () => {
  const editor = withHistory(createEditor());
  editor.children = [
    {
      type: BlockType.OrderedList,
      children: [
        {
          type: BlockType.ListItem,
          children: [
            {
              type: BlockType.Paragraph,
              children: [{ text: 'alpha' }],
            },
            {
              type: BlockType.OrderedList,
              children: [
                {
                  type: BlockType.ListItem,
                  children: [{ text: '' }],
                },
              ],
            },
          ],
        },
      ],
    },
  ];
  editor.selection = {
    anchor: { path: [0, 0, 1, 0, 0], offset: 0 },
    focus: { path: [0, 0, 1, 0, 0], offset: 0 },
  };

  const handled = toggleKeyboardShortcut(editor, {
    key: 'Enter',
    shiftKey: true,
    preventDefault: () => {},
  } as Parameters<typeof toggleKeyboardShortcut>[1]);

  assert.equal(handled, true);
  assert.deepEqual(editor.children, [
    {
      type: BlockType.OrderedList,
      children: [
        {
          type: BlockType.ListItem,
          children: [
            {
              type: BlockType.Paragraph,
              children: [{ text: 'alpha' }],
            },
          ],
        },
        {
          type: BlockType.ListItem,
          children: [{ text: '' }],
        },
      ],
    },
  ]);
});

test('shift-enter on an empty top-level list item exits to a paragraph', () => {
  const editor = withHistory(createEditor());
  editor.children = [
    {
      type: BlockType.OrderedList,
      children: [
        {
          type: BlockType.ListItem,
          children: [{ text: 'alpha' }],
        },
        {
          type: BlockType.ListItem,
          children: [{ text: '' }],
        },
      ],
    },
  ];
  editor.selection = {
    anchor: { path: [0, 1, 0], offset: 0 },
    focus: { path: [0, 1, 0], offset: 0 },
  };

  const handled = toggleKeyboardShortcut(editor, {
    key: 'Enter',
    shiftKey: true,
    preventDefault: () => {},
  } as Parameters<typeof toggleKeyboardShortcut>[1]);

  assert.equal(handled, true);
  assert.deepEqual(editor.children, [
    {
      type: BlockType.OrderedList,
      children: [
        {
          type: BlockType.ListItem,
          children: [{ text: 'alpha' }],
        },
      ],
    },
    {
      type: BlockType.Paragraph,
      children: [{ text: '' }],
    },
  ]);
});

test('shift-enter before toggling a list keeps previous paragraph outside the list', () => {
  const editor = withHistory(createEditor());
  editor.children = [
    {
      type: BlockType.Paragraph,
      children: [{ text: 'normal text' }],
    },
  ];
  editor.selection = {
    anchor: { path: [0, 0], offset: 11 },
    focus: { path: [0, 0], offset: 11 },
  };

  const handled = toggleKeyboardShortcut(editor, {
    key: 'Enter',
    shiftKey: true,
    preventDefault: () => {},
  } as Parameters<typeof toggleKeyboardShortcut>[1]);
  assert.equal(handled, true);

  toggleBlock(editor, BlockType.OrderedList);

  assert.deepEqual(editor.children, [
    {
      type: BlockType.Paragraph,
      children: [{ text: 'normal text' }],
    },
    {
      type: BlockType.OrderedList,
      children: [
        {
          type: BlockType.ListItem,
          children: [{ text: '' }],
        },
      ],
    },
  ]);
});

test('rich html paste preserves safe block and inline structure', () => {
  const fragment = clipboardDataToEditorInput({
    getData: (format) =>
      format === 'text/html'
        ? '<p>Hello <strong>world</strong></p><ol start="3"><li>three</li><li>four</li></ol>'
        : '',
  });

  assert.deepEqual(fragment, [
    {
      type: BlockType.Paragraph,
      children: [{ text: 'Hello ' }, { text: 'world', bold: true }],
    },
    {
      type: BlockType.OrderedList,
      start: 3,
      children: [
        {
          type: BlockType.ListItem,
          children: [{ text: 'three' }],
        },
        {
          type: BlockType.ListItem,
          children: [{ text: 'four' }],
        },
      ],
    },
  ]);
});

test('rich html paste in Markdown mode preserves structure without visible escape characters', () => {
  const fragment = clipboardDataToEditorInput({
    getData: (format) =>
      format === 'text/html'
        ? '<p># literal heading marker and *literal stars*</p><ul><li>one</li><li>two</li></ul>'
        : '',
  });

  assert.deepEqual(fragment, [
    {
      type: BlockType.Paragraph,
      children: [{ text: '# literal heading marker and *literal stars*' }],
    },
    {
      type: BlockType.UnorderedList,
      children: [
        { type: BlockType.ListItem, children: [{ text: 'one' }] },
        { type: BlockType.ListItem, children: [{ text: 'two' }] },
      ],
    },
  ]);
});

for (const markdown of [false, true]) {
  test(`rich html paste preserves safe formatted links with Markdown ${
    markdown ? 'enabled' : 'disabled'
  }`, () => {
    const fragment = clipboardDataToEditorInput({
      getData: (format) =>
        format === 'text/html'
          ? '<p>Read <a href="https://example.org/spec?a=1&amp;b=2"><strong>the spec</strong></a>.</p>'
          : '',
    });

    assert.deepEqual(fragment, [
      {
        type: BlockType.Paragraph,
        children: [
          { text: 'Read ' },
          {
            type: BlockType.Link,
            href: 'https://example.org/spec?a=1&b=2',
            children: [{ text: 'the spec', bold: true }],
          },
          { text: '.' },
        ],
      },
    ]);

    const output = trimCustomHtml(
      toMatrixCustomHTML(fragment ?? [], { allowTextFormatting: true })
    );
    assert.equal(
      output,
      'Read <a href="https://example.org/spec?a=1&amp;b=2"><strong>the spec</strong></a>.'
    );
    assert.deepEqual(htmlToEditorInput(output, markdown), fragment);
  });
}

test('plain clipboard text is inserted byte-for-byte without synthesized Markdown escapes', () => {
  const fragment = clipboardDataToEditorInput({
    getData: (format) => (format === 'text/plain' ? '# heading\n* item *' : ''),
  });

  assert.deepEqual(fragment, [
    { type: BlockType.Paragraph, children: [{ text: '# heading' }] },
    { type: BlockType.Paragraph, children: [{ text: '* item *' }] },
  ]);
});

test('oversized clipboard HTML falls back to bounded plain text', () => {
  const fragment = clipboardDataToEditorInput({
    getData: (format) => {
      if (format === 'text/html') return `<p>${'x'.repeat(MAX_EDITOR_CLIPBOARD_CHARS)}</p>`;
      if (format === 'text/plain') return `${'a'.repeat(MAX_EDITOR_CLIPBOARD_CHARS)}overflow`;
      return '';
    },
  });

  assert.equal(fragment?.length, 1);
  const [paragraph] = fragment ?? [];
  assert.equal(Element.isElement(paragraph), true);
  if (!Element.isElement(paragraph)) throw new Error('Expected bounded paragraph');
  const [boundedText] = paragraph.children;
  assert.equal(Text.isText(boundedText), true);
  if (!Text.isText(boundedText)) throw new Error('Expected bounded text leaf');
  assert.equal(boundedText.text.length, MAX_EDITOR_CLIPBOARD_CHARS);
  assert.equal(boundedText.text.includes('overflow'), false);
});

for (const [name, html] of [
  ['style-only HTML', '<style>.message { color: red; }</style>'],
  ['stripped image HTML', '<p><img /></p>'],
  [
    'empty Office wrapper HTML',
    '<html><head><meta name="Generator" content="Microsoft Word"><style>p{margin:0}</style></head><body><!--StartFragment--><o:p></o:p><!--EndFragment--></body></html>',
  ],
] as const) {
  test(`${name} falls back to its usable plain clipboard flavor`, () => {
    const fragment = clipboardDataToEditorInput({
      getData: (format) => {
        if (format === 'text/html') return html;
        if (format === 'text/plain') return 'Visible fallback';
        return '';
      },
    });

    assert.deepEqual(fragment, [
      { type: BlockType.Paragraph, children: [{ text: 'Visible fallback' }] },
    ]);
  });
}

test('oversized clipboard HTML without plain text is rejected and consumed', () => {
  const editor = withHistory(createEditor());
  editor.children = [{ type: BlockType.Paragraph, children: [{ text: '' }] }];
  editor.selection = {
    anchor: { path: [0, 0], offset: 0 },
    focus: { path: [0, 0], offset: 0 },
  };
  const clipboardData = {
    getData: (format: string) =>
      format === 'text/html' ? `<p>${'x'.repeat(MAX_EDITOR_CLIPBOARD_CHARS)}</p>` : '',
  };

  assert.equal(clipboardDataToEditorInput(clipboardData), undefined);
  const result = insertClipboardData(editor, clipboardData);
  assert.equal(result, ClipboardInsertResult.Rejected);
  assert.equal(shouldPreventDefaultForClipboardInsert(result), true);
  assert.deepEqual(editor.children, [{ type: BlockType.Paragraph, children: [{ text: '' }] }]);
});

test('unsupported clipboard data remains available to the platform default handler', () => {
  const editor = withHistory(createEditor());
  const result = insertClipboardData(editor, { getData: () => '' });

  assert.equal(result, ClipboardInsertResult.Unsupported);
  assert.equal(shouldPreventDefaultForClipboardInsert(result), false);
});

test('room composer and message editor consume recognized rejected clipboard payloads', () => {
  const sources = [
    readFileSync('src/app/features/room/RoomInput.tsx', 'utf8'),
    readFileSync('src/app/features/room/message/MessageEditor.tsx', 'utf8'),
  ];

  sources.forEach((source) => {
    assert.match(source, /const insertion = insertClipboardData\(/);
    assert.match(source, /shouldPreventDefaultForClipboardInsert\(insertion\)/);
    assert.match(
      source,
      /if \(shouldPreventDefaultForClipboardInsert\(insertion\)\) \{\s*evt\.preventDefault\(\)/
    );
  });
});

test('clipboard HTML is reduced through the Matrix allowlist before editor insertion', () => {
  const fragment = clipboardDataToEditorInput({
    getData: (format) =>
      format === 'text/html'
        ? '<style>body{display:none}</style><p style="position:fixed">safe<script>alert(1)</script></p><a href="javascript:alert(1)">link</a>'
        : '',
  });

  assert.deepEqual(fragment, [
    { type: BlockType.Paragraph, children: [{ text: 'safe' }] },
    { type: BlockType.Paragraph, children: [{ text: 'link' }] },
  ]);
});

test('plain text paste falls back to paragraph lines', () => {
  const editor = withHistory(createEditor());
  editor.children = [
    {
      type: BlockType.Paragraph,
      children: [{ text: '' }],
    },
  ];
  editor.selection = {
    anchor: { path: [0, 0], offset: 0 },
    focus: { path: [0, 0], offset: 0 },
  };

  const handled = insertClipboardData(editor, {
    getData: (format) => (format === 'text/plain' ? 'alpha\nbeta' : ''),
  });

  assert.equal(handled, ClipboardInsertResult.Inserted);
  assert.deepEqual(editor.children, [
    {
      type: BlockType.Paragraph,
      children: [{ text: 'alpha' }],
    },
    {
      type: BlockType.Paragraph,
      children: [{ text: 'beta' }],
    },
  ]);
});

test('golden rich content contract covers Matrix HTML, fallback text, and edit input', () => {
  const nodes: Descendant[] = [
    {
      type: BlockType.Heading,
      level: 2,
      children: [{ text: 'Release plan' }],
    },
    {
      type: BlockType.BlockQuote,
      children: [
        {
          type: BlockType.QuoteLine,
          children: [{ text: 'quoted context' }],
        },
      ],
    },
    {
      type: BlockType.CodeBlock,
      children: [
        { type: BlockType.CodeLine, children: [{ text: 'const ready = true;' }] },
        { type: BlockType.CodeLine, children: [{ text: 'return ready;' }] },
      ],
    },
    {
      type: BlockType.Paragraph,
      children: [
        { text: 'Discuss with ' },
        {
          type: BlockType.Mention,
          id: '@alice:example.org',
          name: 'Alice',
          highlight: false,
          eventId: undefined,
          viaServers: undefined,
          children: [{ text: '' }],
        },
        { text: ' using ' },
        {
          type: BlockType.Link,
          href: 'https://example.org/spec',
          children: [{ text: 'the spec' }],
        },
        { text: ' and keep ' },
        { text: 'classified', spoiler: true } as any,
        { text: '.' },
      ],
    },
    {
      type: BlockType.OrderedList,
      start: 10,
      children: [
        { type: BlockType.ListItem, children: [{ text: 'ten' }] },
        {
          type: BlockType.ListItem,
          children: [
            {
              type: BlockType.Paragraph,
              children: [{ text: 'eleven' }],
            },
            {
              type: BlockType.UnorderedList,
              children: [{ type: BlockType.ListItem, children: [{ text: 'nested' }] }],
            },
          ],
        },
      ],
    },
  ];

  const html = trimCustomHtml(toMatrixCustomHTML(nodes, { allowTextFormatting: true }));
  assert.doesNotMatch(html, /\[object Object\]/);
  assert.equal(
    html,
    '<h2>Release plan</h2><blockquote>quoted context<br/></blockquote><pre><code>const ready = true;\nreturn ready;\n</code></pre>Discuss with <a href="https://matrix.to/#/@alice:example.org">Alice</a> using <a href="https://example.org/spec">the spec</a> and keep <span data-mx-spoiler>classified</span>.<br/><ol start="10"><li><p>ten</p></li><li><p>eleven</p><ul><li><p>nested</p></li></ul></li></ol>'
  );

  const plain = toPlainText(nodes, false).trim();
  assert.doesNotMatch(plain, /\[object Object\]/);
  assert.doesNotMatch(plain, /classified/);
  assert.equal(
    plain,
    'Release plan\n| quoted context\n\nconst ready = true;\nreturn ready;\n\nDiscuss with @alice:example.org using [the spec](https://example.org/spec) and keep [spoiler].\n10. ten\n11. eleven\n  - nested'
  );

  const editedInput = htmlToEditorInput(
    '<h2>Release plan</h2><blockquote><p>quoted context</p></blockquote><pre><code>const ready = true;\nreturn ready;</code></pre><p>Discuss with <a href="https://matrix.to/#/@alice:example.org">Alice</a> using <a href="https://example.org/spec">the spec</a> and keep <span data-mx-spoiler>classified</span>.</p><ol start="10"><li>ten</li><li>eleven<ul><li>nested</li></ul></li></ol>'
  );
  assert.deepEqual(editedInput, [
    {
      type: BlockType.Heading,
      level: 2,
      children: [{ text: 'Release plan' }],
    },
    {
      type: BlockType.BlockQuote,
      children: [
        {
          type: BlockType.QuoteLine,
          children: [{ text: 'quoted context' }],
        },
      ],
    },
    {
      type: BlockType.CodeBlock,
      children: [
        { type: BlockType.CodeLine, children: [{ text: 'const ready = true;' }] },
        { type: BlockType.CodeLine, children: [{ text: 'return ready;' }] },
      ],
    },
    {
      type: BlockType.Paragraph,
      children: [
        { text: 'Discuss with ' },
        {
          type: BlockType.Mention,
          id: '@alice:example.org',
          name: 'Alice',
          highlight: false,
          eventId: undefined,
          viaServers: undefined,
          children: [{ text: '' }],
        },
        { text: ' using ' },
        {
          type: BlockType.Link,
          href: 'https://example.org/spec',
          children: [{ text: 'the spec' }],
        },
        { text: ' and keep ' },
        { text: 'classified', spoiler: true },
        { text: '.' },
      ],
    },
    {
      type: BlockType.OrderedList,
      start: 10,
      children: [
        { type: BlockType.ListItem, children: [{ text: 'ten' }] },
        {
          type: BlockType.ListItem,
          children: [
            {
              type: BlockType.Paragraph,
              children: [{ text: 'eleven' }],
            },
            {
              type: BlockType.UnorderedList,
              children: [{ type: BlockType.ListItem, children: [{ text: 'nested' }] }],
            },
          ],
        },
      ],
    },
  ]);
});

test('markdown golden case preserves headings, quotes, code, links, and ordered starts', () => {
  const nodes: Descendant[] = [
    { type: BlockType.Paragraph, children: [{ text: '## Heading' }] },
    { type: BlockType.Paragraph, children: [{ text: '> quoted' }] },
    { type: BlockType.Paragraph, children: [{ text: '```' }] },
    { type: BlockType.Paragraph, children: [{ text: 'code' }] },
    { type: BlockType.Paragraph, children: [{ text: '```' }] },
    { type: BlockType.Paragraph, children: [{ text: '[label](https://example.org)' }] },
    { type: BlockType.Paragraph, children: [{ text: '10. ten' }] },
    { type: BlockType.Paragraph, children: [{ text: '11. eleven' }] },
  ];

  const html = trimCustomHtml(
    toMatrixCustomHTML(nodes, {
      allowTextFormatting: true,
      allowBlockMarkdown: true,
      allowInlineMarkdown: true,
    })
  );

  assert.doesNotMatch(html, /\[object Object\]/);
  assert.equal(
    html,
    '<h2>Heading</h2><blockquote>quoted<br/></blockquote><pre><code>code\n</code></pre><a href="https://example.org">label</a><br/><ol start="10"><li><p>ten</p></li><li><p>eleven</p></li></ol>'
  );
});
