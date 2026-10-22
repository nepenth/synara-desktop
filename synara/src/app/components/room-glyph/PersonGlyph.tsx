import React from 'react';
import { Avatar, color } from 'folds';
import classNames from 'classnames';
import { UserAvatar } from '../user-avatar';
import type { PersonRoomGlyph } from './roomGlyph';
import * as css from './PersonGlyph.css';

type PersonGlyphProps = {
  className?: string;
  glyph: PersonRoomGlyph;
};

/** A DM's peer in the `#` slot: their avatar, or their initial on a neutral tint. */
export function PersonGlyph({ className, glyph }: PersonGlyphProps) {
  return (
    <Avatar className={classNames(css.PersonGlyph, className)} radii="Pill" aria-hidden="true">
      <UserAvatar
        userId={glyph.userId}
        src={glyph.avatarUrl}
        alt=""
        fallbackBackground={color.SurfaceVariant.Container}
        fallbackColor="var(--synara-content-secondary)"
        renderFallback={() => glyph.initial}
      />
    </Avatar>
  );
}
