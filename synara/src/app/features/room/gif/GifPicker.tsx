import React, {
  FormEventHandler,
  UIEventHandler,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react';
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
  gifAttribution,
  gifPickerEnabled,
  gifSearchPlaceholder,
  gifTrendingSupported,
  searchGifProviderPage,
  trendingGifProviderPage,
  type GifPage,
  type GifPickerConfig,
  type GifResult,
} from '../../../utils/gifProvider';
import * as depthCss from '../../../styles/Depth.css';
import { composerErrorMessage } from '../composerSendError';

type GifPickerProps = {
  config?: GifPickerConfig;
  onSelect: (gif: GifResult) => void | Promise<void>;
  disabled?: boolean;
  error?: string;
  /** Search to start with, such as the text after `/gif`. */
  initialQuery?: string;
};

const LOAD_MORE_THRESHOLD_PX = 160;

export function GifPicker({
  config: gifConfig,
  onSelect,
  disabled,
  error: selectError,
  initialQuery = '',
}: GifPickerProps) {
  const { t } = useTranslation();
  const [query, setQuery] = useState(initialQuery);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string>();
  const [results, setResults] = useState<GifResult[]>([]);
  const [nextPage, setNextPage] = useState<number>();
  const searchSeq = useRef(0);
  const enabled = useMemo(() => gifPickerEnabled(gifConfig), [gifConfig]);
  const trending = useMemo(() => gifTrendingSupported(gifConfig), [gifConfig]);

  /** Loads page `page` of the feed for `term`: trending when it is empty. */
  const load = useCallback(
    async (term: string, page: number) => {
      const normalizedTerm = term.trim();
      const showTrending = normalizedTerm.length === 0 && trending;
      if (!gifConfig || !enabled || (!showTrending && normalizedTerm.length < 2)) {
        searchSeq.current += 1;
        setResults([]);
        setNextPage(undefined);
        setLoading(false);
        return;
      }
      const seq = searchSeq.current + 1;
      searchSeq.current = seq;
      setLoading(true);
      setError(undefined);
      try {
        const next: GifPage = showTrending
          ? await trendingGifProviderPage(gifConfig, page)
          : await searchGifProviderPage(gifConfig, normalizedTerm, page);
        if (searchSeq.current !== seq) return;
        setResults((current) => (page > 1 ? [...current, ...next.results] : next.results));
        setNextPage(next.nextPage);
      } catch (err) {
        if (searchSeq.current === seq) {
          setError(composerErrorMessage(err, 'Synara could not search GIFs. Try again.'));
        }
      } finally {
        if (searchSeq.current === seq) setLoading(false);
      }
    },
    [gifConfig, enabled, trending]
  );

  useEffect(() => {
    const trimmed = query.trim();
    if (trimmed.length === 1 || (trimmed.length === 0 && !trending)) {
      searchSeq.current += 1;
      setResults([]);
      setNextPage(undefined);
      setLoading(false);
      return undefined;
    }
    const timeout = window.setTimeout(() => load(query, 1), trimmed.length === 0 ? 0 : 350);
    return () => window.clearTimeout(timeout);
  }, [query, load, trending]);

  const handleSubmit: FormEventHandler<HTMLFormElement> = (evt) => {
    evt.preventDefault();
    load(query, 1);
  };

  const handleScroll: UIEventHandler<HTMLDivElement> = (evt) => {
    if (loading || nextPage === undefined) return;
    const target = evt.currentTarget;
    if (target.scrollHeight - target.scrollTop - target.clientHeight < LOAD_MORE_THRESHOLD_PX) {
      load(query, nextPage);
    }
  };

  const placeholder =
    gifSearchPlaceholder(gifConfig) ?? t('modernization.gif.search_placeholder', 'Search GIFs');
  const attribution = gifAttribution(gifConfig);
  const showingTrending = query.trim().length === 0 && trending;

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
      style={{ width: toRem(420), maxWidth: '80vw', borderRadius: config.radii.R400 }}
    >
      <Header size="500" style={{ padding: `0 ${config.space.S300}` }}>
        <Box as="form" onSubmit={handleSubmit} grow="Yes" alignItems="Center" gap="200">
          {/* Input sizes to its content, so this box gives it the free width. */}
          <Box grow="Yes" direction="Column">
            <Input
              autoFocus
              variant="Background"
              size="400"
              placeholder={placeholder}
              aria-label={t('modernization.gif.search_aria_label', 'Search GIFs')}
              value={query}
              onChange={(evt) => setQuery(evt.currentTarget.value)}
            />
          </Box>
          <Button
            type="submit"
            size="300"
            variant="Secondary"
            style={{ flexShrink: 0 }}
            disabled={loading || disabled}
            aria-label={t('modernization.gif.search_button_aria_label', 'Search GIFs')}
          >
            {loading ? <Spinner size="100" variant="Secondary" /> : <Icon src={Icons.Search} />}
          </Button>
        </Box>
      </Header>
      {showingTrending && (
        <Text
          size="L400"
          priority="300"
          style={{ padding: `${config.space.S200} ${config.space.S400} 0` }}
        >
          {t('modernization.gif.trending', 'Trending')}
        </Text>
      )}
      <Scroll
        size="300"
        hideTrack
        visibility="Hover"
        style={{ maxHeight: toRem(360) }}
        onScroll={handleScroll}
        data-testid="gif-picker-results"
      >
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
              radii="300"
              style={{ width: '100%', height: toRem(112) }}
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
      {attribution && (
        <Text
          size="T200"
          priority="300"
          align="Right"
          style={{ padding: `${config.space.S100} ${config.space.S300} ${config.space.S200}` }}
        >
          {attribution}
        </Text>
      )}
    </Box>
  );
}
