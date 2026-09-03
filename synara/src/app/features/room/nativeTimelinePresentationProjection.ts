import { Element, Text, htmlToDOM, type DOMNode } from 'html-react-parser';
import { prepareNativeFormattedBody } from './nativeTimelineRichText';

export type NativeFormattedElementPresentation =
  | 'codeBlock'
  | 'inlineCode'
  | 'inlineImageFallback'
  | 'matrixColor'
  | 'passthrough'
  | 'spoiler'
  | 'table';

export type NativeFormattedSemanticKind =
  | 'bold'
  | 'heading'
  | 'inlineCode'
  | 'orderedList'
  | 'preformattedCode'
  | 'spoiler'
  | 'strikethrough'
  | 'table'
  | 'unorderedList';

export type NativeFormattedPresentationProjection = {
  sanitizedHtml: string;
  domNodes: DOMNode[];
  semanticKinds: NativeFormattedSemanticKind[];
  links: string[];
  spoilerReasons: string[];
  inlineCode: string[];
  codeBlocks: string[];
  orderedListStarts: number[];
  inlineImageFallbacks: string[];
  resourceOwningElements: number;
};

/**
 * One pure decision table for DOM nodes that require presenter-owned behavior.
 * The React presenter and the contract projection both consume this function,
 * so an allowed Matrix `<img>` can never silently become a resource load in
 * one path while the other path still reports an inert fallback.
 */
export const classifyNativeFormattedElement = (
  name: string,
  attributes: Record<string, string>,
  parentName?: string
): NativeFormattedElementPresentation => {
  if (name === 'table') return 'table';
  if (name === 'span' && 'data-mx-spoiler' in attributes) return 'spoiler';
  if (name === 'span' && ('data-mx-color' in attributes || 'data-mx-bg-color' in attributes)) {
    return 'matrixColor';
  }
  if (name === 'img') return 'inlineImageFallback';
  if (name === 'code' && parentName !== 'pre') return 'inlineCode';
  if (name === 'pre') return 'codeBlock';
  return 'passthrough';
};

const exactText = (node: DOMNode): string => {
  if (node instanceof Text) return node.data;
  if (node instanceof Element) return node.children.map(exactText).join('');
  return '';
};

/**
 * Description of the desktop presenter's real sanitized DOM input and node
 * decisions. The React presenter consumes the returned DOM directly, avoiding
 * a second parse while the contract harness inspects its semantic projection.
 */
export const projectNativeFormattedBody = (
  html: string
): NativeFormattedPresentationProjection | undefined => {
  const sanitizedHtml = prepareNativeFormattedBody(html);
  if (!sanitizedHtml) return undefined;

  const semanticKinds = new Set<NativeFormattedSemanticKind>();
  const links: string[] = [];
  const spoilerReasons: string[] = [];
  const inlineCode: string[] = [];
  const codeBlocks: string[] = [];
  const orderedListStarts: number[] = [];
  const inlineImageFallbacks: string[] = [];
  let resourceOwningElements = 0;

  const visit = (node: DOMNode): void => {
    if (!(node instanceof Element)) return;
    const parentName = node.parent instanceof Element ? node.parent.name : undefined;
    const presentation = classifyNativeFormattedElement(node.name, node.attribs, parentName);

    if (node.name === 'strong' || node.name === 'b') semanticKinds.add('bold');
    if (/^h[1-6]$/.test(node.name)) semanticKinds.add('heading');
    if (node.name === 'ol') {
      semanticKinds.add('orderedList');
      if (node.attribs.start !== undefined) orderedListStarts.push(Number(node.attribs.start));
    }
    if (node.name === 'ul') semanticKinds.add('unorderedList');
    if (node.name === 'del' || node.name === 's') semanticKinds.add('strikethrough');

    switch (presentation) {
      case 'table':
        semanticKinds.add('table');
        break;
      case 'spoiler': {
        semanticKinds.add('spoiler');
        const reason = node.attribs['data-mx-spoiler']?.trim().slice(0, 160);
        if (reason) spoilerReasons.push(reason);
        break;
      }
      case 'inlineCode':
        semanticKinds.add('inlineCode');
        inlineCode.push(exactText(node));
        break;
      case 'codeBlock':
        semanticKinds.add('preformattedCode');
        codeBlocks.push(exactText(node));
        break;
      case 'inlineImageFallback':
        inlineImageFallbacks.push(
          node.attribs.alt?.trim() || node.attribs.title?.trim() || 'Inline image'
        );
        break;
      case 'matrixColor':
      case 'passthrough':
        if (node.name === 'img') resourceOwningElements += 1;
        break;
    }

    if (node.name === 'a' && node.attribs.href) links.push(node.attribs.href);
    node.children.forEach(visit);
  };

  let domNodes: DOMNode[];
  try {
    domNodes = htmlToDOM(sanitizedHtml);
  } catch {
    return undefined;
  }
  domNodes.forEach(visit);
  return {
    sanitizedHtml,
    domNodes,
    semanticKinds: [...semanticKinds],
    links,
    spoilerReasons,
    inlineCode,
    codeBlocks,
    orderedListStarts,
    inlineImageFallbacks,
    resourceOwningElements,
  };
};
