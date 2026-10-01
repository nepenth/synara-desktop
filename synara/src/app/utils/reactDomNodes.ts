import { isCDATA, isDocument, type ChildNode } from 'domhandler';
import type { DOMNode } from 'html-react-parser';

/** HTML parsers do not emit CDATA, but XML-shaped children must retain their text. */
export const reactDomNodes = (nodes: readonly ChildNode[]): DOMNode[] =>
  nodes.flatMap((node) =>
    isCDATA(node) || isDocument(node) ? reactDomNodes(node.children) : [node]
  );
