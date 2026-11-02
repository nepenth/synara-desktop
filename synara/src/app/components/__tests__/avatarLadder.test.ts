import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const read = (path: string) => readFileSync(`src/app/${path}`, 'utf8');

test('the avatar ladder matches iOS SynaraAvatarSize (DS-11)', () => {
  const ladder = read('components/user-avatar/avatarLadder.ts');
  for (const [rung, px] of [
    ['glyph', 20],
    ['dense', 24],
    ['list', 28],
    ['timeline', 36],
    ['profileRow', 44],
    ['hero', 64],
  ] as const) {
    assert.match(ladder, new RegExp(`\\b${rung}: ${px},`), rung);
  }
  assert.match(ladder, /AVATAR_SQUARE_RADIUS = '22%'/);
});

test('lists, the reaction viewer and heroes sit on the ladder', () => {
  for (const [path, rung] of [
    ['features/room/MembersDrawer.tsx', 'list'],
    ['features/room/reaction-viewer/ReactionViewer.tsx', 'list'],
    ['features/settings/account/Profile.tsx', 'hero'],
    ['features/common-settings/general/RoomProfile.tsx', 'hero'],
    ['components/user-profile/UserHero.tsx', 'hero'],
    ['components/room-card/RoomCard.tsx', 'hero'],
    ['features/lobby/LobbyHero.tsx', 'hero'],
    ['features/lobby/RoomItem.tsx', 'profileRow'],
  ] as const) {
    const source = read(path);
    assert.match(source, new RegExp(`avatarLadderStyle\\('${rung}'\\)`), path);
    // Folds stops at 24/36/42/58; 58 px heroes are off the ladder.
    assert.doesNotMatch(source, /<Avatar[^>]*size="500"/, path);
  }
});
