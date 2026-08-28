import React, { FormEventHandler, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import {
  Avatar,
  Box,
  Button,
  Header,
  Icon,
  Icons,
  Input,
  Scroll,
  Spinner,
  Text,
  config,
  toRem,
} from 'folds';
import { useTranslation } from 'react-i18next';
import {
  gifPickerEnabled,
  searchGifProvider,
  type GifPickerConfig,
  type GifResult,
} from '../../../utils/gifProvider';
import * as depthCss from '../../../styles/Depth.css';

type GifPickerProps = {
  config?: GifPickerConfig;
  onSelect: (gif: GifResult) => void | Promise<void>;
  disabled?: boolean;
  error?: string;
};

export function GifPicker({
  config: gifConfig,
  onSelect,
  disabled,
  error: selectError,
}: GifPickerProps) {
  const { t } = useTranslation();
  const [query, setQuery] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string>();
  const [results, setResults] = useState<GifResult[]>([]);
  const searchSeq = useRef(0);
  const enabled = useMemo(() => gifPickerEnabled(gifConfig), [gifConfig]);

  const search = useCallback(
    async (term: string) => {
      const normalizedTerm = term.trim();
      if (!gifConfig || !enabled || normalizedTerm.length < 2) {
        searchSeq.current += 1;
        setResults([]);
        setLoading(false);
        return;
      }
      const seq = searchSeq.current + 1;
      searchSeq.current = seq;
      setLoading(true);
      setError(undefined);
      try {
        const nextResults = await searchGifProvider(gifConfig, normalizedTerm);
        if (searchSeq.current === seq) setResults(nextResults);
      } catch (err) {
        if (searchSeq.current === seq) {
          setError(err instanceof Error ? err.message : 'Failed to search GIFs.');
        }
      } finally {
        if (searchSeq.current === seq) setLoading(false);
      }
    },
    [gifConfig, enabled]
  );

  useEffect(() => {
    if (query.trim().length < 2) {
      searchSeq.current += 1;
      setResults([]);
      setLoading(false);
      return undefined;
    }
    const timeout = window.setTimeout(() => search(query), 350);
    return () => window.clearTimeout(timeout);
  }, [query, search]);

  const handleSubmit: FormEventHandler<HTMLFormElement> = (evt) => {
    evt.preventDefault();
    search(query);
  };

  if (!enabled) {
    return (
      <Box
        className={depthCss.floatingSurface}
        style={{
          padding: config.space.S400,
          width: toRem(280),
          borderRadius: config.radii.R400,
        }}
      >
        <Text size="T300" priority="300">
          {t(
            'modernization.gif.disabled_by_config',
            'GIF search is disabled by this client configuration.'
          )}
        </Text>
      </Box>
    );
  }

  return (
    <Box
      className={depthCss.floatingSurface}
      direction="Column"
      style={{ width: toRem(360), maxWidth: '80vw', borderRadius: config.radii.R400 }}
    >
      <Header size="500" style={{ padding: `0 ${config.space.S300}` }}>
        <Box as="form" onSubmit={handleSubmit} grow="Yes" gap="200">
          <Input
            autoFocus
            variant="Background"
            placeholder={t('modernization.gif.search_placeholder', 'Search GIFs')}
            aria-label={t('modernization.gif.search_aria_label', 'Search GIFs')}
            value={query}
            onChange={(evt) => setQuery(evt.currentTarget.value)}
          />
          <Button
            type="submit"
            size="300"
            variant="Secondary"
            disabled={loading || disabled}
            aria-label={t('modernization.gif.search_button_aria_label', 'Search GIFs')}
          >
            {loading ? <Spinner size="100" variant="Secondary" /> : <Icon src={Icons.Search} />}
          </Button>
        </Box>
      </Header>
      <Scroll size="300" hideTrack visibility="Hover">
        <Box
          style={{
            padding: config.space.S300,
            display: 'grid',
            gridTemplateColumns: 'repeat(3, minmax(0, 1fr))',
            gap: config.space.S200,
            minHeight: toRem(160),
          }}
        >
          {(error || selectError) && (
            <Text size="T300" style={{ gridColumn: '1 / -1' }} priority="300">
              {error || selectError}
            </Text>
          )}
          {!error &&
            !selectError &&
            !loading &&
            query.trim().length >= 2 &&
            results.length === 0 && (
              <Text size="T300" style={{ gridColumn: '1 / -1' }} priority="300">
                {t('modernization.gif.no_results', 'No GIFs found.')}
              </Text>
            )}
          {results.map((gif) => (
            <Avatar
              as="button"
              key={`${gif.provider ?? 'custom'}-${gif.id}`}
              size="500"
              radii="300"
              title={gif.title}
              aria-label={t('modernization.gif.select_aria_label', 'Send {{title}}', {
                title: gif.title,
              })}
              disabled={disabled}
              onClick={() => {
                if (!disabled) onSelect(gif);
              }}
            >
              <img
                alt={gif.title}
                src={gif.previewUrl ?? gif.url}
                loading="lazy"
                referrerPolicy="no-referrer"
                crossOrigin="anonymous"
                style={{ width: '100%', height: '100%', objectFit: 'cover' }}
              />
            </Avatar>
          ))}
        </Box>
      </Scroll>
    </Box>
  );
}
