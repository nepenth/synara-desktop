import type { SettingsSearchEntry } from '../../components/settings-layout/settingsSearch';

export type CommonSettingsPages<T> = {
  general: T;
  permissions: T;
  emojis: T;
  developer: T;
};

/**
 * Every row and section the Room or Space settings pages render, for settings
 * search. Titles must match the rendered titles exactly; the completeness test
 * in `settingsSearchIndex.test.ts` scans the pages.
 */
export const commonSettingsSearchIndex = <T>(
  kind: 'room' | 'space',
  pages: CommonSettingsPages<T>
): SettingsSearchEntry<T>[] => {
  const noun = kind === 'space' ? 'space' : 'room';
  return [
    // General
    {
      page: pages.general,
      title: 'Profile',
      description: `The ${noun} name, topic and avatar.`,
      synonyms: ['name', 'topic', 'avatar', 'icon', 'picture'],
    },
    {
      page: pages.general,
      title: 'Options',
      synonyms: ['access', 'history', 'encryption', 'directory'],
    },
    {
      page: pages.general,
      title: kind === 'space' ? 'Space Access' : 'Room Access',
      description: `Who can join the ${noun}.`,
      synonyms: ['join rule', 'public', 'private', 'invite only', 'knock', 'restricted'],
    },
    {
      page: pages.general,
      title: 'Message History Visibility',
      description: 'Who can read past messages.',
      synonyms: ['history', 'visibility', 'past messages', 'shared'],
    },
    {
      page: pages.general,
      title: 'Message retention',
      description: 'How long messages are kept.',
      synonyms: ['retention', 'expiry', 'delete', 'lifetime'],
    },
    {
      page: pages.general,
      title: 'Room Encryption',
      description: 'End-to-end encryption for messages.',
      synonyms: ['encryption', 'e2ee', 'encrypted'],
    },
    {
      page: pages.general,
      title: 'Encrypted state events',
      description: 'Whether state events such as the name and topic are encrypted.',
      synonyms: ['msc4362', 'state encryption'],
    },
    {
      page: pages.general,
      title: 'Encrypt state events',
      description: 'Encrypt the name, topic and avatar.',
      synonyms: ['msc4362', 'state encryption'],
    },
    {
      page: pages.general,
      title: 'Publish to Directory',
      description: `List the ${noun} in the server's public directory.`,
      synonyms: ['directory', 'public', 'discover', 'listing'],
    },
    {
      page: pages.general,
      title: 'Addresses',
      synonyms: ['alias', 'aliases', 'address', 'link'],
    },
    {
      page: pages.general,
      title: 'Published Addresses',
      description: 'Addresses others can use to find and join.',
      synonyms: ['alias', 'canonical', 'main address'],
    },
    {
      page: pages.general,
      title: 'Local Addresses',
      description: 'Addresses on your homeserver.',
      synonyms: ['alias', 'local alias'],
    },
    {
      page: pages.general,
      title: 'Danger Zone',
      synonyms: ['upgrade', 'destructive'],
    },
    {
      page: pages.general,
      title: kind === 'space' ? 'Upgrade Space' : 'Upgrade Room',
      description: `Move the ${noun} to a newer room version.`,
      synonyms: ['upgrade', 'room version', 'version'],
    },

    // Permissions
    {
      page: pages.permissions,
      title: 'Power Levels',
      description: 'Roles and their power levels.',
      synonyms: ['roles', 'admin', 'moderator', 'power'],
    },
    {
      page: pages.permissions,
      title: 'Founders',
      description: 'Creators who outrank every power level.',
      synonyms: ['creators', 'owners'],
    },
    {
      page: pages.permissions,
      title: 'New Power Level',
      description: 'Add a custom role.',
      synonyms: ['role', 'custom role'],
    },
    {
      page: pages.permissions,
      title: 'Users',
      synonyms: ['members', 'invite', 'kick', 'ban'],
    },
    {
      page: pages.permissions,
      title: 'Default Power',
      description: 'The power level new members get.',
      synonyms: ['default role', 'members'],
    },

    // Custom Emoji
    {
      page: pages.emojis,
      title: 'Packs',
      synonyms: ['emoji', 'stickers', 'custom emoji'],
    },
    {
      page: pages.emojis,
      title: 'New Pack',
      description: 'Create an emoji and sticker pack.',
      synonyms: ['emoji', 'stickers', 'create pack'],
    },

    // Developer Tools
    {
      page: pages.developer,
      title: 'Options',
      synonyms: ['developer', 'devtools'],
    },
    {
      page: pages.developer,
      title: 'Enable Developer Tools',
      synonyms: ['developer', 'devtools', 'debug'],
    },
    {
      page: pages.developer,
      title: 'Room ID',
      description: `The ${noun}'s internal ID.`,
      synonyms: ['id', 'identifier'],
    },
    {
      page: pages.developer,
      title: 'Data',
      synonyms: ['state', 'events', 'json'],
    },
    {
      page: pages.developer,
      title: 'New Message Event',
      description: 'Send a raw event.',
      synonyms: ['send event', 'raw event', 'json'],
    },
    {
      page: pages.developer,
      title: 'Room State',
      description: 'Browse and edit state events.',
      synonyms: ['state events', 'json'],
    },
    {
      page: pages.developer,
      title: 'Account Data',
      description: `Browse and edit ${noun} account data.`,
      synonyms: ['json', 'raw'],
    },
  ];
};
