import React, { MouseEvent } from 'react';
import { Box, Text, color, config, toRem } from 'folds';
import { Page } from '../../components/page';
import { SynaraMark } from '../auth/SynaraMark';
import {
  APP_VERSION,
  SYNARA_PROJECT_URL,
  SYNARA_RELEASES_URL,
  SYNARA_SOURCE_CODE_URL,
  SYNARA_SUPPORT_URL,
  openExternalUrlFromClick,
} from '../../utils/appLinks';

function ExternalLink({
  href,
  onClick,
  children,
}: {
  href: string;
  onClick: (evt: MouseEvent<HTMLAnchorElement>) => void;
  children: string;
}) {
  return (
    <a href={href} onClick={onClick} target="_blank" rel="noreferrer noopener">
      {children}
    </a>
  );
}

export function WelcomePage() {
  return (
    <Page>
      <Box
        grow="Yes"
        alignItems="Center"
        justifyContent="Center"
        style={{ padding: config.space.S400 }}
      >
        <Box
          direction="Column"
          alignItems="Center"
          gap="400"
          style={{ width: '100%', maxWidth: toRem(360), textAlign: 'center' }}
        >
          <Box direction="Column" alignItems="Center" gap="400">
            <SynaraMark size={88} decorative />
            <Box direction="Column" alignItems="Center" gap="100">
              <Text as="h1" size="H3">
                Synara
              </Text>
              <Text size="T300" priority="300">
                <ExternalLink
                  href={SYNARA_RELEASES_URL}
                  onClick={(evt) => openExternalUrlFromClick(evt, SYNARA_RELEASES_URL)}
                >
                  {`v${APP_VERSION}`}
                </ExternalLink>
              </Text>
            </Box>
          </Box>
          <Text size="T300" priority="400">
            Pick a room or direct message from the sidebar.
          </Text>
          <Box
            as="nav"
            aria-label="Synara links"
            justifyContent="Center"
            gap="400"
            wrap="Wrap"
            style={{
              width: '100%',
              paddingTop: config.space.S300,
              borderTop: `${config.borderWidth.B300} solid ${color.Surface.ContainerLine}`,
            }}
          >
            <Text size="T300">
              <ExternalLink
                href={SYNARA_SOURCE_CODE_URL}
                onClick={(evt) => openExternalUrlFromClick(evt, SYNARA_SOURCE_CODE_URL)}
              >
                Source code
              </ExternalLink>
            </Text>
            <Text size="T300">
              <ExternalLink
                href={SYNARA_PROJECT_URL}
                onClick={(evt) => openExternalUrlFromClick(evt, SYNARA_PROJECT_URL)}
              >
                Project
              </ExternalLink>
            </Text>
            <Text size="T300">
              <ExternalLink
                href={SYNARA_SUPPORT_URL}
                onClick={(evt) => openExternalUrlFromClick(evt, SYNARA_SUPPORT_URL)}
              >
                Support this project
              </ExternalLink>
            </Text>
          </Box>
        </Box>
      </Box>
    </Page>
  );
}
