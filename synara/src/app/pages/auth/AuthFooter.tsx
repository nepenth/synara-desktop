import React from 'react';
import { Box, Text } from 'folds';
import * as css from './styles.css';
import {
  APP_VERSION,
  MATRIX_URL,
  SYNARA_PROJECT_URL,
  SYNARA_RELEASES_URL,
  SYNARA_SOURCE_CODE_URL,
  openExternalUrlFromClick,
} from '../../utils/appLinks';

export function AuthFooter() {
  return (
    <Box className={css.AuthFooter} justifyContent="Center" gap="400" wrap="Wrap">
      <Text
        as="a"
        size="T300"
        href={SYNARA_SOURCE_CODE_URL}
        onClick={(evt) => openExternalUrlFromClick(evt, SYNARA_SOURCE_CODE_URL)}
        target="_blank"
        rel="noreferrer"
      >
        About
      </Text>
      <Text
        as="a"
        size="T300"
        href={SYNARA_RELEASES_URL}
        onClick={(evt) => openExternalUrlFromClick(evt, SYNARA_RELEASES_URL)}
        target="_blank"
        rel="noreferrer"
      >
        v{APP_VERSION}
      </Text>
      <Text
        as="a"
        size="T300"
        href={SYNARA_PROJECT_URL}
        onClick={(evt) => openExternalUrlFromClick(evt, SYNARA_PROJECT_URL)}
        target="_blank"
        rel="noreferrer"
      >
        Project
      </Text>
      <Text
        as="a"
        size="T300"
        href={MATRIX_URL}
        onClick={(evt) => openExternalUrlFromClick(evt, MATRIX_URL)}
        target="_blank"
        rel="noreferrer"
      >
        Powered by Matrix
      </Text>
    </Box>
  );
}
