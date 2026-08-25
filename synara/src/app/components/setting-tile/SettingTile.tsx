import React, { ReactNode } from 'react';
import { Box, Text } from 'folds';
import { BreakWord } from '../../styles/Text.css';
import * as css from './SettingTile.css';

type SettingTileProps = {
  title?: ReactNode;
  description?: ReactNode;
  before?: ReactNode;
  after?: ReactNode;
  children?: ReactNode;
};
export function SettingTile({ title, description, before, after, children }: SettingTileProps) {
  return (
    <Box alignItems="Center" gap="300" className={css.SettingTile}>
      {before && <Box shrink="No">{before}</Box>}
      <Box grow="Yes" direction="Column" gap="100" className={css.SettingCopy}>
        {title && (
          <Text className={BreakWord} size="T300">
            {title}
          </Text>
        )}
        {description && (
          <Text className={BreakWord} size="T200" priority="300">
            {description}
          </Text>
        )}
        {children}
      </Box>
      {after && (
        <Box shrink="No" className={css.SettingControl}>
          {after}
        </Box>
      )}
    </Box>
  );
}
