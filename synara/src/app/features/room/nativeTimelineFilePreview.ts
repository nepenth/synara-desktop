import { parseBlockMD, parseInlineMD } from '../../plugins/markdown';
import { MAX_NATIVE_FORMATTED_BODY_BYTES } from './nativeTimelineRichText';

const MARKDOWN_MIME_TYPES = new Set(['text/markdown', 'text/x-markdown']);
const MARKDOWN_EXTENSIONS = ['.md', '.markdown'];

export type NativeTimelineFilePreviewTarget = {
  handleId: string;
  filename?: string;
  mimeType?: string;
};

/** Same presentation budget as formatted timeline bodies. */
export const MAX_NATIVE_MARKDOWN_PREVIEW_BYTES = MAX_NATIVE_FORMATTED_BODY_BYTES;

const utf8ByteLength = (value: string): number => new TextEncoder().encode(value).byteLength;

const filenameLooksLikeMarkdown = (filename?: string): boolean => {
  const name = filename?.trim().toLowerCase();
  if (!name) return false;
  return MARKDOWN_EXTENSIONS.some((extension) => name.endsWith(extension));
};

/**
 * Preview routing for Core `messageType: 'file'` rows. Download always uses
 * the generic save owner; only this helper decides whether the primary click
 * opens an in-client markdown preview. Presenters must not inline MIME tables.
 */
export const isNativeTimelineMarkdownAttachment = ({
  filename,
  mimeType,
}: {
  filename?: string;
  mimeType?: string;
}): boolean => {
  const mime = mimeType?.trim().toLowerCase();
  if (mime && MARKDOWN_MIME_TYPES.has(mime)) return true;
  return filenameLooksLikeMarkdown(filename);
};

export const projectNativeTimelineMarkdownPreview = (
  source: string
): { html: string; plain: string; tooLarge: boolean } => {
  const plain = source;
  if (
    source.length > MAX_NATIVE_MARKDOWN_PREVIEW_BYTES ||
    utf8ByteLength(source) > MAX_NATIVE_MARKDOWN_PREVIEW_BYTES
  ) {
    return { html: '', plain, tooLarge: true };
  }
  try {
    return { html: parseBlockMD(source, parseInlineMD), plain, tooLarge: false };
  } catch {
    return { html: '', plain, tooLarge: false };
  }
};
