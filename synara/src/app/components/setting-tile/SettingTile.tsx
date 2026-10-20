import React, { ReactNode, cloneElement, isValidElement, useId } from 'react';
import { Box, Switch, Text } from 'folds';
import { BreakWord } from '../../styles/Text.css';
import * as css from './SettingTile.css';

type SettingTileProps = {
  title?: ReactNode;
  description?: ReactNode;
  before?: ReactNode;
  after?: ReactNode;
  children?: ReactNode;
};
type ControlNameProps = {
  'aria-label'?: string;
  'aria-labelledby'?: string;
  'aria-describedby'?: string;
};

export function SettingTile({ title, description, before, after, children }: SettingTileProps) {
  const titleId = useId();
  const descriptionId = useId();
  // A folds Switch is an empty button: name it after the row's title so it is
  // not announced as an unlabelled switch.
  const control =
    title &&
    isValidElement<ControlNameProps>(after) &&
    after.type === Switch &&
    !after.props['aria-label'] &&
    !after.props['aria-labelledby']
      ? cloneElement(after, {
          'aria-labelledby': titleId,
          'aria-describedby': description ? descriptionId : undefined,
        })
      : after;

  return (
    <Box
      alignItems="Center"
      gap="300"
      className={css.SettingTile}
      data-setting-title={typeof title === 'string' ? title : undefined}
    >
      {before && <Box shrink="No">{before}</Box>}
      <Box grow="Yes" direction="Column" gap="100" className={css.SettingCopy}>
        {title && (
          <Text className={BreakWord} size="T300" id={titleId}>
            {title}
          </Text>
        )}
        {description && (
          <Text className={BreakWord} size="T200" priority="300" id={descriptionId}>
            {description}
          </Text>
        )}
        {children}
      </Box>
      {after && (
        <Box shrink="No" className={css.SettingControl} data-setting-control>
          {control}
        </Box>
      )}
    </Box>
  );
}
