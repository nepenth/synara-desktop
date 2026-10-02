import {
  cloneNode,
  hasChildren,
  isCDATA,
  isDocument,
  type ChildNode,
  type ParentNode,
} from 'domhandler';
import type { DOMNode } from 'html-react-parser';

const normalizeChildren = (nodes: readonly ChildNode[], parent: ParentNode | null): DOMNode[] => {
  const normalized = nodes.flatMap((node) => {
    if (isCDATA(node) || isDocument(node)) return normalizeChildren(node.children, parent);
    if (hasChildren(node)) node.children = normalizeChildren(node.children, node);
    return [node];
  });
  normalized.forEach((node, index) => {
    node.parent = parent;
    node.prev = normalized[index - 1] ?? null;
    node.next = normalized[index + 1] ?? null;
  });
  return normalized;
};

/** Preserve nested XML-shaped text and parent links without mutating the parser's source tree. */
export const reactDomNodes = (nodes: readonly ChildNode[]): DOMNode[] =>
  normalizeChildren(
    nodes.map((node) => cloneNode(node, true)),
    nodes[0]?.parent ?? null
  );
