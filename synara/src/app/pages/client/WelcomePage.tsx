import React from 'react';
import { Box, Button, Icon, Icons, Text, config, toRem } from 'folds';
import { Page, PageHero, PageHeroSection } from '../../components/page';
import SynaraPNG from '../../../../public/res/png/synara.png';
import {
  APP_VERSION,
  SYNARA_PROJECT_URL,
  SYNARA_RELEASES_URL,
  SYNARA_SOURCE_CODE_URL,
  openExternalUrlFromClick,
} from '../../utils/appLinks';

export function WelcomePage() {
  return (
    <Page>
      <Box
        grow="Yes"
        style={{ padding: config.space.S400, paddingBottom: config.space.S700 }}
        alignItems="Center"
        justifyContent="Center"
      >
        <PageHeroSection>
          <PageHero
            icon={<img width="70" height="70" src={SynaraPNG} alt="Synara Logo" />}
            title="Welcome to Synara"
            subTitle={
              <span>
                A modern Matrix client.{' '}
                <a
                  href={SYNARA_RELEASES_URL}
                  onClick={(evt) => openExternalUrlFromClick(evt, SYNARA_RELEASES_URL)}
                  target="_blank"
                  rel="noreferrer noopener"
                >
                  v{APP_VERSION}
                </a>
              </span>
            }
          >
            <Box justifyContent="Center">
              <Box grow="Yes" style={{ maxWidth: toRem(300) }} direction="Column" gap="300">
                <Button
                  as="a"
                  href={SYNARA_SOURCE_CODE_URL}
                  onClick={(evt) => openExternalUrlFromClick(evt, SYNARA_SOURCE_CODE_URL)}
                  target="_blank"
                  rel="noreferrer noopener"
                  before={<Icon size="200" src={Icons.Code} />}
                >
                  <Text as="span" size="B400" truncate>
                    Source Code
                  </Text>
                </Button>
                <Button
                  as="a"
                  href={SYNARA_PROJECT_URL}
                  onClick={(evt) => openExternalUrlFromClick(evt, SYNARA_PROJECT_URL)}
                  target="_blank"
                  rel="noreferrer noopener"
                  fill="Soft"
                  before={<Icon size="200" src={Icons.Heart} />}
                >
                  <Text as="span" size="B400" truncate>
                    Project
                  </Text>
                </Button>
              </Box>
            </Box>
          </PageHero>
        </PageHeroSection>
      </Box>
    </Page>
  );
}
