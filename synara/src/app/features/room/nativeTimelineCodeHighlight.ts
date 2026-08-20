import { isTag, isText, type ChildNode } from 'domhandler';
import Prism from 'prismjs';

const LANGUAGE_CLASS_RE = /(^|\s)(language-[\w+-]+)(\s|$)/;
const PRISM_LANGUAGE_ALIASES: Record<string, string> = {
  js: 'javascript',
  ts: 'typescript',
  rs: 'rust',
  py: 'python',
  sh: 'bash',
  shell: 'bash',
  zsh: 'bash',
  yml: 'yaml',
};

export const NATIVE_PRISM_CHAR_LIMIT = 50_000;

export const extractTextFromHtmlChildren = (nodes: ChildNode[]): string => {
  let text = '';
  nodes.forEach((node) => {
    if (isText(node)) {
      text += node.data;
    } else if (isTag(node) && node.children) {
      text += extractTextFromHtmlChildren(node.children);
    }
  });
  return text;
};

export const languageClassFromClassName = (className?: string): string | undefined => {
  if (!className) return undefined;
  const match = className.match(LANGUAGE_CLASS_RE);
  return match?.[2];
};

export const normalizePrismLanguage = (languageClass?: string): string | undefined => {
  const className = languageClassFromClassName(languageClass);
  if (!className) return undefined;
  const raw = className.slice('language-'.length).toLowerCase();
  if (!raw) return undefined;
  return PRISM_LANGUAGE_ALIASES[raw] ?? raw;
};

export const prismLanguageClass = (languageClass?: string): string | undefined => {
  const language = normalizePrismLanguage(languageClass);
  return language ? `language-${language}` : languageClassFromClassName(languageClass);
};

export const displayCodeLanguage = (languageClass?: string): string =>
  normalizePrismLanguage(languageClass) ?? 'code';

export const countCodeLines = (code: string): number => {
  const normalized = code.replace(/\n$/, '');
  if (normalized.length === 0) return 1;
  return normalized.split('\n').length;
};

export const formatLineNumbers = (lineCount: number): string =>
  Array.from({ length: Math.max(1, lineCount) }, (_, index) => String(index + 1)).join('\n');

export const nativeCodeBlockFromPreChildren = (
  children: ChildNode[]
): { code: string; languageClass?: string } => {
  const codeEl = children.find((node) => isTag(node) && node.name === 'code');
  if (codeEl && isTag(codeEl)) {
    return {
      code: extractTextFromHtmlChildren(codeEl.children),
      languageClass: languageClassFromClassName(codeEl.attribs.class),
    };
  }
  return { code: extractTextFromHtmlChildren(children) };
};

export const escapeCodeHtml = (code: string): string =>
  code.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');

export const highlightNativeCode = (code: string, languageClass?: string): string => {
  const language = normalizePrismLanguage(languageClass);
  const grammar = language ? Prism.languages[language] : undefined;
  if (!language || !grammar) {
    return escapeCodeHtml(code);
  }
  return Prism.highlight(code, grammar, language);
};
