import React from 'react';
import { Switch } from 'folds';
import { useTranslation } from 'react-i18next';
import { SequenceCard } from '../../../components/sequence-card';
import { useSetting } from '../../../state/hooks/settings';
import { settingsAtom } from '../../../state/settings';
import { SettingTile } from '../../../components/setting-tile';
import { SequenceCardStyle } from '../styles.css';
import { useClientConfig } from '../../../hooks/useClientConfig';
import { gifPickerEnabled } from '../../../utils/gifProvider';
import { SettingsPage, SettingsSection } from '../../../components/settings-layout';

function Media() {
  const [mediaAutoLoad, setMediaAutoLoad] = useSetting(settingsAtom, 'mediaAutoLoad');

  return (
    <SettingsSection title="Media">
      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile
          title="Load media automatically"
          description="Download and show images and video previews as messages arrive. When off, media loads on request."
          after={<Switch variant="Primary" value={mediaAutoLoad} onChange={setMediaAutoLoad} />}
        />
      </SequenceCard>
    </SettingsSection>
  );
}

function Gifs() {
  const { t } = useTranslation();
  const clientConfig = useClientConfig();
  const [gifSearchEnabled, setGifSearchEnabled] = useSetting(settingsAtom, 'gifSearchEnabled');
  const [inlineGifLinks, setInlineGifLinks] = useSetting(settingsAtom, 'inlineGifLinks');
  const gifProviderAvailable = gifPickerEnabled(clientConfig.gifPicker);

  return (
    <SettingsSection title="GIFs">
      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile
          title={t('modernization.settings.gif_search.title', 'GIF search')}
          description={
            gifProviderAvailable
              ? t(
                  'modernization.settings.gif_search.description',
                  'Show GIF search in the message composer.'
                )
              : t(
                  'modernization.settings.gif_search.unavailable',
                  'GIF search is not configured by this client.'
                )
          }
          after={
            <Switch
              variant="Primary"
              value={gifProviderAvailable && gifSearchEnabled}
              disabled={!gifProviderAvailable}
              onChange={setGifSearchEnabled}
            />
          }
        />
      </SequenceCard>
      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile
          title={t('modernization.settings.inline_gif_links.title', 'Play GIF links inline')}
          description={t(
            'modernization.settings.inline_gif_links.description',
            'Show Klipy and GIPHY links as GIFs. Your device loads them from the provider, which can see your IP address.'
          )}
          after={<Switch variant="Primary" value={inlineGifLinks} onChange={setInlineGifLinks} />}
        />
      </SequenceCard>
    </SettingsSection>
  );
}

function Links() {
  const { t } = useTranslation();
  const [linkPreviews, setLinkPreviews] = useSetting(settingsAtom, 'linkPreviews');
  const [xEmbeds, setXEmbeds] = useSetting(settingsAtom, 'xEmbeds');
  const [shortenLinks, setShortenLinks] = useSetting(settingsAtom, 'shortenLinks');

  return (
    <SettingsSection title="Links">
      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile
          title={t('modernization.settings.link_previews.title', 'Link previews')}
          description={t(
            'modernization.settings.link_previews.description',
            'When a message has a link, Synara fetches the page from this device to show a preview. The site can see your IP address.'
          )}
          after={<Switch variant="Primary" value={linkPreviews} onChange={setLinkPreviews} />}
        />
      </SequenceCard>
      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile
          title={t('modernization.settings.x_embeds.title', 'Show X posts')}
          description={t(
            'modernization.settings.x_embeds.description',
            'Show the post and play its video for x.com links. Your device contacts X, which can see your IP address.'
          )}
          after={<Switch variant="Primary" value={xEmbeds} onChange={setXEmbeds} />}
        />
      </SequenceCard>
      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile
          title={t('modernization.settings.shorten_links.title', 'Shorten long links')}
          description={t(
            'modernization.settings.shorten_links.description',
            'Show a long web address as its site and path. The link still opens the full address, shown when you point at it.'
          )}
          after={<Switch variant="Primary" value={shortenLinks} onChange={setShortenLinks} />}
        />
      </SequenceCard>
    </SettingsSection>
  );
}

type MediaLinksProps = {
  requestClose: () => void;
};
/** Media & links: what loads automatically, GIFs, and how links show. */
export function MediaLinks({ requestClose }: MediaLinksProps) {
  return (
    <SettingsPage
      title="Media & links"
      description="What loads by itself in the timeline, GIFs in the composer, and how links show."
      requestClose={requestClose}
    >
      <Media />
      <Gifs />
      <Links />
    </SettingsPage>
  );
}
