export enum EmojiType {
  Emoji = 'emoji',
  CustomEmoji = 'customEmoji',
}

export type EmojiItemInfo = {
  type: EmojiType;
  data: string;
  shortcode: string;
  label: string;
};
