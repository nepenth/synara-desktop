import React from 'react';
import { Box, Text, Icon, Icons, Button, config } from 'folds';
import { SequenceCard } from '../../../components/sequence-card';
import { SequenceCardStyle, SettingsQuietControl } from '../styles.css';
import { SynaraMark } from '../../../pages/auth/SynaraMark';
import {
  APP_VERSION,
  SYNARA_PROJECT_URL,
  SYNARA_SOURCE_CODE_URL,
  SYNARA_SUPPORT_URL,
  openExternalUrlFromClick,
} from '../../../utils/appLinks';
import { SettingsPage, SettingsSection } from '../../../components/settings-layout';
import { BUNDLED_WHATS_NEW } from '../../whats-new/bundledNotes';
import { WhatsNewItems, WhatsNewReleaseLink } from '../../whats-new/WhatsNewCard';
import { whatsNewReleaseUrl } from '../../whats-new/whatsNewNotes';

/**
 * The same notes as the card after an update, for this version, at any
 * time. Without notes for this version, the release page still has them.
 */
function WhatsNewSection() {
  const notes = BUNDLED_WHATS_NEW ?? {
    version: APP_VERSION,
    headline: '',
    items: [],
    releaseUrl: whatsNewReleaseUrl(APP_VERSION),
  };
  const summary = notes.headline ? `${notes.version}: ${notes.headline}` : notes.version;
  return (
    <SettingsSection title="What's new">
      <SequenceCard
        className={SequenceCardStyle}
        variant="SurfaceVariant"
        direction="Column"
        gap="300"
      >
        <Text size="T300" priority="400" data-whats-new-summary="">
          {summary}
        </Text>
        {notes.items.length > 0 && <WhatsNewItems notes={notes} />}
        <WhatsNewReleaseLink notes={notes} />
      </SequenceCard>
    </SettingsSection>
  );
}

type AboutProps = {
  requestClose: () => void;
};
export function About({ requestClose }: AboutProps) {
  return (
    <SettingsPage title="About" requestClose={requestClose}>
      <Box
        direction="Column"
        alignItems="Center"
        gap="400"
        style={{ textAlign: 'center', paddingBlock: config.space.S400 }}
      >
        <SynaraMark size={88} decorative />
        <Box direction="Column" alignItems="Center" gap="300">
          <Box direction="Column" alignItems="Center" gap="100">
            <Text size="H3">Synara</Text>
            <Text size="T300" priority="300">
              Version {APP_VERSION}
            </Text>
          </Box>

          <Box gap="200" wrap="Wrap" justifyContent="Center">
            <Button
              as="a"
              className={SettingsQuietControl}
              href={SYNARA_SOURCE_CODE_URL}
              onClick={(evt) => openExternalUrlFromClick(evt, SYNARA_SOURCE_CODE_URL)}
              rel="noreferrer noopener"
              target="_blank"
              variant="Secondary"
              fill="None"
              size="300"
              radii="300"
              before={<Icon src={Icons.Code} size="100" filled />}
            >
              <Text size="B300">Source code</Text>
            </Button>
            <Button
              as="a"
              className={SettingsQuietControl}
              href={SYNARA_PROJECT_URL}
              onClick={(evt) => openExternalUrlFromClick(evt, SYNARA_PROJECT_URL)}
              rel="noreferrer noopener"
              target="_blank"
              variant="Secondary"
              fill="None"
              size="300"
              radii="300"
              before={<Icon src={Icons.Info} size="100" />}
            >
              <Text size="B300">Project</Text>
            </Button>
            <Button
              as="a"
              className={SettingsQuietControl}
              href={SYNARA_SUPPORT_URL}
              onClick={(evt) => openExternalUrlFromClick(evt, SYNARA_SUPPORT_URL)}
              rel="noreferrer noopener"
              target="_blank"
              variant="Primary"
              fill="Soft"
              size="300"
              radii="300"
              before={<Icon src={Icons.Heart} size="100" filled />}
            >
              <Text size="B300">Support this project</Text>
            </Button>
          </Box>
        </Box>
      </Box>
      <WhatsNewSection />
      <SettingsSection title="Credits">
        <SequenceCard
          className={SequenceCardStyle}
          variant="SurfaceVariant"
          direction="Column"
          gap="400"
        >
          <Box
            as="ul"
            direction="Column"
            gap="200"
            style={{
              margin: 0,
              paddingLeft: config.space.S400,
            }}
          >
            <li>
              <Text size="T300">
                The{' '}
                <a
                  href="https://github.com/matrix-org/matrix-rust-sdk"
                  rel="noreferrer noopener"
                  target="_blank"
                >
                  Matrix Rust SDK
                </a>{' '}
                is ©{' '}
                <a href="https://matrix.org/foundation" rel="noreferrer noopener" target="_blank">
                  The Matrix.org Foundation C.I.C
                </a>{' '}
                and contributors, used under the terms of{' '}
                <a
                  href="http://www.apache.org/licenses/LICENSE-2.0"
                  rel="noreferrer noopener"
                  target="_blank"
                >
                  Apache 2.0
                </a>{' '}
                or{' '}
                <a
                  href="https://opensource.org/license/mit"
                  rel="noreferrer noopener"
                  target="_blank"
                >
                  MIT
                </a>
                .
              </Text>
            </li>
            <li>
              <Text size="T300">
                The{' '}
                <a
                  href="https://github.com/mozilla/twemoji-colr"
                  target="_blank"
                  rel="noreferrer noopener"
                >
                  twemoji-colr
                </a>{' '}
                font is ©{' '}
                <a href="https://mozilla.org/" target="_blank" rel="noreferrer noopener">
                  Mozilla Foundation
                </a>{' '}
                used under the terms of{' '}
                <a
                  href="http://www.apache.org/licenses/LICENSE-2.0"
                  target="_blank"
                  rel="noreferrer noopener"
                >
                  Apache 2.0
                </a>
                .
              </Text>
            </li>
            <li>
              <Text size="T300">
                The{' '}
                <a href="https://twemoji.twitter.com" target="_blank" rel="noreferrer noopener">
                  Twemoji
                </a>{' '}
                emoji art is ©{' '}
                <a href="https://twemoji.twitter.com" target="_blank" rel="noreferrer noopener">
                  Twitter, Inc and other contributors
                </a>{' '}
                used under the terms of{' '}
                <a
                  href="https://creativecommons.org/licenses/by/4.0/"
                  target="_blank"
                  rel="noreferrer noopener"
                >
                  CC-BY 4.0
                </a>
                .
              </Text>
            </li>
            <li>
              <Text size="T300">
                The{' '}
                <a
                  href="https://material.io/design/sound/sound-resources.html"
                  target="_blank"
                  rel="noreferrer noopener"
                >
                  Material sound resources
                </a>{' '}
                are ©{' '}
                <a href="https://google.com" target="_blank" rel="noreferrer noopener">
                  Google
                </a>{' '}
                used under the terms of{' '}
                <a
                  href="https://creativecommons.org/licenses/by/4.0/"
                  target="_blank"
                  rel="noreferrer noopener"
                >
                  CC-BY 4.0
                </a>
                .
              </Text>
            </li>
          </Box>
        </SequenceCard>
      </SettingsSection>
    </SettingsPage>
  );
}
